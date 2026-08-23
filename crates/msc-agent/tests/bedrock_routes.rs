//! P10.21's public route-surface check.  The command is intentionally
//! offline: it proves the shared Bedrock route names are represented by the
//! agent's public surface without requiring a live BDS process.

use serde_json::json;
use std::process::Command;

#[path = "support/bedrock_smoke.rs"]
#[allow(dead_code)]
mod bedrock_smoke;

#[test]
fn bedrock_public_surface_is_advertised() {
    let output = Command::new(env!("CARGO_BIN_EXE_msc"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(
        help.contains("bedrock"),
        "missing Bedrock CLI surface: {help}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cross_backend_api_workflow_uses_one_contract() {
    for backend in bedrock_smoke::Backend::ALL {
        let harness = bedrock_smoke::spawn(backend, true).await;
        let start_fixture = bedrock_smoke::fixture("console-ready");

        let (status, started) =
            bedrock_smoke::request(harness.address, "POST", "/v1/start", "{}").await;
        assert_eq!(status, 200, "{}", backend.name());
        assert_eq!(started["serverType"], "bedrock");
        assert_eq!(
            started["backend"],
            if backend == bedrock_smoke::Backend::MacosSidecar {
                "vz-sidecar"
            } else {
                "native"
            }
        );
        assert_eq!(
            bedrock_smoke::request(harness.address, "GET", "/v1/status", "{}")
                .await
                .1["running"],
            true
        );

        let (status, command) = bedrock_smoke::request(
            harness.address,
            "POST",
            "/v1/command",
            &format!(r#"{{"command":"say hello from {}"}}"#, backend.name()),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(
            command["command"],
            format!("say hello from {}", backend.name())
        );

        let console = bedrock_smoke::request(harness.address, "GET", "/v1/console/tail?n=20", "")
            .await
            .1;
        assert_eq!(console[0]["text"], start_fixture["input"]["line"]);
        assert!(
            console[1]["text"]
                .as_str()
                .unwrap()
                .contains("command received")
        );

        let players = bedrock_smoke::request(harness.address, "GET", "/v1/players", "{}")
            .await
            .1;
        assert_eq!(players["count"], 1);
        assert_eq!(players["players"][0]["name"], "Alex");

        let settings = bedrock_smoke::request(harness.address, "GET", "/v1/settings", "{}")
            .await
            .1;
        assert_eq!(settings["serverType"], "bedrock");
        assert_eq!(
            settings["sections"][0]["fields"]
                .as_array()
                .unwrap()
                .iter()
                .find(|field| field["key"] == "level-name")
                .unwrap()["value"],
            "Realm"
        );
        let updated = bedrock_smoke::request(
            harness.address,
            "POST",
            "/v1/settings",
            r#"{"changes":{"difficulty":"peaceful"}}"#,
        )
        .await
        .1;
        assert_eq!(updated["appliedKeys"], json!(["difficulty"]));

        let allowlist = bedrock_smoke::request(harness.address, "GET", "/v1/allowlist", "{}")
            .await
            .1;
        assert_eq!(allowlist["entries"][0]["name"], "Alex");
        let allowlist = bedrock_smoke::request(
            harness.address,
            "POST",
            "/v1/allowlist",
            r#"{"action":"add","name":"Casey"}"#,
        )
        .await
        .1;
        assert_eq!(allowlist["entries"].as_array().unwrap().len(), 2);

        let operation = bedrock_smoke::request(
            harness.address,
            "POST",
            "/v1/operations",
            r#"{"type":"bedrock-provision"}"#,
        )
        .await
        .1;
        let operation_id = operation["id"].as_str().unwrap();
        assert_eq!(operation["state"], "running");
        let cancelled = bedrock_smoke::request(
            harness.address,
            "POST",
            &format!("/v1/operations/{operation_id}/cancel"),
            "{}",
        )
        .await
        .1;
        assert_eq!(cancelled["state"], "cancelled");

        // A cancelled operation leaves the fake stopped, then the same
        // public start path proves that the service can recover cleanly.
        assert!(
            bedrock_smoke::request(harness.address, "POST", "/v1/start", "{}")
                .await
                .0
                == 200
        );
        assert_eq!(
            bedrock_smoke::request(harness.address, "POST", "/v1/stop", "{}")
                .await
                .1["result"],
            "stopped"
        );

        let capabilities = bedrock_smoke::request(harness.address, "GET", "/v1/capabilities", "{}")
            .await
            .1;
        assert_eq!(capabilities["serverTypes"]["bedrock"]["supported"], true);
        harness.stop();

        let unavailable = bedrock_smoke::spawn(backend, false).await;
        let capabilities =
            bedrock_smoke::request(unavailable.address, "GET", "/v1/capabilities", "{}")
                .await
                .1;
        assert_eq!(capabilities["serverTypes"]["bedrock"]["supported"], false);
        assert_eq!(capabilities["runtime"]["state"], "unavailable");
        let (status, error) =
            bedrock_smoke::request(unavailable.address, "POST", "/v1/start", "{}").await;
        assert_eq!(status, 409);
        assert_eq!(error["code"], "capability_unavailable");
        assert_eq!(error["details"]["reasonCode"], "no_test_hardware");
        unavailable.stop();
    }
}
