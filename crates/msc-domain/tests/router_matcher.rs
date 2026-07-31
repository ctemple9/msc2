//! Port of `fixtures/router-matcher/`'s 10 fixtures — a new-characterization
//! domain (no MSC 1 test file) covering `RouterPortForwardGuideMatcher.match(_:)`
//! against `fixtures/router-sample-catalog.json`'s 6-guide sample catalog.
//!
//! Test functions are prefixed `router_matcher_` so the plan's Verify command
//! (a plain nextest substring filter, matching on test name, not file/binary
//! name) selects all of them.

mod support;

use msc_domain::router::matcher::{self, AdminSurface, Guide, GuideCategory, GuideFamily};
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
    support::load(support::fixtures_dir().join(format!("router-matcher/{case}.json")))
}

fn assert_case(case: &str) {
    let fixture = load(case);
    let catalog = load_catalog();
    let query = fixture.input["query"].as_str().unwrap();

    let result = matcher::match_query(query, &catalog);

    assert_eq!(
        result.normalized_query,
        fixture.expected["normalizedQuery"].as_str().unwrap()
    );

    let expected_tokens: Vec<&str> = fixture.expected["normalizedTokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(result.normalized_tokens, expected_tokens);

    let expected_families: Vec<&str> = fixture.expected["inferredFamilies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let actual_families: Vec<&str> = result
        .inferred_families
        .iter()
        .map(|f| f.raw_value())
        .collect();
    assert_eq!(actual_families, expected_families);

    let expected_candidates = fixture.expected["candidates"].as_array().unwrap();
    assert_eq!(
        result.candidates.len(),
        expected_candidates.len(),
        "candidate count"
    );
    for (actual, expected) in result.candidates.iter().zip(expected_candidates) {
        assert_eq!(actual.guide.id, expected["guideId"].as_str().unwrap());
        assert_eq!(actual.score as i64, expected["score"].as_i64().unwrap());
        let expected_reasons: Vec<&str> = expected["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(actual.reasons, expected_reasons);
    }

    let expected_fallback = fixture.expected["fallbackGuideId"].as_str();
    assert_eq!(
        result.suggested_fallback_guide.map(|g| g.id.as_str()),
        expected_fallback
    );

    assert_eq!(
        result.is_ambiguous,
        fixture.expected["isAmbiguous"].as_bool().unwrap()
    );
    assert_eq!(
        result.matched_direct_guide,
        fixture.expected["matchedDirectGuide"].as_bool().unwrap()
    );
}

#[test]
fn router_matcher_exact_keyword_match_caps_score() {
    assert_case("exact-keyword-match-caps-score");
}

#[test]
fn router_matcher_keyword_prefix_and_substring_partial_match() {
    assert_case("keyword-prefix-and-substring-partial-match");
}

#[test]
fn router_matcher_family_inferred_by_token_subset_phrase() {
    assert_case("family-inferred-by-token-subset-phrase");
}

#[test]
fn router_matcher_misspelled_brand_still_infers_family_via_short_alias() {
    assert_case("misspelled-brand-still-infers-family-via-short-alias");
}

#[test]
fn router_matcher_low_score_tie_does_not_make_result_ambiguous() {
    assert_case("low-score-tie-does-not-make-result-ambiguous");
}

#[test]
fn router_matcher_empty_input_gets_unconditional_unknown_intent_bonus() {
    assert_case("empty-input-gets-unconditional-unknown-intent-bonus");
}

#[test]
fn router_matcher_no_keyword_or_family_match_returns_empty_candidates() {
    assert_case("no-keyword-or-family-match-returns-empty-candidates");
}

#[test]
fn router_matcher_exact_keyword_plus_isp_gateway_hint() {
    assert_case("exact-keyword-plus-isp-gateway-hint");
}

#[test]
fn router_matcher_mesh_intent_ranks_multiple_candidates_with_app_hint() {
    assert_case("mesh-intent-ranks-multiple-candidates-with-app-hint");
}

#[test]
fn router_matcher_exact_score_tie_at_top_is_ambiguous() {
    assert_case("exact-score-tie-at-top-is-ambiguous");
}
