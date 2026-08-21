//! Black-box smoke test for P8.24's add-on, modpack, and staged-upload
//! routes in `build_app()` (`main.rs`): a real `msc serve` process must
//! mount them behind the bearer-auth gate, and their typed route logic
//! must answer on an otherwise-empty fleet.

#![cfg(target_os = "macos")]

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const TOKEN: &str = "msc2_replacementrecovery_recoverysecret";

#[test]
fn phase8_routes_are_mounted_behind_bearer_auth() {
    let temp = temp_dir("phase8-routes-mounted");
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
            json_path(&servers_root),
        ),
    )
    .unwrap();

    let port = free_port();
    let bind = format!("127.0.0.1:{port}");
    let keychain_service = format!(
        "com.msc2.phase8-routes-mounted.{}.{}",
        std::process::id(),
        suffix()
    );
    let log_path = temp.join("agent.log");

    let mut agent = spawn_agent(
        &bind,
        &data_dir,
        &config_path,
        &servers_root,
        &keychain_service,
        &log_path,
    );
    wait_for_health(port);

    for path in [
        "/v1/addons",
        "/v1/catalog/search",
        "/v1/components",
        "/v1/components/client-export",
    ] {
        let response = http_get(port, path, None);
        assert!(
            response.starts_with("HTTP/1.1 401") || response.starts_with("HTTP/1.1 429"),
            "{path} GET expected 401/429 (mounted + auth-gated), got: {}",
            response.lines().next().unwrap_or_default()
        );
    }

    for path in [
        "/v1/staged-uploads",
        "/v1/components/install",
        "/v1/components/remove",
        "/v1/components/update",
        "/v1/modpacks/inspect",
        "/v1/modpacks/import",
        "/v1/modpacks/op-1/manual-file",
    ] {
        let response = http_post(port, path, None);
        assert!(
            response.starts_with("HTTP/1.1 401") || response.starts_with("HTTP/1.1 429"),
            "{path} POST expected 401/429 (mounted + auth-gated), got: {}",
            response.lines().next().unwrap_or_default()
        );
    }

    let response = http_get(port, "/v1/components/does-not-exist-route", None);
    assert!(
        response.starts_with("HTTP/1.1 404"),
        "expected a genuinely unmounted path to 404, got: {}",
        response.lines().next().unwrap_or_default()
    );

    stop_child(&mut agent);
    cleanup_secret(&keychain_service, "remote-api.owner-token");
}

#[test]
fn phase8_routes_accept_the_bootstrap_token() {
    let temp = temp_dir("phase8-routes-authed");
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
            json_path(&servers_root),
        ),
    )
    .unwrap();

    let port = free_port();
    let bind = format!("127.0.0.1:{port}");
    let keychain_service = format!(
        "com.msc2.phase8-routes-authed.{}.{}",
        std::process::id(),
        suffix()
    );
    let log_path = temp.join("agent.log");

    let mut agent = spawn_agent(
        &bind,
        &data_dir,
        &config_path,
        &servers_root,
        &keychain_service,
        &log_path,
    );
    wait_for_health(port);

    let response = http_get(port, "/v1/addons", Some(TOKEN));
    assert!(response.starts_with("HTTP/1.1 200"), "{}", response);
    assert!(response.contains("\"serverSupportsAddons\":false"));

    let response = http_get(port, "/v1/catalog/search?q=fabric&offset=0", Some(TOKEN));
    assert!(response.starts_with("HTTP/1.1 200"), "{}", response);
    assert!(response.contains("\"supportsAddons\":false"));

    let response = http_post_json(
        port,
        "/v1/staged-uploads",
        Some(TOKEN),
        r#"{"purpose":"modpack-archive"}"#,
    );
    assert!(response.starts_with("HTTP/1.1 200"), "{}", response);
    assert!(response.contains("\"stagedUploadId\""));

    let response = http_post_json(
        port,
        "/v1/components/install",
        Some(TOKEN),
        r#"{"projectId":"AABBCC"}"#,
    );
    assert!(response.starts_with("HTTP/1.1 409"), "{}", response);
    assert!(response.contains("no_active_server"));

    let response = http_post_json(
        port,
        "/v1/modpacks/inspect",
        Some(TOKEN),
        r#"{"stagedUploadId":"missing"}"#,
    );
    assert!(response.starts_with("HTTP/1.1 404"), "{}", response);
    assert!(response.contains("not_found"));

    stop_child(&mut agent);
    cleanup_secret(&keychain_service, "remote-api.owner-token");
}

fn spawn_agent(
    bind: &str,
    data_dir: &Path,
    config_path: &Path,
    servers_root: &Path,
    keychain_service: &str,
    log_path: &Path,
) -> Child {
    let log = fs::File::create(log_path).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_msc"));
    command
        .arg("serve")
        .arg("--bind")
        .arg(bind)
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
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log));

    command.spawn().unwrap()
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

fn http_post(port: u16, path: &str, bearer: Option<&str>) -> String {
    http_post_json(port, path, bearer, "{}")
}

fn http_post_json(port: u16, path: &str, bearer: Option<&str>, body: &str) -> String {
    let Ok(mut stream) = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))) else {
        return String::new();
    };
    let auth = bearer
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
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

fn free_port() -> u16 {
    TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn temp_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "msc2-{label}-{}-{}",
        std::process::id(),
        suffix()
    ));
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
    path.display().to_string().replace('\\', "\\\\")
}

fn cleanup_secret(service: &str, account: &str) {
    let _ = Command::new("security")
        .arg("delete-generic-password")
        .arg("-s")
        .arg(service)
        .arg("-a")
        .arg(account)
        .status();
}
