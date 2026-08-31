//! Playit route boundary checks. These tests keep the player-address safety
//! rule from P9.7 and pin P12.20a's credential redaction and host-reset wire
//! contract without contacting playit.gg.

use msc_api::dto::{
    PlayitActionResultDto, PlayitResetResultDto, PlayitSetupAcceptedDto, PlayitStatusDto,
};
use msc_domain::identity::ServerType;
use msc_domain::networking::{
    PlayitTunnelKind, playit_public_address, playit_tunnel_specs, safe_player_address,
};
use serde_json::{Value, json};
use std::path::Path;

fn contract() -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/msc2/api-contract/openapi.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read openapi.json"))
        .expect("openapi.json is valid JSON")
}

#[test]
fn playit_connection_details_cannot_be_a_management_address() {
    assert_eq!(safe_player_address("127.0.0.1", Some(3000)), None);
    assert_eq!(safe_player_address("localhost", Some(3000)), None);
    assert_eq!(
        safe_player_address("join.example.joinmc.link", None).as_deref(),
        Some("join.example.joinmc.link")
    );
}

#[test]
fn tunnel_inventory_matches_java_bedrock_and_voice_rules() {
    assert_eq!(
        playit_tunnel_specs(ServerType::Java, None, false, None, false)
            .iter()
            .map(|spec| (spec.kind, spec.local_port))
            .collect::<Vec<_>>(),
        vec![(PlayitTunnelKind::Java, 25565)]
    );
    assert_eq!(
        playit_tunnel_specs(ServerType::Java, Some(25570), true, Some(19133), true)
            .iter()
            .map(|spec| (spec.kind, spec.local_port))
            .collect::<Vec<_>>(),
        vec![
            (PlayitTunnelKind::Java, 25570),
            (PlayitTunnelKind::Bedrock, 19133),
            (PlayitTunnelKind::Voice, 24454)
        ]
    );
    assert_eq!(
        playit_tunnel_specs(ServerType::Bedrock, None, false, Some(19134), false)
            .iter()
            .map(|spec| (spec.kind, spec.local_port))
            .collect::<Vec<_>>(),
        vec![(PlayitTunnelKind::Bedrock, 19134)]
    );
}

#[test]
fn udp_inventory_addresses_prefer_static_ipv4_and_keep_domain_fallback() {
    assert_eq!(
        playit_public_address(
            PlayitTunnelKind::Bedrock,
            Some("bedrock.example.joinmc.link"),
            Some("198.51.100.20"),
            Some(19132)
        )
        .as_deref(),
        Some("198.51.100.20:19132")
    );
    assert_eq!(
        playit_public_address(
            PlayitTunnelKind::Voice,
            Some("voice.example.joinmc.link"),
            None,
            Some(24454)
        )
        .as_deref(),
        Some("voice.example.joinmc.link:24454")
    );
}

#[test]
fn native_setup_is_networking_scoped_and_uses_the_shared_operation_model() {
    let setup = &contract()["paths"]["/v1/playit/setup"]["post"];
    assert_eq!(setup["x-permission-category"], "networking");
    assert_eq!(setup["operationId"], "setupPlayit");
    assert_eq!(setup["x-operation"]["type"], "playit-setup");
    assert_eq!(
        setup["x-operation"]["cancellation"],
        "POST /v1/operations/{id}/cancel; cooperative and truthful"
    );
    assert_eq!(setup["x-security-boundary"]["credentials"], "memory-only");
    assert_eq!(
        setup["x-security-boundary"]["agentKey"],
        "stored-host-scoped-and-never-returned"
    );
    assert_eq!(
        setup["x-security-boundary"]["browserApiClient"],
        "forbidden"
    );
    assert_eq!(setup["x-error-codes"].as_array().unwrap().len(), 11);
    assert_eq!(
        contract()["paths"]["/v1/playit"]["get"]["x-tunnel-inventory"]["names"]["voice"],
        "MSC Voice"
    );
}

#[test]
fn setup_and_reset_responses_contain_no_credentials_or_agent_details() {
    let setup = serde_json::to_value(PlayitSetupAcceptedDto {
        result: "setup_accepted".into(),
        operation_id: "op-playit-setup".into(),
        message: Some("Setup accepted.".into()),
    })
    .unwrap();
    let reset = serde_json::to_value(PlayitResetResultDto {
        result: "cleared".into(),
        message: Some("Cleared.".into()),
        operation_id: None,
    })
    .unwrap();
    for value in [setup, reset] {
        let object = value.as_object().expect("response object");
        for forbidden in ["email", "password", "secretKey", "sessionKey", "agentId"] {
            assert!(
                !object.contains_key(forbidden),
                "response exposed {forbidden}"
            );
        }
    }
}

#[test]
fn playit_status_and_actions_cannot_return_the_secret_key() {
    let secret = "playit-secret-must-not-leak";
    let status = serde_json::to_value(PlayitStatusDto {
        server_name: "Survival".into(),
        server_type: "java".into(),
        playit_enabled: true,
        is_running: true,
        has_secret_key: true,
        java_address: Some("join.example.joinmc.link".into()),
        bedrock_address: None,
        voice_address: None,
        voice_chat_enabled: false,
        note: Some("Waiting for Playit tunnels.".into()),
    })
    .unwrap();
    let action = serde_json::to_value(PlayitActionResultDto {
        result: "started".into(),
        message: Some("Playit tunnel start accepted.".into()),
        operation_id: Some("op-playit".into()),
    })
    .unwrap();
    for value in [status, action] {
        assert!(!value.to_string().contains(secret));
        assert!(!value.to_string().contains("secretKey"));
    }
}

#[test]
fn reset_is_host_local_idempotent_and_never_deletes_cloud_state() {
    let reset = &contract()["paths"]["/v1/playit/reset"]["post"];
    assert_eq!(reset["x-permission-category"], "networking");
    assert_eq!(reset["x-reset-contract"]["hostScoped"], true);
    assert_eq!(reset["x-reset-contract"]["idempotent"], true);
    assert_eq!(reset["x-reset-contract"]["stopsHelperBeforeClearing"], true);
    assert_eq!(
        reset["x-reset-contract"]["deletesCloudAgentsOrTunnels"],
        false
    );
    assert_eq!(reset["x-reset-contract"]["responseContainsSecrets"], false);
}

#[test]
fn native_setup_error_codes_are_stable_and_actionable_without_provider_details() {
    let document = contract();
    let errors = document["paths"]["/v1/playit/setup"]["post"]["x-error-codes"]
        .as_array()
        .expect("setup error code list");
    for code in [
        "incorrect_credentials",
        "account_banned",
        "two_factor_required",
        "rate_limited",
        "agent_not_found",
        "playit_api_error",
        "credential_store_failed",
        "playit_helper_start_failed",
        "tunnel_mismatch",
        "public_addresses_unavailable",
        "setup_in_progress",
    ] {
        assert!(errors.contains(&json!(code)), "missing stable error {code}");
    }
}

#[test]
fn first_start_uses_the_agent_owned_two_pass_lifecycle() {
    let lifecycle = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes/lifecycle.rs"),
    )
    .expect("read lifecycle routes");
    let provisioning = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../msc-application/src/provisioning.rs"),
    )
    .expect("read provisioning application");

    for marker in [
        "FirstStartCoordinator",
        "prepare_first_start",
        "handle_server_ready",
        "enforce_first_start_safety_cap",
        "firstStartPass1Complete",
        "firstStartComplete",
    ] {
        assert!(
            lifecycle.contains(marker),
            "missing lifecycle marker {marker}"
        );
    }
    assert!(provisioning.contains("has_generated_world_on_disk"));
    assert!(provisioning.contains("first_start_required"));
}

#[test]
fn networking_registers_and_polls_playit_with_the_lifecycle_owner() {
    let networking = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes/networking.rs"),
    )
    .expect("read networking routes");
    for marker in [
        "register_playit_lifecycle",
        "spawn_playit_output_pump(",
        "spawn_broadcast_output_pump(",
        "service.poll()",
        "mark_first_start_transport_for_server",
        "record_start_failure",
        "wait_for_agent_connection",
        "refresh_tunnel_addresses",
    ] {
        assert!(
            networking.contains(marker),
            "missing networking marker {marker}"
        );
    }
}

#[test]
fn reset_waits_for_helper_exit_and_serializes_account_mutation() {
    let networking = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes/networking.rs"),
    )
    .expect("read networking routes");
    for marker in [
        "playit_mutation",
        "spawn_blocking(move || reset_playit_local_state(&state))",
        "service.reset()",
        "state.secrets.delete(PLAYIT_SECRET_KEY)",
        "config.playit_agent_id = None",
        "server.svc_tunnel_prompt_dismissed = false",
    ] {
        assert!(
            networking.contains(marker),
            "missing reset boundary {marker}"
        );
    }
}

#[test]
fn playit_routes_resolve_mutations_through_the_active_server() {
    let networking = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes/networking.rs"),
    )
    .expect("read networking routes");
    for marker in [
        "fn active_server(&self) -> Result<ConfigServer, Response>",
        "let server = match state.active_server()",
        "services.get_mut(&server.id)",
    ] {
        assert!(
            networking.contains(marker),
            "missing active-server boundary {marker}"
        );
    }
}
