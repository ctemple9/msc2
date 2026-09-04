//! P10.32's production-router proof. These tests start the same binary that
//! serves real clients; the Linux case uses a disposable BDS-shaped process,
//! while other hosts prove the structured unavailable path.

use std::fs;
#[cfg(target_os = "linux")]
use std::io::Cursor;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
#[cfg(target_os = "linux")]
use std::sync::Arc;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use msc_domain::app_config_schema::{AppConfig, ConfigServer};
use msc_domain::identity::ServerType;
#[cfg(target_os = "linux")]
use msc_infrastructure::download_staging::sha256_hex;
use serde_json::Value;
#[cfg(target_os = "linux")]
use zip::write::SimpleFileOptions;

const TOKEN: &str = "msc2_bedrock-production-lifecycle_testsecret";

#[cfg(target_os = "linux")]
#[test]
fn production_router_runs_fixture_backed_bedrock_lifecycle() {
    let fixture = TestFixture::available_linux();
    let mut agent = fixture.spawn_agent();
    wait_for_health(fixture.port);

    let capabilities = fixture.get("/v1/capabilities");
    assert_eq!(capabilities["serverTypes"]["bedrock"]["supported"], true);
    assert_eq!(
        capabilities["serverTypes"]["bedrock"]["runtime"]["state"],
        "available"
    );

    assert_websocket_upgrade(fixture.port, "/v1/console/stream");
    let started = fixture.post("/v1/start", "{}");
    assert_eq!(started.0, 200, "start failed: {started:?}");
    assert_eq!(started.1["runtime"]["state"], "available");
    wait_until(&fixture, |value| value["running"] == true);

    let command = fixture.post("/v1/command", r#"{"command":"/say hello"}"#);
    assert_eq!(command.0, 200, "command failed: {command:?}");
    assert_eq!(command.1["command"], "say hello");

    let stopped = fixture.post("/v1/stop", "{}");
    assert_eq!(stopped.0, 200, "stop failed: {stopped:?}");
    wait_until(&fixture, |value| value["running"] == false);
    fixture.stop(&mut agent);
}

#[cfg(target_os = "linux")]
#[test]
fn production_router_provisions_bedrock_before_create_completes() {
    let downloads = BedrockDownloadServer::new();
    let fixture = TestFixture::new();
    let mut agent = fixture.spawn_agent_with_manifest(Some(&downloads.manifest_url));
    wait_for_health(fixture.port);

    let created = fixture.post(
        "/v1/servers/create",
        r#"{"name":"Provisioned","serverType":"bedrock","bedrockVersion":"1.21.80.3","port":19134}"#,
    );
    assert_eq!(created.0, 200, "create failed: {created:?}");
    let operation_id = created.1["operationId"]
        .as_str()
        .expect("create response carries operationId");
    let operation = fixture.wait_for_operation(operation_id);
    assert_eq!(
        operation["state"], "succeeded",
        "create operation: {operation}"
    );
    assert_eq!(downloads.archive_requests.load(Ordering::Relaxed), 1);

    let server_dir = fixture.servers_root.join("bedrock").join("provisioned");
    assert!(server_dir.join("bedrock_server").is_file());
    let properties = fs::read_to_string(server_dir.join("server.properties")).unwrap();
    assert!(properties.contains("server-port=19134"));
    assert!(server_dir.join("worlds").is_dir());
    let provenance: Value =
        serde_json::from_slice(&fs::read(server_dir.join(".msc_bds_provenance.json")).unwrap())
            .unwrap();
    assert_eq!(provenance["version"], "1.21.80.3");
    assert_eq!(provenance["platform"], "linux");

    let capabilities = fixture.get("/v1/capabilities");
    assert_eq!(
        capabilities["serverTypes"]["bedrock"]["runtime"]["state"],
        "available"
    );
    fixture.stop(&mut agent);
    downloads.stop();
}

#[cfg(not(target_os = "linux"))]
#[test]
fn production_router_reports_unavailable_bedrock_lifecycle() {
    let fixture = TestFixture::unavailable();
    let mut agent = fixture.spawn_agent();
    wait_for_health(fixture.port);

    let capabilities = fixture.get("/v1/capabilities");
    assert_eq!(capabilities["serverTypes"]["bedrock"]["supported"], false);
    let response = fixture.post("/v1/start", "{}");
    assert_eq!(response.0, 409, "start should be unavailable: {response:?}");
    assert_eq!(response.1["code"], "capability_unavailable");
    assert_eq!(response.1["details"]["serverType"], "bedrock");
    assert_websocket_upgrade(fixture.port, "/v1/console/stream");
    fixture.stop(&mut agent);
}

struct TestFixture {
    root: PathBuf,
    data_dir: PathBuf,
    config_path: PathBuf,
    servers_root: PathBuf,
    port: u16,
    bedrock_port: u16,
    keychain_service: String,
}

impl TestFixture {
    #[cfg(not(target_os = "linux"))]
    fn unavailable() -> Self {
        let fixture = Self::new();
        let server_dir = fixture.servers_root.join("bedrock").join("missing");
        fs::create_dir_all(&server_dir).unwrap();
        fixture.write_config(server(&server_dir, fixture.bedrock_port as i64, false));
        fixture
    }

    #[cfg(target_os = "linux")]
    fn available_linux() -> Self {
        use std::os::unix::fs::PermissionsExt;

        let fixture = Self::new();
        let server_dir = fixture.servers_root.join("bedrock").join("fixture");
        fs::create_dir_all(&server_dir).unwrap();
        fs::write(
            server_dir.join("bedrock_server"),
            b"#!/bin/sh\nprintf 'Server started\\n'\nwhile IFS= read -r line; do\n  [ \"$line\" = stop ] && exit 0\ndone\n",
        )
        .unwrap();
        fs::set_permissions(
            server_dir.join("bedrock_server"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        fs::write(
            server_dir.join(".msc_bds_provenance.json"),
            r#"{"version":"1.21.80.3","platform":"linux","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
        )
        .unwrap();
        fs::write(server_dir.join(".msc_bds_version"), "1.21.80.3").unwrap();
        fs::write(
            server_dir.join("server.properties"),
            format!(
                "level-name=Fixture\nserver-port={}\nserver-portv6={}\n",
                fixture.bedrock_port,
                fixture.bedrock_port.saturating_add(1)
            ),
        )
        .unwrap();
        fixture.write_config(server(&server_dir, fixture.bedrock_port as i64, true));
        fixture
    }

    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "msc2-bedrock-production-lifecycle-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        let data_dir = root.join("data");
        let servers_root = root.join("servers");
        fs::create_dir_all(&data_dir).unwrap();
        fs::create_dir_all(&servers_root).unwrap();
        Self {
            config_path: data_dir.join("server_config_swift.json"),
            keychain_service: format!(
                "com.msc2.bedrock-production-lifecycle.{}.{}",
                std::process::id(),
                unique_suffix()
            ),
            port: free_port(),
            bedrock_port: free_udp_port(),
            root,
            data_dir,
            servers_root,
        }
    }

    fn write_config(&self, server: ConfigServer) {
        let mut config = AppConfig::default_config(self.servers_root.to_string_lossy());
        config.servers.push(server);
        config.active_server_id = config.servers.first().map(|server| server.id.clone());
        fs::write(
            &self.config_path,
            serde_json::to_vec_pretty(&config.encode()).unwrap(),
        )
        .unwrap();
    }

    fn spawn_agent(&self) -> Child {
        self.spawn_agent_with_manifest(None)
    }

    fn spawn_agent_with_manifest(&self, manifest_url: Option<&str>) -> Child {
        let mut command = Command::new(env!("CARGO_BIN_EXE_msc"));
        command
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
            .env("MSC2_TEST_BOOTSTRAP_TOKEN", TOKEN)
            .env("MSC2_MACOS_USER_KEYCHAIN_SERVICE", &self.keychain_service);
        if let Some(manifest_url) = manifest_url {
            command.env("MSC2_BEDROCK_MANIFEST_URL", manifest_url);
        }
        command
            // Preserve production-agent diagnostics for platform startup
            // failures; otherwise the health timeout hides the root cause.
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap()
    }

    fn get(&self, path: &str) -> Value {
        response_json(http_request(self.port, "GET", path, None)).1
    }

    fn post(&self, path: &str, body: &str) -> (u16, Value) {
        response_json(http_request(self.port, "POST", path, Some(body)))
    }

    #[cfg(target_os = "linux")]
    fn wait_for_operation(&self, operation_id: &str) -> Value {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let operation = self.get(&format!("/v1/operations/{operation_id}"));
            match operation["state"].as_str() {
                Some("queued") | Some("running") => {}
                _ => return operation,
            }
            assert!(
                Instant::now() < deadline,
                "operation did not finish: {operation}"
            );
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn stop(&self, agent: &mut Child) {
        let _ = agent.kill();
        let _ = agent.wait();
        let _ = fs::remove_dir_all(&self.root);
        #[cfg(target_os = "macos")]
        let _ = Command::new("security")
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

#[cfg(target_os = "linux")]
struct BedrockDownloadServer {
    manifest_url: String,
    stop_requested: Arc<AtomicBool>,
    archive_requests: Arc<AtomicUsize>,
    thread: Option<thread::JoinHandle<()>>,
}

#[cfg(target_os = "linux")]
impl BedrockDownloadServer {
    fn new() -> Self {
        let archive = bedrock_archive();
        let archive_checksum = sha256_hex(&archive);
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let archive_url = format!("http://{address}/bedrock-server-1.21.80.3.zip");
        let manifest = serde_json::to_vec(&serde_json::json!({
            "release": {
                "1.21.80": {
                    "linux": {
                        "url": archive_url,
                        "sha256": archive_checksum
                    }
                }
            }
        }))
        .unwrap();
        let stop_requested = Arc::new(AtomicBool::new(false));
        let archive_requests = Arc::new(AtomicUsize::new(0));
        let stop_for_thread = Arc::clone(&stop_requested);
        let requests_for_thread = Arc::clone(&archive_requests);
        let thread = thread::spawn(move || {
            while !stop_for_thread.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut request = [0_u8; 2048];
                        let bytes_read = stream.read(&mut request).unwrap_or(0);
                        let request = String::from_utf8_lossy(&request[..bytes_read]);
                        let path = request.split_whitespace().nth(1).unwrap_or("/");
                        let body = if path.ends_with(".zip") {
                            requests_for_thread.fetch_add(1, Ordering::Relaxed);
                            &archive
                        } else {
                            &manifest
                        };
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let _ = stream.write_all(header.as_bytes());
                        let _ = stream.write_all(body);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            manifest_url: format!("http://{address}/manifest.json"),
            stop_requested,
            archive_requests,
            thread: Some(thread),
        }
    }

    fn stop(mut self) {
        self.stop_requested.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

#[cfg(target_os = "linux")]
fn bedrock_archive() -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut bytes);
        zip.start_file(
            "bedrock_server",
            SimpleFileOptions::default().unix_permissions(0o755),
        )
        .unwrap();
        zip.write_all(b"#!/bin/sh\n").unwrap();
        zip.start_file("server.properties", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"archive-settings\n").unwrap();
        zip.finish().unwrap();
    }
    bytes.into_inner()
}

fn server(directory: &Path, port: i64, fixture: bool) -> ConfigServer {
    let mut server = ConfigServer::new(
        "bedrock-production",
        "Production Bedrock",
        directory.to_string_lossy(),
        "",
        1.0,
        1.0,
    );
    server.server_type = ServerType::Bedrock;
    server.bedrock_enabled = true;
    server.bedrock_port = Some(port);
    server.bedrock_version = Some("1.21.80.3".to_owned());
    server.has_shown_first_start_popup = fixture;
    server
}

fn wait_for_health(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))).is_ok()
            && http_request(port, "GET", "/v1/health", None).starts_with("HTTP/1.1 200")
        {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("agent did not become healthy");
}

#[cfg(target_os = "linux")]
fn wait_until(fixture: &TestFixture, predicate: impl Fn(&Value) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        let status = fixture.get("/v1/status");
        if predicate(&status) {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("agent status did not reach the expected state");
}

fn assert_websocket_upgrade(port: u16, path: &str) {
    let mut stream = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))).unwrap();
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {TOKEN}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut buffer = [0_u8; 1024];
    let bytes_read = stream.read(&mut buffer).unwrap();
    let response = String::from_utf8_lossy(&buffer[..bytes_read]);
    assert!(
        response.starts_with("HTTP/1.1 101"),
        "WebSocket route did not upgrade: {}",
        response.lines().next().unwrap_or_default()
    );
}

fn http_request(port: u16, method: &str, path: &str, body: Option<&str>) -> String {
    http_request_with_headers(port, method, path, body, &[])
}

fn http_request_with_headers(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&str>,
    headers: &[(&str, &str)],
) -> String {
    let mut stream = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))).unwrap();
    let body = body.unwrap_or_default();
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {TOKEN}\r\n"
    );
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str(&format!(
        "Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ));
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn response_json(response: String) -> (u16, Value) {
    let status = response.split_whitespace().nth(1).unwrap().parse().unwrap();
    let body = response.split("\r\n\r\n").nth(1).unwrap_or_default();
    (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn free_udp_port() -> u16 {
    std::net::UdpSocket::bind(("127.0.0.1", 0))
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
