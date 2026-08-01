//! Hand-written coverage of `OperationJournal::admit`, in the same style
//! `tests/operation_journal.rs` (P3.15) used: no MSC 1 fixture to port from
//! (greenfield MSC 2 construction — see the module docs), so these exercise
//! the implementation directly against its own written contract
//! (`msc2-engineering.md` §7: "Only one conflicting operation runs against
//! a server at a time. Starting a backup during a world replacement is
//! refused, not queued silently.").
//!
//! Test functions are prefixed `operation_exclusivity_` so the plan's
//! Verify command (a plain nextest substring filter) selects all of them.

use msc_domain::operation::{OperationId, OperationState};
use msc_infrastructure::fs::FakeFileSystem;
use msc_infrastructure::operation_journal::{AdmitError, JournalEntry, OperationJournal};

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
fn operation_exclusivity_same_target_running_operation_rejects_new_one() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let journal = journal(&fs);
    let backup = entry(
        "op-backup",
        "backup",
        Some("survival2"),
        OperationState::Running,
    );
    journal.admit(&backup).expect("first operation is admitted");

    let world_replace = entry(
        "op-world-replace",
        "world-replace",
        Some("survival2"),
        OperationState::Queued,
    );
    let err = journal
        .admit(&world_replace)
        .expect_err("a conflicting same-target operation must be refused");

    let AdmitError::Conflict(conflict) = err else {
        panic!("expected AdmitError::Conflict, got a journal error instead");
    };
    assert_eq!(conflict.code, "operation_conflict");
    assert_eq!(
        conflict
            .details
            .get("conflictingOperationId")
            .map(String::as_str),
        Some("op-backup")
    );
    assert_eq!(
        conflict.details.get("target").map(String::as_str),
        Some("survival2")
    );

    // Refused, not queued silently: the new operation was never journaled.
    assert_eq!(
        journal.load(&world_replace.id).expect("load"),
        None,
        "a refused operation must not be journaled at all"
    );
    // The existing operation is left exactly as it was.
    assert_eq!(
        journal.load(&backup.id).expect("load").expect("exists"),
        backup
    );
}

#[test]
fn operation_exclusivity_same_target_queued_operation_also_rejects_new_one() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let journal = journal(&fs);
    let first = entry(
        "op-first",
        "backup",
        Some("creative1"),
        OperationState::Queued,
    );
    journal.admit(&first).expect("first operation is admitted");

    let second = entry(
        "op-second",
        "backup",
        Some("creative1"),
        OperationState::Queued,
    );
    let err = journal
        .admit(&second)
        .expect_err("a queued (not just running) same-target operation still conflicts");

    assert!(matches!(err, AdmitError::Conflict(_)));
}

#[test]
fn operation_exclusivity_different_target_admits_successfully() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let journal = journal(&fs);
    let backup = entry(
        "op-backup",
        "backup",
        Some("survival2"),
        OperationState::Running,
    );
    journal.admit(&backup).expect("first operation is admitted");

    let other_server = entry(
        "op-other-server",
        "backup",
        Some("creative1"),
        OperationState::Queued,
    );
    journal
        .admit(&other_server)
        .expect("a different target never conflicts");

    assert_eq!(
        journal.load(&other_server.id).expect("load"),
        Some(other_server)
    );
}

#[test]
fn operation_exclusivity_terminal_same_target_operation_does_not_block() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let journal = journal(&fs);
    let finished = entry(
        "op-finished",
        "backup",
        Some("survival2"),
        OperationState::Succeeded,
    );
    journal
        .admit(&finished)
        .expect("first operation is admitted");

    let next = entry(
        "op-next",
        "backup",
        Some("survival2"),
        OperationState::Queued,
    );
    journal
        .admit(&next)
        .expect("a terminal same-target operation does not hold the target");

    assert_eq!(journal.load(&next.id).expect("load"), Some(next));
}

#[test]
fn operation_exclusivity_untargeted_operations_never_conflict() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let journal = journal(&fs);
    let first = entry("op-first", "java-download", None, OperationState::Running);
    journal.admit(&first).expect("first operation is admitted");

    let second = entry("op-second", "java-download", None, OperationState::Running);
    journal
        .admit(&second)
        .expect("operations with no target never conflict with anything");

    assert_eq!(journal.load(&second.id).expect("load"), Some(second));
}
