use msc_application::lifecycle::{
    ConsoleSink, ImportedJavaServer, JavaServerRepository, LifecycleError, LifecycleService,
    LifecycleState, ServerId,
};
use msc_infrastructure::process::{FakeProcessSupervisor, ProcessSpawnRequest};
use std::path::PathBuf;
use std::sync::Mutex;

const ALL_STATES: [LifecycleState; 5] = [
    LifecycleState::Stopped,
    LifecycleState::Starting,
    LifecycleState::Running,
    LifecycleState::Stopping,
    LifecycleState::Crashed,
];

const LEGAL_TRANSITIONS: [(LifecycleState, LifecycleState); 8] = [
    (LifecycleState::Stopped, LifecycleState::Starting),
    (LifecycleState::Crashed, LifecycleState::Starting),
    (LifecycleState::Starting, LifecycleState::Running),
    (LifecycleState::Starting, LifecycleState::Stopping),
    (LifecycleState::Running, LifecycleState::Stopping),
    (LifecycleState::Starting, LifecycleState::Crashed),
    (LifecycleState::Running, LifecycleState::Crashed),
    (LifecycleState::Stopping, LifecycleState::Stopped),
];

#[derive(Default)]
struct FakeRepository {
    server: Option<ImportedJavaServer>,
}

impl JavaServerRepository for FakeRepository {
    fn load(&self, id: &ServerId) -> Result<Option<ImportedJavaServer>, LifecycleError> {
        Ok(self
            .server
            .as_ref()
            .filter(|server| &server.id == id)
            .cloned())
    }
}

#[derive(Default)]
struct FakeConsole {
    lines: Mutex<Vec<(ServerId, String)>>,
}

impl ConsoleSink for FakeConsole {
    fn append_system_line(&self, server_id: &ServerId, line: &str) {
        self.lines
            .lock()
            .unwrap()
            .push((server_id.clone(), line.to_string()));
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
) -> LifecycleService<'deps> {
    LifecycleService::new(repository, process, console)
}

#[test]
fn lifecycle_state_raw_values_round_trip() {
    for state in ALL_STATES {
        assert_eq!(
            LifecycleState::from_raw_value(state.raw_value()),
            Some(state)
        );
    }
    assert_eq!(LifecycleState::from_raw_value("launching"), None);
}

#[test]
fn lifecycle_state_transition_table_is_closed() {
    for from in ALL_STATES {
        for to in ALL_STATES {
            let expected_legal = LEGAL_TRANSITIONS.contains(&(from, to));
            assert_eq!(
                from.transition_to(to).is_ok(),
                expected_legal,
                "{from:?} -> {to:?}"
            );
        }
    }
}

#[test]
fn lifecycle_state_service_requires_an_active_server_before_starting() {
    let repository = FakeRepository::default();
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let mut service = service(&repository, &process, &console);

    assert_eq!(
        service.start_active_server(launch_request()),
        Err(LifecycleError::NoActiveServer)
    );
    assert_eq!(service.state(), LifecycleState::Stopped);
}

#[test]
fn lifecycle_state_start_uses_injected_repository_process_and_console() {
    let server = paper_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let mut service = service(&repository, &process, &console);

    service
        .select_active_server(server.id.clone())
        .expect("active server can be selected");
    service
        .start_active_server(launch_request())
        .expect("start is delegated to the process supervisor");

    assert_eq!(service.state(), LifecycleState::Starting);
    assert_eq!(
        process
            .spawned_requests()
            .first()
            .map(|(_, request)| request.working_directory.clone()),
        Some(server.directory.clone())
    );
    assert_eq!(
        console.lines.lock().unwrap().as_slice(),
        &[(server.id.clone(), "Starting server: Survival".to_string())]
    );
}

#[test]
fn lifecycle_state_failed_process_start_keeps_server_stopped() {
    let server = paper_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    process.fail_next_spawn("start failed");
    let console = FakeConsole::default();
    let mut service = service(&repository, &process, &console);

    service.select_active_server(server.id).unwrap();

    assert_eq!(
        service.start_active_server(launch_request()),
        Err(LifecycleError::Process("start failed".to_string()))
    );
    assert_eq!(service.state(), LifecycleState::Stopped);
    assert!(console.lines.lock().unwrap().is_empty());
}

#[test]
fn lifecycle_state_ready_line_moves_starting_server_to_running() {
    let server = paper_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let mut service = service(&repository, &process, &console);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service.mark_ready(&server.id).unwrap();

    assert_eq!(service.state(), LifecycleState::Running);
}

#[test]
fn lifecycle_state_failed_stop_keeps_server_running() {
    let server = paper_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let mut service = service(&repository, &process, &console);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service.mark_ready(&server.id).unwrap();

    process.fail_next_stdin("stop failed");

    assert_eq!(
        service.request_stop(),
        Err(LifecycleError::Process("stop failed".to_string()))
    );
    assert_eq!(service.state(), LifecycleState::Running);
}

#[test]
fn lifecycle_state_unexpected_exit_marks_running_server_crashed() {
    let server = paper_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let mut service = service(&repository, &process, &console);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service.mark_ready(&server.id).unwrap();
    service.mark_process_exited(&server.id).unwrap();

    assert_eq!(service.state(), LifecycleState::Crashed);
}

#[test]
fn lifecycle_state_requested_stop_delegates_and_exits_to_stopped() {
    let server = paper_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let mut service = service(&repository, &process, &console);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service.mark_ready(&server.id).unwrap();
    service.request_stop().unwrap();

    assert_eq!(service.state(), LifecycleState::Stopping);
    assert_eq!(
        process.graceful_stops().as_slice(),
        &[msc_infrastructure::process::ProcessId::new(1000)]
    );

    service.mark_process_exited(&server.id).unwrap();
    assert_eq!(service.state(), LifecycleState::Stopped);
}

#[test]
fn lifecycle_state_event_for_non_active_server_is_rejected() {
    let server = paper_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let mut service = service(&repository, &process, &console);
    let other = ServerId::new("other-server");

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();

    assert_eq!(
        service.mark_ready(&other),
        Err(LifecycleError::WrongActiveServer {
            expected: server.id,
            actual: other,
        })
    );
}
