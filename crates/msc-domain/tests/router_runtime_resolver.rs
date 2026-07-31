//! Port of `fixtures/router-runtime-resolver/`'s 15 fixtures — a
//! new-characterization domain (no MSC 1 test file) covering
//! `RouterPortForwardGuideRuntimeResolver`'s `resolve(_:context:)` (against
//! `fixtures/router-sample-catalog.json`'s 6-guide sample catalog, P1.10,
//! composed via P1.12's already-tested `composer::compose_guide_by_id`) and
//! its `makeRecommendedProtocol(javaPort:bedrockPort:bedrockEnabled:)`.
//!
//! Test functions are prefixed `router_runtime_resolver_` so the plan's
//! Verify command (a plain nextest substring filter, matching on test name,
//! not file/binary name) selects all of them.

mod support;

use msc_domain::router::composer::{
    self, AdminSurface, Guide, GuideCategory, GuideNote, GuideStep, ReviewMetadata,
    RouterGuideConfidence, RouterGuideStepKind, RouterGuideToken, SectionItem, SectionKind,
    SectionOrigin, SharedSections, TroubleshootingTopic, TroubleshootingTopicId,
};
use msc_domain::router::runtime_resolver::{self, ResolvedItem, RuntimeContext};
use serde_json::{Value, json};
use support::Fixture;

fn tokens_from(v: &Value) -> Vec<RouterGuideToken> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|t| {
            RouterGuideToken::from_raw_value(t.as_str().unwrap())
                .unwrap_or_else(|| panic!("unhandled token: {t}"))
        })
        .collect()
}

fn load_full_catalog() -> (Vec<Guide>, Vec<TroubleshootingTopic>) {
    let path = support::fixtures_dir().join("router-sample-catalog.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read sample catalog: {e}", path.display()));
    let root: Value = serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!(
            "{}: could not parse sample catalog JSON: {e}",
            path.display()
        )
    });

    let guides = root["guides"]
        .as_array()
        .expect("sample catalog has a `guides` array")
        .iter()
        .map(|g| Guide {
            id: g["id"].as_str().unwrap().to_string(),
            category: GuideCategory::from_raw_value(g["category"].as_str().unwrap())
                .unwrap_or_else(|| panic!("unhandled category: {}", g["category"])),
            admin_addresses: g["adminAddresses"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect(),
            admin_surface: AdminSurface::from_raw_value(g["adminSurface"].as_str().unwrap())
                .unwrap_or_else(|| panic!("unhandled adminSurface: {}", g["adminSurface"])),
            menu_path: g["menuPath"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect(),
            alternate_menu_names: g["alternateMenuNames"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect(),
            steps: g["steps"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| GuideStep {
                    id: s["id"].as_str().unwrap().to_string(),
                    kind: RouterGuideStepKind::from_raw_value(s["kind"].as_str().unwrap())
                        .unwrap_or_else(|| panic!("unhandled step kind: {}", s["kind"])),
                    title: s["title"].as_str().unwrap().to_string(),
                    body: s["body"].as_str().unwrap().to_string(),
                    referenced_tokens: tokens_from(&s["referencedTokens"]),
                    alternate_terms: s["alternateTerms"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().unwrap().to_string())
                        .collect(),
                })
                .collect(),
            notes: g["notes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| GuideNote {
                    id: n["id"].as_str().unwrap().to_string(),
                    title: n["title"].as_str().map(str::to_string),
                    body: n["body"].as_str().unwrap().to_string(),
                })
                .collect(),
            troubleshooting: g["troubleshooting"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| {
                    TroubleshootingTopicId::from_raw_value(t.as_str().unwrap())
                        .unwrap_or_else(|| panic!("unhandled topic id: {t}"))
                })
                .collect(),
            shared_sections: {
                let s = &g["sharedSections"];
                SharedSections {
                    include_shared_intro: s["includeSharedIntro"].as_bool().unwrap(),
                    include_shared_prerequisites: s["includeSharedPrerequisites"]
                        .as_bool()
                        .unwrap(),
                    include_shared_value_summary: s["includeSharedValueSummary"].as_bool().unwrap(),
                    include_shared_troubleshooting_footer: s["includeSharedTroubleshootingFooter"]
                        .as_bool()
                        .unwrap(),
                }
            },
            review: ReviewMetadata {
                source_confidence: RouterGuideConfidence::from_raw_value(
                    g["review"]["sourceConfidence"].as_str().unwrap(),
                )
                .unwrap_or_else(|| {
                    panic!(
                        "unhandled sourceConfidence: {}",
                        g["review"]["sourceConfidence"]
                    )
                }),
            },
            provider_display_name: g["providerDisplayName"].as_str().map(str::to_string),
            device_display_name: g["deviceDisplayName"].as_str().map(str::to_string),
        })
        .collect();

    let topics = root["troubleshootingTopics"]
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
        .collect();

    (guides, topics)
}

fn context_from(v: &Value) -> RuntimeContext {
    RuntimeContext {
        selected_server_id: v["selectedServerId"].as_str().map(str::to_string),
        selected_server_name: v["selectedServerName"].as_str().map(str::to_string),
        detected_local_ip_address: v["detectedLocalIpAddress"].as_str().map(str::to_string),
        detected_gateway_ip_address: v["detectedGatewayIpAddress"].as_str().map(str::to_string),
        java_port: v["javaPort"].as_i64().map(|n| n as i32),
        bedrock_port: v["bedrockPort"].as_i64().map(|n| n as i32),
        recommended_protocol: v["recommendedProtocol"].as_str().map(str::to_string),
        bedrock_enabled: v["bedrockEnabled"].as_bool(),
    }
}

fn resolved_item_to_json(item: &ResolvedItem) -> Value {
    match item {
        ResolvedItem::Paragraph { title, body } => {
            json!({"type": "paragraph", "title": title, "body": body})
        }
        ResolvedItem::BulletList { title, bullets } => {
            json!({"type": "bulletList", "title": title, "bullets": bullets})
        }
        ResolvedItem::MenuPath {
            title,
            path,
            alternate_menu_names,
        } => {
            json!({"type": "menuPath", "title": title, "path": path, "alternateMenuNames": alternate_menu_names})
        }
        ResolvedItem::Step(step) => {
            json!({"type": "step", "id": step.id, "title": step.title, "body": step.body})
        }
        ResolvedItem::Note(note) => {
            json!({"type": "note", "id": note.id, "title": note.title, "body": note.body})
        }
        ResolvedItem::TroubleshootingTopic(topic) => {
            json!({"type": "troubleshootingTopic", "id": topic.id.raw_value()})
        }
    }
}

/// Synthetic test-only `ComposedSection` (not literal MSC 1 content) whose
/// body repeats the same placeholder twice — see
/// `duplicate-placeholder-occurrences-in-one-string-both-replaced`'s fixture
/// notes.
fn synthetic_duplicate_placeholder_guide<'a>(catalog: &'a [Guide]) -> composer::ComposedGuide<'a> {
    let guide = catalog
        .iter()
        .find(|g| g.id == "generic-router")
        .expect("generic-router is in the sample catalog");

    composer::ComposedGuide {
        guide,
        sections: vec![composer::ComposedSection {
            id: "synthetic.section".to_string(),
            kind: SectionKind::ValueSummary,
            title: "Synthetic",
            items: vec![SectionItem::Paragraph {
                title: None,
                body: "Local IP is {{detected_local_ip_address}}. Confirm {{detected_local_ip_address}} matches your host Mac.".to_string(),
                referenced_tokens: vec![RouterGuideToken::DetectedLocalIpAddress],
            }],
            origin: SectionOrigin::Shared,
        }],
    }
}

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("router-runtime-resolver/{case}.json")))
}

fn assert_case(case: &str) {
    let fixture = load(case);
    let (full_catalog, topics) = load_full_catalog();

    match fixture.input["mode"].as_str().unwrap() {
        "resolve" => {
            let guide_id = fixture.input["guideId"].as_str().unwrap();
            let composed = composer::compose_guide_by_id(guide_id, &full_catalog, &topics)
                .unwrap_or_else(|| panic!("no guide '{guide_id}' in full catalog"));
            let context = context_from(&fixture.input["context"]);
            let resolved = runtime_resolver::resolve(composed, &context);

            let actual = json!({
                "id": resolved.id(),
                "sections": resolved.sections.iter().map(|s| json!({
                    "id": s.id,
                    "kind": s.kind.raw_value(),
                    "items": s.items.iter().map(resolved_item_to_json).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
                "unresolvedTokens": resolved.unresolved_tokens.iter().map(|t| json!({
                    "sectionId": t.section_id,
                    "token": t.token.raw_value(),
                })).collect::<Vec<_>>(),
            });

            assert_eq!(actual, fixture.expected);
        }
        "resolve_synthetic_duplicate_placeholder" => {
            let composed = synthetic_duplicate_placeholder_guide(&full_catalog);
            let context = context_from(&fixture.input["context"]);
            let resolved = runtime_resolver::resolve(composed, &context);

            let actual = json!({
                "id": resolved.id(),
                "sections": resolved.sections.iter().map(|s| json!({
                    "id": s.id,
                    "kind": s.kind.raw_value(),
                    "items": s.items.iter().map(resolved_item_to_json).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
                "unresolvedTokens": resolved.unresolved_tokens.iter().map(|t| json!({
                    "sectionId": t.section_id,
                    "token": t.token.raw_value(),
                })).collect::<Vec<_>>(),
            });

            assert_eq!(actual, fixture.expected);
        }
        "make_recommended_protocol" => {
            let java_port = fixture.input["javaPort"].as_i64().map(|n| n as i32);
            let bedrock_port = fixture.input["bedrockPort"].as_i64().map(|n| n as i32);
            let bedrock_enabled = fixture.input["bedrockEnabled"].as_bool().unwrap();

            let result = runtime_resolver::make_recommended_protocol(
                java_port,
                bedrock_port,
                bedrock_enabled,
            );

            assert_eq!(json!({"result": result}), fixture.expected);
        }
        other => panic!("unhandled mode: {other}"),
    }
}

#[test]
fn router_runtime_resolver_full_context_all_tokens_resolve() {
    assert_case("full-context-all-tokens-resolve");
}

#[test]
fn router_runtime_resolver_empty_context_all_tokens_fall_back() {
    assert_case("empty-context-all-tokens-fall-back");
}

#[test]
fn router_runtime_resolver_server_not_selected_name_falls_back_others_resolve() {
    assert_case("server-not-selected-name-falls-back-others-resolve");
}

#[test]
fn router_runtime_resolver_whitespace_only_values_treated_as_unset() {
    assert_case("whitespace-only-values-treated-as-unset");
}

#[test]
fn router_runtime_resolver_bedrock_port_resolves_independent_of_bedrock_enabled_flag() {
    assert_case("bedrock-port-resolves-independent-of-bedrock-enabled-flag");
}

#[test]
fn router_runtime_resolver_guide_steps_with_referenced_tokens_metadata_but_no_literal_placeholder_pass_through()
 {
    assert_case(
        "guide-steps-with-referenced-tokens-metadata-but-no-literal-placeholder-pass-through",
    );
}

#[test]
fn router_runtime_resolver_menu_path_and_note_and_troubleshooting_topic_items_pass_through_unchanged()
 {
    assert_case("menu-path-and-note-and-troubleshooting-topic-items-pass-through-unchanged");
}

#[test]
fn router_runtime_resolver_recommended_protocol_context_value_echoed_verbatim_into_value_summary() {
    assert_case("recommended-protocol-context-value-echoed-verbatim-into-value-summary");
}

#[test]
fn router_runtime_resolver_duplicate_placeholder_occurrences_in_one_string_both_replaced() {
    assert_case("duplicate-placeholder-occurrences-in-one-string-both-replaced");
}

#[test]
fn router_runtime_resolver_protocol_java_only() {
    assert_case("protocol-java-only");
}

#[test]
fn router_runtime_resolver_protocol_java_and_bedrock_enabled_with_geyser() {
    assert_case("protocol-java-and-bedrock-enabled-with-geyser");
}

#[test]
fn router_runtime_resolver_protocol_bedrock_only_no_java_port() {
    assert_case("protocol-bedrock-only-no-java-port");
}

#[test]
fn router_runtime_resolver_protocol_neither_port_known() {
    assert_case("protocol-neither-port-known");
}

#[test]
fn router_runtime_resolver_protocol_bedrock_enabled_but_no_bedrock_port_falls_back_to_java() {
    assert_case("protocol-bedrock-enabled-but-no-bedrock-port-falls-back-to-java");
}

#[test]
fn router_runtime_resolver_protocol_bedrock_port_present_but_not_enabled_ignored() {
    assert_case("protocol-bedrock-port-present-but-not-enabled-ignored");
}
