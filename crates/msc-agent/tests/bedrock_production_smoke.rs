//! P10.35's cross-backend proof through the production agent.
//!
//! The test starts the real `msc serve` composition root. Each platform job
//! supplies the adapter selected by production (`bedrock_server` on Linux or
//! Windows, the Swift-sidecar boundary on Intel macOS); the files and process
//! responses are disposable fixtures, so no BDS package, VM, or provider is
//! started. The adapter table is intentionally shared across jobs so a job
//! cannot silently exercise a different backend than its platform claims.

use serde_json::Value;
use std::thread;
use std::time::{Duration, Instant};

#[path = "support/bedrock_smoke.rs"]
#[allow(dead_code)]
mod bedrock_smoke;

fn output_json(output: std::process::Output, label: &str) -> Value {
    assert!(
        output.status.success(),
        "{label}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{label}: expected JSON ({error}); stdout={}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn wait_for_operation(fixture: &bedrock_smoke::ProductionFixture, id: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let (status, operation) = fixture.http("GET", &format!("/v1/operations/{id}"), None);
        assert_eq!(status, 200, "operation lookup: {operation}");
        match operation["state"].as_str() {
            Some("queued") | Some("running") => {}
            _ => return operation,
        }
        assert!(
            Instant::now() < deadline,
            "operation did not finish: {operation}"
        );
        thread::sleep(Duration::from_millis(100));
    }
}

fn wait_for_status(fixture: &bedrock_smoke::ProductionFixture, expected_running: bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let (status, snapshot) = fixture.http("GET", "/v1/status", None);
        assert_eq!(status, 200, "status lookup: {snapshot}");
        if snapshot["running"] == expected_running {
            return snapshot;
        }
        assert!(
            Instant::now() < deadline,
            "status did not reach running={expected_running}: {snapshot}"
        );
        thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn production_router_covers_the_bedrock_cross_backend_contract() {
    let labels: Vec<_> = bedrock_smoke::ProductionBackend::ALL
        .iter()
        .map(|backend| backend.label())
        .collect();
    assert_eq!(labels, ["linux-native", "windows-native", "macos-sidecar"]);

    let backend = bedrock_smoke::ProductionBackend::current();
    let fixture = bedrock_smoke::ProductionFixture::new();
    fixture.seed(backend);
    let mut agent = fixture.spawn_agent();
    fixture.wait_for_health();

    let (status, capabilities) = fixture.http("GET", "/v1/capabilities", None);
    assert_eq!(status, 200);
    assert_eq!(capabilities["hostOs"], backend.host_os());
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        assert_eq!(
            capabilities["serverTypes"]["bedrock"]["runtime"]["state"],
            "unavailable"
        );
        assert_eq!(
            capabilities["serverTypes"]["bedrock"]["runtime"]["reasonCode"],
            "no_test_hardware"
        );
    } else {
        assert_eq!(
            capabilities["serverTypes"]["bedrock"]["runtime"]["backend"],
            backend.api_backend()
        );
    }

    let create = output_json(
        fixture.cli(&[
            "server",
            "create",
            "Production Smoke Bedrock",
            "--type",
            "bedrock",
            "--version-id",
            "1.21.80.3",
            "--no-wait",
        ]),
        "create",
    );
    let create_operation = create["operationId"].as_str().expect("create operation id");
    assert_eq!(
        wait_for_operation(&fixture, create_operation)["state"],
        "succeeded"
    );

    let imported = output_json(
        fixture.cli(&[
            "server",
            "import",
            fixture.import_source.to_str().unwrap(),
            "--name",
            "Production Smoke Import",
            "--type",
            "bedrock",
        ]),
        "import",
    );
    assert_eq!(imported["state"], "succeeded");

    for path in ["/v1/settings", "/v1/players", "/v1/allowlist", "/v1/status"] {
        let (status, body) = fixture.http("GET", path, None);
        assert_eq!(status, 200, "{path}: {body}");
        if path != "/v1/status" {
            assert!(body["runtime"].is_object(), "{path} omitted runtime state");
        }
    }

    let (status, settings) = fixture.http(
        "POST",
        "/v1/settings",
        Some(r#"{"changes":{"difficulty":"peaceful"}}"#),
    );
    assert_eq!(status, 200, "settings: {settings}");
    assert_eq!(settings["success"], true);

    let (status, allowlist) = fixture.http(
        "POST",
        "/v1/allowlist",
        Some(r#"{"action":"add","name":"Casey"}"#),
    );
    assert_eq!(status, 200, "allowlist: {allowlist}");
    assert_eq!(allowlist["entries"].as_array().unwrap().len(), 2);

    let (status, operation) =
        fixture.http("POST", "/v1/operations", Some(r#"{"type":"demo-install"}"#));
    assert_eq!(status, 202, "operation create: {operation}");
    let operation_id = operation["id"].as_str().unwrap();
    let (status, cancelling) = fixture.http(
        "POST",
        &format!("/v1/operations/{operation_id}/cancel"),
        Some("{}"),
    );
    assert_eq!(status, 202, "operation cancel: {cancelling}");
    assert_eq!(
        wait_for_operation(&fixture, operation_id)["state"],
        "cancelled"
    );

    if cfg!(target_os = "linux") {
        let (status, started) = fixture.http("POST", "/v1/start", Some("{}"));
        assert_eq!(status, 200, "start: {started}");
        assert_eq!(started["runtime"]["backend"], "native");
        let started_status = wait_for_status(&fixture, true);
        assert_eq!(started_status["serverType"], "bedrock");

        let (status, command) = fixture.http(
            "POST",
            "/v1/command",
            Some(r#"{"command":"/say production smoke"}"#),
        );
        assert_eq!(status, 200, "command: {command}");
        assert_eq!(command["command"], "say production smoke");

        let (status, console) = fixture.http("GET", "/v1/console/tail?n=20", None);
        assert_eq!(status, 200);
        assert!(
            console
                .as_array()
                .unwrap()
                .iter()
                .any(|line| line["text"] == "Server started")
        );

        let (status, stopped) = fixture.http("POST", "/v1/stop", Some("{}"));
        assert_eq!(status, 200, "stop: {stopped}");
        wait_for_status(&fixture, false);

        let (status, recovered) = fixture.http("POST", "/v1/start", Some("{}"));
        assert_eq!(status, 200, "recovery start: {recovered}");
        wait_for_status(&fixture, true);
        let (status, _stopped) = fixture.http("POST", "/v1/stop", Some("{}"));
        assert_eq!(status, 200);
        wait_for_status(&fixture, false);
    } else {
        let (status, error) = fixture.http("POST", "/v1/start", Some("{}"));
        assert_eq!(status, 409, "unavailable start: {error}");
        assert_eq!(error["code"], "capability_unavailable");
        assert_eq!(error["details"]["serverType"], "bedrock");
    }

    fixture.stop(&mut agent);

    let unavailable = bedrock_smoke::ProductionFixture::new();
    unavailable.seed_unavailable(backend);
    let mut unavailable_agent = unavailable.spawn_agent();
    unavailable.wait_for_health();
    let (status, error) = unavailable.http("POST", "/v1/start", Some("{}"));
    assert_eq!(status, 409, "unavailable fixture start: {error}");
    assert_eq!(error["code"], "capability_unavailable");
    unavailable.stop(&mut unavailable_agent);
}
