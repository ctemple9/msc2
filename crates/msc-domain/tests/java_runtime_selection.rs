//! Port of 12 of `fixtures/java-runtime-selection/`'s 18 cases (P7.7)
//! against `msc_domain::java_runtime`'s P7.12 extension. The other 6 --
//! the managed Adoptium install, `checkJavaOnPath`, and
//! `hasCriticalMissingDependency` -- need real filesystem/process/network
//! I/O and are P7.16's job; see `java_runtime.rs`'s own module doc for the
//! scope call. `fixtures/java-runtime-guards/` (15 cases) was already
//! ported in an earlier phase (`crates/msc-domain/tests/java_runtime_guards.rs`)
//! and isn't repeated here.

mod support;

use msc_domain::java_runtime::*;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("java-runtime-selection/{case}.json")))
}

#[test]
fn java_runtime_selection_parse_major_temurin_banner() {
    let fixture = load("parse-major-temurin-banner");
    let output = fixture.input["output"].as_str().unwrap();
    assert_eq!(parse_major(output), fixture.expected["major"].as_i64());
}

#[test]
fn java_runtime_selection_parse_major_zulu_banner() {
    let fixture = load("parse-major-zulu-banner");
    let output = fixture.input["output"].as_str().unwrap();
    assert_eq!(parse_major(output), fixture.expected["major"].as_i64());
}

#[test]
fn java_runtime_selection_parse_major_graalvm_banner() {
    let fixture = load("parse-major-graalvm-banner");
    let output = fixture.input["output"].as_str().unwrap();
    assert_eq!(parse_major(output), fixture.expected["major"].as_i64());
}

#[test]
fn java_runtime_selection_parse_major_legacy_1_8_style_banner() {
    let fixture = load("parse-major-legacy-1-8-style-banner");
    let output = fixture.input["output"].as_str().unwrap();
    assert_eq!(parse_major(output), fixture.expected["major"].as_i64());
}

#[test]
fn java_runtime_selection_validate_looks_like_java_accepts_openjdk_keyword() {
    let fixture = load("validate-looks-like-java-accepts-openjdk-keyword");
    let captured = fixture.input["capturedOutput"].as_str().unwrap();
    assert!(validate_looks_like_java("java", captured).is_ok());
    assert_eq!(fixture.expected["throws"].as_bool(), Some(false));
}

#[test]
fn java_runtime_selection_validate_looks_like_java_accepts_hotspot_only_keyword() {
    let fixture = load("validate-looks-like-java-accepts-hotspot-only-keyword");
    let captured = fixture.input["capturedOutput"].as_str().unwrap();
    assert!(validate_looks_like_java("java", captured).is_ok());
    assert_eq!(fixture.expected["throws"].as_bool(), Some(false));
}

#[test]
fn java_runtime_selection_validate_looks_like_java_rejects_non_java_binary_with_first_line_in_error()
 {
    let fixture = load("validate-looks-like-java-rejects-non-java-binary-with-first-line-in-error");
    let display = fixture.input["display"].as_str().unwrap();
    let captured = fixture.input["capturedOutput"].as_str().unwrap();
    let err = validate_looks_like_java(display, captured).expect_err("expected a rejection");
    let message = err.to_string();
    for expected in fixture.expected["errorMessageContains"].as_array().unwrap() {
        let expected = expected.as_str().unwrap();
        assert!(
            message.contains(expected),
            "{message:?} missing {expected:?}"
        );
    }
}

#[test]
fn java_runtime_selection_minecraft_install_options_table() {
    let fixture = load("minecraft-install-options-table");
    let expected_options = fixture.expected["options"].as_array().unwrap();
    assert_eq!(MINECRAFT_INSTALL_OPTIONS.len(), expected_options.len());
    for (actual, expected) in MINECRAFT_INSTALL_OPTIONS.iter().zip(expected_options) {
        assert_eq!(actual.major, expected["major"].as_i64().unwrap());
        assert_eq!(actual.title, expected["title"].as_str().unwrap());
        assert_eq!(
            actual.minecraft_range,
            expected["minecraftRange"].as_str().unwrap()
        );
        assert_eq!(
            actual.is_recommended,
            expected["isRecommended"].as_bool().unwrap()
        );
    }
}

#[test]
fn java_runtime_selection_recommended_option_matches_required_major_else_falls_back_to_recommended_lts()
 {
    let fixture =
        load("recommended-option-matches-required-major-else-falls-back-to-recommended-lts");
    for case in fixture.input["cases"].as_array().unwrap() {
        let mc = case["minecraftVersion"].as_str();
        let option = recommended_option(mc);
        let expected = fixture.expected["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["minecraftVersion"].as_str() == mc)
            .unwrap();
        assert_eq!(
            option.major,
            expected["selectedMajor"].as_i64().unwrap(),
            "mc={mc:?}"
        );
    }
}

#[test]
fn java_runtime_selection_create_time_java_path_nil_falls_back_to_global_config_default() {
    let fixture = load("create-time-java-path-nil-falls-back-to-global-config-default");
    let request_path = fixture.input["createRequestJavaPath"].as_str();
    let global = fixture.input["globalConfigJavaPath"].as_str().unwrap();
    assert_eq!(
        resolve_create_time_java_path(request_path, global),
        fixture.expected["javaPathUsedForInstaller"]
            .as_str()
            .unwrap()
    );
}

#[test]
fn java_runtime_selection_create_time_java_path_override_takes_precedence_over_global_default() {
    let fixture = load("create-time-java-path-override-takes-precedence-over-global-default");
    let request_path = fixture.input["createRequestJavaPath"].as_str();
    let global = fixture.input["globalConfigJavaPath"].as_str().unwrap();
    assert_eq!(
        resolve_create_time_java_path(request_path, global),
        fixture.expected["javaPathUsedForInstaller"]
            .as_str()
            .unwrap()
    );
}

#[test]
fn java_runtime_selection_settings_resolved_java_path_empty_trimmed_defaults_to_bare_java_command()
{
    let fixture = load("settings-resolved-java-path-empty-trimmed-defaults-to-bare-java-command");
    let trimmed_input = fixture.input["trimmedInput"].as_str().unwrap();
    assert_eq!(
        resolved_settings_java_path(trimmed_input),
        fixture.expected["storedJavaPath"].as_str().unwrap()
    );
}
