//! Port of `fixtures/router-troubleshooting/`'s 14 fixtures — a new-characterization
//! domain (no MSC 1 test file) covering `RouterPortForwardTroubleshootingEngine`'s
//! `analyze(symptoms:fallbackState:runtimeContext:)`, against
//! `fixtures/router-sample-catalog.json`'s 6-guide/9-topic sample catalog
//! (P1.10/P1.12) plus a synthetic 10th rule and a synthetic 8-topic list built
//! inline for branches the real 9-rule/9-topic table never reaches.
//!
//! Test functions are prefixed `router_troubleshooting_` so the plan's Verify
//! command (a plain nextest substring filter, matching on test name, not
//! file/binary name) selects all of them.

mod support;

use msc_domain::router::composer::{TroubleshootingTopic, TroubleshootingTopicId};
use msc_domain::router::fallback_tree::{FallbackState, NetworkType};
use msc_domain::router::matcher::{AdminSurface, Guide, GuideCategory, GuideFamily};
use msc_domain::router::troubleshooting::{self, Requirement, Rule, SymptomId};
use serde_json::{Value, json};
use std::collections::HashSet;
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

fn load_topics() -> Vec<TroubleshootingTopic> {
    let path = support::fixtures_dir().join("router-sample-catalog.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read sample catalog: {e}", path.display()));
    let root: Value = serde_json::from_str(&text).unwrap();

    root["troubleshootingTopics"]
        .as_array()
        .expect("sample catalog has a `troubleshootingTopics` array")
        .iter()
        .map(|t| TroubleshootingTopic {
            id: TroubleshootingTopicId::from_raw_value(t["id"].as_str().unwrap())
                .unwrap_or_else(|| panic!("unhandled topic id: {}", t["id"])),
            title: t["title"].as_str().unwrap().to_string(),
            summary: t["summary"].as_str().unwrap().to_string(),
            suggested_next_actions: t["suggestedNextActions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect(),
        })
        .collect()
}

/// A synthetic 10th rule (not part of MSC 1's real 9-rule table) exercising
/// `evaluate`'s `excludedSymptoms` short-circuit — every real rule declares
/// `excludedSymptoms: []`, so that branch is structurally unreachable
/// against real data. See `excluded-symptoms-branch-synthetic-rule`'s
/// fixture notes.
fn rules_with_synthetic_exclusion() -> Vec<Rule> {
    let mut rules = troubleshooting::make_rules();
    rules.push(Rule {
        id: "test-synthetic-excluded",
        topic_id: TroubleshootingTopicId::LocalIpChanged,
        title: "Synthetic test-only rule",
        all_of: vec![],
        any_of: vec![Requirement {
            symptom: SymptomId::MacIpAddressChanged,
            weight: 9,
        }],
        excluded_symptoms: HashSet::from([SymptomId::SecurityToolMayBeBlocking]),
        explanation: "test",
        next_actions: vec!["synthetic next action"],
        escalation_bullets: vec!["synthetic escalation bullet"],
    });
    rules
}

fn symptoms_from(v: &Value) -> Vec<SymptomId> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| {
            SymptomId::from_raw_value(s.as_str().unwrap())
                .unwrap_or_else(|| panic!("unhandled symptom: {s}"))
        })
        .collect()
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

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("router-troubleshooting/{case}.json")))
}

fn assert_case(case: &str) {
    let fixture = load(case);
    let full_catalog = load_catalog();
    let full_topics = load_topics();
    let topics_missing_wrong_device: Vec<TroubleshootingTopic> = full_topics
        .iter()
        .filter(|t| t.id != TroubleshootingTopicId::WrongDevice)
        .cloned()
        .collect();

    let topics = match fixture.input["topics"].as_str().unwrap() {
        "full" => &full_topics,
        "missing_wrong_device" => &topics_missing_wrong_device,
        other => panic!("unhandled topics: {other}"),
    };

    let standard_rules = troubleshooting::make_rules();
    let synthetic_rules = rules_with_synthetic_exclusion();
    let rules = match fixture.input["rules"].as_str().unwrap() {
        "standard" => &standard_rules,
        "synthetic_exclusion" => &synthetic_rules,
        other => panic!("unhandled rules: {other}"),
    };

    let symptoms = symptoms_from(&fixture.input["symptoms"]);
    let fallback_state = match &fixture.input["fallbackState"] {
        Value::Null => None,
        v => Some(state_from(v)),
    };
    let gateway_ip = fixture.input["detectedGatewayIpAddress"].as_str();

    let report = troubleshooting::analyze(
        &symptoms,
        rules,
        topics,
        &full_catalog,
        fallback_state.as_ref(),
        gateway_ip,
    );

    let actual = json!({
        "symptoms": report.symptoms.iter().map(|s| s.raw_value()).collect::<Vec<_>>(),
        "likelyCauses": report.likely_causes.iter().map(|c| json!({
            "id": c.id.raw_value(),
            "confidence": c.confidence.raw_value(),
            "score": c.score,
            "severity": c.severity().raw_value(),
            "matchedSymptoms": c.matched_symptoms.iter().map(|s| s.raw_value()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "recommendedActions": report.recommended_actions,
        "escalationBullets": report.escalation_bullets,
        "fallbackResolutionKind": report.fallback_resolution.map(|r| r.kind.raw_value()),
        "summary": report.summary,
    });

    assert_eq!(actual, fixture.expected);
}

#[test]
fn router_troubleshooting_single_rule_required_only_match_no_admin_access() {
    assert_case("single-rule-required-only-match-no-admin-access");
}

#[test]
fn router_troubleshooting_strong_confidence_full_any_of_match_local_ip_changed() {
    assert_case("strong-confidence-full-any-of-match-local-ip-changed");
}

#[test]
fn router_troubleshooting_possible_confidence_partial_any_of_match_double_nat() {
    assert_case("possible-confidence-partial-any-of-match-double-nat");
}

#[test]
fn router_troubleshooting_no_symptoms_no_causes_no_fallback_generic_summary() {
    assert_case("no-symptoms-no-causes-no-fallback-generic-summary");
}

#[test]
fn router_troubleshooting_multiple_causes_score_and_confidence_tiebreak_sort() {
    assert_case("multiple-causes-score-and-confidence-tiebreak-sort");
}

#[test]
fn router_troubleshooting_duplicate_symptoms_normalized_preserving_first_occurrence_order() {
    assert_case("duplicate-symptoms-normalized-preserving-first-occurrence-order");
}

#[test]
fn router_troubleshooting_two_causes_also_check_summary_joins_remaining_titles() {
    assert_case("two-causes-also-check-summary-joins-remaining-titles");
}

#[test]
fn router_troubleshooting_fallback_state_unknown_router_help_summary_and_bullets_merged() {
    assert_case("fallback-state-unknown-router-help-summary-and-bullets-merged");
}

#[test]
fn router_troubleshooting_fallback_state_exact_guide_kind_still_generic_summary() {
    assert_case("fallback-state-exact-guide-kind-still-generic-summary");
}

#[test]
fn router_troubleshooting_fallback_state_and_gateway_ip_with_causes_bullets_appended_deduped() {
    assert_case("fallback-state-and-gateway-ip-with-causes-bullets-appended-deduped");
}

#[test]
fn router_troubleshooting_fallback_state_needs_more_info_kind_identify_router_path_summary() {
    assert_case("fallback-state-needs-more-info-kind-identify-router-path-summary");
}

#[test]
fn router_troubleshooting_evaluate_skips_rule_whose_topic_is_missing_synthetic_topics() {
    assert_case("evaluate-skips-rule-whose-topic-is-missing-synthetic-topics");
}

#[test]
fn router_troubleshooting_excluded_symptoms_branch_synthetic_rule() {
    assert_case("excluded-symptoms-branch-synthetic-rule");
}

#[test]
fn router_troubleshooting_excluded_symptoms_absent_synthetic_rule_fires_normally() {
    assert_case("excluded-symptoms-absent-synthetic-rule-fires-normally");
}
