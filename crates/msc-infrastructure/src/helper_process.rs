//! Shared supervision for player-facing helper programs.
//!
//! A helper is deliberately not a server process: this manager only owns
//! programs such as Playit or Xbox Broadcast, keyed by the server and helper
//! function. The application layer must still admit and journal the action
//! through Phase 4's operation boundary before it calls this module. On an
//! agent restart no old PID is trusted; callers restore keys with
//! [`HelperProcessManager::recover_after_restart`] and must reconcile them
//! before starting another helper.

use crate::process::{
    OutputStream, OutputStreamLineFramer, ProcessError, ProcessExitStatus, ProcessId,
    ProcessSpawnRequest, ProcessSupervisor,
};
use std::collections::{BTreeMap, VecDeque};
use std::fmt;

/// Maximum complete output lines retained per helper. This is diagnostic
/// context, not a second unbounded console history.
pub const HELPER_DIAGNOSTIC_LIMIT: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HelperKey {
    pub server_id: String,
    pub function: String,
}

impl HelperKey {
    pub fn new(server_id: impl Into<String>, function: impl Into<String>) -> Self {
        Self {
            server_id: server_id.into(),
            function: function.into(),
        }
    }

    /// The stable per-helper target for the existing operation journal.
    pub fn operation_target(&self) -> String {
        format!("helper:{}:{}", self.server_id, self.function)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagedHelperStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed { exit: Option<ProcessExitStatus> },
    UnknownUntilReconciled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperDiagnostic {
    pub stream: OutputStream,
    pub line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedHelperSnapshot {
    pub key: HelperKey,
    pub status: ManagedHelperStatus,
    pub diagnostics: Vec<HelperDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperProcessError {
    AlreadyManaged(HelperKey),
    UnknownHelper(HelperKey),
    NotRunning(HelperKey),
    Process(String),
}

impl fmt::Display for HelperProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyManaged(key) => write!(
                f,
                "{} is already managed for server {}",
                key.function, key.server_id
            ),
            Self::UnknownHelper(key) => write!(
                f,
                "{} is not managed for server {}",
                key.function, key.server_id
            ),
            Self::NotRunning(key) => write!(
                f,
                "{} is not running for server {}",
                key.function, key.server_id
            ),
            Self::Process(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for HelperProcessError {}

impl From<ProcessError> for HelperProcessError {
    fn from(value: ProcessError) -> Self {
        Self::Process(value.to_string())
    }
}

struct ManagedHelper {
    pid: Option<ProcessId>,
    status: ManagedHelperStatus,
    framer: OutputStreamLineFramer,
    diagnostics: VecDeque<HelperDiagnostic>,
}

/// Owns helper subprocesses within one running agent. It intentionally has
/// no persistence: a PID cannot be safely recovered across an agent restart.
pub struct HelperProcessManager<'supervisor> {
    supervisor: &'supervisor dyn ProcessSupervisor,
    helpers: BTreeMap<HelperKey, ManagedHelper>,
}

impl<'supervisor> HelperProcessManager<'supervisor> {
    pub fn new(supervisor: &'supervisor dyn ProcessSupervisor) -> Self {
        Self {
            supervisor,
            helpers: BTreeMap::new(),
        }
    }

    /// Restores only the fact that a helper was in scope before the restart.
    /// It never reports the old process as running, because that PID belongs
    /// to the prior agent and may now identify an unrelated process.
    pub fn recover_after_restart(
        supervisor: &'supervisor dyn ProcessSupervisor,
        keys: impl IntoIterator<Item = HelperKey>,
    ) -> Self {
        let mut manager = Self::new(supervisor);
        for key in keys {
            manager.helpers.insert(
                key,
                ManagedHelper {
                    pid: None,
                    status: ManagedHelperStatus::UnknownUntilReconciled,
                    framer: OutputStreamLineFramer::new(),
                    diagnostics: VecDeque::new(),
                },
            );
        }
        manager
    }

    pub fn start(
        &mut self,
        key: HelperKey,
        request: ProcessSpawnRequest,
    ) -> Result<ProcessId, HelperProcessError> {
        if self.helpers.contains_key(&key) {
            return Err(HelperProcessError::AlreadyManaged(key));
        }

        match self.supervisor.spawn(request) {
            Ok(pid) => {
                self.helpers.insert(
                    key,
                    ManagedHelper {
                        pid: Some(pid),
                        status: ManagedHelperStatus::Starting,
                        framer: OutputStreamLineFramer::new(),
                        diagnostics: VecDeque::new(),
                    },
                );
                Ok(pid)
            }
            Err(error) => {
                self.helpers.insert(
                    key,
                    ManagedHelper {
                        pid: None,
                        status: ManagedHelperStatus::Failed { exit: None },
                        framer: OutputStreamLineFramer::new(),
                        diagnostics: VecDeque::from([HelperDiagnostic {
                            stream: OutputStream::Stderr,
                            line: format!("failed to start helper: {error}"),
                        }]),
                    },
                );
                Err(error.into())
            }
        }
    }

    /// Records a provider-specific readiness signal after the caller has
    /// interpreted its output. This foundation does not guess provider log
    /// formats, which arrive with each helper integration.
    pub fn record_ready(&mut self, key: &HelperKey) -> Result<(), HelperProcessError> {
        let helper = self.helper_mut(key)?;
        if helper.pid.is_none() {
            return Err(HelperProcessError::NotRunning(key.clone()));
        }
        helper.status = ManagedHelperStatus::Running;
        Ok(())
    }

    /// Asks the helper to stop, but leaves the process supervised until it
    /// actually exits. Call [`Self::force_terminate`] only after the caller's
    /// bounded grace period has elapsed.
    pub fn request_graceful_stop(&mut self, key: &HelperKey) -> Result<(), HelperProcessError> {
        let pid = self
            .helper_mut(key)?
            .pid
            .ok_or_else(|| HelperProcessError::NotRunning(key.clone()))?;
        self.supervisor.request_graceful_stop(pid)?;
        self.helper_mut(key)?.status = ManagedHelperStatus::Stopping;
        Ok(())
    }

    pub fn force_terminate(&mut self, key: &HelperKey) -> Result<(), HelperProcessError> {
        let pid = self
            .helper_mut(key)?
            .pid
            .ok_or_else(|| HelperProcessError::NotRunning(key.clone()))?;
        self.supervisor.force_terminate(pid)?;
        self.helper_mut(key)?.status = ManagedHelperStatus::Stopping;
        Ok(())
    }

    /// Drains output and exit events for every live helper. Complete lines
    /// retain their original stream; partial lines are retained when exit
    /// makes them final.
    pub fn poll(&mut self) -> Result<(), HelperProcessError> {
        let live = self
            .helpers
            .iter()
            .filter_map(|(key, helper)| helper.pid.map(|pid| (key.clone(), pid)))
            .collect::<Vec<_>>();

        for (key, pid) in live {
            let events = self.supervisor.drain_events(pid)?;
            let helper = self.helper_mut(&key)?;
            for event in events {
                match event {
                    crate::process::ProcessEvent::Output { stream, bytes } => {
                        for line in helper.framer.push(stream, &bytes) {
                            push_diagnostic(&mut helper.diagnostics, stream, line);
                        }
                    }
                    crate::process::ProcessEvent::Exited(exit) => {
                        for (stream, line) in helper.framer.flush() {
                            push_diagnostic(&mut helper.diagnostics, stream, line);
                        }
                        helper.pid = None;
                        helper.status = if exit.success() {
                            ManagedHelperStatus::Stopped
                        } else {
                            ManagedHelperStatus::Failed { exit: Some(exit) }
                        };
                    }
                }
            }
        }
        Ok(())
    }

    /// Resolves a restart-unknown helper only after the caller has performed
    /// its provider-specific reconciliation.
    pub fn reconcile_as_stopped(&mut self, key: &HelperKey) -> Result<(), HelperProcessError> {
        let helper = self.helper_mut(key)?;
        if helper.status == ManagedHelperStatus::UnknownUntilReconciled {
            helper.status = ManagedHelperStatus::Stopped;
        }
        Ok(())
    }

    pub fn snapshot(&self, key: &HelperKey) -> Option<ManagedHelperSnapshot> {
        self.helpers.get(key).map(|helper| ManagedHelperSnapshot {
            key: key.clone(),
            status: helper.status.clone(),
            diagnostics: helper.diagnostics.iter().cloned().collect(),
        })
    }

    fn helper_mut(&mut self, key: &HelperKey) -> Result<&mut ManagedHelper, HelperProcessError> {
        self.helpers
            .get_mut(key)
            .ok_or_else(|| HelperProcessError::UnknownHelper(key.clone()))
    }
}

fn push_diagnostic(
    diagnostics: &mut VecDeque<HelperDiagnostic>,
    stream: OutputStream,
    line: String,
) {
    diagnostics.push_back(HelperDiagnostic { stream, line });
    while diagnostics.len() > HELPER_DIAGNOSTIC_LIMIT {
        diagnostics.pop_front();
    }
}
