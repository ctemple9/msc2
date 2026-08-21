use msc_application::lifecycle::{
    ConsoleSink, ImportedJavaServer, JavaServerRepository, LifecycleError, LifecycleService,
    LifecycleState, ServerId,
};
use msc_infrastructure::fs::FakeFileSystem;
use msc_infrastructure::process::{
    FakeProcessSupervisor, OutputLineFramer, ProcessSpawnRequest, ProcessSupervisor,
};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

struct FakeRepository {
    server: ImportedJavaServer,
}

impl JavaServerRepository for FakeRepository {
    fn load(&self, id: &ServerId) -> Result<Option<ImportedJavaServer>, LifecycleError> {
        Ok((&self.server.id == id).then(|| self.server.clone()))
    }
}

#[derive(Default)]
struct FakeConsole {
    lines: Mutex<Vec<String>>,
}

impl ConsoleSink for FakeConsole {
    fn append_system_line(&self, _server_id: &ServerId, line: &str) {
        self.lines.lock().unwrap().push(line.to_string());
    }
}

struct RunningService<'deps> {
    service: LifecycleService<'deps>,
    pid: msc_infrastructure::process::ProcessId,
}

fn fixture(case: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/java-stop-restart")
        .join(format!("{case}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn paper_server() -> ImportedJavaServer {
    ImportedJavaServer::paper("paper-1", "Survival", PathBuf::from("/srv/paper"))
}

fn launch_request() -> ProcessSpawnRequest {
    ProcessSpawnRequest::new("/usr/bin/java", "/srv/paper").args(["-jar", "paper.jar", "--nogui"])
}

fn running_service<'deps>(
    repository: &'deps FakeRepository,
    process: &'deps FakeProcessSupervisor,
    console: &'deps FakeConsole,
    fs: &'deps FakeFileSystem,
) -> RunningService<'deps> {
    let mut service = LifecycleService::new(repository, process, console, fs);
    service
        .select_active_server(repository.server.id.clone())
        .unwrap();
    let pid = service.start_active_server(launch_request()).unwrap();
    service
        .mark_ready(&repository.server.id, "2026-08-20T00:00:00Z")
        .unwrap();
    RunningService { service, pid }
}

fn assert_state(state: LifecycleState, expected: &Value) {
    assert_eq!(
        state.raw_value(),
        expected.as_str().expect("expected state")
    );
}

fn expected_stdin_writes(fixture: &Value) -> Vec<Vec<u8>> {
    fixture["expected"]["stdinWrites"]
        .as_array()
        .expect("expected stdinWrites")
        .iter()
        .map(|value| value.as_str().expect("stdin write").as_bytes().to_vec())
        .collect()
}

#[test]
fn java_stop_restart_graceful_stop_sends_stop_and_enters_stopping() {
    let case = fixture("graceful-stop-sends-stop-and-enters-stopping");
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let RunningService { mut service, pid } = running_service(&repository, &process, &console, &fs);

    service.request_stop().unwrap();

    assert_state(service.state(), &case["expected"]["stateAfterRequest"]);
    assert_eq!(
        process.stdin_writes(pid).unwrap(),
        expected_stdin_writes(&case)
    );
    assert_eq!(
        process.spawned_requests().len() as u64,
        case["expected"]["spawnCount"].as_u64().unwrap()
    );
}

#[test]
fn java_stop_restart_graceful_stop_waits_for_process_exit() {
    let case = fixture("graceful-stop-waits-for-process-exit");
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let RunningService { mut service, pid } = running_service(&repository, &process, &console, &fs);

    service.request_stop().unwrap();
    assert_state(service.state(), &case["expected"]["stateBeforeExit"]);

    process.exit_normally(pid).unwrap();
    for event in process.drain_events(pid).unwrap() {
        service
            .handle_process_event(pid, &event, "2024-01-01T00:00:00Z")
            .unwrap();
    }

    assert_state(service.state(), &case["expected"]["stateAfterExit"]);
    assert_eq!(service.active_process(), None);
}

#[test]
fn java_stop_restart_stop_write_failure_keeps_running() {
    let case = fixture("stop-write-failure-keeps-running");
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let RunningService { mut service, pid } = running_service(&repository, &process, &console, &fs);
    process.fail_next_stdin(case["input"]["stdinFailure"].as_str().unwrap());

    let error = service
        .request_stop()
        .expect_err("failed stdin write should reject stop");

    assert!(
        error
            .to_string()
            .contains(case["expected"]["errorContains"].as_str().unwrap())
    );
    assert_state(service.state(), &case["expected"]["stateAfterRequest"]);
    assert_eq!(
        process.stdin_writes(pid).unwrap(),
        expected_stdin_writes(&case)
    );
}

#[test]
fn java_stop_restart_restart_sends_stop_and_defers_launch_until_exit() {
    let case = fixture("restart-sends-stop-and-defers-launch-until-exit");
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let RunningService { mut service, pid } = running_service(&repository, &process, &console, &fs);

    service.restart_active_server(launch_request()).unwrap();

    assert_state(service.state(), &case["expected"]["stateAfterRequest"]);
    assert_eq!(
        process.stdin_writes(pid).unwrap(),
        expected_stdin_writes(&case)
    );
    assert_eq!(
        process.spawned_requests().len() as u64,
        case["expected"]["spawnCountBeforeExit"].as_u64().unwrap()
    );

    let duplicate = service
        .restart_active_server(launch_request())
        .expect_err("second restart should not spawn while graceful stop is pending");
    assert!(matches!(duplicate, LifecycleError::IllegalTransition(_)));
    assert!(
        case["expected"]["duplicateRestartRejected"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(
        process.spawned_requests().len() as u64,
        case["expected"]["spawnCountBeforeExit"].as_u64().unwrap()
    );
}

#[test]
fn java_stop_restart_restart_starts_new_process_after_exit() {
    let case = fixture("restart-starts-new-process-after-exit");
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let RunningService { mut service, pid } = running_service(&repository, &process, &console, &fs);

    service.restart_active_server(launch_request()).unwrap();
    process.exit_normally(pid).unwrap();
    for event in process.drain_events(pid).unwrap() {
        service
            .handle_process_event(pid, &event, "2024-01-01T00:00:00Z")
            .unwrap();
    }

    assert_state(service.state(), &case["expected"]["stateAfterExit"]);
    assert_eq!(
        process.spawned_requests().len() as u64,
        case["expected"]["spawnCountAfterExit"].as_u64().unwrap()
    );
    assert_eq!(
        pid.raw() as u64,
        case["expected"]["oldPid"].as_u64().unwrap()
    );
    assert_eq!(
        service.active_process().map(|pid| pid.raw() as u64),
        Some(case["expected"]["newPid"].as_u64().unwrap())
    );
}

#[test]
fn java_stop_restart_process_exit_flushes_trailing_console_line() {
    let case = fixture("process-exit-flushes-trailing-console-line");
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let RunningService { mut service, pid } = running_service(&repository, &process, &console, &fs);
    let mut framer = OutputLineFramer::new();
    service.request_stop().unwrap();

    for chunk in case["input"]["stdoutChunks"].as_array().unwrap() {
        process
            .emit_stdout(pid, chunk.as_str().unwrap().as_bytes())
            .unwrap();
    }
    process.exit_normally(pid).unwrap();

    let mut lines = Vec::new();
    for event in process.drain_events(pid).unwrap() {
        lines.extend(framer.push_event(&event));
        service
            .handle_process_event(pid, &event, "2024-01-01T00:00:00Z")
            .unwrap();
    }

    let expected_lines = case["expected"]["flushedConsoleLines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(lines, expected_lines);
    assert_state(service.state(), &case["expected"]["stateAfterExit"]);
}
