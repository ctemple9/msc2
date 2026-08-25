#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use msc_infrastructure::secret_store::SecretStore;
use msc_infrastructure::service::{
    ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceName, ServiceState,
    ServiceStatusReport,
};
use reqwest::{header, Method, Url};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

mod update;

const DESKTOP_CREDENTIAL_KEY_PREFIX: &str = "msc.desktop.host-token.";
const DESKTOP_SECRET_SERVICE: &str = "com.ctemple.msc2.desktop";
const LOCAL_HOST_ID_KEY: &str = "msc.desktop.local-agent-host-id";
const AGENT_SERVICE_NAME: &str = "com.ctemple.msc2.agent";
const AGENT_PORT: u16 = 48001;
const LOCAL_BOOTSTRAP_SOCKET: &str = "bootstrap.sock";
const PROTOCOL_VERSION: u32 = 1;
const PROOF_DOMAIN: &[u8] = b"msc2-local-bootstrap-v1\0";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum AgentServiceAction {
    Install,
    Start,
    Stop,
    Repair,
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
struct DesktopCredentialResult {
    agent_host_id: String,
    credential_id: String,
    token: String,
    expires_at: Option<String>,
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
    #[cfg(target_os = "macos")]
    {
        return bootstrap_local_macos();
    }
    #[allow(unreachable_code)]
    Err("Local desktop bootstrap is unavailable on this platform.".to_string())
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
        .write_all(
            format!(
                "{}\n",
                serde_json::json!({ "version": PROTOCOL_VERSION })
            )
            .as_bytes(),
        )
        .map_err(|error| format!("Could not send the bootstrap hello: {error}"))?;
    let mut reader = BufReader::new(stream.try_clone().map_err(|error| {
        format!("Could not read the bootstrap channel: {error}")
    })?);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|error| format!("Could not read the bootstrap challenge: {error}"))?;
    let challenge: LocalBootstrapChallenge = serde_json::from_str(&line)
        .map_err(|error| format!("The local agent returned an invalid bootstrap challenge: {error}"))?;
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
    let response: LocalBootstrapResponse = serde_json::from_str(&line)
        .map_err(|error| format!("The local agent returned an invalid bootstrap result: {error}"))?;
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

#[cfg(target_os = "macos")]
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

/// Reports the service separately from the browser's connection state. It does
/// not start anything: opening or closing the desktop shell is never a server
/// lifecycle action.
#[tauri::command]
fn agent_service_status() -> Result<AgentServiceStatus, String> {
    service_manager()?
        .execute(ServiceManagerCommand::Status {
            service_name: ServiceName::new(AGENT_SERVICE_NAME),
        })
        .map(report_status)
        .map_err(|error| error.to_string())
}

/// Checks the pre-auth health route from the native shell. A webview fetch is
/// cross-origin and can be blocked by browser policy before credentials exist.
#[tauri::command]
async fn agent_health_check() -> bool {
    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    else {
        return false;
    };
    client
        .get("http://127.0.0.1:48001/v1/health")
        .send()
        .await
        .is_ok_and(|response| response.status().is_success())
}

/// The shared setup screen has this narrow native seam for an explicit local
/// service action. Platform registration may trigger the OS elevation flow;
/// routine agent operation remains under the installing user's account.
#[tauri::command]
fn manage_agent_service(action: AgentServiceAction) -> Result<AgentServiceStatus, String> {
    let service_name = ServiceName::new(AGENT_SERVICE_NAME);
    #[cfg(target_os = "macos")]
    let report = match action {
        AgentServiceAction::Install | AgentServiceAction::Repair => {
            msc_platform_macos::service::install_and_start_elevated(agent_install_request()?)
                .map_err(|error| error.to_string())?
        }
        AgentServiceAction::Start => {
            msc_platform_macos::service::start_elevated(service_name.as_str())
                .map_err(|error| error.to_string())?
        }
        AgentServiceAction::Stop => service_manager()?
            .execute(ServiceManagerCommand::Stop { service_name })
            .map_err(|error| error.to_string())?,
    };
    #[cfg(not(target_os = "macos"))]
    let report = {
        let manager = service_manager()?;
        match action {
            AgentServiceAction::Install | AgentServiceAction::Repair => {
                let request = agent_install_request()?;
                manager
                    .execute(ServiceManagerCommand::Install(request))
                    .map_err(|error| error.to_string())?;
                manager
                    .execute(ServiceManagerCommand::Start { service_name })
                    .map_err(|error| error.to_string())?
            }
            AgentServiceAction::Start => manager
                .execute(ServiceManagerCommand::Start { service_name })
                .map_err(|error| error.to_string())?,
            AgentServiceAction::Stop => manager
                .execute(ServiceManagerCommand::Stop { service_name })
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
    let binary_path = packaged_agent_path()?;
    if !binary_path.is_file() {
        return Err(format!(
            "The compatible agent package is missing at {}. Reinstall this desktop app before registering the service.",
            binary_path.display()
        ));
    }
    let working_directory = agent_data_directory()?;
    let secret_store_directory = working_directory.join("secrets");
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
        working_directory.join(LOCAL_BOOTSTRAP_SOCKET).display().to_string(),
    );
    #[cfg(target_os = "macos")]
    let request = request.env("MSC2_MACOS_DESKTOP_REQUIREMENT", desktop_requirement);
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
    return Ok(directory.join("../lib/msc2/agent/msc"));
    #[allow(unreachable_code)]
    Err("This desktop platform has no agent-package layout.".to_string())
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
        return msc_platform_macos::secret_store::MacosSecretStore::default_keychain_for_service(
            DESKTOP_SECRET_SERVICE,
        )
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

/// Opens a user-approved HTTPS link in the host operating system's default
/// browser. Tauri's webview does not reliably hand external anchors to the OS,
/// so this command keeps setup links working on every desktop platform.
#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    let parsed = Url::parse(&url).map_err(|_| "External link is not a valid URL.".to_string())?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() {
        return Err("Only HTTPS links with a host may be opened.".to_string());
    }

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
            desktop_authorized_request,
            open_external_url,
            agent_service_status,
            agent_health_check,
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
}
