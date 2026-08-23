//! P10.21's scriptable Bedrock CLI argument-surface check.

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
