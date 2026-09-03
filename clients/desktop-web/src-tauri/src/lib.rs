#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use msc_infrastructure::secret_store::SecretStore;
use msc_infrastructure::service::{
    ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceName, ServiceState,
    ServiceStatusReport,
};
use rand::RngCore;
use reqwest::{header, Method, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

mod update;

const DESKTOP_CREDENTIAL_KEY_PREFIX: &str = "msc.desktop.host-token.";
const LOCAL_HOST_ID_KEY: &str = "msc.desktop.local-agent-host-id";
const AGENT_SERVICE_NAME: &str = "com.ctemple.msc2.agent";
const AGENT_PORT: u16 = 48001;
const LOCAL_AGENT_BROWSER_ORIGIN: &str = "http://127.0.0.1:48001";
const LOCAL_BOOTSTRAP_SOCKET: &str = "bootstrap.sock";
const BEDROCK_SIDECAR_DIRECTORY_ENV: &str = "MSC2_BEDROCK_SIDECAR_DIR";
const PROTOCOL_VERSION: u32 = 1;
const PROOF_DOMAIN: &[u8] = b"msc2-local-bootstrap-v1\0";
static STAGED_PACKAGED_AGENT_PATH: PackagedAgentPathCache = PackagedAgentPathCache::new();

struct PackagedAgentPathCache(Mutex<Option<Result<PathBuf, String>>>);

impl PackagedAgentPathCache {
    const fn new() -> Self {
        Self(Mutex::new(None))
    }

    fn resolve(&self, stage: impl FnOnce() -> Result<PathBuf, String>) -> Result<PathBuf, String> {
        let mut cached = self.0.lock().expect("packaged agent path cache poisoned");
        if let Some(path) = cached.as_ref() {
            return path.clone();
        }
        let path = stage();
        *cached = Some(path.clone());
        path
    }

    fn refresh(&self, stage: impl FnOnce() -> Result<PathBuf, String>) -> Result<PathBuf, String> {
        let path = stage();
        *self.0.lock().expect("packaged agent path cache poisoned") = Some(path.clone());
        path
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum AgentServiceAction {
    Install,
    Start,
    Stop,
    Repair,
    Uninstall,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentServiceStatus {
    available: bool,
    platform: &'static str,
    service_name: &'static str,
    state: &'static str,
    pid: Option<u32>,
    detail: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopPairingRequest {
    base_url: String,
    pairing_code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopPairingResult {
    agent_host_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopRequest {
    agent_host_id: String,
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredDesktopCredential {
    base_url: String,
    token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForgetDesktopCredentialsRequest {
    host_ids: Vec<String>,
    include_local_host: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopCredentialResult {
    agent_host_id: String,
    credential_id: String,
    token: String,
    expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowserPairingResult {
    pairing_code: String,
    agent_host_id: String,
    client_kind: String,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalBootstrapChallenge {
    status: String,
    version: u32,
    host_id: String,
    challenge: String,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalBootstrapResponse {
    status: String,
    version: u32,
    agent_host_id: Option<String>,
    token: Option<String>,
    code: Option<String>,
}

/// Redeems a pairing code entirely in the native process. The webview receives
/// only the agent host ID; the bearer value goes directly to the platform
/// credential store and is never a Tauri command result.
#[tauri::command]
async fn desktop_exchange_pairing(
    request: DesktopPairingRequest,
) -> Result<DesktopPairingResult, String> {
    let base_url = canonical_base_url(&request.base_url)?;
    if request.pairing_code.trim().is_empty() {
        return Err("A desktop pairing code is required.".to_string());
    }
    let response = reqwest::Client::new()
        .post(format!("{base_url}/v1/auth/desktop-pairings"))
        .json(&serde_json::json!({ "pairingCode": request.pairing_code }))
        .send()
        .await
        .map_err(|error| format!("Desktop pairing request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Desktop pairing was refused (HTTP {}).",
            response.status()
        ));
    }
    let result: DesktopCredentialResult = response
        .json()
        .await
        .map_err(|error| format!("Desktop pairing returned an invalid response: {error}"))?;
    if result.agent_host_id.trim().is_empty()
        || result.credential_id.trim().is_empty()
        || !result.token.starts_with("msc2_")
    {
        return Err("Desktop pairing returned an invalid credential.".to_string());
    }
    let record = StoredDesktopCredential {
        base_url,
        token: result.token,
    };
    desktop_secret_store()?
        .set(
            &credential_key(&result.agent_host_id),
            &serde_json::to_string(&record).expect("desktop credential serializes"),
        )
        .map_err(|error| error.to_string())?;
    let _ = result.expires_at;
    Ok(DesktopPairingResult {
        agent_host_id: result.agent_host_id,
    })
}

/// Performs the same-machine bootstrap over the agent's Unix socket. The
/// bearer value is written directly to the native credential store and is
/// never returned to Svelte.
#[tauri::command]
fn desktop_bootstrap_local() -> Result<DesktopPairingResult, String> {
    ensure_current_local_agent_service()?;
    #[cfg(target_os = "macos")]
    {
        return bootstrap_local_macos();
    }
    #[allow(unreachable_code)]
    Err("Local desktop bootstrap is unavailable on this platform.".to_string())
}

/// Removes only credentials named by the client, plus the special local-host
/// record when requested. The bearer values never cross back into the webview.
#[tauri::command]
fn desktop_forget_credentials(request: ForgetDesktopCredentialsRequest) -> Result<(), String> {
    let store = desktop_secret_store()?;
    for host_id in request.host_ids {
        let host_id = host_id.trim();
        if !host_id.is_empty() {
            store
                .delete(&credential_key(host_id))
                .map_err(|error| error.to_string())?;
        }
    }
    if request.include_local_host {
        if let Some(local_host_id) = store
            .get(LOCAL_HOST_ID_KEY)
            .map_err(|error| error.to_string())?
        {
            store
                .delete(&credential_key(&local_host_id))
                .map_err(|error| error.to_string())?;
        }
        store
            .delete(LOCAL_HOST_ID_KEY)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn bootstrap_local_macos() -> Result<DesktopPairingResult, String> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    let store = desktop_secret_store()?;
    if let Some(host_id) = store
        .get(LOCAL_HOST_ID_KEY)
        .map_err(|error| error.to_string())?
    {
        if store
            .get(&credential_key(&host_id))
            .map_err(|error| error.to_string())?
            .is_some()
        {
            return Ok(DesktopPairingResult {
                agent_host_id: host_id,
            });
        }
    }

    let key = ensure_local_bootstrap_key()?;
    let socket = agent_data_directory()?.join(LOCAL_BOOTSTRAP_SOCKET);
    let mut stream = UnixStream::connect(&socket).map_err(|error| {
        format!(
            "Could not connect to the local agent bootstrap channel at {}: {error}",
            socket.display()
        )
    })?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|error| format!("Could not configure the bootstrap channel: {error}"))?;
    stream
        .write_all(format!("{}\n", serde_json::json!({ "version": PROTOCOL_VERSION })).as_bytes())
        .map_err(|error| format!("Could not send the bootstrap hello: {error}"))?;
    let mut reader = BufReader::new(
        stream
            .try_clone()
            .map_err(|error| format!("Could not read the bootstrap channel: {error}"))?,
    );
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|error| format!("Could not read the bootstrap challenge: {error}"))?;
    let challenge: LocalBootstrapChallenge = serde_json::from_str(&line).map_err(|error| {
        format!("The local agent returned an invalid bootstrap challenge: {error}")
    })?;
    if challenge.status != "challenge" || challenge.version != PROTOCOL_VERSION {
        return Err("The local agent rejected the bootstrap protocol.".to_string());
    }

    let proof = local_bootstrap_proof(&key, &challenge.challenge, &challenge.host_id);
    stream
        .write_all(
            format!(
                "{}\n",
                serde_json::json!({
                    "version": PROTOCOL_VERSION,
                    "hostId": challenge.host_id,
                    "proof": proof,
                })
            )
            .as_bytes(),
        )
        .map_err(|error| format!("Could not send the bootstrap proof: {error}"))?;
    line.clear();
    reader
        .read_line(&mut line)
        .map_err(|error| format!("Could not read the bootstrap result: {error}"))?;
    let response: LocalBootstrapResponse = serde_json::from_str(&line).map_err(|error| {
        format!("The local agent returned an invalid bootstrap result: {error}")
    })?;
    if response.status != "ok" || response.version != PROTOCOL_VERSION {
        return Err(format!(
            "The local agent refused desktop bootstrap{}.",
            response
                .code
                .map(|code| format!(" ({code})"))
                .unwrap_or_default()
        ));
    }
    let host_id = response
        .agent_host_id
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "The local agent returned no host identity.".to_string())?;
    let token = response
        .token
        .filter(|value| value.starts_with("msc2_"))
        .ok_or_else(|| "The local agent returned no valid desktop credential.".to_string())?;
    let record = StoredDesktopCredential {
        base_url: "http://127.0.0.1:48001".to_string(),
        token,
    };
    store
        .set(
            &credential_key(&host_id),
            &serde_json::to_string(&record).expect("desktop credential serializes"),
        )
        .map_err(|error| error.to_string())?;
    store
        .set(LOCAL_HOST_ID_KEY, &host_id)
        .map_err(|error| error.to_string())?;
    Ok(DesktopPairingResult {
        agent_host_id: host_id,
    })
}

/// A plain 0600 file under the agent's own secrets directory, not the
/// keychain (see `secret_store.rs`'s module doc): the desktop app and the
/// agent daemon always run as the same regular user, so this file's own
/// Unix permissions are exactly as strong a boundary as a keychain ACL would
/// be here, without the ACL/session pitfalls that mechanism kept hitting in
/// practice. Generated once, unprivileged, before the elevated install step
/// even runs — nothing needs a password to provision this secret.
#[cfg(target_os = "macos")]
fn ensure_local_bootstrap_key() -> Result<String, String> {
    use std::os::unix::fs::PermissionsExt;

    let secrets_dir = secrets_directory()?;
    std::fs::create_dir_all(&secrets_dir)
        .map_err(|error| format!("Could not create the secrets directory: {error}"))?;
    let path = msc_platform_macos::service::local_bootstrap_key_path(&secrets_dir);
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let key = hex_lower(&bytes);
    std::fs::write(&path, &key)
        .map_err(|error| format!("Could not write the bootstrap key: {error}"))?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("Could not restrict the bootstrap key permissions: {error}"))?;
    Ok(key)
}

#[cfg(target_os = "macos")]
fn secrets_directory() -> Result<PathBuf, String> {
    Ok(agent_data_directory()?.join("secrets"))
}

#[cfg(target_os = "macos")]
fn local_bootstrap_proof(key: &str, challenge: &str, host_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(PROOF_DOMAIN);
    digest.update(key.as_bytes());
    digest.update(challenge.as_bytes());
    digest.update(host_id.as_bytes());
    hex_lower(&digest.finalize())
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

/// Proxies one request to the origin stored with this host's credential. The
/// caller supplies a relative API path, so a compromised webview cannot turn
/// the shell into a bearer-token relay to a different origin.
#[tauri::command]
async fn desktop_authorized_request(request: DesktopRequest) -> Result<DesktopResponse, String> {
    let key = credential_key(&request.agent_host_id);
    let store = desktop_secret_store()?;
    let Some(record) = store.get(&key).map_err(|error| error.to_string())? else {
        return Err("This desktop has no credential for the selected host.".to_string());
    };
    let record: StoredDesktopCredential = serde_json::from_str(&record)
        .map_err(|error| format!("Stored desktop credential is invalid: {error}"))?;
    if record.base_url == LOCAL_AGENT_BROWSER_ORIGIN {
        ensure_current_local_agent_service()?;
    }
    let method = Method::from_bytes(request.method.as_bytes())
        .map_err(|_| "The requested HTTP method is not supported.".to_string())?;
    let url = relative_request_url(&record.base_url, &request.path)?;
    let mut builder = reqwest::Client::new()
        .request(method, url)
        .header(header::AUTHORIZATION, format!("Bearer {}", record.token));
    for (name, value) in request.headers {
        let lower = name.to_ascii_lowercase();
        if matches!(lower.as_str(), "authorization" | "cookie" | "host") {
            continue;
        }
        builder = builder.header(name, value);
    }
    if let Some(body) = request.body {
        builder = builder.body(body);
    }
    let response = builder
        .send()
        .await
        .map_err(|error| format!("Desktop request failed: {error}"))?;
    let status = response.status();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_string(), value.to_string()))
        })
        .collect();
    let body = response
        .bytes()
        .await
        .map_err(|error| format!("Desktop response could not be read: {error}"))?
        .to_vec();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        // A revoked or expired credential must not linger locally after the
        // agent has authoritatively rejected it.
        store.delete(&key).map_err(|error| error.to_string())?;
    }
    Ok(DesktopResponse {
        status: status.as_u16(),
        headers,
        body,
    })
}

/// Gives the default browser its own revocable cookie session without exposing
/// the desktop bearer token to either the webview or the browser. The one-use
/// pairing code stays in the URL fragment, which HTTP never sends to the agent.
#[tauri::command]
async fn open_local_agent_browser() -> Result<(), String> {
    let local = desktop_bootstrap_local()?;
    let mut response = create_local_browser_pairing(&local.agent_host_id).await?;
    // `desktop_authorized_request` removes a rejected bearer record. Refresh
    // immediately so opening the browser is one action, not an invisible
    // failed click followed by a second attempt after the stale record is gone.
    if browser_pairing_needs_local_credential_refresh(response.status) {
        let refreshed = desktop_bootstrap_local()?;
        response = create_local_browser_pairing(&refreshed.agent_host_id).await?;
    }
    if response.status != reqwest::StatusCode::CREATED.as_u16() {
        return Err(format!(
            "The local agent refused to create a browser session (HTTP {}).",
            response.status
        ));
    }
    let pairing: BrowserPairingResult = serde_json::from_slice(&response.body)
        .map_err(|error| format!("The local agent returned an invalid browser pairing: {error}"))?;
    if pairing.client_kind != "browser" || pairing.agent_host_id.trim().is_empty() {
        return Err("The local agent returned an invalid browser pairing.".to_string());
    }
    open_external_url(local_browser_handoff_url(&pairing.pairing_code)?)
}

async fn create_local_browser_pairing(agent_host_id: &str) -> Result<DesktopResponse, String> {
    desktop_authorized_request(DesktopRequest {
        agent_host_id: agent_host_id.to_string(),
        method: "POST".to_string(),
        path: "/v1/auth/pairings".to_string(),
        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
        body: Some(
            serde_json::to_vec(&serde_json::json!({
                "clientKind": "browser",
                "label": "Local browser",
                "role": "admin",
                "permissions": [
                    "serverControl",
                    "players",
                    "settings",
                    "addons",
                    "worlds",
                    "broadcast",
                    "networking",
                    "fleet",
                    "admin"
                ]
            }))
            .expect("local browser pairing request serializes"),
        ),
    })
    .await
}

fn browser_pairing_needs_local_credential_refresh(status: u16) -> bool {
    status == reqwest::StatusCode::UNAUTHORIZED.as_u16()
}

fn local_browser_handoff_url(pairing_code: &str) -> Result<String, String> {
    if !pairing_code.starts_with("pair_")
        || !pairing_code
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    {
        return Err("The local agent returned an invalid browser pairing.".to_string());
    }
    Ok(format!(
        "{LOCAL_AGENT_BROWSER_ORIGIN}/#browser-pairing={pairing_code}"
    ))
}

/// Reports the service separately from the browser's connection state. It does
/// not start anything: opening or closing the desktop shell is never a server
/// lifecycle action.
#[tauri::command]
fn agent_service_status() -> Result<AgentServiceStatus, String> {
    let expected_binary = staged_packaged_agent_path()?;
    service_manager()?
        .execute(ServiceManagerCommand::Status {
            service_name: ServiceName::new(AGENT_SERVICE_NAME),
        })
        .map(|report| report_status_for_packaged_agent(report, &expected_binary))
        .map_err(|error| error.to_string())
}

/// Checks the cheap pre-auth liveness route from the native shell. A webview
/// fetch is cross-origin and can be blocked by browser policy before
/// credentials exist; this route must not perform the full health-card scan.
#[tauri::command]
async fn agent_health_check() -> bool {
    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    else {
        return false;
    };
    client
        .get("http://127.0.0.1:48001/v1/healthz")
        .send()
        .await
        .is_ok_and(|response| response.status().is_success())
}

/// Ends the desktop process after a reset has removed all local state. The
/// agent is a separate service and is stopped before this command is called.
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// The shared setup screen has this narrow native seam for an explicit local
/// service action. Platform registration may trigger the OS elevation flow;
/// routine agent operation remains under the installing user's account.
#[tauri::command]
fn manage_agent_service(action: AgentServiceAction) -> Result<AgentServiceStatus, String> {
    let service_name = ServiceName::new(AGENT_SERVICE_NAME);
    if matches!(&action, AgentServiceAction::Start) {
        ensure_installed_agent_matches_package()?;
    }
    #[cfg(target_os = "macos")]
    let report = match action {
        AgentServiceAction::Install | AgentServiceAction::Repair => {
            let request = agent_install_request()?;
            let expected_binary = request.binary_path.clone();
            let report = msc_platform_macos::service::install_and_start_elevated(request)
                .map_err(|error| error.to_string())?;
            ensure_service_report_uses_binary(report, &expected_binary)?
        }
        AgentServiceAction::Start => {
            msc_platform_macos::service::start_elevated(service_name.as_str())
                .map_err(|error| error.to_string())?
        }
        AgentServiceAction::Stop => {
            msc_platform_macos::service::stop_elevated(service_name.as_str())
                .map_err(|error| error.to_string())?
        }
        AgentServiceAction::Uninstall => {
            msc_platform_macos::service::uninstall_elevated(service_name.as_str())
                .map_err(|error| error.to_string())?
        }
    };
    #[cfg(not(target_os = "macos"))]
    let report = {
        let manager = service_manager()?;
        match action {
            AgentServiceAction::Install | AgentServiceAction::Repair => {
                let request = agent_install_request()?;
                let expected_binary = request.binary_path.clone();
                manager
                    .execute(ServiceManagerCommand::Install(request))
                    .map_err(|error| error.to_string())?;
                let report = manager
                    .execute(ServiceManagerCommand::Start { service_name })
                    .map_err(|error| error.to_string())?;
                ensure_service_report_uses_binary(report, &expected_binary)?
            }
            AgentServiceAction::Start => manager
                .execute(ServiceManagerCommand::Start { service_name })
                .map_err(|error| error.to_string())?,
            AgentServiceAction::Stop => manager
                .execute(ServiceManagerCommand::Stop { service_name })
                .map_err(|error| error.to_string())?,
            AgentServiceAction::Uninstall => manager
                .execute(ServiceManagerCommand::Uninstall { service_name })
                .map_err(|error| error.to_string())?,
        }
    };
    Ok(report_status(report))
}

/// Copies a complete, signed release set into immutable staging. It cannot
/// install anything; a platform installer is launched only after the shared
/// client has collected a separate, explicit confirmation for this release.
#[tauri::command]
fn stage_coordinated_update(request: update::StageRequest) -> Result<update::StageResult, String> {
    update::stage(request, &agent_data_directory()?)
}

fn service_manager() -> Result<Box<dyn ServiceManager>, String> {
    #[cfg(target_os = "macos")]
    {
        return Ok(Box::new(
            msc_platform_macos::service::MacosLaunchdServiceManager::new(),
        ));
    }
    #[cfg(target_os = "windows")]
    {
        return Ok(Box::new(
            msc_platform_windows::service::WindowsServiceManager::new(),
        ));
    }
    #[cfg(target_os = "linux")]
    {
        return Ok(Box::new(
            msc_platform_linux::service::LinuxSystemdServiceManager::new(),
        ));
    }
    #[allow(unreachable_code)]
    Err("This desktop platform has no MSC service manager.".to_string())
}

fn agent_install_request() -> Result<ServiceInstallRequest, String> {
    // Repair is the explicit boundary where a running dev shell may have
    // received a newly staged resource. Refresh the content-addressed copy
    // instead of trusting the status path cache from before that rebuild.
    let binary_path = refresh_staged_packaged_agent_path()?;
    let working_directory = agent_data_directory()?;
    let secret_store_directory = working_directory.join("secrets");
    #[cfg(target_os = "macos")]
    let sidecar_directory = packaged_bedrock_sidecar_directory()?;
    std::fs::create_dir_all(working_directory.join("logs"))
        .map_err(|error| format!("Could not create the agent data directory: {error}"))?;
    std::fs::create_dir_all(&secret_store_directory)
        .map_err(|error| format!("Could not create the agent secret directory: {error}"))?;
    let home = std::env::var("HOME")
        .map_err(|_| "Could not determine the installing user's home directory.".to_string())?;
    // Both writes below are unprivileged (the desktop app already owns this
    // directory) and must happen before the elevated install step, which
    // only checks that the bootstrap key file exists rather than
    // provisioning any secret itself — see `secret_store.rs`'s module doc.
    #[cfg(target_os = "macos")]
    let desktop_requirement = {
        ensure_local_bootstrap_key()?;
        desktop_code_requirement()?
    };
    let request = ServiceInstallRequest::new(
        AGENT_SERVICE_NAME,
        binary_path,
        &working_directory,
        working_directory.join("logs/agent.log"),
        AGENT_PORT,
    )
    .args(["serve", "--bind", "127.0.0.1:48001"])
    .env("HOME", home)
    .env("MSC2_DATA_DIR", working_directory.display().to_string())
    .env(
        "MSC2_MACOS_SECRET_STORE_DIR",
        secret_store_directory.display().to_string(),
    )
    .env(
        "MSC2_LOCAL_BOOTSTRAP_SOCKET",
        working_directory
            .join(LOCAL_BOOTSTRAP_SOCKET)
            .display()
            .to_string(),
    );
    #[cfg(target_os = "macos")]
    let request = request
        .env(
            BEDROCK_SIDECAR_DIRECTORY_ENV,
            sidecar_directory.display().to_string(),
        )
        .env("MSC2_MACOS_DESKTOP_REQUIREMENT", desktop_requirement);
    Ok(request.run_user(installing_user()?))
}

#[cfg(target_os = "macos")]
fn desktop_code_requirement() -> Result<String, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("Could not locate the desktop executable: {error}"))?;
    read_designated_requirement(&executable)?.ok_or_else(|| {
        "The desktop executable has no designated code requirement. \
         It should have been ad-hoc signed at startup — this is unexpected."
            .to_string()
    })
}

/// A `cargo`/`tauri dev` build is never Developer-ID signed (only `tauri
/// build`'s bundling step signs for real), so it has no designated
/// requirement for the agent to check the running app against. Ad-hoc
/// signing gives it one without touching an already-signed release build's
/// real trust chain — this only fires when no requirement exists at all.
///
/// Signing the file out from under the *already-running* process that was
/// exec'd from it invalidates that process's own live code identity in the
/// kernel's eyes (self-modifying your own mapped executable is exactly what
/// code-signing enforcement exists to catch) — `verify_process_code_identity`
/// on the agent side would then refuse this very process with "code identity
/// has been invalidated". So instead of signing and continuing, this re-execs
/// into the freshly-signed file, replacing the process image outright; the
/// process that continues past this call was loaded fresh from the signed
/// binary and has a valid, uninvalidated identity.
#[cfg(target_os = "macos")]
fn ensure_ad_hoc_signed_or_reexec() {
    use std::os::unix::process::CommandExt;

    let Ok(executable) = std::env::current_exe() else {
        return;
    };
    if matches!(read_designated_requirement(&executable), Ok(Some(_))) {
        return;
    }
    let signed = std::process::Command::new("/usr/bin/codesign")
        .args(["--force", "--sign", "-", "--"])
        .arg(&executable)
        .status()
        .is_ok_and(|status| status.success());
    if !signed {
        // Falls through to the ordinary startup path; `desktop_code_requirement`
        // will surface a clear error if a bootstrap install is attempted.
        return;
    }
    let error = std::process::Command::new(&executable)
        .args(std::env::args_os().skip(1))
        .exec();
    eprintln!("msc2-desktop-web: re-exec after ad-hoc signing failed: {error}");
}

#[cfg(target_os = "macos")]
fn read_designated_requirement(executable: &Path) -> Result<Option<String>, String> {
    let output = std::process::Command::new("/usr/bin/codesign")
        .args(["-d", "-r-", "--"])
        .arg(executable)
        .output()
        .map_err(|error| format!("Could not inspect the desktop code signature: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(parse_designated_requirement(&stdout, &stderr).map(str::to_string))
}

#[cfg(target_os = "macos")]
fn parse_designated_requirement<'a>(stdout: &'a str, stderr: &'a str) -> Option<&'a str> {
    stdout
        .lines()
        .chain(stderr.lines())
        .find_map(|line| {
            line.strip_prefix("# designated => ")
                .or_else(|| line.strip_prefix("designated => "))
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn packaged_agent_path() -> Result<PathBuf, String> {
    let desktop_binary = std::env::current_exe()
        .map_err(|error| format!("Could not locate the desktop application: {error}"))?;
    let directory = desktop_binary
        .parent()
        .ok_or_else(|| "The desktop application has no containing directory.".to_string())?;
    #[cfg(target_os = "macos")]
    return Ok(directory.join("../Resources/agent/msc"));
    #[cfg(target_os = "windows")]
    return Ok(directory.join("agent/msc.exe"));
    #[cfg(target_os = "linux")]
    {
        let development_path = directory.join("agent/msc");
        if development_path.is_file() {
            return Ok(development_path);
        }
        return Ok(directory.join("../lib/msc2-desktop-web/agent/msc"));
    }
    #[allow(unreachable_code)]
    Err("This desktop platform has no agent-package layout.".to_string())
}

#[cfg(target_os = "macos")]
fn packaged_bedrock_sidecar_directory() -> Result<PathBuf, String> {
    let desktop_binary = std::env::current_exe()
        .map_err(|error| format!("Could not locate the desktop application: {error}"))?;
    let directory = desktop_binary
        .parent()
        .ok_or_else(|| "The desktop application has no containing directory.".to_string())?;
    let sidecar = directory.join("../Resources/agent/sidecar");
    for name in ["BedrockSidecar", "vmlinuz-kata", "appliance-initramfs.gz"] {
        let resource = sidecar.join(name);
        if !resource.is_file() {
            return Err(format!(
                "The Bedrock sidecar package is missing {name} at {}. Rebuild the macOS package with MSC2_BEDROCK_APPLIANCE_DIR set before registering the service.",
                resource.display()
            ));
        }
    }
    Ok(sidecar)
}

fn staged_packaged_agent_path() -> Result<PathBuf, String> {
    STAGED_PACKAGED_AGENT_PATH.resolve(stage_packaged_agent_once)
}

fn stage_packaged_agent_once() -> Result<PathBuf, String> {
    let source = packaged_agent_path()?;
    if !source.is_file() {
        return Err(format!(
            "The compatible agent package is missing at {}. Reinstall this desktop app before registering the service.",
            source.display()
        ));
    }
    stage_packaged_agent(&source, &agent_data_directory()?)
}

fn refresh_staged_packaged_agent_path() -> Result<PathBuf, String> {
    STAGED_PACKAGED_AGENT_PATH.refresh(stage_packaged_agent_once)
}

fn stage_packaged_agent(source: &Path, data_directory: &Path) -> Result<PathBuf, String> {
    let source_bytes = std::fs::read(source)
        .map_err(|error| format!("Could not read the packaged agent: {error}"))?;
    let digest = hex_lower(&Sha256::digest(&source_bytes));
    let file_name = source
        .file_name()
        .ok_or_else(|| "The packaged agent path has no file name.".to_string())?;
    let build_directory = data_directory.join("agent/builds").join(digest);
    let destination = build_directory.join(file_name);

    if destination.is_file() {
        verify_staged_agent(&destination, &source_bytes)?;
        return Ok(destination);
    }

    std::fs::create_dir_all(&build_directory)
        .map_err(|error| format!("Could not create the packaged agent directory: {error}"))?;
    let temporary = build_directory.join(format!(
        ".{}.{}.stage",
        file_name.to_string_lossy(),
        std::process::id()
    ));
    std::fs::copy(source, &temporary)
        .map_err(|error| format!("Could not stage the packaged agent: {error}"))?;
    match std::fs::rename(&temporary, &destination) {
        Ok(()) => {}
        Err(_) if destination.is_file() => {
            let _ = std::fs::remove_file(&temporary);
            verify_staged_agent(&destination, &source_bytes)?;
        }
        Err(error) => {
            return Err(format!("Could not finalize the packaged agent: {error}"));
        }
    }
    verify_staged_agent(&destination, &source_bytes)?;
    Ok(destination)
}

fn verify_staged_agent(destination: &Path, source_bytes: &[u8]) -> Result<(), String> {
    let staged_bytes = std::fs::read(destination)
        .map_err(|error| format!("Could not verify the staged agent: {error}"))?;
    if Sha256::digest(&staged_bytes) != Sha256::digest(source_bytes) {
        return Err(format!(
            "The immutable staged agent at {} does not match its package digest.",
            destination.display()
        ));
    }
    Ok(())
}

fn agent_data_directory() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| "Could not determine the installing user's home directory.".to_string())?;
    #[cfg(target_os = "macos")]
    return Ok(home.join("Library/Application Support/MSC 2"));
    #[cfg(target_os = "windows")]
    return Ok(home.join("AppData/Roaming/MSC2"));
    #[cfg(target_os = "linux")]
    return Ok(home.join(".local/share/msc2"));
    #[allow(unreachable_code)]
    Err("This desktop platform has no agent data directory.".to_string())
}

fn installing_user() -> Result<String, String> {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .map_err(|_| "Could not determine the installing user for the service.".to_string())?;
    if user.trim().is_empty() {
        return Err("Could not determine the installing user for the service.".to_string());
    }
    Ok(user)
}

fn report_status(report: ServiceStatusReport) -> AgentServiceStatus {
    let (state, detail) = match report.state {
        ServiceState::NotInstalled => {
            ("not-installed", "The local agent service is not installed.")
        }
        ServiceState::Stopped => (
            "stopped",
            "The local agent service is installed but stopped.",
        ),
        ServiceState::Running => (
            "running",
            "The local agent service is running independently of this window.",
        ),
    };
    AgentServiceStatus {
        available: true,
        platform: std::env::consts::OS,
        service_name: AGENT_SERVICE_NAME,
        state,
        pid: report.pid,
        detail: detail.to_string(),
    }
}

fn report_status_for_packaged_agent(
    report: ServiceStatusReport,
    expected_binary: &Path,
) -> AgentServiceStatus {
    if report.state != ServiceState::NotInstalled
        && !service_report_uses_binary(&report, expected_binary)
    {
        return AgentServiceStatus {
            available: true,
            platform: std::env::consts::OS,
            service_name: AGENT_SERVICE_NAME,
            state: "unavailable",
            pid: report.pid,
            detail: "The installed agent belongs to a different desktop build. Repair service before connecting."
                .to_string(),
        };
    }
    report_status(report)
}

fn service_report_uses_binary(report: &ServiceStatusReport, expected_binary: &Path) -> bool {
    report
        .definition
        .as_ref()
        .is_some_and(|definition| definition.binary_path == expected_binary)
}

fn ensure_service_report_uses_binary(
    report: ServiceStatusReport,
    expected_binary: &Path,
) -> Result<ServiceStatusReport, String> {
    if service_report_uses_binary(&report, expected_binary) {
        return Ok(report);
    }
    Err(format!(
        "Repair completed, but the installed agent does not use the refreshed binary at {}.",
        expected_binary.display()
    ))
}

fn current_local_agent_report() -> Result<(ServiceStatusReport, PathBuf), String> {
    let expected_binary = staged_packaged_agent_path()?;
    let report = service_manager()?
        .execute(ServiceManagerCommand::Status {
            service_name: ServiceName::new(AGENT_SERVICE_NAME),
        })
        .map_err(|error| error.to_string())?;
    Ok((report, expected_binary))
}

fn ensure_installed_agent_matches_package() -> Result<(), String> {
    let (report, expected_binary) = current_local_agent_report()?;
    if service_report_uses_binary(&report, &expected_binary) {
        return Ok(());
    }
    Err(
        "The installed agent belongs to a different desktop build. Repair service before starting it."
            .to_string(),
    )
}

fn ensure_current_local_agent_service() -> Result<(), String> {
    let (report, expected_binary) = current_local_agent_report()?;
    if report.state == ServiceState::Running
        && service_report_uses_binary(&report, &expected_binary)
    {
        return Ok(());
    }
    Err(
        "The current packaged agent is not the running service. Open Agent and choose Repair service before connecting."
            .to_string(),
    )
}

fn credential_key(agent_host_id: &str) -> String {
    format!("{DESKTOP_CREDENTIAL_KEY_PREFIX}{agent_host_id}")
}

fn canonical_base_url(value: &str) -> Result<String, String> {
    let mut url =
        Url::parse(value).map_err(|_| "The agent address is not a valid URL.".to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("The agent address must be an HTTP(S) origin without credentials.".to_string());
    }
    url.set_path("");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.as_str().trim_end_matches('/').to_string())
}

fn relative_request_url(base_url: &str, path: &str) -> Result<Url, String> {
    if !path.starts_with('/') || path.starts_with("//") || path.contains("..") {
        return Err("Desktop requests must use a safe relative API path.".to_string());
    }
    Url::parse(&format!("{base_url}{path}"))
        .map_err(|_| "The desktop request path is invalid.".to_string())
}

fn desktop_secret_store() -> Result<Box<dyn SecretStore>, String> {
    #[cfg(target_os = "macos")]
    {
        // Same file-rooted store the agent itself uses (msc-platform-macos's
        // secret_store module doc) -- the desktop app and the agent always
        // run as the same regular user, so there is no reason for this,
        // unlike the agent's install-time secrets, to be the one remaining
        // thing in this feature still hitting the login keychain's ACL/
        // session prompt. `system()` self-provisions its root key and shares
        // `agent_data_directory()`'s secrets/ directory with the plain
        // `local-bootstrap.key` file this same process already writes.
        return msc_platform_macos::secret_store::MacosSecretStore::system()
            .map(|store| Box::new(store) as Box<dyn SecretStore>)
            .map_err(|error| error.to_string());
    }
    #[cfg(target_os = "windows")]
    {
        return Ok(Box::new(
            msc_platform_windows::secret_store::WindowsSecretStore::new(),
        ));
    }
    #[cfg(target_os = "linux")]
    {
        return Ok(Box::new(
            msc_platform_linux::secret_store::LinuxSecretStore::new(),
        ));
    }
    #[allow(unreachable_code)]
    Err("No desktop credential store is available for this platform.".to_string())
}

/// Opens a user-approved external link in the host operating system's default
/// browser. HTTPS links are permitted generally; HTTP is restricted to the
/// loopback-only local agent UI, which has no public network hop to protect.
#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    approved_external_url(&url)?;

    #[cfg(target_os = "macos")]
    let status = std::process::Command::new("open").arg(&url).status();
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("cmd")
        .args(["/C", "start", "", &url])
        .status();
    #[cfg(target_os = "linux")]
    let status = std::process::Command::new("xdg-open").arg(&url).status();

    status
        .map_err(|error| format!("Could not open the default browser: {error}"))
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(format!(
                    "The default browser rejected the link (status {status})."
                ))
            }
        })
}

fn approved_external_url(url: &str) -> Result<(), String> {
    let parsed = Url::parse(url).map_err(|_| "External link is not a valid URL.".to_string())?;
    let host = parsed.host_str();
    let is_https = parsed.scheme() == "https" && host.is_some();
    let is_loopback_http =
        parsed.scheme() == "http" && matches!(host, Some("127.0.0.1" | "localhost" | "::1"));
    if !is_https && !is_loopback_http {
        return Err("Only HTTPS links or HTTP loopback addresses may be opened.".to_string());
    }
    Ok(())
}

/// Reveals a local file or folder in the OS file manager (Finder/Explorer/
/// the desktop Linux file manager `xdg-open` hands off to). Mirrors
/// `open_external_url`'s shape -- one platform-dispatched shell-out -- but
/// for a local filesystem path instead of an HTTPS URL. Only meaningful for
/// a locally-connected agent: `path` names something on this same machine,
/// so the client must never call this for a remote host's server files.
#[tauri::command]
fn reveal_in_file_manager(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("That path no longer exists on this machine.".to_string());
    }

    reveal_command(&path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not open the file manager: {error}"))
}

#[cfg(target_os = "macos")]
fn reveal_command(path: &Path) -> std::process::Command {
    let mut command = std::process::Command::new("open");
    command.arg("-R").arg(path);
    command
}

#[cfg(target_os = "windows")]
fn reveal_command(path: &Path) -> std::process::Command {
    let mut command = std::process::Command::new("explorer");
    command.arg(format!("/select,{}", path.display()));
    command
}

#[cfg(target_os = "linux")]
fn reveal_command(path: &Path) -> std::process::Command {
    // xdg-open has no "select this item" concept -- it can only open a
    // directory, so a file target falls back to revealing its parent.
    let target = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf())
    };
    let mut command = std::process::Command::new("xdg-open");
    command.arg(target);
    command
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "macos")]
    ensure_ad_hoc_signed_or_reexec();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            desktop_exchange_pairing,
            desktop_bootstrap_local,
            desktop_forget_credentials,
            desktop_authorized_request,
            open_local_agent_browser,
            open_external_url,
            reveal_in_file_manager,
            agent_service_status,
            agent_health_check,
            quit_app,
            manage_agent_service,
            stage_coordinated_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running the MSC 2 desktop shell");
}

#[cfg(test)]
mod tests {
    use super::*;
    use msc_infrastructure::service::FakeServiceManager;

    #[test]
    fn agent_service_report_never_claims_window_ownership() {
        let manager = FakeServiceManager::new();
        let request = ServiceInstallRequest::new(
            AGENT_SERVICE_NAME,
            "/opt/msc/msc",
            "/tmp/msc",
            "/tmp/msc/agent.log",
            AGENT_PORT,
        )
        .run_user("owner");
        manager
            .execute(ServiceManagerCommand::Install(request))
            .expect("synthetic service installs");
        let report = manager
            .execute(ServiceManagerCommand::Start {
                service_name: ServiceName::new(AGENT_SERVICE_NAME),
            })
            .expect("synthetic service starts");

        let status = report_status(report);
        assert_eq!(status.state, "running");
        assert!(status.detail.contains("independently of this window"));
    }

    #[test]
    fn packaged_agent_mismatch_is_repair_required() {
        let request = ServiceInstallRequest::new(
            AGENT_SERVICE_NAME,
            "/old-build/msc",
            "/tmp/msc",
            "/tmp/msc/agent.log",
            AGENT_PORT,
        );
        let status = report_status_for_packaged_agent(
            ServiceStatusReport::running(request, 42),
            Path::new("/current-build/msc"),
        );

        assert_eq!(status.state, "unavailable");
        assert_eq!(status.pid, Some(42));
        assert!(status.detail.contains("Repair service"));
    }

    #[test]
    fn packaged_agent_match_remains_running() {
        let request = ServiceInstallRequest::new(
            AGENT_SERVICE_NAME,
            "/current-build/msc",
            "/tmp/msc",
            "/tmp/msc/agent.log",
            AGENT_PORT,
        );
        let status = report_status_for_packaged_agent(
            ServiceStatusReport::running(request, 42),
            Path::new("/current-build/msc"),
        );

        assert_eq!(status.state, "running");
        assert_eq!(status.pid, Some(42));
    }

    #[test]
    fn packaged_agent_path_is_staged_once_per_process() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let cache = PackagedAgentPathCache::new();
        let stage_count = AtomicUsize::new(0);
        let stage = || {
            stage_count.fetch_add(1, Ordering::Relaxed);
            Ok(PathBuf::from("/current-build/msc"))
        };

        assert_eq!(
            cache.resolve(stage).unwrap(),
            Path::new("/current-build/msc")
        );
        assert_eq!(
            cache.resolve(stage).unwrap(),
            Path::new("/current-build/msc")
        );
        assert_eq!(stage_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn repair_refreshes_a_path_cached_before_a_rebuild() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let cache = PackagedAgentPathCache::new();
        let stage_count = AtomicUsize::new(0);
        let old_stage = || {
            stage_count.fetch_add(1, Ordering::Relaxed);
            Ok(PathBuf::from("/old-build/msc"))
        };
        let new_stage = || {
            stage_count.fetch_add(1, Ordering::Relaxed);
            Ok(PathBuf::from("/new-build/msc"))
        };

        assert_eq!(cache.resolve(old_stage).unwrap(), Path::new("/old-build/msc"));
        assert_eq!(
            cache.refresh(new_stage).unwrap(),
            Path::new("/new-build/msc")
        );
        assert_eq!(
            cache.resolve(|| Ok(PathBuf::from("/unexpected-build/msc")))
                .unwrap(),
            Path::new("/new-build/msc")
        );
        assert_eq!(stage_count.load(Ordering::Relaxed), 2);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn designated_requirement_accepts_macos_stdout_shape() {
        assert_eq!(
            parse_designated_requirement(
                "# designated => cdhash H\"abc123\"\n",
                "Executable=/Applications/MSC 2.app/Contents/MacOS/msc2-desktop-web\n",
            ),
            Some("cdhash H\"abc123\"")
        );
    }

    #[test]
    fn external_url_policy_allows_https_and_local_agent_loopback_only() {
        assert!(approved_external_url("https://docs.example.test/guide").is_ok());
        assert!(approved_external_url("http://127.0.0.1:48001").is_ok());
        assert!(approved_external_url("http://localhost:48001").is_ok());
        assert!(approved_external_url("http://192.168.1.10:48001").is_err());
        assert!(approved_external_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn local_browser_handoff_keeps_the_pairing_out_of_the_http_url() {
        let url = local_browser_handoff_url("pair_one-time_value").unwrap();
        let parsed = Url::parse(&url).unwrap();

        assert_eq!(
            url,
            "http://127.0.0.1:48001/#browser-pairing=pair_one-time_value"
        );
        assert_eq!(parsed.path(), "/");
        assert!(parsed.query().is_none());
        assert!(local_browser_handoff_url("pair_one/time").is_err());
    }

    #[test]
    fn local_browser_handoff_refreshes_only_a_rejected_desktop_credential() {
        assert!(browser_pairing_needs_local_credential_refresh(401));
        assert!(!browser_pairing_needs_local_credential_refresh(201));
        assert!(!browser_pairing_needs_local_credential_refresh(403));
    }
}
