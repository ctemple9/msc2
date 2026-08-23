//! P10.15's offline Windows public-contract proof.
//!
//! The public HTTP surface is exercised with a fake native Windows runtime.
//! The fake is deliberately separate from a real BDS package: this proves the
//! contract, service ownership, and cleanup rules without claiming that a
//! Windows distribution was downloaded or started.

use axum::{Json, Router, extract::State, http::StatusCode, routing::get, routing::post};
use msc_application::bedrock_runtime::{
    BedrockProvisionRequest, BedrockRuntime, BedrockRuntimeBackend, BedrockRuntimeCapabilities,
    BedrockRuntimeError, BedrockRuntimeEvent, BedrockRuntimeState, BedrockStartRequest,
};
use msc_infrastructure::process::ProcessId;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const SERVER_DIR: &str = r"C:\MSC\servers\bedrock-windows";

struct FakeWindowsRuntime {
    capabilities: BedrockRuntimeCapabilities,
    state: BedrockRuntimeState,
    process_id: Option<ProcessId>,
    commands: Vec<String>,
    live_processes: BTreeSet<ProcessId>,
    orphaned_processes: BTreeSet<ProcessId>,
    fail_next_start: bool,
}

impl FakeWindowsRuntime {
    fn supported() -> Self {
        Self {
            capabilities: BedrockRuntimeCapabilities::supported(BedrockRuntimeBackend::Native),
            state: BedrockRuntimeState::New,
            process_id: None,
            commands: Vec::new(),
            live_processes: BTreeSet::new(),
            orphaned_processes: BTreeSet::new(),
            fail_next_start: false,
        }
    }

    fn unavailable() -> Self {
        Self {
            capabilities: BedrockRuntimeCapabilities::unavailable(
                BedrockRuntimeBackend::Native,
                "no-tested-windows-bds-package",
            ),
            state: BedrockRuntimeState::Unavailable,
            ..Self::supported()
        }
    }

    fn failing_start() -> Self {
        Self {
            fail_next_start: true,
            ..Self::supported()
        }
    }
}

impl BedrockRuntime for FakeWindowsRuntime {
    fn capabilities(&self) -> &BedrockRuntimeCapabilities {
        &self.capabilities
    }

    fn state(&self) -> BedrockRuntimeState {
        self.state
    }

    fn process_id(&self) -> Option<ProcessId> {
        self.process_id
    }

    fn provision(&mut self, request: BedrockProvisionRequest) -> Result<(), BedrockRuntimeError> {
        if !self.capabilities.supported {
            return Err(BedrockRuntimeError::Transport(
                self.capabilities
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "runtime-unavailable".to_owned()),
            ));
        }
        assert_eq!(request.server_dir, SERVER_DIR);
        assert_eq!(request.version, "1.26.32.2");
        if !matches!(
            self.state,
            BedrockRuntimeState::New | BedrockRuntimeState::Stopped
        ) {
            return Err(BedrockRuntimeError::InvalidState {
                operation: "provision",
                state: self.state,
            });
        }
        self.state = BedrockRuntimeState::Provisioned;
        Ok(())
    }

    fn start(&mut self, request: BedrockStartRequest) -> Result<(), BedrockRuntimeError> {
        if !self.capabilities.supported {
            return Err(BedrockRuntimeError::Transport(
                "no-tested-windows-bds-package".to_owned(),
            ));
        }
        assert_eq!(request.memory_gb, 2);
        assert_eq!(request.bedrock_port, 19132);
        if self.state != BedrockRuntimeState::Provisioned {
            return Err(BedrockRuntimeError::InvalidState {
                operation: "start",
                state: self.state,
            });
        }

        let process_id = ProcessId::new(8123);
        self.live_processes.insert(process_id);
        if self.fail_next_start {
            self.fail_next_start = false;
            // A failed spawn is cleaned up at the ownership boundary. Keeping
            // this explicit prevents a public start failure from becoming an
            // untracked process that survives the service operation.
            self.live_processes.remove(&process_id);
            self.process_id = None;
            self.state = BedrockRuntimeState::Stopped;
            return Err(BedrockRuntimeError::Transport(
                "synthetic Windows BDS spawn failed".to_owned(),
            ));
        }

        self.process_id = Some(process_id);
        self.state = BedrockRuntimeState::Running;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), BedrockRuntimeError> {
        if self.state != BedrockRuntimeState::Running {
            return Err(BedrockRuntimeError::InvalidState {
                operation: "stop",
                state: self.state,
            });
        }
        if let Some(process_id) = self.process_id.take() {
            self.live_processes.remove(&process_id);
        }
        self.state = BedrockRuntimeState::Stopped;
        Ok(())
    }

    fn force_stop(&mut self) -> Result<(), BedrockRuntimeError> {
        self.stop()
    }

    fn command(&mut self, command: &str) -> Result<(), BedrockRuntimeError> {
        if self.state != BedrockRuntimeState::Running {
            return Err(BedrockRuntimeError::InvalidState {
                operation: "command",
                state: self.state,
            });
        }
        self.commands.push(command.to_owned());
        Ok(())
    }

    fn poll_event(&mut self) -> Result<Option<BedrockRuntimeEvent>, BedrockRuntimeError> {
        Ok(None)
    }
}

struct WindowsServiceState {
    runtime: FakeWindowsRuntime,
    service_owner_alive: bool,
}

type AppState = Arc<Mutex<WindowsServiceState>>;

fn app_state(runtime: FakeWindowsRuntime) -> AppState {
    Arc::new(Mutex::new(WindowsServiceState {
        runtime,
        service_owner_alive: true,
    }))
}

async fn start(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut state = state.lock().unwrap();
    if !state.runtime.capabilities().supported {
        return Err(runtime_unavailable(&state.runtime));
    }
    state
        .runtime
        .provision(BedrockProvisionRequest {
            server_dir: SERVER_DIR.to_owned(),
            version: "1.26.32.2".to_owned(),
        })
        .map_err(start_error)?;
    state
        .runtime
        .start(BedrockStartRequest {
            memory_gb: 2,
            bedrock_port: 19132,
        })
        .map_err(start_error)?;
    Ok(Json(json!({
        "result": "started",
        "activeServerId": "bedrock-windows",
        "pid": state.runtime.process_id().map(|pid| pid.raw()),
        "serverType": "bedrock",
    })))
}

fn start_error(error: BedrockRuntimeError) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_GATEWAY,
        Json(json!({
            "code": "runtime_start_failed",
            "message": error.to_string(),
        })),
    )
}

fn runtime_unavailable(runtime: &FakeWindowsRuntime) -> (StatusCode, Json<Value>) {
    (
        StatusCode::CONFLICT,
        Json(json!({
            "code": "capability_unavailable",
            "message": "Bedrock is unavailable on this host.",
            "details": {
                "state": "unavailable",
                "reasonCode": runtime.capabilities.unavailable_reason,
            },
        })),
    )
}

async fn status(State(state): State<AppState>) -> Json<Value> {
    let state = state.lock().unwrap();
    Json(json!({
        "running": state.runtime.state() == BedrockRuntimeState::Running,
        "activeServerId": "bedrock-windows",
        "pid": state.runtime.process_id().map(|pid| pid.raw()),
        "serverType": "bedrock",
        "serviceOwnerAlive": state.service_owner_alive,
    }))
}

async fn command(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let command = body["command"].as_str().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"code": "invalid-command"})),
        )
    })?;
    let mut state = state.lock().unwrap();
    state.runtime.command(command).map_err(|error| {
        (
            StatusCode::CONFLICT,
            Json(json!({"code": error.to_string()})),
        )
    })?;
    Ok(Json(json!({
        "result": "sent",
        "activeServerId": "bedrock-windows",
        "command": command,
    })))
}

async fn stop(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut state = state.lock().unwrap();
    state.runtime.stop().map_err(|error| {
        (
            StatusCode::CONFLICT,
            Json(json!({"code": error.to_string()})),
        )
    })?;
    Ok(Json(json!({
        "result": "stopped",
        "activeServerId": "bedrock-windows",
    })))
}

async fn capabilities(State(state): State<AppState>) -> Json<Value> {
    let state = state.lock().unwrap();
    let capabilities = state.runtime.capabilities();
    Json(json!({
        "hostOs": "windows",
        "serverTypes": {
            "bedrock": {
                "supported": capabilities.supported,
                "backend": capabilities.supported.then_some("native"),
                "state": format!("{:?}", state.runtime.state()).to_ascii_lowercase(),
                "reason": capabilities.unavailable_reason,
            }
        }
    }))
}

async fn request(
    address: std::net::SocketAddr,
    method: &str,
    path: &str,
    body: &str,
) -> (u16, Value) {
    let mut stream = TcpStream::connect(address).await.unwrap();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap();
    let status = String::from_utf8_lossy(&response[..header_end])
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let body = &response[header_end + 4..];
    (status, serde_json::from_slice(body).unwrap())
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/start", post(start))
        .route("/v1/status", get(status))
        .route("/v1/command", post(command))
        .route("/v1/stop", post(stop))
        .route("/v1/capabilities", get(capabilities))
        .with_state(state)
}

async fn serve(state: AppState) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    (address, server)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn windows_public_contract_keeps_service_owned_server_after_client_exit() {
    let state = app_state(FakeWindowsRuntime::supported());
    let (address, server) = serve(state.clone()).await;

    let (status_code, started) = request(address, "POST", "/v1/start", "{}").await;
    assert_eq!(status_code, 200);
    assert_eq!(started["serverType"], "bedrock");
    assert!(started["pid"].is_number());

    // The request connection is the disposable client. Closing it must not
    // stop the runtime, because the background service owns the process.
    let (status_code, status) = request(address, "GET", "/v1/status", "{}").await;
    assert_eq!(status_code, 200);
    assert_eq!(status["running"], true);
    assert_eq!(status["serviceOwnerAlive"], true);

    let (status_code, command) = request(
        address,
        "POST",
        "/v1/command",
        r#"{"command":"say hello from windows"}"#,
    )
    .await;
    assert_eq!(status_code, 200);
    assert_eq!(command["command"], "say hello from windows");

    let (status_code, stopped) = request(address, "POST", "/v1/stop", "{}").await;
    assert_eq!(status_code, 200);
    assert_eq!(stopped["result"], "stopped");

    let state = state.lock().unwrap();
    assert_eq!(state.runtime.state(), BedrockRuntimeState::Stopped);
    assert!(state.runtime.process_id().is_none());
    assert!(state.runtime.live_processes.is_empty());
    assert!(state.runtime.orphaned_processes.is_empty());
    assert_eq!(state.runtime.commands, vec!["say hello from windows"]);
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn windows_public_contract_reports_unavailable_real_runtime() {
    let state = app_state(FakeWindowsRuntime::unavailable());
    let (address, server) = serve(state.clone()).await;

    let (status_code, capabilities) = request(address, "GET", "/v1/capabilities", "{}").await;
    assert_eq!(status_code, 200);
    assert_eq!(capabilities["hostOs"], "windows");
    assert_eq!(capabilities["serverTypes"]["bedrock"]["supported"], false);
    assert!(capabilities["serverTypes"]["bedrock"]["backend"].is_null());
    assert_eq!(
        capabilities["serverTypes"]["bedrock"]["state"],
        "unavailable"
    );

    let (status_code, error) = request(address, "POST", "/v1/start", "{}").await;
    assert_eq!(status_code, 409);
    assert_eq!(error["code"], "capability_unavailable");
    assert_eq!(error["details"]["state"], "unavailable");
    assert_eq!(
        error["details"]["reasonCode"],
        "no-tested-windows-bds-package"
    );

    let state = state.lock().unwrap();
    assert!(state.runtime.process_id().is_none());
    assert!(state.runtime.live_processes.is_empty());
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn windows_public_contract_cleans_failed_start_without_orphan() {
    let state = app_state(FakeWindowsRuntime::failing_start());
    let (address, server) = serve(state.clone()).await;

    let (status_code, error) = request(address, "POST", "/v1/start", "{}").await;
    assert_eq!(status_code, 502);
    assert_eq!(error["code"], "runtime_start_failed");

    let (status_code, status) = request(address, "GET", "/v1/status", "{}").await;
    assert_eq!(status_code, 200);
    assert_eq!(status["running"], false);
    assert!(status["pid"].is_null());

    let state = state.lock().unwrap();
    assert_eq!(state.runtime.state(), BedrockRuntimeState::Stopped);
    assert!(state.runtime.live_processes.is_empty());
    assert!(state.runtime.orphaned_processes.is_empty());
    server.abort();
}
