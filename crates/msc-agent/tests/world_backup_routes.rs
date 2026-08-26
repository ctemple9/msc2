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

use msc_domain::app_config_schema::{AppConfig, ConfigServer};
use msc_domain::identity::ServerType;
use msc_infrastructure::config_repository::save_app_config;
use msc_infrastructure::fs::StdFileSystem;

const TOKEN: &str = "msc2_replacementrecovery_recoverysecret";

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
    for path in [
        "/v1/worlds",
        "/v1/worlds/convert/formats",
        "/v1/worlds/slot-id/thumbnail",
        "/v1/backups",
        "/v1/backups/config",
    ] {
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

    let response = http_post(port, "/v1/worlds/slot-id/thumbnail", None);
    assert!(
        response.starts_with("HTTP/1.1 401"),
        "/v1/worlds/slot-id/thumbnail expected 401 (mounted + auth-gated), got: {}",
        response.lines().next().unwrap_or_default()
    );

    stop_child(&mut agent);
    cleanup_secret(&keychain_service, "remote-api.owner-token");
}

#[test]
fn world_backup_routes_restart_recovers_each_active_replacement_boundary_and_operation() {
    for boundary in ["staged", "prior_moved", "installed"] {
        let temp = temp_dir(&format!("world-replace-recovery-{boundary}"));
        let data_dir = temp.join("data");
        let servers_root = temp.join("servers");
        let server_dir = servers_root.join("replacement-server");
        let config_path = data_dir.join("server_config_swift.json");
        let journal_dir = data_dir.join("journal");
        fs::create_dir_all(&server_dir).unwrap();
        fs::create_dir_all(&journal_dir).unwrap();
        fs::write(server_dir.join("paper.jar"), b"fake jar").unwrap();
        fs::write(
            server_dir.join("server.properties"),
            "server-port=25565\nlevel-name=world\n",
        )
        .unwrap();
        seed_replace_boundary(&server_dir, boundary);
        write_single_server_config(&config_path, &servers_root, &server_dir);

        let operation_id = format!("replace-restart-{boundary}");
        fs::write(
            journal_dir.join(format!("{operation_id}.json")),
            format!(
                r#"{{"id":"{operation_id}","operationType":"world-replace-active","target":"replacement-server","state":"running","error":null}}"#
            ),
        )
        .unwrap();

        let port = free_port();
        let keychain_service = format!(
            "com.msc2.world-replace-recovery.{boundary}.{}.{}",
            std::process::id(),
            suffix()
        );
        let log_path = temp.join("agent.log");
        let mut agent = spawn_agent(
            &format!("127.0.0.1:{port}"),
            &data_dir,
            &config_path,
            &servers_root,
            &keychain_service,
            &log_path,
        );
        wait_for_health(port);

        let operation = http_get(port, &format!("/v1/operations/{operation_id}"), Some(TOKEN));
        assert!(
            operation.starts_with("HTTP/1.1 200")
                && operation.contains(r#""state":"failed""#)
                && operation.contains("operation_interrupted"),
            "{boundary} operation record was not truthfully reconciled: {operation}"
        );

        let properties = fs::read_to_string(server_dir.join("server.properties")).unwrap();
        if boundary == "installed" {
            assert_eq!(
                fs::read(server_dir.join("newname/level.dat")).unwrap(),
                b"complete replacement world"
            );
            assert!(properties.contains("level-name=newname"));
        } else {
            assert_eq!(
                fs::read(server_dir.join("world/level.dat")).unwrap(),
                b"complete old world"
            );
            assert!(properties.contains("level-name=world"));
        }
        assert!(!server_dir.join("world_slots/.replace").exists());

        let worlds = http_get(port, "/v1/worlds", Some(TOKEN));
        assert!(
            worlds.starts_with("HTTP/1.1 200"),
            "{boundary} recovered server should remain publicly inspectable: {worlds}"
        );

        stop_child(&mut agent);
        cleanup_secret(&keychain_service, "remote-api.owner-token");
    }
}

#[test]
fn world_backup_routes_imports_an_on_disk_legacy_backup_without_redeeming_it() {
    let temp = temp_dir("world-legacy-backup-import");
    let data_dir = temp.join("data");
    let servers_root = temp.join("servers");
    let server_dir = servers_root.join("legacy-backup-server");
    let config_path = data_dir.join("server_config_swift.json");
    let backup_path = server_dir.join("backups/legacy-world.zip");
    fs::create_dir_all(&data_dir).unwrap();
    fs::create_dir_all(backup_path.parent().unwrap()).unwrap();
    fs::write(server_dir.join("paper.jar"), b"fake jar").unwrap();
    fs::write(&backup_path, b"legacy backup bytes").unwrap();
    write_single_server_config(&config_path, &servers_root, &server_dir);

    let port = free_port();
    let keychain_service = format!(
        "com.msc2.world-legacy-backup-import.{}.{}",
        std::process::id(),
        suffix()
    );
    let log_path = temp.join("agent.log");
    let mut agent = spawn_agent(
        &format!("127.0.0.1:{port}"),
        &data_dir,
        &config_path,
        &servers_root,
        &keychain_service,
        &log_path,
    );
    wait_for_health(port);

    let response = http_post_json(
        port,
        "/v1/worlds/import",
        TOKEN,
        r#"{"name":"Imported Legacy","backupId":"legacy-world.zip"}"#,
    );
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "unexpected response: {response}"
    );
    assert!(
        response.contains(r#""name":"Imported Legacy"#),
        "unexpected response: {response}"
    );
    assert!(backup_path.exists(), "legacy backup must remain available");

    stop_child(&mut agent);
    cleanup_secret(&keychain_service, "remote-api.owner-token");
}

fn seed_replace_boundary(server_dir: &Path, boundary: &str) {
    let replace_dir = server_dir.join("world_slots/.replace");
    fs::create_dir_all(&replace_dir).unwrap();
    fs::write(
        replace_dir.join("manifest.json"),
        br#"{"level_name":"newname"}"#,
    )
    .unwrap();

    match boundary {
        "staged" => {
            write_level(server_dir.join("world"), b"complete old world");
            write_level(
                replace_dir.join("staged/newname"),
                b"complete replacement world",
            );
        }
        "prior_moved" => {
            write_level(replace_dir.join("prior/world"), b"complete old world");
            write_level(
                replace_dir.join("staged/newname"),
                b"complete replacement world",
            );
        }
        "installed" => {
            write_level(server_dir.join("newname"), b"complete replacement world");
            write_level(replace_dir.join("prior/world"), b"complete old world");
        }
        _ => unreachable!(),
    }
}

fn write_level(dir: PathBuf, contents: &[u8]) {
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("level.dat"), contents).unwrap();
}

fn write_single_server_config(config_path: &Path, servers_root: &Path, server_dir: &Path) {
    let mut config = AppConfig::default_config(servers_root.to_string_lossy().into_owned());
    let mut server = ConfigServer::new(
        "replacement-server",
        "Replacement Server",
        server_dir.to_string_lossy().into_owned(),
        server_dir.join("paper.jar").to_string_lossy().into_owned(),
        1.0,
        2.0,
    );
    server.server_type = ServerType::Java;
    config.servers = vec![server];
    config.active_server_id = Some("replacement-server".to_string());
    save_app_config(&StdFileSystem, config_path, &config).unwrap();
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

fn http_post_json(port: u16, path: &str, bearer: &str, body: &str) -> String {
    let Ok(mut stream) = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))) else {
        return String::new();
    };
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {bearer}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
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
