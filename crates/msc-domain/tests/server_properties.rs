//! Port of `ServerPropertiesModelTests.swift`'s 7 fixtures.
//!
//! Test functions are prefixed `server_properties_` so the plan's Verify
//! command (a plain nextest substring filter, which matches on test name,
//! not file/binary name) selects all of them.

mod support;

use msc_domain::properties::{LevelType, ServerPropertiesModel};
use serde_json::Value;
use std::collections::HashMap;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("server-properties/{case}.json")))
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

/// Applies the fixture's `mutations` object (typed-property assignments made
/// on the model after construction, before `mergedInto` is called) by field
/// name — distinct from `apply_java`'s wire-key validation, since these are
/// direct, always-valid assignments in the source test.
fn apply_mutations(model: &mut ServerPropertiesModel, mutations: &Value) {
    let obj = mutations.as_object().expect("mutations object");
    for (key, value) in obj {
        match key.as_str() {
            "motd" => model.motd = value.as_str().expect("motd string").to_string(),
            "maxPlayers" => model.max_players = value.as_i64().expect("maxPlayers int"),
            "pvp" => model.pvp = value.as_bool().expect("pvp bool"),
            "onlineMode" => model.online_mode = value.as_bool().expect("onlineMode bool"),
            "levelType" => {
                model.level_type = match value.as_str().expect("levelType string") {
                    "normal" => LevelType::Normal,
                    "flat" => LevelType::Flat,
                    "largeBiomes" => LevelType::LargeBiomes,
                    "amplified" => LevelType::Amplified,
                    other => panic!("unknown levelType case name: {other}"),
                };
            }
            other => panic!("unhandled mutation key: {other}"),
        }
    }
}

fn resolve_target(fixture: &Fixture, model: &ServerPropertiesModel) -> HashMap<String, String> {
    if fixture
        .input
        .get("targetIsRawProperties")
        .and_then(Value::as_bool)
        == Some(true)
    {
        model.raw_properties.clone()
    } else {
        dict_from_value(&fixture.input["target"])
    }
}

/// Only asserts the keys present in `expected` — `mergedInto`'s result
/// includes every known field, but most fixtures here only pin the subset
/// their source test actually checked.
fn assert_subset(actual: &HashMap<String, String>, expected: &Value) {
    let obj = expected.as_object().expect("expected object");
    for (key, value) in obj {
        let expected_str = value.as_str().expect("expected string value");
        assert_eq!(
            actual.get(key).map(String::as_str),
            Some(expected_str),
            "key {key}"
        );
    }
}

fn run_merged_into_case(case: &str) {
    let fixture = load(case);
    let from = dict_from_value(&fixture.input["from"]);
    let mut model = ServerPropertiesModel::from_dict(&from, None);
    apply_mutations(&mut model, &fixture.input["mutations"]);
    let target = resolve_target(&fixture, &model);
    let result = model.merged_into(&target);
    assert_subset(&result, &fixture.expected);
}

#[test]
fn server_properties_init_from_dict_applies_defaults_for_missing_keys() {
    let fixture = load("init-from-dict-applies-defaults-for-missing-keys");
    let from = dict_from_value(&fixture.input["from"]);
    let model = ServerPropertiesModel::from_dict(&from, None);
    assert_eq!(
        model.max_players,
        fixture.expected["maxPlayers"].as_i64().unwrap()
    );
    assert_eq!(
        model.server_port,
        fixture.expected["serverPort"].as_i64().unwrap()
    );
    assert_eq!(
        model.online_mode,
        fixture.expected["onlineMode"].as_bool().unwrap()
    );
    assert_eq!(
        model.difficulty.raw_value(),
        fixture.expected["difficulty"].as_str().unwrap()
    );
    assert_eq!(
        model.gamemode.raw_value(),
        fixture.expected["gamemode"].as_str().unwrap()
    );
    assert_eq!(
        model.op_permission_level,
        fixture.expected["opPermissionLevel"].as_i64().unwrap()
    );
}

#[test]
fn server_properties_level_type_legacy_all_caps_parsing() {
    let fixture = load("level-type-legacy-all-caps-parsing");
    let cases = fixture.input["cases"].as_array().unwrap();
    let results = fixture.expected["results"].as_array().unwrap();
    for (case, expected) in cases.iter().zip(results) {
        let from = dict_from_value(&case["from"]);
        let model = ServerPropertiesModel::from_dict(&from, None);
        assert_eq!(
            level_type_case_name(model.level_type),
            expected["levelType"].as_str().unwrap()
        );
    }
}

#[test]
fn server_properties_known_keys_are_overlaid_from_model() {
    run_merged_into_case("known-keys-are-overlaid-from-model");
}

#[test]
fn server_properties_merged_into_preserves_unknown_keys_from_arbitrary_target_dict() {
    run_merged_into_case("merged-into-preserves-unknown-keys-from-arbitrary-target-dict");
}

#[test]
fn server_properties_merged_into_writes_canonical_bool_strings() {
    run_merged_into_case("merged-into-writes-canonical-bool-strings");
}

#[test]
fn server_properties_merged_into_writes_escaped_level_type_raw_value() {
    run_merged_into_case("merged-into-writes-escaped-level-type-raw-value");
}

#[test]
fn server_properties_unknown_keys_survive_round_trip() {
    run_merged_into_case("unknown-keys-survive-round-trip");
}
