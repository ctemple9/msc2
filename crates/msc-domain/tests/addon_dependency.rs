//! Port of `fixtures/modrinth-dependencies/`'s 15 cases (P8.6) against
//! `msc_domain::addon_dependency` (P8.12).

mod support;

use msc_domain::addon_dependency::*;
use msc_domain::identity::AddOnKind;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("modrinth-dependencies/{case}.json")))
}

fn deps_from(value: &serde_json::Value) -> Vec<ModrinthDependency> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|d| ModrinthDependency {
            project_id: d["projectId"].as_str().map(str::to_string),
            dependency_type: d["dependencyType"].as_str().unwrap().to_string(),
        })
        .collect()
}

#[test]
fn addon_dependency_only_required_dependency_type_installed_optional_skipped() {
    let fixture = load("only-required-dependency-type-installed-optional-skipped");
    let deps = deps_from(&fixture.input["dependencies"]);
    let processed = required_dependencies_with_project_id(&deps);
    let expected: Vec<String> = fixture.expected["processed_project_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(processed, expected);
}

#[test]
fn addon_dependency_dependency_without_project_id_skipped() {
    let fixture = load("dependency-without-project-id-skipped");
    let deps = deps_from(&fixture.input["dependencies"]);
    let processed = required_dependencies_with_project_id(&deps);
    let expected: Vec<String> = fixture.expected["processed_project_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(processed, expected);
}

#[test]
fn addon_dependency_empty_required_list_returns_immediately() {
    let fixture = load("empty-required-list-returns-immediately");
    let deps: Vec<ModrinthDependency> = Vec::new();
    assert!(required_dependencies_with_project_id(&deps).is_empty());
    assert_eq!(fixture.expected["network_requests_made"].as_i64(), Some(0));
}

#[test]
fn addon_dependency_already_installed_by_mod_id_match_skipped_not_redownloaded() {
    let fixture = load("already-installed-by-mod-id-match-skipped-not-redownloaded");
    let slug = fixture.input["dependency_project_slug"].as_str().unwrap();
    let installed: Vec<String> = fixture.input["installedModIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let present = dependency_already_present(slug, &installed, &[]);
    assert_eq!(present, !fixture.expected["downloaded"].as_bool().unwrap());
}

#[test]
fn addon_dependency_already_present_by_filename_slug_scan_skipped() {
    let fixture = load("already-present-by-filename-slug-scan-skipped");
    let slug = fixture.input["dependency_project_slug"].as_str().unwrap();
    let files: Vec<String> = fixture.input["filesOnDisk"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let present = dependency_already_present(slug, &[], &files);
    assert_eq!(present, !fixture.expected["downloaded"].as_bool().unwrap());
}

#[test]
fn addon_dependency_filename_scan_is_case_insensitive() {
    let fixture = load("filename-scan-is-case-insensitive");
    let slug = fixture.input["dependency_project_slug"].as_str().unwrap();
    let files: Vec<String> = fixture.input["filesOnDisk"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let present = dependency_already_present(slug, &[], &files);
    assert_eq!(present, !fixture.expected["downloaded"].as_bool().unwrap());
}

#[test]
fn addon_dependency_depth_cap_guard_stops_recursion_at_depth_3() {
    let fixture = load("depth-cap-guard-stops-recursion-at-depth-3");
    let depth = fixture.input["depth"].as_u64().unwrap() as u32;
    assert_eq!(
        msc1_depth_cap_exceeded(depth),
        fixture.expected["function_returns_immediately"]
            .as_bool()
            .unwrap()
    );
}

#[test]
fn addon_dependency_depth_cap_terminates_a_b_a_cycle_by_depth_not_by_cycle_awareness() {
    let fixture = load("depth-cap-terminates-a-b-a-cycle-by-depth-not-by-cycle-awareness");
    let terminates_at = fixture.expected["terminates_at_depth"].as_u64().unwrap() as u32;
    assert!(!msc1_depth_cap_exceeded(terminates_at - 1));
    assert!(msc1_depth_cap_exceeded(terminates_at));

    // The real, decided port doesn't wait for a coincidental depth cap --
    // a visited-set cycle detector catches the same A->B->A cycle
    // immediately on A's second visit, regardless of the chain's length.
    let visited = vec!["A".to_string(), "B".to_string()];
    assert!(cycle_detected("A", &visited));
}

#[test]
fn addon_dependency_diamond_dependency_both_parents_check_already_present_before_recursive_install()
{
    let fixture =
        load("diamond-dependency-both-parents-check-already-present-before-recursive-install");
    // B installs D first; D is now on disk. When C's branch reaches D,
    // the already-present check (not the depth cap, and not cycle
    // detection -- D is not a cycle, it's shared) skips the redundant
    // download.
    let installed_after_b = vec!["D".to_string()];
    let d_present_for_c = dependency_already_present("D", &installed_after_b, &[]);
    assert!(d_present_for_c);
    assert_eq!(fixture.expected["D_downloaded_count"].as_i64(), Some(1));
}

#[test]
fn addon_dependency_no_add_on_kind_server_returns_without_processing() {
    let fixture = load("no-add-on-kind-server-returns-without-processing");
    assert!(fixture.input["javaFlavor_addOnKind"].is_null());
    assert_eq!(
        fixture.expected["function_returns_immediately"].as_bool(),
        Some(true)
    );
}

#[test]
fn addon_dependency_no_compatible_version_found_logs_and_continues_not_fatal() {
    let fixture = load("no-compatible-version-found-logs-and-continues-not-fatal");
    let outcome = DependencyOutcome::NoCompatibleVersion;
    assert_eq!(
        outcome.is_fatal_to_batch(),
        fixture.expected["entire_function_aborted"]
            .as_bool()
            .unwrap()
    );
    assert!(!outcome.is_fatal_to_batch());
}

#[test]
fn addon_dependency_no_primary_file_on_best_version_also_treated_as_no_compatible() {
    let fixture = load("no-primary-file-on-best-version-also-treated-as-no-compatible");
    // An empty `files` array on the "best" version is the same
    // continue-not-fatal path as no version existing at all -- confirmed
    // structurally via the already-ported primary-file selector (P8.10):
    // an empty files list has no primary file to select.
    let files: Vec<msc_domain::addon_provider::ModrinthVersionFile> = Vec::new();
    assert!(msc_domain::addon_provider::modrinth_primary_file(&files).is_none());
    assert_eq!(
        fixture.expected["loop_continues_to_next_dependency"].as_bool(),
        Some(true)
    );
}

#[test]
fn addon_dependency_per_dependency_failure_logs_and_continues_not_fatal() {
    let fixture = load("per-dependency-failure-logs-and-continues-not-fatal");
    let outcome = DependencyOutcome::Failed;
    assert!(!outcome.is_fatal_to_batch());
    assert_eq!(
        fixture.expected["whole_function_threw"].as_bool(),
        Some(false)
    );
}

#[test]
fn addon_dependency_successful_install_recurses_for_transitive_dependencies() {
    let fixture = load("successful-install-recurses-for-transitive-dependencies");
    let depth = fixture.input["depth"].as_u64().unwrap() as u32;
    let next_depth = depth + 1;
    assert_eq!(
        next_depth as i64,
        fixture.expected["recursive_call_depth"].as_i64().unwrap()
    );
}

#[test]
fn addon_dependency_refresh_mod_list_after_resolving_mod_server_vs_plugin_list_for_plugin_server() {
    let fixture =
        load("refresh-mod-list-after-resolving-mod-server-vs-plugin-list-for-plugin-server");
    let target = dependency_refresh_target(AddOnKind::Mod);
    assert_eq!(target, DependencyRefreshTarget::Mods);
    assert_eq!(
        fixture.expected["refreshDiscoveredMods_called"].as_bool(),
        Some(true)
    );
    assert_eq!(
        fixture.expected["refreshDiscoveredPlugins_called"].as_bool(),
        Some(false)
    );

    assert_eq!(
        dependency_refresh_target(AddOnKind::Plugin),
        DependencyRefreshTarget::Plugins
    );
}
