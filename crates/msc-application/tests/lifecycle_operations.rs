use msc_application::operations::{LifecycleOperationError, LifecycleOperations, lifecycle_error};
use msc_domain::operation::{OperationId, OperationState};
use msc_infrastructure::fs::FakeFileSystem;

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
