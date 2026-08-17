//! P6.29 black-box proof that a server whose startup world-reconciliation
//! (or restart-transaction recovery) fails is placed in a degraded,
//! read-only-for-mutations state, and that this is (a) scoped to exactly
//! that one server and (b) deterministic across a second agent startup.
//!
//! This crate has no `lib.rs`, so — same constraint `world_backup_routes.rs`
//! already documents — an external test file can't call
//! `crate::routes::worlds`/`crate::routes::lifecycle` directly. This test
//! drives a real `msc serve` process instead: two servers are registered in
//! a real on-disk app config before the agent ever starts, one with an
//! ordinary live world folder, the other with an explicit active-slot
//! marker pointing at a slot whose recorded `world.zip` is corrupt (a
//! realistic on-disk shape: an interrupted copy/transfer can easily leave
//! a truncated archive behind). `crates/msc-application/tests/
//! world_import_reconciliation.rs` already proves `reconcile_imported_worlds`
//! itself returns `Err` and touches nothing on a corrupt archive; this file
//! proves the agent actually *acts* on that error the way P6.29 requires:
//! world/backup mutation routes for the broken server return one
//! structured `409 world_reconciliation_degraded` error instead of
//! running, the healthy server's own routes are entirely unaffected, and a
//! second full agent startup against the same (still-broken) disk state
//! reaches the same conclusion rather than crashing, hanging, or drifting.
//!
//! macOS-only for the same reason `world_backup_routes.rs` is: `msc serve`
//! unconditionally provisions a real production `SecretStore`, and only
//! macOS Keychain is available, isolated, and fast enough here to
//! exercise that path in a test. `MSC2_TEST_BOOTSTRAP_TOKEN` is the same
//! dev-only owner-credential bootstrap `tools/phase6/phase6-gate-smoke.sh`
//! already uses for its own real, authenticated agent round trips.

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

const TOKEN: &str = "msc2_reconciliationgate_recgatesecret";

#[test]
fn world_import_reconciliation_degrades_only_the_broken_server_and_is_idempotent_across_restarts() {
    let temp = temp_dir("world-import-reconciliation-gate");
    let data_dir = temp.join("data");
    let servers_root = temp.join("servers");
    let config_path = data_dir.join("server_config_swift.json");
    fs::create_dir_all(&data_dir).unwrap();
    fs::create_dir_all(&servers_root).unwrap();

    let healthy_dir = servers_root.join("healthy");
    fs::create_dir_all(healthy_dir.join("world")).unwrap();
    fs::write(healthy_dir.join("world/level.dat"), b"real level data").unwrap();
    fs::write(healthy_dir.join("paper.jar"), b"fake jar").unwrap();

    // An explicit active-slot marker resolves to a real, recorded slot —
    // not one of the "unresolvable, tolerated" corrupt-metadata cases
    // `crates/msc-application/tests/world_import_reconciliation.rs`
    // already covers — whose own archive is corrupt. Extraction must be
    // attempted and must fail, which is exactly the case P6.29 requires
    // to degrade the server rather than silently warn and continue.
    let broken_dir = servers_root.join("broken");
    fs::create_dir_all(broken_dir.join("world_slots/slot-corrupt")).unwrap();
    fs::write(
        broken_dir.join("world_slots/slot-corrupt/slot.json"),
        r#"{"id":"slot-corrupt","name":"Slot","created_at":"2026-01-01T00:00:00Z"}"#,
    )
    .unwrap();
    fs::write(
        broken_dir.join("world_slots/slot-corrupt/world.zip"),
        b"not a real zip archive",
    )
    .unwrap();
    fs::write(
        broken_dir.join("world_slots/active_slot_id.txt"),
        "slot-corrupt",
    )
    .unwrap();
    fs::write(broken_dir.join("paper.jar"), b"fake jar").unwrap();

    write_app_config(&config_path, &servers_root, &healthy_dir, &broken_dir);

    let port = free_port();
    let base_url = format!("127.0.0.1:{port}");
    let keychain_service = format!(
        "com.msc2.world-import-reconciliation-gate.{}.{}",
        std::process::id(),
        suffix()
    );
    let log_path = temp.join("agent-1.log");
    let mut agent = spawn_agent(
        &base_url,
        &data_dir,
        &config_path,
        &servers_root,
        &keychain_service,
        &log_path,
    );
    wait_for_health(port);

    // The healthy server (already active per the config) reconciled fine
    // at startup: an ordinary mutation succeeds.
    assert_world_create_succeeds(port, "First Run Healthy");

    // The broken server remains registered, but P6.38 closes authority
    // one step earlier than P6.29 did: it cannot even become active.
    assert_server_activation_degraded(port, "broken");

    // Switching back proves the healthy server was never blocked by the
    // broken one sharing the same agent process.
    activate_server(port, "healthy");
    assert_world_create_succeeds(port, "First Run Healthy Again");

    // A server imported after startup must pass the same authority gate.
    // Its corrupt active archive makes reconciliation fail after the
    // copied server has been registered; it remains visible for diagnosis
    // but cannot become active.
    let post_start_source = temp.join("post-start-broken-source");
    write_corrupt_import_source(&post_start_source);
    let import_response = http_post(
        port,
        "/v1/servers/import",
        Some(TOKEN),
        &format!(
            r#"{{"action":"importExisting","sourcePath":"{}","importKind":"folder","displayName":"Post Start Broken","serverType":"java"}}"#,
            json_path(&post_start_source)
        ),
    );
    assert!(
        import_response.starts_with("HTTP/1.1 202"),
        "post-start import should be accepted promptly: {import_response}"
    );
    let operation_id = response_json(&import_response)["operationId"]
        .as_str()
        .expect("import response carries operationId")
        .to_string();
    let operation = wait_for_operation(port, &operation_id);
    assert_eq!(operation["state"], "succeeded", "{operation}");
    assert_eq!(operation["result"]["ready"], "false", "{operation}");
    let imported_id = operation["result"]["serverId"]
        .as_str()
        .expect("completed import operation carries serverId")
        .to_string();
    let activation = http_post(
        port,
        "/v1/active-server",
        Some(TOKEN),
        &format!(r#"{{"serverId":"{imported_id}"}}"#),
    );
    assert!(
        activation.starts_with("HTTP/1.1 409")
            && activation.contains("world_reconciliation_degraded"),
        "post-start degraded import must not be selectable: {activation}"
    );

    stop_child(&mut agent);
    cleanup_secret(&keychain_service, "remote-api.owner-token");

    // Second startup, same on-disk state (nothing fixed the broken
    // server's corrupt archive): reconciliation must reach the same
    // conclusion deterministically — the broken server degrades again,
    // the healthy server is unaffected again, and the agent comes up
    // cleanly rather than failing to start at all.
    let port2 = free_port();
    let base_url2 = format!("127.0.0.1:{port2}");
    let log_path2 = temp.join("agent-2.log");
    let mut agent2 = spawn_agent(
        &base_url2,
        &data_dir,
        &config_path,
        &servers_root,
        &keychain_service,
        &log_path2,
    );
    wait_for_health(port2);

    activate_server(port2, "healthy");
    assert_world_create_succeeds(port2, "Second Startup Healthy");
    assert_server_activation_degraded(port2, "broken");

    stop_child(&mut agent2);
    cleanup_secret(&keychain_service, "remote-api.owner-token");
}

fn write_corrupt_import_source(source: &Path) {
    fs::create_dir_all(source.join("world_slots/slot-corrupt")).unwrap();
    fs::write(source.join("paper-1.21.1.jar"), b"fake jar").unwrap();
    fs::write(
        source.join("server.properties"),
        "server-port=25570\nlevel-name=world\n",
    )
    .unwrap();
    fs::write(
        source.join("world_slots/slot-corrupt/slot.json"),
        r#"{"id":"slot-corrupt","name":"Broken","created_at":"2026-01-01T00:00:00Z"}"#,
    )
    .unwrap();
    fs::write(
        source.join("world_slots/slot-corrupt/world.zip"),
        b"not a real zip archive",
    )
    .unwrap();
    fs::write(
        source.join("world_slots/active_slot_id.txt"),
        "slot-corrupt",
    )
    .unwrap();
}

fn response_json(response: &str) -> serde_json::Value {
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .expect("HTTP response contains a body separator");
    serde_json::from_str(body).expect("HTTP response body is JSON")
}

fn json_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

fn write_app_config(
    config_path: &Path,
    servers_root: &Path,
    healthy_dir: &Path,
    broken_dir: &Path,
) {
    let mut config = AppConfig::default_config(servers_root.to_string_lossy().into_owned());
    let mut healthy = ConfigServer::new(
        "healthy",
        "Healthy",
        healthy_dir.to_string_lossy().into_owned(),
        healthy_dir.join("paper.jar").to_string_lossy().into_owned(),
        1.0,
        2.0,
    );
    healthy.server_type = ServerType::Java;
    let mut broken = ConfigServer::new(
        "broken",
        "Broken",
        broken_dir.to_string_lossy().into_owned(),
        broken_dir.join("paper.jar").to_string_lossy().into_owned(),
        1.0,
        2.0,
    );
    broken.server_type = ServerType::Java;
    config.servers = vec![healthy, broken];
    config.active_server_id = Some("healthy".to_string());
    save_app_config(&StdFileSystem, config_path, &config).unwrap();
}

fn activate_server(port: u16, server_id: &str) {
    let response = http_post(
        port,
        "/v1/active-server",
        Some(TOKEN),
        &format!(r#"{{"serverId":"{server_id}"}}"#),
    );
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "activating {server_id} failed: {}",
        response.lines().next().unwrap_or_default()
    );
}

fn assert_server_activation_degraded(port: u16, server_id: &str) {
    let response = http_post(
        port,
        "/v1/active-server",
        Some(TOKEN),
        &format!(r#"{{"serverId":"{server_id}"}}"#),
    );
    assert!(
        response.starts_with("HTTP/1.1 409") && response.contains("world_reconciliation_degraded"),
        "selecting degraded server {server_id} should fail closed: {response}"
    );
}

fn assert_world_create_succeeds(port: u16, name: &str) {
    let response = http_post(
        port,
        "/v1/worlds/create",
        Some(TOKEN),
        &format!(r#"{{"name":"{name}"}}"#),
    );
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "expected world create to succeed for a healthy server, got: {}\n{response}",
        response.lines().next().unwrap_or_default()
    );
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
        .env("MSC2_OPERATION_JOURNAL_DIR", data_dir.join("journal"))
        .env("MSC2_AUDIT_LOG_DIR", data_dir.join("audit-log"))
        .env("MSC2_MACOS_USER_KEYCHAIN_SERVICE", keychain_service)
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

fn wait_for_operation(port: u16, operation_id: &str) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        let response = http_get(port, &format!("/v1/operations/{operation_id}"), Some(TOKEN));
        if response.starts_with("HTTP/1.1 200") {
            let operation = response_json(&response);
            if matches!(
                operation["state"].as_str(),
                Some("succeeded" | "failed" | "cancelled")
            ) {
                return operation;
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("operation {operation_id} did not become terminal");
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

fn http_post(port: u16, path: &str, bearer: Option<&str>, body: &str) -> String {
    let Ok(mut stream) = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))) else {
        return String::new();
    };
    let auth = bearer
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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
