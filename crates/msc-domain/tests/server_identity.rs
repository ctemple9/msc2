//! Port of `fixtures/server-identity/`'s 16 fixtures — a new-characterization
//! domain (no MSC 1 test file) covering `JavaServerFlavor`'s per-flavor
//! property bundle, the `tpsPollCommand`/`supportsVanillaTickQuery`
//! 1.20.3 threshold, and `createFlowChoices`.
//!
//! Test functions are prefixed `server_identity_` so the plan's Verify
//! command (a plain nextest substring filter, which matches on test name,
//! not file/binary name) selects all of them.

mod support;

use msc_domain::identity::{JavaServerCategory, JavaServerFlavor};
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("server-identity/{case}.json")))
}

fn add_on_kind_raw(flavor: JavaServerFlavor) -> Option<&'static str> {
    flavor.add_on_kind().map(|k| k.raw_value())
}

fn assert_property_bundle(case: &str) {
    let fixture = load(case);
    let flavor = JavaServerFlavor::from_raw_value(fixture.input["flavor"].as_str().unwrap())
        .expect("known flavor");
    let expected = &fixture.expected;

    assert_eq!(
        flavor.category().raw_value(),
        expected["category"].as_str().unwrap()
    );
    assert_eq!(
        flavor.is_forge_family(),
        expected["isForgeFamily"].as_bool().unwrap()
    );
    assert_eq!(add_on_kind_raw(flavor), expected["addOnKind"].as_str());
    assert_eq!(
        flavor.provisioning_kind().raw_value(),
        expected["provisioningKind"].as_str().unwrap()
    );
    assert_eq!(
        flavor.modrinth_project_type(),
        expected["modrinthProjectType"].as_str().unwrap()
    );
    let expected_facets: Vec<&str> = expected["modrinthLoaderFacets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(flavor.modrinth_loader_facets(), expected_facets.as_slice());
    assert_eq!(
        flavor.auto_tps_command(),
        expected["autoTpsCommand"].as_str()
    );
    assert_eq!(
        flavor.is_recommended(),
        expected["isRecommended"].as_bool().unwrap()
    );
    assert_eq!(
        flavor.is_available_in_create_flow(),
        expected["isAvailableInCreateFlow"].as_bool().unwrap()
    );
}

#[test]
fn server_identity_paper_property_bundle() {
    assert_property_bundle("paper-property-bundle");
}

#[test]
fn server_identity_purpur_property_bundle() {
    assert_property_bundle("purpur-property-bundle");
}

#[test]
fn server_identity_pufferfish_property_bundle() {
    assert_property_bundle("pufferfish-property-bundle");
}

#[test]
fn server_identity_vanilla_property_bundle() {
    assert_property_bundle("vanilla-property-bundle");
}

#[test]
fn server_identity_fabric_property_bundle() {
    assert_property_bundle("fabric-property-bundle");
}

#[test]
fn server_identity_neoforge_property_bundle() {
    assert_property_bundle("neoforge-property-bundle");
}

#[test]
fn server_identity_spigot_property_bundle() {
    assert_property_bundle("spigot-property-bundle");
}

#[test]
fn server_identity_forge_property_bundle() {
    assert_property_bundle("forge-property-bundle");
}

#[test]
fn server_identity_quilt_property_bundle() {
    assert_property_bundle("quilt-property-bundle");
}

fn assert_tps_poll_command_case(case: &str) {
    let fixture = load(case);
    let minecraft_version = fixture.input["minecraftVersion"].as_str();
    let expected = &fixture.expected;

    assert_eq!(
        msc_domain::identity::supports_vanilla_tick_query(minecraft_version),
        expected["supportsVanillaTickQuery"].as_bool().unwrap()
    );

    let expected_commands = expected["tpsPollCommand"].as_object().unwrap();
    for (flavor_raw, expected_command) in expected_commands {
        let flavor = JavaServerFlavor::from_raw_value(flavor_raw).expect("known flavor");
        assert_eq!(
            flavor.tps_poll_command(minecraft_version),
            expected_command.as_str(),
            "flavor {flavor_raw}"
        );
    }
}

#[test]
fn server_identity_tps_poll_command_below_threshold() {
    assert_tps_poll_command_case("tps-poll-command-below-threshold");
}

#[test]
fn server_identity_tps_poll_command_at_threshold() {
    assert_tps_poll_command_case("tps-poll-command-at-threshold");
}

#[test]
fn server_identity_tps_poll_command_above_threshold() {
    assert_tps_poll_command_case("tps-poll-command-above-threshold");
}

#[test]
fn server_identity_tps_poll_command_nil_version() {
    assert_tps_poll_command_case("tps-poll-command-nil-version");
}

#[test]
fn server_identity_tps_poll_command_empty_version() {
    assert_tps_poll_command_case("tps-poll-command-empty-version");
}

fn assert_create_flow_choices_case(case: &str) {
    let fixture = load(case);
    let category = match fixture.input["category"].as_str().unwrap() {
        "standard" => JavaServerCategory::Standard,
        "modded" => JavaServerCategory::Modded,
        other => panic!("unhandled category: {other}"),
    };
    let expected: Vec<&str> = fixture.expected["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let actual: Vec<&str> = JavaServerFlavor::create_flow_choices(category)
        .into_iter()
        .map(|f| f.raw_value())
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn server_identity_create_flow_choices_standard() {
    assert_create_flow_choices_case("create-flow-choices-standard");
}

#[test]
fn server_identity_create_flow_choices_modded() {
    assert_create_flow_choices_case("create-flow-choices-modded");
}
