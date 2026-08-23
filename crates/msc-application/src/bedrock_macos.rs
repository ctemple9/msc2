//! macOS Bedrock runtime backed by the Swift Virtualization sidecar.
//!
//! Rust owns process supervision, JSON-lines framing, and the shared runtime
//! state. The Swift executable owns `Virtualization.framework`; this module
//! deliberately has no VM types or guest boot logic.

use crate::bedrock_runtime::{
    BedrockProvisionRequest, BedrockRuntime, BedrockRuntimeBackend, BedrockRuntimeCapabilities,
    BedrockRuntimeError, BedrockRuntimeEvent, BedrockRuntimeState, BedrockStartRequest,
    SidecarReceive, SidecarRuntime, SidecarTransport,
};
use msc_infrastructure::bedrock_sidecar::{
    BedrockSidecarProcess, SidecarReceive as ProcessSidecarReceive,
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

    fn capabilities(self) -> BedrockRuntimeCapabilities {
        match self {
            Self::Intel => BedrockRuntimeCapabilities::supported(BedrockRuntimeBackend::Sidecar),
            Self::AppleSilicon => BedrockRuntimeCapabilities::unavailable(
                BedrockRuntimeBackend::Sidecar,
                "apple-silicon-unavailable-no-test-hardware",
            ),
            Self::Other => BedrockRuntimeCapabilities::unavailable(
                BedrockRuntimeBackend::Sidecar,
                "macos-bedrock-sidecar-requires-macos",
            ),
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

pub struct MacosBedrockRuntime<T> {
    inner: SidecarRuntime<T>,
    capabilities: BedrockRuntimeCapabilities,
}

impl<T: SidecarTransport> MacosBedrockRuntime<T> {
    pub fn with_transport(transport: T, host: MacosBedrockHost) -> Self {
        Self {
            inner: SidecarRuntime::new(transport),
            capabilities: host.capabilities(),
        }
    }

    pub fn sidecar(&self) -> &SidecarRuntime<T> {
        &self.inner
    }

    pub fn sidecar_mut(&mut self) -> &mut SidecarRuntime<T> {
        &mut self.inner
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
        let transport =
            SidecarProcessTransport::spawn(process_supervisor, executable_path, working_directory)
                .map_err(|error| BedrockRuntimeError::Transport(error.to_string()))?;
        Ok(Self::with_transport(transport, MacosBedrockHost::current()))
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
