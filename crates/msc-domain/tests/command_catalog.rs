//! Port of `fixtures/command-catalog/`'s 18 fixtures — a new-characterization
//! domain (no MSC 1 test file) covering `MinecraftCommandRegistry.commands(for:)`'s
//! Java/Bedrock filter and the `suggestions(for:serverType:onlinePlayers:)`
//! autocomplete engine.
//!
//! Test functions are prefixed `command_catalog_` so the plan's Verify
//! command (a plain nextest substring filter, matching on test name, not
//! file/binary name) selects all of them.

mod support;

use msc_domain::commands::{self, OnlinePlayer};
use msc_domain::identity::ServerType;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("command-catalog/{case}.json")))
}

fn server_type_from(raw: &str) -> ServerType {
    match raw {
        "java" => ServerType::Java,
        "bedrock" => ServerType::Bedrock,
        other => panic!("unhandled serverType: {other}"),
    }
}

fn assert_commands_for_case(case: &str) {
    let fixture = load(case);
    let server_type = server_type_from(fixture.input["serverType"].as_str().unwrap());
    let expected: Vec<&str> = fixture.expected["names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let actual: Vec<&str> = commands::commands_for(server_type)
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn command_catalog_commands_for_java_server_type() {
    assert_commands_for_case("commands-for-java-server-type");
}

#[test]
fn command_catalog_commands_for_bedrock_server_type() {
    assert_commands_for_case("commands-for-bedrock-server-type");
}

fn assert_suggestions_case(case: &str) {
    let fixture = load(case);
    let input = fixture.input["input"].as_str().unwrap();
    let server_type = server_type_from(fixture.input["serverType"].as_str().unwrap());
    let online_players: Vec<OnlinePlayer> = fixture.input["onlinePlayers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| OnlinePlayer {
            name: v.as_str().unwrap().to_string(),
        })
        .collect();
    let expected: Vec<&str> = fixture.expected["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let actual = commands::suggestions(input, server_type, &online_players);
    assert_eq!(actual, expected);
}

#[test]
fn command_catalog_suggestions_empty_input() {
    assert_suggestions_case("suggestions-empty-input");
}

#[test]
fn command_catalog_suggestions_command_name_prefix() {
    assert_suggestions_case("suggestions-command-name-prefix");
}

#[test]
fn command_catalog_suggestions_command_name_prefix_leading_slash() {
    assert_suggestions_case("suggestions-command-name-prefix-leading-slash");
}

#[test]
fn command_catalog_suggestions_command_name_prefix_case_insensitive() {
    assert_suggestions_case("suggestions-command-name-prefix-case-insensitive");
}

#[test]
fn command_catalog_suggestions_command_name_prefix_cap_at_six() {
    assert_suggestions_case("suggestions-command-name-prefix-cap-at-six");
}

#[test]
fn command_catalog_suggestions_command_name_prefix_no_match() {
    assert_suggestions_case("suggestions-command-name-prefix-no-match");
}

#[test]
fn command_catalog_suggestions_command_name_prefix_respects_server_type() {
    assert_suggestions_case("suggestions-command-name-prefix-respects-server-type");
}

#[test]
fn command_catalog_suggestions_unknown_command_name() {
    assert_suggestions_case("suggestions-unknown-command-name");
}

#[test]
fn command_catalog_suggestions_space_after_single_slot_command_out_of_range() {
    assert_suggestions_case("suggestions-space-after-single-slot-command-out-of-range");
}

#[test]
fn command_catalog_suggestions_space_after_command_skips_to_slot_one() {
    assert_suggestions_case("suggestions-space-after-command-skips-to-slot-one");
}

#[test]
fn command_catalog_suggestions_player_name_partial_prefix() {
    assert_suggestions_case("suggestions-player-name-partial-prefix");
}

#[test]
fn command_catalog_suggestions_player_name_case_insensitive_partial() {
    assert_suggestions_case("suggestions-player-name-case-insensitive-partial");
}

#[test]
fn command_catalog_suggestions_player_name_slot_out_of_range_after_full_arg() {
    assert_suggestions_case("suggestions-player-name-slot-out-of-range-after-full-arg");
}

#[test]
fn command_catalog_suggestions_keyword_partial() {
    assert_suggestions_case("suggestions-keyword-partial");
}

#[test]
fn command_catalog_suggestions_player_name_cap_at_six() {
    assert_suggestions_case("suggestions-player-name-cap-at-six");
}

#[test]
fn command_catalog_suggestions_non_suggestable_slot_type_returns_empty() {
    assert_suggestions_case("suggestions-non-suggestable-slot-type-returns-empty");
}
