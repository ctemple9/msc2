//! P10.13's offline HTTP contract proof.
//!
//! The production Bedrock routes are wired in a later step. This test keeps
//! the public contract honest by putting the shared runtime boundary behind a
//! loopback Axum router and replacing BDS with an in-memory runtime. It
//! exercises the wire-shaped lifecycle without credentials, a world, or the
//! public network.

use axum::{Json, Router, extract::State, routing::get, routing::post};
use msc_application::bedrock_runtime::{
    BedrockProvisionRequest, BedrockRuntime, BedrockRuntimeBackend, BedrockRuntimeCapabilities,
    BedrockRuntimeError, BedrockRuntimeEvent, BedrockRuntimeState, BedrockStartRequest,
    BedrockTerminationReason,
};
use msc_application::lifecycle::LifecycleState;
use msc_infrastructure::process::ProcessId;
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const SERVER_DIR: &str = "/synthetic/bedrock";

struct FakeRuntime {
    state: BedrockRuntimeState,
    events: VecDeque<BedrockRuntimeEvent>,
    commands: Vec<String>,
    provision_count: usize,
    start_count: usize,
    process_id: Option<ProcessId>,
}

impl FakeRuntime {
    fn new() -> Self {
        Self {
            state: BedrockRuntimeState::New,
            events: VecDeque::new(),
            commands: Vec::new(),
            provision_count: 0,
            start_count: 0,
            process_id: Some(ProcessId::new(77)),
        }
    }

    fn emit(&mut self, event: BedrockRuntimeEvent) {
        self.events.push_back(event);
    }
}

impl BedrockRuntime for FakeRuntime {
    fn capabilities(&self) -> &BedrockRuntimeCapabilities {
        static CAPABILITIES: std::sync::OnceLock<BedrockRuntimeCapabilities> =
            std::sync::OnceLock::new();
        CAPABILITIES
            .get_or_init(|| BedrockRuntimeCapabilities::supported(BedrockRuntimeBackend::Native))
    }

    fn state(&self) -> BedrockRuntimeState {
        self.state
    }

    fn process_id(&self) -> Option<ProcessId> {
        self.process_id
    }

    fn provision(&mut self, request: BedrockProvisionRequest) -> Result<(), BedrockRuntimeError> {
        assert_eq!(request.server_dir, SERVER_DIR);
        assert_eq!(request.version, "1.21.80.3");
        assert!(matches!(
            self.state,
            BedrockRuntimeState::New | BedrockRuntimeState::Stopped
        ));
        self.provision_count += 1;
        self.state = BedrockRuntimeState::Provisioned;
        Ok(())
    }

    fn start(&mut self, request: BedrockStartRequest) -> Result<(), BedrockRuntimeError> {
        assert_eq!(request.memory_gb, 2);
        assert_eq!(request.bedrock_port, 19132);
        assert_eq!(self.state, BedrockRuntimeState::Provisioned);
        self.start_count += 1;
        self.state = BedrockRuntimeState::Starting;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), BedrockRuntimeError> {
        self.state = BedrockRuntimeState::Stopping;
        Ok(())
    }

    fn force_stop(&mut self) -> Result<(), BedrockRuntimeError> {
        self.state = BedrockRuntimeState::Stopping;
        Ok(())
    }

    fn command(&mut self, command: &str) -> Result<(), BedrockRuntimeError> {
        self.commands.push(command.to_owned());
        Ok(())
    }

    fn poll_event(&mut self) -> Result<Option<BedrockRuntimeEvent>, BedrockRuntimeError> {
        if matches!(
            self.events.front(),
            Some(BedrockRuntimeEvent::Terminated { .. })
        ) {
            self.state = BedrockRuntimeState::Stopped;
        }
        Ok(self.events.pop_front())
    }
}

struct ContractState {
    lifecycle: LifecycleState,
    runtime: FakeRuntime,
    metric_history_len: usize,
}

type AppState = Arc<Mutex<ContractState>>;

fn app_state() -> AppState {
    Arc::new(Mutex::new(ContractState {
        lifecycle: LifecycleState::Stopped,
        runtime: FakeRuntime::new(),
        metric_history_len: 0,
    }))
}

async fn start(State(state): State<AppState>) -> Json<Value> {
    let mut state = state.lock().unwrap();
    state
        .runtime
        .provision(BedrockProvisionRequest {
            server_dir: SERVER_DIR.into(),
            version: "1.21.80.3".into(),
        })
        .unwrap();
    state
        .runtime
        .start(BedrockStartRequest {
            memory_gb: 2,
            bedrock_port: 19132,
        })
        .unwrap();
    state.runtime.emit(BedrockRuntimeEvent::Ready {
        address: Some("127.0.0.1".into()),
        port: 19132,
    });
    state.runtime.poll_event().unwrap();
    state.lifecycle = LifecycleState::Running;
    Json(json!({
        "result": "started",
        "activeServerId": "bedrock-linux",
    }))
}

async fn status(State(state): State<AppState>) -> Json<Value> {
    let state = state.lock().unwrap();
    Json(json!({
        "running": state.lifecycle == LifecycleState::Running,
        "activeServerId": "bedrock-linux",
        "pid": state.runtime.process_id().map(|pid| pid.raw()),
        "serverType": "bedrock",
    }))
}

async fn command(State(state): State<AppState>, Json(body): Json<Value>) -> Json<Value> {
    let command = body["command"].as_str().unwrap();
    let mut state = state.lock().unwrap();
    assert_eq!(state.lifecycle, LifecycleState::Running);
    state.runtime.command(command).unwrap();
    Json(json!({
        "result": "sent",
        "activeServerId": "bedrock-linux",
        "command": command,
    }))
}

async fn stop(State(state): State<AppState>) -> Json<Value> {
    let mut state = state.lock().unwrap();
    assert_eq!(state.lifecycle, LifecycleState::Running);
    state.runtime.stop().unwrap();
    state.runtime.emit(BedrockRuntimeEvent::Terminated {
        reason: BedrockTerminationReason::Clean,
    });
    state.runtime.poll_event().unwrap();
    state.lifecycle = LifecycleState::Stopped;
    Json(json!({
        "result": "stopped",
        "activeServerId": "bedrock-linux",
    }))
}

async fn performance(State(state): State<AppState>) -> Json<Value> {
    let mut state = state.lock().unwrap();
    assert_eq!(state.lifecycle, LifecycleState::Running);
    state.metric_history_len += 1;
    Json(json!({
        "ts": "2026-08-23T12:00:01Z",
        "playersOnline": 0,
        "cpuPercent": 12.5,
        "ramUsedMb": 512.0,
        "metricHistoryLen": state.metric_history_len,
        "serverType": "bedrock",
    }))
}

async fn capabilities() -> Json<Value> {
    Json(json!({
        "serverTypes": {
            "bedrock": {"supported": false, "backend": null}
        },
        "reason": "synthetic host has no live BDS boundary",
    }))
}

async fn request(addr: std::net::SocketAddr, method: &str, path: &str, body: &str) -> Value {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let body_start = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap()
        + 4;
    let body = &response[body_start..];
    serde_json::from_slice(body).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn linux_public_contract_covers_lifecycle_metrics_and_unavailability() {
    let state = app_state();
    let app = Router::new()
        .route("/v1/start", post(start))
        .route("/v1/status", get(status))
        .route("/v1/command", post(command))
        .route("/v1/stop", post(stop))
        .route("/v1/performance", get(performance))
        .route("/v1/capabilities", get(capabilities))
        .with_state(state.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let started = request(address, "POST", "/v1/start", "{}").await;
    assert_eq!(started["result"], "started");
    assert_eq!(
        request(address, "GET", "/v1/status", "{}").await["serverType"],
        "bedrock"
    );
    let command = request(
        address,
        "POST",
        "/v1/command",
        r#"{"command":"say hello from linux"}"#,
    )
    .await;
    assert_eq!(command["command"], "say hello from linux");
    let performance = request(address, "GET", "/v1/performance", "{}").await;
    assert_eq!(performance["cpuPercent"], 12.5);
    assert_eq!(performance["ramUsedMb"], 512.0);
    assert_eq!(performance["metricHistoryLen"], 1);
    assert_eq!(
        request(address, "POST", "/v1/stop", "{}").await["result"],
        "stopped"
    );
    let unavailable = request(address, "GET", "/v1/capabilities", "{}").await;
    assert_eq!(unavailable["serverTypes"]["bedrock"]["supported"], false);
    assert!(unavailable["serverTypes"]["bedrock"]["backend"].is_null());

    let state = state.lock().unwrap();
    assert_eq!(state.lifecycle, LifecycleState::Stopped);
    assert_eq!(state.runtime.provision_count, 1);
    assert_eq!(state.runtime.start_count, 1);
    assert_eq!(state.runtime.commands, vec!["say hello from linux"]);
    server.abort();
}
