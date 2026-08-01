//! Hand-written coverage of `OperationJournal`, in the same style P2.9's
//! `crates/msc-domain/tests/operation.rs` used for `OperationState`: no
//! MSC 1 fixture to port from (greenfield MSC 2 construction — see the
//! module docs), so these exercise the implementation directly against
//! its own written contract (`msc2-engineering.md` §7's "incomplete
//! operations are reconciled and their outcome explained rather than
//! silently forgotten").
//!
//! Test functions are prefixed `operation_journal_` so the plan's Verify
//! command (a plain nextest substring filter) selects all of them.

use msc_domain::operation::{OperationError, OperationId, OperationState};
use msc_infrastructure::fs::FakeFileSystem;
use msc_infrastructure::operation_journal::{JournalEntry, OperationJournal};
use std::collections::BTreeMap;

const DIR: &str = "/srv/agent/operations";

fn entry(
    id: &str,
    operation_type: &str,
    target: Option<&str>,
    state: OperationState,
) -> JournalEntry {
    JournalEntry {
        id: OperationId::new(id),
        operation_type: operation_type.to_string(),
        target: target.map(str::to_string),
        state,
        error: None,
    }
}

fn journal(fs: &FakeFileSystem) -> OperationJournal<'_> {
    OperationJournal::new(fs, DIR)
}

#[test]
fn operation_journal_record_and_load_round_trips() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let journal = journal(&fs);
    let original = entry(
        "op-1",
        "demo-install",
        Some("survival2"),
        OperationState::Queued,
    );

    journal.record(&original).expect("record");
    let loaded = journal
        .load(&original.id)
        .expect("load")
        .expect("entry exists");

    assert_eq!(loaded, original);
}

#[test]
fn operation_journal_load_of_never_journaled_id_returns_none() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let journal = journal(&fs);

    let loaded = journal
        .load(&OperationId::new("never-journaled"))
        .expect("load");

    assert_eq!(loaded, None);
}

#[test]
fn operation_journal_completed_entry_is_inert_on_restart() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let journal = journal(&fs);

    let succeeded = entry(
        "op-succeeded",
        "demo-install",
        Some("survival2"),
        OperationState::Succeeded,
    );
    let failed = JournalEntry {
        error: Some(OperationError {
            code: "boom".to_string(),
            message: "already failed before restart".to_string(),
            help_id: None,
            details: BTreeMap::new(),
        }),
        ..entry("op-failed", "demo-install", None, OperationState::Failed)
    };
    let cancelled = entry(
        "op-cancelled",
        "demo-install",
        None,
        OperationState::Cancelled,
    );

    for e in [&succeeded, &failed, &cancelled] {
        journal.record(e).expect("record");
    }

    let records = journal.reconcile_on_startup().expect("reconcile");
    assert_eq!(
        records,
        Vec::new(),
        "no already-terminal entry should be reconciled"
    );

    for e in [&succeeded, &failed, &cancelled] {
        let reloaded = journal.load(&e.id).expect("load").expect("entry exists");
        assert_eq!(
            &reloaded, e,
            "terminal entry {:?} must be left exactly as it was",
            e.id
        );
    }
}

#[test]
fn operation_journal_running_entry_is_reconciled_to_failed() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let journal = journal(&fs);
    let running = entry(
        "op-running",
        "modpack-install",
        Some("survival2"),
        OperationState::Running,
    );
    journal.record(&running).expect("record");

    let records = journal.reconcile_on_startup().expect("reconcile");

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, running.id);
    assert_eq!(records[0].from, OperationState::Running);
    assert_eq!(records[0].to, OperationState::Failed);
    assert_eq!(records[0].reason, "agent restarted mid-operation");

    let reconciled = journal
        .load(&running.id)
        .expect("load")
        .expect("entry exists");
    assert_eq!(reconciled.state, OperationState::Failed);
    let error = reconciled
        .error
        .expect("a reconciled running entry carries an explanatory error");
    assert_eq!(error.code, "operation_interrupted");
    assert_eq!(error.message, "agent restarted mid-operation");
}

#[test]
fn operation_journal_queued_entry_is_reconciled_rather_than_silently_resumed() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let journal = journal(&fs);
    let queued = entry("op-queued", "java-download", None, OperationState::Queued);
    journal.record(&queued).expect("record");

    let records = journal.reconcile_on_startup().expect("reconcile");

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id, queued.id);
    assert_eq!(records[0].from, OperationState::Queued);
    // Not `failed`: `queued -> failed` isn't a legal transition
    // (`OperationState::transition_to`) — `cancelled` is the terminal
    // state this phase's state machine actually allows for an operation
    // that never started, and it's what proves this wasn't silently
    // resumed instead.
    assert_eq!(records[0].to, OperationState::Cancelled);

    let reconciled = journal
        .load(&queued.id)
        .expect("load")
        .expect("entry exists");
    assert_eq!(reconciled.state, OperationState::Cancelled);
    // Cancellation carries neither result nor error, per
    // operation-model.md §2 — unlike the `running -> failed` case above.
    assert_eq!(reconciled.error, None);
}

#[test]
fn operation_journal_reconcile_on_startup_reconciles_every_non_terminal_entry() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let journal = journal(&fs);
    let running = entry("op-a", "demo-install", None, OperationState::Running);
    let queued = entry("op-b", "demo-install", None, OperationState::Queued);
    let succeeded = entry("op-c", "demo-install", None, OperationState::Succeeded);
    for e in [&running, &queued, &succeeded] {
        journal.record(e).expect("record");
    }

    let mut records = journal.reconcile_on_startup().expect("reconcile");
    records.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));

    assert_eq!(
        records.len(),
        2,
        "only the two non-terminal entries reconcile"
    );
    assert_eq!(records[0].id, running.id);
    assert_eq!(records[0].to, OperationState::Failed);
    assert_eq!(records[1].id, queued.id);
    assert_eq!(records[1].to, OperationState::Cancelled);
}
