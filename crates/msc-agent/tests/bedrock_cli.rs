//! P10.21's scriptable Bedrock CLI argument-surface check.

#[path = "support/bedrock_smoke.rs"]
#[allow(dead_code)]
mod bedrock_smoke;

use serde_json::Value;
use std::process::Command;

#[test]
fn bedrock_cli_exposes_players_and_allowlist() {
    let output = Command::new(env!("CARGO_BIN_EXE_msc"))
        .args(["bedrock", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("players"), "missing players command: {help}");
    assert!(
        help.contains("allowlist"),
        "missing allowlist command: {help}"
    );
}

fn cli(url: &str, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_msc"))
        .args(["--base-url", url, "--token", "synthetic-token", "--json"])
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: std::process::Output, label: &str) -> Value {
    assert!(
        output.status.success(),
        "{label}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cross_backend_cli_workflow_uses_one_contract() {
    for backend in bedrock_smoke::Backend::ALL {
        let harness = bedrock_smoke::spawn(backend, true).await;
        let url = harness.url();
        assert_eq!(
            assert_success(cli(&url, &["server", "start"]), backend.name())["result"],
            "started"
        );
        assert_eq!(
            assert_success(cli(&url, &["status"]), backend.name())["running"],
            true
        );
        assert_eq!(
            assert_success(
                cli(&url, &["command", "say hello from cli"]),
                backend.name()
            )["command"],
            "say hello from cli"
        );
        let ready_line = bedrock_smoke::fixture("console-ready")["input"]["line"].clone();
        let console = assert_success(
            cli(&url, &["console", "tail", "--lines", "20"]),
            backend.name(),
        );
        assert!(
            console
                .as_array()
                .unwrap()
                .iter()
                .any(|line| { line["text"] == ready_line })
        );
        let players = assert_success(cli(&url, &["bedrock", "players"]), backend.name());
        assert_eq!(players["count"], 1);
        let settings = assert_success(cli(&url, &["settings", "get"]), backend.name());
        assert_eq!(settings["serverType"], "bedrock");
        let updated = assert_success(
            cli(&url, &["settings", "set", "difficulty=peaceful"]),
            backend.name(),
        );
        assert_eq!(updated["appliedKeys"], serde_json::json!(["difficulty"]));
        let allowlist = assert_success(cli(&url, &["bedrock", "allowlist", "get"]), backend.name());
        assert_eq!(allowlist["entries"][0]["name"], "Alex");
        let added = assert_success(
            cli(&url, &["bedrock", "allowlist", "add", "Casey"]),
            backend.name(),
        );
        assert_eq!(added["entries"].as_array().unwrap().len(), 2);
        assert_eq!(
            assert_success(cli(&url, &["server", "stop"]), backend.name())["result"],
            "stopped"
        );
        assert_eq!(
            assert_success(cli(&url, &["server", "start"]), backend.name())["result"],
            "started"
        );
        assert_eq!(
            assert_success(cli(&url, &["server", "stop"]), backend.name())["result"],
            "stopped"
        );
        harness.stop();

        let unavailable = bedrock_smoke::spawn(backend, false).await;
        let capabilities =
            assert_success(cli(&unavailable.url(), &["capabilities"]), backend.name());
        assert_eq!(capabilities["serverTypes"]["bedrock"]["supported"], false);
        assert!(cli(&unavailable.url(), &["server", "start"]).status.code() == Some(3));
        unavailable.stop();
    }
}
