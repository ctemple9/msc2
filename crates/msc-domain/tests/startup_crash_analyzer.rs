//! Port of `StartupCrashAnalyzerTests.swift`'s 7 fixtures.
//!
//! Test functions are prefixed `startup_crash_analyzer_` so the plan's
//! Verify command (a plain nextest substring filter, which matches on test
//! name, not file/binary name) selects all of them.

mod support;

use msc_domain::crash_analysis::{ModEntry, StartupProblem, analyze};
use serde_json::Value;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("startup-crash-analyzer/{case}.json")))
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

/// Asserts only the fields present in `expected` — fixtures here only pin
/// the subset their source test actually checked. A key present with an
/// explicit JSON `null` still enforces `None`; an absent key skips the
/// check entirely.
fn assert_problem_fields(problem: &StartupProblem, expected: &Value) {
    if let Some(kind) = expected.get("kind").and_then(Value::as_str) {
        assert_eq!(problem.kind.raw_value(), kind);
    }
    if let Some(name) = expected.get("offenderName").and_then(Value::as_str) {
        assert_eq!(problem.offender_name, name);
    }
    if let Some(v) = expected.get("offenderId") {
        assert_eq!(problem.offender_id.as_deref(), v.as_str());
    }
    if let Some(v) = expected.get("installedFile") {
        assert_eq!(problem.installed_file.as_deref(), v.as_str());
    }
    if let Some(v) = expected.get("installedJarStem") {
        assert_eq!(problem.installed_jar_stem.as_deref(), v.as_str());
    }
    if let Some(v) = expected.get("missingDependency") {
        assert_eq!(problem.missing_dependency.as_deref(), v.as_str());
    }
    if let Some(prefix) = expected.get("requirementHasPrefix").and_then(Value::as_str) {
        assert!(
            problem
                .requirement
                .as_deref()
                .unwrap_or("")
                .starts_with(prefix)
        );
    }
    if let Some(contains) = expected.get("requirementContains").and_then(Value::as_str) {
        assert!(
            problem
                .requirement
                .as_deref()
                .unwrap_or("")
                .contains(contains)
        );
    }
}

#[test]
fn startup_crash_analyzer_empty_excerpt_returns_empty() {
    let problems = run_analyze(&load("empty-excerpt-returns-empty"));
    assert!(problems.is_empty());
}

#[test]
fn startup_crash_analyzer_fabric_incompatible_version_attributed_to_installed_mod() {
    let fixture = load("fabric-incompatible-version-attributed-to-installed-mod");
    let problems = run_analyze(&fixture);
    assert_eq!(
        problems.len(),
        fixture.expected["problemsCount"].as_i64().unwrap() as usize
    );
    assert_problem_fields(&problems[0], &fixture.expected["problem"]);
}

#[test]
fn startup_crash_analyzer_fabric_missing_dependency_parsed() {
    let fixture = load("fabric-missing-dependency-parsed");
    let problems = run_analyze(&fixture);
    assert_eq!(
        problems.len(),
        fixture.expected["problemsCount"].as_i64().unwrap() as usize
    );
    assert_problem_fields(&problems[0], &fixture.expected["problem"]);
}

#[test]
fn startup_crash_analyzer_forge_missing_dependency_parsed() {
    let fixture = load("forge-missing-dependency-parsed");
    let problems = run_analyze(&fixture);
    assert_eq!(
        problems.len(),
        fixture.expected["problemsCount"].as_i64().unwrap() as usize
    );
    assert_problem_fields(&problems[0], &fixture.expected["problem"]);
}

#[test]
fn startup_crash_analyzer_forge_wrong_dependency_version_attributes_to_dependency() {
    let fixture = load("forge-wrong-dependency-version-attributes-to-dependency");
    let problems = run_analyze(&fixture);
    assert_eq!(
        problems.len(),
        fixture.expected["problemsCount"].as_i64().unwrap() as usize
    );
    assert_problem_fields(&problems[0], &fixture.expected["problem"]);
}

#[test]
fn startup_crash_analyzer_garbage_log_returns_nothing_for_parseable_flavor() {
    let problems = run_analyze(&load("garbage-log-returns-nothing-for-parseable-flavor"));
    assert!(problems.is_empty());
}

#[test]
fn startup_crash_analyzer_unsupported_flavor_returns_empty_even_with_parseable_lines() {
    let problems = run_analyze(&load(
        "unsupported-flavor-returns-empty-even-with-parseable-lines",
    ));
    assert!(problems.is_empty());
}
