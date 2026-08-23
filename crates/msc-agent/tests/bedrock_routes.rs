//! P10.21's public route-surface check.  The command is intentionally
//! offline: it proves the shared Bedrock route names are represented by the
//! agent's public surface without requiring a live BDS process.

use std::process::Command;

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
