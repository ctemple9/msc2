//! P6.22 CLI structure tests, matching `cli_lifecycle.rs`'s own
//! established pattern: clap-driven `--help`/usage-error assertions that
//! need no live agent, not a live round trip (P6.21's inline
//! `world_backup_routes_*` tests and this repo's own `world_backup_routes.rs`
//! already cover the real request/response wiring these commands call
//! into).

use std::process::Command as ProcessCommand;

#[test]
fn cli_worlds_backups_root_help_lists_world_and_backup_commands() {
    let output = run_cli(&["--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("world"));
    assert!(stdout.contains("backup"));
}

#[test]
fn cli_worlds_backups_world_help_lists_every_verb() {
    let output = run_cli(&["world", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    for verb in [
        "list",
        "create",
        "rename",
        "delete",
        "duplicate",
        "copy",
        "import",
        "export",
        "activate",
        "convert",
    ] {
        assert!(stdout.contains(verb), "missing `{verb}` in: {stdout}");
    }
}

#[test]
fn cli_worlds_backups_backup_help_lists_every_verb() {
    let output = run_cli(&["backup", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    for verb in ["list", "now", "delete", "restore", "config"] {
        assert!(stdout.contains(verb), "missing `{verb}` in: {stdout}");
    }
}

#[test]
fn cli_worlds_backups_backup_config_help_lists_get_and_set() {
    let output = run_cli(&["backup", "config", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("get"));
    assert!(stdout.contains("set"));
}

#[test]
fn cli_worlds_backups_world_copy_help_shows_into_and_from() {
    let output = run_cli(&["world", "copy", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("--into"));
    assert!(stdout.contains("--from"));
}

#[test]
fn cli_worlds_backups_world_import_help_shows_path_and_name() {
    let output = run_cli(&["world", "import", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("<PATH>"));
    assert!(stdout.contains("<NAME>"));
}

#[test]
fn cli_worlds_backups_world_export_help_shows_output_flag() {
    let output = run_cli(&["world", "export", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("--output"));
}

#[test]
fn cli_worlds_backups_world_activate_help_shows_no_wait_flag() {
    let output = run_cli(&["world", "activate", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("--no-wait"));
}

#[test]
fn cli_worlds_backups_backup_now_help_shows_no_wait_flag() {
    let output = run_cli(&["backup", "now", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("--no-wait"));
}

#[test]
fn cli_worlds_backups_backup_restore_help_shows_no_wait_flag() {
    let output = run_cli(&["backup", "restore", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("--no-wait"));
}

#[test]
fn cli_worlds_backups_world_convert_help_shows_required_target_flags() {
    let output = run_cli(&["world", "convert", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("--target-server"));
    assert!(stdout.contains("--target-format"));
    assert!(stdout.contains("--target-name"));
    assert!(stdout.contains("--target-slot"));
}

#[test]
fn cli_worlds_backups_world_convert_rejects_both_target_name_and_target_slot() {
    let output = run_cli(
        &[
            "world",
            "convert",
            "some-slot",
            "--target-server",
            "some-server",
            "--target-format",
            "bedrock",
            "--target-name",
            "New World",
            "--target-slot",
            "some-slot-id",
        ],
        &[("MSC2_CLI_TOKEN", "msc2_testid_testsecret")],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("exactly one of --target-name or --target-slot"),
        "stderr: {}",
        stderr(&output)
    );
}

#[test]
fn cli_worlds_backups_world_convert_rejects_neither_target_name_nor_target_slot() {
    let output = run_cli(
        &[
            "world",
            "convert",
            "some-slot",
            "--target-server",
            "some-server",
            "--target-format",
            "bedrock",
        ],
        &[("MSC2_CLI_TOKEN", "msc2_testid_testsecret")],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("exactly one of --target-name or --target-slot"));
}

#[test]
fn cli_worlds_backups_backup_config_set_rejects_no_fields() {
    let output = run_cli(
        &["backup", "config", "set"],
        &[("MSC2_CLI_TOKEN", "msc2_testid_testsecret")],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("at least one of"));
}

#[test]
fn cli_worlds_backups_world_list_without_token_exits_with_usage_error() {
    let output = run_cli(&["world", "list"], &[("MSC2_CLI_TOKEN", "")]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("pass --token or set MSC2_CLI_TOKEN"));
}

#[test]
fn cli_worlds_backups_world_import_missing_file_exits_with_usage_error() {
    let output = run_cli(
        &[
            "world",
            "import",
            "/nonexistent/path/does-not-exist.zip",
            "Imported",
        ],
        &[("MSC2_CLI_TOKEN", "msc2_testid_testsecret")],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("failed to read"),
        "{}",
        stderr(&output)
    );
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
