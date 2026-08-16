//! Black-box smoke test for P6.21's world/backup routes in `build_app()`
//! (`main.rs`): a real `msc serve` process must come up healthy with the
//! new routes actually mounted behind the same bearer-auth gate every
//! other protected route uses.
//!
//! This crate has no `lib.rs`, so an external test file can't reach
//! `crate::routes::worlds`/`crate::routes::backups` directly — the
//! substantive route-logic coverage (CRUD, staged transfer, activation,
//! restore guard ordering, exclusivity, permission checks) lives as
//! `world_backup_routes_*`-prefixed `#[cfg(test)]` tests inline inside
//! `src/routes/worlds.rs`/`src/routes/backups.rs` instead, the same
//! "tests live inline" precedent `src/routes/settings.rs` already set —
//! see those modules' own test-module doc comments. This file only
//! proves the real wiring in `main.rs` actually mounts the new routes:
//! `axum::Router::route_layer` (the bearer-auth gate every protected
//! route in this agent sits behind) only runs for a path that matches a
//! real route — an unmounted path falls straight through to axum's
//! default 404 *without* the auth middleware running at all. So a
//! `401 unauthorized` (not `404`) on an unauthenticated `GET /v1/worlds`/
//! `GET /v1/backups` is itself proof these routes are really mounted,
//! the same `CARGO_BIN_EXE_msc`-driven pattern
//! `tests/backup_scheduler.rs`/`tests/startup_secret_migration.rs`
//! already established. macOS-only for the same reason those files are:
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
use std::time::{Duration, Instant};

#[test]
fn world_backup_routes_are_mounted_behind_bearer_auth() {
    let temp = temp_dir("world-backup-routes-mounted");
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
        "com.msc2.world-backup-routes-mounted.{}.{}",
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

    // Mounted-but-unauthenticated: 401, not 404, on every new world/
    // backup GET route.
    for path in ["/v1/worlds", "/v1/backups", "/v1/backups/config"] {
        let response = http_get(port, path, None);
        assert!(
            response.starts_with("HTTP/1.1 401"),
            "{path} expected 401 (mounted + auth-gated), got: {}",
            response.lines().next().unwrap_or_default()
        );
    }

    // An actually-unmounted path still 404s, proving the 401s above
    // aren't just this agent's blanket behavior for any unknown path.
    let response = http_get(port, "/v1/worlds/does-not-exist-route", None);
    assert!(
        response.starts_with("HTTP/1.1 404"),
        "expected a genuinely unmounted path to 404, got: {}",
        response.lines().next().unwrap_or_default()
    );

    // P6.34: the new direct-live-world-replacement route is a POST, not
    // a GET -- same mounted-but-auth-gated proof via a POST helper.
    let response = http_post(port, "/v1/worlds/replace-active-world", None);
    assert!(
        response.starts_with("HTTP/1.1 401"),
        "/v1/worlds/replace-active-world expected 401 (mounted + auth-gated), got: {}",
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

fn http_post(port: u16, path: &str, bearer: Option<&str>) -> String {
    let Ok(mut stream) = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))) else {
        return String::new();
    };
    let auth = bearer
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let body = "{}";
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

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("msc2-{tag}-{}-{}", std::process::id(), suffix()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn json_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn cleanup_secret(keychain_service: &str, account: &str) {
    let _ = Command::new("security")
        .args([
            "delete-generic-password",
            "-s",
            keychain_service,
            "-a",
            account,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
