use msc_application::lifecycle::{
    ConsoleSink, ImportedJavaServer, JavaServerRepository, LifecycleError, LifecycleService,
    ServerId,
};
use msc_application::session_log::{SessionEventType, append_event, clear_events, load_events};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::process::{FakeProcessSupervisor, ProcessSpawnRequest};
use serde_json::json;
use std::path::{Path, PathBuf};

#[derive(Clone)]
struct FakeRepository {
    server: ImportedJavaServer,
}

impl JavaServerRepository for FakeRepository {
    fn load(&self, id: &ServerId) -> Result<Option<ImportedJavaServer>, LifecycleError> {
        Ok((&self.server.id == id).then(|| self.server.clone()))
    }
}

#[derive(Default)]
struct NullConsole;

impl ConsoleSink for NullConsole {
    fn append_system_line(&self, _server_id: &ServerId, _line: &str) {}
}

fn paper_server() -> ImportedJavaServer {
    ImportedJavaServer::paper("paper-1", "Survival", PathBuf::from("/srv/paper"))
}

fn launch_request() -> ProcessSpawnRequest {
    ProcessSpawnRequest::new("/usr/bin/java", "/srv/paper").args(["-jar", "paper.jar"])
}

fn session_file(fs: &FakeFileSystem) -> Vec<u8> {
    fs.read(Path::new("/srv/paper/session_log.json"))
        .expect("session log should exist")
}

#[test]
fn session_log_json_shape_matches_swift_codable_output() {
    let fs = FakeFileSystem::new().with_dir("/srv/paper");
    append_event(
        &fs,
        Path::new("/srv/paper"),
        "Alex",
        SessionEventType::Joined,
        "2026-08-26T12:34:56Z".to_string(),
    )
    .unwrap();

    let value: serde_json::Value = serde_json::from_slice(&session_file(&fs)).unwrap();
    let event = &value[0];
    assert_eq!(event["id"].as_str().unwrap().len(), 36);
    assert_eq!(
        event,
        &json!({
            "id": event["id"],
            "playerName": "Alex",
            "eventType": "joined",
            "timestamp": "2026-08-26T12:34:56Z"
        })
    );
}

#[test]
fn session_log_load_append_and_clear_match_manager_behavior() {
    let fs = FakeFileSystem::new().with_dir("/srv/paper");
    let server_dir = Path::new("/srv/paper");

    assert!(load_events(&fs, server_dir).is_empty());
    let events = append_event(
        &fs,
        server_dir,
        "Alex",
        SessionEventType::Joined,
        "2026-08-26T12:34:56Z".to_string(),
    )
    .unwrap();
    assert_eq!(events.len(), 1);
    let events = append_event(
        &fs,
        server_dir,
        "Alex",
        SessionEventType::Left,
        "2026-08-26T13:34:56Z".to_string(),
    )
    .unwrap();
    let loaded = load_events(&fs, server_dir);
    assert_eq!(loaded.len(), events.len());
    assert_eq!(loaded[0].player_name, events[0].player_name);
    assert_eq!(loaded[0].event_type, events[0].event_type);
    assert_eq!(loaded[0].timestamp, events[0].timestamp);
    assert_eq!(loaded[1].player_name, events[1].player_name);
    assert_eq!(loaded[1].event_type, events[1].event_type);
    assert_eq!(loaded[1].timestamp, events[1].timestamp);

    clear_events(&fs, server_dir).unwrap();
    assert!(load_events(&fs, server_dir).is_empty());
    clear_events(&fs, server_dir).unwrap();
}

#[test]
fn malformed_session_log_loads_as_empty() {
    let fs =
        FakeFileSystem::new().with_file("/srv/paper/session_log.json", b"not json".to_vec(), false);
    assert!(load_events(&fs, Path::new("/srv/paper")).is_empty());
}

#[test]
fn java_console_player_events_are_persisted_without_blocking_lifecycle() {
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = NullConsole;
    let fs = FakeFileSystem::new().with_dir("/srv/paper");
    let mut service = LifecycleService::new(&repository, &process, &console, &fs);
    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();

    service
        .ingest_console_line(
            "[Server thread/INFO]: Alex joined the game",
            "2026-08-26T12:34:56Z",
        )
        .unwrap();
    service
        .ingest_console_line(
            "[Server thread/INFO]: Alex left the game",
            "2026-08-26T13:34:56Z",
        )
        .unwrap();

    let events = load_events(&fs, Path::new("/srv/paper"));
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].player_name, "Alex");
    assert_eq!(events[0].event_type, SessionEventType::Joined);
    assert_eq!(events[1].event_type, SessionEventType::Left);
}

#[test]
fn session_log_write_failure_does_not_fail_console_ingestion() {
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = NullConsole;
    let fs = FakeFileSystem::new();
    let mut service = LifecycleService::new(&repository, &process, &console, &fs);
    service.select_active_server(server.id.clone()).unwrap();
    service.start_active_server(launch_request()).unwrap();

    let events = service
        .ingest_console_line(
            "[Server thread/INFO]: Alex joined the game",
            "2026-08-26T12:34:56Z",
        )
        .unwrap();
    assert_eq!(events.len(), 1);
}
