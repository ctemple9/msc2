//! Playit route boundary checks. These tests keep the player-address safety
//! rule from P9.7 and pin P12.20a's credential redaction and host-reset wire
//! contract without contacting playit.gg.

use msc_api::dto::{PlayitResetResultDto, PlayitSetupAcceptedDto};
use msc_domain::networking::safe_player_address;
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
        "setup_in_progress",
    ] {
        assert!(errors.contains(&json!(code)), "missing stable error {code}");
    }
}
