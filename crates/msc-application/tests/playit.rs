use msc_application::operations::LifecycleOperations;
use msc_application::playit::{PLAYIT_OPERATION_TYPE, PlayitError, PlayitService};
use msc_domain::helper::HelperStatus;
use msc_domain::operation::OperationState;
use msc_infrastructure::fs::FakeFileSystem;
use msc_infrastructure::playit::{PLAYIT_SECRET_KEY, PlayitLaunch};
use msc_infrastructure::process::FakeProcessSupervisor;
use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};
use std::path::PathBuf;

const OPERATIONS_DIR: &str = "/agent/operations";

fn launch() -> PlayitLaunch {
    PlayitLaunch {
        executable_path: PathBuf::from("/agent/bin/playitd"),
        working_directory: PathBuf::from("/agent"),
        secret_path: PathBuf::from("/agent/secret-bridge/playit"),
    }
}

#[test]
fn playit_start_is_journaled_and_secret_never_reaches_process_arguments() {
    let fs = FakeFileSystem::new().with_file(format!("{OPERATIONS_DIR}/.keep"), [], false);
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let secrets = FakeSecretStore::new();
    secrets
        .set(PLAYIT_SECRET_KEY, "playit-secret-must-not-leak")
        .unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);

    let started = service.start(launch()).unwrap();
    let operation_id = started.operation_id.unwrap();
    assert_eq!(started.status, HelperStatus::Starting);
    assert_eq!(
        operations
            .snapshot(&msc_domain::operation::OperationId::new(operation_id))
            .unwrap()
            .unwrap()
            .operation_type,
        PLAYIT_OPERATION_TYPE
    );

    let (_, request) = supervisor.spawned_requests().pop().unwrap();
    assert_eq!(
        request.arguments,
        ["--secret-path", "/agent/secret-bridge/playit"]
    );
    assert!(!format!("{request:?}").contains("playit-secret-must-not-leak"));
}

#[test]
fn playit_readiness_is_a_player_address_not_a_management_address() {
    let fs = FakeFileSystem::new().with_file(format!("{OPERATIONS_DIR}/.keep"), [], false);
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);
    let operation_id = service.start(launch()).unwrap().operation_id.unwrap();

    service.observe_output("tunnel setup").unwrap();
    service.observe_output("join.example.joinmc.link").unwrap();

    assert_eq!(service.status().status, HelperStatus::Running);
    assert_eq!(
        service.status().player_address.as_deref(),
        Some("join.example.joinmc.link")
    );
    assert_eq!(
        operations
            .snapshot(&msc_domain::operation::OperationId::new(operation_id))
            .unwrap()
            .unwrap()
            .state,
        OperationState::Succeeded
    );
}

#[test]
fn playit_start_can_be_cancelled_before_ready_and_restart_is_unknown() {
    let fs = FakeFileSystem::new().with_file(format!("{OPERATIONS_DIR}/.keep"), [], false);
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);
    let operation_id = service.start(launch()).unwrap().operation_id.unwrap();
    let operation_id = msc_domain::operation::OperationId::new(operation_id);

    operations
        .request_cancel(&operation_id, "Cancelling Playit.")
        .unwrap();
    assert!(service.cancel_start_if_requested().unwrap());
    assert_eq!(
        operations.snapshot(&operation_id).unwrap().unwrap().state,
        OperationState::Cancelled
    );

    service.recover_after_restart();
    assert_eq!(
        service.status().status,
        HelperStatus::UnknownUntilReconciled
    );
}

#[test]
fn playit_ready_signal_times_out_at_the_msc1_watchdog_boundary() {
    let fs = FakeFileSystem::new().with_file(format!("{OPERATIONS_DIR}/.keep"), [], false);
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);
    let operation_id = msc_domain::operation::OperationId::new(
        service.start(launch()).unwrap().operation_id.unwrap(),
    );

    assert!(!service.ready_timeout_elapsed(74).unwrap());
    assert!(service.ready_timeout_elapsed(75).unwrap());
    assert_eq!(service.status().status, HelperStatus::TimedOut);
    let operation = operations.snapshot(&operation_id).unwrap().unwrap();
    assert_eq!(operation.state, OperationState::Failed);
    assert_eq!(operation.error.unwrap().code, "playit_ready_timeout");
}

#[test]
fn disabled_or_unconfigured_playit_does_not_spawn_a_process() {
    let fs = FakeFileSystem::new().with_file(format!("{OPERATIONS_DIR}/.keep"), [], false);
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let secrets = FakeSecretStore::new();
    let mut service = PlayitService::new("paper-1", false, &supervisor, &secrets, &operations);
    assert_eq!(service.start(launch()).unwrap_err(), PlayitError::Disabled);

    let mut enabled = PlayitService::new("paper-2", true, &supervisor, &secrets, &operations);
    assert_eq!(
        enabled.start(launch()).unwrap_err(),
        PlayitError::MissingSecret
    );
    assert!(supervisor.spawned_requests().is_empty());
}

#[test]
fn playit_secret_is_trimmed_and_only_the_secret_store_retains_it() {
    let fs = FakeFileSystem::new().with_file(format!("{OPERATIONS_DIR}/.keep"), [], false);
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let secrets = FakeSecretStore::new();
    let service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);

    service.save_secret("  configured-secret  ").unwrap();
    assert!(service.has_secret().unwrap());
    assert_eq!(
        secrets.get(PLAYIT_SECRET_KEY).unwrap().as_deref(),
        Some("configured-secret")
    );
    service.remove_secret().unwrap();
    assert!(!service.has_secret().unwrap());
}
