//! Lifecycle application boundary for the Phase 4 Java vertical slice.
//!
//! This crate is where MSC 1's view-model-owned lifecycle behavior starts
//! becoming an application service: it owns server state and calls injected
//! dependencies, but it does not know about HTTP routes, CLI commands, iOS,
//! or any other client surface.

use msc_domain::crash_analysis;
use msc_domain::identity::{AddOnKind, JavaServerFlavor};
use msc_domain::tps;
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::metrics::{ProcessMetricsProvider, directory_size_mb};
use msc_infrastructure::process::{
    ProcessError, ProcessEvent, ProcessId, ProcessSpawnRequest, ProcessSupervisor,
};
use std::fmt;
use std::path::PathBuf;

use crate::diagnostics;
use crate::output_reducer::{JavaOutputReducer, OutputEvent};
use crate::session_log::SessionEventType;
use crate::status::{LifecycleStatusSnapshot, PerformanceSnapshot};

/// Paper soft-failure analysis uses `suffix(400)` in MSC 1
/// (`AppViewModel+OutputHandling.swift:258`). The hard-crash analyzer
/// below still receives only its own source-accurate last 120 lines.
const RECENT_CONSOLE_CAPACITY: usize = 400;
const CRASH_EXCERPT_CAPACITY: usize = 120;

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

pub trait JavaServerRepository: Send + Sync {
    fn load(&self, id: &ServerId) -> Result<Option<ImportedJavaServer>, LifecycleError>;
}

pub trait ConsoleSink: Send + Sync {
    fn append_system_line(&self, server_id: &ServerId, line: &str);
}

pub struct LifecycleService<'deps> {
    repository: &'deps dyn JavaServerRepository,
    process_supervisor: &'deps dyn ProcessSupervisor,
    console: &'deps dyn ConsoleSink,
    fs: &'deps dyn FileSystem,
    active_server: Option<ServerId>,
    active_process: Option<ProcessId>,
    active_ram_max_mb: Option<f64>,
    state: LifecycleState,
    output_reducer: JavaOutputReducer,
    latest_tps: Option<tps::Sample>,
    pending_restart: Option<ProcessSpawnRequest>,
    /// Recent server console lines (ANSI already stripped, the only form
    /// this service ever sees), capped at [`RECENT_CONSOLE_CAPACITY`] —
    /// `diagnose_unexpected_stop`'s own `console_excerpt` input. Reset on
    /// every new start, same lifetime as `output_reducer`.
    recent_console_lines: Vec<String>,
}

impl<'deps> LifecycleService<'deps> {
    pub fn new(
        repository: &'deps dyn JavaServerRepository,
        process_supervisor: &'deps dyn ProcessSupervisor,
        console: &'deps dyn ConsoleSink,
        fs: &'deps dyn FileSystem,
    ) -> Self {
        Self {
            repository,
            process_supervisor,
            console,
            fs,
            active_server: None,
            active_process: None,
            active_ram_max_mb: None,
            state: LifecycleState::Stopped,
            output_reducer: JavaOutputReducer::new(),
            latest_tps: None,
            pending_restart: None,
            recent_console_lines: Vec::new(),
        }
    }

    pub fn state(&self) -> LifecycleState {
        self.state
    }

    pub fn active_server(&self) -> Option<&ServerId> {
        self.active_server.as_ref()
    }

    /// Drops the selected server after a host reset. The reset route refuses
    /// live processes first, so clearing this in-memory pointer cannot orphan
    /// a process that the lifecycle service still owns.
    pub fn clear_selection(&mut self) {
        self.active_server = None;
        self.active_process = None;
        self.active_ram_max_mb = None;
        self.state = LifecycleState::Stopped;
        self.output_reducer = JavaOutputReducer::new();
        self.latest_tps = None;
        self.pending_restart = None;
        self.recent_console_lines.clear();
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
            tps_5m: self.latest_tps.and_then(|sample| sample.t5),
            tps_15m: self.latest_tps.and_then(|sample| sample.t15),
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
        self.recent_console_lines.clear();
        self.console
            .append_system_line(&id, &format!("Starting server: {}", server.name));
        Ok(pid)
    }

    pub fn mark_ready(&mut self, id: &ServerId, now: &str) -> Result<(), LifecycleError> {
        self.require_active_server(id)?;
        self.transition_to(LifecycleState::Running)?;
        self.scan_paper_plugins_once_ready(id, now);
        Ok(())
    }

    /// `scanPaperSoftFailures(for:)`'s real trigger point — MSC 1 calls it
    /// from the same "reached ready" branch this service's own
    /// `OutputEvent::Ready` handling already has (`AppViewModel+
    /// OutputHandling.swift:44-58`); this port's own P7.32-era note
    /// flagged that nothing called it here yet. Fires at most once per
    /// start: `ingest_console_line` only reaches `mark_ready` while
    /// `state == Starting`, and this method's own `transition_to` above
    /// has already left that state by the time this runs, so a second
    /// `Ready` event this same run (the reducer's own `reached_ready`
    /// latch, `output_reducer.rs`) can never re-enter it. Best-effort like
    /// [`Self::record_stop_diagnostics`]: a server the repository can no
    /// longer load skips the scan silently rather than failing the
    /// ready-transition already committed above.
    fn scan_paper_plugins_once_ready(&self, id: &ServerId, now: &str) {
        let Ok(server) = self.load_server(id) else {
            return;
        };
        if server.flavor.add_on_kind() != Some(AddOnKind::Plugin) {
            return;
        }
        let plugins_dir = server.directory.join(AddOnKind::Plugin.folder_name());
        let installed_plugins = crate::add_on_inventory::scan_plugins(self.fs, &plugins_dir);
        let problems =
            crash_analysis::analyze_paper_plugins(&self.recent_console_lines, &installed_plugins);
        diagnostics::scan_paper_soft_failures(
            self.fs,
            &server.directory,
            now,
            true,
            true,
            problems,
        );
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

    pub fn ingest_console_line(
        &mut self,
        clean: &str,
        now: &str,
    ) -> Result<Vec<OutputEvent>, LifecycleError> {
        let id = self.active_server_id()?.clone();
        self.recent_console_lines.push(clean.to_string());
        if self.recent_console_lines.len() > RECENT_CONSOLE_CAPACITY {
            self.recent_console_lines.remove(0);
        }
        let events = self.output_reducer.process_line(clean);
        for event in &events {
            match event {
                OutputEvent::Ready if self.state == LifecycleState::Starting => {
                    self.mark_ready(&id, now)?;
                }
                OutputEvent::TpsSample(sample) => {
                    self.latest_tps = Some(*sample);
                }
                OutputEvent::PlayerJoined(name) => {
                    self.record_session_event(&id, name, SessionEventType::Joined, now);
                }
                OutputEvent::PlayerLeft(name) => {
                    self.record_session_event(&id, name, SessionEventType::Left, now);
                }
                OutputEvent::Ready => {}
            }
        }
        Ok(events)
    }

    /// Session history is best-effort: a damaged or unavailable server
    /// directory must not turn one console line into a failed lifecycle
    /// transition. This mirrors MSC 1's catch-and-log around `recordSessionEvent`.
    fn record_session_event(
        &self,
        server_id: &ServerId,
        player_name: &str,
        event_type: SessionEventType,
        timestamp: &str,
    ) {
        let server = match self.load_server(server_id) {
            Ok(server) => server,
            Err(error) => {
                eprintln!("[Session] Failed to persist session event for {player_name}: {error}");
                return;
            }
        };
        if let Err(error) = crate::session_log::append_event(
            self.fs,
            &server.directory,
            player_name,
            event_type,
            timestamp.to_string(),
        ) {
            eprintln!("[Session] Failed to persist session event for {player_name}: {error}");
        }
    }

    pub fn mark_process_exited(&mut self, id: &ServerId, now: &str) -> Result<(), LifecycleError> {
        self.require_active_server(id)?;
        self.active_process = None;
        self.active_ram_max_mb = None;
        self.latest_tps = None;
        let was_user_requested_stop = self.state == LifecycleState::Stopping;
        let reached_ready_state = self.output_reducer.reached_ready();
        let next = match self.state {
            LifecycleState::Stopping => LifecycleState::Stopped,
            LifecycleState::Starting | LifecycleState::Running => LifecycleState::Crashed,
            LifecycleState::Stopped | LifecycleState::Crashed => return Ok(()),
        };
        self.transition_to(next)?;
        self.record_stop_diagnostics(id, now, was_user_requested_stop, reached_ready_state);

        if let Some(launch) = self.pending_restart.take() {
            self.start_active_server(launch)?;
        }

        Ok(())
    }

    /// `javaBackend.onDidTerminate`'s post-transition branch
    /// (`AppViewModel.swift:1141-1175`): an unrequested stop always runs
    /// `diagnoseUnexpectedStop` (crash analysis when modded and never
    /// ready); a user-requested stop (`request_stop`/`restart_active_server`
    /// — the only ways `Stopping` is ever entered) skips analysis entirely
    /// and only records the generic "stopped before ready" fatal line when
    /// the server never got there. Best-effort like
    /// `write_last_startup_result` itself: a server the repository can no
    /// longer load (deleted mid-run) simply skips the record rather than
    /// failing the exit handling already committed above.
    ///
    /// P7.36's local add-on inventory supplies installed mod identity so
    /// the analyzer can map a log id back to the jar stem required by
    /// verified disable/delete repairs.
    fn record_stop_diagnostics(
        &self,
        id: &ServerId,
        now: &str,
        was_user_requested_stop: bool,
        reached_ready_state: bool,
    ) {
        let Ok(server) = self.load_server(id) else {
            return;
        };
        if was_user_requested_stop {
            if !reached_ready_state {
                diagnostics::write_last_startup_result(
                    self.fs,
                    &server.directory,
                    now,
                    false,
                    vec!["Server stopped before reaching ready state.".to_string()],
                    Vec::new(),
                    Vec::new(),
                );
            }
            return;
        }
        let is_modded = server.flavor.add_on_kind() == Some(AddOnKind::Mod);
        let installed_mods = if is_modded {
            let mods_dir = server.directory.join(AddOnKind::Mod.folder_name());
            crate::add_on_inventory::scan_mods(self.fs, &mods_dir)
        } else {
            Vec::new()
        };
        let crash_excerpt_start = self
            .recent_console_lines
            .len()
            .saturating_sub(CRASH_EXCERPT_CAPACITY);
        diagnostics::diagnose_unexpected_stop(
            self.fs,
            &server.directory,
            now,
            reached_ready_state,
            is_modded,
            server.flavor.raw_value(),
            &self.recent_console_lines[crash_excerpt_start..],
            &installed_mods,
        );
    }

    pub fn handle_process_event(
        &mut self,
        pid: ProcessId,
        event: &ProcessEvent,
        now: &str,
    ) -> Result<(), LifecycleError> {
        self.require_active_process(pid)?;
        match event {
            ProcessEvent::Output { .. } => Ok(()),
            ProcessEvent::Exited(_) => {
                let id = self.active_server_id()?.clone();
                self.mark_process_exited(&id, now)
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
