//! Port of `fixtures/modpack-client-only/`'s 28 cases (P6/P7 + P8.6) and
//! the policy subset of `fixtures/pack-managed-guard/`'s 17 cases (P8.6)
//! against `msc_domain::modpack` (P8.12).
//!
//! Six `pack-managed-guard` fixtures are deliberately NOT re-tested here,
//! each already covered elsewhere and cited so this isn't silent scope
//! creep: `pack-managed-defaults-false`, `pack-managed-encoded-key-name`,
//! `pack-provenance-round-trip`, and `old-json-missing-pack-fields-decodes-cleanly`
//! exercise `ConfigServer.pack_managed`/`pack_name`/`pack_version`, already
//! ported and tested by `app_config_schema.rs`/`tests/app_config_schema.rs`
//! since Phase 7 (P7.8) -- this step doesn't touch that file. `addons-response-*`
//! (x2) exercise `AddonsResponseDTO`'s pack fields on the API/client-model
//! layer, which belongs to P8.24 (routes)/the copied iOS models, not
//! `msc-domain`.

mod support;

use msc_domain::modpack::*;
use support::Fixture;

fn load_client_only(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("modpack-client-only/{case}.json")))
}

fn load_pack_guard(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("pack-managed-guard/{case}.json")))
}

// --- is_mods_jar ---

#[test]
fn modpack_policy_client_only_is_mods_jar() {
    let fixture = load_client_only("is-mods-jar");
    let paths: Vec<String> = fixture.input["paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let results: Vec<bool> = paths.iter().map(|p| is_mods_jar(p)).collect();
    let expected: Vec<bool> = fixture.expected["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_bool().unwrap())
        .collect();
    assert_eq!(results, expected);
}

// --- isManifestServerUnsupported ---

fn env_from(v: &serde_json::Value) -> Option<MrpackEnv> {
    if v.is_null() {
        return None;
    }
    Some(MrpackEnv {
        client: v["client"].as_str().map(str::to_string),
        server: v["server"].as_str().map(str::to_string),
    })
}

#[test]
fn modpack_policy_client_only_manifest_env_server_supported_is_not_client_only() {
    let fixture = load_client_only("manifest-env-server-supported-is-not-client-only");
    let envs = fixture.input["envs"].as_array().unwrap();
    let results: Vec<bool> = envs
        .iter()
        .map(|e| is_manifest_server_unsupported(env_from(e).as_ref()))
        .collect();
    let expected: Vec<bool> = fixture.expected["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_bool().unwrap())
        .collect();
    assert_eq!(results, expected);
}

#[test]
fn modpack_policy_client_only_manifest_env_unsupported_is_client_only() {
    let fixture = load_client_only("manifest-env-unsupported-is-client-only");
    let envs = fixture.input["envs"].as_array().unwrap();
    let results: Vec<bool> = envs
        .iter()
        .map(|e| is_manifest_server_unsupported(env_from(e).as_ref()))
        .collect();
    let expected: Vec<bool> = fixture.expected["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_bool().unwrap())
        .collect();
    assert_eq!(results, expected);
}

// --- Tier 0: known_client_only_reason ---

#[test]
fn modpack_policy_client_only_known_client_only_exact_stem_match() {
    let fixture = load_client_only("known-client-only-exact-stem-match");
    let stem = fixture.input["jarStem"].as_str().unwrap();
    let reason = known_client_only_reason(stem);
    assert!(reason.is_some());
    assert!(
        reason
            .unwrap()
            .contains("Known client-only shader/renderer mod")
    );
}

#[test]
fn modpack_policy_client_only_known_client_only_hyphen_separator_with_version_suffix() {
    let fixture = load_client_only("known-client-only-hyphen-separator-with-version-suffix");
    let stem = fixture.input["jarStem"].as_str().unwrap();
    assert!(known_client_only_reason(stem).is_some());
}

#[test]
fn modpack_policy_client_only_known_client_only_is_case_insensitive() {
    let fixture = load_client_only("known-client-only-is-case-insensitive");
    let stem = fixture.input["jarStem"].as_str().unwrap();
    assert!(known_client_only_reason(stem).is_some());
}

#[test]
fn modpack_policy_client_only_known_client_only_nil_for_non_matching_stem() {
    let fixture = load_client_only("known-client-only-nil-for-non-matching-stem");
    let stem = fixture.input["jarStem"].as_str().unwrap();
    assert!(known_client_only_reason(stem).is_none());
}

#[test]
fn modpack_policy_client_only_known_client_only_plus_separator() {
    let fixture = load_client_only("known-client-only-plus-separator");
    let stem = fixture.input["jarStem"].as_str().unwrap();
    assert!(known_client_only_reason(stem).is_some());
}

#[test]
fn modpack_policy_client_only_known_client_only_substring_without_separator_does_not_match() {
    let fixture = load_client_only("known-client-only-substring-without-separator-does-not-match");
    let stem = fixture.input["jarStem"].as_str().unwrap();
    assert!(known_client_only_reason(stem).is_none());
}

#[test]
fn modpack_policy_client_only_known_client_only_underscore_separator() {
    let fixture = load_client_only("known-client-only-underscore-separator");
    let stem = fixture.input["jarStem"].as_str().unwrap();
    assert!(known_client_only_reason(stem).is_some());
}

// --- Tier 2/3: client_only_reason ---

#[test]
fn modpack_policy_client_only_jar_client_fallback_when_modrinth_unknown() {
    let fixture = load_client_only("jar-client-fallback-when-modrinth-unknown");
    let reason = client_only_reason(
        fixture.input["modrinthServerSide"].as_str(),
        fixture.input["modrinthProjectTitle"].as_str(),
        fixture.input["jarEnvironment"].as_str(),
    );
    assert!(reason.is_some());
    assert!(reason.unwrap().contains("fabric.mod.json"));
}

#[test]
fn modpack_policy_client_only_jar_server_fallback_keeps_enabled() {
    let fixture = load_client_only("jar-server-fallback-keeps-enabled");
    let reason = client_only_reason(
        fixture.input["modrinthServerSide"].as_str(),
        fixture.input["modrinthProjectTitle"].as_str(),
        fixture.input["jarEnvironment"].as_str(),
    );
    assert!(reason.is_none());
}

#[test]
fn modpack_policy_client_only_modrinth_optional_keeps_enabled() {
    let fixture = load_client_only("modrinth-optional-keeps-enabled");
    let reason = client_only_reason(
        fixture.input["modrinthServerSide"].as_str(),
        fixture.input["modrinthProjectTitle"].as_str(),
        fixture.input["jarEnvironment"].as_str(),
    );
    assert!(reason.is_none());
}

#[test]
fn modpack_policy_client_only_modrinth_required_keeps_enabled_even_if_jar_says_client() {
    let fixture = load_client_only("modrinth-required-keeps-enabled-even-if-jar-says-client");
    let reason = client_only_reason(
        fixture.input["modrinthServerSide"].as_str(),
        fixture.input["modrinthProjectTitle"].as_str(),
        fixture.input["jarEnvironment"].as_str(),
    );
    assert!(reason.is_none());
}

#[test]
fn modpack_policy_client_only_modrinth_unsupported_disables() {
    let fixture = load_client_only("modrinth-unsupported-disables");
    let reason = client_only_reason(
        fixture.input["modrinthServerSide"].as_str(),
        fixture.input["modrinthProjectTitle"].as_str(),
        fixture.input["jarEnvironment"].as_str(),
    )
    .unwrap();
    assert!(reason.contains("Continuity"));
    assert!(reason.contains("Modrinth"));
}

#[test]
fn modpack_policy_client_only_modrinth_unsupported_wins_over_jar_server() {
    let fixture = load_client_only("modrinth-unsupported-wins-over-jar-server");
    let reason = client_only_reason(
        fixture.input["modrinthServerSide"].as_str(),
        fixture.input["modrinthProjectTitle"].as_str(),
        fixture.input["jarEnvironment"].as_str(),
    );
    assert!(reason.is_some());
}

#[test]
fn modpack_policy_client_only_no_signals_keeps_enabled() {
    let fixture = load_client_only("no-signals-keeps-enabled");
    let cases = fixture.input["cases"].as_array().unwrap();
    let expected = fixture.expected["results"].as_array().unwrap();
    for (case, exp) in cases.iter().zip(expected.iter()) {
        let reason = client_only_reason(
            case["modrinthServerSide"].as_str(),
            case["modrinthProjectTitle"].as_str(),
            case["jarEnvironment"].as_str(),
        );
        assert_eq!(reason.is_none(), exp["isNil"].as_bool().unwrap());
    }
}

// --- disabled_url ---

#[test]
fn modpack_policy_client_only_disabled_url_appends_disabled_extension() {
    let fixture = load_client_only("disabled-url-appends-disabled-extension");
    let url = fixture.input["jarURL"].as_str().unwrap();
    assert_eq!(
        disabled_url(url),
        fixture.expected["disabledURL"].as_str().unwrap()
    );
}

#[test]
fn modpack_policy_client_only_disabled_url_appends_rather_than_replaces_jar_extension() {
    let fixture = load_client_only("disabled-url-appends-rather-than-replaces-jar-extension");
    let url = fixture.input["jarURL"].as_str().unwrap();
    let result = disabled_url(url);
    let last = result.rsplit('/').next().unwrap();
    assert_eq!(
        last,
        fixture.expected["disabledURL_lastPathComponent"]
            .as_str()
            .unwrap()
    );
}

#[test]
fn modpack_policy_client_only_disabled_url_is_pure_no_filesystem_access() {
    let fixture = load_client_only("disabled-url-is-pure-no-filesystem-access");
    let url = fixture.input["jarURL"].as_str().unwrap();
    assert_eq!(
        disabled_url(url),
        fixture.expected["disabledURL"].as_str().unwrap()
    );
}

// --- disableJar decision ---

#[test]
fn modpack_policy_client_only_disable_jar_renames_to_disabled() {
    let _fixture = load_client_only("disable-jar-renames-to-disabled");
    assert_eq!(
        decide_disable_jar_action(true, false),
        DisableJarAction::Rename
    );
}

#[test]
fn modpack_policy_client_only_disable_jar_never_clobbers_existing_disabled() {
    let _fixture = load_client_only("disable-jar-never-clobbers-existing-disabled");
    assert_eq!(
        decide_disable_jar_action(true, true),
        DisableJarAction::DropActiveKeepExistingDisabled
    );
}

#[test]
fn modpack_policy_client_only_disable_jar_returns_nil_when_nothing_to_disable() {
    let _fixture = load_client_only("disable-jar-returns-nil-when-nothing-to-disable");
    assert_eq!(
        decide_disable_jar_action(false, false),
        DisableJarAction::NoOp
    );
}

// --- modrinth_project_id ---

#[test]
fn modpack_policy_client_only_project_id_from_modrinth_cdn_url() {
    let fixture = load_client_only("project-id-from-modrinth-cdn-url");
    let urls: Vec<String> = fixture.input["downloadURLs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        modrinth_project_id(&urls),
        fixture.expected.as_str().map(str::to_string)
    );
}

#[test]
fn modpack_policy_client_only_project_id_nil_for_empty_downloads() {
    let urls: Vec<String> = Vec::new();
    assert!(modrinth_project_id(&urls).is_none());
}

#[test]
fn modpack_policy_client_only_project_id_nil_for_malformed_modrinth_url() {
    let fixture = load_client_only("project-id-nil-for-malformed-modrinth-url");
    let urls: Vec<String> = fixture.input["downloadURLs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(modrinth_project_id(&urls).is_none());
}

#[test]
fn modpack_policy_client_only_project_id_nil_for_non_modrinth_urls() {
    let fixture = load_client_only("project-id-nil-for-non-modrinth-urls");
    let urls: Vec<String> = fixture.input["downloadURLs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(modrinth_project_id(&urls).is_none());
}

#[test]
fn modpack_policy_client_only_project_id_prefers_modrinth_among_multiple_mirrors() {
    let fixture = load_client_only("project-id-prefers-modrinth-among-multiple-mirrors");
    let urls: Vec<String> = fixture.input["downloadURLs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        modrinth_project_id(&urls),
        fixture.expected.as_str().map(str::to_string)
    );
}

// --- Pack-managed mutation guard ---

#[test]
fn modpack_policy_pack_managed_guard_non_pack_managed_server_mutations_proceed_normally() {
    let fixture = load_pack_guard("non-pack-managed-server-mutations-proceed-normally");
    assert!(!pack_mutation_refused(false, AddonMutationKind::Install));
    assert_eq!(fixture.expected["refused"].as_bool(), Some(false));
}

#[test]
fn modpack_policy_pack_managed_server_allows_individual_install() {
    let fixture = load_pack_guard("pack-managed-refuses-individual-install");
    assert!(!pack_mutation_refused(true, AddonMutationKind::Install));
    assert_eq!(fixture.expected["refused"].as_bool(), Some(false));
}

#[test]
fn modpack_policy_pack_managed_server_allows_individual_remove() {
    let fixture = load_pack_guard("pack-managed-refuses-individual-remove");
    assert!(!pack_mutation_refused(true, AddonMutationKind::Remove));
    assert_eq!(fixture.expected["refused"].as_bool(), Some(false));
}

#[test]
fn modpack_policy_pack_managed_server_allows_individual_toggle() {
    let fixture = load_pack_guard("pack-managed-refuses-individual-toggle");
    assert!(!pack_mutation_refused(true, AddonMutationKind::Toggle));
    assert_eq!(fixture.expected["refused"].as_bool(), Some(false));
}

#[test]
fn modpack_policy_pack_managed_server_allows_individual_update() {
    let fixture = load_pack_guard("pack-managed-refuses-individual-update");
    assert!(!pack_mutation_refused(true, AddonMutationKind::Update));
    assert_eq!(fixture.expected["refused"].as_bool(), Some(false));
}

#[test]
fn modpack_policy_pack_managed_guard_pack_managed_allows_explicit_whole_pack_replacement() {
    let fixture = load_pack_guard("pack-managed-allows-explicit-whole-pack-replacement");
    assert!(!pack_replace_refused(true, true));
    assert_eq!(fixture.expected["refused"].as_bool(), Some(false));
}

#[test]
fn modpack_policy_pack_managed_guard_explicit_replace_intent_required_not_inferred_from_reimport_alone()
 {
    let fixture =
        load_pack_guard("explicit-replace-intent-required-not-inferred-from-reimport-alone");
    assert!(pack_replace_refused(true, false));
    assert_eq!(fixture.expected["refused"].as_bool(), Some(true));
}

#[test]
fn modpack_policy_pack_managed_dependency_install_follows_parent_mutation() {
    let fixture = load_pack_guard(
        "dependency-auto-install-transitively-blocked-when-parent-mutation-refused",
    );
    // A pack import does not prevent a normal add-on install, so dependency
    // resolution remains available to the same parent operation.
    assert!(!pack_mutation_refused(true, AddonMutationKind::Install));
    assert_eq!(
        fixture.expected["parent_install_refused"].as_bool(),
        Some(false)
    );
    assert_eq!(
        fixture.expected["dependency_installer_invoked"].as_bool(),
        Some(true)
    );
}

#[test]
fn modpack_policy_pack_managed_health_repair_update_install_actions_remain_available() {
    let fixture = load_pack_guard("health-repair-update-install-actions-also-subject-to-guard");
    // P8.23 routes health-repair's update/install actions through the
    // exact same mutation paths P8.17 builds -- the same gate applies,
    // not a parallel repair-specific check.
    assert!(!pack_mutation_refused(true, AddonMutationKind::Update));
    assert!(!pack_mutation_refused(true, AddonMutationKind::Install));
    assert_eq!(fixture.expected["refused"].as_bool(), Some(false));
}

#[test]
fn modpack_policy_pack_managed_guard_msc1_baseline_warns_but_never_gates_contrast() {
    let fixture = load_pack_guard("msc1-baseline-warns-but-never-gates-contrast");
    // Contrast baseline: MSC 1 itself never refuses this call (only
    // confirmation-dialog copy changes) -- Phase 8's decided policy
    // deliberately disagrees, which is the whole point of this guard.
    assert_eq!(fixture.expected["refused_by_msc1"].as_bool(), Some(false));
    assert!(!pack_mutation_refused(true, AddonMutationKind::Update));
}
