//! P10.33's production-router proof for the shared Bedrock surfaces.
//!
//! The fixture deliberately has no verified BDS installation.  The test
//! proves the useful split in the public contract: settings, player data,
//! allowlist, versions, metrics, worlds, and backup inventory remain readable
//! from disk, while operations that need a live runtime return the same
//! structured capability error.

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use msc_domain::app_config_schema::{AppConfig, ConfigServer};
use msc_domain::identity::ServerType;
use serde_json::Value;

const TOKEN: &str = "msc2_bedrock-production-surfaces_testsecret";

#[test]
fn production_router_exposes_shared_bedrock_surfaces_and_runtime_errors() {
    let fixture = TestFixture::new();
    fixture.seed_server();
    let mut agent = fixture.spawn_agent();
    fixture.wait_for_health();

    for path in [
        "/v1/settings",
        "/v1/versions",
        "/v1/players",
        "/v1/allowlist",
        "/v1/performance",
        "/v1/status",
        "/v1/worlds",
        "/v1/backups",
    ] {
        let (status, body) = fixture.request("GET", path, None);
        assert_eq!(status, 200, "{path}: {body}");
        if path != "/v1/worlds" && path != "/v1/players" {
            assert_ne!(body["runtime"], Value::Null, "{path}: {body}");
            assert_ne!(body["runtime"]["state"], Value::Null, "{path}: {body}");
        }
    }

    let (status, settings) = fixture.request(
        "POST",
        "/v1/settings",
        Some(r#"{"changes":{"max-players":"32"}}"#),
    );
    assert_eq!(status, 200, "settings update: {settings}");
    assert_eq!(settings["success"], true);
    assert_eq!(settings["runtime"]["state"], fixture.runtime_state());

    let (status, allowlist) = fixture.request(
        "POST",
        "/v1/allowlist",
        Some(r#"{"action":"add","name":"Casey"}"#),
    );
    assert_eq!(status, 200, "allowlist update: {allowlist}");
    assert_eq!(allowlist["runtime"]["state"], fixture.runtime_state());
    assert!(fixture.server_dir.join("allowlist.json").is_file());

    for (path, body) in [
        ("/v1/start", "{}"),
        ("/v1/command", r#"{"command":"say hello"}"#),
        ("/v1/backups/now", "{}"),
        ("/v1/components/version", r#"{"versionId":"1.21.80.3"}"#),
        ("/v1/worlds/repair", r#"{"slotId":"missing"}"#),
    ] {
        let (status, error) = fixture.request("POST", path, Some(body));
        assert_eq!(status, 409, "{path}: {error}");
        assert_eq!(error["code"], "capability_unavailable", "{path}: {error}");
        assert_eq!(error["details"]["capability"], "bedrock-runtime");
        assert_eq!(error["details"]["serverType"], "bedrock");
    }

    fixture.stop(&mut agent);
}

struct TestFixture {
    root: PathBuf,
    data_dir: PathBuf,
    config_path: PathBuf,
    servers_root: PathBuf,
    server_dir: PathBuf,
    port: u16,
    bedrock_port: u16,
    keychain_service: String,
}

impl TestFixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "msc2-bedrock-production-surfaces-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        let data_dir = root.join("data");
        let servers_root = root.join("servers");
        let server_dir = servers_root.join("bedrock").join("surface-fixture");
        fs::create_dir_all(&data_dir).unwrap();
        fs::create_dir_all(server_dir.join("worlds/Realm/db")).unwrap();
        Self {
            config_path: data_dir.join("server_config_swift.json"),
            keychain_service: format!(
                "com.msc2.bedrock-production-surfaces.{}.{}",
                std::process::id(),
                unique_suffix()
            ),
            port: free_port(),
            bedrock_port: free_udp_port(),
            root,
            data_dir,
            servers_root,
            server_dir,
        }
    }

    fn seed_server(&self) {
        fs::write(
            self.server_dir.join("server.properties"),
            format!(
                "level-name=Realm\ndifficulty=normal\nserver-port={}\n",
                self.bedrock_port
            ),
        )
        .unwrap();
        fs::write(
            self.server_dir.join("allowlist.json"),
            r#"[{"name":"Alex","xuid":"123","ignoresPlayerLimit":false}]"#,
        )
        .unwrap();
        fs::write(self.server_dir.join("permissions.json"), "[]").unwrap();
        let mut config = AppConfig::default_config(self.servers_root.to_string_lossy());
        let mut server = ConfigServer::new(
            "bedrock-surfaces",
            "Bedrock surfaces",
            self.server_dir.to_string_lossy(),
            "",
            1.0,
            2.0,
        );
        server.server_type = ServerType::Bedrock;
        server.bedrock_enabled = true;
        server.bedrock_port = Some(i64::from(self.bedrock_port));
        server.bedrock_version = Some("1.21.80.3".to_owned());
        config.servers.push(server);
        config.active_server_id = config.servers.first().map(|server| server.id.clone());
        fs::write(
            &self.config_path,
            serde_json::to_vec_pretty(&config.encode()).unwrap(),
        )
        .unwrap();
    }

    fn spawn_agent(&self) -> Child {
        Command::new(env!("CARGO_BIN_EXE_msc"))
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
            .env("MSC2_MACOS_USER_KEYCHAIN_SERVICE", &self.keychain_service)
            // Keep this fixture focused on the unavailable-runtime contract;
            // a hosted runner must not turn it into a real download test.
            .env(
                "MSC2_BEDROCK_MANIFEST_URL",
                "http://127.0.0.1:1/msc2-test-manifest.json",
            )
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }

    fn wait_for_health(&self) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], self.port))).is_ok()
                && http_request(self.port, "GET", "/v1/health", None).starts_with("HTTP/1.1 200")
            {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
        panic!("agent did not become healthy");
    }

    fn request(&self, method: &str, path: &str, body: Option<&str>) -> (u16, Value) {
        response_json(http_request(self.port, method, path, body))
    }

    fn runtime_state(&self) -> Value {
        self.request("GET", "/v1/capabilities", None).1["serverTypes"]["bedrock"]["runtime"][
            "state"
        ]
        .clone()
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

fn http_request(port: u16, method: &str, path: &str, body: Option<&str>) -> String {
    let mut stream = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))).unwrap();
    let body = body.unwrap_or_default();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {TOKEN}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
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
