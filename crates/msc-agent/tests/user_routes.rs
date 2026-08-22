//! Black-box coverage for P9.12's durable named-token routes.
//!
//! The production service uses the macOS Keychain, so these tests use a
//! unique test service and a disposable agent data directory. The same
//! process boundary is used for the restart assertion: no test-only auth
//! state is injected into the service.

#![cfg(target_os = "macos")]

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const OWNER_TOKEN: &str = "msc2_replacementrecovery_recoverysecret";

#[test]
fn user_routes_are_admin_only_and_never_list_raw_tokens() {
    let temp = temp_dir("user-routes-crud");
    let keychain_service = unique_service("crud");
    let (mut agent, port) = spawn_agent(&temp, &keychain_service);
    wait_for_agent(port, &mut agent);

    let created = http_post_json(
        port,
        "/v1/users",
        OWNER_TOKEN,
        r#"{"label":"  phone  ","role":"named","permissions":["players"]}"#,
    );
    assert_status(&created, 200);
    let token = json_string(&created, "token");
    let user_id = json_string(&created, "id");
    assert!(created.contains("\"label\":\"phone\""));
    assert!(!created.contains("remote-api.token"));

    let list = http_get(port, "/v1/users", OWNER_TOKEN);
    assert_status(&list, 200);
    assert!(list.contains(&user_id));
    assert!(
        !list.contains(&token),
        "list response exposed the raw bearer"
    );

    let named_list = http_get(port, "/v1/users", &token);
    assert_status(&named_list, 403);

    let updated = http_post_json(
        port,
        "/v1/users/update",
        OWNER_TOKEN,
        &format!(r#"{{"userId":"{user_id}","label":"phone 2","expiresInDays":7}}"#),
    );
    assert_status(&updated, 200);
    assert!(updated.contains("\"label\":\"phone 2\""));
    assert!(
        !updated.contains(&token),
        "update response exposed the raw bearer"
    );

    let revoked = http_post_json(
        port,
        "/v1/users/revoke",
        OWNER_TOKEN,
        &format!(r#"{{"userId":"{user_id}"}}"#),
    );
    assert_status(&revoked, 200);
    assert!(revoked.contains("\"message\":\"revoked\""));
    assert_status(&http_get(port, "/v1/me", &token), 401);

    let list_after_revoke = http_get(port, "/v1/users", OWNER_TOKEN);
    assert_status(&list_after_revoke, 200);
    assert!(!list_after_revoke.contains(&user_id));

    stop_agent(&mut agent);
    cleanup_secret(&keychain_service, "remote-api.token.replacementrecovery");
}

#[test]
fn revoked_user_stays_rejected_after_agent_restart() {
    let temp = temp_dir("user-routes-restart");
    let keychain_service = unique_service("restart");
    let (mut first, port) = spawn_agent(&temp, &keychain_service);
    wait_for_agent(port, &mut first);

    let created = http_post_json(
        port,
        "/v1/users",
        OWNER_TOKEN,
        r#"{"label":"restart proof","role":"named","permissions":["players"]}"#,
    );
    assert_status(&created, 200);
    let token = json_string(&created, "token");
    let user_id = json_string(&created, "id");
    let revoked = http_post_json(
        port,
        "/v1/users/revoke",
        OWNER_TOKEN,
        &format!(r#"{{"userId":"{user_id}"}}"#),
    );
    assert_status(&revoked, 200);
    stop_agent(&mut first);

    let (mut second, restarted_port) = spawn_agent(&temp, &keychain_service);
    wait_for_agent(restarted_port, &mut second);
    assert_status(&http_get(restarted_port, "/v1/me", &token), 401);
    let list = http_get(restarted_port, "/v1/users", OWNER_TOKEN);
    assert_status(&list, 200);
    assert!(!list.contains(&user_id));

    stop_agent(&mut second);
    cleanup_secret(&keychain_service, "remote-api.token.replacementrecovery");
}

fn spawn_agent(temp: &Path, keychain_service: &str) -> (Child, u16) {
    let data_dir = temp.join("data");
    let servers_root = temp.join("servers");
    let config_path = data_dir.join("server_config_swift.json");
    fs::create_dir_all(&data_dir).unwrap();
    fs::create_dir_all(&servers_root).unwrap();
    fs::write(
        &config_path,
        format!(
            r#"{{"config_version":1,"servers_root":"{}","servers":[]}}"#,
            json_path(&servers_root)
        ),
    )
    .unwrap();

    let port = free_port();
    let log_path = temp.join(format!("agent-{port}.log"));
    let log = fs::File::create(log_path).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_msc"));
    command
        .arg("serve")
        .arg("--bind")
        .arg(format!("127.0.0.1:{port}"))
        .env("MSC2_DATA_DIR", &data_dir)
        .env("MSC2_APP_CONFIG_PATH", &config_path)
        .env("MSC2_AGENT_SERVERS_ROOT", &servers_root)
        .env(
            "MSC2_CREDENTIAL_REGISTRY_PATH",
            data_dir.join("credential-registry.json"),
        )
        .env("MSC2_MACOS_USER_KEYCHAIN_SERVICE", keychain_service)
        .env("MSC2_OPERATION_JOURNAL_DIR", data_dir.join("journal"))
        .env("MSC2_TEST_BOOTSTRAP_TOKEN", OWNER_TOKEN)
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log));
    (command.spawn().unwrap(), port)
}

fn wait_for_agent(port: u16, child: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if http_get(port, "/v1/health", "").starts_with("HTTP/1.1 200") {
            return;
        }
        if child.try_wait().unwrap().is_some() {
            panic!("agent exited before becoming healthy");
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("agent did not become healthy on port {port}");
}

fn stop_agent(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn http_get(port: u16, path: &str, bearer: &str) -> String {
    request(port, "GET", path, bearer, "")
}

fn http_post_json(port: u16, path: &str, bearer: &str, body: &str) -> String {
    request(port, "POST", path, bearer, body)
}

fn request(port: u16, method: &str, path: &str, bearer: &str, body: &str) -> String {
    let Ok(mut stream) = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port))) else {
        return String::new();
    };
    let auth = if bearer.is_empty() {
        String::new()
    } else {
        format!("Authorization: Bearer {bearer}\r\n")
    };
    let content_type = if body.is_empty() {
        String::new()
    } else {
        format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        )
    };
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth}{content_type}Connection: close\r\n\r\n{body}"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn assert_status(response: &str, expected: u16) {
    assert!(
        response.starts_with(&format!("HTTP/1.1 {expected}")),
        "expected HTTP {expected}, got {}",
        response.lines().next().unwrap_or_default()
    );
}

fn json_string(response: &str, field: &str) -> String {
    let marker = format!("\"{field}\":\"");
    let start = response
        .find(&marker)
        .unwrap_or_else(|| panic!("missing JSON field {field}: {response}"))
        + marker.len();
    let end = response[start..]
        .find('"')
        .map(|offset| start + offset)
        .unwrap();
    response[start..end].to_string()
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("msc2-{tag}-{}", suffix()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn unique_service(tag: &str) -> String {
    format!("com.msc2.user-routes-{tag}-{}", suffix())
}

fn suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
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

fn json_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
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
