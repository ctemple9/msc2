//! Port of `ComponentVersionParsingTests.swift`. One test per
//! `fixtures/component-version/` case, so a failing case names itself in
//! `cargo nextest run` output the same way a failing Python fixture does.
//!
//! Test functions are prefixed `version_comparison_` so the plan's Verify
//! command (a plain nextest substring filter, which matches on test name,
//! not file/binary name) selects all of them.

mod support;

use msc_domain::version::{
    build_display_string, is_downgrade, parse_paper_jar_filename, parse_trailing_build_number,
};
use serde_json::Value;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("component-version/{case}.json")))
}

/// Reads a `{"cases": [{"from": ..., "to": ...}, ...]}` input paired with a
/// `{"results": [bool, ...]}` expected shape.
fn downgrade_batch(fixture: &Fixture) -> (Vec<(Option<String>, String)>, Vec<bool>) {
    let cases = fixture.input["cases"].as_array().expect("cases array");
    let inputs = cases
        .iter()
        .map(|c| {
            let from = c["from"].as_str().map(|s| s.to_string());
            let to = c["to"].as_str().expect("to").to_string();
            (from, to)
        })
        .collect();
    let expected = fixture.expected["results"]
        .as_array()
        .expect("results array")
        .iter()
        .map(|v| v.as_bool().expect("bool result"))
        .collect();
    (inputs, expected)
}

fn assert_downgrade_batch(case: &str) {
    let fixture = load(case);
    let (cases, expected) = downgrade_batch(&fixture);
    let actual: Vec<bool> = cases
        .iter()
        .map(|(from, to)| is_downgrade(from.as_deref(), to))
        .collect();
    assert_eq!(actual, expected, "case: {case}");
}

/// Reads a `{"from": ..., "to": "..."}` input paired with a plain boolean
/// `expected`.
fn assert_downgrade_single(case: &str) {
    let fixture = load(case);
    let from = fixture.input["from"].as_str().map(|s| s.to_string());
    let to = fixture.input["to"].as_str().expect("to").to_string();
    let expected = fixture.expected.as_bool().expect("bool expected");
    assert_eq!(is_downgrade(from.as_deref(), &to), expected, "case: {case}");
}

/// Reads a `{"filename": "..."}` input against an `expected` that is either
/// `null` or an object naming a subset of `PaperJarVersion`'s fields.
fn assert_paper_jar(case: &str) {
    let fixture = load(case);
    let filename = fixture.input["filename"].as_str().expect("filename");
    let actual = parse_paper_jar_filename(filename);
    match &fixture.expected {
        Value::Null => assert!(
            actual.is_none(),
            "case {case}: expected None, got {actual:?}"
        ),
        Value::Object(obj) => {
            let actual = actual.unwrap_or_else(|| panic!("case {case}: expected Some, got None"));
            if let Some(v) = obj.get("mcVersion") {
                assert_eq!(
                    v.as_str().expect("mcVersion"),
                    actual.mc_version,
                    "case {case}"
                );
            }
            if let Some(v) = obj.get("build") {
                assert_eq!(v.as_i64().expect("build"), actual.build, "case {case}");
            }
            if let Some(v) = obj.get("displayString") {
                assert_eq!(
                    v.as_str().expect("displayString"),
                    actual.display_string(),
                    "case {case}"
                );
            }
            if let Some(v) = obj.get("compactString") {
                assert_eq!(
                    v.as_str().expect("compactString"),
                    actual.compact_string(),
                    "case {case}"
                );
            }
        }
        other => panic!("case {case}: unexpected expected shape: {other:?}"),
    }
}

/// Reads a `{"filename": "..."}` input against an `expected` that is either
/// `null` or an integer.
fn assert_trailing_build_number_single(case: &str) {
    let fixture = load(case);
    let filename = fixture.input["filename"].as_str().expect("filename");
    let actual = parse_trailing_build_number(filename);
    match &fixture.expected {
        Value::Null => assert!(
            actual.is_none(),
            "case {case}: expected None, got {actual:?}"
        ),
        Value::Number(n) => assert_eq!(actual, n.as_i64(), "case {case}"),
        other => panic!("case {case}: unexpected expected shape: {other:?}"),
    }
}

// --- MCVersionComparator.isDowngrade: batched cases ---

#[test]
fn version_comparison_bedrock_four_part_versions() {
    assert_downgrade_batch("bedrock-four-part-versions");
}

#[test]
fn version_comparison_downgrade_returns_true() {
    assert_downgrade_batch("downgrade-returns-true");
}

#[test]
fn version_comparison_latest_target_skips_check() {
    assert_downgrade_batch("latest-target-skips-check");
}

#[test]
fn version_comparison_non_numeric_snapshot_skips_check() {
    assert_downgrade_batch("non-numeric-snapshot-skips-check");
}

#[test]
fn version_comparison_padded_components() {
    assert_downgrade_batch("padded-components");
}

#[test]
fn version_comparison_upgrade_returns_false() {
    assert_downgrade_batch("upgrade-returns-false");
}

// --- MCVersionComparator.isDowngrade: single cases ---

#[test]
fn version_comparison_empty_current_skips_check() {
    assert_downgrade_single("empty-current-skips-check");
}

#[test]
fn version_comparison_latest_current_skips_check() {
    assert_downgrade_single("latest-current-skips-check");
}

#[test]
fn version_comparison_nil_current_skips_check() {
    let fixture = load("nil-current-skips-check");
    assert!(
        fixture.input["from"].is_null(),
        "fixture assumes a null 'from'"
    );
    let to = fixture.input["to"].as_str().expect("to").to_string();
    let expected = fixture.expected.as_bool().expect("bool expected");
    assert_eq!(is_downgrade(None, &to), expected);
}

#[test]
fn version_comparison_same_version_returns_false() {
    assert_downgrade_single("same-version-returns-false");
}

// --- ComponentVersionParsing.parsePaperJarFilename ---

#[test]
fn version_comparison_non_paper_prefix_rejected() {
    assert_paper_jar("non-paper-prefix-rejected");
}

#[test]
fn version_comparison_paper_bare_build_number_form() {
    assert_paper_jar("paper-bare-build-number-form");
}

#[test]
fn version_comparison_paper_build_keyword_form() {
    assert_paper_jar("paper-build-keyword-form");
}

#[test]
fn version_comparison_paper_case_insensitive_prefix() {
    assert_paper_jar("paper-case-insensitive-prefix");
}

#[test]
fn version_comparison_paper_missing_build_rejected() {
    assert_paper_jar("paper-missing-build-rejected");
}

#[test]
fn version_comparison_paper_non_numeric_build_rejected() {
    assert_paper_jar("paper-non-numeric-build-rejected");
}

// --- ComponentVersionParsing.parseTrailingBuildNumber ---

#[test]
fn version_comparison_trailing_build_number_floodgate() {
    assert_trailing_build_number_single("trailing-build-number-floodgate");
}

#[test]
fn version_comparison_trailing_build_number_geyser() {
    assert_trailing_build_number_single("trailing-build-number-geyser");
}

#[test]
fn version_comparison_trailing_build_number_no_separator_uses_whole_stem() {
    let fixture = load("trailing-build-number-no-separator-uses-whole-stem");
    let filenames = fixture.input["filenames"]
        .as_array()
        .expect("filenames array");
    let expected: Vec<Option<i64>> = fixture.expected["results"]
        .as_array()
        .expect("results array")
        .iter()
        .map(|v| v.as_i64())
        .collect();
    let actual: Vec<Option<i64>> = filenames
        .iter()
        .map(|f| parse_trailing_build_number(f.as_str().expect("filename")))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn version_comparison_trailing_build_number_non_numeric_tail_is_nil() {
    assert_trailing_build_number_single("trailing-build-number-non-numeric-tail-is-nil");
}

// --- ComponentVersionParsing.buildDisplayString ---

#[test]
fn version_comparison_build_display_string_case() {
    let fixture = load("build-display-string");
    let builds = fixture.input["builds"].as_array().expect("builds array");
    let actual: Vec<Option<String>> = builds
        .iter()
        .map(|b| build_display_string(b.as_i64()))
        .collect();
    let expected: Vec<Option<String>> = fixture.expected["results"]
        .as_array()
        .expect("results array")
        .iter()
        .map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    assert_eq!(actual, expected);
}
