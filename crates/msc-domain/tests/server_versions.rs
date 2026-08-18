//! Port of `fixtures/server-jar-providers/`'s 26 cases (P7.4) against
//! `msc_domain::server_versions` (P7.10).
//!
//! 25 of the 26 fixture files map to a test below.
//! `pufferfish-excluded-from-list-versions-and-download-version-download-latest-only`
//! documents the `ServerJarProvider` flavor-dispatch enum's behavior for an
//! excluded flavor (`listVersions`/`downloadVersion`/`downloadLatest`
//! dispatch, `ServerJarProviders.swift:64-118`) -- dispatcher-level
//! behavior this module's per-family parse functions don't own; deferred to
//! P7.17, which builds the real dispatch.

mod support;

use msc_domain::server_versions::*;
use serde_json::{Value, json};
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("server-jar-providers/{case}.json")))
}

/// Some fixture inputs point at a sibling sample file instead of embedding
/// the raw response inline (real corpus responses with something removed
/// to isolate a branch -- see `fixtures/server-jar-providers/samples/`).
fn raw_body_of(value: &Value) -> String {
    if let Some(file) = value.get("file").and_then(Value::as_str) {
        let workspace_root = support::fixtures_dir().parent().unwrap().to_path_buf();
        std::fs::read_to_string(workspace_root.join(file))
            .unwrap_or_else(|e| panic!("could not read sample file {file}: {e}"))
    } else {
        value.to_string()
    }
}

// --- ServerVersionEntry shape ---

#[test]
fn server_versions_version_entry_latest_sentinel_and_per_family_identity() {
    let entry = ServerVersionEntry::latest();
    assert_eq!(entry.id, "__latest__");
    assert_eq!(entry.mc_version, "");
    assert!(entry.is_stable);
    assert!(entry.is_latest());

    // Download-and-go families (Vanilla/Paper/Purpur/Fabric): id == mc_version.
    // `simple_stable_entry` (used by vanilla/purpur/fabric) enforces this by
    // construction; exercise it via a public entry point.
    let vanilla_entries = vanilla_list_versions(
        &json!({"versions": [{"id": "1.21.11", "type": "release"}]}).to_string(),
    )
    .unwrap();
    assert_eq!(vanilla_entries[0].id, vanilla_entries[0].mc_version);

    // Install-step families (NeoForge/Forge): id is the paired
    // "{mc}—{loaderVersion}" string, not the bare mc_version.
    let neoforge_entries = neoforge_build_entries(
        "<metadata><versioning><versions><version>21.4.154</version></versions></versioning></metadata>",
    );
    assert_eq!(neoforge_entries[0].id, "1.21.4—21.4.154");
    assert_ne!(neoforge_entries[0].id, neoforge_entries[0].mc_version);

    let forge_entries = forge_parse_maven_metadata(
        "<metadata><versioning><versions><version>1.21.11-61.0.0</version></versions></versioning></metadata>",
    );
    assert_eq!(forge_entries[0].id, "1.21.11—61.0.0");
    assert_ne!(forge_entries[0].id, forge_entries[0].mc_version);
}

#[test]
fn server_versions_version_entry_isstable_rule_per_family() {
    // Vanilla/Purpur/Fabric/NeoForge: pre-filtered or hardcoded, so
    // isStable is trivially true on every returned entry.
    let vanilla = vanilla_list_versions(
        &json!({"versions": [{"id": "1.20", "type": "release"}]}).to_string(),
    )
    .unwrap();
    assert!(vanilla.iter().all(|e| e.is_stable));

    let purpur = purpur_list_versions(&json!({"versions": ["1.20"]}).to_string()).unwrap();
    assert!(purpur.iter().all(|e| e.is_stable));

    let fabric =
        fabric_list_versions(&json!([{"version": "1.20", "stable": true}]).to_string()).unwrap();
    assert!(fabric.iter().all(|e| e.is_stable));

    let neoforge = neoforge_build_entries(
        "<metadata><versioning><versions><version>20.4.237</version></versions></versioning></metadata>",
    );
    assert!(neoforge.iter().all(|e| e.is_stable));

    // Forge: genuinely varies per entry -- the only family where it does.
    let forge = forge_parse_maven_metadata(
        "<metadata><versioning><versions><version>1.20.1-47.4.5</version><version>1.20.1-47.4.5-beta</version></versions></versioning></metadata>",
    );
    let stable_flags: Vec<bool> = forge.iter().map(|e| e.is_stable).collect();
    assert!(stable_flags.contains(&true));
    assert!(stable_flags.contains(&false));
}

// --- Paper ---

#[test]
fn server_versions_paper_versions_flattened_and_sorted_numerically_descending() {
    let fixture = load("paper-versions-flattened-and-sorted-numerically-descending");
    let raw = fixture.input["raw_response"].to_string();
    let sorted = paper_flatten_and_sort(&raw).unwrap();

    let expected_first_10: Vec<String> = fixture.expected["sorted_first_10"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(&sorted[..10], expected_first_10.as_slice());

    let expected_last_2: Vec<String> = fixture.expected["sorted_last_2"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(&sorted[sorted.len() - 2..], expected_last_2.as_slice());

    assert_eq!(
        sorted.len() as i64,
        fixture.expected["total_count"].as_i64().unwrap()
    );
}

#[test]
fn server_versions_paper_20_candidate_cap_stops_scanning_once_hit() {
    let fixture = load("paper-20-candidate-cap-stops-scanning-once-hit");
    let candidates: Vec<String> = fixture.input["candidates_in_walk_order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let has_qualifying = fixture.input["has_qualifying_build"].clone();
    let limit = fixture.input["limit"].as_u64().unwrap() as usize;

    let outcome = paper_walk_candidates(&candidates, limit, |v| {
        if has_qualifying.get(v).and_then(Value::as_bool) == Some(true) {
            Some(())
        } else {
            None
        }
    });

    assert_eq!(
        outcome.tried as i64,
        fixture.expected["tried_count"].as_i64().unwrap()
    );
    assert_eq!(
        outcome.results.len() as i64,
        fixture.expected["results_count"].as_i64().unwrap()
    );
}

#[test]
fn server_versions_paper_best_build_selection_stable_wins_beta_fallback_when_absent() {
    let fixture = load("paper-best-build-selection-stable-wins-beta-fallback-when-absent");
    for (input_case, expected_case) in fixture.input["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(fixture.expected["cases"].as_array().unwrap())
    {
        let raw = raw_body_of(&input_case["raw_response"]);
        let include_experimental = input_case["include_experimental"].as_bool().unwrap();
        let selection = paper_select_build(&raw, include_experimental)
            .unwrap_or_else(|| panic!("case {}: expected Some", input_case["name"]));

        assert_eq!(
            selection.build_id,
            expected_case["selected_build_id"].as_i64().unwrap()
        );
        assert_eq!(
            selection.channel,
            expected_case["channel"].as_str().unwrap()
        );
        assert_eq!(
            selection.build_label(),
            expected_case["build_label"].as_str().unwrap()
        );
        assert_eq!(
            selection.is_stable,
            expected_case["is_stable"].as_bool().unwrap()
        );
    }
}

#[test]
fn server_versions_paper_experimental_track_skips_version_with_existing_stable_build() {
    let fixture = load("paper-experimental-track-skips-version-with-existing-stable-build");
    let raw = fixture.input["raw_response"].to_string();
    let selection = paper_select_build(&raw, true);
    assert!(selection.is_none());
}

#[test]
fn server_versions_build_entry_missing_download_url_is_skipped_not_fatal() {
    let fixture = load("build-entry-missing-download-url-is-skipped-not-fatal");
    let raw = fixture.input["raw_response"].to_string();
    let selection = paper_select_build(&raw, false).expect("expected Some");
    assert_eq!(
        selection.build_id,
        fixture.expected["selected_build_id"].as_i64().unwrap()
    );
}

// --- Purpur ---

#[test]
fn server_versions_purpur_versions_filtered_to_1_prefix_sorted_and_picker_shape() {
    let fixture = load("purpur-versions-filtered-to-1-prefix-sorted-and-picker-shape");
    let raw = fixture.input["raw_response"].to_string();
    let entries = purpur_list_versions(&raw).unwrap();

    let dropped: Vec<String> = fixture.expected["dropped_non_1_prefix"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    for d in &dropped {
        assert!(!entries.iter().any(|e| &e.id == d), "{d} should be dropped");
    }

    let first_6: Vec<String> = entries.iter().take(6).map(|e| e.id.clone()).collect();
    let expected_first_6: Vec<String> = fixture.expected["sorted_first_6"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(first_6, expected_first_6);

    assert_eq!(
        entries.len() as i64,
        fixture.expected["total_count"].as_i64().unwrap()
    );
    assert!(
        entries
            .iter()
            .all(|e| e.build_label.is_none() && e.is_stable)
    );
}

#[test]
fn server_versions_purpur_download_latest_prefers_paper_stable_version_when_present() {
    let fixture = load("purpur-download-latest-prefers-paper-stable-version-when-present");
    let purpur_versions: Vec<String> = fixture.input["purpur_versions_list"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let papers_stable = fixture.input["papers_stable_version"].as_str().unwrap();

    let target = purpur_pick_target_version(&purpur_versions, Some(papers_stable));
    assert_eq!(
        target.as_deref(),
        fixture.expected["target_version"].as_str()
    );
}

// --- Vanilla ---

#[test]
fn server_versions_vanilla_list_versions_filters_to_release_type_only_unsorted() {
    let fixture = load("vanilla-list-versions-filters-to-release-type-only-unsorted");
    let raw = fixture.input["raw_response"].to_string();
    let entries = vanilla_list_versions(&raw).unwrap();

    let expected_ids: Vec<String> = fixture.expected["result_ids_in_order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let actual_ids: Vec<String> = entries.iter().map(|e| e.id.clone()).collect();
    assert_eq!(actual_ids, expected_ids);
    assert!(
        entries
            .iter()
            .all(|e| e.is_stable && e.loader_version.is_none() && e.build_label.is_none())
    );
}

#[test]
fn server_versions_vanilla_download_latest_resolves_release_then_per_version_server_url() {
    let fixture = load("vanilla-download-latest-resolves-release-then-per-version-server-url");
    let manifest = fixture.input["manifest_response"].to_string();
    let per_version = fixture.input["per_version_metadata_response"].to_string();

    let (release_id, _meta_url) = vanilla_resolve_metadata_url(&manifest, None).unwrap();
    assert_eq!(
        release_id,
        fixture.expected["resolved_release_id"].as_str().unwrap()
    );

    let download_url = vanilla_server_download_url(&per_version, &release_id).unwrap();
    assert_eq!(
        download_url,
        fixture.expected["resolved_server_download_url"]
            .as_str()
            .unwrap()
    );
    // `build` is always the literal "release" for Vanilla -- not derived from
    // any function, so asserted here as documentation rather than a call.
    assert_eq!(fixture.expected["build_label"].as_str().unwrap(), "release");
}

// --- Fabric ---

#[test]
fn server_versions_fabric_list_versions_filters_stable_true_and_loaderversion_always_nil() {
    let fixture = load("fabric-list-versions-filters-stable-true-and-loaderversion-always-nil");
    let raw = fixture.input["raw_response"].to_string();
    let entries = fabric_list_versions(&raw).unwrap();

    let expected_ids: Vec<String> = fixture.expected["result_ids_in_order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let actual_ids: Vec<String> = entries.iter().map(|e| e.id.clone()).collect();
    assert_eq!(actual_ids, expected_ids);
    assert!(
        entries
            .iter()
            .all(|e| e.loader_version.is_none() && e.build_label.is_none() && e.is_stable)
    );
}

#[test]
fn server_versions_fabric_loader_and_installer_first_stable_selection() {
    let fixture = load("fabric-loader-and-installer-first-stable-selection");
    let loader_raw = fixture.input["loader_response_for_game_1_21_11"].to_string();
    let installer_raw = fixture.input["installer_list_response"].to_string();

    let loader = fabric_select_loader(&loader_raw).unwrap();
    assert_eq!(
        loader,
        fixture.expected["selected_loader_version"]
            .as_str()
            .unwrap()
    );

    let installer = fabric_first_stable_version(&installer_raw, "Fabric installer").unwrap();
    assert_eq!(
        installer,
        fixture.expected["selected_installer_version"]
            .as_str()
            .unwrap()
    );
}

#[test]
fn server_versions_fabric_first_stable_version_falls_back_to_first_entry_when_none_stable() {
    let fixture = load("fabric-first-stable-version-falls-back-to-first-entry-when-none-stable");
    let raw = raw_body_of(&fixture.input["raw_response"]);
    let loader = fabric_select_loader(&raw).unwrap();
    assert_eq!(
        loader,
        fixture.expected["selected_loader_version"]
            .as_str()
            .unwrap()
    );
}

// --- NeoForge ---

#[test]
fn server_versions_neoforge_stable_filter_excludes_hyphenated_versions() {
    let fixture = load("neoforge-stable-filter-excludes-hyphenated-versions");
    let xml = fixture.input["raw_metadata_xml"].as_str().unwrap();
    let stable = neoforge_stable_versions(xml);
    let expected: Vec<String> = fixture.expected["stable_after_filter"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(stable, expected);
}

#[test]
fn server_versions_neoforge_minecraft_version_derivation_classic_and_new_scheme() {
    let fixture = load("neoforge-minecraft-version-derivation-classic-and-new-scheme");
    let cases = fixture.input["cases"].as_array().unwrap();
    let expected: Vec<String> = fixture.expected["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    for (case, expected_mc) in cases.iter().zip(expected.iter()) {
        let nfv = case["neoforge_version"].as_str().unwrap();
        assert_eq!(&neoforge_minecraft_version(nfv), expected_mc, "case {nfv}");
    }
}

#[test]
fn server_versions_neoforge_version_pairs_sorted_mc_desc_then_build_desc_deduped() {
    let fixture = load("neoforge-version-pairs-sorted-mc-desc-then-build-desc-deduped");
    let xml = fixture.input["raw_metadata_xml"].as_str().unwrap();
    let entries = neoforge_build_entries(xml);

    let expected_entries = fixture.expected["sorted_entries"].as_array().unwrap();
    assert_eq!(entries.len(), expected_entries.len());
    for (actual, expected) in entries.iter().zip(expected_entries) {
        assert_eq!(actual.mc_version, expected["mcVersion"].as_str().unwrap());
        assert_eq!(
            actual.loader_version.as_deref(),
            expected["loaderVersion"].as_str()
        );
        assert_eq!(actual.id, expected["id"].as_str().unwrap());
    }
    assert!(entries.iter().all(|e| {
        e.build_label.as_deref().unwrap().starts_with("NeoForge ")
            && e.display_label == e.mc_version
            && e.is_stable
    }));
}

// --- Forge ---

#[test]
fn server_versions_forge_maven_metadata_parsed_to_mc_forge_pairs() {
    let fixture = load("forge-maven-metadata-parsed-to-mc-forge-pairs");
    for pair in fixture.expected["parsed_pairs"].as_array().unwrap() {
        let raw = pair["raw"].as_str().unwrap();
        let (mc, forge) =
            forge_parse_maven_version(raw).unwrap_or_else(|| panic!("{raw} should parse"));
        assert_eq!(mc, pair["mc"].as_str().unwrap());
        assert_eq!(forge, pair["forge"].as_str().unwrap());
    }
}

#[test]
fn server_versions_forge_version_pairs_sorted_mc_desc_then_forge_desc_same_mc_not_deduped() {
    let fixture = load("forge-version-pairs-sorted-mc-desc-then-forge-desc-same-mc-not-deduped");
    let xml = fixture.input["raw_metadata_xml"].as_str().unwrap();
    let entries = forge_parse_maven_metadata(xml);

    let expected_entries = fixture.expected["sorted_entries"].as_array().unwrap();
    assert_eq!(entries.len(), expected_entries.len());
    for (actual, expected) in entries.iter().zip(expected_entries) {
        assert_eq!(actual.mc_version, expected["mcVersion"].as_str().unwrap());
        assert_eq!(
            actual.loader_version.as_deref(),
            expected["loaderVersion"].as_str()
        );
        assert_eq!(actual.id, expected["id"].as_str().unwrap());
    }
    assert!(fixture.expected["every_entry_isstable"].as_bool().unwrap());
    assert!(entries.iter().all(|e| e.is_stable));
}

#[test]
fn server_versions_forge_latest_recommended_uses_promotions_not_stale_metadata_latest_tag() {
    let fixture = load("forge-latest-recommended-uses-promotions-not-stale-metadata-latest-tag");
    let promotions = fixture.input["promotions_response"].to_string();
    let (mc, forge) = forge_latest_recommended(&promotions).unwrap();
    assert_eq!(
        mc,
        fixture.expected["resolved_mc_version"].as_str().unwrap()
    );
    assert_eq!(
        forge,
        fixture.expected["resolved_forge_version"].as_str().unwrap()
    );
}

// --- Cross-cutting: malformed input, HTTP errors, empty lists, the floor filter ---

#[test]
fn server_versions_malformed_xml_metadata_silently_yields_empty_list_not_an_error() {
    let fixture = load("malformed-xml-metadata-silently-yields-empty-list-not-an-error");
    for case in fixture.input["cases"].as_array().unwrap() {
        let xml = case["raw_metadata_xml"].as_str().unwrap();
        match case["provider"].as_str().unwrap() {
            "neoforge" => assert!(neoforge_build_entries(xml).is_empty()),
            "forge" => assert!(forge_parse_maven_metadata(xml).is_empty()),
            other => panic!("unexpected provider {other}"),
        }
    }
}

#[test]
fn server_versions_malformed_json_response_rejected_as_invalid_shape() {
    let fixture = load("malformed-json-response-rejected-as-invalid-shape");
    for case in fixture.input["cases"].as_array().unwrap() {
        let body = case["raw_response_body"].as_str().unwrap();
        let name = case["name"].as_str().unwrap();
        let result = vanilla_list_versions(body);
        match name {
            "valid-json-wrong-shape" => {
                assert!(
                    matches!(result, Err(CatalogError::InvalidResponse(_))),
                    "{name}"
                );
            }
            "syntactically-invalid-json" => {
                assert!(
                    matches!(result, Err(CatalogError::InvalidJson(_))),
                    "{name}"
                );
            }
            other => panic!("unexpected case {other}"),
        }
    }
}

#[test]
fn server_versions_http_error_status_aborts_catalog_fetch() {
    let fixture = load("http-error-status-aborts-catalog-fetch");
    for (input_case, expected_case) in fixture.input["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(fixture.expected["cases"].as_array().unwrap())
    {
        let what = input_case["what"].as_str().unwrap();
        let status = input_case["http_status"].as_u64().unwrap() as u16;
        let err = ensure_http_ok(status, what).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!(
                "Network error: {}",
                expected_case["message"].as_str().unwrap()
            )
        );
    }
}

#[test]
fn server_versions_empty_version_list_rejected_not_unified_across_providers() {
    let fixture = load("empty-version-list-rejected-not-unified-across-providers");
    for case in fixture.input["cases"].as_array().unwrap() {
        match case["provider"].as_str().unwrap() {
            "paper" => {
                let raw = json!({"versions": case["raw_versions_field"]}).to_string();
                let result = paper_flatten_and_sort(&raw);
                assert!(matches!(result, Err(CatalogError::InvalidResponse(_))));
            }
            "fabric" => {
                let raw = case["raw_list"].to_string();
                let result = fabric_first_stable_version(&raw, "Fabric installer");
                assert!(matches!(result, Err(CatalogError::InvalidResponse(_))));
            }
            "neoforge" => {
                let result = neoforge_pick_latest_stable(&[]);
                assert!(matches!(result, Err(CatalogError::NoStableVersion)));
            }
            "purpur" => {
                let raw = json!({"versions": case["raw_versions_field"]}).to_string();
                let result = purpur_list_versions(&raw).unwrap();
                assert!(result.is_empty());
            }
            other => panic!("unexpected provider {other}"),
        }
    }
}

#[test]
fn server_versions_create_flow_1_20_floor_filter_applies_to_catalog_not_import() {
    let fixture = load("create-flow-1-20-floor-filter-applies-to-catalog-not-import");
    let pre_filter: Vec<String> = fixture.input["pre_filter_result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let post_filter = filter_to_create_flow_floor(&pre_filter);
    let expected: Vec<String> = fixture.expected["post_filter_result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(post_filter, expected);
}
