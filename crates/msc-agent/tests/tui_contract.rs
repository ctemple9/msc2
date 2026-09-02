#[path = "../src/cli/mod.rs"]
mod cli;

use std::ffi::OsStr;
use std::process::Command as ProcessCommand;

#[test]
fn root_dispatch_preserves_scriptable_commands_and_selects_only_a_real_tui() {
    assert_eq!(
        cli::select_invocation(true, false, false, false, None),
        cli::InvocationTarget::Command
    );
    assert_eq!(
        cli::select_invocation(false, true, true, true, Some(OsStr::new("xterm-256color"))),
        cli::InvocationTarget::Usage
    );
    assert_eq!(
        cli::select_invocation(false, false, false, true, Some(OsStr::new("xterm"))),
        cli::InvocationTarget::Usage
    );
    assert_eq!(
        cli::select_invocation(false, false, true, true, Some(OsStr::new("xterm-256color"))),
        cli::InvocationTarget::Tui
    );
    assert_eq!(
        cli::select_invocation(false, false, true, true, Some(OsStr::new("dumb"))),
        cli::InvocationTarget::Usage
    );
}

#[test]
fn tui_placeholder_does_not_emit_terminal_control_sequences() {
    let error = cli::run_tui(cli::CommonArgs {
        base_url: None,
        host: "127.0.0.1".to_string(),
        port: 48001,
        token: None,
        json: false,
    })
    .expect_err("P13.1 reserves the TUI seam but does not enter terminal mode");

    assert_eq!(error.exit_code(), 1);
}

#[test]
fn binary_help_and_bare_script_invocations_remain_conventional() {
    let help = run_cli(&["--help"]);
    assert!(help.status.success(), "help stderr: {}", stderr(&help));
    assert!(stdout(&help).contains("status"));
    assert_no_terminal_control_bytes(&help);

    let bare = run_cli(&[]);
    assert_eq!(bare.status.code(), Some(2));
    assert_no_terminal_control_bytes(&bare);

    let json = run_cli(&["--json"]);
    assert_eq!(json.status.code(), Some(2));
    assert_no_terminal_control_bytes(&json);
}

fn run_cli(args: &[&str]) -> std::process::Output {
    ProcessCommand::new(env!("CARGO_BIN_EXE_msc"))
        .args(args)
        .env_remove("MSC2_CLI_TOKEN")
        .env_remove("MSC2_TEST_BOOTSTRAP_TOKEN")
        .output()
        .expect("CLI process runs")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("CLI stdout is UTF-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("CLI stderr is UTF-8")
}

fn assert_no_terminal_control_bytes(output: &std::process::Output) {
    assert!(!output.stdout.contains(&0x1b));
    assert!(!output.stderr.contains(&0x1b));
}
