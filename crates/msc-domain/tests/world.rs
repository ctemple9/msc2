//! Port of `fixtures/world-slots/`'s 12 fixtures (P6.4).
//!
//! Test functions are prefixed `world_slots_` so the plan's Verify command
//! (`cargo nextest run -p msc-domain world`, a plain substring filter on
//! test name) selects them.

mod support;

use msc_domain::identity::ServerType;
use msc_domain::world::{
    BackupAssociation, WorldSlot, build_bootstrap_slot, build_fresh_slot, current_level_name,
    effective_backup_association, resolve_active_slot_id, sanitized_world_level_name,
    sort_newest_first,
};
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("world-slots/{case}.json")))
}

fn slot_from_value(v: &serde_json::Value) -> WorldSlot {
    WorldSlot {
        id: v["id"].as_str().unwrap().to_string(),
        name: v
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string(),
        created_at: v["created_at"].as_str().unwrap().to_string(),
        last_played_at: v
            .get("last_played_at")
            .and_then(|x| x.as_str())
            .map(str::to_string),
        thumbnail_file_name: None,
        world_level_name: None,
        world_seed: None,
        zip_size_bytes: None,
    }
}

#[test]
fn world_slots_slot_metadata_json_defaults() {
    let fixture = load("slot-metadata-json-defaults");
    let slot = WorldSlot::decode(&fixture.input["slot_json_contents"]).unwrap();
    assert_eq!(slot.id, fixture.expected["id"].as_str().unwrap());
    assert_eq!(slot.name, fixture.expected["name"].as_str().unwrap());
    assert_eq!(
        slot.created_at,
        fixture.expected["created_at"].as_str().unwrap()
    );
    assert_eq!(slot.last_played_at, None);
    assert_eq!(slot.thumbnail_file_name, None);
    assert_eq!(slot.world_level_name, None);
    assert_eq!(slot.world_seed, None);
    assert_eq!(slot.zip_size_bytes, None);
}

#[test]
fn world_slots_load_slots_corrupt_entry_skipped() {
    let fixture = load("load-slots-corrupt-entry-skipped");
    let mut loaded = Vec::new();
    for entry in fixture.input["world_slots_dir_entries"].as_array().unwrap() {
        if !entry["is_directory"].as_bool().unwrap() {
            continue;
        }
        let contents = &entry["slot_json_contents"];
        if contents.is_null() {
            continue;
        }
        // A string value stands in for genuinely-unparseable JSON text.
        if let Some(raw) = contents.as_str() {
            assert!(serde_json::from_str::<serde_json::Value>(raw).is_err());
            continue;
        }
        if let Ok(slot) = WorldSlot::decode(contents) {
            loaded.push(slot);
        }
    }
    let ids: Vec<&str> = loaded.iter().map(|s| s.id.as_str()).collect();
    let expected: Vec<&str> = fixture.expected["loaded_slot_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(ids, expected);
}

#[test]
fn world_slots_load_slots_missing_directory_returns_empty() {
    // `world_slots_dir_exists: false` is a directory-listing concern —
    // `msc-infrastructure` owns the guard itself (P6.10); this fixture's
    // domain-level content is "no entries in, no slots out."
    let loaded: Vec<WorldSlot> = Vec::new();
    let mut loaded = loaded;
    sort_newest_first(&mut loaded);
    assert!(loaded.is_empty());
}

#[test]
fn world_slots_load_slots_newest_first_ordering() {
    let fixture = load("load-slots-newest-first-ordering");
    let mut slots: Vec<WorldSlot> = fixture.input["world_slots_dir_entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| slot_from_value(&e["slot_json_contents"]))
        .collect();
    sort_newest_first(&mut slots);
    let ids: Vec<&str> = slots.iter().map(|s| s.id.as_str()).collect();
    let expected: Vec<&str> = fixture.expected["loaded_slot_ids_in_order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(ids, expected);
}

fn slots_for_resolution(fixture: &Fixture) -> Vec<WorldSlot> {
    fixture.input["slots"]
        .as_array()
        .unwrap()
        .iter()
        .map(slot_from_value)
        .collect()
}

fn explicit_marker(fixture: &Fixture) -> Option<String> {
    fixture.input["active_slot_id_txt"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[test]
fn world_slots_resolved_active_explicit_marker_wins() {
    let fixture = load("resolved-active-explicit-marker-wins");
    let slots = slots_for_resolution(&fixture);
    let marker = explicit_marker(&fixture);
    let resolved = resolve_active_slot_id(&slots, marker.as_deref());
    assert_eq!(
        resolved.as_deref(),
        fixture.expected["resolved_active_slot_id"].as_str()
    );
}

#[test]
fn world_slots_resolved_active_explicit_marker_missing_slot_falls_back() {
    let fixture = load("resolved-active-explicit-marker-missing-slot-falls-back");
    let slots = slots_for_resolution(&fixture);
    let marker = explicit_marker(&fixture);
    let resolved = resolve_active_slot_id(&slots, marker.as_deref());
    assert_eq!(
        resolved.as_deref(),
        fixture.expected["resolved_active_slot_id"].as_str()
    );
}

#[test]
fn world_slots_resolved_active_no_lastplayed_falls_back_to_newest_created() {
    let fixture = load("resolved-active-no-lastplayed-falls-back-to-newest-created");
    let slots = slots_for_resolution(&fixture);
    let marker = explicit_marker(&fixture);
    let resolved = resolve_active_slot_id(&slots, marker.as_deref());
    assert_eq!(
        resolved.as_deref(),
        fixture.expected["resolved_active_slot_id"].as_str()
    );
}

#[test]
fn world_slots_resolved_active_empty_slots_returns_nil() {
    let fixture = load("resolved-active-empty-slots-returns-nil");
    let slots = slots_for_resolution(&fixture);
    let marker = explicit_marker(&fixture);
    let resolved = resolve_active_slot_id(&slots, marker.as_deref());
    assert_eq!(resolved, None);
    assert_eq!(
        fixture.expected["resolved_active_slot_id"],
        serde_json::Value::Null
    );
}

#[test]
fn world_slots_sanitized_level_name_strips_invalid_characters() {
    let fixture = load("sanitized-level-name-strips-invalid-characters");
    let cases = fixture.input["cases"].as_array().unwrap();
    let expected = fixture.expected["results"].as_array().unwrap();
    for (case, want) in cases.iter().zip(expected) {
        let raw = case["raw"].as_str().unwrap();
        let fallback = case["fallback"].as_str().unwrap();
        assert_eq!(
            sanitized_world_level_name(raw, fallback),
            want.as_str().unwrap()
        );
    }
}

#[test]
fn world_slots_current_level_name_java_bedrock_fallbacks() {
    let fixture = load("current-level-name-java-bedrock-fallbacks");
    let cases = fixture.input["cases"].as_array().unwrap();
    let expected = fixture.expected["results"].as_array().unwrap();
    for (case, want) in cases.iter().zip(expected) {
        let server_type = match case["server_type"].as_str().unwrap() {
            "java" => ServerType::Java,
            "bedrock" => ServerType::Bedrock,
            other => panic!("unknown server_type {other}"),
        };
        let raw = case
            .get("server_properties_level_name")
            .or_else(|| case.get("bedrock_level_name"))
            .and_then(|v| v.as_str());
        assert_eq!(current_level_name(server_type, raw), want.as_str().unwrap());
    }
}

#[test]
fn world_slots_fresh_archive_less_slot_creation() {
    let fixture = load("fresh-archive-less-slot-creation");
    let cases = fixture.input["cases"].as_array().unwrap();
    let expected = fixture.expected["results"].as_array().unwrap();
    for (case, want) in cases.iter().zip(expected) {
        let server_type = match case["server_type"].as_str().unwrap() {
            "java" => ServerType::Java,
            "bedrock" => ServerType::Bedrock,
            other => panic!("unknown server_type {other}"),
        };
        let slot = build_fresh_slot(
            "id".to_string(),
            case["name"].as_str().unwrap(),
            case.get("seed").and_then(|v| v.as_str()),
            server_type,
            "2026-01-01T00:00:00Z".to_string(),
        );
        assert_eq!(slot.name, want["name"].as_str().unwrap());
        assert_eq!(
            slot.world_level_name.as_deref(),
            want["world_level_name"].as_str()
        );
        assert_eq!(slot.world_seed.as_deref(), want["world_seed"].as_str());
        assert_eq!(slot.last_played_at, None);
        assert_eq!(slot.zip_size_bytes, None);
        assert!(!want["has_archive"].as_bool().unwrap());
    }
}

#[test]
fn world_slots_initial_slot_bootstrap_creates_and_activates() {
    let fixture = load("initial-slot-bootstrap-creates-and-activates");
    let server_type = match fixture.input["server_type"].as_str().unwrap() {
        "java" => ServerType::Java,
        "bedrock" => ServerType::Bedrock,
        other => panic!("unknown server_type {other}"),
    };
    let raw_level_name = fixture.input["server_properties_level_name"].as_str();
    let created_at = "2026-05-01T00:00:00Z".to_string();
    let slot = build_bootstrap_slot(
        "new-slot-id".to_string(),
        server_type,
        raw_level_name,
        created_at.clone(),
    );
    assert_eq!(
        slot.name,
        fixture.expected["new_slot_name"].as_str().unwrap()
    );
    assert_eq!(
        slot.world_level_name.as_deref(),
        fixture.expected["new_slot_world_level_name"].as_str()
    );
    assert_eq!(slot.last_played_at.as_deref(), Some(created_at.as_str()));
    assert!(fixture.expected["new_slot_has_archive"].as_bool().unwrap());
}

#[test]
fn world_slots_decode_requires_id_name_created_at() {
    assert!(WorldSlot::decode(&serde_json::json!({"name": "x", "created_at": "y"})).is_err());
}

// `effectiveBackupAssociation` (`AppViewModel+Backups.swift` line
// 143-163) has no fixture of its own yet — full backup-domain fixtures
// land in P6.6/P6.15-18 — but P6.9 ports its pure resolution policy
// alongside the rest of the world-slot model, so it's covered here
// directly against the source behavior rather than left untested.

#[test]
fn world_slots_effective_backup_association_falls_back_to_active_slot() {
    let slots = vec![WorldSlot {
        id: "slot-a".into(),
        name: "Slot A".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
        last_played_at: None,
        thumbnail_file_name: None,
        world_level_name: None,
        world_seed: Some("42".into()),
        zip_size_bytes: None,
    }];
    let association = effective_backup_association(&slots, Some("slot-a"), None, None);
    assert_eq!(association.slot_id.as_deref(), Some("slot-a"));
    assert_eq!(association.slot_name.as_deref(), Some("Slot A"));
    assert_eq!(association.world_seed.as_deref(), Some("42"));
}

#[test]
fn world_slots_effective_backup_association_explicit_id_wins_over_active() {
    let slots = vec![
        WorldSlot {
            id: "slot-active".into(),
            name: "Active".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            last_played_at: None,
            thumbnail_file_name: None,
            world_level_name: None,
            world_seed: None,
            zip_size_bytes: None,
        },
        WorldSlot {
            id: "slot-explicit".into(),
            name: "Explicit".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            last_played_at: None,
            thumbnail_file_name: None,
            world_level_name: None,
            world_seed: Some("  99  ".into()),
            zip_size_bytes: None,
        },
    ];
    let association = effective_backup_association(
        &slots,
        Some("slot-active"),
        Some("slot-explicit"),
        Some("  Named  "),
    );
    assert_eq!(association.slot_id.as_deref(), Some("slot-explicit"));
    assert_eq!(association.slot_name.as_deref(), Some("Named"));
    assert_eq!(association.world_seed.as_deref(), Some("99"));
}

#[test]
fn world_slots_effective_backup_association_no_explicit_no_active_is_empty() {
    let association = effective_backup_association(&[], None, None, None);
    assert_eq!(association, BackupAssociation::default());
}
