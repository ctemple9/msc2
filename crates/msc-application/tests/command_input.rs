use msc_application::commands::{CommandInputError, stdin_payload, validate_api_command};
use msc_application::lifecycle::{
    ConsoleSink, ImportedJavaServer, JavaServerRepository, LifecycleError, LifecycleService,
    ServerId,
};
use msc_infrastructure::fs::FakeFileSystem;
use msc_infrastructure::process::{FakeProcessSupervisor, ProcessSpawnRequest};
use serde_json::Value;
use std::path::PathBuf;

struct FakeRepository {
    server: ImportedJavaServer,
}

impl JavaServerRepository for FakeRepository {
    fn load(&self, id: &ServerId) -> Result<Option<ImportedJavaServer>, LifecycleError> {
        Ok((&self.server.id == id).then(|| self.server.clone()))
    }
}

struct NullConsole;

impl ConsoleSink for NullConsole {
    fn append_system_line(&self, _server_id: &ServerId, _line: &str) {}
}

fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/command-input")
        .join(format!("{name}.json"));
    let raw = std::fs::read_to_string(path).expect("fixture should be readable");
    serde_json::from_str(&raw).expect("fixture should be valid JSON")
}

fn paper_server() -> ImportedJavaServer {
    ImportedJavaServer::paper("paper-1", "Survival", PathBuf::from("/srv/paper"))
}

fn launch_request() -> ProcessSpawnRequest {
    ProcessSpawnRequest::new("/usr/bin/java", "/srv/paper").args(["-jar", "paper.jar", "--nogui"])
}

#[test]
fn command_input_api_validation_rejects_missing_and_empty() {
    let case = fixture("api-validation-rejects-missing-and-empty");
    let requests = case["input"]["requests"].as_array().unwrap();
    let expected = case["expected"]["results"].as_array().unwrap();

    for (request, expected) in requests.iter().zip(expected) {
        let command = request.get("command").and_then(Value::as_str);
        let error = validate_api_command(command).expect_err("fixture should be rejected");
        assert_eq!(error.code(), expected["error"].as_str().unwrap());
    }

    assert_eq!(
        validate_api_command(None),
        Err(CommandInputError::MissingCommand)
    );
}

#[test]
fn command_input_without_newline_appends_newline() {
    let case = fixture("command-without-newline-appends-newline");
    let command = case["input"]["command"].as_str().unwrap();
    let expected = case["expected"]["stdinBytes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|byte| byte.as_u64().unwrap() as u8)
        .collect::<Vec<_>>();

    assert_eq!(stdin_payload(command), expected);
}

#[test]
fn command_input_existing_newline_is_not_doubled() {
    let case = fixture("command-with-existing-newline-is-not-doubled");
    let command = case["input"]["command"].as_str().unwrap();
    let expected = case["expected"]["stdinBytes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|byte| byte.as_u64().unwrap() as u8)
        .collect::<Vec<_>>();

    assert_eq!(stdin_payload(command), expected);
}

#[test]
fn command_input_running_server_writes_to_process_stdin() {
    let case = fixture("command-without-newline-appends-newline");
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = NullConsole;
    let fs = FakeFileSystem::new();
    let mut service = LifecycleService::new(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    let pid = service.start_active_server(launch_request()).unwrap();
    service.mark_ready(&server.id).unwrap();
    service
        .send_command(case["input"]["command"].as_str().unwrap())
        .unwrap();

    assert_eq!(
        process.stdin_writes(pid).unwrap(),
        vec![b"say hi\n".to_vec()]
    );
}

#[test]
fn command_input_stdin_write_failure_surfaces() {
    let case = fixture("stdin-write-failure-surfaces");
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = NullConsole;
    let fs = FakeFileSystem::new();
    let mut service = LifecycleService::new(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    let pid = service.start_active_server(launch_request()).unwrap();
    service.mark_ready(&server.id).unwrap();
    process.fail_next_stdin(case["input"]["stdinFailure"].as_str().unwrap());

    let error = service
        .send_command(case["input"]["command"].as_str().unwrap())
        .expect_err("stdin write should fail");

    assert!(
        error
            .to_string()
            .contains(case["expected"]["errorContains"].as_str().unwrap())
    );
    assert_eq!(process.stdin_writes(pid).unwrap(), Vec::<Vec<u8>>::new());
}

#[test]
fn command_input_stopped_server_refuses_command() {
    let case = fixture("stopped-server-refuses-command");
    let server = paper_server();
    let repository = FakeRepository {
        server: server.clone(),
    };
    let process = FakeProcessSupervisor::new();
    let console = NullConsole;
    let fs = FakeFileSystem::new();
    let mut service = LifecycleService::new(&repository, &process, &console, &fs);

    service.select_active_server(server.id.clone()).unwrap();
    let error = service
        .send_command(case["input"]["command"].as_str().unwrap())
        .expect_err("stopped server should reject commands");

    assert_eq!(error, LifecycleError::ServerNotRunning);
    assert!(process.spawned_requests().is_empty());
}
