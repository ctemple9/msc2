//! Port of `fixtures/router-fallback-tree/`'s 20 fixtures — a new-characterization
//! domain (no MSC 1 test file) covering `RouterPortForwardFallbackDecisionTree`'s
//! tree structure (`make_tree`, `unknown_router_bullets`) and its resolution
//! engine (`resolve`), against `fixtures/router-sample-catalog.json`'s 6-guide
//! sample catalog (P1.10).
//!
//! Test functions are prefixed `router_fallback_tree_` so the plan's Verify
//! command (a plain nextest substring filter, matching on test name, not
//! file/binary name) selects all of them.

mod support;

use msc_domain::router::fallback_tree::{self, FallbackState, NetworkType};
use msc_domain::router::matcher::{AdminSurface, Guide, GuideCategory, GuideFamily};
use serde_json::Value;
use support::Fixture;

fn load_catalog() -> Vec<Guide> {
    let path = support::fixtures_dir().join("router-sample-catalog.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read sample catalog: {e}", path.display()));
    let root: Value = serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!(
            "{}: could not parse sample catalog JSON: {e}",
            path.display()
        )
    });

    root["guides"]
        .as_array()
        .expect("sample catalog has a `guides` array")
        .iter()
        .map(|g| Guide {
            id: g["id"].as_str().unwrap().to_string(),
            display_name: g["displayName"].as_str().unwrap().to_string(),
            category: GuideCategory::from_raw_value(g["category"].as_str().unwrap())
                .unwrap_or_else(|| panic!("unhandled category: {}", g["category"])),
            family: GuideFamily::from_raw_value(g["family"].as_str().unwrap())
                .unwrap_or_else(|| panic!("unhandled family: {}", g["family"])),
            search_keywords: g["searchKeywords"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect(),
            admin_surface: AdminSurface::from_raw_value(g["adminSurface"].as_str().unwrap())
                .unwrap_or_else(|| panic!("unhandled adminSurface: {}", g["adminSurface"])),
            provider_display_name: g["providerDisplayName"].as_str().map(str::to_string),
            device_display_name: g["deviceDisplayName"].as_str().map(str::to_string),
        })
        .collect()
}

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("router-fallback-tree/{case}.json")))
}

fn opt_str(v: &Value) -> Option<&str> {
    v.as_str()
}

fn assert_tree_case(case: &str) {
    let fixture = load(case);
    let tree = fallback_tree::make_tree();

    let expected_nodes = fixture.expected["nodes"].as_array().unwrap();
    assert_eq!(tree.len(), expected_nodes.len(), "node count");

    for (actual, expected) in tree.iter().zip(expected_nodes) {
        assert_eq!(actual.id.raw_value(), expected["id"].as_str().unwrap());
        assert_eq!(actual.kind.raw_value(), expected["kind"].as_str().unwrap());

        let expected_choices = expected["choices"].as_array().unwrap();
        assert_eq!(
            actual.choices.len(),
            expected_choices.len(),
            "choice count for node {}",
            actual.id.raw_value()
        );

        for (actual_choice, expected_choice) in actual.choices.iter().zip(expected_choices) {
            assert_eq!(actual_choice.id, expected_choice["id"].as_str().unwrap());
            assert_eq!(
                actual_choice.next_node_id.map(|n| n.raw_value()),
                expected_choice["nextNodeId"].as_str()
            );
            assert_eq!(
                actual_choice.implied_network_type.map(|n| n.raw_value()),
                expected_choice["impliedNetworkType"].as_str()
            );
        }
    }
}

fn assert_bullets_case(case: &str) {
    let fixture = load(case);
    let ip = opt_str(&fixture.input["detectedGatewayIpAddress"]);
    let actual = fallback_tree::unknown_router_bullets(ip);
    let expected: Vec<&str> = fixture.expected["bullets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(actual, expected);
}

fn state_from(v: &Value) -> FallbackState {
    FallbackState {
        network_type: v["networkType"]
            .as_str()
            .and_then(NetworkType::from_raw_value),
        search_query: v["searchQuery"].as_str().unwrap().to_string(),
        only_knows_isp: v["onlyKnowsIsp"].as_bool().unwrap(),
        only_knows_mesh_system: v["onlyKnowsMeshSystem"].as_bool().unwrap(),
        unsure_whether_isp_or_own_router: v["unsureWhetherIspOrOwnRouter"].as_bool().unwrap(),
        wants_advanced_troubleshooting: v["wantsAdvancedTroubleshooting"].as_bool().unwrap(),
    }
}

fn assert_resolve_case(case: &str) {
    let fixture = load(case);
    let full_catalog = load_catalog();
    let no_mesh_catalog: Vec<Guide> = full_catalog
        .iter()
        .filter(|g| g.family != GuideFamily::GenericMesh)
        .cloned()
        .collect();

    let catalog = match fixture.input["catalog"].as_str().unwrap() {
        "full" => &full_catalog,
        "no-mesh" => &no_mesh_catalog,
        other => panic!("unhandled catalog: {other}"),
    };

    let state = state_from(&fixture.input["state"]);
    let gateway_ip = opt_str(&fixture.input["detectedGatewayIpAddress"]);

    let actual = fallback_tree::resolve(&state, catalog, gateway_ip);
    let expected = &fixture.expected;

    assert_eq!(actual.kind.raw_value(), expected["kind"].as_str().unwrap());
    assert_eq!(
        actual.availability.raw_value(),
        expected["availability"].as_str().unwrap()
    );
    assert_eq!(
        actual.matched_guide.map(|g| g.id.as_str()),
        expected["matchedGuideId"].as_str()
    );
    assert_eq!(
        actual.fallback_guide.map(|g| g.id.as_str()),
        expected["fallbackGuideId"].as_str()
    );
    assert_eq!(
        actual.desired_family.map(|f| f.raw_value()),
        expected["desiredFamily"].as_str()
    );

    let expected_families: Vec<&str> = expected["inferredFamilies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let actual_families: Vec<&str> = actual
        .inferred_families
        .iter()
        .map(|f| f.raw_value())
        .collect();
    assert_eq!(actual_families, expected_families);

    let expected_bullets: Vec<&str> = expected["explanationBullets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(actual.explanation_bullets, expected_bullets);

    assert_eq!(
        actual.recommended_next_node_id.map(|n| n.raw_value()),
        expected["recommendedNextNodeId"].as_str()
    );

    let expected_terms: Vec<&str> = expected["suggestedSearchTerms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(actual.suggested_search_terms, expected_terms.as_slice());

    assert_eq!(
        actual.matched_query.as_deref(),
        expected["matchedQuery"].as_str()
    );
}

#[test]
fn router_fallback_tree_structure_and_node_transitions() {
    assert_tree_case("tree-structure-and-node-transitions");
}

#[test]
fn router_fallback_tree_unknown_router_bullets_without_gateway_ip() {
    assert_bullets_case("unknown-router-bullets-without-gateway-ip");
}

#[test]
fn router_fallback_tree_unknown_router_bullets_with_gateway_ip() {
    assert_bullets_case("unknown-router-bullets-with-gateway-ip");
}

#[test]
fn router_fallback_tree_unknown_router_bullets_blank_gateway_ip_not_inserted() {
    assert_bullets_case("unknown-router-bullets-blank-gateway-ip-not-inserted");
}

#[test]
fn router_fallback_tree_resolve_wants_advanced_troubleshooting_short_circuits() {
    assert_resolve_case("resolve-wants-advanced-troubleshooting-short-circuits");
}

#[test]
fn router_fallback_tree_resolve_query_top_candidate_is_troubleshooting_family() {
    assert_resolve_case("resolve-query-top-candidate-is-troubleshooting-family");
}

#[test]
fn router_fallback_tree_resolve_query_exact_keyword_returns_exact_guide() {
    assert_resolve_case("resolve-query-exact-keyword-returns-exact-guide");
}

#[test]
fn router_fallback_tree_resolve_query_family_alias_returns_family_guide() {
    assert_resolve_case("resolve-query-family-alias-returns-family-guide");
}

#[test]
fn router_fallback_tree_resolve_family_recognized_not_seeded_generic_router_fallback() {
    assert_resolve_case("resolve-family-recognized-not-seeded-generic-router-fallback");
}

#[test]
fn router_fallback_tree_resolve_family_recognized_not_seeded_generic_mesh_fallback() {
    assert_resolve_case("resolve-family-recognized-not-seeded-generic-mesh-fallback");
}

#[test]
fn router_fallback_tree_resolve_query_no_family_inferred_falls_through_to_generic_router() {
    assert_resolve_case("resolve-query-no-family-inferred-falls-through-to-generic-router");
}

#[test]
fn router_fallback_tree_resolve_network_type_mesh_system_seeded() {
    assert_resolve_case("resolve-network-type-mesh-system-seeded");
}

#[test]
fn router_fallback_tree_resolve_network_type_mesh_system_not_seeded_falls_back_to_generic_router() {
    assert_resolve_case("resolve-network-type-mesh-system-not-seeded-falls-back-to-generic-router");
}

#[test]
fn router_fallback_tree_resolve_network_type_isp_gateway_recommends_provider_choice() {
    assert_resolve_case("resolve-network-type-isp-gateway-recommends-provider-choice");
}

#[test]
fn router_fallback_tree_resolve_network_type_isp_gateway_unsure_recommends_clarifier() {
    assert_resolve_case("resolve-network-type-isp-gateway-unsure-recommends-clarifier");
}

#[test]
fn router_fallback_tree_resolve_network_type_own_router_recommends_brand_choice() {
    assert_resolve_case("resolve-network-type-own-router-recommends-brand-choice");
}

#[test]
fn router_fallback_tree_resolve_only_knows_mesh_system_flag_uses_generic_mesh() {
    assert_resolve_case("resolve-only-knows-mesh-system-flag-uses-generic-mesh");
}

#[test]
fn router_fallback_tree_resolve_empty_state_falls_back_to_unknown_router_help() {
    assert_resolve_case("resolve-empty-state-falls-back-to-unknown-router-help");
}

#[test]
fn router_fallback_tree_resolve_unsure_isp_or_own_router_flag_recommends_clarifier() {
    assert_resolve_case("resolve-unsure-isp-or-own-router-flag-recommends-clarifier");
}

#[test]
fn router_fallback_tree_resolve_mesh_not_seeded_threads_detected_gateway_ip_into_bullet() {
    assert_resolve_case("resolve-mesh-not-seeded-threads-detected-gateway-ip-into-bullet");
}
