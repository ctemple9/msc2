//! P7.36's port of `StartupCrashAnalyzer.analyzePaperPlugins` — no MSC 1
//! test file exercises it (already flagged by P1.7's own doc, closed
//! here), so every fixture is characterized directly from source's
//! closed, deterministic logic. Same harness shape as
//! `startup_crash_analyzer.rs`.

mod support;

use msc_domain::crash_analysis::{PluginEntry, StartupProblem, analyze_paper_plugins};
use serde_json::Value;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("paper-plugin-crash-analysis/{case}.json")))
}

fn plugin_entry_from_value(v: &Value) -> PluginEntry {
    PluginEntry {
        filename: v["filename"].as_str().unwrap().to_string(),
        jar_stem: v["jarStem"].as_str().unwrap().to_string(),
        display_name: v["displayName"].as_str().unwrap().to_string(),
        version: v["version"].as_str().map(str::to_string),
        is_enabled: v["isEnabled"].as_bool().unwrap(),
    }
}

fn plugins_from_value(v: &Value) -> Vec<PluginEntry> {
    v.as_array()
        .unwrap()
        .iter()
        .map(plugin_entry_from_value)
        .collect()
}

fn excerpt_from_value(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

fn run(fixture: &Fixture) -> Vec<StartupProblem> {
    let excerpt = excerpt_from_value(&fixture.input["consoleExcerpt"]);
    let installed_plugins = plugins_from_value(&fixture.input["installedPlugins"]);
    analyze_paper_plugins(&excerpt, &installed_plugins)
}

/// Same "only what `expected` names" convention as
/// `startup_crash_analyzer.rs`'s own `assert_problem_fields`.
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
    if let Some(v) = expected.get("requirement").and_then(Value::as_str) {
        assert_eq!(problem.requirement.as_deref(), Some(v));
    }
}

#[test]
fn paper_plugin_crash_analysis_empty_excerpt_returns_empty() {
    let problems = run(&load("empty-excerpt-returns-empty"));
    assert!(problems.is_empty());
}

#[test]
fn paper_plugin_crash_analysis_missing_dependency_single_dep_attributed_to_installed_plugin() {
    let fixture = load("missing-dependency-single-dep-attributed-to-installed-plugin");
    let problems = run(&fixture);
    assert_eq!(
        problems.len(),
        fixture.expected["problemsCount"].as_i64().unwrap() as usize
    );
    assert_problem_fields(&problems[0], &fixture.expected["problem"]);
}

#[test]
fn paper_plugin_crash_analysis_missing_dependency_multiple_deps_produce_multiple_problems() {
    let fixture = load("missing-dependency-multiple-deps-produce-multiple-problems");
    let problems = run(&fixture);
    let expected_problems = fixture.expected["problems"].as_array().unwrap();
    assert_eq!(problems.len(), expected_problems.len());
    for (problem, expected) in problems.iter().zip(expected_problems) {
        assert_problem_fields(problem, expected);
    }
}

#[test]
fn paper_plugin_crash_analysis_missing_dependency_unmatched_plugin_still_recorded() {
    let fixture = load("missing-dependency-unmatched-plugin-still-recorded");
    let problems = run(&fixture);
    assert_eq!(
        problems.len(),
        fixture.expected["problemsCount"].as_i64().unwrap() as usize
    );
    assert_problem_fields(&problems[0], &fixture.expected["problem"]);
}

#[test]
fn paper_plugin_crash_analysis_enable_error_trims_version_suffix() {
    let fixture = load("enable-error-trims-version-suffix");
    let problems = run(&fixture);
    assert_eq!(
        problems.len(),
        fixture.expected["problemsCount"].as_i64().unwrap() as usize
    );
    assert_problem_fields(&problems[0], &fixture.expected["problem"]);
}

#[test]
fn paper_plugin_crash_analysis_enable_error_trims_parenthetical() {
    let fixture = load("enable-error-trims-parenthetical");
    let problems = run(&fixture);
    assert_eq!(
        problems.len(),
        fixture.expected["problemsCount"].as_i64().unwrap() as usize
    );
    assert_problem_fields(&problems[0], &fixture.expected["problem"]);
}

#[test]
fn paper_plugin_crash_analysis_enable_error_matched_via_jar_stem_contains() {
    let fixture = load("enable-error-matched-via-jar-stem-contains");
    let problems = run(&fixture);
    assert_eq!(
        problems.len(),
        fixture.expected["problemsCount"].as_i64().unwrap() as usize
    );
    assert_problem_fields(&problems[0], &fixture.expected["problem"]);
}

#[test]
fn paper_plugin_crash_analysis_noise_lines_ignored_no_false_positives() {
    let problems = run(&load("noise-lines-ignored-no-false-positives"));
    assert!(problems.is_empty());
}
