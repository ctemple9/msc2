//! Black-box smoke test for P6.40's operation routes in `build_app()`
//! (`main.rs`): a real `msc serve` process must still expose
//! `POST /v1/operations`, `GET /v1/operations/{id}`, and
//! `POST /v1/operations/{id}/cancel` mounted behind the same bearer-auth
//! gate every other protected route uses. P6.40 makes `cancel` return
//! `202` immediately while the target worker is still stopping instead
//! of waiting inside the HTTP request or transitioning state itself. P6.44
//! additionally makes acceptance and the returned non-terminal snapshot one
//! atomic application-layer decision.
//!
//! This crate has no `lib.rs`, so an external test file can't reach
//! `OperationsState::request_cancel`/`cancellation_check` or the real
//! `demo-install` ticker directly — the substantive truthful-cancellation
//! behavior (cancel returns Accepted while pending, the target stays
//! exclusively held, and a cancel that arrives after
//! natural completion reports the true outcome instead of a fabricated
//! `cancelled`) is proven inline, against the real handlers, in
//! `operation_cancellation_*`-prefixed `#[cfg(test)]` tests inside
//! `src/routes/operations.rs` itself — the same "tests live inline"
//! precedent `src/routes/worlds.rs`/`src/routes/backups.rs` already set,
//! for the identical reason. This file only proves the real wiring:
//! `axum::Router::route_layer` (the bearer-auth gate) only runs for a
//! path that matches a real route, so a `401 unauthorized` — not `404` —
//! on every operations route below is itself proof they're really
//! mounted, the same `CARGO_BIN_EXE_msc`-driven pattern
//! `tests/world_backup_routes.rs`/`tests/backup_scheduler.rs` already
//! established. macOS-only for the same reason those files are:
//! `build_app()` unconditionally provisions a real production
//! `SecretStore`, and only macOS Keychain is available, isolated, and
//! fast enough in this environment to exercise that path in a test.

#![cfg(target_os = "macos")]

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[test]
fn operation_cancellation_routes_are_mounted_behind_bearer_auth() {
    let temp = temp_dir("operation-cancellation-mounted");
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
    let base_url = format!("127.0.0.1:{port}");
    let keychain_service = format!(
        "com.msc2.operation-cancellation-mounted.{}.{}",
        std::process::id(),
        suffix()
    );
    let log_path = temp.join("agent.log");

    let mut agent = spawn_agent(
        &base_url,
        &data_dir,
        &config_path,
        &servers_root,
        &keychain_service,
        &log_path,
    );
    wait_for_health(port);

    // Mounted-but-unauthenticated: 401, not 404.
    let response = http_get(port, "/v1/operations/does-not-exist", None);
    assert!(
        response.starts_with("HTTP/1.1 401"),
        "GET /v1/operations/:id expected 401 (mounted + auth-gated), got: {}",
        response.lines().next().unwrap_or_default()
    );

    let response = http_post(port, "/v1/operations", None, r#"{"type":"demo-install"}"#);
    assert!(
        response.starts_with("HTTP/1.1 401"),
        "POST /v1/operations expected 401 (mounted + auth-gated), got: {}",
        response.lines().next().unwrap_or_default()
    );

    // Auth runs before the cancellation handler. A slow response here
    // (rather than a prompt 401) would itself be a wiring regression.
    let started = Instant::now();
    let response = http_post(port, "/v1/operations/does-not-exist/cancel", None, "");
    assert!(
        response.starts_with("HTTP/1.1 401"),
        "POST /v1/operations/:id/cancel expected 401 (mounted + auth-gated), got: {}",
        response.lines().next().unwrap_or_default()
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "unauthenticated cancel took {:?} — should 401 immediately",
        started.elapsed()
    );

    // An actually-unmounted path still 404s, proving the 401s above
    // aren't just this agent's blanket behavior for any unknown path.
    let response = http_get(
        port,
        "/v1/operations/does-not-exist/not-a-real-suffix",
        None,
    );
    assert!(
        response.starts_with("HTTP/1.1 404"),
        "expected a genuinely unmounted path to 404, got: {}",
        response.lines().next().unwrap_or_default()
    );

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

fn http_post(port: u16, path: &str, bearer: Option<&str>, json_body: &str) -> String {
    let Ok(mut stream) = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))) else {
        return String::new();
    };
    let auth = bearer
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json_body}",
        json_body.len()
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
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

fn temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("msc2-{name}-{}", suffix()));
    fs::create_dir_all(&path).unwrap();
    path
}

fn suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn json_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

fn cleanup_secret(service: &str, account: &str) {
    let _ = Command::new("/usr/bin/security")
        .arg("delete-generic-password")
        .arg("-s")
        .arg(service)
        .arg("-a")
        .arg(account)
        .status();
}
