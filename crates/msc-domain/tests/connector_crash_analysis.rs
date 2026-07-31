//! Port of `ConnectorCrashAnalysisTests.swift`'s 11 fixtures — covers
//! `ModrinthSlugNormalizer` (bundled into the same MSC 1 test file as the
//! analyzer) plus the Connector-entrypoint / cross-separator-matching
//! slices of `StartupCrashAnalyzer`.
//!
//! Test functions are prefixed `connector_crash_analysis_` so the plan's
//! Verify command (a plain nextest substring filter, which matches on test
//! name, not file/binary name) selects all of them.

mod support;

use msc_domain::crash_analysis::{
    ModEntry, StartupProblem, analyze, match_installed_mod, normalized_identifier,
};
use msc_domain::slug::{canonical_slug, is_known_alias, normalized_slug};
use serde_json::Value;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("connector-crash-analysis/{case}.json")))
}

fn mod_entry_from_value(v: &Value) -> ModEntry {
    ModEntry {
        filename: v["filename"].as_str().unwrap().to_string(),
        jar_stem: v["jarStem"].as_str().unwrap().to_string(),
        display_name: v["displayName"].as_str().unwrap().to_string(),
        mod_id: v["modId"].as_str().map(str::to_string),
        version: v["version"].as_str().map(str::to_string),
        is_enabled: v["isEnabled"].as_bool().unwrap(),
    }
}

fn mods_from_value(v: &Value) -> Vec<ModEntry> {
    v.as_array()
        .unwrap()
        .iter()
        .map(mod_entry_from_value)
        .collect()
}

fn excerpt_from_value(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

fn run_analyze(fixture: &Fixture) -> Vec<StartupProblem> {
    let flavor = fixture.input["flavor"].as_str().unwrap();
    let excerpt = excerpt_from_value(&fixture.input["consoleExcerpt"]);
    let installed_mods = mods_from_value(&fixture.input["installedMods"]);
    analyze(flavor, &excerpt, &installed_mods)
}

fn run_canonical_slug_cases(fixture: &Fixture) {
    let cases = fixture.input["cases"].as_array().unwrap();
    let expected = fixture.expected["results"].as_array().unwrap();
    for (case, exp) in cases.iter().zip(expected) {
        let slug = case["slug"].as_str().unwrap();
        let forge_family = case["forgeFamily"].as_bool().unwrap();
        assert_eq!(canonical_slug(slug, forge_family), exp.as_str().unwrap());
    }
}

#[test]
fn connector_crash_analysis_common_aliases_apply_regardless_of_loader() {
    run_canonical_slug_cases(&load("common-aliases-apply-regardless-of-loader"));
}

#[test]
fn connector_crash_analysis_fabric_api_alias_is_forge_family_only() {
    run_canonical_slug_cases(&load("fabric-api-alias-is-forge-family-only"));
}

#[test]
fn connector_crash_analysis_normalized_slug_basics() {
    let fixture = load("normalized-slug-basics");
    let strings = fixture.input["strings"].as_array().unwrap();
    let expected = fixture.expected["results"].as_array().unwrap();
    for (s, exp) in strings.iter().zip(expected) {
        assert_eq!(normalized_slug(s.as_str().unwrap()), exp.as_str().unwrap());
    }
}

#[test]
fn connector_crash_analysis_unknown_slug_normalizes_but_is_not_an_alias() {
    let fixture = load("unknown-slug-normalizes-but-is-not-an-alias");
    let calls = fixture.input["calls"].as_array().unwrap();
    let expected = fixture.expected["results"].as_array().unwrap();
    for (call, exp) in calls.iter().zip(expected) {
        let slug = call["slug"].as_str().unwrap();
        let forge_family = call["forgeFamily"].as_bool().unwrap();
        match call["call"].as_str().unwrap() {
            "canonicalSlug" => {
                assert_eq!(canonical_slug(slug, forge_family), exp.as_str().unwrap());
            }
            "isKnownAlias" => {
                assert_eq!(is_known_alias(slug, forge_family), exp.as_bool().unwrap());
            }
            other => panic!("unhandled call: {other}"),
        }
    }
}

#[test]
fn connector_crash_analysis_normalized_identifier_collapses_separators() {
    let fixture = load("normalized-identifier-collapses-separators");
    let identifiers = fixture.input["identifiers"].as_array().unwrap();
    let expected = fixture.expected["results"].as_array().unwrap();
    for (id, exp) in identifiers.iter().zip(expected) {
        assert_eq!(
            normalized_identifier(id.as_str().unwrap()),
            exp.as_str().unwrap()
        );
    }
}

#[test]
fn connector_crash_analysis_match_installed_mod_across_separator_forms() {
    let fixture = load("match-installed-mod-across-separator-forms");
    let cases = fixture.input["cases"].as_array().unwrap();
    let results = fixture.expected["results"].as_array().unwrap();
    for (case, expected) in cases.iter().zip(results) {
        let identifier = case["identifier"].as_str().unwrap();
        let mods = mods_from_value(&case["installedMods"]);
        let found = match_installed_mod(identifier, &mods);
        match expected {
            Value::Null => assert!(found.is_none(), "identifier {identifier}"),
            Value::Object(obj) => {
                let found = found.unwrap_or_else(|| panic!("expected a match for {identifier}"));
                if let Some(display_name) = obj.get("displayName").and_then(Value::as_str) {
                    assert_eq!(found.display_name, display_name);
                }
                if let Some(mod_id) = obj.get("modId").and_then(Value::as_str) {
                    assert_eq!(found.mod_id.as_deref(), Some(mod_id));
                }
            }
            other => panic!("unexpected expected value {other:?}"),
        }
    }
}

#[test]
fn connector_crash_analysis_garbage_log_yields_nothing() {
    let problems = run_analyze(&load("garbage-log-yields-nothing"));
    assert!(problems.is_empty());
}

#[test]
fn connector_crash_analysis_connector_entrypoint_failure_becomes_load_error() {
    let fixture = load("connector-entrypoint-failure-becomes-load-error");
    let problems = run_analyze(&fixture);
    assert_eq!(
        problems.len(),
        fixture.expected["problemsCount"].as_i64().unwrap() as usize
    );
    let expected = &fixture.expected["problem"];
    let problem = &problems[0];
    assert_eq!(problem.kind.raw_value(), expected["kind"].as_str().unwrap());
    assert_eq!(
        problem.offender_name,
        expected["offenderName"].as_str().unwrap()
    );
    assert_eq!(
        problem.installed_file.as_deref(),
        expected["installedFile"].as_str()
    );
    assert_eq!(
        problem.installed_jar_stem.as_deref(),
        expected["installedJarStem"].as_str()
    );
    assert!(
        problem
            .requirement
            .as_deref()
            .unwrap_or("")
            .contains(expected["requirementContains"].as_str().unwrap())
    );
}

#[test]
fn connector_crash_analysis_connector_entrypoint_matches_by_display_name_when_mod_id_missing() {
    let fixture = load("connector-entrypoint-matches-by-display-name-when-mod-id-missing");
    let problems = run_analyze(&fixture);
    let expected = &fixture.expected["firstProblem"];
    let problem = problems.first().expect("at least one problem");
    assert_eq!(
        problem.offender_name,
        expected["offenderName"].as_str().unwrap()
    );
    assert_eq!(
        problem.installed_file.as_deref(),
        expected["installedFile"].as_str()
    );
}

#[test]
fn connector_crash_analysis_connector_entrypoint_unmatched_keeps_raw_id_but_still_reports() {
    let fixture = load("connector-entrypoint-unmatched-keeps-raw-id-but-still-reports");
    let problems = run_analyze(&fixture);
    assert_eq!(
        problems.len(),
        fixture.expected["problemsCount"].as_i64().unwrap() as usize
    );
    let expected = &fixture.expected["firstProblem"];
    let problem = &problems[0];
    assert_eq!(
        problem.offender_name,
        expected["offenderName"].as_str().unwrap()
    );
    assert_eq!(problem.kind.raw_value(), expected["kind"].as_str().unwrap());
    assert_eq!(
        problem.installed_file.as_deref(),
        expected["installedFile"].as_str()
    );
}

#[test]
fn connector_crash_analysis_forge_dependency_block_parses() {
    let fixture = load("forge-dependency-block-parses");
    let problems = run_analyze(&fixture);
    assert_eq!(
        !problems.is_empty(),
        fixture.expected["problemsNotEmpty"].as_bool().unwrap()
    );
    let assertions = fixture.expected["assertions"].as_array().unwrap();
    for assertion in assertions {
        if let Some(find) = assertion.get("find") {
            let missing_dep = find["missingDependency"].as_str().unwrap();
            let found = problems
                .iter()
                .find(|p| p.missing_dependency.as_deref() == Some(missing_dep))
                .unwrap_or_else(|| panic!("no problem with missingDependency {missing_dep}"));
            let expect_fields = &assertion["expectFields"];
            if let Some(kind) = expect_fields.get("kind").and_then(Value::as_str) {
                assert_eq!(found.kind.raw_value(), kind);
            }
            if let Some(offender_id) = expect_fields.get("offenderId").and_then(Value::as_str) {
                assert_eq!(found.offender_id.as_deref(), Some(offender_id));
            }
        } else if let Some(exists) = assertion.get("exists") {
            let matches = |p: &StartupProblem| {
                if let Some(md) = exists.get("missingDependency").and_then(Value::as_str)
                    && p.missing_dependency.as_deref() != Some(md)
                {
                    return false;
                }
                if let Some(oid) = exists.get("offenderId").and_then(Value::as_str)
                    && p.offender_id.as_deref() != Some(oid)
                {
                    return false;
                }
                if let Some(kind) = exists.get("kind").and_then(Value::as_str)
                    && p.kind.raw_value() != kind
                {
                    return false;
                }
                true
            };
            assert!(
                problems.iter().any(matches),
                "no problem matching {exists:?}"
            );
        } else {
            panic!("unhandled assertion shape: {assertion:?}");
        }
    }
}
