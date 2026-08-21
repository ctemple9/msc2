//! P8.25 CLI structure tests: help/usage coverage for the new add-on and
//! modpack command surface, without a live agent.

use std::process::Command as ProcessCommand;

#[test]
fn cli_phase8_root_help_lists_addon_and_modpack_commands() {
    let output = run_cli(&["--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("addon"));
    assert!(stdout.contains("modpack"));
}

#[test]
fn cli_phase8_addon_help_lists_every_phase8_verb() {
    let output = run_cli(&["addon", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    for verb in [
        "list",
        "search",
        "install-catalog",
        "install-local",
        "update",
        "update-all",
        "enable",
        "disable",
        "remove",
        "link",
        "set-source",
        "remove-source",
        "export",
    ] {
        assert!(stdout.contains(verb), "missing `{verb}` in: {stdout}");
    }
}

#[test]
fn cli_phase8_modpack_help_lists_every_phase8_verb() {
    let output = run_cli(&["modpack", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    for verb in ["inspect", "import", "replace", "manual-file"] {
        assert!(stdout.contains(verb), "missing `{verb}` in: {stdout}");
    }
}

#[test]
fn cli_phase8_server_create_help_shows_modpack_flag() {
    let output = run_cli(&["server", "create", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("--modpack"));
}

#[test]
fn cli_phase8_addon_export_help_shows_selected_and_output_flags() {
    let output = run_cli(&["addon", "export", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("--selected"));
    assert!(stdout.contains("--output"));
}

#[test]
fn cli_phase8_addon_install_local_missing_file_exits_with_usage_error() {
    let output = run_cli(
        &[
            "addon",
            "install-local",
            "/nonexistent/path/does-not-exist.jar",
        ],
        &[("MSC2_CLI_TOKEN", "msc2_testid_testsecret")],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("failed to read"), "{}", stderr(&output));
}

#[test]
fn cli_phase8_modpack_inspect_missing_file_exits_with_usage_error() {
    let output = run_cli(
        &["modpack", "inspect", "/nonexistent/path/does-not-exist.mrpack"],
        &[("MSC2_CLI_TOKEN", "msc2_testid_testsecret")],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("failed to read"), "{}", stderr(&output));
}

#[test]
fn cli_phase8_doctor_repair_help_mentions_update_and_install() {
    let output = run_cli(&["doctor", "repair", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("update"));
    assert!(stdout.contains("install"));
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
