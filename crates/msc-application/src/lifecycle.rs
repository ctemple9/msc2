//! Lifecycle application boundary for the Phase 4 Java vertical slice.
//!
//! This crate is where MSC 1's view-model-owned lifecycle behavior starts
//! becoming an application service: it owns server state and calls injected
//! dependencies, but it does not know about HTTP routes, CLI commands, iOS,
//! or any other client surface.

use msc_domain::identity::JavaServerFlavor;
use msc_domain::tps;
use msc_infrastructure::metrics::{ProcessMetricsProvider, directory_size_mb};
use msc_infrastructure::process::{
    ProcessError, ProcessEvent, ProcessId, ProcessSpawnRequest, ProcessSupervisor,
};
use std::fmt;
use std::path::PathBuf;

use crate::output_reducer::{JavaOutputReducer, OutputEvent};
use crate::status::{LifecycleStatusSnapshot, PerformanceSnapshot};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServerId(String);

impl ServerId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Crashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IllegalLifecycleTransition {
    pub from: LifecycleState,
    pub to: LifecycleState,
}

impl LifecycleState {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Crashed => "crashed",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        match raw {
            "stopped" => Some(Self::Stopped),
            "starting" => Some(Self::Starting),
            "running" => Some(Self::Running),
            "stopping" => Some(Self::Stopping),
            "crashed" => Some(Self::Crashed),
            _ => None,
        }
    }

    pub fn process_may_be_alive(self) -> bool {
        matches!(self, Self::Starting | Self::Running | Self::Stopping)
    }

    fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Stopped, Self::Starting)
                | (Self::Crashed, Self::Starting)
                | (Self::Starting, Self::Running)
                | (Self::Starting, Self::Stopping)
                | (Self::Running, Self::Stopping)
                | (Self::Starting, Self::Crashed)
                | (Self::Running, Self::Crashed)
                | (Self::Stopping, Self::Stopped)
        )
    }

    pub fn transition_to(self, next: Self) -> Result<Self, IllegalLifecycleTransition> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(IllegalLifecycleTransition {
                from: self,
                to: next,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedJavaServer {
    pub id: ServerId,
    pub name: String,
    pub directory: PathBuf,
    pub flavor: JavaServerFlavor,
}

impl ImportedJavaServer {
    pub fn paper(
        id: impl Into<String>,
        name: impl Into<String>,
        directory: impl Into<PathBuf>,
    ) -> Self {
        Self {
            id: ServerId::new(id),
            name: name.into(),
            directory: directory.into(),
            flavor: JavaServerFlavor::Paper,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleError {
    NoActiveServer,
    ServerNotFound(ServerId),
    AlreadyInState(LifecycleState),
    ServerNotRunning,
    WrongActiveServer {
        expected: ServerId,
        actual: ServerId,
    },
    IllegalTransition(IllegalLifecycleTransition),
    Repository(String),
    Process(String),
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoActiveServer => write!(f, "no active server selected"),
            Self::ServerNotFound(id) => write!(f, "server not found: {}", id.as_str()),
            Self::AlreadyInState(state) => write!(f, "server is already {}", state.raw_value()),
            Self::ServerNotRunning => write!(f, "server is not running"),
            Self::WrongActiveServer { expected, actual } => write!(
                f,
                "event for server {} does not match active server {}",
                actual.as_str(),
                expected.as_str()
            ),
            Self::IllegalTransition(transition) => write!(
                f,
                "illegal lifecycle transition: {} -> {}",
                transition.from.raw_value(),
                transition.to.raw_value()
            ),
            Self::Repository(message) | Self::Process(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for LifecycleError {}

impl From<ProcessError> for LifecycleError {
    fn from(value: ProcessError) -> Self {
        Self::Process(value.to_string())
    }
}

pub trait JavaServerRepository {
    fn load(&self, id: &ServerId) -> Result<Option<ImportedJavaServer>, LifecycleError>;
}

pub trait ConsoleSink {
    fn append_system_line(&self, server_id: &ServerId, line: &str);
}

pub struct LifecycleService<'deps> {
    repository: &'deps dyn JavaServerRepository,
    process_supervisor: &'deps dyn ProcessSupervisor,
    console: &'deps dyn ConsoleSink,
    active_server: Option<ServerId>,
    active_process: Option<ProcessId>,
    active_ram_max_mb: Option<f64>,
    state: LifecycleState,
    output_reducer: JavaOutputReducer,
    latest_tps: Option<tps::Sample>,
    pending_restart: Option<ProcessSpawnRequest>,
}

impl<'deps> LifecycleService<'deps> {
    pub fn new(
        repository: &'deps dyn JavaServerRepository,
        process_supervisor: &'deps dyn ProcessSupervisor,
        console: &'deps dyn ConsoleSink,
    ) -> Self {
        Self {
            repository,
            process_supervisor,
            console,
            active_server: None,
            active_process: None,
            active_ram_max_mb: None,
            state: LifecycleState::Stopped,
            output_reducer: JavaOutputReducer::new(),
            latest_tps: None,
            pending_restart: None,
        }
    }

    pub fn state(&self) -> LifecycleState {
        self.state
    }

    pub fn active_server(&self) -> Option<&ServerId> {
        self.active_server.as_ref()
    }

    pub fn active_process(&self) -> Option<ProcessId> {
        self.active_process
    }

    pub fn status_snapshot(&self) -> Result<LifecycleStatusSnapshot, LifecycleError> {
        let server = self.load_active_server_if_selected()?;
        Ok(LifecycleStatusSnapshot {
            running: self.state.process_may_be_alive(),
            active_server_id: self
                .active_server
                .as_ref()
                .map(|id| id.as_str().to_string()),
            pid: self.active_process.map(|pid| i64::from(pid.raw())),
            server_type: server.map(|server| server.flavor.raw_value().to_string()),
        })
    }

    pub fn performance_snapshot(
        &self,
        metrics: &dyn ProcessMetricsProvider,
        ts: impl Into<String>,
    ) -> Result<PerformanceSnapshot, LifecycleError> {
        let server = self.load_active_server_if_selected()?;
        let usage = self
            .active_process
            .and_then(|pid| metrics.process_usage(pid));
        let world_size_mb = server
            .as_ref()
            .and_then(|server| directory_size_mb(&server.directory.join("world")).ok());
        let server_type = server.map(|server| server.flavor.raw_value().to_string());

        Ok(PerformanceSnapshot {
            ts: ts.into(),
            tps_1m: self.latest_tps.map(|sample| sample.t1),
            players_online: Some(self.output_reducer.online_players().len() as i64),
            cpu_percent: usage.and_then(|usage| usage.cpu_percent),
            ram_used_mb: usage.and_then(|usage| usage.ram_used_mb),
            ram_max_mb: self.active_ram_max_mb,
            world_size_mb,
            server_type,
        })
    }

    pub fn select_active_server(&mut self, id: ServerId) -> Result<(), LifecycleError> {
        if self.state.process_may_be_alive() && self.active_server.as_ref() != Some(&id) {
            return Err(LifecycleError::AlreadyInState(self.state));
        }

        self.load_server(&id)?;
        self.active_server = Some(id);
        Ok(())
    }

    pub fn start_active_server(
        &mut self,
        launch: ProcessSpawnRequest,
    ) -> Result<ProcessId, LifecycleError> {
        let id = self.active_server_id()?.clone();
        let server = self.load_server(&id)?;
        let next = self
            .state
            .transition_to(LifecycleState::Starting)
            .map_err(LifecycleError::IllegalTransition)?;
        let ram_max_mb = parse_xmx_mb(&launch);
        let pid = self.process_supervisor.spawn(launch)?;
        self.state = next;
        self.active_process = Some(pid);
        self.active_ram_max_mb = ram_max_mb;
        self.output_reducer = JavaOutputReducer::new();
        self.latest_tps = None;
        self.pending_restart = None;
        self.console
            .append_system_line(&id, &format!("Starting server: {}", server.name));
        Ok(pid)
    }

    pub fn mark_ready(&mut self, id: &ServerId) -> Result<(), LifecycleError> {
        self.require_active_server(id)?;
        self.transition_to(LifecycleState::Running)
    }

    pub fn request_stop(&mut self) -> Result<(), LifecycleError> {
        let id = self.active_server_id()?.clone();
        let next = self
            .state
            .transition_to(LifecycleState::Stopping)
            .map_err(LifecycleError::IllegalTransition)?;
        let pid = self.active_process_id()?;
        self.process_supervisor.request_graceful_stop(pid)?;
        self.state = next;
        self.console.append_system_line(&id, "Stopping server.");
        Ok(())
    }

    pub fn restart_active_server(
        &mut self,
        launch: ProcessSpawnRequest,
    ) -> Result<(), LifecycleError> {
        let id = self.active_server_id()?.clone();
        let next = self
            .state
            .transition_to(LifecycleState::Stopping)
            .map_err(LifecycleError::IllegalTransition)?;
        let pid = self.active_process_id()?;
        self.process_supervisor.request_graceful_stop(pid)?;
        self.state = next;
        self.pending_restart = Some(launch);
        self.console.append_system_line(&id, "Restarting server.");
        Ok(())
    }

    pub fn send_command(&self, command: &str) -> Result<(), LifecycleError> {
        if self.state != LifecycleState::Running {
            return Err(LifecycleError::ServerNotRunning);
        }
        let pid = self.active_process_id()?;
        let payload = crate::commands::stdin_payload(command);
        self.process_supervisor.write_stdin(pid, &payload)?;
        Ok(())
    }

    pub fn ingest_console_line(&mut self, clean: &str) -> Result<Vec<OutputEvent>, LifecycleError> {
        let id = self.active_server_id()?.clone();
        let events = self.output_reducer.process_line(clean);
        for event in &events {
            match event {
                OutputEvent::Ready if self.state == LifecycleState::Starting => {
                    self.mark_ready(&id)?;
                }
                OutputEvent::TpsSample(sample) => {
                    self.latest_tps = Some(*sample);
                }
                OutputEvent::Ready | OutputEvent::PlayerJoined(_) | OutputEvent::PlayerLeft(_) => {}
            }
        }
        Ok(events)
    }

    pub fn mark_process_exited(&mut self, id: &ServerId) -> Result<(), LifecycleError> {
        self.require_active_server(id)?;
        self.active_process = None;
        self.active_ram_max_mb = None;
        let next = match self.state {
            LifecycleState::Stopping => LifecycleState::Stopped,
            LifecycleState::Starting | LifecycleState::Running => LifecycleState::Crashed,
            LifecycleState::Stopped | LifecycleState::Crashed => return Ok(()),
        };
        self.transition_to(next)?;

        if let Some(launch) = self.pending_restart.take() {
            self.start_active_server(launch)?;
        }

        Ok(())
    }

    pub fn handle_process_event(
        &mut self,
        pid: ProcessId,
        event: &ProcessEvent,
    ) -> Result<(), LifecycleError> {
        self.require_active_process(pid)?;
        match event {
            ProcessEvent::Output { .. } => Ok(()),
            ProcessEvent::Exited(_) => {
                let id = self.active_server_id()?.clone();
                self.mark_process_exited(&id)
            }
        }
    }

    fn active_server_id(&self) -> Result<&ServerId, LifecycleError> {
        self.active_server
            .as_ref()
            .ok_or(LifecycleError::NoActiveServer)
    }

    fn active_process_id(&self) -> Result<ProcessId, LifecycleError> {
        self.active_process
            .ok_or_else(|| LifecycleError::Process("no active process".to_string()))
    }

    fn require_active_process(&self, pid: ProcessId) -> Result<(), LifecycleError> {
        let active = self.active_process_id()?;
        if active == pid {
            Ok(())
        } else {
            Err(LifecycleError::Process(format!(
                "event for process {} does not match active process {}",
                pid.raw(),
                active.raw()
            )))
        }
    }

    fn require_active_server(&self, id: &ServerId) -> Result<(), LifecycleError> {
        let active = self.active_server_id()?;
        if active == id {
            Ok(())
        } else {
            Err(LifecycleError::WrongActiveServer {
                expected: active.clone(),
                actual: id.clone(),
            })
        }
    }

    fn load_server(&self, id: &ServerId) -> Result<ImportedJavaServer, LifecycleError> {
        self.repository
            .load(id)?
            .ok_or_else(|| LifecycleError::ServerNotFound(id.clone()))
    }

    fn load_active_server_if_selected(&self) -> Result<Option<ImportedJavaServer>, LifecycleError> {
        self.active_server
            .as_ref()
            .map(|id| self.load_server(id))
            .transpose()
    }

    fn transition_to(&mut self, next: LifecycleState) -> Result<(), LifecycleError> {
        self.state = self
            .state
            .transition_to(next)
            .map_err(LifecycleError::IllegalTransition)?;
        Ok(())
    }
}

fn parse_xmx_mb(launch: &ProcessSpawnRequest) -> Option<f64> {
    launch
        .arguments
        .iter()
        .rev()
        .find_map(|argument| parse_memory_flag_mb(argument, "-Xmx"))
}

fn parse_memory_flag_mb(argument: &str, prefix: &str) -> Option<f64> {
    let raw = argument.strip_prefix(prefix)?;
    let (number, multiplier) = match raw.chars().last()? {
        'g' | 'G' => (&raw[..raw.len() - 1], 1024.0),
        'm' | 'M' => (&raw[..raw.len() - 1], 1.0),
        _ => (raw, 1.0),
    };
    Some(number.parse::<f64>().ok()? * multiplier)
}
