//! P10.31's production-composition proof for Bedrock capability disclosure.
//!
//! The test starts the real `msc serve` binary with an empty fleet. That
//! proves `/v1/capabilities` is reading the runtime selected by `main.rs`,
//! including the honest provisioning-required or Apple-Silicon-unavailable
//! result, rather than a route-local constant.

#![cfg(target_os = "macos")]

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

const TOKEN: &str = "msc2_bedrock_runtime_selection_testsecret";

#[test]
fn production_agent_reports_the_selected_bedrock_runtime() {
    let temp = std::env::temp_dir().join(format!(
        "msc2-bedrock-runtime-selection-{}-{}",
        std::process::id(),
        suffix()
    ));
    let data_dir = temp.join("data");
    let servers_root = temp.join("servers");
    let config_path = data_dir.join("server_config_swift.json");
    fs::create_dir_all(&data_dir).unwrap();
    fs::create_dir_all(&servers_root).unwrap();
    fs::write(
        &config_path,
        format!(
            r#"{{
  "config_version": 1,
  "servers_root": "{}",
  "servers": []
}}"#,
            json_path(&servers_root)
        ),
    )
    .unwrap();

    let port = free_port();
    let keychain_service = format!(
        "com.msc2.bedrock-runtime-selection.{}.{}",
        std::process::id(),
        suffix()
    );
    let mut agent = spawn_agent(
        port,
        &data_dir,
        &config_path,
        &servers_root,
        &keychain_service,
    );
    wait_for_health(port);

    let response = http_get(port, "/v1/capabilities", Some(TOKEN));
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "capabilities expected 200, got: {}",
        response.lines().next().unwrap_or_default()
    );
    let body = response.split("\r\n\r\n").nth(1).unwrap_or_default();
    let capabilities: Value = serde_json::from_str(body).unwrap();
    let bedrock = &capabilities["serverTypes"]["bedrock"];
    let runtime = &bedrock["runtime"];
    assert_eq!(bedrock["supported"], false);

    #[cfg(target_arch = "x86_64")]
    {
        assert_eq!(bedrock["backend"], "vz-sidecar");
        assert_eq!(runtime["state"], "provisioning_required");
        assert_eq!(runtime["backend"], "vz-sidecar");
        assert_eq!(runtime["hostOs"], "macos");
    }

    #[cfg(target_arch = "aarch64")]
    {
        assert!(bedrock["backend"].is_null());
        assert_eq!(runtime["state"], "unavailable");
        assert!(runtime["backend"].is_null());
        assert_eq!(runtime["reasonCode"], "no_test_hardware");
        assert_eq!(runtime["hostOs"], "macos");
    }

    stop_child(&mut agent);
    cleanup_secret(&keychain_service);
}

fn spawn_agent(
    port: u16,
    data_dir: &Path,
    config_path: &Path,
    servers_root: &Path,
    keychain_service: &str,
) -> Child {
    Command::new(env!("CARGO_BIN_EXE_msc"))
        .args(["serve", "--bind", &format!("127.0.0.1:{port}")])
        .env("MSC2_DATA_DIR", data_dir)
        .env("MSC2_APP_CONFIG_PATH", config_path)
        .env("MSC2_AGENT_SERVERS_ROOT", servers_root)
        .env(
            "MSC2_CREDENTIAL_REGISTRY_PATH",
            data_dir.join("credential-registry.json"),
        )
        .env("MSC2_MACOS_USER_KEYCHAIN_SERVICE", keychain_service)
        .env("MSC2_OPERATION_JOURNAL_DIR", data_dir.join("journal"))
        .env("MSC2_TEST_BOOTSTRAP_TOKEN", TOKEN)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn wait_for_health(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if http_get(port, "/v1/health", None).starts_with("HTTP/1.1 200") {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("agent did not become healthy");
}

fn http_get(port: u16, path: &str, bearer: Option<&str>) -> String {
    let Ok(mut stream) = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))) else {
        return String::new();
    };
    let auth = bearer
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth}Connection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn cleanup_secret(service: &str) {
    let _ = Command::new("security")
        .args([
            "delete-generic-password",
            "-s",
            service,
            "-a",
            "remote-api.owner-token",
        ])
        .output();
}

fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn json_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

fn suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
