//! P10.18's synthetic macOS sidecar public-contract proof.
//!
//! The router is the agent boundary and the in-memory transport is a
//! disposable stand-in for the Swift sidecar's JSON-lines process. This test
//! proves the ordering and failure vocabulary without claiming that a real
//! Virtualization.framework appliance booted.

use axum::{Json, Router, extract::State, http::StatusCode, routing::get, routing::post};
use msc_application::bedrock_macos::{MacosBedrockHost, MacosBedrockRuntime};
use msc_application::bedrock_runtime::{
    BedrockProvisionRequest, BedrockRuntime, BedrockRuntimeError, BedrockRuntimeEvent,
    BedrockRuntimeState, BedrockStartRequest, BedrockTerminationReason, SidecarFrame,
    SidecarReceive, SidecarTransport, decode_frame, encode_frame,
};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const SERVER_DIR: &str = "/synthetic/bedrock-macos";

#[derive(Default)]
struct FakeSidecarTransport {
    sent: Vec<SidecarFrame>,
    responses: VecDeque<String>,
    dhcp_seen: bool,
    relay_started: bool,
    crashed: bool,
}

impl FakeSidecarTransport {
    fn push(&mut self, frame: SidecarFrame) {
        self.responses.push_back(encode_frame(&frame).unwrap());
    }

    fn emit_dhcp_then_ready(&mut self) {
        self.dhcp_seen = true;
        self.relay_started = true;
        self.push(SidecarFrame::ConsoleLine {
            line: "[appliance] dhcp: 192.168.64.7/24".to_owned(),
        });
        self.push(SidecarFrame::Ready {
            guest_ip: "192.168.64.7".to_owned(),
            port: 19132,
            relay_up: true,
        });
    }

    fn crash(&mut self) {
        self.responses.clear();
        self.crashed = true;
    }
}

impl SidecarTransport for FakeSidecarTransport {
    fn send_line(&mut self, line: &str) -> Result<(), String> {
        let frame = decode_frame(line).map_err(|error| error.to_string())?;
        self.sent.push(frame.clone());
        match frame {
            SidecarFrame::Provision { .. } => self.push(SidecarFrame::Provisioned {
                ok: true,
                reason: None,
            }),
            SidecarFrame::Start { .. } => self.push(SidecarFrame::Started {
                accepted: true,
                reason: None,
            }),
            SidecarFrame::Command { .. } => self.push(SidecarFrame::CommandResult {
                ok: true,
                reason: None,
            }),
            SidecarFrame::Stop => self.push(SidecarFrame::Terminated {
                reason: BedrockTerminationReason::Clean,
            }),
            SidecarFrame::ForceStop => self.push(SidecarFrame::Terminated {
                reason: BedrockTerminationReason::GuestError("forced".to_owned()),
            }),
            SidecarFrame::Provisioned { .. }
            | SidecarFrame::Started { .. }
            | SidecarFrame::Ready { .. }
            | SidecarFrame::CommandResult { .. }
            | SidecarFrame::ConsoleLine { .. }
            | SidecarFrame::Terminated { .. } => {}
        }
        Ok(())
    }

    fn receive_line(&mut self) -> Result<Option<String>, String> {
        Ok(self.responses.pop_front())
    }

    fn receive_status(&mut self) -> Result<SidecarReceive, String> {
        if self.crashed {
            return Ok(SidecarReceive::Eof);
        }
        Ok(self
            .responses
            .pop_front()
            .map(SidecarReceive::Line)
            .unwrap_or(SidecarReceive::Pending))
    }
}

struct AgentState {
    runtime: MacosBedrockRuntime<FakeSidecarTransport>,
    failure: Option<String>,
}

type AppState = Arc<Mutex<AgentState>>;

fn app_state() -> AppState {
    AppState::new(Mutex::new(AgentState {
        runtime: MacosBedrockRuntime::with_transport(
            FakeSidecarTransport::default(),
            MacosBedrockHost::Intel,
        ),
        failure: None,
    }))
}

fn error_response(status: StatusCode, error: impl ToString) -> (StatusCode, Json<Value>) {
    (
        status,
        Json(json!({
            "code": "bedrock_runtime_error",
            "message": error.to_string(),
        })),
    )
}

async fn start(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut state = state.lock().unwrap();
    state
        .runtime
        .provision(BedrockProvisionRequest {
            server_dir: SERVER_DIR.to_owned(),
            version: "1.26.32.2".to_owned(),
        })
        .map_err(|error| error_response(StatusCode::BAD_GATEWAY, error))?;
    state
        .runtime
        .start(BedrockStartRequest {
            memory_gb: 2,
            bedrock_port: 19132,
        })
        .map_err(|error| error_response(StatusCode::BAD_GATEWAY, error))?;
    Ok(Json(json!({
        "result": "starting",
        "serverType": "bedrock",
        "backend": "sidecar",
        "ready": false,
    })))
}

async fn status(State(state): State<AppState>) -> Json<Value> {
    let state = state.lock().unwrap();
    Json(json!({
        "running": state.runtime.state() == BedrockRuntimeState::Running,
        "state": format!("{:?}", state.runtime.state()).to_lowercase(),
        "serverType": "bedrock",
        "backend": "sidecar",
    }))
}

async fn command(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let command = body["command"]
        .as_str()
        .ok_or_else(|| error_response(StatusCode::BAD_REQUEST, "command must be a string"))?;
    let mut state = state.lock().unwrap();
    state
        .runtime
        .command(command)
        .map_err(|error| error_response(StatusCode::CONFLICT, error))?;
    Ok(Json(json!({
        "result": "sent",
        "command": command,
    })))
}

async fn stop(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut state = state.lock().unwrap();
    state
        .runtime
        .stop()
        .map_err(|error| error_response(StatusCode::CONFLICT, error))?;
    Ok(Json(json!({"result": "stopping"})))
}

async fn force_stop(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut state = state.lock().unwrap();
    state
        .runtime
        .force_stop()
        .map_err(|error| error_response(StatusCode::CONFLICT, error))?;
    Ok(Json(json!({"result": "stopping", "forced": true})))
}

async fn poll_event(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut state = state.lock().unwrap();
    let event = match state.runtime.poll_event() {
        Ok(Some(event)) => event,
        Ok(None) => return Ok(Json(json!({"event": null}))),
        Err(error) => {
            state.failure = Some(error.to_string());
            return Err(error_response(StatusCode::BAD_GATEWAY, error));
        }
    };
    Ok(Json(JsonValueEvent::from(event).0))
}

struct JsonValueEvent(Value);

impl From<BedrockRuntimeEvent> for JsonValueEvent {
    fn from(event: BedrockRuntimeEvent) -> Self {
        Self(match event {
            BedrockRuntimeEvent::Ready { address, port } => {
                json!({"event": {"type": "ready", "address": address, "port": port}})
            }
            BedrockRuntimeEvent::ConsoleLine(line) => {
                json!({"event": {"type": "console-line", "line": line}})
            }
            BedrockRuntimeEvent::Metrics(metrics) => json!({
                "event": {
                    "type": "metrics",
                    "cpuPercent": metrics.cpu_percent,
                    "ramUsedMb": metrics.ram_used_mb,
                }
            }),
            BedrockRuntimeEvent::Terminated { reason } => json!({
                "event": {
                    "type": "terminated",
                    "reason": format!("{reason:?}").to_lowercase(),
                }
            }),
        })
    }
}

async fn capabilities(State(state): State<AppState>) -> Json<Value> {
    let state = state.lock().unwrap();
    let supported = state.failure.is_none() && state.runtime.capabilities().supported;
    Json(json!({
        "backend": supported.then_some("sidecar"),
        "supported": supported,
        "state": format!("{:?}", state.runtime.state()).to_lowercase(),
        "reason": state.failure,
    }))
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/start", post(start))
        .route("/v1/status", get(status))
        .route("/v1/command", post(command))
        .route("/v1/stop", post(stop))
        .route("/v1/force-stop", post(force_stop))
        .route("/v1/events", get(poll_event))
        .route("/v1/capabilities", get(capabilities))
        .with_state(state)
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
    let headers = String::from_utf8_lossy(&response[..header_end]);
    let status = headers
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let body = serde_json::from_slice(&response[header_end + 4..]).unwrap();
    (status, body)
}

async fn server(state: AppState) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    (address, task)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn macos_routes_wait_for_dhcp_relay_and_complete_sidecar_lifecycle() {
    let state = app_state();
    let (address, task) = server(state.clone()).await;

    let (_, started) = request(address, "POST", "/v1/start", "{}").await;
    assert_eq!(started["ready"], false);
    assert_eq!(
        request(address, "GET", "/v1/status", "{}").await.1["state"],
        "starting"
    );

    {
        let mut state = state.lock().unwrap();
        state
            .runtime
            .sidecar_mut()
            .transport_mut()
            .emit_dhcp_then_ready();
    }
    let (_, console) = request(address, "GET", "/v1/events", "{}").await;
    assert_eq!(console["event"]["type"], "console-line");
    assert_eq!(
        request(address, "GET", "/v1/status", "{}").await.1["running"],
        false
    );
    let (_, ready) = request(address, "GET", "/v1/events", "{}").await;
    assert_eq!(ready["event"]["type"], "ready");
    assert_eq!(ready["event"]["address"], "192.168.64.7");
    assert_eq!(
        request(address, "GET", "/v1/status", "{}").await.1["running"],
        true
    );

    let (_, command) = request(
        address,
        "POST",
        "/v1/command",
        r#"{"command":"say hello from macOS"}"#,
    )
    .await;
    assert_eq!(command["command"], "say hello from macOS");
    let (_, stopping) = request(address, "POST", "/v1/stop", "{}").await;
    assert_eq!(stopping["result"], "stopping");
    let (_, terminated) = request(address, "GET", "/v1/events", "{}").await;
    assert_eq!(terminated["event"]["type"], "terminated");
    assert_eq!(
        request(address, "GET", "/v1/status", "{}").await.1["state"],
        "stopped"
    );

    let state = state.lock().unwrap();
    let transport = state.runtime.sidecar().transport();
    assert!(transport.dhcp_seen);
    assert!(transport.relay_started);
    assert_eq!(
        state
            .runtime
            .sidecar()
            .directory_mapping()
            .unwrap()
            .server_dir,
        SERVER_DIR
    );
    assert!(transport.sent.iter().any(|frame| matches!(
        frame,
        SidecarFrame::Command { command } if command == "say hello from macOS"
    )));
    assert!(
        transport
            .sent
            .iter()
            .any(|frame| matches!(frame, SidecarFrame::Stop))
    );
    task.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn macos_routes_report_forced_stop_and_sidecar_crash_as_distinct_outcomes() {
    let state = app_state();
    let (address, task) = server(state.clone()).await;
    request(address, "POST", "/v1/start", "{}").await;
    {
        let mut state = state.lock().unwrap();
        state
            .runtime
            .sidecar_mut()
            .transport_mut()
            .emit_dhcp_then_ready();
    }
    request(address, "GET", "/v1/events", "{}").await;
    request(address, "GET", "/v1/events", "{}").await;
    let (_, forced) = request(address, "POST", "/v1/force-stop", "{}").await;
    assert_eq!(forced["forced"], true);
    let (_, forced_event) = request(address, "GET", "/v1/events", "{}").await;
    assert_eq!(forced_event["event"]["reason"], "guesterror(\"forced\")");
    assert_eq!(
        request(address, "GET", "/v1/status", "{}").await.1["state"],
        "stopped"
    );
    task.abort();

    let crashed = app_state();
    let (address, task) = server(crashed.clone()).await;
    request(address, "POST", "/v1/start", "{}").await;
    crashed
        .lock()
        .unwrap()
        .runtime
        .sidecar_mut()
        .transport_mut()
        .crash();
    let (status, error) = request(address, "GET", "/v1/events", "{}").await;
    assert_eq!(status, 502);
    assert_eq!(error["code"], "bedrock_runtime_error");
    let (status, capabilities) = request(address, "GET", "/v1/capabilities", "{}").await;
    assert_eq!(status, 200);
    assert_eq!(capabilities["supported"], false);
    assert_eq!(capabilities["state"], "unavailable");
    assert_eq!(capabilities["reason"], "sidecar ended unexpectedly");
    task.abort();
}

#[test]
fn fresh_sidecar_runtime_reuses_host_directory_after_previous_vm_ends() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("msc-bedrock-macos-{unique}"));
    fs::create_dir_all(directory.join("worlds")).unwrap();
    let marker = directory.join("worlds").join("level.dat");
    fs::write(&marker, b"host-owned-world").unwrap();

    let server_dir = directory.to_string_lossy().into_owned();
    let mut first = MacosBedrockRuntime::with_transport(
        FakeSidecarTransport::default(),
        MacosBedrockHost::Intel,
    );
    first
        .provision(BedrockProvisionRequest {
            server_dir: server_dir.clone(),
            version: "1.26.32.2".to_owned(),
        })
        .unwrap();
    drop(first);

    let mut replacement = MacosBedrockRuntime::with_transport(
        FakeSidecarTransport::default(),
        MacosBedrockHost::Intel,
    );
    replacement
        .provision(BedrockProvisionRequest {
            server_dir,
            version: "1.26.32.2".to_owned(),
        })
        .unwrap();
    assert_eq!(fs::read(&marker).unwrap(), b"host-owned-world");
    assert_eq!(
        replacement
            .sidecar()
            .directory_mapping()
            .unwrap()
            .guest_mount,
        "/mnt"
    );
    assert_eq!(
        replacement.sidecar().directory_mapping().unwrap().tag,
        "world"
    );
    let _ = fs::remove_dir_all(marker.parent().unwrap().parent().unwrap());
}

#[test]
fn apple_silicon_capability_is_unavailable_and_does_not_start_sidecar() {
    let mut runtime = MacosBedrockRuntime::with_transport(
        FakeSidecarTransport::default(),
        MacosBedrockHost::AppleSilicon,
    );
    assert_eq!(runtime.state(), BedrockRuntimeState::Unavailable);
    assert!(!runtime.capabilities().supported);
    let error = runtime
        .provision(BedrockProvisionRequest {
            server_dir: SERVER_DIR.to_owned(),
            version: "1.26.32.2".to_owned(),
        })
        .unwrap_err();
    assert!(matches!(error, BedrockRuntimeError::Transport(_)));
}
