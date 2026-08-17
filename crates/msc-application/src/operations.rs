//! Application-level operation coordination for real lifecycle work.
//!
//! Phase 3 built the durable substrate: `OperationJournal::record`,
//! restart reconciliation, and per-target admission. This module is the
//! Phase 4 application layer that uses that substrate around real Java
//! lifecycle mutations.

use msc_domain::operation::{OperationError, OperationId, OperationProgress, OperationState};
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::operation_journal::{
    AdmitError, JournalEntry, JournalError, OperationJournal, ReconciliationRecord,
};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleOperationSnapshot {
    pub id: OperationId,
    pub operation_type: String,
    pub target: Option<String>,
    pub state: OperationState,
    pub progress: Option<OperationProgress>,
    pub status_line: Option<String>,
    pub result: Option<BTreeMap<String, String>>,
    pub error: Option<OperationError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OperationRecord {
    operation_type: String,
    target: Option<String>,
    state: OperationState,
    progress: Option<OperationProgress>,
    status_line: Option<String>,
    result: Option<BTreeMap<String, String>>,
    error: Option<OperationError>,
}

#[derive(Debug)]
pub enum LifecycleOperationError {
    Journal(String),
    Conflict(OperationError),
    UnknownOperation(OperationId),
    IllegalTransition {
        id: OperationId,
        from: OperationState,
        to: OperationState,
    },
}

impl fmt::Display for LifecycleOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Journal(message) => write!(f, "{message}"),
            Self::Conflict(error) => write!(f, "{}", error.message),
            Self::UnknownOperation(id) => write!(f, "unknown operation {}", id.as_str()),
            Self::IllegalTransition { id, from, to } => write!(
                f,
                "illegal operation transition for {}: {} -> {}",
                id.as_str(),
                from.raw_value(),
                to.raw_value()
            ),
        }
    }
}

impl std::error::Error for LifecycleOperationError {}

impl From<JournalError> for LifecycleOperationError {
    fn from(value: JournalError) -> Self {
        Self::Journal(value.to_string())
    }
}

pub struct LifecycleOperations<'fs> {
    journal: OperationJournal<'fs>,
    records: Mutex<HashMap<OperationId, OperationRecord>>,
    /// One cooperative-cancellation flag per operation, created alongside
    /// its record in [`Self::begin_running`] and never removed (the same
    /// "no eviction" lifetime `records` itself already has). Kept
    /// separate from `records`'s own `Mutex` so [`Self::cancellation_check`]
    /// can hand a worker a cheap, lock-free `'static` closure instead of a
    /// reference back into this store.
    cancel_flags: Mutex<HashMap<OperationId, Arc<AtomicBool>>>,
}

impl<'fs> LifecycleOperations<'fs> {
    pub fn new(fs: &'fs dyn FileSystem, dir: impl Into<PathBuf>) -> Self {
        Self {
            journal: OperationJournal::new(fs, dir),
            records: Mutex::new(HashMap::new()),
            cancel_flags: Mutex::new(HashMap::new()),
        }
    }

    pub fn reconcile_on_startup(
        &self,
    ) -> Result<Vec<ReconciliationRecord>, LifecycleOperationError> {
        let records = self.journal.reconcile_on_startup()?;
        for record in &records {
            if let Some(existing) = self.records.lock().unwrap().get_mut(&record.id) {
                existing.state = record.to;
                existing.status_line = Some(record.reason.clone());
                if record.to == OperationState::Failed {
                    existing.error = Some(interrupted_error(&record.reason));
                }
            }
        }
        Ok(records)
    }

    /// Admit the operation for its target and journal it as `running`
    /// before the caller starts mutating the server.
    pub fn begin_running(
        &self,
        operation_type: impl Into<String>,
        target: Option<String>,
        status_line: impl Into<String>,
    ) -> Result<OperationId, LifecycleOperationError> {
        let id = next_operation_id();
        let operation_type = operation_type.into();
        let status_line = status_line.into();
        let queued_entry = JournalEntry {
            id: id.clone(),
            operation_type: operation_type.clone(),
            target: target.clone(),
            state: OperationState::Queued,
            error: None,
        };
        self.journal
            .admit(&queued_entry)
            .map_err(|error| match error {
                AdmitError::Journal(error) => LifecycleOperationError::from(error),
                AdmitError::Conflict(error) => LifecycleOperationError::Conflict(error),
            })?;

        let running = OperationState::Queued
            .transition_to(OperationState::Running)
            .expect("queued->running is a legal operation transition");
        let record = OperationRecord {
            operation_type,
            target,
            state: running,
            progress: None,
            status_line: Some(status_line),
            result: None,
            error: None,
        };
        self.record_journal_state(&id, &record)?;
        self.records.lock().unwrap().insert(id.clone(), record);
        self.cancel_flags
            .lock()
            .unwrap()
            .insert(id.clone(), Arc::new(AtomicBool::new(false)));
        Ok(id)
    }

    pub fn set_progress(
        &self,
        id: &OperationId,
        current: u64,
        total: u64,
        status_line: impl Into<String>,
    ) -> Result<(), LifecycleOperationError> {
        let mut records = self.records.lock().unwrap();
        let record = records
            .get_mut(id)
            .ok_or_else(|| LifecycleOperationError::UnknownOperation(id.clone()))?;
        record.progress = Some(OperationProgress { current, total });
        record.status_line = Some(status_line.into());
        Ok(())
    }

    pub fn succeed(
        &self,
        id: &OperationId,
        status_line: impl Into<String>,
        result: BTreeMap<String, String>,
    ) -> Result<(), LifecycleOperationError> {
        self.transition_terminal(
            id,
            OperationState::Succeeded,
            status_line.into(),
            Some(result),
            None,
        )
    }

    pub fn fail(
        &self,
        id: &OperationId,
        error: OperationError,
    ) -> Result<(), LifecycleOperationError> {
        self.transition_terminal(
            id,
            OperationState::Failed,
            error.message.clone(),
            None,
            Some(error),
        )
    }

    pub fn cancel(
        &self,
        id: &OperationId,
        status_line: impl Into<String>,
    ) -> Result<(), LifecycleOperationError> {
        self.transition_terminal(
            id,
            OperationState::Cancelled,
            status_line.into(),
            None,
            None,
        )
    }

    /// Signal cooperative cancellation for a non-terminal operation and
    /// return the exact non-terminal snapshot accepted by that decision.
    /// Deliberately does **not** transition the record to `cancelled` or
    /// touch the durable journal — only the worker itself, once it has
    /// actually observed [`Self::cancellation_check`]'s flag at one of its
    /// own safe boundaries and stopped, calls [`Self::cancel`] to finalize
    /// the terminal state. Until then the record (and the journal's
    /// per-target admission it backs) stays `running`, so a second
    /// mutation against the same target keeps refusing exactly as it did
    /// before cancellation was requested — releasing that exclusivity
    /// early is the exact "truthful cancellation" gap this method closes.
    pub fn request_cancel(
        &self,
        id: &OperationId,
        status_line: impl Into<String>,
    ) -> Result<LifecycleOperationSnapshot, LifecycleOperationError> {
        let mut records = self.records.lock().unwrap();
        let Some(record) = records.get_mut(id) else {
            return match self.journal.load(id)? {
                Some(entry) if entry.state.is_terminal() => {
                    Err(LifecycleOperationError::IllegalTransition {
                        id: id.clone(),
                        from: entry.state,
                        to: OperationState::Cancelled,
                    })
                }
                _ => Err(LifecycleOperationError::UnknownOperation(id.clone())),
            };
        };
        if record.state.is_terminal() {
            return Err(LifecycleOperationError::IllegalTransition {
                id: id.clone(),
                from: record.state,
                to: OperationState::Cancelled,
            });
        }
        record.status_line = Some(status_line.into());
        self.cancel_flags
            .lock()
            .unwrap()
            .entry(id.clone())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .store(true, Ordering::SeqCst);

        // Worker terminal transitions take the same records lock. Clone the
        // accepted state before releasing it so a later transition cannot
        // rewrite the HTTP response that reports this cancellation decision.
        Ok(snapshot_from_record(id, record.clone()))
    }

    /// A cheap, lock-free, `'static` closure a real worker
    /// (`worlds::activate_slot`, `world_conversion::convert_world`,
    /// `backups::create_backup`, `backups::restore_backup`) can poll at
    /// its own safe boundaries as its `should_cancel` parameter, without
    /// holding a reference back into this store across a `tokio::spawn`/
    /// `spawn_blocking` boundary. An operation id this store has never
    /// seen (shouldn't happen — every caller gets one from
    /// [`Self::begin_running`] first) reports "never cancelled" rather
    /// than panicking.
    pub fn cancellation_check(
        &self,
        id: &OperationId,
    ) -> impl Fn() -> bool + Clone + Send + Sync + 'static {
        let flag = self
            .cancel_flags
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
        move || flag.load(Ordering::SeqCst)
    }

    pub fn snapshot(
        &self,
        id: &OperationId,
    ) -> Result<Option<LifecycleOperationSnapshot>, LifecycleOperationError> {
        if let Some(record) = self.records.lock().unwrap().get(id).cloned() {
            return Ok(Some(snapshot_from_record(id, record)));
        }

        Ok(self.journal.load(id)?.map(snapshot_from_journal_entry))
    }

    fn transition_terminal(
        &self,
        id: &OperationId,
        to: OperationState,
        status_line: String,
        result: Option<BTreeMap<String, String>>,
        error: Option<OperationError>,
    ) -> Result<(), LifecycleOperationError> {
        let mut records = self.records.lock().unwrap();
        let record = records
            .get_mut(id)
            .ok_or_else(|| LifecycleOperationError::UnknownOperation(id.clone()))?;
        let from = record.state;
        record.state =
            from.transition_to(to)
                .map_err(|_| LifecycleOperationError::IllegalTransition {
                    id: id.clone(),
                    from,
                    to,
                })?;
        record.status_line = Some(status_line);
        record.result = result;
        record.error = error;
        self.record_journal_state(id, record)
    }

    fn record_journal_state(
        &self,
        id: &OperationId,
        record: &OperationRecord,
    ) -> Result<(), LifecycleOperationError> {
        self.journal.record(&JournalEntry {
            id: id.clone(),
            operation_type: record.operation_type.clone(),
            target: record.target.clone(),
            state: record.state,
            error: record.error.clone(),
        })?;
        Ok(())
    }
}

pub fn lifecycle_error(code: impl Into<String>, message: impl Into<String>) -> OperationError {
    OperationError {
        code: code.into(),
        message: message.into(),
        help_id: None,
        details: BTreeMap::new(),
    }
}

fn next_operation_id() -> OperationId {
    OperationId::new(format!(
        "op-{}-{}",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

fn interrupted_error(reason: &str) -> OperationError {
    lifecycle_error("operation_interrupted", reason)
}

fn snapshot_from_record(id: &OperationId, record: OperationRecord) -> LifecycleOperationSnapshot {
    LifecycleOperationSnapshot {
        id: id.clone(),
        operation_type: record.operation_type,
        target: record.target,
        state: record.state,
        progress: record.progress,
        status_line: record.status_line,
        result: record.result,
        error: record.error,
    }
}

fn snapshot_from_journal_entry(entry: JournalEntry) -> LifecycleOperationSnapshot {
    LifecycleOperationSnapshot {
        id: entry.id,
        operation_type: entry.operation_type,
        target: entry.target,
        state: entry.state,
        progress: None,
        status_line: entry.error.as_ref().map(|error| error.message.clone()),
        result: None,
        error: entry.error,
    }
}
