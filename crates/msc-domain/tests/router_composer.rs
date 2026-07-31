//! Port of `fixtures/router-composer/`'s 7 fixtures — a new-characterization
//! domain (no MSC 1 test file) covering `RouterPortForwardGuideComposer`'s
//! `composeGuide(_:)`/`composeGuide(id:)`, against `fixtures/router-sample-catalog.json`'s
//! 6-guide sample catalog (P1.10) plus one synthetic ad-hoc guide built inline
//! for branches the real sample data never reaches.
//!
//! Test functions are prefixed `router_composer_` so the plan's Verify
//! command (a plain nextest substring filter, matching on test name, not
//! file/binary name) selects all of them.

mod support;

use msc_domain::router::composer::{
    self, AdminSurface, Guide, GuideCategory, GuideNote, GuideStep, ReviewMetadata,
    RouterGuideConfidence, RouterGuideStepKind, RouterGuideToken, SectionItem, SharedSections,
    TroubleshootingTopic, TroubleshootingTopicId,
};
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

/// A synthetic, ad-hoc guide (not part of the shared sample catalog, not
/// literal MSC 1 content) exercising branches none of the 6 real sample
/// guides reach — see `compose-guide-synthetic-minimal-omits-optional-sections`'s
/// fixture notes.
fn synthetic_minimal_guide() -> Guide {
    Guide {
        id: "test-minimal".to_string(),
        category: GuideCategory::RetailRouter,
        admin_addresses: vec![],
        admin_surface: AdminSurface::WebBrowser,
        menu_path: vec![],
        alternate_menu_names: vec![],
        steps: vec![GuideStep {
            id: "minimal-step-1".to_string(),
            kind: RouterGuideStepKind::Navigate,
            title: "x".to_string(),
            body: "x".to_string(),
            referenced_tokens: vec![RouterGuideToken::DetectedLocalIpAddress],
            alternate_terms: vec![],
        }],
        notes: vec![],
        troubleshooting: vec![],
        shared_sections: SharedSections {
            include_shared_intro: true,
            include_shared_prerequisites: true,
            include_shared_value_summary: true,
            include_shared_troubleshooting_footer: true,
        },
        review: ReviewMetadata {
            source_confidence: RouterGuideConfidence::CommunityBased,
        },
        provider_display_name: None,
        device_display_name: None,
    }
}

fn item_to_json(item: &SectionItem) -> Value {
    match item {
        SectionItem::Paragraph {
            title,
            body,
            referenced_tokens,
        } => {
            json!({"type": "paragraph", "title": title, "body": body, "referencedTokens": referenced_tokens.iter().map(|t| t.raw_value()).collect::<Vec<_>>()})
        }
        SectionItem::BulletList {
            title,
            bullets,
            referenced_tokens,
        } => {
            json!({"type": "bulletList", "title": title, "bullets": bullets, "referencedTokens": referenced_tokens.iter().map(|t| t.raw_value()).collect::<Vec<_>>()})
        }
        SectionItem::MenuPath {
            title,
            path,
            alternate_menu_names,
        } => {
            json!({"type": "menuPath", "title": title, "path": path, "alternateMenuNames": alternate_menu_names})
        }
        SectionItem::Step(step) => {
            json!({"type": "step", "id": step.id, "kind": step.kind.raw_value(), "referencedTokens": step.referenced_tokens.iter().map(|t| t.raw_value()).collect::<Vec<_>>()})
        }
        SectionItem::Note(note) => json!({"type": "note", "id": note.id, "title": note.title}),
        SectionItem::TroubleshootingTopic(topic) => {
            json!({"type": "troubleshootingTopic", "id": topic.id.raw_value()})
        }
    }
}

fn composed_guide_to_json(g: &composer::ComposedGuide) -> Value {
    json!({
        "id": g.id(),
        "sections": g.sections.iter().map(|s| json!({
            "id": s.id,
            "kind": s.kind.raw_value(),
            "title": s.title,
            "origin": s.origin.raw_value(),
            "items": s.items.iter().map(item_to_json).collect::<Vec<_>>(),
            "referencedTokens": s.referenced_tokens().iter().map(|t| t.raw_value()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("router-composer/{case}.json")))
}

fn assert_case(case: &str) {
    let fixture = load(case);
    let (full_catalog, topics) = load_full_catalog();
    let synthetic = synthetic_minimal_guide();

    let guide_id = fixture.input["guideId"].as_str().unwrap();

    match fixture.input["mode"].as_str().unwrap() {
        "compose_guide" => {
            let guide = match fixture.input["catalog"].as_str().unwrap() {
                "full" => full_catalog
                    .iter()
                    .find(|g| g.id == guide_id)
                    .unwrap_or_else(|| panic!("no guide '{guide_id}' in full catalog")),
                "synthetic-minimal" => &synthetic,
                other => panic!("unhandled catalog: {other}"),
            };
            let composed = composer::compose_guide(guide, &topics);
            assert_eq!(composed_guide_to_json(&composed), fixture.expected);
        }
        "compose_guide_by_id" => {
            let composed = composer::compose_guide_by_id(guide_id, &full_catalog, &topics);
            match composed {
                Some(composed) => assert_eq!(composed_guide_to_json(&composed), fixture.expected),
                None => assert_eq!(fixture.expected["id"], Value::Null),
            }
        }
        other => panic!("unhandled mode: {other}"),
    }
}

#[test]
fn router_composer_full_shared_sections_and_menu_path() {
    assert_case("compose-guide-full-shared-sections-and-menu-path");
}

#[test]
fn router_composer_provider_and_device_differ() {
    assert_case("compose-guide-provider-and-device-differ");
}

#[test]
fn router_composer_shared_prerequisites_and_troubleshooting_footer_omitted() {
    assert_case("compose-guide-shared-prerequisites-and-troubleshooting-footer-omitted");
}

#[test]
fn router_composer_mesh_category_and_mobile_app_surface() {
    assert_case("compose-guide-mesh-category-and-mobile-app-surface");
}

#[test]
fn router_composer_synthetic_minimal_omits_optional_sections() {
    assert_case("compose-guide-synthetic-minimal-omits-optional-sections");
}

#[test]
fn router_composer_compose_guide_by_id_found() {
    assert_case("compose-guide-by-id-found");
}

#[test]
fn router_composer_compose_guide_by_id_not_found() {
    assert_case("compose-guide-by-id-not-found");
}
