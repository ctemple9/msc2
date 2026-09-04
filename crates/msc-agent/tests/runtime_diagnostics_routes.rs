//! Black-box smoke test for P7.24's runtime/version/diagnostics routes in
//! `build_app()` (`main.rs`): a real `msc serve` process must come up
//! healthy with the new routes actually mounted behind the same
//! bearer-auth gate every other protected route uses, and `GET /v1/health`
//! must keep serving unauthenticated per Phase 2's own design.
//!
//! Same "external test file can't reach `crate::routes` directly, so this
//! file only proves mounting" shape as `tests/world_backup_routes.rs`'s
//! own doc comment explains. macOS-only for the same reason that file is:
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

const TOKEN: &str = "msc2_replacementrecovery_recoverysecret";

#[test]
fn runtime_diagnostics_routes_are_mounted_behind_bearer_auth() {
    let temp = temp_dir("runtime-diagnostics-routes-mounted");
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
        "com.msc2.runtime-diagnostics-routes-mounted.{}.{}",
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

    let response = http_get(port, "/v1/healthz", None);
    assert!(
        response.starts_with("HTTP/1.1 204"),
        "/v1/healthz expected 204 unauthenticated, got: {}",
        response.lines().next().unwrap_or_default()
    );

    // GET /v1/health stays public (Phase 2's own design) — and now
    // returns real data instead of the canned `demo-card`.
    let response = http_get(port, "/v1/health", None);
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "/v1/health expected 200 unauthenticated, got: {}",
        response.lines().next().unwrap_or_default()
    );
    assert!(
        response
            .lines()
            .any(|line| line.eq_ignore_ascii_case("cache-control: no-store")),
        "/v1/health must not be served from an HTTP cache"
    );
    assert!(
        !response.contains("demo-card"),
        "/v1/health should no longer serve the Phase 2 canned card"
    );

    // Mounted-but-unauthenticated: 401 (or, once ten failures have
    // already tripped `auth.rs`'s own rate limiter partway through this
    // loop, 429) — never 404 — on every new GET route. Either code only
    // happens because the auth middleware actually ran, which only
    // happens for a mounted path, so both are equally valid proof of
    // mounting.
    for path in [
        "/v1/versions",
        "/v1/versions/create",
        "/v1/java-runtimes",
        "/v1/config/java-runtime",
        "/v1/config/ram",
        "/v1/health/problems",
    ] {
        let response = http_get(port, path, None);
        assert!(
            response.starts_with("HTTP/1.1 401") || response.starts_with("HTTP/1.1 429"),
            "{path} GET expected 401/429 (mounted + auth-gated), got: {}",
            response.lines().next().unwrap_or_default()
        );
    }

    // Same proof for every new POST route.
    for path in [
        "/v1/components/version",
        "/v1/java-runtimes/install",
        "/v1/config/java-runtime",
        "/v1/config/ram",
        "/v1/health/repair",
    ] {
        let response = http_post(port, path, None);
        assert!(
            response.starts_with("HTTP/1.1 401") || response.starts_with("HTTP/1.1 429"),
            "{path} POST expected 401/429 (mounted + auth-gated), got: {}",
            response.lines().next().unwrap_or_default()
        );
    }

    // An actually-unmounted path still 404s, proving the 401s above
    // aren't just this agent's blanket behavior for any unknown path.
    let response = http_get(port, "/v1/versions/does-not-exist-route", None);
    assert!(
        response.starts_with("HTTP/1.1 404"),
        "expected a genuinely unmounted path to 404, got: {}",
        response.lines().next().unwrap_or_default()
    );

    stop_child(&mut agent);
    cleanup_secret(&keychain_service, "remote-api.owner-token");
}

#[test]
fn runtime_diagnostics_routes_accept_the_bootstrap_token() {
    let temp = temp_dir("runtime-diagnostics-routes-authed");
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
        "com.msc2.runtime-diagnostics-routes-authed.{}.{}",
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

    // Authenticated GET /v1/versions against an empty fleet (no active
    // server) succeeds with an honest "no active server" note, not a 500
    // — proof the route is really wired to `LifecycleRoutesState`.
    let response = http_get(port, "/v1/versions", Some(TOKEN));
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "authenticated GET /v1/versions expected 200, got: {}",
        response.lines().next().unwrap_or_default()
    );
    assert!(response.contains("\"supportsVersions\":false"));

    // Authenticated GET /v1/java-runtimes returns a real (possibly empty)
    // detected-runtimes array, not an error.
    let response = http_get(port, "/v1/java-runtimes", Some(TOKEN));
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "authenticated GET /v1/java-runtimes expected 200, got: {}",
        response.lines().next().unwrap_or_default()
    );
    assert!(response.contains("\"runtimes\""));

    // An authenticated RAM update naming neither field is a typed
    // no_changes 400, not a 401/500 -- proof the route's own validation
    // runs.
    let response = http_post_json(port, "/v1/config/ram", Some(TOKEN), "{}");
    assert!(
        response.starts_with("HTTP/1.1 400"),
        "authenticated no-op ram update expected 400, got: {}",
        response.lines().next().unwrap_or_default()
    );
    assert!(response.contains("no_changes"));

    // An authenticated Java runtime install naming an unrecognized major
    // is a typed 400, not a 401/500.
    let response = http_post_json(
        port,
        "/v1/java-runtimes/install",
        Some(TOKEN),
        r#"{"major":99}"#,
    );
    assert!(
        response.starts_with("HTTP/1.1 400"),
        "authenticated bad-major install expected 400, got: {}",
        response.lines().next().unwrap_or_default()
    );
    assert!(response.contains("invalid_major"));

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
