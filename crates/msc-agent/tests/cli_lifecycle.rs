use std::process::Command as ProcessCommand;

#[test]
fn cli_lifecycle_root_help_lists_phase4_commands() {
    let output = run_cli(&["--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("serve"));
    assert!(stdout.contains("token"));
    assert!(stdout.contains("server"));
    assert!(stdout.contains("command"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("console"));
}

#[test]
fn cli_lifecycle_server_help_lists_vertical_slice_subcommands() {
    let output = run_cli(&["server", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("import"));
    assert!(stdout.contains("rescan"));
    assert!(stdout.contains("start"));
    assert!(stdout.contains("stop"));
    assert!(stdout.contains("restart"));
}

#[test]
fn cli_lifecycle_console_tail_help_shows_line_count_flag() {
    let output = run_cli(&["console", "tail", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("--lines"));
    assert!(stdout.contains("--server"));
}

#[test]
fn cli_lifecycle_command_help_shows_server_selector() {
    let output = run_cli(&["command", "--help"], &[]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("--server"));
    assert!(stdout.contains("<TEXT>"));
}

#[test]
fn cli_lifecycle_token_print_test_reads_bootstrap_env() {
    let output = run_cli(
        &["token", "print", "--test"],
        &[("MSC2_TEST_BOOTSTRAP_TOKEN", "msc2_testid_testsecret")],
    );

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), "msc2_testid_testsecret");
}

#[test]
fn cli_lifecycle_status_without_token_exits_with_usage_error() {
    let output = run_cli(&["status"], &[("MSC2_CLI_TOKEN", "")]);

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
