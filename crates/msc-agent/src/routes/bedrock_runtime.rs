//! Production Bedrock runtime selection.
//!
//! Host and resource detection belongs at agent composition time. Keeping it
//! here means HTTP handlers only report the selected result; they do not need
//! to know how a native process or a macOS sidecar is built.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use msc_api::dto::{BedrockBackendDto, BedrockRuntimeStateDto, HostOsDto};
use msc_application::bedrock_runtime::{
    BedrockHost, BedrockRuntime, BedrockRuntimeBackend, BedrockRuntimeEligibility,
    BedrockRuntimeEligibilityState, BedrockRuntimePaths, BedrockRuntimeState,
    BedrockSidecarResources,
};
use msc_domain::identity::ServerType;

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
}

#[derive(Clone)]
pub struct BedrockRuntimeSelection {
    eligibility: BedrockRuntimeEligibility,
    runtime: Arc<Mutex<BedrockRuntimeHandle>>,
}

impl BedrockRuntimeSelection {
    /// Build the one runtime selected by the real agent composition root.
    /// Native adapters can exist in a provisioning-required state; the
    /// macOS adapter is spawned only after all sidecar prerequisites pass.
    pub fn production(app_config: &AgentAppConfigStore) -> Self {
        let server_dir = app_config
            .servers()
            .into_iter()
            .find(|server| server.server_type == ServerType::Bedrock)
            .map(|server| PathBuf::from(server.server_dir))
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
                if eligibility.state != BedrockRuntimeEligibilityState::Available {
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
        let runtime_state = self.runtime.lock().unwrap().state();
        let state = if self.eligibility.state == BedrockRuntimeEligibilityState::Available
            && runtime_state == BedrockRuntimeState::Unavailable
        {
            "unavailable"
        } else {
            match self.eligibility.state {
                BedrockRuntimeEligibilityState::Available => "available",
                BedrockRuntimeEligibilityState::ProvisioningRequired => "provisioning_required",
                BedrockRuntimeEligibilityState::Unavailable => "unavailable",
            }
        };
        let unavailable = state == "unavailable";
        BedrockRuntimeStateDto {
            state: state.to_owned(),
            backend: self.eligibility.backend.map(backend_dto),
            host_os: match self.eligibility.host.host_os() {
                "linux" => Some(HostOsDto::Linux),
                "windows" => Some(HostOsDto::Windows),
                "macos" => Some(HostOsDto::Macos),
                _ => None,
            },
            reason_code: self.eligibility.reason_code.clone(),
            message: Some(self.eligibility.message.clone()),
            help_id: unavailable.then(|| "bedrock.runtime-unavailable".to_owned()),
        }
    }

    fn new(eligibility: BedrockRuntimeEligibility, runtime: BedrockRuntimeHandle) -> Self {
        Self {
            eligibility,
            runtime: Arc::new(Mutex::new(runtime)),
        }
    }
}

fn backend_dto(backend: BedrockRuntimeBackend) -> BedrockBackendDto {
    match backend {
        BedrockRuntimeBackend::Native => BedrockBackendDto::Native,
        BedrockRuntimeBackend::Sidecar => BedrockBackendDto::VzSidecar,
    }
}

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
