//! Port of `ServerSettingsSchemaTests.swift`'s 16 fixtures.
//!
//! Test functions are prefixed `settings_schema_` so the plan's Verify
//! command (a plain nextest substring filter, which matches on test name,
//! not file/binary name) selects all of them.

mod support;

use msc_domain::properties::{LevelType, ServerPropertiesModel};
use msc_domain::settings_schema::{ApplyResult, apply_java, level_from_token, level_token};
use serde_json::Value;
use std::collections::HashMap;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("settings-schema/{case}.json")))
}

fn dict_from_value(v: &Value) -> HashMap<String, String> {
    v.as_object()
        .expect("object")
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().expect("string value").to_string()))
        .collect()
}

fn level_type_case_name(t: LevelType) -> &'static str {
    match t {
        LevelType::Normal => "normal",
        LevelType::Flat => "flat",
        LevelType::LargeBiomes => "largeBiomes",
        LevelType::Amplified => "amplified",
    }
}

fn level_type_from_case_name(s: &str) -> LevelType {
    match s {
        "normal" => LevelType::Normal,
        "flat" => LevelType::Flat,
        "largeBiomes" => LevelType::LargeBiomes,
        "amplified" => LevelType::Amplified,
        other => panic!("unknown levelType case name: {other}"),
    }
}

fn baseline_model(fixture: &Fixture) -> ServerPropertiesModel {
    let baseline = &fixture.input["baseline"];
    let from = dict_from_value(&baseline["from"]);
    let fallback_motd = baseline["fallbackMotd"].as_str();
    ServerPropertiesModel::from_dict(&from, fallback_motd)
}

fn changes_from(value: &Value) -> HashMap<String, String> {
    dict_from_value(&value["changes"])
}

fn run_apply_java(case: &str) -> (Fixture, ServerPropertiesModel, ApplyResult) {
    let fixture = load(case);
    let mut model = baseline_model(&fixture);
    let changes = changes_from(&fixture.input);
    let result = apply_java(&changes, &mut model);
    (fixture, model, result)
}

fn assert_applied(result: &ApplyResult, expected: &Value) {
    let expected: Vec<String> = expected
        .as_array()
        .expect("applied array")
        .iter()
        .map(|v| v.as_str().expect("applied entry").to_string())
        .collect();
    assert_eq!(result.applied, expected);
}

fn assert_rejected_reasons_only(result: &ApplyResult, expected: &Value) {
    let expected = expected.as_array().expect("rejected array");
    assert_eq!(result.rejected.len(), expected.len());
    for (actual, exp) in result.rejected.iter().zip(expected) {
        assert_eq!(actual.reason, exp["reason"].as_str().expect("reason"));
        if let Some(key) = exp.get("key").and_then(Value::as_str) {
            assert_eq!(actual.key, key);
        }
    }
}

#[test]
fn settings_schema_apply_level_type_uses_wire_token_and_sets_escaped_raw_value() {
    let (fixture, model, result) =
        run_apply_java("apply-level-type-uses-wire-token-and-sets-escaped-raw-value");
    assert_applied(&result, &fixture.expected["applied"]);
    assert_eq!(
        level_type_case_name(model.level_type),
        fixture.expected["model"]["levelType"].as_str().unwrap()
    );
    assert_eq!(
        model.level_type.raw_value(),
        fixture.expected["model"]["levelTypeRawValue"]
            .as_str()
            .unwrap()
    );
}

#[test]
fn settings_schema_bool_accepts_synonyms() {
    let fixture = load("bool-accepts-synonyms");
    let cases = fixture.input["cases"].as_array().unwrap();
    let results = fixture.expected["results"].as_array().unwrap();
    for (case, expected) in cases.iter().zip(results) {
        let mut model = baseline_model(&fixture);
        let changes = changes_from(case);
        let result = apply_java(&changes, &mut model);
        assert_applied(&result, &expected["applied"]);
        assert_eq!(model.pvp, expected["model"]["pvp"].as_bool().unwrap());
    }
}

#[test]
fn settings_schema_bool_rejects_garbage() {
    let (fixture, _model, result) = run_apply_java("bool-rejects-garbage");
    assert!(result.applied.is_empty());
    assert_rejected_reasons_only(&result, &fixture.expected["rejected"]);
}

#[test]
fn settings_schema_difficulty_enum_applied_and_case_insensitive() {
    let (fixture, model, result) = run_apply_java("difficulty-enum-applied-and-case-insensitive");
    assert_applied(&result, &fixture.expected["applied"]);
    assert_eq!(
        model.difficulty.raw_value(),
        fixture.expected["model"]["difficulty"].as_str().unwrap()
    );
}

#[test]
fn settings_schema_int_clamps_above_max() {
    let (fixture, model, result) = run_apply_java("int-clamps-above-max");
    assert_applied(&result, &fixture.expected["applied"]);
    assert_eq!(
        model.max_players,
        fixture.expected["model"]["maxPlayers"].as_i64().unwrap()
    );
    assert!(result.rejected.is_empty());
}

#[test]
fn settings_schema_int_clamps_below_min() {
    let (fixture, model, result) = run_apply_java("int-clamps-below-min");
    assert_applied(&result, &fixture.expected["applied"]);
    assert_eq!(
        model.view_distance,
        fixture.expected["model"]["viewDistance"].as_i64().unwrap()
    );
}

#[test]
fn settings_schema_int_within_range_applied_verbatim() {
    let (fixture, model, result) = run_apply_java("int-within-range-applied-verbatim");
    assert_applied(&result, &fixture.expected["applied"]);
    assert_eq!(
        model.server_port,
        fixture.expected["model"]["serverPort"].as_i64().unwrap()
    );
}

#[test]
fn settings_schema_invalid_enum_rejected() {
    let (fixture, _model, result) = run_apply_java("invalid-enum-rejected");
    assert!(result.applied.is_empty());
    assert_rejected_reasons_only(&result, &fixture.expected["rejected"]);
}

#[test]
fn settings_schema_level_from_token_rejects_unknown() {
    let fixture = load("level-from-token-rejects-unknown");
    let token = fixture.input["token"].as_str().unwrap();
    assert!(level_from_token(token).is_none());
    assert!(fixture.expected.is_null());
}

#[test]
fn settings_schema_level_from_token_round_trips() {
    let fixture = load("level-from-token-round-trips");
    let level_types = fixture.input["levelTypes"].as_array().unwrap();
    let expected = fixture.expected["results"].as_array().unwrap();
    for (lt, exp) in level_types.iter().zip(expected) {
        let level_type = level_type_from_case_name(lt.as_str().unwrap());
        let token = level_token(level_type);
        let round_tripped = level_from_token(token).expect("round trip token should resolve");
        assert_eq!(level_type_case_name(round_tripped), exp.as_str().unwrap());
    }
}

#[test]
fn settings_schema_level_token_maps_large_biomes_to_underscore_form() {
    let fixture = load("level-token-maps-large-biomes-to-underscore-form");
    let level_types = fixture.input["levelTypes"].as_array().unwrap();
    let expected = fixture.expected["results"].as_array().unwrap();
    for (lt, exp) in level_types.iter().zip(expected) {
        let level_type = level_type_from_case_name(lt.as_str().unwrap());
        assert_eq!(level_token(level_type), exp.as_str().unwrap());
    }
}

#[test]
fn settings_schema_mixed_batch_partitions_applied_and_rejected() {
    let (fixture, model, result) = run_apply_java("mixed-batch-partitions-applied-and-rejected");
    let applied_contains = fixture.expected["appliedContains"].as_array().unwrap();
    for key in applied_contains {
        assert!(result.applied.contains(&key.as_str().unwrap().to_string()));
    }
    assert_eq!(
        model.pvp,
        fixture.expected["model"]["pvp"].as_bool().unwrap()
    );
    let rejected_reasons = fixture.expected["rejectedReasons"].as_object().unwrap();
    for (key, reason) in rejected_reasons {
        let reason = reason.as_str().unwrap();
        assert!(
            result
                .rejected
                .iter()
                .any(|r| &r.key == key && r.reason == reason),
            "expected rejection for key {key} with reason {reason}, got {:?}",
            result.rejected
        );
    }
}

#[test]
fn settings_schema_motd_truncated_to_200_chars() {
    let (fixture, model, result) = run_apply_java("motd-truncated-to-200-chars");
    assert_applied(&result, &fixture.expected["applied"]);
    assert_eq!(
        model.motd.chars().count() as i64,
        fixture.expected["model"]["motdLength"].as_i64().unwrap()
    );
}

#[test]
fn settings_schema_non_integer_int_rejected() {
    let (fixture, model, result) = run_apply_java("non-integer-int-rejected");
    assert!(result.applied.is_empty());
    assert_eq!(
        model.max_players,
        fixture.expected["model"]["maxPlayers"].as_i64().unwrap()
    );
    assert_rejected_reasons_only(&result, &fixture.expected["rejected"]);
}

#[test]
fn settings_schema_op_permission_level_bounds_enforced() {
    let fixture = load("op-permission-level-bounds-enforced");
    let cases = fixture.input["cases"].as_array().unwrap();
    let results = fixture.expected["results"].as_array().unwrap();
    for (case, expected) in cases.iter().zip(results) {
        let mut model = baseline_model(&fixture);
        let changes = changes_from(case);
        let result = apply_java(&changes, &mut model);
        if let Some(applied) = expected.get("applied") {
            assert_applied(&result, applied);
        }
        if let Some(v) = expected
            .get("model")
            .and_then(|m| m.get("opPermissionLevel"))
        {
            assert_eq!(model.op_permission_level, v.as_i64().unwrap());
        }
        if let Some(rejected) = expected.get("rejected") {
            assert_rejected_reasons_only(&result, rejected);
        }
    }
}

#[test]
fn settings_schema_unknown_key_rejected() {
    let (fixture, _model, result) = run_apply_java("unknown-key-rejected");
    assert!(result.applied.is_empty());
    assert_rejected_reasons_only(&result, &fixture.expected["rejected"]);
}
