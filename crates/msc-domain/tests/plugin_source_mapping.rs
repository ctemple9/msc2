//! Port of `fixtures/plugin-source-mapping/`'s 24 cases (P8.5) against
//! `msc_domain::addon_update` (P8.11).
//!
//! Two fixtures in this directory (`set-plugin-source-strips-old-prefix-matching-entries-before-writing-new-key`,
//! `remove-plugin-source-also-strips-prefix-matching-entries`) claim a
//! stale-prefix strip between `"LuckPerms-5.4.141"` and `"LuckPerms-5.5.0"`
//! -- reading the real oracle (`AppViewModel+PluginManagement.swift:214-229`)
//! directly confirms the check is the same literal symmetric `hasPrefix`
//! `findSource` uses, and those two strings are NOT prefix-related in
//! either direction (they diverge at "5.4" vs "5.5"). This looks like a
//! P8.5 fixture-authoring slip, not a real case -- the same class of issue
//! P8.10 found in `plugin-source-resolution`'s `strip-scheme` fixture.
//! Both tests below exercise the confirmed real behavior (a genuinely
//! prefix-related pair) rather than hard-coding the fixture's own
//! internally-inconsistent expectation; flagged for Cameron in
//! `rolling-plan.md`'s P8.11 report.

mod support;

use msc_domain::addon_update::*;
use msc_domain::app_config_schema::{PluginSourceConfig, PluginSourceKind};
use std::collections::HashMap;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("plugin-source-mapping/{case}.json")))
}

fn source(url: &str, kind: PluginSourceKind) -> PluginSourceConfig {
    PluginSourceConfig {
        url: url.to_string(),
        source_type: kind,
        extra: Default::default(),
    }
}

fn sources_map(entries: &[(&str, &str)]) -> HashMap<String, PluginSourceConfig> {
    entries
        .iter()
        .map(|(k, url)| (k.to_string(), source(url, PluginSourceKind::Modrinth)))
        .collect()
}

#[test]
fn plugin_source_mapping_find_source_exact_match_preferred_over_prefix_match() {
    let fixture = load("find-source-exact-match-preferred-over-prefix-match");
    let sources = sources_map(&[
        (
            "LuckPerms-Bukkit-5.4.141",
            "https://modrinth.com/plugin/luckperms-exact",
        ),
        ("LuckPerms", "https://modrinth.com/plugin/luckperms-prefix"),
    ]);
    let found = find_source("LuckPerms-Bukkit-5.4.141", &sources).unwrap();
    assert_eq!(
        found.url,
        fixture.expected["matched_source_url"].as_str().unwrap()
    );
}

#[test]
fn plugin_source_mapping_find_source_prefix_match_when_current_stem_is_longer_than_key() {
    let fixture = load("find-source-prefix-match-when-current-stem-is-longer-than-key");
    let sources = sources_map(&[("LuckPerms", "https://modrinth.com/plugin/luckperms")]);
    let found = find_source("LuckPerms-5.4.141", &sources).unwrap();
    assert_eq!(
        found.url,
        fixture.expected["matched_source_url"].as_str().unwrap()
    );
}

#[test]
fn plugin_source_mapping_find_source_prefix_match_when_key_is_longer_than_current_stem() {
    let fixture = load("find-source-prefix-match-when-key-is-longer-than-current-stem");
    let sources = sources_map(&[(
        "LuckPerms-Bukkit-5.4.141",
        "https://modrinth.com/plugin/luckperms",
    )]);
    let found = find_source("LuckPerms", &sources).unwrap();
    assert_eq!(
        found.url,
        fixture.expected["matched_source_url"].as_str().unwrap()
    );
}

#[test]
fn plugin_source_mapping_find_source_returns_nil_when_no_match() {
    let fixture = load("find-source-returns-nil-when-no-match");
    let sources = sources_map(&[("LuckPerms-Bukkit-5.4.141", "https://x")]);
    let found = find_source("TotallyDifferentPlugin", &sources);
    assert!(found.is_none());
    assert!(fixture.expected["matched_source_url"].is_null());
}

#[test]
fn plugin_source_mapping_tier_managed_for_geyser_jar_regardless_of_source_config() {
    let fixture = load("tier-managed-for-geyser-jar-regardless-of-source-config");
    let sources = sources_map(&[("geyser-spigot", "https://x")]);
    let tier = derive_plugin_tier("geyser-spigot", &sources);
    assert_eq!(format!("{tier:?}").to_lowercase(), "managed");
    assert_eq!(fixture.expected["tier"].as_str(), Some("managed"));
}

#[test]
fn plugin_source_mapping_tier_managed_for_floodgate_jar() {
    let fixture = load("tier-managed-for-floodgate-jar");
    let sources = HashMap::new();
    let tier = derive_plugin_tier(fixture.input["jarStem"].as_str().unwrap(), &sources);
    assert_eq!(tier, PluginTier::Managed);
    assert_eq!(fixture.expected["tier"].as_str(), Some("managed"));
}

#[test]
fn plugin_source_mapping_tier_user_sourced_when_source_config_found_and_not_managed() {
    let fixture = load("tier-user-sourced-when-source-config-found-and-not-managed");
    let sources = sources_map(&[(
        "LuckPerms-Bukkit-5.4.141",
        "https://modrinth.com/plugin/luckperms",
    )]);
    let tier = derive_plugin_tier("LuckPerms-Bukkit-5.4.141", &sources);
    assert_eq!(tier, PluginTier::UserSourced);
    assert_eq!(fixture.expected["tier"].as_str(), Some("userSourced"));
}

#[test]
fn plugin_source_mapping_tier_unmanaged_when_no_source_config_and_not_managed() {
    let fixture = load("tier-unmanaged-when-no-source-config-and-not-managed");
    let sources = HashMap::new();
    let tier = derive_plugin_tier(fixture.input["jarStem"].as_str().unwrap(), &sources);
    assert_eq!(tier, PluginTier::Unmanaged);
    assert_eq!(fixture.expected["tier"].as_str(), Some("unmanaged"));
}

#[test]
fn plugin_source_mapping_sort_order_managed_before_user_sourced_before_unmanaged() {
    let fixture = load("sort-order-managed-before-user-sourced-before-unmanaged");
    let mut entries = [
        ("zeta-jar", "Zeta Unmanaged", PluginTier::Unmanaged),
        ("alpha-jar", "Alpha UserSourced", PluginTier::UserSourced),
        (
            "beta-jar",
            "Beta Managed (not geyser/floodgate-named)",
            PluginTier::Managed,
        ),
    ];
    entries.sort_by_key(|(stem, name, tier)| plugin_entry_sort_key(stem, name, *tier));
    let ordered: Vec<&str> = entries.iter().map(|(_, name, _)| *name).collect();
    let expected: Vec<String> = fixture.expected["ordered_display_names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(ordered, expected);
}

#[test]
fn plugin_source_mapping_sort_managed_tier_geyser_before_floodgate_by_convention() {
    let fixture = load("sort-managed-tier-geyser-before-floodgate-by-convention");
    let mut entries = [
        ("floodgate-spigot", "Floodgate", PluginTier::Managed),
        ("geyser-spigot", "Geyser", PluginTier::Managed),
    ];
    entries.sort_by_key(|(stem, name, tier)| plugin_entry_sort_key(stem, name, *tier));
    let ordered: Vec<&str> = entries.iter().map(|(_, name, _)| *name).collect();
    let expected: Vec<String> = fixture.expected["ordered_display_names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(ordered, expected);
}

#[test]
fn plugin_source_mapping_sort_alphabetical_by_display_name_within_tier() {
    let fixture = load("sort-alphabetical-by-display-name-within-tier");
    let mut entries = [
        ("zeta-jar", "Zeta Plugin", PluginTier::UserSourced),
        ("alpha-jar", "alpha plugin", PluginTier::UserSourced),
    ];
    entries.sort_by_key(|(stem, name, tier)| plugin_entry_sort_key(stem, name, *tier));
    let ordered: Vec<&str> = entries.iter().map(|(_, name, _)| *name).collect();
    let expected: Vec<String> = fixture.expected["ordered_display_names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(ordered, expected);
}

#[test]
fn plugin_source_mapping_set_plugin_source_creates_new_entry() {
    let fixture = load("set-plugin-source-creates-new-entry");
    let mut sources = HashMap::new();
    set_plugin_source(
        &mut sources,
        "NewPlugin-1.0",
        source(
            "https://modrinth.com/plugin/newplugin",
            PluginSourceKind::Modrinth,
        ),
    );
    assert_eq!(sources.len(), 1);
    assert_eq!(
        sources["NewPlugin-1.0"].url,
        "https://modrinth.com/plugin/newplugin"
    );
    assert_eq!(fixture.expected["config_saved"].as_bool(), Some(true));
}

#[test]
fn plugin_source_mapping_set_plugin_source_strips_old_prefix_matching_entries_before_writing_new_key()
 {
    // See the module-level doc comment: the fixture's own literal data
    // ("LuckPerms-5.4.141" vs "LuckPerms-5.5.0") isn't actually
    // prefix-related, so this exercises the confirmed real rule against a
    // genuinely prefix-related pair instead.
    let mut sources = sources_map(&[("LuckPerms", "https://old-url")]);
    set_plugin_source(
        &mut sources,
        "LuckPerms-5.5.0",
        source("https://new-url", PluginSourceKind::Modrinth),
    );
    assert_eq!(sources.len(), 1);
    assert_eq!(sources["LuckPerms-5.5.0"].url, "https://new-url");
}

#[test]
fn plugin_source_mapping_remove_plugin_source_deletes_exact_key() {
    let sources = sources_map(&[("LuckPerms-Bukkit-5.4.141", "https://x")]);
    let after = remove_plugin_source(sources, "LuckPerms-Bukkit-5.4.141");
    assert!(after.is_none());
}

#[test]
fn plugin_source_mapping_remove_plugin_source_also_strips_prefix_matching_entries() {
    // See module-level doc comment.
    let sources = sources_map(&[("LuckPerms", "https://stale")]);
    let after = remove_plugin_source(sources, "LuckPerms-5.5.0");
    assert!(after.is_none());
}

#[test]
fn plugin_source_mapping_remove_plugin_source_sets_map_to_nil_when_empty_after_removal() {
    let fixture = load("remove-plugin-source-sets-map-to-nil-when-empty-after-removal");
    let sources = sources_map(&[("OnlyPlugin", "https://x")]);
    let after = remove_plugin_source(sources, "OnlyPlugin");
    assert!(after.is_none());
    assert!(fixture.expected["pluginSources_after"].is_null());
}

#[test]
fn plugin_source_mapping_old_jars_with_matching_display_name_prefix_removed_before_final_move() {
    let fixture = load("old-jars-with-matching-display-name-prefix-removed-before-final-move");
    let files: Vec<String> = fixture.input["pluginsDir_contents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let display_name = fixture.input["entry_displayName"].as_str().unwrap();
    let removed = stale_jars_to_remove(&files, display_name);
    let expected: Vec<String> = fixture.expected["removed_before_move"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(removed, expected);
}

#[test]
fn plugin_source_mapping_final_filename_derived_from_download_url_last_path_component_when_it_ends_in_jar()
 {
    let fixture =
        load("final-filename-derived-from-download-url-last-path-component-when-it-ends-in-jar");
    let url = fixture.input["downloadURL"].as_str().unwrap();
    let name = plugin_final_filename(url, "unused", None);
    assert_eq!(name, fixture.expected["finalName"].as_str().unwrap());
}

#[test]
fn plugin_source_mapping_final_filename_falls_back_to_display_name_and_version_when_url_has_no_jar_suffix()
 {
    let fixture =
        load("final-filename-falls-back-to-display-name-and-version-when-url-has-no-jar-suffix");
    let url = fixture.input["downloadURL"].as_str().unwrap();
    let display_name = fixture.input["entry_displayName"].as_str().unwrap();
    let version = fixture.input["entry_onlineVersion"].as_str();
    let name = plugin_final_filename(url, display_name, version);
    assert_eq!(name, fixture.expected["finalName"].as_str().unwrap());
}

#[test]
fn plugin_source_mapping_rekey_after_download_when_final_filename_stem_differs_from_entry_stem() {
    let fixture = load("rekey-after-download-when-final-filename-stem-differs-from-entry-stem");
    let entry_stem = fixture.input["entry_jarStem"].as_str().unwrap();
    let final_filename = fixture.input["final_filename"].as_str().unwrap();
    let rekeyed = plugin_source_rekey(entry_stem, final_filename);
    assert_eq!(
        rekeyed.as_deref(),
        fixture.expected["new_source_key_written"].as_str()
    );
}

#[test]
fn plugin_source_mapping_rekey_no_op_when_final_filename_stem_matches_entry_stem() {
    let fixture = load("rekey-no-op-when-final-filename-stem-matches-entry-stem");
    let entry_stem = fixture.input["entry_jarStem"].as_str().unwrap();
    let final_filename = fixture.input["final_filename"].as_str().unwrap();
    let rekeyed = plugin_source_rekey(entry_stem, final_filename);
    assert!(rekeyed.is_none());
    assert_eq!(
        fixture.expected["setPluginSource_called"].as_bool(),
        Some(false)
    );
}

#[test]
fn plugin_source_mapping_direct_source_download_skips_online_check_uses_literal_direct_version() {
    let fixture = load("direct-source-download-skips-online-check-uses-literal-direct-version");
    let dispatch = plugin_version_dispatch(PluginSourceKind::Direct);
    assert_eq!(
        dispatch,
        PluginVersionDispatch::DirectImmediate {
            version: "(direct)"
        }
    );
    assert_eq!(
        fixture.expected["fetchOnlineVersion_called"].as_bool(),
        Some(false)
    );
    assert_eq!(
        fixture.expected["onlineVersion_set_to"].as_str(),
        Some("(direct)")
    );

    assert_eq!(
        plugin_version_dispatch(PluginSourceKind::Modrinth),
        PluginVersionDispatch::FetchOnlineFirst
    );
}

#[test]
fn plugin_source_mapping_managed_plugin_online_version_mirrored_from_geyser_floodgate_snapshot_not_fetched_per_plugin()
 {
    let fixture = load(
        "managed-plugin-online-version-mirrored-from-geyser-floodgate-snapshot-not-fetched-per-plugin",
    );
    let geyser = fixture.input["geyserOnline"].as_str().unwrap();
    let floodgate = fixture.input["floodgateOnline"].as_str().unwrap();
    assert_eq!(
        managed_plugin_online_version("geyser-spigot", geyser, floodgate).as_deref(),
        fixture.expected["geyser_entry_onlineVersion"].as_str()
    );
    assert_eq!(
        managed_plugin_online_version("floodgate-spigot", geyser, floodgate).as_deref(),
        fixture.expected["floodgate_entry_onlineVersion"].as_str()
    );
}

#[test]
fn plugin_source_mapping_managed_plugins_excluded_from_user_sourced_online_check_pass() {
    let fixture = load("managed-plugins-excluded-from-user-sourced-online-check-pass");
    let entries = [
        ("geyser-spigot", "geyser-spigot"),
        ("LuckPerms-Bukkit", "LuckPerms-Bukkit"),
        ("SomePlugin", "SomePlugin"),
    ];
    // Managed/unmanaged come from the derivation rule directly; userSourced
    // needs a source entry, so re-derive against a map that has one for
    // LuckPerms-Bukkit only, matching this fixture's own input shape.
    let mut with_source = HashMap::new();
    with_source.insert(
        "LuckPerms-Bukkit".to_string(),
        source("https://x", PluginSourceKind::Modrinth),
    );
    let checked: Vec<&str> = entries
        .iter()
        .filter(|(stem, _)| derive_plugin_tier(stem, &with_source) == PluginTier::UserSourced)
        .map(|(_, name)| *name)
        .collect();
    let expected: Vec<String> = fixture.expected["plugins_checked"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(checked, expected);
}
