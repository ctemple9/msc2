//! macOS Bedrock runtime backed by the Swift Virtualization sidecar.
//!
//! Rust owns process supervision, JSON-lines framing, and the shared runtime
//! state. The Swift executable owns `Virtualization.framework`; this module
//! deliberately has no VM types or guest boot logic.

use crate::bedrock_runtime::{
    BedrockHost, BedrockProvisionRequest, BedrockRuntime, BedrockRuntimeBackend,
    BedrockRuntimeCapabilities, BedrockRuntimeDiagnostics, BedrockRuntimeEligibility,
    BedrockRuntimeError, BedrockRuntimeEvent, BedrockRuntimePaths, BedrockRuntimeState,
    BedrockSidecarResources, BedrockStartRequest, SidecarReceive, SidecarRuntime, SidecarTransport,
};
use msc_infrastructure::bedrock_sidecar::{
    BedrockSidecarProcess, SidecarReceive as ProcessSidecarReceive,
};
#[cfg(target_os = "macos")]
use msc_infrastructure::bedrock_sidecar::{
    BedrockSidecarSocket, BedrockSidecarSocketError, SidecarReceive as SocketSidecarReceive,
};
use msc_infrastructure::process::{ProcessError, ProcessId, ProcessSupervisor};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacosBedrockHost {
    Intel,
    AppleSilicon,
    Other,
}

impl MacosBedrockHost {
    pub const fn current() -> Self {
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            Self::Intel
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Self::AppleSilicon
        }
        #[cfg(not(any(
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64")
        )))]
        {
            Self::Other
        }
    }
}

pub struct SidecarProcessTransport<'supervisor> {
    process: BedrockSidecarProcess<'supervisor>,
}

impl<'supervisor> SidecarProcessTransport<'supervisor> {
    pub fn spawn(
        process_supervisor: &'supervisor dyn ProcessSupervisor,
        executable_path: impl Into<PathBuf>,
        working_directory: impl Into<PathBuf>,
    ) -> Result<Self, ProcessError> {
        Ok(Self {
            process: BedrockSidecarProcess::spawn(
                process_supervisor,
                executable_path,
                working_directory,
            )?,
        })
    }

    pub fn from_process(process: BedrockSidecarProcess<'supervisor>) -> Self {
        Self { process }
    }

    pub fn process_id(&self) -> ProcessId {
        self.process.process_id()
    }

    pub fn force_terminate(&mut self) -> Result<(), ProcessError> {
        self.process.force_terminate()
    }
}

impl SidecarTransport for SidecarProcessTransport<'_> {
    fn send_line(&mut self, line: &str) -> Result<(), String> {
        self.process
            .send_line(line)
            .map_err(|error| error.to_string())
    }

    fn receive_line(&mut self) -> Result<Option<String>, String> {
        match self.receive_status()? {
            SidecarReceive::Line(line) => Ok(Some(line)),
            SidecarReceive::Pending | SidecarReceive::Eof => Ok(None),
        }
    }

    fn receive_status(&mut self) -> Result<SidecarReceive, String> {
        self.process
            .receive()
            .map(|status| match status {
                ProcessSidecarReceive::Line(line) => SidecarReceive::Line(line),
                ProcessSidecarReceive::Pending => SidecarReceive::Pending,
                ProcessSidecarReceive::Eof => SidecarReceive::Eof,
            })
            .map_err(|error| error.to_string())
    }
}

/// The installed macOS agent uses this transport to reach the root-owned
/// helper. It deliberately exposes the same line-based surface as the
/// development child-process transport above, so lifecycle ordering remains
/// owned by [`SidecarRuntime`] rather than duplicated per transport.
#[cfg(target_os = "macos")]
pub struct SidecarSocketTransport {
    socket: BedrockSidecarSocket,
}

#[cfg(target_os = "macos")]
impl SidecarSocketTransport {
    pub fn connect(socket_path: impl Into<PathBuf>) -> Result<Self, BedrockSidecarSocketError> {
        BedrockSidecarSocket::connect(socket_path).map(|socket| Self { socket })
    }
}

#[cfg(target_os = "macos")]
impl SidecarTransport for SidecarSocketTransport {
    fn send_line(&mut self, line: &str) -> Result<(), String> {
        self.socket.send_line(line)
    }

    fn receive_line(&mut self) -> Result<Option<String>, String> {
        match self.receive_status()? {
            SidecarReceive::Line(line) => Ok(Some(line)),
            SidecarReceive::Pending | SidecarReceive::Eof => Ok(None),
        }
    }

    fn receive_status(&mut self) -> Result<SidecarReceive, String> {
        self.socket.receive().map(|status| match status {
            SocketSidecarReceive::Line(line) => SidecarReceive::Line(line),
            SocketSidecarReceive::Pending => SidecarReceive::Pending,
            SocketSidecarReceive::Eof => SidecarReceive::Eof,
        })
    }
}

/// A single macOS runtime can be backed by either the installed helper or an
/// explicitly selected development child process. Keeping this enum at the
/// adapter boundary avoids making the protocol runtime aware of OS transport
/// details.
#[cfg(target_os = "macos")]
pub enum MacosBedrockTransport<'supervisor> {
    Helper(SidecarSocketTransport),
    Development(SidecarProcessTransport<'supervisor>),
}

#[cfg(target_os = "macos")]
impl SidecarTransport for MacosBedrockTransport<'_> {
    fn send_line(&mut self, line: &str) -> Result<(), String> {
        match self {
            Self::Helper(transport) => transport.send_line(line),
            Self::Development(transport) => transport.send_line(line),
        }
    }

    fn receive_line(&mut self) -> Result<Option<String>, String> {
        match self {
            Self::Helper(transport) => transport.receive_line(),
            Self::Development(transport) => transport.receive_line(),
        }
    }

    fn receive_status(&mut self) -> Result<SidecarReceive, String> {
        match self {
            Self::Helper(transport) => transport.receive_status(),
            Self::Development(transport) => transport.receive_status(),
        }
    }
}

pub struct MacosBedrockRuntime<T> {
    inner: SidecarRuntime<T>,
    capabilities: BedrockRuntimeCapabilities,
}

impl<T: SidecarTransport> MacosBedrockRuntime<T> {
    pub fn with_transport(transport: T, host: MacosBedrockHost) -> Self {
        let eligibility = match host {
            MacosBedrockHost::Intel => BedrockRuntimeEligibility::synthetic_available(
                BedrockHost::MacosIntel,
                BedrockRuntimeBackend::Sidecar,
            ),
            MacosBedrockHost::AppleSilicon => BedrockRuntimeEligibility {
                host: BedrockHost::MacosAppleSilicon,
                backend: None,
                state: crate::bedrock_runtime::BedrockRuntimeEligibilityState::Unavailable,
                reason_code: Some("apple-silicon-unavailable-no-test-hardware".to_owned()),
                message: "Bedrock is unavailable on Apple Silicon under D-028.".to_owned(),
            },
            MacosBedrockHost::Other => BedrockRuntimeEligibility {
                host: BedrockHost::Other,
                backend: None,
                state: crate::bedrock_runtime::BedrockRuntimeEligibilityState::Unavailable,
                reason_code: Some("macos-bedrock-sidecar-requires-macos".to_owned()),
                message: "Bedrock has no macOS sidecar backend for this host.".to_owned(),
            },
        };
        Self::with_eligibility(transport, eligibility)
    }

    pub fn with_eligibility(transport: T, eligibility: BedrockRuntimeEligibility) -> Self {
        Self {
            inner: SidecarRuntime::new(transport),
            capabilities: eligibility.capabilities_for(BedrockRuntimeBackend::Sidecar),
        }
    }

    pub fn refresh_eligibility(&mut self, eligibility: BedrockRuntimeEligibility) {
        self.capabilities = eligibility.capabilities_for(BedrockRuntimeBackend::Sidecar);
    }

    pub fn sidecar(&self) -> &SidecarRuntime<T> {
        &self.inner
    }

    pub fn sidecar_mut(&mut self) -> &mut SidecarRuntime<T> {
        &mut self.inner
    }

    pub fn diagnostics(&self) -> Option<&BedrockRuntimeDiagnostics> {
        self.inner.diagnostics()
    }

    fn require_supported(&self) -> Result<(), BedrockRuntimeError> {
        if self.capabilities.supported {
            Ok(())
        } else {
            Err(BedrockRuntimeError::Transport(
                self.capabilities
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "macos-bedrock-sidecar-unavailable".to_owned()),
            ))
        }
    }
}

impl<'supervisor> MacosBedrockRuntime<SidecarProcessTransport<'supervisor>> {
    pub fn spawn(
        process_supervisor: &'supervisor dyn ProcessSupervisor,
        executable_path: impl Into<PathBuf>,
        working_directory: impl Into<PathBuf>,
    ) -> Result<Self, BedrockRuntimeError> {
        let executable_path = executable_path.into();
        let working_directory = working_directory.into();
        let transport = SidecarProcessTransport::spawn(
            process_supervisor,
            &executable_path,
            &working_directory,
        )
        .map_err(|error| BedrockRuntimeError::Transport(error.to_string()))?;
        let eligibility = BedrockRuntimeEligibility::detect(
            &msc_infrastructure::fs::StdFileSystem,
            &BedrockRuntimePaths {
                server_dir: working_directory.clone(),
                sidecar: Some(BedrockSidecarResources {
                    executable: executable_path,
                    kernel: working_directory.join("vmlinuz-kata"),
                    initramfs: working_directory.join("appliance-initramfs.gz"),
                }),
            },
        );
        Ok(Self::with_eligibility(transport, eligibility))
    }

    pub fn spawn_from_paths(
        process_supervisor: &'supervisor dyn ProcessSupervisor,
        executable_path: &Path,
        working_directory: &Path,
    ) -> Result<Self, BedrockRuntimeError> {
        Self::spawn(
            process_supervisor,
            executable_path.to_owned(),
            working_directory.to_owned(),
        )
    }

    pub fn sidecar_process_id(&self) -> ProcessId {
        self.inner.transport().process_id()
    }
}

impl<T: SidecarTransport> BedrockRuntime for MacosBedrockRuntime<T> {
    fn capabilities(&self) -> &BedrockRuntimeCapabilities {
        &self.capabilities
    }

    fn state(&self) -> BedrockRuntimeState {
        if self.capabilities.supported {
            self.inner.state()
        } else {
            BedrockRuntimeState::Unavailable
        }
    }

    fn provision(&mut self, request: BedrockProvisionRequest) -> Result<(), BedrockRuntimeError> {
        self.require_supported()?;
        self.inner.provision(request)
    }

    fn start(&mut self, request: BedrockStartRequest) -> Result<(), BedrockRuntimeError> {
        self.require_supported()?;
        self.inner.start(request)
    }

    fn stop(&mut self) -> Result<(), BedrockRuntimeError> {
        self.require_supported()?;
        self.inner.stop()
    }

    fn force_stop(&mut self) -> Result<(), BedrockRuntimeError> {
        self.require_supported()?;
        self.inner.force_stop()
    }

    fn command(&mut self, command: &str) -> Result<(), BedrockRuntimeError> {
        self.require_supported()?;
        self.inner.command(command)
    }

    fn poll_event(&mut self) -> Result<Option<BedrockRuntimeEvent>, BedrockRuntimeError> {
        self.require_supported()?;
        self.inner.poll_event()
    }
}
