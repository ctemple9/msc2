//! P9.13 CLI argument-surface checks.  These deliberately invoke clap help,
//! so they prove the commands are reachable without requiring a live helper
//! or a bearer token.

use std::process::Command;

#[test]
fn phase9_root_commands_are_reachable() {
    let output = Command::new(env!("CARGO_BIN_EXE_msc"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for command in [
        "capabilities",
        "network",
        "playit",
        "broadcast",
        "resource-pack",
    ] {
        assert!(help.contains(command), "missing {command} in {help}");
    }
}

#[test]
fn phase9_subcommands_expose_machine_safe_operations() {
    for (command, expected) in [
        (vec!["network"], "connectivity"),
        (vec!["network", "duckdns"], "get"),
        (vec!["playit"], "start"),
        (vec!["broadcast"], "credentials"),
        (vec!["resource-pack"], "activate"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_msc"))
            .args(command)
            .arg("--help")
            .output()
            .unwrap();
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains(expected), "missing {expected} in {help}");
    }
}
