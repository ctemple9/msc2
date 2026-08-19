//! P7.25 CLI structure tests, matching `cli_worlds_backups.rs`'s own
//! established pattern: clap-driven `--help`/usage-error assertions that
//! need no live agent — every command here goes through the same HTTP API
//! `provisioning_routes.rs`/`runtime_diagnostics_routes.rs` already prove
//! is really mounted and wired, so this file only proves the CLI's own
//! argument surface.

use std::process::Command as ProcessCommand;

#[test]
fn cli_provisioning_root_help_lists_new_commands() {
    let output = run_cli(&["--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    for command in ["version", "template", "java", "doctor"] {
        assert!(stdout.contains(command), "missing `{command}` in: {stdout}");
    }
}

#[test]
fn cli_provisioning_server_help_lists_new_verbs() {
    let output = run_cli(&["server", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    for verb in ["create", "delete", "rename", "eula"] {
        assert!(stdout.contains(verb), "missing `{verb}` in: {stdout}");
    }
}

#[test]
fn cli_provisioning_server_create_help_shows_every_flag() {
    let output = run_cli(&["server", "create", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("<NAME>"));
    for flag in [
        "--type",
        "--flavor",
        "--port",
        "--max-players",
        "--difficulty",
        "--gamemode",
        "--world-name",
        "--world-seed",
        "--version-id",
        "--loader-version",
        "--accept-eula",
        "--cross-play",
        "--cross-play-bedrock-port",
        "--playit",
        "--xbox-broadcast",
        "--java-path",
        "--no-wait",
    ] {
        assert!(stdout.contains(flag), "missing `{flag}` in: {stdout}");
    }
}

#[test]
fn cli_provisioning_server_delete_and_rename_help_show_positionals() {
    let delete = run_cli(&["server", "delete", "--help"], &[]);
    assert!(delete.status.success(), "stderr: {}", stderr(&delete));
    assert!(stdout(&delete).contains("<SERVER>"));

    let rename = run_cli(&["server", "rename", "--help"], &[]);
    assert!(rename.status.success(), "stderr: {}", stderr(&rename));
    let rename_stdout = stdout(&rename);
    assert!(rename_stdout.contains("<SERVER>"));
    assert!(rename_stdout.contains("<NAME>"));
}

#[test]
fn cli_provisioning_server_eula_help_shows_server_flag() {
    let output = run_cli(&["server", "eula", "--help"], &[]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("--server"));
}

#[test]
fn cli_provisioning_version_help_lists_every_verb() {
    let output = run_cli(&["version", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    for verb in ["list", "create", "set"] {
        assert!(stdout.contains(verb), "missing `{verb}` in: {stdout}");
    }
}

#[test]
fn cli_provisioning_version_set_help_shows_loader_version_and_no_wait() {
    let output = run_cli(&["version", "set", "--help"], &[]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("<VERSION_ID>"));
    assert!(stdout.contains("--loader-version"));
    assert!(stdout.contains("--no-wait"));
}

#[test]
fn cli_provisioning_template_help_lists_every_verb() {
    let output = run_cli(&["template", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    for verb in ["list", "export", "create"] {
        assert!(stdout.contains(verb), "missing `{verb}` in: {stdout}");
    }
}

#[test]
fn cli_provisioning_template_create_help_shows_template_id_and_name() {
    let output = run_cli(&["template", "create", "--help"], &[]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("<TEMPLATE_ID>"));
    assert!(stdout.contains("<NAME>"));
}

#[test]
fn cli_provisioning_java_help_lists_every_verb() {
    let output = run_cli(&["java", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    for verb in ["list", "get", "set", "install"] {
        assert!(stdout.contains(verb), "missing `{verb}` in: {stdout}");
    }
}

#[test]
fn cli_provisioning_java_install_help_shows_major_and_no_wait() {
    let output = run_cli(&["java", "install", "--help"], &[]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("<MAJOR>"));
    assert!(stdout.contains("--no-wait"));
}

#[test]
fn cli_provisioning_doctor_help_shows_repair_subcommand() {
    let output = run_cli(&["doctor", "--help"], &[]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("repair"));
}

#[test]
fn cli_provisioning_doctor_repair_help_shows_problem_id_and_action() {
    let output = run_cli(&["doctor", "repair", "--help"], &[]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("<PROBLEM_ID>"));
    assert!(stdout.contains("<ACTION>"));
}

#[test]
fn cli_provisioning_server_create_without_token_exits_with_usage_error() {
    let output = run_cli(
        &["server", "create", "Test Server"],
        &[("MSC2_CLI_TOKEN", "")],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("pass --token or set MSC2_CLI_TOKEN"));
}

fn run_cli(args: &[&str], envs: &[(&str, &str)]) -> std::process::Output {
    let mut command = ProcessCommand::new(env!("CARGO_BIN_EXE_msc"));
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().unwrap()
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}
