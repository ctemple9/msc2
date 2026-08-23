use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Json, Router, routing::get, routing::post};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Backend {
    LinuxNative,
    WindowsNative,
    MacosSidecar,
}

impl Backend {
    pub const ALL: [Self; 3] = [Self::LinuxNative, Self::WindowsNative, Self::MacosSidecar];

    pub fn name(self) -> &'static str {
        match self {
            Self::LinuxNative => "linux-native",
            Self::WindowsNative => "windows-native",
            Self::MacosSidecar => "macos-sidecar",
        }
    }

    fn server_id(self) -> &'static str {
        match self {
            Self::LinuxNative => "bedrock-linux",
            Self::WindowsNative => "bedrock-windows",
            Self::MacosSidecar => "bedrock-macos",
        }
    }

    fn host_os(self) -> &'static str {
        match self {
            Self::LinuxNative => "linux",
            Self::WindowsNative => "windows",
            Self::MacosSidecar => "macos",
        }
    }

    fn api_backend(self) -> &'static str {
        match self {
            Self::LinuxNative | Self::WindowsNative => "native",
            Self::MacosSidecar => "vz-sidecar",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Lifecycle {
    Stopped,
    Running,
}

pub struct SmokeState {
    pub backend: Backend,
    pub available: bool,
    lifecycle: Lifecycle,
    pub provision_count: usize,
    pub commands: Vec<String>,
    pub settings: BTreeMap<String, String>,
    pub allowlist: Vec<Value>,
    pub operation: Option<Value>,
    console: Vec<Value>,
    players: Vec<Value>,
}

type SharedState = Arc<Mutex<SmokeState>>;

pub struct Harness {
    pub address: std::net::SocketAddr,
    task: tokio::task::JoinHandle<()>,
}

impl Harness {
    pub fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    pub fn stop(self) {
        self.task.abort();
    }
}

pub fn fixture(path: &str) -> Value {
    let contents = match path {
        "console-ready" => {
            include_str!(
                "../../../../fixtures/bedrock-runtime/server-started-substring-is-readiness.json"
            )
        }
        "console-player" => {
            include_str!(
                "../../../../fixtures/bedrock-console/connect-extracts-xuid-after-comma.json"
            )
        }
        "properties" => {
            include_str!(
                "../../../../fixtures/bedrock-properties/model-reads-all-recognized-values.json"
            )
        }
        _ => panic!("unknown Bedrock smoke fixture: {path}"),
    };
    serde_json::from_str(contents).unwrap()
}

pub async fn spawn(backend: Backend, available: bool) -> Harness {
    let ready_fixture = fixture("console-ready");
    let player_fixture = fixture("console-player");
    let properties_fixture = fixture("properties");
    let expected_properties = &properties_fixture["expected"];
    let mut settings = BTreeMap::new();
    settings.insert(
        "level-name".to_owned(),
        expected_properties["level_name"]
            .as_str()
            .unwrap()
            .to_owned(),
    );
    settings.insert(
        "difficulty".to_owned(),
        expected_properties["difficulty"]
            .as_str()
            .unwrap()
            .to_owned(),
    );

    let state = Arc::new(Mutex::new(SmokeState {
        backend,
        available,
        lifecycle: Lifecycle::Stopped,
        provision_count: 0,
        commands: Vec::new(),
        settings,
        allowlist: vec![json!({
            "name": "Alex",
            "xuid": "2535416361514257",
            "ignoresPlayerLimit": false
        })],
        operation: None,
        console: vec![json!({
            "ts": "2026-08-23T12:00:00Z",
            "source": "bedrock",
            "level": "info",
            "text": ready_fixture["input"]["line"]
        })],
        players: player_fixture["expected"]["online_players"]
            .as_array()
            .unwrap()
            .clone(),
    }));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task_state = state.clone();
    let task = tokio::spawn(async move {
        axum::serve(listener, router(task_state)).await.unwrap();
    });
    Harness { address, task }
}

fn router(state: SharedState) -> Router {
    Router::new()
        .route("/v1/start", post(start))
        .route("/v1/status", get(status))
        .route("/v1/command", post(command))
        .route("/v1/stop", post(stop))
        .route("/v1/console/tail", get(console_tail))
        .route("/v1/players", get(players))
        .route("/v1/settings", get(settings).post(update_settings))
        .route("/v1/allowlist", get(allowlist).post(update_allowlist))
        .route("/v1/capabilities", get(capabilities))
        .route("/v1/operations", post(create_operation))
        .route("/v1/operations/:id", get(get_operation))
        .route("/v1/operations/:id/cancel", post(cancel_operation))
        .with_state(state)
}

fn unavailable(state: &SmokeState) -> (StatusCode, Json<Value>) {
    (
        StatusCode::CONFLICT,
        Json(json!({
            "code": "capability_unavailable",
            "message": "Bedrock is unavailable on this host.",
            "helpId": "bedrock.runtime-unavailable",
            "details": {
                "capability": "bedrock-runtime",
                "serverType": "bedrock",
                "state": "unavailable",
                "backend": null,
                "reasonCode": "no_test_hardware",
                "hostOs": state.backend.host_os()
            }
        })),
    )
}

async fn start(State(state): State<SharedState>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut state = state.lock().unwrap();
    if !state.available {
        return Err(unavailable(&state));
    }
    state.provision_count += 1;
    state.lifecycle = Lifecycle::Running;
    Ok(Json(json!({
        "result": "started",
        "activeServerId": state.backend.server_id(),
        "serverType": "bedrock",
        "backend": state.backend.api_backend()
    })))
}

async fn status(State(state): State<SharedState>) -> Json<Value> {
    let state = state.lock().unwrap();
    Json(json!({
        "running": state.lifecycle == Lifecycle::Running,
        "activeServerId": state.backend.server_id(),
        "pid": if state.lifecycle == Lifecycle::Running { json!(8123) } else { Value::Null },
        "serverType": "bedrock"
    }))
}

async fn command(
    State(state): State<SharedState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let command = body["command"].as_str().unwrap_or_default().trim();
    let mut state = state.lock().unwrap();
    if state.lifecycle != Lifecycle::Running {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"code": "conflict", "message": "Bedrock is not running."})),
        ));
    }
    state.commands.push(command.to_owned());
    state.console.push(json!({
        "ts": "2026-08-23T12:00:01Z",
        "source": "bedrock",
        "level": "info",
        "text": format!("command received: {command}")
    }));
    Ok(Json(json!({
        "result": "sent",
        "activeServerId": state.backend.server_id(),
        "command": command
    })))
}

async fn stop(State(state): State<SharedState>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut state = state.lock().unwrap();
    if state.lifecycle != Lifecycle::Running {
        return Err((
            StatusCode::CONFLICT,
            Json(json!({"code": "conflict", "message": "Bedrock is not running."})),
        ));
    }
    state.lifecycle = Lifecycle::Stopped;
    Ok(Json(json!({
        "result": "stopped",
        "activeServerId": state.backend.server_id()
    })))
}

async fn console_tail(State(state): State<SharedState>) -> Json<Vec<Value>> {
    Json(state.lock().unwrap().console.clone())
}

async fn players(State(state): State<SharedState>) -> Json<Value> {
    let state = state.lock().unwrap();
    Json(json!({
        "players": state.players,
        "count": state.players.len(),
        "serverType": "bedrock"
    }))
}

fn settings_payload(state: &SmokeState) -> Value {
    let fields = state
        .settings
        .iter()
        .map(|(key, value)| {
            json!({
                "key": key,
                "label": key,
                "type": "string",
                "value": value
            })
        })
        .collect::<Vec<_>>();
    json!({
        "serverType": "bedrock",
        "serverName": state.backend.server_id(),
        "serverRunning": state.lifecycle == Lifecycle::Running,
        "editable": true,
        "sections": [{"id": "bedrock", "title": "Bedrock", "icon": "cube", "fields": fields}]
    })
}

async fn settings(State(state): State<SharedState>) -> Json<Value> {
    Json(settings_payload(&state.lock().unwrap()))
}

async fn update_settings(State(state): State<SharedState>, Json(body): Json<Value>) -> Json<Value> {
    let mut state = state.lock().unwrap();
    let mut applied_keys = Vec::new();
    if let Some(changes) = body["changes"].as_object() {
        for (key, value) in changes {
            state
                .settings
                .insert(key.clone(), value.as_str().unwrap_or_default().to_owned());
            applied_keys.push(key.clone());
        }
    }
    Json(json!({
        "success": true,
        "message": "settings updated",
        "restartRequired": false,
        "appliedKeys": applied_keys,
        "sections": settings_payload(&state)["sections"]
    }))
}

async fn allowlist(State(state): State<SharedState>) -> Json<Value> {
    let state = state.lock().unwrap();
    Json(json!({
        "serverType": "bedrock",
        "entries": state.allowlist
    }))
}

async fn update_allowlist(
    State(state): State<SharedState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let mut state = state.lock().unwrap();
    let action = body["action"].as_str().unwrap_or_default();
    let name = body["name"].as_str().unwrap_or_default();
    if action == "add"
        && !state
            .allowlist
            .iter()
            .any(|entry| entry["name"].as_str() == Some(name))
    {
        state.allowlist.push(json!({
            "name": name,
            "xuid": null,
            "ignoresPlayerLimit": false
        }));
    }
    if action == "remove" {
        state
            .allowlist
            .retain(|entry| entry["name"].as_str() != Some(name));
    }
    Json(json!({
        "success": true,
        "message": action,
        "serverType": "bedrock",
        "entries": state.allowlist
    }))
}

async fn capabilities(State(state): State<SharedState>) -> Json<Value> {
    let state = state.lock().unwrap();
    Json(json!({
        "agentVersion": "0.1.0",
        "apiMajor": 1,
        "apiMinor": 0,
        "hostOs": state.backend.host_os(),
        "permissions": [],
        "serverTypes": {
            "vanilla": false,
            "paper": false,
            "fabric": false,
            "forge": false,
            "neoforge": false,
            "bedrock": {
                "supported": state.available,
                "backend": state.available.then_some(state.backend.api_backend())
            }
        },
        "helpers": {"playit": false, "duckdns": false, "geyser": false},
        "runtime": {
            "state": if state.available { "available" } else { "unavailable" },
            "backend": state.available.then_some(state.backend.api_backend()),
            "reasonCode": (!state.available).then_some("no_test_hardware")
        }
    }))
}

async fn create_operation(
    State(state): State<SharedState>,
    Json(_body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut state = state.lock().unwrap();
    if !state.available {
        return Err(unavailable(&state));
    }
    let id = format!("synthetic-{}", state.backend.name());
    let operation = json!({
        "id": id,
        "type": "bedrock-provision",
        "state": "running",
        "cancelable": true,
        "progress": 0.25,
        "statusLine": "staging verified Bedrock files"
    });
    state.operation = Some(operation.clone());
    Ok(Json(operation))
}

async fn get_operation(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let state = state.lock().unwrap();
    match &state.operation {
        Some(operation) if operation["id"].as_str() == Some(id.as_str()) => {
            Ok(Json(operation.clone()))
        }
        _ => Err((StatusCode::NOT_FOUND, Json(json!({"code": "not_found"})))),
    }
}

async fn cancel_operation(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mut state = state.lock().unwrap();
    let cancelled = {
        let Some(operation) = state.operation.as_mut() else {
            return Err((StatusCode::NOT_FOUND, Json(json!({"code": "not_found"}))));
        };
        if operation["id"].as_str() != Some(id.as_str()) {
            return Err((StatusCode::NOT_FOUND, Json(json!({"code": "not_found"}))));
        }
        operation["state"] = json!("cancelled");
        operation["cancelable"] = json!(false);
        operation.clone()
    };
    state.lifecycle = Lifecycle::Stopped;
    Ok(Json(cancelled))
}

pub async fn request(
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
    (
        status,
        serde_json::from_slice(&response[header_end + 4..]).unwrap(),
    )
}
