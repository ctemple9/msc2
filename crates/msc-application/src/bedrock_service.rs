//! Application orchestration for a native Bedrock server.
//!
//! `LinuxBedrockRuntime` owns the child process.  This service owns the
//! product-facing consequences of that process: readiness, bounded console
//! history, player state, log mirroring, save coordination, metrics, and the
//! durable operation state that explains a start or stop across a restart.

use crate::bedrock_runtime::{
    BedrockProvisionRequest, BedrockRuntime, BedrockRuntimeError, BedrockRuntimeEvent,
    BedrockRuntimeState, BedrockStartRequest, BedrockTerminationReason,
};
use crate::lifecycle::LifecycleState;
use crate::operations::{LifecycleOperationError, LifecycleOperationSnapshot, LifecycleOperations};
use msc_domain::bedrock::{
    BedrockConsoleEvent, BedrockPlayer, BedrockPlayerEvent, backfill_allowlist_xuid,
    classify_console_line,
};
use msc_infrastructure::atomic_write::atomic_write;
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::metrics::{ProcessMetricsProvider, ProcessResourceUsage};
use std::collections::VecDeque;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

pub const BEDROCK_CONSOLE_HISTORY_CAPACITY: usize = 200;
pub const BEDROCK_PLAYER_COUNT_HISTORY_CAPACITY: usize = 30;
pub const BEDROCK_METRIC_HISTORY_CAPACITY: usize = 60;
pub const BEDROCK_ROLLED_LOG_LIMIT: usize = 10;
pub const BEDROCK_SAVE_QUERY_LIMIT: usize = 10;
pub const BEDROCK_START_OPERATION: &str = "bedrock-start";
pub const BEDROCK_STOP_OPERATION: &str = "bedrock-stop";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockServerInfo {
    pub id: String,
    pub name: String,
    pub directory: PathBuf,
    pub version: String,
    pub memory_gb: u32,
    pub bedrock_port: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BedrockMetricSample {
    pub cpu_percent: Option<f64>,
    pub ram_used_mb: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BedrockPerformanceSnapshot {
    pub ts: String,
    pub players_online: usize,
    pub cpu_percent: Option<f64>,
    pub ram_used_mb: Option<f64>,
    pub metric_history_len: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BedrockServiceEvent {
    Ready,
    PlayerJoined(BedrockPlayer),
    PlayerLeft(BedrockPlayer),
    Version(String),
    ConsoleLine(String),
    CleanStop,
    Crash(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockSavePause {
    pub saves_held: bool,
    pub ready_to_copy: bool,
    pub query_attempts: usize,
}

#[derive(Debug)]
pub enum BedrockServiceError {
    Runtime(BedrockRuntimeError),
    Operation(LifecycleOperationError),
    Filesystem(io::Error),
    AtomicWrite(String),
    InvalidState {
        operation: &'static str,
        state: LifecycleState,
    },
    NotRunning,
}

impl fmt::Display for BedrockServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(error) => write!(f, "{error}"),
            Self::Operation(error) => write!(f, "{error}"),
            Self::Filesystem(error) => write!(f, "{error}"),
            Self::AtomicWrite(error) => write!(f, "{error}"),
            Self::InvalidState { operation, state } => {
                write!(f, "cannot {operation} while Bedrock service is {state:?}")
            }
            Self::NotRunning => f.write_str("Bedrock server is not running"),
        }
    }
}

impl std::error::Error for BedrockServiceError {}

impl From<BedrockRuntimeError> for BedrockServiceError {
    fn from(error: BedrockRuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<LifecycleOperationError> for BedrockServiceError {
    fn from(error: LifecycleOperationError) -> Self {
        Self::Operation(error)
    }
}

impl From<io::Error> for BedrockServiceError {
    fn from(error: io::Error) -> Self {
        Self::Filesystem(error)
    }
}

/// The stateful application service for one Bedrock server.
pub struct BedrockService<'deps, 'fs, R: BedrockRuntime> {
    runtime: R,
    fs: &'deps dyn FileSystem,
    metrics: &'deps dyn ProcessMetricsProvider,
    operations: &'deps LifecycleOperations<'fs>,
    server: BedrockServerInfo,
    state: LifecycleState,
    active_operation: Option<msc_domain::operation::OperationId>,
    last_operation: Option<msc_domain::operation::OperationId>,
    console_history: VecDeque<String>,
    player_count_history: VecDeque<usize>,
    online_players: Vec<BedrockPlayer>,
    session_history: Vec<String>,
    metric_history: VecDeque<BedrockMetricSample>,
    log_open: bool,
    save_ready_seen: bool,
}

impl<'deps, 'fs, R: BedrockRuntime> BedrockService<'deps, 'fs, R> {
    pub fn new(
        runtime: R,
        fs: &'deps dyn FileSystem,
        metrics: &'deps dyn ProcessMetricsProvider,
        operations: &'deps LifecycleOperations<'fs>,
        server: BedrockServerInfo,
    ) -> Self {
        Self {
            runtime,
            fs,
            metrics,
            operations,
            server,
            state: LifecycleState::Stopped,
            active_operation: None,
            last_operation: None,
            console_history: VecDeque::with_capacity(BEDROCK_CONSOLE_HISTORY_CAPACITY),
            player_count_history: VecDeque::with_capacity(BEDROCK_PLAYER_COUNT_HISTORY_CAPACITY),
            online_players: Vec::new(),
            session_history: Vec::new(),
            metric_history: VecDeque::with_capacity(BEDROCK_METRIC_HISTORY_CAPACITY),
            log_open: false,
            save_ready_seen: false,
        }
    }

    pub fn runtime(&self) -> &R {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut R {
        &mut self.runtime
    }

    pub fn state(&self) -> LifecycleState {
        self.state
    }

    pub fn runtime_state(&self) -> BedrockRuntimeState {
        self.runtime.state()
    }

    pub fn server(&self) -> &BedrockServerInfo {
        &self.server
    }

    pub fn online_players(&self) -> &[BedrockPlayer] {
        &self.online_players
    }

    pub fn session_history(&self) -> &[String] {
        &self.session_history
    }

    pub fn console_tail(&self, requested: usize) -> Vec<String> {
        let count = requested.clamp(1, BEDROCK_CONSOLE_HISTORY_CAPACITY);
        self.console_history
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub fn player_count_history(&self) -> impl Iterator<Item = &usize> {
        self.player_count_history.iter()
    }

    pub fn metric_history(&self) -> impl Iterator<Item = &BedrockMetricSample> {
        self.metric_history.iter()
    }

    pub fn active_operation(&self) -> Option<&msc_domain::operation::OperationId> {
        self.active_operation.as_ref()
    }

    pub fn operation_snapshot(
        &self,
    ) -> Result<Option<LifecycleOperationSnapshot>, BedrockServiceError> {
        match self.active_operation.as_ref() {
            Some(id) => self
                .operations
                .snapshot(id)
                .map_err(BedrockServiceError::from),
            None => match self.last_operation.as_ref() {
                Some(id) => self
                    .operations
                    .snapshot(id)
                    .map_err(BedrockServiceError::from),
                None => Ok(None),
            },
        }
    }

    /// Reconcile an operation left open if the agent was restarted. The
    /// journal owns the state transition; the service deliberately does not
    /// pretend that a process can be resumed without a fresh start.
    pub fn reconcile_on_startup(
        &self,
    ) -> Result<Vec<msc_infrastructure::operation_journal::ReconciliationRecord>, BedrockServiceError>
    {
        self.operations
            .reconcile_on_startup()
            .map_err(BedrockServiceError::from)
    }

    pub fn start(&mut self, now: &str) -> Result<(), BedrockServiceError> {
        if !matches!(
            self.state,
            LifecycleState::Stopped | LifecycleState::Crashed
        ) {
            return Err(BedrockServiceError::InvalidState {
                operation: "start",
                state: self.state,
            });
        }
        let operation = self.operations.begin_running(
            BEDROCK_START_OPERATION,
            Some(self.server.id.clone()),
            "Starting Bedrock server.",
        )?;
        self.active_operation = Some(operation.clone());
        self.last_operation = Some(operation.clone());

        let result = self.runtime.provision(BedrockProvisionRequest {
            server_dir: self.server.directory.to_string_lossy().into_owned(),
            version: self.server.version.clone(),
        });
        if let Err(error) = result.and_then(|()| {
            self.runtime.start(BedrockStartRequest {
                memory_gb: self.server.memory_gb,
                bedrock_port: self.server.bedrock_port,
            })
        }) {
            self.fail_active_operation("bedrock_start_failed", error.to_string());
            return Err(error.into());
        }

        self.start_log_file(now);
        self.console_history.clear();
        self.online_players.clear();
        self.session_history.clear();
        self.player_count_history.clear();
        self.metric_history.clear();
        self.save_ready_seen = false;
        self.state = LifecycleState::Starting;
        Ok(())
    }

    pub fn restart_after_crash(&mut self, now: &str) -> Result<(), BedrockServiceError> {
        if self.state != LifecycleState::Crashed {
            return Err(BedrockServiceError::InvalidState {
                operation: "restart-after-crash",
                state: self.state,
            });
        }
        self.start(now)
    }

    pub fn stop(&mut self) -> Result<(), BedrockServiceError> {
        if !matches!(
            self.state,
            LifecycleState::Starting | LifecycleState::Running
        ) {
            return Err(BedrockServiceError::InvalidState {
                operation: "stop",
                state: self.state,
            });
        }
        let operation = self.operations.begin_running(
            BEDROCK_STOP_OPERATION,
            Some(self.server.id.clone()),
            "Stopping Bedrock server.",
        )?;
        self.active_operation = Some(operation);
        self.last_operation = self.active_operation.clone();
        if let Err(error) = self.runtime.stop() {
            self.fail_active_operation("bedrock_stop_failed", error.to_string());
            return Err(error.into());
        }
        self.state = LifecycleState::Stopping;
        Ok(())
    }

    pub fn command(&mut self, command: &str) -> Result<(), BedrockServiceError> {
        if self.state != LifecycleState::Running {
            return Err(BedrockServiceError::NotRunning);
        }
        self.runtime.command(command).map_err(Into::into)
    }

    /// Drain all currently available runtime events. A runtime event is the
    /// process boundary; parsing and persistence happen exactly once here.
    pub fn poll(&mut self) -> Result<Vec<BedrockServiceEvent>, BedrockServiceError> {
        let mut emitted = Vec::new();
        while let Some(event) = self.runtime.poll_event()? {
            match event {
                BedrockRuntimeEvent::Ready { .. } => {
                    let was_starting = self.state == LifecycleState::Starting;
                    if was_starting {
                        self.state = LifecycleState::Running;
                        if let Some(id) = self.active_operation.take() {
                            self.operations.succeed(
                                &id,
                                "Bedrock server is ready.",
                                [("state".to_owned(), "running".to_owned())]
                                    .into_iter()
                                    .collect(),
                            )?;
                        }
                    }
                    if was_starting {
                        emitted.push(BedrockServiceEvent::Ready);
                    }
                }
                BedrockRuntimeEvent::ConsoleLine(line) => {
                    emitted.extend(self.ingest_console_line(&line));
                }
                BedrockRuntimeEvent::Metrics(_) => {
                    // Native Linux metrics come from `ps` through the shared
                    // provider. `[MSCSTATS]` is a sidecar-only protocol and
                    // must never become the native metrics source.
                }
                BedrockRuntimeEvent::Terminated { reason } => {
                    emitted.push(self.handle_termination(reason)?);
                }
            }
        }
        Ok(emitted)
    }

    pub fn ingest_console_line(&mut self, line: &str) -> Vec<BedrockServiceEvent> {
        let classified = classify_console_line(line);
        if !matches!(classified, BedrockConsoleEvent::Stats(_)) {
            self.console_history.push_back(line.to_owned());
            while self.console_history.len() > BEDROCK_CONSOLE_HISTORY_CAPACITY {
                self.console_history.pop_front();
            }
            self.append_log_line(line);
        }

        match classified {
            // The runtime emits a separate Ready event after the console
            // line. Keeping readiness at that boundary prevents one BDS
            // line from producing duplicate lifecycle notifications.
            BedrockConsoleEvent::Ready => Vec::new(),
            BedrockConsoleEvent::Version(version) => vec![BedrockServiceEvent::Version(version)],
            BedrockConsoleEvent::Player(event) => vec![self.record_player_event(event)],
            BedrockConsoleEvent::Stats(_) | BedrockConsoleEvent::GuestIp(_) => Vec::new(),
            BedrockConsoleEvent::Other => {
                if line
                    .to_ascii_lowercase()
                    .contains("files are now ready to be copied")
                {
                    self.save_ready_seen = true;
                }
                vec![BedrockServiceEvent::ConsoleLine(line.to_owned())]
            }
        }
    }

    pub fn performance_snapshot(&mut self, ts: impl Into<String>) -> BedrockPerformanceSnapshot {
        let usage = if self.state.process_may_be_alive() {
            self.runtime
                .process_id()
                .and_then(|pid| self.metrics.process_usage(pid))
        } else {
            None
        };
        if let Some(usage) = usage {
            self.push_metric(usage);
        }
        BedrockPerformanceSnapshot {
            ts: ts.into(),
            players_online: self.online_players.len(),
            cpu_percent: usage.and_then(|value| value.cpu_percent),
            ram_used_mb: usage.and_then(|value| value.ram_used_mb),
            metric_history_len: self.metric_history.len(),
        }
    }

    /// Implements MSC 1's best-effort Bedrock save protocol. A failed
    /// `save query` send or a timeout does not abort the backup; only a
    /// successful `save hold` creates a save state that must be resumed.
    pub fn hold_saves(
        &mut self,
        query_limit: usize,
    ) -> Result<BedrockSavePause, BedrockServiceError> {
        if self.state != LifecycleState::Running {
            return Err(BedrockServiceError::NotRunning);
        }
        if self.runtime.command("save hold").is_err() {
            return Ok(BedrockSavePause {
                saves_held: false,
                ready_to_copy: false,
                query_attempts: 0,
            });
        }
        self.save_ready_seen = false;
        let mut attempts = 0;
        for _ in 0..query_limit.min(BEDROCK_SAVE_QUERY_LIMIT) {
            attempts += 1;
            let _ = self.runtime.command("save query");
            self.poll()?;
            if self.save_ready_seen {
                break;
            }
        }
        Ok(BedrockSavePause {
            saves_held: true,
            ready_to_copy: self.save_ready_seen,
            query_attempts: attempts,
        })
    }

    pub fn resume_saves(&mut self) -> bool {
        if self.state != LifecycleState::Running {
            return false;
        }
        self.runtime.command("save resume").is_ok()
    }

    pub fn run_with_save_hold<T>(
        &mut self,
        query_limit: usize,
        backup: impl FnOnce() -> T,
    ) -> Result<(T, BedrockSavePause), BedrockServiceError> {
        let pause = self.hold_saves(query_limit)?;
        let value = backup();
        if pause.saves_held {
            let _ = self.resume_saves();
        }
        Ok((value, pause))
    }

    fn record_player_event(&mut self, event: BedrockPlayerEvent) -> BedrockServiceEvent {
        match event {
            BedrockPlayerEvent::Connected(player) => {
                if let Some(existing) = self
                    .online_players
                    .iter_mut()
                    .find(|existing| existing.name.eq_ignore_ascii_case(&player.name))
                {
                    if player.xuid.is_some() {
                        existing.xuid = player.xuid.clone();
                    }
                    existing.name = player.name.clone();
                } else {
                    self.online_players.push(player.clone());
                }
                if !self
                    .session_history
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(&player.name))
                {
                    self.session_history.push(player.name.clone());
                }
                self.append_player_count();
                if let Some(xuid) = player.xuid.as_deref() {
                    self.backfill_allowlist(&player.name, xuid);
                }
                BedrockServiceEvent::PlayerJoined(player)
            }
            BedrockPlayerEvent::Disconnected(player) => {
                self.online_players
                    .retain(|existing| !existing.name.eq_ignore_ascii_case(&player.name));
                self.append_player_count();
                BedrockServiceEvent::PlayerLeft(player)
            }
        }
    }

    fn append_player_count(&mut self) {
        self.player_count_history
            .push_back(self.online_players.len());
        while self.player_count_history.len() > BEDROCK_PLAYER_COUNT_HISTORY_CAPACITY {
            self.player_count_history.pop_front();
        }
    }

    fn push_metric(&mut self, usage: ProcessResourceUsage) {
        self.metric_history.push_back(BedrockMetricSample {
            cpu_percent: usage.cpu_percent,
            ram_used_mb: usage.ram_used_mb,
        });
        while self.metric_history.len() > BEDROCK_METRIC_HISTORY_CAPACITY {
            self.metric_history.pop_front();
        }
    }

    fn handle_termination(
        &mut self,
        reason: BedrockTerminationReason,
    ) -> Result<BedrockServiceEvent, BedrockServiceError> {
        self.log_open = false;
        match reason {
            BedrockTerminationReason::Clean => {
                self.state = LifecycleState::Stopped;
                self.succeed_active_operation("Bedrock server stopped.")?;
                Ok(BedrockServiceEvent::CleanStop)
            }
            BedrockTerminationReason::GuestError(message)
            | BedrockTerminationReason::StartFailed(message) => {
                self.state = LifecycleState::Crashed;
                if self.active_operation.is_none() {
                    let operation = self.operations.begin_running(
                        "bedrock-crash",
                        Some(self.server.id.clone()),
                        "Recording unexpected Bedrock termination.",
                    )?;
                    self.active_operation = Some(operation.clone());
                    self.last_operation = Some(operation);
                }
                self.fail_active_operation("bedrock_crash", message.clone());
                Ok(BedrockServiceEvent::Crash(message))
            }
        }
    }

    fn succeed_active_operation(&mut self, status: &str) -> Result<(), BedrockServiceError> {
        if let Some(id) = self.active_operation.take() {
            self.operations.succeed(&id, status, Default::default())?;
        }
        Ok(())
    }

    fn fail_active_operation(&mut self, code: &str, message: String) {
        if let Some(id) = self.active_operation.take() {
            let _ = self
                .operations
                .fail(&id, crate::operations::lifecycle_error(code, message));
        }
    }

    fn start_log_file(&mut self, now: &str) {
        let logs = self.server.directory.join("logs");
        let latest = logs.join("latest.log");
        let result = (|| -> Result<(), io::Error> {
            self.fs.create_dir_all(&logs)?;
            if self.fs.stat(&latest).is_ok() {
                self.fs.rename(
                    &latest,
                    &logs.join(format!("console-{}.log", log_stamp(now))),
                )?;
                self.prune_rolled_logs(&logs);
            }
            self.fs.write(
                &latest,
                format!("=== MSC Bedrock console log — started {now} ===\n").as_bytes(),
            )?;
            Ok(())
        })();
        self.log_open = result.is_ok();
    }

    fn append_log_line(&self, line: &str) {
        if !self.log_open {
            return;
        }
        let latest = self.server.directory.join("logs/latest.log");
        let Ok(mut current) = self.fs.read(&latest) else {
            return;
        };
        current.extend_from_slice(line.as_bytes());
        if !line.ends_with('\n') {
            current.push(b'\n');
        }
        let _ = self.fs.write(&latest, &current);
    }

    fn prune_rolled_logs(&self, logs: &Path) {
        let Ok(mut candidates) = self.fs.list(logs) else {
            return;
        };
        candidates.retain(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("console-") && name.ends_with(".log"))
        });
        candidates.sort_by(|left, right| {
            let left_modified = self.fs.stat(left).ok().map(|meta| meta.modified);
            let right_modified = self.fs.stat(right).ok().map(|meta| meta.modified);
            right_modified
                .cmp(&left_modified)
                .then_with(|| right.cmp(left))
        });
        for path in candidates.into_iter().skip(BEDROCK_ROLLED_LOG_LIMIT) {
            let _ = self.fs.remove(&path);
        }
    }

    fn backfill_allowlist(&self, name: &str, xuid: &str) {
        let path = self.server.directory.join("allowlist.json");
        let Ok(bytes) = self.fs.read(&path) else {
            return;
        };
        let entries = msc_domain::bedrock::parse_allowlist(&String::from_utf8_lossy(&bytes));
        let Some(updated) = backfill_allowlist_xuid(true, &entries, name, xuid) else {
            return;
        };
        let Ok(contents) = serde_json::to_vec_pretty(&updated) else {
            return;
        };
        let _ = atomic_write(self.fs, &path, &contents);
    }
}

fn log_stamp(now: &str) -> String {
    now.chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect::<String>()
        .replace('T', "-")
        .replace('Z', "")
}
