//! Black-box smoke test for P6.17's scheduler wiring in `build_app()`
//! (`main.rs`): a real `msc serve` process, started against a config
//! with one auto-backup-enabled server, must come up healthy and stay
//! up — proving `BackupScheduler::reconfigure`'s startup call (spawning
//! a real tokio interval task per enabled server) doesn't panic or wedge
//! the server. This crate has no `lib.rs`, so it can't unit-test
//! `BackupScheduler`/`SchedulerBackend` directly from here — that
//! substantive coverage (gate order, cadence, live-reconfiguration
//! start/stop/restart, all against a paused tokio clock) lives as
//! internal unit tests inside `src/backup_scheduler.rs` itself instead;
//! this file only needs to prove the real wiring doesn't crash, the same
//! `CARGO_BIN_EXE_msc`-driven pattern `startup_secret_migration.rs`
//! already established.
//!
//! macOS-only for the same reason that file is: `build_app()`
//! unconditionally provisions a real production `SecretStore`, and only
//! macOS Keychain is available, isolated, and fast enough in this
//! environment to exercise that path in a test — Linux needs a running
//! credential-helper socket this environment doesn't provide.

#![cfg(target_os = "macos")]

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[test]
fn backup_scheduler_startup_with_auto_backup_enabled_server_stays_healthy() {
    let temp = temp_dir("backup-scheduler-startup");
    let data_dir = temp.join("data");
    let servers_root = temp.join("servers");
    let config_path = data_dir.join("server_config_swift.json");
    fs::create_dir_all(&data_dir).unwrap();
    fs::create_dir_all(&servers_root).unwrap();
    let server_dir = servers_root.join("auto_backup_java");
    fs::create_dir_all(&server_dir).unwrap();

    fs::write(
        &config_path,
        format!(
            r#"{{
  "config_version": 1,
  "servers_root": "{}",
  "servers": [
    {{
      "id": "22222222-2222-2222-2222-222222222222",
      "display_name": "Auto Backup Java",
      "server_dir": "{}",
      "paper_jar_path": "{}/paper.jar",
      "min_ram_gb": 1,
      "max_ram_gb": 2,
      "server_type": "java",
      "auto_backup_enabled": true,
      "auto_backup_interval_minutes": 1,
      "auto_backup_max_count": 3
    }}
  ]
}}"#,
            json_path(&servers_root),
            json_path(&server_dir),
            json_path(&server_dir),
        ),
    )
    .unwrap();

    let port = free_port();
    let base_url = format!("127.0.0.1:{port}");
    let keychain_service = format!(
        "com.msc2.backup-scheduler-startup.{}.{}",
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

    // Still healthy a moment later — the scheduler's spawned per-server
    // interval task hasn't panicked the process or otherwise wedged it.
    thread::sleep(Duration::from_millis(500));
    assert!(
        http_get(port, "/v1/health", None).starts_with("HTTP/1.1 200"),
        "agent stopped responding after startup"
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
