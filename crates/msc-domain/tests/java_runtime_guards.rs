//! Port of the pure-guard cases from `JavaRuntimeGuardsTests.swift`. Only
//! the 7 fixtures that don't touch the filesystem are wired here — see
//! `rolling-plan.md` P1.5's scope note for the other 8 (deferred to
//! `msc-infrastructure` in Phase 3).
//!
//! Test functions are prefixed `java_runtime_guards_` so the plan's Verify
//! command (a plain nextest substring filter, which matches on test name,
//! not file/binary name) selects all of them.

mod support;

use msc_domain::java_runtime::{compatibility_warning_text, required_java_major};
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("java-runtime-guards/{case}.json")))
}

fn assert_warning_is_nil(case: &str) {
    let fixture = load(case);
    let minecraft_version = fixture.input["minecraftVersion"].as_str();
    let required = fixture.input["required"].as_i64().expect("required");
    let detected = fixture.input["detected"].as_i64().expect("detected");
    assert!(fixture.expected["isNil"].as_bool().expect("isNil"));
    let actual = compatibility_warning_text(minecraft_version, required, detected);
    assert!(
        actual.is_none(),
        "case {case}: expected None, got {actual:?}"
    );
}

fn assert_warning_contains(case: &str) {
    let fixture = load(case);
    let minecraft_version = fixture.input["minecraftVersion"].as_str();
    let required = fixture.input["required"].as_i64().expect("required");
    let detected = fixture.input["detected"].as_i64().expect("detected");
    assert!(!fixture.expected["isNil"].as_bool().expect("isNil"));

    let actual = compatibility_warning_text(minecraft_version, required, detected)
        .unwrap_or_else(|| panic!("case {case}: expected Some, got None"));

    if let Some(contains) = fixture.expected.get("contains").and_then(|v| v.as_array()) {
        for substring in contains {
            let substring = substring.as_str().expect("contains entry");
            assert!(
                actual.contains(substring),
                "case {case}: expected text to contain {substring:?}, got {actual:?}"
            );
        }
    }
    if let Some(not_contains) = fixture
        .expected
        .get("notContains")
        .and_then(|v| v.as_array())
    {
        for substring in not_contains {
            let substring = substring.as_str().expect("notContains entry");
            assert!(
                !actual.contains(substring),
                "case {case}: expected text to NOT contain {substring:?}, got {actual:?}"
            );
        }
    }
}

// --- JavaRuntimeManager.requiredJavaMajor ---

#[test]
fn java_runtime_guards_required_java_major_mapping() {
    let fixture = load("required-java-major-mapping");
    let versions = fixture.input["minecraftVersions"]
        .as_array()
        .expect("minecraftVersions array");
    let expected: Vec<i64> = fixture.expected["results"]
        .as_array()
        .expect("results array")
        .iter()
        .map(|v| v.as_i64().expect("i64 result"))
        .collect();
    let actual: Vec<i64> = versions
        .iter()
        .map(|v| required_java_major(v.as_str()))
        .collect();
    assert_eq!(actual, expected);
}

// --- JavaRuntimeManager.compatibilityWarningText: no warning ---

#[test]
fn java_runtime_guards_no_warning_java17_era_with_exact_java17() {
    assert_warning_is_nil("no-warning-java17-era-with-exact-java17");
}

#[test]
fn java_runtime_guards_no_warning_java21_era_with_java21() {
    assert_warning_is_nil("no-warning-java21-era-with-java21");
}

#[test]
fn java_runtime_guards_no_warning_java21_era_with_java25() {
    assert_warning_is_nil("no-warning-java21-era-with-java25");
}

// --- JavaRuntimeManager.compatibilityWarningText: too old / too new ---

#[test]
fn java_runtime_guards_too_old_warning_still_fires() {
    assert_warning_contains("too-old-warning-still-fires");
}

#[test]
fn java_runtime_guards_too_new_warning_java17_era_with_java21() {
    assert_warning_contains("too-new-warning-java17-era-with-java21");
}

#[test]
fn java_runtime_guards_too_new_warning_java17_era_with_java25() {
    assert_warning_contains("too-new-warning-java17-era-with-java25");
}
