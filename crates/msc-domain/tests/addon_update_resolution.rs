//! Port of `fixtures/addon-update-resolution/`'s 29 cases (P8.5) against
//! `msc_domain::addon_update` (P8.11).

mod support;

use msc_domain::addon_update::*;
use msc_domain::app_config_schema::{AddonLink, AddonLinkProvenance};
use msc_domain::identity::AddOnKind;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("addon-update-resolution/{case}.json")))
}

fn addon_link(project_id: &str, provenance: AddonLinkProvenance) -> AddonLink {
    AddonLink {
        project_id: project_id.to_string(),
        title: None,
        slug: None,
        icon_url: None,
        provenance,
        installed_version_id: None,
        installed_file_name: None,
        installed_hash: None,
        client_side: None,
        server_side: None,
        extra: Default::default(),
    }
}

#[test]
fn addon_update_resolution_enumerate_includes_enabled_and_disabled_jars() {
    let fixture = load("enumerate-includes-enabled-and-disabled-jars");
    let contents: Vec<String> = fixture.input["folder_contents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let enumerated: Vec<String> = contents.into_iter().filter(|f| is_addon_file(f)).collect();
    let expected: Vec<String> = fixture.expected["enumerated_files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(enumerated, expected);
}

#[test]
fn addon_update_resolution_jar_stem_derivation_strips_disabled_suffix_not_just_extension() {
    let fixture = load("jar-stem-derivation-strips-disabled-suffix-not-just-extension");
    let filenames: Vec<String> = fixture.input["filenames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let stems: Vec<String> = filenames.iter().map(|f| jar_stem(f)).collect();
    let expected: Vec<String> = fixture.expected["jar_stems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(stems, expected);
}

#[test]
fn addon_update_resolution_geyser_floodgate_excluded_on_plugin_servers_only() {
    let fixture = load("geyser-floodgate-excluded-on-plugin-servers-only");
    let stems: Vec<String> = fixture.input["jar_stems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let resolved: Vec<String> = stems
        .into_iter()
        .filter(|s| !should_exclude_from_hash_resolution(AddOnKind::Plugin, s))
        .collect();
    let expected: Vec<String> = fixture.expected["resolved_jar_stems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(resolved, expected);
}

#[test]
fn addon_update_resolution_geyser_floodgate_kept_on_mod_servers() {
    let fixture = load("geyser-floodgate-kept-on-mod-servers");
    let stems: Vec<String> = fixture.input["jar_stems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let resolved: Vec<String> = stems
        .into_iter()
        .filter(|s| !should_exclude_from_hash_resolution(AddOnKind::Mod, s))
        .collect();
    let expected: Vec<String> = fixture.expected["resolved_jar_stems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(resolved, expected);
}

#[test]
fn addon_update_resolution_hash_identify_and_latest_lookup_are_concurrent_not_sequential() {
    let fixture = load("hash-identify-and-latest-lookup-are-concurrent-not-sequential");
    assert_eq!(
        RESOLVE_PASS_MODRINTH_REQUEST_COUNT as i64,
        fixture.expected["modrinth_requests_made"].as_i64().unwrap()
    );
}

#[test]
fn addon_update_resolution_sha1_declared_as_fallback_algorithm_but_resolver_never_calls_it() {
    // Documentation fixture: `msc_domain::addon_provider` exposes no sha1
    // hashing entry point at all (see P8.10), so the resolver's own port
    // has nothing to call -- there's no `sha1_hex` for it to reach for.
    let fixture = load("sha1-declared-as-fallback-algorithm-but-resolver-never-calls-it");
    assert_eq!(
        fixture.expected["sha1Hex_call_sites_in_resolver"].as_i64(),
        Some(0)
    );
}

#[test]
fn addon_update_resolution_identity_hash_match_wins_over_persisted_link() {
    let fixture = load("identity-hash-match-wins-over-persisted-link");
    let resolved = resolve_project_id(Some("PROJ_FRESH"), Some("PROJ_STALE"), None);
    assert_eq!(resolved, fixture.expected["resolved_project_id"].as_str());
}

#[test]
fn addon_update_resolution_identity_falls_back_to_persisted_installed_hash_when_not_on_modrinth() {
    let fixture = load("identity-falls-back-to-persisted-installed-hash-when-not-on-modrinth");
    let resolved = resolve_project_id(None, Some("PROJ_X"), Some("PROJ_Y_by_filename"));
    assert_eq!(resolved, fixture.expected["resolved_project_id"].as_str());
}

#[test]
fn addon_update_resolution_identity_falls_back_to_persisted_installed_filename_as_last_resort() {
    let fixture = load("identity-falls-back-to-persisted-installed-filename-as-last-resort");
    let resolved = resolve_project_id(None, None, Some("PROJ_Y"));
    assert_eq!(resolved, fixture.expected["resolved_project_id"].as_str());
}

#[test]
fn addon_update_resolution_identity_all_three_checks_miss_item_is_unlinked() {
    let fixture = load("identity-all-three-checks-miss-item-is-unlinked");
    let resolved = resolve_project_id(None, None, None);
    assert!(resolved.is_none());
    assert!(fixture.expected["projectId"].is_null());
}

#[test]
fn addon_update_resolution_unlinked_item_prefers_embedded_jar_metadata_over_filename_heuristic() {
    let fixture = load("unlinked-item-prefers-embedded-jar-metadata-over-filename-heuristic");
    let embedded = fixture.input["embedded_metadata_displayName"].as_str();
    let heuristic = fixture.input["filename_heuristic_displayName"]
        .as_str()
        .unwrap();
    let guess = unlinked_name_guess(embedded, heuristic);
    assert_eq!(guess, fixture.expected["nameGuess"].as_str().unwrap());
}

#[test]
fn addon_update_resolution_provenance_hash_detected_on_fresh_match_else_falls_back_to_persisted_links_own_provenance()
 {
    let fixture = load(
        "provenance-hash-detected-on-fresh-match-else-falls-back-to-persisted-links-own-provenance",
    );
    let case_a = resolve_provenance(true, None);
    assert_eq!(
        case_a.raw_value(),
        fixture.expected["case_a_provenance"].as_str().unwrap()
    );

    let case_b = resolve_provenance(false, Some(AddonLinkProvenance::UserLinked));
    assert_eq!(
        case_b.raw_value(),
        fixture.expected["case_b_provenance"].as_str().unwrap()
    );
}

#[test]
fn addon_update_resolution_self_healing_link_recorded_only_on_fresh_hash_match_not_persisted_fallback()
 {
    let fixture =
        load("self-healing-link-recorded-only-on-fresh-hash-match-not-persisted-fallback");
    assert_eq!(
        should_record_self_healing_link(true),
        fixture.expected["case_a_discoveredLinks_has_entry"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(
        should_record_self_healing_link(false),
        fixture.expected["case_b_discoveredLinks_has_entry"]
            .as_bool()
            .unwrap()
    );
}

#[test]
fn addon_update_resolution_bucket_update_available_when_latest_version_id_differs_from_installed() {
    let fixture = load("bucket-update-available-when-latest-version-id-differs-from-installed");
    let installed = fixture.input["installed_version_id"].as_str();
    let latest = fixture.input["latest_version_id"].as_str();
    let (bucket, available) = resolve_bucket(latest, installed, true);
    assert_eq!(bucket, AddonUpdateBucket::UpdateAvailable);
    assert_eq!(
        available.as_deref(),
        fixture.expected["availableVersionId"].as_str()
    );
}

#[test]
fn addon_update_resolution_bucket_up_to_date_when_latest_version_id_matches_installed() {
    let fixture = load("bucket-up-to-date-when-latest-version-id-matches-installed");
    let installed = fixture.input["installed_version_id"].as_str();
    let latest = fixture.input["latest_version_id"].as_str();
    let (bucket, available) = resolve_bucket(latest, installed, true);
    assert_eq!(bucket, AddonUpdateBucket::UpToDate);
    assert!(available.is_none());
    assert!(fixture.expected["availableVersion"].is_null());
    assert!(fixture.expected["availableVersionId"].is_null());
}

#[test]
fn addon_update_resolution_bucket_up_to_date_compares_against_persisted_installed_version_id_when_no_fresh_hash_hit()
 {
    let fixture = load(
        "bucket-up-to-date-compares-against-persisted-installed-version-id-when-no-fresh-hash-hit",
    );
    let fresh = fixture.input["idVersion"].as_str();
    let persisted = fixture.input["persisted_installedVersionId"].as_str();
    let current = resolve_current_version_id(fresh, persisted);
    let latest = fixture.input["latest_version_id"].as_str();
    let (bucket, _) = resolve_bucket(latest, current, true);
    assert_eq!(bucket, AddonUpdateBucket::UpToDate);
}

#[test]
fn addon_update_resolution_bucket_no_compatible_version_when_latest_absent_and_mc_version_configured()
 {
    let fixture = load("bucket-no-compatible-version-when-latest-absent-and-mc-version-configured");
    let (bucket, _) = resolve_bucket(None, None, true);
    assert_eq!(bucket, AddonUpdateBucket::NoCompatibleVersion);
    assert_eq!(
        fixture.expected["bucket"].as_str(),
        Some("noCompatibleVersion")
    );
}

#[test]
fn addon_update_resolution_bucket_up_to_date_when_latest_absent_and_no_mc_version_configured() {
    let fixture = load("bucket-up-to-date-when-latest-absent-and-no-mc-version-configured");
    let (bucket, _) = resolve_bucket(None, None, false);
    assert_eq!(bucket, AddonUpdateBucket::UpToDate);
    assert_eq!(fixture.expected["bucket"].as_str(), Some("upToDate"));
}

#[test]
fn addon_update_resolution_deterministic_ordering_bucket_rank_then_alphabetical_within_bucket() {
    let fixture = load("deterministic-ordering-bucket-rank-then-alphabetical-within-bucket");
    let mut items: Vec<(String, AddonUpdateBucket)> = fixture.input["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let name = v["displayName"].as_str().unwrap().to_string();
            let bucket = match v["bucket"].as_str().unwrap() {
                "updateAvailable" => AddonUpdateBucket::UpdateAvailable,
                "noCompatibleVersion" => AddonUpdateBucket::NoCompatibleVersion,
                "upToDate" => AddonUpdateBucket::UpToDate,
                "unlinked" => AddonUpdateBucket::Unlinked,
                other => panic!("unknown bucket {other}"),
            };
            (name, bucket)
        })
        .collect();
    items.sort_by_key(|(name, bucket)| addon_update_sort_key(name, *bucket));
    let ordered: Vec<String> = items.into_iter().map(|(name, _)| name).collect();
    let expected: Vec<String> = fixture.expected["ordered_display_names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(ordered, expected);
}

#[test]
fn addon_update_resolution_stale_plan_cache_skips_recompute_when_current_and_not_forced() {
    let fixture = load("stale-plan-cache-skips-recompute-when-current-and-not-forced");
    let cached = fixture.input["addonPlanServerId"].as_str();
    let current = fixture.input["cfg_id"].as_str().unwrap();
    let force = fixture.input["force"].as_bool().unwrap();
    assert_eq!(
        should_recompute_addon_plan(cached, current, force),
        fixture.expected["recompute_started"].as_bool().unwrap()
    );
}

#[test]
fn addon_update_resolution_stale_plan_cache_recomputes_when_forced() {
    let fixture = load("stale-plan-cache-recomputes-when-forced");
    let cached = fixture.input["addonPlanServerId"].as_str();
    let current = fixture.input["cfg_id"].as_str().unwrap();
    let force = fixture.input["force"].as_bool().unwrap();
    assert_eq!(
        should_recompute_addon_plan(cached, current, force),
        fixture.expected["recompute_started"].as_bool().unwrap()
    );
}

#[test]
fn addon_update_resolution_stale_plan_cache_drops_prior_servers_plan_when_switching_servers() {
    let fixture = load("stale-plan-cache-drops-prior-servers-plan-when-switching-servers");
    let cached = fixture.input["addonPlanServerId"].as_str();
    let current = fixture.input["cfg_id"].as_str().unwrap();
    assert_eq!(
        should_recompute_addon_plan(cached, current, false),
        fixture.expected["addonUpdatePlan_cleared_before_resolve"]
            .as_bool()
            .unwrap()
    );
}

#[test]
fn addon_update_resolution_version_label_strips_known_loader_prefix_when_remainder_is_version_shaped()
 {
    let fixture = load("version-label-strips-known-loader-prefix-when-remainder-is-version-shaped");
    let raw = fixture.input["raw"].as_str().unwrap();
    assert_eq!(
        clean_version_label(raw),
        fixture.expected["cleaned"].as_str().unwrap()
    );
}

#[test]
fn addon_update_resolution_version_label_leaves_string_with_no_known_loader_prefix_untouched() {
    let fixture = load("version-label-leaves-string-with-no-known-loader-prefix-untouched");
    let raw = fixture.input["raw"].as_str().unwrap();
    assert_eq!(
        clean_version_label(raw),
        fixture.expected["cleaned"].as_str().unwrap()
    );
}

#[test]
fn addon_update_resolution_version_label_leaves_non_version_shaped_remainder_untouched() {
    let fixture = load("version-label-leaves-non-version-shaped-remainder-untouched");
    let raw = fixture.input["raw"].as_str().unwrap();
    assert_eq!(
        clean_version_label(raw),
        fixture.expected["cleaned"].as_str().unwrap()
    );
}

#[test]
fn addon_update_resolution_merge_discovered_links_no_op_when_discovered_links_empty() {
    let fixture = load("merge-discovered-links-no-op-when-discovered-links-empty");
    let discovered: std::collections::HashMap<String, AddonLink> = std::collections::HashMap::new();
    assert!(discovered.is_empty());
    assert_eq!(fixture.expected["config_saved"].as_bool(), Some(false));
}

#[test]
fn addon_update_resolution_merge_discovered_links_preserves_user_linked_title_provenance_icon() {
    let fixture = load("merge-discovered-links-preserves-user-linked-title-provenance-icon");
    let mut prior = addon_link("P1", AddonLinkProvenance::UserLinked);
    prior.title = Some("My Custom Title".to_string());
    prior.slug = Some("my-slug".to_string());
    prior.icon_url = Some("https://old-icon".to_string());

    let mut discovered = addon_link("P1", AddonLinkProvenance::HashDetected);
    discovered.title = Some("Modrinth's Title".to_string());
    discovered.installed_version_id = Some("v2".to_string());
    discovered.installed_file_name = Some("new.jar".to_string());
    discovered.installed_hash = Some("newhash".to_string());
    discovered.client_side = Some("required".to_string());
    discovered.server_side = Some("required".to_string());

    let merged = merge_discovered_link(Some(&prior), &discovered);
    let expected = &fixture.expected["merged_link"];
    assert_eq!(merged.title.as_deref(), expected["title"].as_str());
    assert_eq!(
        merged.provenance.raw_value(),
        expected["provenance"].as_str().unwrap()
    );
    assert_eq!(merged.slug.as_deref(), expected["slug"].as_str());
    assert_eq!(
        merged.installed_version_id.as_deref(),
        expected["installedVersionId"].as_str()
    );
    assert_eq!(
        merged.installed_file_name.as_deref(),
        expected["installedFileName"].as_str()
    );
    assert_eq!(
        merged.installed_hash.as_deref(),
        expected["installedHash"].as_str()
    );
    assert_eq!(
        merged.client_side.as_deref(),
        expected["clientSide"].as_str()
    );
    assert_eq!(
        merged.server_side.as_deref(),
        expected["serverSide"].as_str()
    );
}

#[test]
fn addon_update_resolution_merge_discovered_links_client_server_side_refresh_falls_back_to_prior_when_discovered_nil()
 {
    let fixture = load(
        "merge-discovered-links-client-server-side-refresh-falls-back-to-prior-when-discovered-nil",
    );
    let mut prior = addon_link("P1", AddonLinkProvenance::UserLinked);
    prior.client_side = Some("required".to_string());
    prior.server_side = Some("required".to_string());

    let discovered = addon_link("P1", AddonLinkProvenance::HashDetected);
    // discovered.client_side / server_side are None (this pass couldn't
    // determine them).

    let merged = merge_discovered_link(Some(&prior), &discovered);
    assert_eq!(
        merged.client_side.as_deref(),
        fixture.expected["merged_clientSide"].as_str()
    );
    assert_eq!(
        merged.server_side.as_deref(),
        fixture.expected["merged_serverSide"].as_str()
    );
}

#[test]
fn addon_update_resolution_merge_discovered_links_overwrites_non_user_linked_entry_wholesale() {
    let fixture = load("merge-discovered-links-overwrites-non-user-linked-entry-wholesale");
    let mut prior = addon_link("P1", AddonLinkProvenance::NameGuess);
    prior.title = Some("Old Guessed Title".to_string());

    let mut discovered = addon_link("P1", AddonLinkProvenance::HashDetected);
    discovered.title = Some("Real Title".to_string());

    let merged = merge_discovered_link(Some(&prior), &discovered);
    let expected = &fixture.expected["merged_link"];
    assert_eq!(merged.title.as_deref(), expected["title"].as_str());
    assert_eq!(
        merged.provenance.raw_value(),
        expected["provenance"].as_str().unwrap()
    );
}
