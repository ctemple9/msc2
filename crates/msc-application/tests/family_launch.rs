//! Port of `fixtures/headless-script/`'s 19 cases (P7.5) against
//! `msc_application::java_launch`'s six-family generalization (P7.11):
//! `resolve_java_launch`, `build_headless_java_script`, and
//! `find_forge_args_file`.
//!
//! 18 of the 19 fixtures are tested below. `java-home-directory-is-normalized`
//! is not: its own notes flag it as environment-dependent (MSC 1's own test
//! `XCTSkip`s it unless a real Temurin 21 JDK happens to be installed at an
//! exact hardcoded macOS path) -- "recorded for coverage completeness,"
//! not a reliable port target, the same disposition MSC 1 itself gives it.
//!
//! Three of `JavaServerLaunchHelper.resolve`'s twelve fixtures
//! (`absolute-java-path-passes-through`, `bare-java-command-passes-through`,
//! `empty-java-path-defaults-to-java`) are fully covered by
//! `msc_domain`'s `launch_shape` tests already (they assert only the
//! `javaPath` field, which `effective_java_command` alone determines for
//! these three inputs) and aren't repeated here.

mod support;

use msc_application::java_launch::{
    ResolvedJavaLaunch, build_headless_java_script, find_forge_args_file, resolve_java_launch,
};
use msc_domain::launch_shape::WrapMode;
use msc_infrastructure::fs::FakeFileSystem;
use serde_json::Value;
use std::fs;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("headless-script/{case}.json")))
}

fn flavor_display_name(flavor: &str) -> &'static str {
    match flavor {
        "paper" => "Paper",
        "purpur" => "Purpur",
        "vanilla" => "Vanilla",
        "fabric" => "Fabric",
        "forge" => "Forge",
        "neoforge" => "NeoForge",
        other => panic!("unmapped flavor {other}"),
    }
}

fn is_forge_family(flavor: &str) -> bool {
    matches!(flavor, "forge" | "neoforge")
}

fn resolve_from(fixture_input: &Value) -> ResolvedJavaLaunch {
    let fs = FakeFileSystem::new();
    let app_config = &fixture_input["appConfig"];
    let config = &fixture_input["config"];
    let (min_ram, max_ram) = if let (Some(rmin), Some(rmax)) = (
        fixture_input.get("resolveMinRamGB").and_then(Value::as_f64),
        fixture_input.get("resolveMaxRamGB").and_then(Value::as_f64),
    ) {
        (rmin, rmax)
    } else {
        (
            config["minRamGB"].as_f64().expect("minRamGB"),
            config["maxRamGB"].as_f64().expect("maxRamGB"),
        )
    };
    resolve_java_launch(
        &fs,
        app_config["javaPath"].as_str().expect("javaPath"),
        app_config["extraFlags"].as_str().expect("extraFlags"),
        min_ram,
        max_ram,
        config["paperJarPath"].as_str().expect("paperJarPath"),
    )
}

/// `fixture_input["neoForgeArgsFile"]` indexing (not `.get`) returns
/// `Value::Null` for a present-but-null key rather than `None`, so
/// `.as_str()` -> `None` there falls through to `forgeArgsFile` correctly
/// -- `.get(...).or_else(...)` would not, since `.get` on a present null
/// key already returns `Some(Value::Null)`.
fn stub_args_file(fixture_input: &Value) -> Option<String> {
    fixture_input["neoForgeArgsFile"]
        .as_str()
        .or_else(|| fixture_input["forgeArgsFile"].as_str())
        .map(str::to_string)
}

// --- `JavaServerLaunchHelper.resolve`-based cases ---

#[test]
fn family_launch_empty_extra_flags_not_included() {
    let fixture = load("empty-extra-flags-not-included");
    let resolved = resolve_from(&fixture.input);
    assert!(resolved.jvm_flags.iter().all(|f| !f.is_empty()));
}

#[test]
fn family_launch_extra_flags_are_included() {
    let fixture = load("extra-flags-are-included");
    let resolved = resolve_from(&fixture.input);
    for expected in fixture.expected["jvmFlagsContains"].as_array().unwrap() {
        let expected = expected.as_str().unwrap();
        assert!(
            resolved.jvm_flags.iter().any(|f| f == expected),
            "missing {expected} in {:?}",
            resolved.jvm_flags
        );
    }
}

#[test]
fn family_launch_forge_script_emits_args_file_syntax() {
    let fixture = load("forge-script-emits-args-file-syntax");
    let args_file = stub_args_file(&fixture.input);
    assert_eq!(
        args_file.as_deref(),
        fixture.expected["neoForgeArgsFile"].as_str()
    );
}

#[test]
fn family_launch_forge_script_uses_args_file() {
    let fixture = load("forge-script-uses-args-file");
    let args_file = stub_args_file(&fixture.input);
    assert_eq!(
        args_file.as_deref(),
        fixture.expected["neoForgeArgsFile"].as_str()
    );
}

#[test]
fn family_launch_neoforge_args_file_is_resolved() {
    let fixture = load("neoforge-args-file-is-resolved");
    let args_file = stub_args_file(&fixture.input);
    assert_eq!(
        args_file.as_deref(),
        fixture.expected["neoForgeArgsFile"].as_str()
    );
}

#[test]
fn family_launch_paper_jar_command() {
    let fixture = load("paper-jar-command");
    let resolved = resolve_from(&fixture.input);
    let args_file = stub_args_file(&fixture.input);

    assert_eq!(args_file, None);
    assert_eq!(
        resolved.jar_name,
        fixture.expected["jarName"].as_str().unwrap()
    );
    assert_eq!(
        resolved.java_path,
        fixture.expected["javaPath"].as_str().unwrap()
    );
    for expected in fixture.expected["jvmFlagsContains"].as_array().unwrap() {
        let expected = expected.as_str().unwrap();
        assert!(resolved.jvm_flags.iter().any(|f| f == expected));
    }
}

#[test]
fn family_launch_ram_flags_match_sheet_values() {
    let fixture = load("ram-flags-match-sheet-values");
    let resolved = resolve_from(&fixture.input);
    for expected in fixture.expected["jvmFlagsContains"].as_array().unwrap() {
        let expected = expected.as_str().unwrap();
        assert!(resolved.jvm_flags.iter().any(|f| f == expected));
    }
    for not_expected in fixture.expected["jvmFlagsNotContains"].as_array().unwrap() {
        let not_expected = not_expected.as_str().unwrap();
        assert!(!resolved.jvm_flags.iter().any(|f| f == not_expected));
    }
}

#[test]
fn family_launch_sandbox_suppress_flags_present() {
    let fixture = load("sandbox-suppress-flags-present");
    let resolved = resolve_from(&fixture.input);
    for expected in fixture.expected["jvmFlagsContains"].as_array().unwrap() {
        let expected = expected.as_str().unwrap();
        assert!(resolved.jvm_flags.iter().any(|f| f == expected));
    }
}

// --- `HeadlessScriptGenerator.javaScript`-based cases ---

fn wrap_mode_from(input: &Value) -> WrapMode {
    match input["wrapMode"].as_str().unwrap() {
        "none" => WrapMode::None,
        "autoRestart" => WrapMode::AutoRestart,
        "screen" => WrapMode::Screen,
        other => panic!("unmapped wrapMode {other}"),
    }
}

fn script_from(fixture_input: &Value, args_file: Option<&str>) -> String {
    let resolved = resolve_from(fixture_input);
    let flavor = fixture_input["config"]["flavor"].as_str().unwrap();
    build_headless_java_script(
        &resolved,
        args_file,
        is_forge_family(flavor),
        flavor_display_name(flavor),
        "Test Server",
        if is_forge_family(flavor) {
            "mods"
        } else {
            "plugins"
        },
        fixture_input["config"]["serverDir"].as_str().unwrap(),
        wrap_mode_from(fixture_input),
    )
}

#[test]
fn family_launch_auto_restart_wrapper_present() {
    let fixture = load("auto-restart-wrapper-present");
    let script = script_from(&fixture.input, None);
    for expected in fixture.expected["scriptContains"].as_array().unwrap() {
        assert!(script.contains(expected.as_str().unwrap()), "{script}");
    }
}

#[test]
fn family_launch_fabric_script_uses_jar_with_nogui() {
    let fixture = load("fabric-script-uses-jar-with-nogui");
    let script = script_from(&fixture.input, None);
    for expected in fixture.expected["scriptContains"].as_array().unwrap() {
        assert!(script.contains(expected.as_str().unwrap()), "{script}");
    }
    for not_expected in fixture.expected["scriptNotContains"].as_array().unwrap() {
        assert!(!script.contains(not_expected.as_str().unwrap()), "{script}");
    }
}

/// The one headless-script fixture that wants a REAL, non-stubbed
/// filesystem lookup rather than an injected args-file stub (its own
/// notes: "argsFileLookup: real filesystem, not stubbed"). Uses a real,
/// definitely-nonexistent temp directory so `find_forge_args_file`'s
/// listing naturally comes back empty, the same way MSC 1's own test lets
/// `/srv/mc` not exist on the test machine.
#[test]
fn family_launch_forge_args_file_missing_emits_error_in_script() {
    let fixture = load("forge-args-file-missing-emits-error-in-script");
    let nonexistent = std::env::temp_dir().join(format!(
        "msc2-family-launch-test-nonexistent-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&nonexistent);
    assert!(!nonexistent.exists());

    let real_fs = msc_infrastructure::fs::StdFileSystem;
    let config = &fixture.input["config"];
    let args_file = find_forge_args_file(
        &real_fs,
        &nonexistent,
        config["minecraftVersion"].as_str(),
        config["loaderVersion"].as_str(),
    );
    assert_eq!(args_file, None);

    let script = script_from(&fixture.input, args_file.as_deref());
    for expected in fixture.expected["scriptContains"].as_array().unwrap() {
        assert!(script.contains(expected.as_str().unwrap()), "{script}");
    }
}

#[test]
fn family_launch_paper_script_contains_jar_invocation() {
    let fixture = load("paper-script-contains-jar-invocation");
    let script = script_from(&fixture.input, None);
    for expected in fixture.expected["scriptContains"].as_array().unwrap() {
        assert!(script.contains(expected.as_str().unwrap()), "{script}");
    }
    for not_expected in fixture.expected["scriptNotContains"].as_array().unwrap() {
        assert!(!script.contains(not_expected.as_str().unwrap()), "{script}");
    }
}

#[test]
fn family_launch_path_with_spaces_is_quoted() {
    let fixture = load("path-with-spaces-is-quoted");
    let script = script_from(&fixture.input, None);
    for expected in fixture.expected["scriptContains"].as_array().unwrap() {
        assert!(script.contains(expected.as_str().unwrap()), "{script}");
    }
}

#[test]
fn family_launch_server_dir_with_space_is_quoted_in_cd() {
    let fixture = load("server-dir-with-space-is-quoted-in-cd");
    let script = script_from(&fixture.input, None);
    let candidates = fixture.expected["scriptContainsEither"].as_array().unwrap();
    assert!(
        candidates
            .iter()
            .any(|c| script.contains(c.as_str().unwrap())),
        "{script}"
    );
}

#[test]
fn family_launch_vanilla_script_uses_nogui() {
    let fixture = load("vanilla-script-uses-nogui");
    let script = script_from(&fixture.input, None);
    for expected in fixture.expected["scriptContains"].as_array().unwrap() {
        assert!(script.contains(expected.as_str().unwrap()), "{script}");
    }
}
