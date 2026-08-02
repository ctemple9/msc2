use msc_infrastructure::process::{
    FakeProcessSupervisor, OutputLineFramer, OutputStream, ProcessEvent, ProcessExitStatus,
    ProcessId, ProcessSpawnRequest, ProcessSupervisor,
};

fn spawn_request() -> ProcessSpawnRequest {
    ProcessSpawnRequest::new("/usr/bin/java", "/srv/paper")
        .args(["-Xmx2048M", "-jar", "paper.jar", "--nogui"])
        .env("MSC_TEST", "1")
}

#[test]
fn process_supervisor_fake_spawn_records_command_and_returns_pid() {
    let supervisor = FakeProcessSupervisor::new();
    let request = spawn_request();

    let pid = supervisor
        .spawn(request.clone())
        .expect("fake process should spawn");

    assert_eq!(pid, ProcessId::new(1000));
    assert_eq!(supervisor.spawned_requests(), vec![(pid, request)]);
}

#[test]
fn process_supervisor_fake_streams_bytes_and_holds_partial_lines_until_newline_or_exit() {
    let supervisor = FakeProcessSupervisor::new();
    let pid = supervisor.spawn(spawn_request()).unwrap();
    let mut framer = OutputLineFramer::new();

    supervisor.emit_stdout(pid, b"Done ").unwrap();
    let first = supervisor.drain_events(pid).unwrap();
    assert!(
        first
            .iter()
            .flat_map(|event| framer.push_event(event))
            .next()
            .is_none()
    );

    supervisor
        .emit_stdout(pid, b"(1.234s)\nTrailing partial")
        .unwrap();
    let lines = supervisor
        .drain_events(pid)
        .unwrap()
        .iter()
        .flat_map(|event| framer.push_event(event))
        .collect::<Vec<_>>();
    assert_eq!(lines, vec!["Done (1.234s)"]);

    supervisor.exit_normally(pid).unwrap();
    let lines = supervisor
        .drain_events(pid)
        .unwrap()
        .iter()
        .flat_map(|event| framer.push_event(event))
        .collect::<Vec<_>>();
    assert_eq!(lines, vec!["Trailing partial"]);
}

#[test]
fn process_supervisor_fake_preserves_stdout_and_stderr_stream_identity() {
    let supervisor = FakeProcessSupervisor::new();
    let pid = supervisor.spawn(spawn_request()).unwrap();

    supervisor.emit_stdout(pid, b"out\n").unwrap();
    supervisor.emit_stderr(pid, b"err\n").unwrap();

    let events = supervisor.drain_events(pid).unwrap();
    assert_eq!(
        events,
        vec![
            ProcessEvent::Output {
                stream: OutputStream::Stdout,
                bytes: b"out\n".to_vec(),
            },
            ProcessEvent::Output {
                stream: OutputStream::Stderr,
                bytes: b"err\n".to_vec(),
            },
        ]
    );
}

#[test]
fn process_supervisor_fake_accepts_commands_and_graceful_stop_writes_stop_newline() {
    let supervisor = FakeProcessSupervisor::new();
    let pid = supervisor.spawn(spawn_request()).unwrap();

    supervisor.write_stdin(pid, b"say hello\n").unwrap();
    supervisor.request_graceful_stop(pid).unwrap();

    assert_eq!(
        supervisor.stdin_writes(pid).unwrap(),
        vec![b"say hello\n".to_vec(), b"stop\n".to_vec()]
    );
    assert_eq!(supervisor.graceful_stops(), vec![pid]);
}

#[test]
fn process_supervisor_fake_can_fail_spawn_and_stdin_for_lifecycle_tests() {
    let supervisor = FakeProcessSupervisor::new();
    supervisor.fail_next_spawn("start failed");

    assert_eq!(
        supervisor.spawn(spawn_request()),
        Err(msc_infrastructure::process::ProcessError::Spawn(
            "start failed".to_string()
        ))
    );

    let pid = supervisor.spawn(spawn_request()).unwrap();
    supervisor.fail_next_stdin("stdin failed");

    assert_eq!(
        supervisor.write_stdin(pid, b"say hello\n"),
        Err(msc_infrastructure::process::ProcessError::Stdin(
            "stdin failed".to_string()
        ))
    );
}

#[test]
fn process_supervisor_fake_simulates_normal_crash_and_forced_exits() {
    let supervisor = FakeProcessSupervisor::new();
    let normal = supervisor.spawn(spawn_request()).unwrap();
    let crash = supervisor.spawn(spawn_request()).unwrap();
    let forced = supervisor.spawn(spawn_request()).unwrap();

    supervisor.exit_normally(normal).unwrap();
    supervisor.crash(crash, 1).unwrap();
    supervisor.force_terminate(forced).unwrap();

    assert_eq!(
        supervisor.drain_events(normal).unwrap(),
        vec![ProcessEvent::Exited(ProcessExitStatus::exited(0))]
    );
    assert_eq!(
        supervisor.drain_events(crash).unwrap(),
        vec![ProcessEvent::Exited(ProcessExitStatus::exited(1))]
    );
    assert_eq!(
        supervisor.drain_events(forced).unwrap(),
        vec![ProcessEvent::Exited(ProcessExitStatus::signaled(15))]
    );
    assert_eq!(supervisor.force_terminations(), vec![forced]);
}
