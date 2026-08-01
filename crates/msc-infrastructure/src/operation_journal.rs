//! `OperationJournal`: restart survival and per-target exclusivity for
//! `msc_domain::operation`'s `OperationState` machine.
//!
//! Greenfield MSC 2 construction, not a port — MSC 1 has no
//! operation-journal concept, the same D-018 exemption P2.9 recorded for
//! `OperationState` itself. `msc2-engineering.md` §7: "incomplete
//! operations are reconciled and their outcome explained rather than
//! silently forgotten." This module is what makes that true: every
//! operation is journaled (via [`crate::atomic_write`]) before it begins,
//! and [`OperationJournal::reconcile_on_startup`] walks the journal on
//! agent startup, turning any entry still `queued` or `running` into a
//! terminal state instead of leaving it stuck or silently resuming it —
//! this phase has no real work-resumption mechanism to resume it under.
//!
//! Two different terminal states, not one, because
//! `OperationState::transition_to` (`msc-domain`) only allows `running ->
//! failed`, not `queued -> failed` — a `queued` operation that never
//! actually started is reconciled to `cancelled` instead, which *is* a
//! legal transition out of `queued`. This falls directly out of reusing
//! the domain type's own transition table rather than reaching around it.
//!
//! §7 also has a second sentence: "Only one conflicting operation runs
//! against a server at a time. Starting a backup during a world
//! replacement is refused, not queued silently." [`OperationJournal::admit`]
//! is what makes that true — the per-target exclusivity check `record`
//! alone doesn't perform, since `record` has to stay usable for
//! `reconcile_on_startup`'s own re-journaling of entries that have already
//! passed the check once.

use crate::atomic_write::{AtomicWriteError, atomic_write};
use crate::fs::FileSystem;
use msc_domain::operation::{OperationError, OperationId, OperationState};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// The reason attached to every entry reconciled by
/// [`OperationJournal::reconcile_on_startup`] — the same explanation
/// whether the entry lands on `failed` or `cancelled`, since in both
/// cases the true cause is the same: the agent restarted while this
/// operation was still open.
const RESTART_REASON: &str = "agent restarted mid-operation";

/// One journaled operation record — the durable half of
/// `msc_domain::operation`'s in-memory `OperationDTO` shape
/// (`operation-model.md` §2's `id`/`type`/`target`), plus `state` and,
/// once failed, the `error` that explains it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    pub id: OperationId,
    pub operation_type: String,
    pub target: Option<String>,
    pub state: OperationState,
    pub error: Option<OperationError>,
}

/// What [`OperationJournal::reconcile_on_startup`] did to one entry —
/// returned so the caller can explain it (to the audit log, to a client
/// polling `GET /v1/operations/{id}`) rather than let the reconciliation
/// happen silently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationRecord {
    pub id: OperationId,
    pub from: OperationState,
    pub to: OperationState,
    pub reason: String,
}

#[derive(Debug)]
pub enum JournalError {
    Io(io::Error),
    Parse(serde_json::Error),
    Write(AtomicWriteError),
}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JournalError::Io(err) => write!(f, "{err}"),
            JournalError::Parse(err) => write!(f, "{err}"),
            JournalError::Write(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for JournalError {}

/// What [`OperationJournal::admit`] returns instead of journaling a new
/// entry, per §7's per-target exclusivity rule. `Conflict` reuses
/// `OperationError` — already P2.4's `ErrorDTO` shape, per that type's own
/// doc comment — rather than inventing a second structured-error type for
/// the same wire shape.
#[derive(Debug)]
pub enum AdmitError {
    Journal(JournalError),
    Conflict(OperationError),
}

impl fmt::Display for AdmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdmitError::Journal(err) => write!(f, "{err}"),
            AdmitError::Conflict(err) => write!(f, "{}", err.message),
        }
    }
}

impl std::error::Error for AdmitError {}

/// One journal file per operation (`<dir>/<id>.json`), mirroring
/// [`crate::audit_log::AuditLog`]'s one-file-per-day convention: each
/// write is independent, so recording one operation's transition never
/// touches another's file. Like every other Phase 3 primitive built on
/// [`FileSystem`], `dir` must already exist — this type does not create
/// it (same "caller ensures the parent exists" rule
/// [`crate::atomic_write`], [`crate::config_repository`], and
/// [`crate::audit_log`] already established).
pub struct OperationJournal<'fs> {
    fs: &'fs dyn FileSystem,
    dir: PathBuf,
}

impl<'fs> OperationJournal<'fs> {
    pub fn new(fs: &'fs dyn FileSystem, dir: impl Into<PathBuf>) -> Self {
        Self {
            fs,
            dir: dir.into(),
        }
    }

    fn entry_path(&self, id: &OperationId) -> PathBuf {
        self.dir.join(format!("{}.json", id.as_str()))
    }

    /// Journals `entry`, creating or overwriting its record via
    /// [`atomic_write`] — a reader (including a concurrent
    /// `reconcile_on_startup`) never observes a half-written entry. Must
    /// be called before an operation begins doing real work (§7: "every
    /// long operation journaled before it begins") and again on every
    /// later state transition, so the journal's on-disk state always
    /// matches the operation's true last-known state.
    pub fn record(&self, entry: &JournalEntry) -> Result<(), JournalError> {
        let bytes = serde_json::to_vec_pretty(&entry_to_value(entry))
            .expect("JournalEntry always serializes to valid JSON");
        atomic_write(self.fs, &self.entry_path(&entry.id), &bytes).map_err(JournalError::Write)
    }

    /// Journals `entry`, first enforcing `msc2-engineering.md` §7's
    /// per-target exclusivity rule: "Only one conflicting operation runs
    /// against a server at a time. Starting a backup during a world
    /// replacement is refused, not queued silently." If any other
    /// non-terminal (`queued`/`running`) entry already shares `entry`'s
    /// target, `entry` is rejected with [`AdmitError::Conflict`] instead of
    /// being journaled — refused outright, never silently queued behind
    /// the existing operation — and the existing entry is left untouched.
    ///
    /// An entry with no target (`target: None`) never conflicts with
    /// anything and is always admitted: there is no shared target to hold
    /// exclusively. The rule itself is deliberately coarse —
    /// same-target-any-operation conflicts with same-target-any-operation
    /// — since the real operation-type catalog (which types may safely
    /// coexist against the same target) doesn't exist until later phases
    /// populate it.
    pub fn admit(&self, entry: &JournalEntry) -> Result<(), AdmitError> {
        if let Some(target) = &entry.target
            && let Some(existing) = self
                .find_non_terminal_for_target(target, &entry.id)
                .map_err(AdmitError::Journal)?
        {
            return Err(AdmitError::Conflict(conflict_error(&existing, target)));
        }
        self.record(entry).map_err(AdmitError::Journal)
    }

    /// The first non-terminal journaled entry (other than `exclude_id`)
    /// whose target matches `target`, or `None` if there isn't one.
    /// Shares `reconcile_on_startup`'s walk-every-file-under-`dir` shape,
    /// skipping anything that isn't a journal entry this module wrote —
    /// same reasoning as there.
    fn find_non_terminal_for_target(
        &self,
        target: &str,
        exclude_id: &OperationId,
    ) -> Result<Option<JournalEntry>, JournalError> {
        for path in self.fs.list(&self.dir).map_err(JournalError::Io)? {
            let bytes = match self.fs.read(&path) {
                Ok(bytes) => bytes,
                Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => return Err(JournalError::Io(err)),
            };
            let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
                continue; // not a journal file this module wrote; leave it alone
            };
            let Some(candidate) = entry_from_value(&value) else {
                continue;
            };
            if &candidate.id != exclude_id
                && candidate.target.as_deref() == Some(target)
                && !candidate.state.is_terminal()
            {
                return Ok(Some(candidate));
            }
        }
        Ok(None)
    }

    /// Reads back `id`'s journaled entry, or `None` if it was never
    /// journaled (or its file has since been removed).
    pub fn load(&self, id: &OperationId) -> Result<Option<JournalEntry>, JournalError> {
        match self.fs.read(&self.entry_path(id)) {
            Ok(bytes) => {
                let value: Value = serde_json::from_slice(&bytes).map_err(JournalError::Parse)?;
                Ok(entry_from_value(&value))
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(JournalError::Io(err)),
        }
    }

    /// Walks every journaled entry under `dir` on agent startup.
    ///
    /// - A `succeeded`/`failed`/`cancelled` entry is already terminal —
    ///   left exactly as it is on disk (inert).
    /// - A `running` entry is reconciled to `failed`, carrying a fresh
    ///   [`OperationError`] (`code: "operation_interrupted"`,
    ///   `message: "agent restarted mid-operation"`) — real work was in
    ///   flight and cannot be assumed to have completed.
    /// - A `queued` entry — never actually started — is reconciled to
    ///   `cancelled` instead of `failed`, since `queued -> failed` isn't
    ///   a legal transition (`OperationState::transition_to`) and there
    ///   is nothing to silently resume it into.
    ///
    /// Each reconciled entry is re-journaled with its new state before
    /// this returns, so the on-disk journal reflects the reconciliation,
    /// not just the in-memory report. Returns one [`ReconciliationRecord`]
    /// per entry actually reconciled — empty if every journaled entry was
    /// already terminal.
    pub fn reconcile_on_startup(&self) -> Result<Vec<ReconciliationRecord>, JournalError> {
        let mut records = Vec::new();
        for path in self.fs.list(&self.dir).map_err(JournalError::Io)? {
            let bytes = match self.fs.read(&path) {
                Ok(bytes) => bytes,
                Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => return Err(JournalError::Io(err)),
            };
            let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
                continue; // not a journal file this module wrote; leave it alone
            };
            let Some(mut entry) = entry_from_value(&value) else {
                continue;
            };
            let Some(target) = reconciliation_target(entry.state) else {
                continue; // already terminal, inert
            };

            let from = entry.state;
            entry.state = from
                .transition_to(target)
                .expect("reconciliation_target only ever names a legal transition");
            if entry.state == OperationState::Failed {
                entry.error = Some(OperationError {
                    code: "operation_interrupted".to_string(),
                    message: RESTART_REASON.to_string(),
                    help_id: None,
                    details: BTreeMap::new(),
                });
            }

            self.record(&entry)?;
            records.push(ReconciliationRecord {
                id: entry.id,
                from,
                to: target,
                reason: RESTART_REASON.to_string(),
            });
        }
        Ok(records)
    }
}

/// The terminal state a non-terminal `state` reconciles to on restart, or
/// `None` if `state` is already terminal. The only two cases this phase's
/// state machine actually allows — see the module docs for why `queued`
/// doesn't also go to `failed`.
fn reconciliation_target(state: OperationState) -> Option<OperationState> {
    match state {
        OperationState::Running => Some(OperationState::Failed),
        OperationState::Queued => Some(OperationState::Cancelled),
        OperationState::Succeeded | OperationState::Failed | OperationState::Cancelled => None,
    }
}

/// The structured conflict error [`OperationJournal::admit`] returns when
/// `existing` already holds `target` — P2.4's `ErrorDTO` shape, same as
/// `reconcile_on_startup`'s `operation_interrupted` error above. `code` is
/// this step's own choice: `operation-model.md` §4.3 leaves "what
/// `ErrorDTO.code` a conflicting request gets back" to whichever phase
/// actually implements exclusivity, which is this one.
fn conflict_error(existing: &JournalEntry, target: &str) -> OperationError {
    let mut details = BTreeMap::new();
    details.insert("target".to_string(), target.to_string());
    details.insert(
        "conflictingOperationId".to_string(),
        existing.id.as_str().to_string(),
    );
    OperationError {
        code: "operation_conflict".to_string(),
        message: format!(
            "a {} operation ({}) is already {} against {target}",
            existing.operation_type,
            existing.id.as_str(),
            existing.state.raw_value(),
        ),
        help_id: None,
        details,
    }
}

fn entry_to_value(entry: &JournalEntry) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert(
        "id".to_string(),
        Value::String(entry.id.as_str().to_string()),
    );
    obj.insert(
        "operationType".to_string(),
        Value::String(entry.operation_type.clone()),
    );
    obj.insert(
        "target".to_string(),
        entry
            .target
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    obj.insert(
        "state".to_string(),
        Value::String(entry.state.raw_value().to_string()),
    );
    obj.insert(
        "error".to_string(),
        entry
            .error
            .as_ref()
            .map(error_to_value)
            .unwrap_or(Value::Null),
    );
    Value::Object(obj)
}

fn error_to_value(err: &OperationError) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert("code".to_string(), Value::String(err.code.clone()));
    obj.insert("message".to_string(), Value::String(err.message.clone()));
    obj.insert(
        "helpId".to_string(),
        err.help_id
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    let details = err
        .details
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    obj.insert("details".to_string(), Value::Object(details));
    Value::Object(obj)
}

fn entry_from_value(value: &Value) -> Option<JournalEntry> {
    let obj = value.as_object()?;
    let id = obj.get("id")?.as_str()?.to_string();
    let operation_type = obj.get("operationType")?.as_str()?.to_string();
    let target = obj
        .get("target")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let state = OperationState::from_raw_value(obj.get("state")?.as_str()?)?;
    let error = obj
        .get("error")
        .filter(|v| !v.is_null())
        .and_then(error_from_value);
    Some(JournalEntry {
        id: OperationId::new(id),
        operation_type,
        target,
        state,
        error,
    })
}

fn error_from_value(value: &Value) -> Option<OperationError> {
    let obj = value.as_object()?;
    let code = obj.get("code")?.as_str()?.to_string();
    let message = obj.get("message")?.as_str()?.to_string();
    let help_id = obj
        .get("helpId")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let details = obj
        .get("details")
        .and_then(|v| v.as_object())
        .map(|map| {
            map.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();
    Some(OperationError {
        code,
        message,
        help_id,
        details,
    })
}
