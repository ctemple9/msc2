//! Native Linux Bedrock Dedicated Server runtime.
//!
//! This adapter owns only the native-process side of the boundary.  Verified
//! distribution staging happens in `bedrock_provisioning`; this module starts
//! the already-staged `bedrock_server`, frames its output, and reports the
//! same lifecycle events used by the macOS sidecar.

use crate::bedrock_runtime::{
    BedrockProvisionRequest, BedrockRuntime, BedrockRuntimeBackend, BedrockRuntimeCapabilities,
    BedrockRuntimeEligibility, BedrockRuntimeError, BedrockRuntimeEvent, BedrockRuntimePaths,
    BedrockRuntimeState, BedrockStartRequest, BedrockTerminationReason,
};
use msc_infrastructure::bedrock_native::{self, LinuxBedrockProcessSupervisor, NativeBedrockHost};
use msc_infrastructure::process::{
    OutputStreamLineFramer, ProcessEvent, ProcessId, ProcessSpawnRequest, ProcessSupervisor,
};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const GRACEFUL_STOP_TIMEOUT: Duration = Duration::from_secs(20);

pub trait BedrockRuntimeClock {
    fn now(&self) -> Instant;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemBedrockRuntimeClock;

impl BedrockRuntimeClock for SystemBedrockRuntimeClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// The Linux runtime can be constructed on any host for capability reporting,
/// but only a Linux host is allowed to claim a native BDS implementation.
pub struct LinuxBedrockRuntime<'supervisor, C = SystemBedrockRuntimeClock> {
    process_supervisor: &'supervisor dyn ProcessSupervisor,
    clock: C,
    capabilities: BedrockRuntimeCapabilities,
    state: BedrockRuntimeState,
    server_dir: Option<PathBuf>,
    bedrock_port: Option<u16>,
    process: Option<ProcessId>,
    output_framer: OutputStreamLineFramer,
    pending_events: VecDeque<BedrockRuntimeEvent>,
    graceful_stop_at: Option<Instant>,
    force_stop_sent: bool,
    ready_reported: bool,
}

impl<'supervisor> LinuxBedrockRuntime<'supervisor> {
    pub fn new(process_supervisor: &'supervisor dyn ProcessSupervisor) -> Self {
        let paths = BedrockRuntimePaths {
            server_dir: PathBuf::new(),
            sidecar: None,
        };
        Self::with_eligibility(
            process_supervisor,
            BedrockRuntimeEligibility::detect(&msc_infrastructure::fs::StdFileSystem, &paths),
            SystemBedrockRuntimeClock,
        )
    }
}

impl<'supervisor, C: BedrockRuntimeClock> LinuxBedrockRuntime<'supervisor, C> {
    pub fn with_host(
        process_supervisor: &'supervisor dyn ProcessSupervisor,
        host: NativeBedrockHost,
        clock: C,
    ) -> Self {
        let eligibility = if host == NativeBedrockHost::Linux {
            BedrockRuntimeEligibility::synthetic_available(
                crate::bedrock_runtime::BedrockHost::Linux,
                BedrockRuntimeBackend::Native,
            )
        } else {
            BedrockRuntimeEligibility::for_host(
                &msc_infrastructure::fs::FakeFileSystem::new(),
                crate::bedrock_runtime::BedrockHost::Other,
                &BedrockRuntimePaths {
                    server_dir: PathBuf::new(),
                    sidecar: None,
                },
            )
        };
        Self::with_eligibility(process_supervisor, eligibility, clock)
    }

    pub fn with_eligibility(
        process_supervisor: &'supervisor dyn ProcessSupervisor,
        eligibility: BedrockRuntimeEligibility,
        clock: C,
    ) -> Self {
        let supported =
            eligibility.state == crate::bedrock_runtime::BedrockRuntimeEligibilityState::Available;
        let state = if supported {
            BedrockRuntimeState::New
        } else {
            BedrockRuntimeState::Unavailable
        };
        let capabilities = eligibility.capabilities_for(BedrockRuntimeBackend::Native);
        Self {
            process_supervisor,
            clock,
            capabilities,
            state,
            server_dir: None,
            bedrock_port: None,
            process: None,
            output_framer: OutputStreamLineFramer::new(),
            pending_events: VecDeque::new(),
            graceful_stop_at: None,
            force_stop_sent: false,
            ready_reported: false,
        }
    }

    pub fn process_id(&self) -> Option<ProcessId> {
        self.process
    }

    pub fn launch_request(&self) -> Option<ProcessSpawnRequest> {
        self.server_dir
            .as_deref()
            .map(bedrock_native::linux_bedrock_spawn_request)
    }

    fn require_supported(&self) -> Result<(), BedrockRuntimeError> {
        if self.capabilities.supported {
            Ok(())
        } else {
            Err(BedrockRuntimeError::Transport(
                self.capabilities
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "no-supported-bedrock-backend".to_owned()),
            ))
        }
    }

    fn require_state(
        &self,
        operation: &'static str,
        allowed: &[BedrockRuntimeState],
    ) -> Result<(), BedrockRuntimeError> {
        if allowed.contains(&self.state) {
            Ok(())
        } else {
            Err(BedrockRuntimeError::InvalidState {
                operation,
                state: self.state,
            })
        }
    }

    fn enqueue_line(&mut self, line: String) {
        let ready = !self.ready_reported && line.to_ascii_lowercase().contains("server started");
        self.pending_events
            .push_back(BedrockRuntimeEvent::ConsoleLine(line));
        if ready {
            self.ready_reported = true;
            self.state = BedrockRuntimeState::Running;
            self.pending_events.push_back(BedrockRuntimeEvent::Ready {
                address: None,
                port: self.bedrock_port.unwrap_or(0),
            });
        }
    }

    fn maybe_force_stop(&mut self) -> Result<(), BedrockRuntimeError> {
        let Some(requested_at) = self.graceful_stop_at else {
            return Ok(());
        };
        if self.force_stop_sent
            || self
                .clock
                .now()
                .checked_duration_since(requested_at)
                .is_none_or(|elapsed| elapsed < GRACEFUL_STOP_TIMEOUT)
        {
            return Ok(());
        }
        let pid = self.process.ok_or(BedrockRuntimeError::InvalidState {
            operation: "force-stop",
            state: self.state,
        })?;
        self.process_supervisor
            .force_terminate(pid)
            .map_err(|error| BedrockRuntimeError::Transport(error.to_string()))?;
        self.force_stop_sent = true;
        Ok(())
    }

    fn enqueue_process_events(
        &mut self,
        events: Vec<ProcessEvent>,
    ) -> Result<(), BedrockRuntimeError> {
        for event in events {
            match event {
                ProcessEvent::Output { stream, bytes } => {
                    for line in self.output_framer.push(stream, &bytes) {
                        self.enqueue_line(line);
                    }
                }
                ProcessEvent::Exited(status) => {
                    for (_, line) in self.output_framer.flush() {
                        self.enqueue_line(line);
                    }
                    let reason = if self.graceful_stop_at.is_some() {
                        BedrockTerminationReason::Clean
                    } else if self.ready_reported {
                        BedrockTerminationReason::GuestError(
                            "native Bedrock process exited unexpectedly".to_owned(),
                        )
                    } else {
                        BedrockTerminationReason::StartFailed(format!(
                            "native Bedrock process exited with {}",
                            status.code.map_or_else(
                                || "a signal".to_owned(),
                                |code| format!("exit code {code}"),
                            )
                        ))
                    };
                    self.process = None;
                    self.state = BedrockRuntimeState::Stopped;
                    self.pending_events
                        .push_back(BedrockRuntimeEvent::Terminated { reason });
                }
            }
        }
        Ok(())
    }
}

impl<C: BedrockRuntimeClock> BedrockRuntime for LinuxBedrockRuntime<'_, C> {
    fn capabilities(&self) -> &BedrockRuntimeCapabilities {
        &self.capabilities
    }

    fn state(&self) -> BedrockRuntimeState {
        self.state
    }

    fn process_id(&self) -> Option<ProcessId> {
        self.process_id()
    }

    fn provision(&mut self, request: BedrockProvisionRequest) -> Result<(), BedrockRuntimeError> {
        self.require_supported()?;
        self.require_state(
            "provision",
            &[BedrockRuntimeState::New, BedrockRuntimeState::Stopped],
        )?;
        self.server_dir = Some(PathBuf::from(request.server_dir));
        self.pending_events.clear();
        self.graceful_stop_at = None;
        self.force_stop_sent = false;
        self.ready_reported = false;
        self.state = BedrockRuntimeState::Provisioned;
        Ok(())
    }

    fn start(&mut self, request: BedrockStartRequest) -> Result<(), BedrockRuntimeError> {
        self.require_supported()?;
        self.require_state("start", &[BedrockRuntimeState::Provisioned])?;
        let server_dir = self.server_dir.as_deref().ok_or_else(|| {
            BedrockRuntimeError::Transport("native Bedrock server directory is missing".to_owned())
        })?;
        bedrock_native::preflight_udp_bind(
            bedrock_native::BEDROCK_BIND_ADDRESS,
            request.bedrock_port,
        )
        .map_err(|error| BedrockRuntimeError::Transport(error.to_string()))?;
        let pid = self
            .process_supervisor
            .spawn(bedrock_native::linux_bedrock_spawn_request(server_dir))
            .map_err(|error| BedrockRuntimeError::Transport(error.to_string()))?;
        let _ = request.memory_gb;
        self.process = Some(pid);
        self.bedrock_port = Some(request.bedrock_port);
        self.state = BedrockRuntimeState::Starting;
        self.output_framer = OutputStreamLineFramer::new();
        self.pending_events.clear();
        self.graceful_stop_at = None;
        self.force_stop_sent = false;
        self.ready_reported = false;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), BedrockRuntimeError> {
        self.require_state(
            "stop",
            &[BedrockRuntimeState::Starting, BedrockRuntimeState::Running],
        )?;
        let pid = self.process.ok_or(BedrockRuntimeError::InvalidState {
            operation: "stop",
            state: self.state,
        })?;
        self.process_supervisor
            .request_graceful_stop(pid)
            .map_err(|error| BedrockRuntimeError::Transport(error.to_string()))?;
        self.graceful_stop_at = Some(self.clock.now());
        self.state = BedrockRuntimeState::Stopping;
        Ok(())
    }

    fn force_stop(&mut self) -> Result<(), BedrockRuntimeError> {
        self.require_state(
            "force-stop",
            &[
                BedrockRuntimeState::Starting,
                BedrockRuntimeState::Running,
                BedrockRuntimeState::Stopping,
            ],
        )?;
        let pid = self.process.ok_or(BedrockRuntimeError::InvalidState {
            operation: "force-stop",
            state: self.state,
        })?;
        self.process_supervisor
            .force_terminate(pid)
            .map_err(|error| BedrockRuntimeError::Transport(error.to_string()))?;
        self.force_stop_sent = true;
        self.state = BedrockRuntimeState::Stopping;
        Ok(())
    }

    fn command(&mut self, command: &str) -> Result<(), BedrockRuntimeError> {
        self.require_state("command", &[BedrockRuntimeState::Running])?;
        let pid = self.process.ok_or(BedrockRuntimeError::InvalidState {
            operation: "command",
            state: self.state,
        })?;
        let command = command.trim().trim_start_matches('/');
        self.process_supervisor
            .write_stdin(pid, crate::commands::stdin_payload(command).as_slice())
            .map_err(|error| BedrockRuntimeError::Transport(error.to_string()))
    }

    fn poll_event(&mut self) -> Result<Option<BedrockRuntimeEvent>, BedrockRuntimeError> {
        if let Some(event) = self.pending_events.pop_front() {
            return Ok(Some(event));
        }
        self.maybe_force_stop()?;
        let Some(pid) = self.process else {
            return Ok(None);
        };
        let events = self
            .process_supervisor
            .drain_events(pid)
            .map_err(|error| BedrockRuntimeError::Transport(error.to_string()))?;
        self.enqueue_process_events(events)?;
        Ok(self.pending_events.pop_front())
    }
}

pub type NativeLinuxBedrockRuntime<'supervisor> = LinuxBedrockRuntime<'supervisor>;
pub type NativeLinuxBedrockProcessSupervisor = LinuxBedrockProcessSupervisor;
