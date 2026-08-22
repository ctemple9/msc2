//! Fixture-backed coverage for P9.5's side-effect-free helper lifecycle rules.

mod support;

use msc_domain::helper::*;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("helper-lifecycle/{case}.json")))
}

#[test]
fn helper_first_run_broadcast_timeout_is_sixty_seconds() {
    let fixture = load("first-run-broadcast-technical-timeout-is-60-seconds");
    assert_eq!(
        first_run_timeout(
            FirstRunTransport::Broadcast,
            fixture.input["broadcastNotReadyAtSeconds"]
                .as_u64()
                .unwrap()
        ),
        Some(HelperStatus::TimedOut)
    );
}

#[test]
fn helper_first_run_second_pass_waits_for_enabled_transports() {
    let fixture = load("first-run-pass-two-awaits-transports");
    assert_eq!(
        first_run_needs_second_pass(
            fixture.input["playitEnabled"].as_bool().unwrap(),
            fixture.input["broadcastEnabled"].as_bool().unwrap(),
            fixture.input["serverReady"].as_bool().unwrap()
        ),
        fixture.expected["startSecondPass"]
    );
}

#[test]
fn helper_first_run_playit_timeout_is_seventy_five_seconds() {
    let fixture = load("first-run-playit-watchdog-is-75-seconds");
    assert_eq!(
        first_run_timeout(
            FirstRunTransport::Playit,
            fixture.input["playitNotReadyAtSeconds"].as_u64().unwrap()
        ),
        Some(HelperStatus::TimedOut)
    );
}

#[test]
fn helper_first_run_safety_cap_is_ten_minutes() {
    let fixture = load("first-run-safety-cap-is-ten-minutes");
    assert!(first_run_safety_cap_reached(
        fixture.input["workflowUnresolvedAtSeconds"]
            .as_u64()
            .unwrap()
    ));
}

#[test]
fn helper_exit_clears_running_state_and_address() {
    let fixture = load("playit-exit-clears-running-and-address");
    let exited = HelperSnapshot {
        status: HelperStatus::Running,
        player_address: Some(fixture.input["tunnelAddress"].as_str().unwrap().to_owned()),
    }
    .on_exit();
    assert_eq!(exited.status, HelperStatus::Stopped);
    assert_eq!(exited.player_address, None);
}

#[test]
fn helper_restart_never_claims_a_leftover_pid_is_running() {
    let fixture = load("restart-reconciles-helper-state-honestly");
    assert_eq!(
        HelperSnapshot::after_agent_restart().status,
        HelperStatus::UnknownUntilReconciled
    );
    assert_eq!(fixture.expected["mustNotReport"], "running");
}

#[test]
fn helper_start_with_secret_launches() {
    let fixture = load("start-downloads-writes-secret-and-launches");
    assert_eq!(
        decide_playit_start(
            fixture.input["enabled"].as_bool().unwrap(),
            fixture.input["secretPresent"].as_bool().unwrap()
        ),
        HelperStartDecision::Launch
    );
}

#[test]
fn helper_stop_clears_state() {
    let fixture = load("stop-terminates-running-helper");
    assert_eq!(
        HelperSnapshot::stopped(),
        HelperSnapshot {
            status: HelperStatus::Stopped,
            player_address: None
        }
    );
    assert_eq!(fixture.expected["terminate"], true);
}
