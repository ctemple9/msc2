//! Production Bedrock runtime selection.
//!
//! Host and resource detection belongs at agent composition time. Keeping it
//! here means HTTP handlers only report the selected result; they do not need
//! to know how a native process or a macOS sidecar is built.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use msc_api::dto::{BedrockBackendDto, BedrockRuntimeStateDto, HostOsDto};
use msc_application::bedrock_runtime::{
    BedrockHost, BedrockProvisionRequest, BedrockRuntime, BedrockRuntimeBackend,
    BedrockRuntimeEligibility, BedrockRuntimeEligibilityState, BedrockRuntimeError,
    BedrockRuntimeEvent, BedrockRuntimePaths, BedrockRuntimeState, BedrockSidecarResources,
    BedrockStartRequest,
};
use msc_domain::identity::ServerType;
use msc_infrastructure::bedrock_distribution::BedrockPlatform;

use super::lifecycle::AgentAppConfigStore;

#[cfg(target_os = "linux")]
use msc_application::bedrock_linux::{LinuxBedrockRuntime, SystemBedrockRuntimeClock};
#[cfg(target_os = "macos")]
use msc_application::bedrock_macos::{MacosBedrockRuntime, SidecarProcessTransport};
#[cfg(target_os = "windows")]
use msc_application::bedrock_windows::{SystemBedrockRuntimeClock, WindowsBedrockRuntime};
#[cfg(target_os = "linux")]
use msc_infrastructure::bedrock_native::LinuxBedrockProcessSupervisor;
use msc_infrastructure::process::ProcessSupervisor;
#[cfg(target_os = "macos")]
use msc_platform_macos::process::MacosJavaProcessSupervisor;
#[cfg(target_os = "windows")]
use msc_platform_windows::process::WindowsJavaProcessSupervisor;

enum BedrockRuntimeHandle {
    #[cfg(target_os = "linux")]
    Linux(Box<LinuxBedrockRuntime<'static>>),
    #[cfg(target_os = "windows")]
    Windows(Box<WindowsBedrockRuntime<'static>>),
    #[cfg(target_os = "macos")]
    Macos(Box<MacosBedrockRuntime<SidecarProcessTransport<'static>>>),
    Unavailable,
}

impl BedrockRuntimeHandle {
    fn state(&self) -> BedrockRuntimeState {
        match self {
            #[cfg(target_os = "linux")]
            Self::Linux(runtime) => runtime.state(),
            #[cfg(target_os = "windows")]
            Self::Windows(runtime) => runtime.state(),
            #[cfg(target_os = "macos")]
            Self::Macos(runtime) => runtime.state(),
            Self::Unavailable => BedrockRuntimeState::Unavailable,
        }
    }

    fn provision(&mut self, request: BedrockProvisionRequest) -> Result<(), BedrockRuntimeError> {
        match self {
            #[cfg(target_os = "linux")]
            Self::Linux(runtime) => runtime.provision(request),
            #[cfg(target_os = "windows")]
            Self::Windows(runtime) => runtime.provision(request),
            #[cfg(target_os = "macos")]
            Self::Macos(runtime) => runtime.provision(request),
            Self::Unavailable => Err(BedrockRuntimeError::Transport(
                "Bedrock runtime is unavailable.".to_owned(),
            )),
        }
    }

    fn start(&mut self, request: BedrockStartRequest) -> Result<(), BedrockRuntimeError> {
        match self {
            #[cfg(target_os = "linux")]
            Self::Linux(runtime) => runtime.start(request),
            #[cfg(target_os = "windows")]
            Self::Windows(runtime) => runtime.start(request),
            #[cfg(target_os = "macos")]
            Self::Macos(runtime) => runtime.start(request),
            Self::Unavailable => Err(BedrockRuntimeError::Transport(
                "Bedrock runtime is unavailable.".to_owned(),
            )),
        }
    }

    fn stop(&mut self) -> Result<(), BedrockRuntimeError> {
        match self {
            #[cfg(target_os = "linux")]
            Self::Linux(runtime) => runtime.stop(),
            #[cfg(target_os = "windows")]
            Self::Windows(runtime) => runtime.stop(),
            #[cfg(target_os = "macos")]
            Self::Macos(runtime) => runtime.stop(),
            Self::Unavailable => Err(BedrockRuntimeError::Transport(
                "Bedrock runtime is unavailable.".to_owned(),
            )),
        }
    }

    fn command(&mut self, command: &str) -> Result<(), BedrockRuntimeError> {
        match self {
            #[cfg(target_os = "linux")]
            Self::Linux(runtime) => runtime.command(command),
            #[cfg(target_os = "windows")]
            Self::Windows(runtime) => runtime.command(command),
            #[cfg(target_os = "macos")]
            Self::Macos(runtime) => runtime.command(command),
            Self::Unavailable => Err(BedrockRuntimeError::Transport(
                "Bedrock runtime is unavailable.".to_owned(),
            )),
        }
    }

    fn poll_event(&mut self) -> Result<Option<BedrockRuntimeEvent>, BedrockRuntimeError> {
        match self {
            #[cfg(target_os = "linux")]
            Self::Linux(runtime) => runtime.poll_event(),
            #[cfg(target_os = "windows")]
            Self::Windows(runtime) => runtime.poll_event(),
            #[cfg(target_os = "macos")]
            Self::Macos(runtime) => runtime.poll_event(),
            Self::Unavailable => Ok(None),
        }
    }

    fn process_id(&self) -> Option<msc_infrastructure::process::ProcessId> {
        match self {
            #[cfg(target_os = "linux")]
            Self::Linux(runtime) => runtime.process_id(),
            #[cfg(target_os = "windows")]
            Self::Windows(runtime) => runtime.process_id(),
            #[cfg(target_os = "macos")]
            Self::Macos(runtime) => runtime.process_id(),
            Self::Unavailable => None,
        }
    }

    fn refresh_eligibility(&mut self, eligibility: BedrockRuntimeEligibility) {
        match self {
            #[cfg(target_os = "linux")]
            Self::Linux(runtime) => runtime.refresh_eligibility(eligibility),
            #[cfg(target_os = "windows")]
            Self::Windows(runtime) => runtime.refresh_eligibility(eligibility),
            #[cfg(target_os = "macos")]
            Self::Macos(runtime) => runtime.refresh_eligibility(eligibility),
            Self::Unavailable => {}
        }
    }

    fn is_selected(&self) -> bool {
        !matches!(self, Self::Unavailable)
    }
}

#[derive(Clone)]
pub struct BedrockRuntimeSelection {
    eligibility: Arc<Mutex<BedrockRuntimeEligibility>>,
    runtime: Arc<Mutex<BedrockRuntimeHandle>>,
    bound_server_dir: Arc<Mutex<Option<PathBuf>>>,
}

impl BedrockRuntimeSelection {
    /// Build the one runtime selected by the real agent composition root.
    /// Native adapters can exist in a provisioning-required state; the
    /// macOS adapter is spawned only after all sidecar prerequisites pass.
    pub fn production(app_config: &AgentAppConfigStore) -> Self {
        let servers = app_config.servers();
        let server_dir = app_config
            .active_server_id()
            .and_then(|active_id| {
                servers
                    .iter()
                    .find(|server| {
                        server.id == active_id && server.server_type == ServerType::Bedrock
                    })
                    .map(|server| PathBuf::from(&server.server_dir))
            })
            .or_else(|| {
                servers
                    .iter()
                    .find(|server| server.server_type == ServerType::Bedrock)
                    .map(|server| PathBuf::from(&server.server_dir))
            })
            .unwrap_or_else(|| app_config.servers_root());
        let paths = runtime_paths(server_dir);
        let eligibility =
            BedrockRuntimeEligibility::detect(&msc_infrastructure::fs::StdFileSystem, &paths);

        match BedrockHost::current() {
            #[cfg(target_os = "linux")]
            BedrockHost::Linux => {
                let supervisor: &'static dyn ProcessSupervisor =
                    Box::leak(Box::new(LinuxBedrockProcessSupervisor::new()));
                let runtime = LinuxBedrockRuntime::with_eligibility(
                    supervisor,
                    eligibility.clone(),
                    SystemBedrockRuntimeClock,
                );
                Self::new(eligibility, BedrockRuntimeHandle::Linux(Box::new(runtime)))
            }
            #[cfg(target_os = "windows")]
            BedrockHost::Windows => {
                let supervisor: &'static dyn ProcessSupervisor =
                    Box::leak(Box::new(WindowsJavaProcessSupervisor::new()));
                let runtime = WindowsBedrockRuntime::with_eligibility(
                    supervisor,
                    eligibility.clone(),
                    SystemBedrockRuntimeClock,
                );
                Self::new(
                    eligibility,
                    BedrockRuntimeHandle::Windows(Box::new(runtime)),
                )
            }
            #[cfg(target_os = "macos")]
            BedrockHost::MacosIntel => {
                if eligibility.state == BedrockRuntimeEligibilityState::Unavailable
                    || eligibility
                        .reason_code
                        .as_deref()
                        .is_some_and(|reason| reason.starts_with("sidecar_"))
                {
                    return Self::new(eligibility, BedrockRuntimeHandle::Unavailable);
                }

                let Some(resources) = paths.sidecar.as_ref() else {
                    return Self::new(
                        unavailable_eligibility(
                            BedrockHost::MacosIntel,
                            BedrockRuntimeBackend::Sidecar,
                            "sidecar_resources_required",
                            "Bedrock sidecar resources are not installed.",
                        ),
                        BedrockRuntimeHandle::Unavailable,
                    );
                };
                let supervisor: &'static dyn ProcessSupervisor =
                    Box::leak(Box::new(MacosJavaProcessSupervisor::new()));
                let working_directory = resources
                    .executable
                    .parent()
                    .unwrap_or_else(|| Path::new("."));
                match SidecarProcessTransport::spawn(
                    supervisor,
                    resources.executable.clone(),
                    working_directory,
                ) {
                    Ok(transport) => {
                        let runtime =
                            MacosBedrockRuntime::with_eligibility(transport, eligibility.clone());
                        Self::new(eligibility, BedrockRuntimeHandle::Macos(Box::new(runtime)))
                    }
                    Err(error) => Self::new(
                        unavailable_eligibility(
                            BedrockHost::MacosIntel,
                            BedrockRuntimeBackend::Sidecar,
                            "sidecar_start_failed",
                            format!("Bedrock sidecar could not start: {error}"),
                        ),
                        BedrockRuntimeHandle::Unavailable,
                    ),
                }
            }
            #[cfg(target_os = "macos")]
            BedrockHost::MacosAppleSilicon => {
                Self::new(eligibility, BedrockRuntimeHandle::Unavailable)
            }
            BedrockHost::Other => Self::new(eligibility, BedrockRuntimeHandle::Unavailable),
            _ => Self::new(eligibility, BedrockRuntimeHandle::Unavailable),
        }
    }

    pub fn unavailable_for_tests() -> Self {
        Self::new(
            BedrockRuntimeEligibility {
                host: BedrockHost::Other,
                backend: None,
                state: BedrockRuntimeEligibilityState::Unavailable,
                reason_code: Some("test_runtime_not_selected".to_owned()),
                message: "No production Bedrock runtime was selected.".to_owned(),
            },
            BedrockRuntimeHandle::Unavailable,
        )
    }

    pub fn state_dto(&self) -> BedrockRuntimeStateDto {
        let eligibility = self.eligibility.lock().unwrap().clone();
        let runtime_state = self.runtime.lock().unwrap().state();
        let state = if eligibility.state == BedrockRuntimeEligibilityState::Available
            && runtime_state == BedrockRuntimeState::Unavailable
        {
            "unavailable"
        } else {
            match eligibility.state {
                BedrockRuntimeEligibilityState::Available => "available",
                BedrockRuntimeEligibilityState::ProvisioningRequired => "provisioning_required",
                BedrockRuntimeEligibilityState::Unavailable => "unavailable",
            }
        };
        let unavailable = state == "unavailable";
        BedrockRuntimeStateDto {
            state: state.to_owned(),
            backend: eligibility.backend.map(backend_dto),
            host_os: match eligibility.host.host_os() {
                "linux" => Some(HostOsDto::Linux),
                "windows" => Some(HostOsDto::Windows),
                "macos" => Some(HostOsDto::Macos),
                _ => None,
            },
            reason_code: eligibility.reason_code,
            message: Some(eligibility.message),
            help_id: unavailable.then(|| "bedrock.runtime-unavailable".to_owned()),
        }
    }

    pub fn state(&self) -> BedrockRuntimeState {
        self.runtime.lock().unwrap().state()
    }

    /// Rebind the shared eligibility snapshot to the server that is about to
    /// become active. The runtime itself is shared by all Bedrock routes, so
    /// leaving the startup server's filesystem result in place would let one
    /// server borrow another server's readiness claim.
    pub fn refresh_for_server(&self, server_dir: impl AsRef<Path>) {
        let host = self.eligibility.lock().unwrap().host;
        let refreshed = BedrockRuntimeEligibility::for_host(
            &msc_infrastructure::fs::StdFileSystem,
            host,
            &runtime_paths(server_dir.as_ref().to_path_buf()),
        );
        {
            let mut runtime = self.runtime.lock().unwrap();
            runtime.refresh_eligibility(refreshed.clone());
        }
        *self.eligibility.lock().unwrap() = refreshed;
    }

    pub fn is_bound(&self) -> bool {
        matches!(
            self.state(),
            BedrockRuntimeState::Provisioned
                | BedrockRuntimeState::Starting
                | BedrockRuntimeState::Running
                | BedrockRuntimeState::Stopping
        )
    }

    pub fn is_bound_to_other_server(&self, server_dir: impl AsRef<Path>) -> bool {
        self.is_bound()
            && self
                .bound_server_dir
                .lock()
                .unwrap()
                .as_deref()
                .is_some_and(|bound| bound != server_dir.as_ref())
    }

    /// Download and verify the BDS distribution for a server without binding
    /// the lifecycle adapter to it. Imports can prepare several records while
    /// the one shared adapter remains free for the eventual active server.
    pub fn ensure_distribution(
        &self,
        request: BedrockProvisionRequest,
        pre_downgrade_backup: impl FnOnce() -> bool,
    ) -> Result<(), BedrockRuntimeError> {
        let server_dir = PathBuf::from(&request.server_dir);
        self.refresh_for_server(&server_dir);
        let eligibility = self.eligibility.lock().unwrap().clone();
        let platform = distribution_platform(&eligibility).ok_or_else(|| {
            BedrockRuntimeError::Provisioning("no Bedrock distribution platform is selected".into())
        })?;
        if !self.runtime.lock().unwrap().is_selected() {
            return Err(BedrockRuntimeError::Transport(
                "Bedrock runtime is unavailable.".to_owned(),
            ));
        }

        let manifest_url = msc_application::bedrock_provisioning::production_manifest_url();
        let provisioning_request = msc_application::bedrock_provisioning::ProvisionRequest {
            server_dir: &server_dir,
            version: Some(&request.version),
            platform,
            force: false,
            manifest_url: &manifest_url,
        };
        msc_application::bedrock_provisioning::ensure_installed(
            &msc_infrastructure::fs::StdFileSystem,
            &msc_infrastructure::jar_provider::HttpTransport::new(),
            &provisioning_request,
            pre_downgrade_backup,
        )
        .map_err(|error| BedrockRuntimeError::Provisioning(error.to_string()))?;

        let refreshed = BedrockRuntimeEligibility::for_host(
            &msc_infrastructure::fs::StdFileSystem,
            eligibility.host,
            &runtime_paths(server_dir),
        );
        if refreshed.state != BedrockRuntimeEligibilityState::Available {
            return Err(BedrockRuntimeError::Provisioning(refreshed.message));
        }
        {
            let mut runtime = self.runtime.lock().unwrap();
            runtime.refresh_eligibility(refreshed.clone());
        }
        *self.eligibility.lock().unwrap() = refreshed;
        Ok(())
    }

    /// The route layer owns the application operation, while this selection
    /// owns the one mutable backend instance chosen at agent startup.
    pub fn provision(
        &self,
        request: BedrockProvisionRequest,
        pre_downgrade_backup: impl FnOnce() -> bool,
    ) -> Result<(), BedrockRuntimeError> {
        let server_dir = PathBuf::from(&request.server_dir);
        if self.is_bound_to_other_server(&server_dir) {
            return Err(BedrockRuntimeError::InvalidState {
                operation: "provision",
                state: self.state(),
            });
        }
        self.ensure_distribution(request.clone(), pre_downgrade_backup)?;
        if self.state() == BedrockRuntimeState::Provisioned
            && self.bound_server_dir.lock().unwrap().as_deref() == Some(server_dir.as_path())
        {
            return Ok(());
        }
        self.runtime.lock().unwrap().provision(request)?;
        *self.bound_server_dir.lock().unwrap() = Some(server_dir);
        Ok(())
    }

    pub fn start(&self, request: BedrockStartRequest) -> Result<(), BedrockRuntimeError> {
        self.runtime.lock().unwrap().start(request)
    }

    pub fn stop(&self) -> Result<(), BedrockRuntimeError> {
        self.runtime.lock().unwrap().stop()
    }

    pub fn command(&self, command: &str) -> Result<(), BedrockRuntimeError> {
        self.runtime.lock().unwrap().command(command)
    }

    pub fn poll_event(&self) -> Result<Option<BedrockRuntimeEvent>, BedrockRuntimeError> {
        self.runtime.lock().unwrap().poll_event()
    }

    pub fn process_id(&self) -> Option<msc_infrastructure::process::ProcessId> {
        self.runtime.lock().unwrap().process_id()
    }

    fn new(eligibility: BedrockRuntimeEligibility, runtime: BedrockRuntimeHandle) -> Self {
        Self {
            eligibility: Arc::new(Mutex::new(eligibility)),
            runtime: Arc::new(Mutex::new(runtime)),
            bound_server_dir: Arc::new(Mutex::new(None)),
        }
    }
}

fn distribution_platform(eligibility: &BedrockRuntimeEligibility) -> Option<BedrockPlatform> {
    match (eligibility.host, eligibility.backend) {
        (BedrockHost::Linux, Some(BedrockRuntimeBackend::Native)) => Some(BedrockPlatform::Linux),
        (BedrockHost::Windows, Some(BedrockRuntimeBackend::Native)) => {
            Some(BedrockPlatform::Windows)
        }
        (BedrockHost::MacosIntel, Some(BedrockRuntimeBackend::Sidecar)) => {
            Some(BedrockPlatform::Linux)
        }
        _ => None,
    }
}

fn backend_dto(backend: BedrockRuntimeBackend) -> BedrockBackendDto {
    match backend {
        BedrockRuntimeBackend::Native => BedrockBackendDto::Native,
        BedrockRuntimeBackend::Sidecar => BedrockBackendDto::VzSidecar,
    }
}

#[cfg(target_os = "macos")]
fn unavailable_eligibility(
    host: BedrockHost,
    backend: BedrockRuntimeBackend,
    reason_code: &str,
    message: impl Into<String>,
) -> BedrockRuntimeEligibility {
    BedrockRuntimeEligibility {
        host,
        backend: Some(backend),
        state: BedrockRuntimeEligibilityState::Unavailable,
        reason_code: Some(reason_code.to_owned()),
        message: message.into(),
    }
}

fn runtime_paths(server_dir: PathBuf) -> BedrockRuntimePaths {
    BedrockRuntimePaths {
        server_dir,
        sidecar: sidecar_resources(),
    }
}

#[cfg(target_os = "macos")]
fn sidecar_resources() -> Option<BedrockSidecarResources> {
    let root = std::env::var_os("MSC2_BEDROCK_SIDECAR_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok()?.parent().map(Path::to_owned))?;
    Some(BedrockSidecarResources {
        executable: root.join("BedrockSidecar"),
        kernel: root.join("vmlinuz-kata"),
        initramfs: root.join("appliance-initramfs.gz"),
    })
}

#[cfg(not(target_os = "macos"))]
fn sidecar_resources() -> Option<BedrockSidecarResources> {
    None
}
