use msc_application::lifecycle::{
    ConsoleSink, ImportedJavaServer, JavaServerRepository, LifecycleError, LifecycleService,
    LifecycleState, ServerId,
};
use msc_infrastructure::fs::FakeFileSystem;
use msc_infrastructure::process::{
    FakeProcessSupervisor, OutputLineFramer, ProcessEvent, ProcessSpawnRequest, ProcessSupervisor,
};
use std::path::PathBuf;
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

fn paper_server() -> ImportedJavaServer {
    ImportedJavaServer::paper("paper-1", "Survival", PathBuf::from("/srv/paper"))
}

fn launch_request() -> ProcessSpawnRequest {
    ProcessSpawnRequest::new("/usr/bin/java", "/srv/paper").args(["-jar", "paper.jar", "--nogui"])
}

fn service<'deps>(
    repository: &'deps FakeRepository,
    process: &'deps FakeProcessSupervisor,
    console: &'deps FakeConsole,
    fs: &'deps FakeFileSystem,
) -> LifecycleService<'deps> {
    LifecycleService::new(repository, process, console, fs)
}

#[test]
fn lifecycle_with_fake_process_start_spawns_and_tracks_pid() {
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    let pid = service.start_active_server(launch_request()).unwrap();

    assert_eq!(service.state(), LifecycleState::Starting);
    assert_eq!(service.active_process(), Some(pid));
    assert_eq!(
        process.spawned_requests()[0].1.arguments,
        vec!["-jar", "paper.jar", "--nogui"]
    );
}

#[test]
fn lifecycle_with_fake_process_partial_ready_line_can_drive_running_state() {
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);
    let mut framer = OutputLineFramer::new();

    service.select_active_server(server.id.clone()).unwrap();
    let pid = service.start_active_server(launch_request()).unwrap();

    process.emit_stdout(pid, b"Preparing spawn area").unwrap();
    process
        .emit_stdout(pid, b"\nDone (1.234s)! For help")
        .unwrap();
    process.emit_stdout(pid, b", type \"help\"\n").unwrap();

    for event in process.drain_events(pid).unwrap() {
        service
            .handle_process_event(pid, &event, "2024-01-01T00:00:00Z")
            .unwrap();
        for line in framer.push_event(&event) {
            if line.starts_with("Done (") {
                service
                    .mark_ready(&server.id, "2026-08-20T00:00:00Z")
                    .unwrap();
            }
        }
    }

    assert_eq!(service.state(), LifecycleState::Running);
}

#[test]
fn lifecycle_with_fake_process_graceful_stop_waits_for_exit_before_stopped() {
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    let pid = service.start_active_server(launch_request()).unwrap();
    service
        .mark_ready(&server.id, "2026-08-20T00:00:00Z")
        .unwrap();

    service.request_stop().unwrap();
    assert_eq!(service.state(), LifecycleState::Stopping);
    assert_eq!(process.stdin_writes(pid).unwrap(), vec![b"stop\n".to_vec()]);

    process.exit_normally(pid).unwrap();
    for event in process.drain_events(pid).unwrap() {
        service
            .handle_process_event(pid, &event, "2024-01-01T00:00:00Z")
            .unwrap();
    }

    assert_eq!(service.state(), LifecycleState::Stopped);
    assert_eq!(service.active_process(), None);
}

#[test]
fn lifecycle_with_fake_process_crash_exit_marks_running_server_crashed() {
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    let pid = service.start_active_server(launch_request()).unwrap();
    service
        .mark_ready(&server.id, "2026-08-20T00:00:00Z")
        .unwrap();

    process.crash(pid, 1).unwrap();
    for event in process.drain_events(pid).unwrap() {
        assert!(matches!(event, ProcessEvent::Exited(_)));
        service
            .handle_process_event(pid, &event, "2024-01-01T00:00:00Z")
            .unwrap();
    }

    assert_eq!(service.state(), LifecycleState::Crashed);
    assert_eq!(service.active_process(), None);
}
