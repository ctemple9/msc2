#![cfg(target_os = "macos")]

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const TOKEN: &str = "msc2_host_reset_routes_resetsecret";

#[test]
fn host_reset_is_mounted_and_performs_full_reset_without_uninstalling_service() {
    let temp = temp_dir("host-reset-routes");
    let data_dir = temp.join("data");
    let servers_root = temp.join("servers");
    let config_path = data_dir.join("server_config_swift.json");
    fs::create_dir_all(&data_dir).unwrap();
    fs::create_dir_all(servers_root.join("paper")).unwrap();
    fs::write(servers_root.join("paper/world.dat"), b"world").unwrap();
    fs::write(
        &config_path,
        format!(
            "{{\"config_version\":1,\"servers_root\":\"{}\",\"servers\":[]}}",
            json_path(&servers_root)
        ),
    )
    .unwrap();

    let port = free_port();
    let service = format!(
        "com.msc2.host-reset-routes.{}.{}",
        std::process::id(),
        suffix()
    );
    let mut agent = spawn_agent(port, &data_dir, &config_path, &servers_root, &service);
    wait_for_health(port);

    let response = http_post(port, "/v1/host/reset", None, "{}");
    assert!(response.starts_with("HTTP/1.1 401"), "{response}");

    let pairing = Command::new(env!("CARGO_BIN_EXE_msc"))
        .args(["pairing", "create", "--client-kind", "desktop", "--json"])
        .env("MSC2_DATA_DIR", &data_dir)
        .env(
            "MSC2_CREDENTIAL_REGISTRY_PATH",
            data_dir.join("credentials.json"),
        )
        .env("MSC2_MACOS_USER_KEYCHAIN_SERVICE", &service)
        .output()
        .unwrap();
    assert!(
        pairing.status.success(),
        "{}",
        String::from_utf8_lossy(&pairing.stderr)
    );
    let pairing_json: serde_json::Value = serde_json::from_slice(&pairing.stdout).unwrap();
    let host_id = pairing_json["agentHostId"].as_str().unwrap();
    let reset = http_post(
        port,
        "/v1/host/reset",
        Some(TOKEN),
        &format!(r#"{{"mode":"everything","confirmation":"RESET {host_id}"}}"#),
    );
    assert!(reset.starts_with("HTTP/1.1 202"), "{reset}");
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && (config_path.exists() || servers_root.exists()) {
        thread::sleep(Duration::from_millis(50));
    }
    assert!(!config_path.exists(), "host config survived reset");
    assert!(
        !servers_root.exists(),
        "managed server tree survived full reset"
    );

    stop_child(&mut agent);
    cleanup_secret(&service);
}

fn spawn_agent(
    port: u16,
    data_dir: &Path,
    config_path: &Path,
    servers_root: &Path,
    service: &str,
) -> Child {
    let log_path = data_dir.join("agent.log");
    let log = fs::File::create(&log_path).unwrap();
    Command::new(env!("CARGO_BIN_EXE_msc"))
        .arg("serve")
        .arg("--bind")
        .arg(format!("127.0.0.1:{port}"))
        .env("MSC2_DATA_DIR", data_dir)
        .env("MSC2_APP_CONFIG_PATH", config_path)
        .env("MSC2_AGENT_SERVERS_ROOT", servers_root)
        .env(
            "MSC2_CREDENTIAL_REGISTRY_PATH",
            data_dir.join("credentials.json"),
        )
        .env("MSC2_MACOS_USER_KEYCHAIN_SERVICE", service)
        .env("MSC2_OPERATION_JOURNAL_DIR", data_dir.join("journal"))
        .env("MSC2_TEST_BOOTSTRAP_TOKEN", TOKEN)
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
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

fn http_get(port: u16, path: &str, token: Option<&str>) -> String {
    request(port, "GET", path, token, "")
}

fn http_post(port: u16, path: &str, token: Option<&str>, json: &str) -> String {
    request(port, "POST", path, token, json)
}

fn request(port: u16, method: &str, path: &str, token: Option<&str>, json: &str) -> String {
    let Ok(mut stream) = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))) else {
        return String::new();
    };
    let auth = token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
        json.len()
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
        .args(["delete-generic-password", "-s", service])
        .output();
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn temp_dir(name: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("msc2-{name}-{}-{}", std::process::id(), suffix()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn json_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}
