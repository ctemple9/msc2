use msc_application::operations::{LifecycleOperationError, LifecycleOperations, lifecycle_error};
use msc_application::provisioning::CREATE_OPERATION_TYPE;
use msc_domain::operation::{OperationId, OperationState};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use std::sync::{Arc, Barrier};

const DIR: &str = "/srv/agent/operations";

fn make_operations(fs: &FakeFileSystem) -> LifecycleOperations<'_> {
    LifecycleOperations::new(fs, DIR)
}

#[test]
fn lifecycle_operations_start_is_journaled_running_before_mutation_completes() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let operations = make_operations(&fs);

    let id = operations
        .begin_running(
            "java-start",
            Some("paper-1".to_string()),
            "Starting Java server.",
        )
        .unwrap();

    let snapshot = operations.snapshot(&id).unwrap().expect("operation exists");
    assert_eq!(snapshot.operation_type, "java-start");
    assert_eq!(snapshot.target.as_deref(), Some("paper-1"));
    assert_eq!(snapshot.state, OperationState::Running);

    let restarted_operations = make_operations(&fs);
    let restarted = restarted_operations
        .snapshot(&OperationId::new(id.as_str().to_string()))
        .unwrap()
        .expect("journal fallback can load by id");
    assert_eq!(restarted.state, OperationState::Running);
}

#[test]
fn lifecycle_operations_same_server_conflict_is_refused_not_queued() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let operations = make_operations(&fs);
    let first = operations
        .begin_running("java-start", Some("paper-1".to_string()), "Starting.")
        .unwrap();

    let error = operations
        .begin_running("java-restart", Some("paper-1".to_string()), "Restarting.")
        .expect_err("same target should conflict");

    let LifecycleOperationError::Conflict(conflict) = error else {
        panic!("expected operation conflict");
    };
    assert_eq!(conflict.code, "operation_conflict");
    assert_eq!(
        conflict
            .details
            .get("conflictingOperationId")
            .map(String::as_str),
        Some(first.as_str())
    );
}

#[test]
fn lifecycle_operations_terminal_operation_releases_target() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let operations = make_operations(&fs);
    let first = operations
        .begin_running("java-start", Some("paper-1".to_string()), "Starting.")
        .unwrap();
    operations
        .succeed(&first, "Java server is ready.", Default::default())
        .unwrap();

    let second = operations
        .begin_running("java-restart", Some("paper-1".to_string()), "Restarting.")
        .unwrap();

    assert_ne!(first, second);
    assert_eq!(
        operations.snapshot(&second).unwrap().unwrap().state,
        OperationState::Running
    );
}

#[test]
fn lifecycle_operations_restart_reconciliation_marks_running_lifecycle_failed() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let operations = make_operations(&fs);
    let id = operations
        .begin_running("java-restart", Some("paper-1".to_string()), "Restarting.")
        .unwrap();

    let reconciled = operations.reconcile_on_startup().unwrap();

    assert_eq!(reconciled.len(), 1);
    assert_eq!(reconciled[0].id, id);
    assert_eq!(reconciled[0].from, OperationState::Running);
    assert_eq!(reconciled[0].to, OperationState::Failed);

    let snapshot = operations.snapshot(&id).unwrap().unwrap();
    assert_eq!(snapshot.state, OperationState::Failed);
    assert_eq!(snapshot.error.unwrap().code, "operation_interrupted");
}

/// P6.30: requesting cancellation signals the worker's flag but does
/// *not* itself transition the record — the operation stays `running`
/// (and its journal admission still exclusive) until the worker that
/// owns it actually observes the flag and calls `cancel()` itself.
#[test]
fn lifecycle_operations_request_cancel_does_not_transition_state() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let operations = make_operations(&fs);
    let id = operations
        .begin_running("world-activate", Some("paper-1".to_string()), "Working.")
        .unwrap();

    let accepted = operations.request_cancel(&id, "Cancelling…").unwrap();

    let snapshot = operations.snapshot(&id).unwrap().unwrap();
    assert_eq!(accepted, snapshot);
    assert_eq!(snapshot.state, OperationState::Running);
    assert_eq!(snapshot.status_line.as_deref(), Some("Cancelling…"));
    assert!(operations.cancellation_check(&id)());

    // A second mutation against the same target is still refused —
    // cancellation was only requested, not finalized, so exclusivity
    // hasn't been released.
    let error = operations
        .begin_running("world-activate", Some("paper-1".to_string()), "Working.")
        .expect_err("target is still held while cancellation is pending");
    assert!(matches!(error, LifecycleOperationError::Conflict(_)));
}

/// The worker itself, once it observes the flag at its own safe
/// boundary, finalizes cancellation by calling `cancel()` — only then
/// does the record go terminal and the target free up.
#[test]
fn lifecycle_operations_worker_finalized_cancel_transitions_to_cancelled_and_frees_target() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let operations = make_operations(&fs);
    let id = operations
        .begin_running("world-activate", Some("paper-1".to_string()), "Working.")
        .unwrap();
    operations.request_cancel(&id, "Cancelling…").unwrap();

    let should_cancel = operations.cancellation_check(&id);
    assert!(should_cancel());
    operations.cancel(&id, "Activation cancelled.").unwrap();

    let snapshot = operations.snapshot(&id).unwrap().unwrap();
    assert_eq!(snapshot.state, OperationState::Cancelled);

    let second = operations
        .begin_running("world-activate", Some("paper-1".to_string()), "Working.")
        .expect("target is free once the worker finalized cancellation");
    assert_ne!(id, second);
}

/// Requesting cancellation against an operation that has already reached
/// a terminal state (e.g. it succeeded before the request landed) is
/// refused, not silently accepted.
#[test]
fn lifecycle_operations_request_cancel_against_terminal_operation_is_refused() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let operations = make_operations(&fs);
    let id = operations
        .begin_running("world-activate", Some("paper-1".to_string()), "Working.")
        .unwrap();
    operations
        .succeed(&id, "Activation complete.", Default::default())
        .unwrap();

    let error = operations
        .request_cancel(&id, "Cancelling…")
        .expect_err("cancellation is not legal against a terminal operation");
    assert!(matches!(
        error,
        LifecycleOperationError::IllegalTransition { .. }
    ));
}

/// Cancellation admission and a worker's terminal transition contend on the
/// same record lock. Whichever acquires it first determines the wire outcome:
/// terminal-first is a conflict, while cancellation-first returns the owned
/// non-terminal snapshot captured before the worker can update the record.
#[test]
fn lifecycle_operations_cancel_and_terminal_race_has_only_atomic_outcomes() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let operations = make_operations(&fs);

    for attempt in 0..128 {
        let id = operations
            .begin_running(
                "world-activate",
                Some(format!("paper-{attempt}")),
                "Working.",
            )
            .unwrap();
        let barrier = Arc::new(Barrier::new(3));

        let (cancel_result, terminal_result) = std::thread::scope(|scope| {
            let cancel_barrier = Arc::clone(&barrier);
            let cancel_operations = &operations;
            let cancel_id = &id;
            let cancel = scope.spawn(move || {
                cancel_barrier.wait();
                cancel_operations.request_cancel(cancel_id, "Cancelling…")
            });
            let terminal_barrier = Arc::clone(&barrier);
            let terminal_operations = &operations;
            let terminal_id = &id;
            let terminal = scope.spawn(move || {
                terminal_barrier.wait();
                if attempt % 2 == 0 {
                    terminal_operations.succeed(
                        terminal_id,
                        "Activation complete.",
                        Default::default(),
                    )
                } else {
                    terminal_operations.fail(
                        terminal_id,
                        lifecycle_error("activation_failed", "Activation failed."),
                    )
                }
            });

            barrier.wait();
            (cancel.join().unwrap(), terminal.join().unwrap())
        });

        terminal_result.expect("a cancellation request does not fabricate a terminal state");
        match cancel_result {
            Ok(snapshot) => {
                assert_eq!(snapshot.state, OperationState::Running);
                assert_eq!(snapshot.status_line.as_deref(), Some("Cancelling…"));
                assert!(snapshot.result.is_none());
                assert!(snapshot.error.is_none());
            }
            Err(LifecycleOperationError::IllegalTransition {
                from: OperationState::Succeeded | OperationState::Failed,
                to: OperationState::Cancelled,
                ..
            }) => {}
            other => panic!("unexpected cancellation race outcome: {other:?}"),
        }
    }
}

/// P7.33: a `"server-create"` operation still `running` when the agent
/// restarts is reconciled to `Failed` exactly as before, but now also
/// sweeps the half-provisioned server directory it named — the gap
/// `docs/msc2/families/phase7-scope.md`'s P7.1 note flagged and P7.30's
/// gate audit confirmed was still open.
#[test]
fn lifecycle_operations_reconcile_sweeps_orphaned_create_directory() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{DIR}/.keep"), Vec::new(), false)
        .with_dir("/servers/java/orphaned_server");
    let operations = LifecycleOperations::new(&fs, DIR).with_servers_root("/servers");
    let id = operations
        .begin_running(
            CREATE_OPERATION_TYPE,
            Some("orphaned_server".to_string()),
            "Creating server.",
        )
        .unwrap();

    let reconciled = operations.reconcile_on_startup().unwrap();

    assert_eq!(reconciled.len(), 1);
    assert_eq!(reconciled[0].id, id);
    assert_eq!(reconciled[0].to, OperationState::Failed);
    assert!(
        fs.stat(std::path::Path::new("/servers/java/orphaned_server"))
            .is_err(),
        "the half-provisioned directory should have been swept"
    );
}

/// Without `with_servers_root`, reconciliation still corrects the
/// operation but never touches the filesystem — the exact pre-P7.33
/// behavior, preserved for every operation store that has no server
/// directory to sweep (world/backup-only stores, the `demo-install`
/// ticker).
#[test]
fn lifecycle_operations_reconcile_without_servers_root_leaves_directory_untouched() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{DIR}/.keep"), Vec::new(), false)
        .with_dir("/servers/java/orphaned_server");
    let operations = make_operations(&fs);
    operations
        .begin_running(
            CREATE_OPERATION_TYPE,
            Some("orphaned_server".to_string()),
            "Creating server.",
        )
        .unwrap();

    operations.reconcile_on_startup().unwrap();

    assert!(
        fs.stat(std::path::Path::new("/servers/java/orphaned_server"))
            .is_ok(),
        "no servers_root configured -- the directory must be left alone"
    );
}

/// Only a reconciled `"server-create"` operation triggers the sweep — a
/// different operation type that happens to share a target string (e.g.
/// a `world-activate` against a server whose folder name matches) must
/// never have its own directory removed.
#[test]
fn lifecycle_operations_reconcile_only_sweeps_server_create_operations() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{DIR}/.keep"), Vec::new(), false)
        .with_dir("/servers/java/paper-1");
    let operations = LifecycleOperations::new(&fs, DIR).with_servers_root("/servers");
    operations
        .begin_running("world-activate", Some("paper-1".to_string()), "Working.")
        .unwrap();

    operations.reconcile_on_startup().unwrap();

    assert!(
        fs.stat(std::path::Path::new("/servers/java/paper-1"))
            .is_ok(),
        "a non-server-create operation must never sweep its target's directory"
    );
}

#[test]
fn lifecycle_operations_failure_is_persisted_to_journal() {
    let fs = FakeFileSystem::new().with_file(format!("{DIR}/.keep"), Vec::new(), false);
    let operations = make_operations(&fs);
    let id = operations
        .begin_running("paper-import", Some("/srv/paper".to_string()), "Importing.")
        .unwrap();

    operations
        .fail(&id, lifecycle_error("import_error", "No Paper jar found."))
        .unwrap();

    let snapshot = operations.snapshot(&id).unwrap().unwrap();
    assert_eq!(snapshot.state, OperationState::Failed);
    assert_eq!(snapshot.error.unwrap().code, "import_error");
}
