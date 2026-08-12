#![cfg(target_os = "macos")]

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[test]
fn startup_secret_migration_survives_real_process_restart() {
    #[cfg(target_os = "linux")]
    {
        eprintln!(
            "skipping subprocess startup migration on Linux: production auth requires the installed credential-helper socket"
        );
    }

    #[cfg(not(target_os = "linux"))]
    run_startup_secret_migration_restart();
}

#[cfg(not(target_os = "linux"))]
fn run_startup_secret_migration_restart() {
    let temp = temp_dir("startup-secret-migration");
    let data_dir = temp.join("data");
    let servers_root = temp.join("servers");
    let config_path = data_dir.join("server_config_swift.json");
    fs::create_dir_all(&data_dir).unwrap();
    fs::create_dir_all(&servers_root).unwrap();
    let legacy_server_dir = servers_root.join("legacy_java");
    fs::create_dir_all(&legacy_server_dir).unwrap();

    fs::write(
        &config_path,
        format!(
            r#"{{
  "config_version": 1,
  "servers_root": "{}",
  "remote_api_token": "legacy-owner-secret-xyz",
  "servers": [
    {{
      "id": "11111111-1111-1111-1111-111111111111",
      "display_name": "Legacy Java",
      "server_dir": "{}",
      "paper_jar_path": "{}/paper.jar",
      "min_ram_gb": 2,
      "max_ram_gb": 4,
      "server_type": "java",
      "xbox_broadcast_alt_password": "legacy-alt-password"
    }}
  ]
}}"#,
            json_path(&servers_root),
            json_path(&legacy_server_dir),
            json_path(&legacy_server_dir),
        ),
    )
    .unwrap();

    let port = free_port();
    let base_url = format!("127.0.0.1:{port}");
    let keychain_service = format!(
        "com.msc2.startup-migration.{}.{}",
        std::process::id(),
        suffix()
    );
    let first_log = temp.join("agent-first.log");
    let second_log = temp.join("agent-second.log");

    let mut first = spawn_agent(
        &base_url,
        &data_dir,
        &config_path,
        &servers_root,
        &keychain_service,
        &first_log,
    );
    wait_for_health(port);
    let bearer = wait_for_migrated_bearer(&first_log);
    assert_authorized_status(port, &bearer);
    stop_child(&mut first);

    let rewritten = fs::read_to_string(&config_path).unwrap();
    assert!(!rewritten.contains("remote_api_token"));
    assert!(!rewritten.contains("xbox_broadcast_alt_password"));

    let mut second = spawn_agent(
        &base_url,
        &data_dir,
        &config_path,
        &servers_root,
        &keychain_service,
        &second_log,
    );
    wait_for_health(port);
    assert_authorized_status(port, &bearer);
    stop_child(&mut second);

    cleanup_secret(&keychain_service, legacy_owner_key());
    cleanup_secret(&keychain_service, legacy_alt_password_key());
    if let Some(credential_id) = bearer.split('_').nth(1) {
        cleanup_secret(
            &keychain_service,
            &format!("remote-api.token.{credential_id}"),
        );
    }
}

#[cfg(not(target_os = "linux"))]
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
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log));

    #[cfg(target_os = "macos")]
    command.env("MSC2_MACOS_USER_KEYCHAIN_SERVICE", keychain_service);

    command.spawn().unwrap()
}

#[cfg(not(target_os = "linux"))]
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

#[cfg(not(target_os = "linux"))]
fn wait_for_migrated_bearer(log_path: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(20);
    let marker = "new bearer token (shown once): ";
    while Instant::now() < deadline {
        let text = fs::read_to_string(log_path).unwrap_or_default();
        if let Some(start) = text.find(marker) {
            let token = text[start + marker.len()..]
                .lines()
                .next()
                .unwrap_or_default()
                .trim();
            if token.starts_with("msc2_") {
                return token.to_string();
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("agent never printed migrated bearer token");
}

#[cfg(not(target_os = "linux"))]
fn assert_authorized_status(port: u16, bearer: &str) {
    let response = http_get(port, "/v1/status", Some(bearer));
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "expected authorized status response, got {response}"
    );
}

#[cfg(not(target_os = "linux"))]
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

#[cfg(not(target_os = "linux"))]
fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(not(target_os = "linux"))]
fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

#[cfg(not(target_os = "linux"))]
fn temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("msc2-{name}-{}", suffix()));
    fs::create_dir_all(&path).unwrap();
    path
}

#[cfg(not(target_os = "linux"))]
fn suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

#[cfg(not(target_os = "linux"))]
fn json_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

#[cfg(all(not(target_os = "linux"), target_os = "macos"))]
fn cleanup_secret(service: &str, account: &str) {
    let _ = Command::new("/usr/bin/security")
        .arg("delete-generic-password")
        .arg("-s")
        .arg(service)
        .arg("-a")
        .arg(account)
        .status();
}

#[cfg(all(not(target_os = "linux"), not(target_os = "macos")))]
fn cleanup_secret(_service: &str, _account: &str) {}

#[cfg(not(target_os = "linux"))]
fn legacy_owner_key() -> &'static str {
    "remote-api.owner-token"
}

#[cfg(not(target_os = "linux"))]
fn legacy_alt_password_key() -> &'static str {
    "xbox-broadcast.alt-password.11111111-1111-1111-1111-111111111111"
}
