//! Support state for the local agent and first connection.
//!
//! Service actions use the existing cross-platform service managers. Pairing
//! exchanges use the existing desktop-pairing route and return a client that
//! lives only in the current TUI process; no token is written or displayed.

use crossterm::event::KeyCode;
use msc_api::dto::{CapabilitiesResponseDto, MeResponseDto, PermissionCategoryDto};
use msc_infrastructure::service::{
    ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceName, ServiceState,
    ServiceStatusReport,
};
#[cfg(target_os = "macos")]
use rand::RngCore;
use serde::{Deserialize, Serialize};

use super::transport::SharedClient;
use crate::cli::CliError;

const AGENT_SERVICE_NAME: &str = "com.ctemple.msc2.agent";
const AGENT_PORT: u16 = 48001;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentSurface {
    #[default]
    Status,
    Pairing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentServiceAction {
    Install,
    Start,
    Stop,
    Reconnect,
    Repair,
}

impl AgentServiceAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Reconnect => "reconnect",
            Self::Repair => "repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentIntent {
    Service(AgentServiceAction),
    BeginPairing,
    ExchangePairing(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalServiceState {
    NotInstalled,
    Stopped,
    Running,
    Unavailable,
}

impl LocalServiceState {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotInstalled => "NOT INSTALLED",
            Self::Stopped => "STOPPED",
            Self::Running => "RUNNING",
            Self::Unavailable => "UNAVAILABLE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAgentServiceStatus {
    pub platform: String,
    pub service_name: String,
    pub state: LocalServiceState,
    pub pid: Option<u32>,
    pub detail: String,
}

impl Default for LocalAgentServiceStatus {
    fn default() -> Self {
        Self {
            platform: std::env::consts::OS.to_string(),
            service_name: AGENT_SERVICE_NAME.to_string(),
            state: LocalServiceState::Unavailable,
            pid: None,
            detail: "Local service status has not been checked.".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairingCreateResult {
    pairing_code: String,
    agent_host_id: String,
    expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopPairingResult {
    agent_host_id: String,
    credential_id: String,
    token: String,
    expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PairingCreateRequest {
    client_kind: String,
    label: String,
    role: String,
    permissions: Vec<PermissionCategoryDto>,
    expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopPairingRequest {
    pairing_code: String,
}

#[derive(Debug, Clone)]
pub struct PairingExchange {
    pub client: SharedClient,
    pub agent_host_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct AgentState {
    pub surface: AgentSurface,
    pub identity: Option<MeResponseDto>,
    pub capabilities: Option<CapabilitiesResponseDto>,
    pub service: LocalAgentServiceStatus,
    pub pairing_code: Option<String>,
    pub pairing_host_id: Option<String>,
    pub pairing_expires_at: Option<String>,
    pub pairing_input: Option<String>,
    pub loaded: bool,
    pub error: Option<String>,
    pub status: Option<String>,
}

impl AgentState {
    pub async fn load(client: &SharedClient) -> Result<Self, CliError> {
        let identity = client.get_json("/v1/me").await?;
        let capabilities = client.get_json("/v1/capabilities").await?;
        Ok(Self {
            identity: Some(identity),
            capabilities: Some(capabilities),
            service: local_service_status(),
            loaded: true,
            ..Self::default()
        })
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<AgentIntent> {
        if let Some(mut value) = self.pairing_input.take() {
            return match key {
                KeyCode::Esc => None,
                KeyCode::Backspace => {
                    value.pop();
                    self.pairing_input = Some(value);
                    None
                }
                KeyCode::Char(character) => {
                    value.push(character);
                    self.pairing_input = Some(value);
                    None
                }
                KeyCode::Enter if !value.trim().is_empty() => {
                    Some(AgentIntent::ExchangePairing(value.trim().to_string()))
                }
                _ => {
                    self.pairing_input = Some(value);
                    None
                }
            };
        }

        match key {
            KeyCode::Char('1') => self.surface = AgentSurface::Status,
            KeyCode::Char('2') => self.surface = AgentSurface::Pairing,
            KeyCode::Char('p') if self.surface == AgentSurface::Pairing => {
                self.pairing_input = Some(String::new());
            }
            KeyCode::Char('c') if self.surface == AgentSurface::Pairing => {
                return Some(AgentIntent::BeginPairing);
            }
            KeyCode::Char('I') => return Some(AgentIntent::Service(AgentServiceAction::Install)),
            KeyCode::Char('S') => return Some(AgentIntent::Service(AgentServiceAction::Start)),
            KeyCode::Char('X') => return Some(AgentIntent::Service(AgentServiceAction::Stop)),
            KeyCode::Char('R') => {
                return Some(AgentIntent::Service(AgentServiceAction::Reconnect));
            }
            KeyCode::Char('F') => return Some(AgentIntent::Service(AgentServiceAction::Repair)),
            KeyCode::Char('r') => self.loaded = false,
            _ => {}
        }
        None
    }

    pub async fn create_pairing(&mut self, client: &SharedClient) -> Result<(), CliError> {
        let permissions = self
            .identity
            .as_ref()
            .map(|identity| identity.permissions.clone())
            .unwrap_or_default();
        let result: PairingCreateResult = client
            .post_json(
                "/v1/auth/pairings",
                &PairingCreateRequest {
                    client_kind: "desktop".to_string(),
                    label: "tui-session".to_string(),
                    role: "admin".to_string(),
                    permissions,
                    expires_at: None,
                },
            )
            .await?;
        self.pairing_code = Some(result.pairing_code);
        self.pairing_host_id = Some(result.agent_host_id);
        self.pairing_expires_at = Some(result.expires_at);
        self.status = Some("One-use desktop pairing code created.".to_string());
        Ok(())
    }

    pub async fn exchange_pairing(
        &mut self,
        client: &SharedClient,
        pairing_code: String,
    ) -> Result<PairingExchange, CliError> {
        let result: DesktopPairingResult = client
            .post_json(
                "/v1/auth/desktop-pairings",
                &DesktopPairingRequest { pairing_code },
            )
            .await?;
        // Keeping these fields private to this module makes it impossible for
        // rendering code to accidentally echo the newly issued credential.
        let _credential_id = result.credential_id;
        let _expires_at = result.expires_at;
        self.pairing_input = None;
        self.pairing_code = None;
        self.pairing_host_id = Some(result.agent_host_id.clone());
        self.status = Some("Pairing exchanged; this host session is memory-only.".to_string());
        Ok(PairingExchange {
            client: client.with_token(result.token),
            agent_host_id: result.agent_host_id,
        })
    }

    pub fn execute_service(action: AgentServiceAction) -> Result<LocalAgentServiceStatus, String> {
        let service_name = ServiceName::new(AGENT_SERVICE_NAME);
        #[cfg(target_os = "macos")]
        {
            let report = match action {
                AgentServiceAction::Install | AgentServiceAction::Repair => {
                    msc_platform_macos::service::install_and_start_elevated(install_request()?)
                        .map_err(|error| error.to_string())?
                }
                AgentServiceAction::Start => {
                    msc_platform_macos::service::start_elevated(service_name.as_str())
                        .map_err(|error| error.to_string())?
                }
                AgentServiceAction::Stop => {
                    msc_platform_macos::service::stop_elevated(service_name.as_str())
                        .map_err(|error| error.to_string())?
                }
                AgentServiceAction::Reconnect => {
                    let manager = local_service_manager()?;
                    manager
                        .execute(ServiceManagerCommand::Start { service_name })
                        .map_err(|error| error.to_string())?
                }
            };
            return Ok(status_from_report(report));
        }
        #[cfg(not(target_os = "macos"))]
        {
            let manager = local_service_manager()?;
            let report = match action {
                AgentServiceAction::Install | AgentServiceAction::Repair => {
                    manager
                        .execute(ServiceManagerCommand::Install(install_request()?))
                        .map_err(|error| error.to_string())?;
                    manager
                        .execute(ServiceManagerCommand::Start { service_name })
                        .map_err(|error| error.to_string())?
                }
                AgentServiceAction::Start | AgentServiceAction::Reconnect => manager
                    .execute(ServiceManagerCommand::Start { service_name })
                    .map_err(|error| error.to_string())?,
                AgentServiceAction::Stop => manager
                    .execute(ServiceManagerCommand::Stop { service_name })
                    .map_err(|error| error.to_string())?,
            };
            Ok(status_from_report(report))
        }
    }
}

fn status_from_report(report: ServiceStatusReport) -> LocalAgentServiceStatus {
    let state = match report.state {
        ServiceState::NotInstalled => LocalServiceState::NotInstalled,
        ServiceState::Stopped => LocalServiceState::Stopped,
        ServiceState::Running => LocalServiceState::Running,
    };
    let detail = match state {
        LocalServiceState::NotInstalled => "The local agent service is not installed.",
        LocalServiceState::Stopped => "The local agent service is installed but stopped.",
        LocalServiceState::Running => {
            "The local agent service is running independently of this TUI."
        }
        LocalServiceState::Unavailable => "The local agent service is unavailable.",
    };
    LocalAgentServiceStatus {
        platform: std::env::consts::OS.to_string(),
        service_name: report.service_name.as_str().to_string(),
        state,
        pid: report.pid,
        detail: detail.to_string(),
    }
}

pub(crate) fn local_service_status() -> LocalAgentServiceStatus {
    let service_name = ServiceName::new(AGENT_SERVICE_NAME);
    match local_service_manager().and_then(|manager| {
        manager
            .execute(ServiceManagerCommand::Status { service_name })
            .map_err(|error| error.to_string())
    }) {
        Ok(report) => status_from_report(report),
        Err(error) => LocalAgentServiceStatus {
            detail: format!("Local service status unavailable: {error}"),
            ..LocalAgentServiceStatus::default()
        },
    }
}

fn install_request() -> Result<ServiceInstallRequest, String> {
    let binary =
        std::env::current_exe().map_err(|error| format!("locating msc binary: {error}"))?;
    let data_dir = agent_data_dir()?;
    std::fs::create_dir_all(data_dir.join("logs"))
        .map_err(|error| format!("creating agent log directory: {error}"))?;
    #[cfg(target_os = "macos")]
    {
        let secrets_dir = data_dir.join("secrets");
        std::fs::create_dir_all(&secrets_dir)
            .map_err(|error| format!("creating agent secret directory: {error}"))?;
        ensure_local_bootstrap_key(&secrets_dir)?;
    }
    let mut request = ServiceInstallRequest::new(
        AGENT_SERVICE_NAME,
        binary,
        &data_dir,
        data_dir.join("logs/agent.log"),
        AGENT_PORT,
    )
    .args(["serve", "--bind", "127.0.0.1:48001"])
    .env("MSC2_DATA_DIR", data_dir.display().to_string());
    if let Some(home) = std::env::var_os("HOME") {
        request = request.env("HOME", home.to_string_lossy().into_owned());
    }
    #[cfg(target_os = "macos")]
    {
        let secrets_dir = data_dir.join("secrets");
        request = request
            .env(
                "MSC2_MACOS_SECRET_STORE_DIR",
                secrets_dir.display().to_string(),
            )
            .env(
                "MSC2_LOCAL_BOOTSTRAP_SOCKET",
                data_dir.join("bootstrap.sock").display().to_string(),
            );
    }
    if let Some(user) = std::env::var_os("USER").or_else(|| std::env::var_os("USERNAME")) {
        if !user.is_empty() {
            request = request.run_user(user.to_string_lossy());
        }
    }
    Ok(request)
}

fn agent_data_dir() -> Result<std::path::PathBuf, String> {
    if let Some(path) = std::env::var_os("MSC2_DATA_DIR") {
        return Ok(path.into());
    }
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| "HOME is not set; cannot locate the agent data directory".to_string())?;
    #[cfg(target_os = "macos")]
    return Ok(home.join("Library/Application Support/MSC 2"));
    #[cfg(target_os = "windows")]
    return Ok(home.join("AppData/Roaming/MSC2"));
    #[cfg(target_os = "linux")]
    return Ok(home.join(".local/share/msc2"));
    #[allow(unreachable_code)]
    Err("this platform has no default agent data directory".to_string())
}

#[cfg(target_os = "macos")]
fn ensure_local_bootstrap_key(secrets_dir: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let path = msc_platform_macos::service::local_bootstrap_key_path(secrets_dir);
    if path.is_file() {
        return Ok(());
    }
    let mut bytes = [0_u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let mut key = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        key.push_str(&format!("{byte:02x}"));
    }
    std::fs::write(&path, key).map_err(|error| format!("writing bootstrap key: {error}"))?;
    let mut permissions = std::fs::metadata(&path)
        .map_err(|error| format!("reading bootstrap key permissions: {error}"))?
        .permissions();
    permissions.set_mode(0o600);
    std::fs::set_permissions(path, permissions)
        .map_err(|error| format!("restricting bootstrap key permissions: {error}"))
}

fn local_service_manager() -> Result<Box<dyn ServiceManager>, String> {
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
    Err("this platform has no MSC service manager".to_string())
}
