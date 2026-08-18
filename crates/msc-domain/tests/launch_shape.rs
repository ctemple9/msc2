//! Port of the pure pieces of P7.5's launch-shape fixtures against
//! `msc_domain::launch_shape` (P7.11).
//!
//! `fixtures/args-file-resolution/`'s 12 cases (all 12 tested here) exercise
//! `neoforge_select_args_file`/`forge_select_args_file` -- the *selection*
//! half of `findArgsFile`. Each fixture's `fsTree` is a real on-disk shape
//! (`ArgsFileResolutionTests.swift` materializes it in a temp dir); since
//! the selection functions here are pure and take an already-obtained
//! listing, this test derives that listing directly from the fixture's
//! `fsTree` keys (which version/pair directories contain `unix_args.txt`)
//! rather than writing real files -- no I/O needed to prove the pure
//! decision. The real, I/O-backed directory scan that produces this
//! listing in production is `msc-application`'s job (P7.11's
//! `crates/msc-application/src/java_launch.rs`), exercised end-to-end by
//! `family_launch_forge_args_file_missing_emits_error_in_script` there
//! (`crates/msc-application/tests/family_launch.rs`), the one headless-script
//! fixture that explicitly wants a real, non-stubbed filesystem.
//!
//! `fixtures/headless-script/`'s 12 `JavaServerLaunchHelper.resolve`-based
//! cases stub `neoForgeArgsFile` as a direct input rather than deriving it
//! from an `fsTree` (per those fixtures' own notes: "inject a stub finder
//! returning a fixed path") and combine `javaPath`/`jvmFlags`/`jarName`/
//! `argsFile` assertions into one holistic case -- ported as one composed
//! `resolve_java_launch` in `msc-application` instead of fragmented here.
//! This file covers only the two pure sub-pieces of `resolve` that don't
//! need that composition: `effective_java_command`'s three javaPath cases
//! (empty/bare/absolute all reduce to pure trim-and-default logic, since
//! MSC 1's own `resolve` falls back to this same value whenever the I/O
//! normalization step that follows fails or is a no-op for these inputs),
//! and `jar_basename` against `paper-jar-command`'s `jarName` expectation.

mod support;

use msc_domain::launch_shape::*;
use serde_json::Value;
use support::Fixture;

fn load_args_file(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("args-file-resolution/{case}.json")))
}

fn load_headless(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("headless-script/{case}.json")))
}

/// Extracts the version/pair directory names present in an `fsTree`
/// carrying a `.../<family-base>/<version-or-pair>/unix_args.txt` shape,
/// in the fsTree's own JSON key order (matching how each fixture's own
/// single-entry-at-a-time trees make the eventual real listing order
/// irrelevant to the assertion).
fn installed_dirs_from_fs_tree(fs_tree: &Value, base_marker: &str) -> Vec<String> {
    fs_tree
        .as_object()
        .expect("fsTree object")
        .keys()
        .filter_map(|path| {
            let after_base = path.split(base_marker).nth(1)?;
            after_base.split('/').next().map(str::to_string)
        })
        .collect()
}

fn run_neoforge_case(case: &str) {
    let fixture = load_args_file(case);
    let installed = installed_dirs_from_fs_tree(&fixture.input["fsTree"], "/neoforge/");
    let specific_version = fixture.input["specificVersion"].as_str();
    let actual = neoforge_select_args_file(&installed, specific_version);
    let expected = fixture.expected.as_str().map(str::to_string);
    assert_eq!(actual, expected, "case {case}");
}

fn run_forge_case(case: &str) {
    let fixture = load_args_file(case);
    let installed = installed_dirs_from_fs_tree(&fixture.input["fsTree"], "/forge/");
    let mc = fixture.input["mcVersion"].as_str();
    let forge = fixture.input["forgeVersion"].as_str();
    let actual = forge_select_args_file(&installed, mc, forge);
    let expected = fixture.expected.as_str().map(str::to_string);
    assert_eq!(actual, expected, "case {case}");
}

#[test]
fn launch_shape_neoforge_single_version_found() {
    run_neoforge_case("neoforge-single-version-found");
}

#[test]
fn launch_shape_neoforge_picks_configured_version_among_multiple() {
    run_neoforge_case("neoforge-picks-configured-version-among-multiple");
}

#[test]
fn launch_shape_neoforge_falls_back_when_configured_version_missing() {
    run_neoforge_case("neoforge-falls-back-when-configured-version-missing");
}

#[test]
fn launch_shape_neoforge_nil_version_falls_back_to_first_match() {
    run_neoforge_case("neoforge-nil-version-falls-back-to-first-match");
}

#[test]
fn launch_shape_neoforge_empty_version_falls_back_to_first_match() {
    run_neoforge_case("neoforge-empty-version-falls-back-to-first-match");
}

#[test]
fn launch_shape_neoforge_returns_nil_when_nothing_installed() {
    run_neoforge_case("neoforge-returns-nil-when-nothing-installed");
}

#[test]
fn launch_shape_forge_single_version_found() {
    run_forge_case("forge-single-version-found");
}

#[test]
fn launch_shape_forge_picks_configured_pair_among_multiple() {
    run_forge_case("forge-picks-configured-pair-among-multiple");
}

#[test]
fn launch_shape_forge_falls_back_when_configured_pair_missing() {
    run_forge_case("forge-falls-back-when-configured-pair-missing");
}

#[test]
fn launch_shape_forge_nil_mc_version_falls_back_to_first_match() {
    run_forge_case("forge-nil-mc-version-falls-back-to-first-match");
}

#[test]
fn launch_shape_forge_nil_forge_version_falls_back_to_first_match() {
    run_forge_case("forge-nil-forge-version-falls-back-to-first-match");
}

#[test]
fn launch_shape_forge_returns_nil_when_nothing_installed() {
    run_forge_case("forge-returns-nil-when-nothing-installed");
}

// --- The two pure sub-pieces of `resolve` ---

#[test]
fn launch_shape_absolute_java_path_passes_through() {
    let fixture = load_headless("absolute-java-path-passes-through");
    let raw = fixture.input["appConfig"]["javaPath"].as_str().unwrap();
    assert_eq!(
        effective_java_command(raw),
        fixture.expected["javaPath"].as_str().unwrap()
    );
}

#[test]
fn launch_shape_bare_java_command_passes_through() {
    let fixture = load_headless("bare-java-command-passes-through");
    let raw = fixture.input["appConfig"]["javaPath"].as_str().unwrap();
    assert_eq!(
        effective_java_command(raw),
        fixture.expected["javaPath"].as_str().unwrap()
    );
}

#[test]
fn launch_shape_empty_java_path_defaults_to_java() {
    let fixture = load_headless("empty-java-path-defaults-to-java");
    let raw = fixture.input["appConfig"]["javaPath"].as_str().unwrap();
    assert_eq!(
        effective_java_command(raw),
        fixture.expected["javaPath"].as_str().unwrap()
    );
}

#[test]
fn launch_shape_paper_jar_command_jar_name() {
    let fixture = load_headless("paper-jar-command");
    let raw = fixture.input["config"]["paperJarPath"].as_str().unwrap();
    assert_eq!(
        jar_basename(raw),
        fixture.expected["jarName"].as_str().unwrap()
    );
}

// --- Direct coverage of the remaining pure pieces (composed end-to-end by
// msc-application's family_launch tests against the full headless-script
// fixture set; exercised here in isolation too) ---

#[test]
fn launch_shape_shell_quote_wraps_special_characters() {
    assert_eq!(shell_quote("/srv/mc"), "/srv/mc");
    assert_eq!(shell_quote("/srv/my server"), "\"/srv/my server\"");
    assert_eq!(shell_quote("a\"b"), "\"a\\\"b\"");
}

#[test]
fn launch_shape_build_java_invocation_paper_vs_forge_family() {
    let jvm_flags = vec!["-Xms1024M".to_string(), "-Xmx2048M".to_string()];
    assert_eq!(
        build_java_invocation("java", &jvm_flags, None, "paper.jar", false, "Paper"),
        "java -Xms1024M -Xmx2048M -jar paper.jar --nogui"
    );
    assert_eq!(
        build_java_invocation(
            "java",
            &jvm_flags,
            Some("libraries/net/minecraftforge/forge/1.20.1-47.4.1/unix_args.txt"),
            "server.jar",
            true,
            "Forge"
        ),
        "java -Xms1024M -Xmx2048M @libraries/net/minecraftforge/forge/1.20.1-47.4.1/unix_args.txt nogui"
    );
    let missing = build_java_invocation("java", &jvm_flags, None, "server.jar", true, "Forge");
    assert!(missing.contains("exit 1"));
    assert!(missing.contains("Forge"));
}

#[test]
fn launch_shape_wrap_command_lines_auto_restart_contains_sleep_and_loop() {
    let lines = wrap_command_lines("java -jar paper.jar --nogui", WrapMode::AutoRestart);
    let joined = lines.join("\n");
    assert!(joined.contains("while true; do"));
    assert!(joined.contains("sleep 5"));
}
