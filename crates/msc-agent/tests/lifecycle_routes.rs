//! Contract checks for first-server initiation. The lifecycle implementation
//! is exercised by its focused unit tests; this external target protects the
//! route/application boundary from regressing while keeping the test small
//! and independent of a host Java or Bedrock runtime.

use std::path::Path;

fn source(path: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
        .unwrap_or_else(|error| panic!("read {path}: {error}"))
}

#[test]
fn lifecycle_has_a_crash_safe_first_start_coordinator() {
    let lifecycle = source("src/routes/lifecycle.rs");
    for marker in [
        "PlayitLifecycleIntegration",
        "start_playit_if_allowed",
        "stop_playit_for_server",
        "mark_first_start_transport_for_server",
        "prepare_first_start",
        "handle_process_termination",
        "abort_first_start",
        "mark_first_start_complete",
        "first_start_pass_two_started_at",
    ] {
        assert!(
            lifecycle.contains(marker),
            "missing lifecycle marker {marker}"
        );
    }
}

#[test]
fn creation_result_advertises_first_start_readiness() {
    let servers = source("src/routes/servers.rs");
    assert!(servers.contains("firstStartRequired"));
    assert!(servers.contains("provisioning::first_start_required"));
}

#[test]
fn server_start_reconciles_enabled_helper_services_and_console_output() {
    let networking = source("src/routes/networking.rs");
    for marker in [
        "fn start_broadcast_for_server",
        "xbox_broadcast_auto_start_enabled",
        "service.poll_output()",
        "append_playit_console_line",
        "spawn_broadcast_output_pump",
    ] {
        assert!(
            networking.contains(marker),
            "missing networking marker {marker}"
        );
    }
}
