use msc_infrastructure::helper_process::{
    HELPER_DIAGNOSTIC_LIMIT, HelperKey, HelperProcessError, HelperProcessManager,
    ManagedHelperStatus,
};
use msc_infrastructure::process::{FakeProcessSupervisor, OutputStream, ProcessSpawnRequest};

fn request() -> ProcessSpawnRequest {
    ProcessSpawnRequest::new("/usr/local/bin/helper", "/srv/example")
}

fn key() -> HelperKey {
    HelperKey::new("server-a", "playit")
}

#[test]
fn helper_process_prevents_duplicate_server_function_ownership() {
    let supervisor = FakeProcessSupervisor::new();
    let mut manager = HelperProcessManager::new(&supervisor);
    let first = manager.start(key(), request()).unwrap();

    assert_eq!(
        manager.start(key(), request()),
        Err(HelperProcessError::AlreadyManaged(key()))
    );
    assert_eq!(supervisor.spawned_requests().len(), 1);
    assert_eq!(
        manager.snapshot(&key()).unwrap().status,
        ManagedHelperStatus::Starting
    );
    assert_eq!(first.raw(), 1000);
}

#[test]
fn helper_process_preserves_per_stream_framing_and_bounds_diagnostics() {
    let supervisor = FakeProcessSupervisor::new();
    let mut manager = HelperProcessManager::new(&supervisor);
    let process = manager.start(key(), request()).unwrap();

    supervisor.emit_stdout(process, b"stdout ").unwrap();
    supervisor.emit_stderr(process, b"stderr\n").unwrap();
    manager.poll().unwrap();
    supervisor.emit_stdout(process, b"complete\n").unwrap();
    for index in 0..=HELPER_DIAGNOSTIC_LIMIT {
        supervisor
            .emit_stderr(process, format!("line-{index}\n"))
            .unwrap();
    }
    manager.poll().unwrap();

    let diagnostics = manager.snapshot(&key()).unwrap().diagnostics;
    assert_eq!(diagnostics.len(), HELPER_DIAGNOSTIC_LIMIT);
    assert_eq!(diagnostics[0].line, "line-1");
    assert_eq!(
        diagnostics.last().unwrap().line,
        format!("line-{HELPER_DIAGNOSTIC_LIMIT}")
    );
    assert!(
        diagnostics
            .iter()
            .all(|entry| entry.stream == OutputStream::Stderr)
    );
}

#[test]
fn helper_process_records_readiness_and_nonzero_exit() {
    let supervisor = FakeProcessSupervisor::new();
    let mut manager = HelperProcessManager::new(&supervisor);
    let process = manager.start(key(), request()).unwrap();

    manager.record_ready(&key()).unwrap();
    assert_eq!(
        manager.snapshot(&key()).unwrap().status,
        ManagedHelperStatus::Running
    );

    supervisor.crash(process, 23).unwrap();
    manager.poll().unwrap();
    assert_eq!(
        manager.snapshot(&key()).unwrap().status,
        ManagedHelperStatus::Failed {
            exit: Some(msc_infrastructure::process::ProcessExitStatus::exited(23))
        }
    );
}

#[test]
fn helper_process_requests_graceful_stop_before_forced_termination() {
    let supervisor = FakeProcessSupervisor::new();
    let mut manager = HelperProcessManager::new(&supervisor);
    let process = manager.start(key(), request()).unwrap();

    manager.request_graceful_stop(&key()).unwrap();
    assert_eq!(supervisor.graceful_stops(), vec![process]);
    assert_eq!(
        manager.snapshot(&key()).unwrap().status,
        ManagedHelperStatus::Stopping
    );

    manager.force_terminate(&key()).unwrap();
    assert_eq!(supervisor.force_terminations(), vec![process]);
    manager.poll().unwrap();
    assert!(matches!(
        manager.snapshot(&key()).unwrap().status,
        ManagedHelperStatus::Failed { .. }
    ));
}

#[test]
fn helper_process_never_claims_a_pre_restart_pid_is_running() {
    let supervisor = FakeProcessSupervisor::new();
    let mut manager = HelperProcessManager::recover_after_restart(&supervisor, [key()]);

    assert_eq!(
        manager.snapshot(&key()).unwrap().status,
        ManagedHelperStatus::UnknownUntilReconciled
    );
    assert_eq!(
        manager.start(key(), request()),
        Err(HelperProcessError::AlreadyManaged(key()))
    );
    manager.reconcile_as_stopped(&key()).unwrap();
    assert_eq!(
        manager.snapshot(&key()).unwrap().status,
        ManagedHelperStatus::Stopped
    );
}
