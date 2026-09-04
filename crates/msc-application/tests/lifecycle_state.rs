use msc_application::diagnostics;
use msc_application::lifecycle::{
    ConsoleSink, ImportedJavaServer, JavaServerRepository, LifecycleError, LifecycleService,
    LifecycleState, ServerId,
};
use msc_domain::crash_analysis::StartupProblemKind;
use msc_domain::identity::JavaServerFlavor;
use msc_infrastructure::fs::FakeFileSystem;
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
    fs: &'deps FakeFileSystem,
) -> LifecycleService<'deps> {
    LifecycleService::new(repository, process, console, fs)
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
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

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
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

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
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

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
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service
        .mark_ready(&server.id, "2026-08-20T00:00:00Z")
        .unwrap();

    assert_eq!(service.state(), LifecycleState::Running);
}

#[test]
fn lifecycle_state_ready_start_replaces_previous_failed_start_record() {
    let server = paper_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    diagnostics::write_last_startup_result(
        &fs,
        &server.directory,
        "2026-08-19T00:00:00Z",
        false,
        vec!["Server stopped before reaching ready state.".to_string()],
        Vec::new(),
        Vec::new(),
    );
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service
        .mark_ready(&server.id, "2026-08-20T00:00:00Z")
        .unwrap();

    let record = diagnostics::read_last_startup_result(&fs, &server.directory)
        .expect("a ready server should persist its successful start");
    assert_eq!(record.started_at, "2026-08-20T00:00:00Z");
    assert!(record.was_clean);
    assert!(record.fatal_errors.is_empty());
    assert!(record.warnings.is_empty());
    assert!(record.problems.is_none());
}

#[test]
fn lifecycle_state_paper_soft_failure_keeps_source_accurate_400_line_window() {
    let server = paper_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new().with_file(
        "/srv/paper/plugins/BrokenPlugin-1.0.jar",
        b"jar".to_vec(),
        false,
    );
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service
        .ingest_console_line(
            "Error occurred while enabling BrokenPlugin v1.0 (Is it up to date?)",
            "2026-08-20T00:00:00Z",
        )
        .unwrap();
    for index in 0..200 {
        service
            .ingest_console_line(
                &format!("ordinary startup line {index}"),
                "2026-08-20T00:00:00Z",
            )
            .unwrap();
    }
    service
        .ingest_console_line(
            "Done (1.234s)! For help, type \"help\"",
            "2026-08-20T00:00:00Z",
        )
        .unwrap();

    let record = diagnostics::read_last_startup_result(&fs, &server.directory)
        .expect("the early plugin failure must remain in Paper's 400-line window");
    let problems = record.problems.expect("soft failure should be structured");
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].offender_name, "BrokenPlugin");
    assert_eq!(
        problems[0].installed_jar_stem.as_deref(),
        Some("BrokenPlugin-1.0")
    );
}

#[test]
fn lifecycle_state_failed_stop_keeps_server_running() {
    let server = paper_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service
        .mark_ready(&server.id, "2026-08-20T00:00:00Z")
        .unwrap();

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
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service
        .mark_ready(&server.id, "2026-08-20T00:00:00Z")
        .unwrap();
    service
        .mark_process_exited(&server.id, "2024-01-01T00:00:00Z")
        .unwrap();

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
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service
        .mark_ready(&server.id, "2026-08-20T00:00:00Z")
        .unwrap();
    service.request_stop().unwrap();

    assert_eq!(service.state(), LifecycleState::Stopping);
    assert_eq!(
        process.graceful_stops().as_slice(),
        &[msc_infrastructure::process::ProcessId::new(1000)]
    );

    service
        .mark_process_exited(&server.id, "2024-01-01T00:00:00Z")
        .unwrap();
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
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);
    let other = ServerId::new("other-server");

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();

    assert_eq!(
        service.mark_ready(&other, "2026-08-20T00:00:00Z"),
        Err(LifecycleError::WrongActiveServer {
            expected: server.id,
            actual: other,
        })
    );
}

// --- P7.32: real diagnose_unexpected_stop / write_last_startup_result wiring ---

fn fabric_server() -> ImportedJavaServer {
    ImportedJavaServer {
        id: ServerId::new("fabric-1"),
        name: "Modded".to_string(),
        directory: PathBuf::from("/srv/fabric"),
        flavor: JavaServerFlavor::Fabric,
    }
}

#[test]
fn lifecycle_state_unrequested_exit_before_ready_records_generic_startup_failure() {
    let server = paper_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service
        .mark_process_exited(&server.id, "2024-01-01T00:00:00Z")
        .unwrap();

    assert_eq!(service.state(), LifecycleState::Crashed);
    let record = diagnostics::read_last_startup_result(&fs, &server.directory)
        .expect("an unrequested exit before ready should write a record");
    assert!(!record.was_clean);
    assert_eq!(
        record.fatal_errors,
        vec!["Server stopped before reaching ready state.".to_string()]
    );
    assert_eq!(record.problems.as_ref().map(Vec::len), Some(1));
    assert_eq!(
        record.problems.as_ref().map(|problems| problems[0].kind),
        Some(StartupProblemKind::Unknown)
    );
}

#[test]
fn lifecycle_state_unrequested_exit_after_ready_preserves_clean_start_record() {
    let server = paper_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service
        .ingest_console_line(
            "Done (1.234s)! For help, type \"help\"",
            "2026-08-20T00:00:00Z",
        )
        .unwrap();
    service
        .mark_process_exited(&server.id, "2024-01-01T00:00:00Z")
        .unwrap();

    assert_eq!(service.state(), LifecycleState::Crashed);
    let record = diagnostics::read_last_startup_result(&fs, &server.directory)
        .expect("a crash after a clean boot should preserve the successful start record");
    assert!(record.was_clean);
    assert!(record.fatal_errors.is_empty());
    assert!(record.warnings.is_empty());
    assert!(record.problems.is_none());
}

#[test]
fn lifecycle_state_requested_stop_before_ready_records_generic_failure_without_crash_analysis() {
    let server = fabric_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service
        .ingest_console_line(
            "A mod requires a dependency that is missing:",
            "2026-08-20T00:00:00Z",
        )
        .unwrap();
    service
        .ingest_console_line(
            "\t - Mod 'MyMod' (mymod) 1.0 requires any version of fabric api, which is missing!",
            "2026-08-20T00:00:00Z",
        )
        .unwrap();
    service.request_stop().unwrap();
    service
        .mark_process_exited(&server.id, "2024-01-01T00:00:00Z")
        .unwrap();

    assert_eq!(service.state(), LifecycleState::Stopped);
    let record = diagnostics::read_last_startup_result(&fs, &server.directory)
        .expect("a user-requested stop before ready should still record the generic failure");
    assert!(!record.was_clean);
    assert_eq!(
        record.fatal_errors,
        vec!["Server stopped before reaching ready state.".to_string()]
    );
    assert_eq!(record.problems.as_ref().map(Vec::len), Some(1));
    assert_eq!(
        record.problems.as_ref().map(|problems| problems[0].kind),
        Some(StartupProblemKind::Unknown),
        "a user-requested stop still gets a generic finding without mod analysis"
    );
}

#[test]
fn lifecycle_state_requested_stop_after_ready_preserves_clean_start_record() {
    let server = paper_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service
        .ingest_console_line(
            "Done (1.234s)! For help, type \"help\"",
            "2026-08-20T00:00:00Z",
        )
        .unwrap();
    service.request_stop().unwrap();
    service
        .mark_process_exited(&server.id, "2024-01-01T00:00:00Z")
        .unwrap();

    assert_eq!(service.state(), LifecycleState::Stopped);
    let record = diagnostics::read_last_startup_result(&fs, &server.directory)
        .expect("a clean requested stop should preserve the successful start record");
    assert!(record.was_clean);
    assert!(record.fatal_errors.is_empty());
    assert!(record.warnings.is_empty());
    assert!(record.problems.is_none());
}

#[test]
fn lifecycle_state_unrequested_exit_before_ready_on_modded_server_attributes_crash_to_the_mod() {
    let server = fabric_server();
    let repository = FakeRepository {
        server: Some(server.clone()),
    };
    let process = FakeProcessSupervisor::default();
    let console = FakeConsole::default();
    let fs = FakeFileSystem::new();
    let mut service = service(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();
    service
        .ingest_console_line(
            "A mod requires a dependency that is missing:",
            "2026-08-20T00:00:00Z",
        )
        .unwrap();
    service
        .ingest_console_line(
            "\t - Mod 'MyMod' (mymod) 1.0 requires any version of fabric api, which is missing!",
            "2026-08-20T00:00:00Z",
        )
        .unwrap();
    service
        .mark_process_exited(&server.id, "2024-01-01T00:00:00Z")
        .unwrap();

    assert_eq!(service.state(), LifecycleState::Crashed);
    let record = diagnostics::read_last_startup_result(&fs, &server.directory)
        .expect("an unrequested exit before ready on a modded server should analyze the crash");
    assert!(!record.was_clean);
    let problems = record
        .problems
        .expect("crash analysis should attribute a problem");
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].offender_name, "MyMod");
    assert_eq!(
        record.fatal_errors,
        vec!["MyMod: Requires any version of fabric api".to_string()]
    );
}
