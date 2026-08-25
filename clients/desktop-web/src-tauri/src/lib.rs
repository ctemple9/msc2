#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use msc_infrastructure::secret_store::SecretStore;
use msc_infrastructure::service::{
    ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceName, ServiceState,
    ServiceStatusReport,
};
use reqwest::{header, Method, Url};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod update;

const DESKTOP_CREDENTIAL_KEY_PREFIX: &str = "msc.desktop.host-token.";
const DESKTOP_SECRET_SERVICE: &str = "com.ctemple.msc2.desktop";
const AGENT_SERVICE_NAME: &str = "com.ctemple.msc2.agent";
const AGENT_PORT: u16 = 48001;

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

/// The shared setup screen has this narrow native seam for an explicit local
/// service action. Platform registration may trigger the OS elevation flow;
/// routine agent operation remains under the installing user's account.
#[tauri::command]
fn manage_agent_service(action: AgentServiceAction) -> Result<AgentServiceStatus, String> {
    let manager = service_manager()?;
    let service_name = ServiceName::new(AGENT_SERVICE_NAME);
    let report = match action {
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
    std::fs::create_dir_all(working_directory.join("logs"))
        .map_err(|error| format!("Could not create the agent data directory: {error}"))?;
    Ok(ServiceInstallRequest::new(
        AGENT_SERVICE_NAME,
        binary_path,
        &working_directory,
        working_directory.join("logs/agent.log"),
        AGENT_PORT,
    )
    .args(["serve", "--bind", "127.0.0.1:48001"])
    .run_user(installing_user()?))
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
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            desktop_exchange_pairing,
            desktop_authorized_request,
            open_external_url,
            agent_service_status,
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
}
