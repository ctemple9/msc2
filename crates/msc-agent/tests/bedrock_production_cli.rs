//! P10.34's same-binary CLI proof for the production Bedrock router.
//!
//! The test starts the real agent composition root and then invokes the same
//! `msc` binary as a client.  The fixture is a disposable BDS-shaped server,
//! so the test exercises serialization, authentication, runtime disclosure,
//! and CLI decoding without downloading BDS or starting a VM.

use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use msc_domain::app_config_schema::{AppConfig, ConfigServer};
use msc_domain::identity::ServerType;

const TOKEN: &str = "msc2_bedrock-production-cli_testsecret";

#[test]
fn production_cli_decodes_bedrock_surfaces_and_unavailable_runtime() {
    let fixture = Fixture::new();
    fixture.seed_server(&fixture.server_dir);
    fixture.seed_server(&fixture.import_source);
    fixture.write_config();

    let mut agent = fixture.spawn_agent();
    fixture.wait_for_health();

    let capabilities = fixture.cli_json(&["capabilities"]);
    if cfg!(target_os = "linux") {
        assert_eq!(
            capabilities["serverTypes"]["bedrock"]["runtime"]["state"],
            "available"
        );
    } else {
        assert_ne!(
            capabilities["serverTypes"]["bedrock"]["runtime"]["state"],
            "available"
        );
    }

    let create = fixture.cli_json(&[
        "server",
        "create",
        "CLI Created Bedrock",
        "--type",
        "bedrock",
        "--version-id",
        "1.21.80.3",
        "--no-wait",
    ]);
    let create_operation = create["operationId"].as_str().expect("create operation id");
    let created = fixture.wait_for_operation(create_operation);
    assert_eq!(created["state"], "succeeded", "create operation: {created}");

    let imported = fixture.cli(&[
        "server",
        "import",
        fixture.import_source.to_str().unwrap(),
        "--name",
        "CLI Imported Bedrock",
        "--type",
        "bedrock",
    ]);
    assert!(imported.status.success(), "{}", output_text(&imported));
    let imported_operation: Value = serde_json::from_slice(&imported.stdout)
        .expect("CLI import emits the production operation serializer");
    assert_eq!(imported_operation["state"], "succeeded");

    let settings = fixture.cli_json(&["settings", "get"]);
    assert_eq!(settings["serverType"], "bedrock");
    assert!(settings["runtime"].is_object());

    let players = fixture.cli_json(&["bedrock", "players"]);
    assert_eq!(players["count"], 0);
    assert!(players["runtime"].is_object());

    let allowlist = fixture.cli_json(&["bedrock", "allowlist", "get"]);
    assert_eq!(allowlist["serverType"], "bedrock");
    assert_eq!(allowlist["entries"][0]["name"], "Alex");

    let versions = fixture.cli_json(&["version", "list"]);
    assert_eq!(versions["isBedrock"], true);
    assert!(versions["runtime"].is_object());

    let start = fixture.cli(&["server", "start"]);
    if cfg!(target_os = "linux") {
        assert!(start.status.success(), "{}", output_text(&start));
        let started: Value = serde_json::from_slice(&start.stdout).expect("start JSON");
        assert_eq!(started["result"], "start_requested");
        assert!(started["operationId"].is_string());
        assert_eq!(started["runtime"]["state"], "available");

        let stop = fixture.cli(&["server", "stop"]);
        assert!(stop.status.success(), "{}", output_text(&stop));
        let stopped: Value = serde_json::from_slice(&stop.stdout).expect("stop JSON");
        assert_eq!(stopped["result"], "stop_requested");
        assert_eq!(stopped["runtime"]["state"], "available");
    } else {
        assert!(
            !start.status.success(),
            "unavailable start unexpectedly succeeded"
        );
        let error: Value = serde_json::from_str(String::from_utf8_lossy(&start.stderr).trim())
            .expect("unavailable CLI error JSON");
        assert_eq!(error["code"], "capability_unavailable");
        assert_eq!(error["details"]["serverType"], "bedrock");
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        assert_eq!(
            capabilities["serverTypes"]["bedrock"]["runtime"]["state"],
            "unavailable"
        );
        assert_eq!(
            capabilities["serverTypes"]["bedrock"]["runtime"]["reasonCode"],
            "no_test_hardware"
        );
    }

    fixture.stop(&mut agent);
}

struct Fixture {
    root: PathBuf,
    data_dir: PathBuf,
    config_path: PathBuf,
    servers_root: PathBuf,
    server_dir: PathBuf,
    import_source: PathBuf,
    port: u16,
    keychain_service: String,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "msc2-bedrock-production-cli-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        let data_dir = root.join("data");
        let servers_root = root.join("servers");
        let server_dir = servers_root.join("bedrock").join("cli-fixture");
        let import_source = root.join("import-source");
        fs::create_dir_all(&data_dir).unwrap();
        fs::create_dir_all(&server_dir).unwrap();
        fs::create_dir_all(&import_source).unwrap();
        Self {
            config_path: data_dir.join("server_config_swift.json"),
            keychain_service: format!(
                "com.msc2.bedrock-production-cli.{}.{}",
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

    fn seed_server(&self, directory: &Path) {
        fs::create_dir_all(directory.join("worlds/Realm/db")).unwrap();
        fs::write(
            directory.join("server.properties"),
            "level-name=Realm\ndifficulty=normal\nserver-port=19132\nmax-players=10\n",
        )
        .unwrap();
        fs::write(
            directory.join("allowlist.json"),
            r#"[{"name":"Alex","xuid":"123","ignoresPlayerLimit":false}]"#,
        )
        .unwrap();
        fs::write(directory.join("permissions.json"), "[]").unwrap();
        fs::write(
            directory.join(".msc_bds_provenance.json"),
            r#"{"version":"1.21.80.3","platform":"linux","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
        )
        .unwrap();
        fs::write(
            directory.join("bedrock_server"),
            b"#!/bin/sh\nprintf 'Server started\\n'\nwhile IFS= read -r line; do\n  [ \"$line\" = stop ] && exit 0\ndone\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                directory.join("bedrock_server"),
                fs::Permissions::from_mode(0o755),
            )
            .unwrap();
        }
    }

    fn write_config(&self) {
        let mut config = AppConfig::default_config(self.servers_root.to_string_lossy());
        let mut server = ConfigServer::new(
            "bedrock-cli",
            "CLI Bedrock",
            self.server_dir.to_string_lossy(),
            "",
            1.0,
            2.0,
        );
        server.server_type = ServerType::Bedrock;
        server.bedrock_enabled = true;
        server.bedrock_port = Some(19132);
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
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }

    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_msc"))
            .args([
                "--base-url",
                &format!("http://127.0.0.1:{}", self.port),
                "--token",
                TOKEN,
                "--json",
            ])
            .args(args)
            .output()
            .unwrap()
    }

    fn cli_json(&self, args: &[&str]) -> Value {
        let output = self.cli(args);
        assert!(output.status.success(), "{}", output_text(&output));
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!("CLI output was not JSON: {error}: {}", output_text(&output))
        })
    }

    fn wait_for_operation(&self, operation_id: &str) -> Value {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let operation = self.get_json(&format!("/v1/operations/{operation_id}"));
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

    fn get_json(&self, path: &str) -> Value {
        let response = http_request(self.port, "GET", path, None);
        let body = response.split("\r\n\r\n").nth(1).unwrap_or_default();
        serde_json::from_str(body).unwrap()
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

fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
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

fn output_text(output: &Output) -> String {
    format!(
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
