use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Json, Router, routing::get, routing::post};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// The production smoke runs once per CI host.  Keeping the expected backend
/// in one adapter table makes a platform job fail if the composition root
/// selects the wrong runtime, without pretending that one host can emulate
/// another host's process or VM boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionBackend {
    LinuxNative,
    WindowsNative,
    MacosSidecar,
}

impl ProductionBackend {
    pub const ALL: [Self; 3] = [Self::LinuxNative, Self::WindowsNative, Self::MacosSidecar];

    pub const fn current() -> Self {
        #[cfg(target_os = "linux")]
        {
            return Self::LinuxNative;
        }
        #[cfg(target_os = "windows")]
        {
            return Self::WindowsNative;
        }
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            return Self::MacosSidecar;
        }
        #[allow(unreachable_code)]
        Self::MacosSidecar
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::LinuxNative => "linux-native",
            Self::WindowsNative => "windows-native",
            Self::MacosSidecar => "macos-sidecar",
        }
    }

    pub const fn host_os(self) -> &'static str {
        match self {
            Self::LinuxNative => "linux",
            Self::WindowsNative => "windows",
            Self::MacosSidecar => "macos",
        }
    }

    pub const fn api_backend(self) -> &'static str {
        match self {
            Self::LinuxNative | Self::WindowsNative => "native",
            Self::MacosSidecar => "vz-sidecar",
        }
    }
}

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

/// Disposable on-disk state for the production-router smoke. The binary is
/// started normally, so the selected runtime and operation journal are the
/// same ones used outside tests; only the BDS/sidecar inputs are synthetic.
pub struct ProductionFixture {
    pub root: std::path::PathBuf,
    pub data_dir: std::path::PathBuf,
    pub config_path: std::path::PathBuf,
    pub servers_root: std::path::PathBuf,
    pub server_dir: std::path::PathBuf,
    pub import_source: std::path::PathBuf,
    pub port: u16,
    keychain_service: String,
}

impl ProductionFixture {
    pub fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "msc2-bedrock-production-smoke-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        let data_dir = root.join("data");
        let servers_root = root.join("servers");
        let server_dir = servers_root.join("bedrock").join("smoke-fixture");
        let import_source = root.join("import-source");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(&server_dir).unwrap();
        std::fs::create_dir_all(&import_source).unwrap();
        Self {
            config_path: data_dir.join("server_config_swift.json"),
            keychain_service: format!(
                "com.msc2.bedrock-production-smoke.{}.{}",
                std::process::id(),
                unique_suffix()
            ),
            port: free_port(),
            root,
            data_dir,
            servers_root,
            server_dir,
            import_source,
        }
    }

    pub fn seed(&self, backend: ProductionBackend) {
        self.seed_server(&self.server_dir, backend);
        self.seed_server(&self.import_source, backend);
        self.write_config();
    }

    pub fn seed_unavailable(&self, backend: ProductionBackend) {
        std::fs::create_dir_all(&self.server_dir).unwrap();
        let _ = backend;
        self.write_config();
    }

    fn write_config(&self) {
        let mut config = msc_domain::app_config_schema::AppConfig::default_config(
            self.servers_root.to_string_lossy(),
        );
        let mut server = msc_domain::app_config_schema::ConfigServer::new(
            "bedrock-production-smoke",
            "Production Bedrock Smoke",
            self.server_dir.to_string_lossy(),
            "",
            1.0,
            2.0,
        );
        server.server_type = msc_domain::identity::ServerType::Bedrock;
        server.bedrock_enabled = true;
        server.bedrock_port = Some(19132);
        server.bedrock_version = Some("1.21.80.3".to_owned());
        config.servers.push(server);
        config.active_server_id = config.servers.first().map(|server| server.id.clone());
        std::fs::write(
            &self.config_path,
            serde_json::to_vec_pretty(&config.encode()).unwrap(),
        )
        .unwrap();
    }

    fn seed_server(&self, directory: &std::path::Path, backend: ProductionBackend) {
        std::fs::create_dir_all(directory.join("worlds/Realm/db")).unwrap();
        std::fs::write(
            directory.join("server.properties"),
            "level-name=Realm\ndifficulty=normal\nserver-port=19132\nmax-players=10\n",
        )
        .unwrap();
        std::fs::write(
            directory.join("allowlist.json"),
            r#"[{"name":"Alex","xuid":"123","ignoresPlayerLimit":false}]"#,
        )
        .unwrap();
        std::fs::write(directory.join("permissions.json"), "[]").unwrap();
        let executable = match backend {
            ProductionBackend::WindowsNative => "bedrock_server.exe",
            ProductionBackend::LinuxNative | ProductionBackend::MacosSidecar => "bedrock_server",
        };
        if backend != ProductionBackend::WindowsNative {
            std::fs::write(directory.join(executable), b"fixture adapter").unwrap();
            std::fs::write(
                directory.join(".msc_bds_provenance.json"),
                format!(
                    r#"{{"version":"1.21.80.3","platform":"{}","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#,
                    match backend {
                        ProductionBackend::LinuxNative => "linux",
                        ProductionBackend::WindowsNative => "windows",
                        ProductionBackend::MacosSidecar => "macos",
                    }
                ),
            )
            .unwrap();
        }
        #[cfg(unix)]
        if backend == ProductionBackend::LinuxNative {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(
                directory.join(executable),
                b"#!/bin/sh\nprintf 'Server started\\n'\nwhile IFS= read -r line; do\n  [ \"$line\" = stop ] && exit 0\ndone\n",
            )
            .unwrap();
            std::fs::set_permissions(
                directory.join(executable),
                std::fs::Permissions::from_mode(0o755),
            )
            .unwrap();
        }
    }

    pub fn spawn_agent(&self) -> std::process::Child {
        std::process::Command::new(env!("CARGO_BIN_EXE_msc"))
            .args(["serve", "--bind", &format!("127.0.0.1:{}", self.port)])
            .env("MSC2_APP_CONFIG_PATH", &self.config_path)
            .env("MSC2_AGENT_SERVERS_ROOT", &self.servers_root)
            .env(
                "MSC2_CREDENTIAL_REGISTRY_PATH",
                self.data_dir.join("credentials.json"),
            )
            .env("MSC2_OPERATION_JOURNAL_DIR", self.data_dir.join("journal"))
            .env(
                "MSC2_LINUX_FOREGROUND_SECRET_STORE_DIR",
                self.data_dir.join("secrets"),
            )
            .env(
                "MSC2_TEST_BOOTSTRAP_TOKEN",
                "msc2_bedrock-production-smoke_testsecret",
            )
            .env("MSC2_MACOS_USER_KEYCHAIN_SERVICE", &self.keychain_service)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap()
    }

    pub fn wait_for_health(&self) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while std::time::Instant::now() < deadline {
            if std::net::TcpStream::connect(std::net::SocketAddr::from(([127, 0, 0, 1], self.port)))
                .is_ok()
                && raw_http(self.port, "GET", "/v1/health", None).starts_with("HTTP/1.1 200")
            {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        panic!("production agent did not become healthy");
    }

    pub fn http(&self, method: &str, path: &str, body: Option<&str>) -> (u16, Value) {
        let response = raw_http(self.port, method, path, body);
        let status = response.split_whitespace().nth(1).unwrap().parse().unwrap();
        let body = response.split("\r\n\r\n").nth(1).unwrap_or_default();
        (status, serde_json::from_str(body).unwrap_or(Value::Null))
    }

    pub fn cli(&self, args: &[&str]) -> std::process::Output {
        std::process::Command::new(env!("CARGO_BIN_EXE_msc"))
            .args([
                "--base-url",
                &format!("http://127.0.0.1:{}", self.port),
                "--token",
                "msc2_bedrock-production-smoke_testsecret",
                "--json",
            ])
            .args(args)
            .output()
            .unwrap()
    }

    pub fn stop(&self, agent: &mut std::process::Child) {
        let _ = agent.kill();
        let _ = agent.wait();
        let _ = std::fs::remove_dir_all(&self.root);
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("security")
            .args([
                "delete-generic-password",
                "-s",
                &self.keychain_service,
                "-a",
                "remote-api.owner-token",
            ])
            .output();
    }
}

fn raw_http(port: u16, method: &str, path: &str, body: Option<&str>) -> String {
    use std::io::{Read, Write};
    let mut stream =
        std::net::TcpStream::connect(std::net::SocketAddr::from(([127, 0, 0, 1], port))).unwrap();
    let body = body.unwrap_or_default();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer msc2_bedrock-production-smoke_testsecret\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn free_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
