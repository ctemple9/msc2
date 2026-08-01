//! Port of the filesystem-backed `JavaRuntimeGuardsTests.swift` cases P1.5
//! deferred to this phase (`fixtures/java-runtime-guards/`,
//! `detect-installed-java-runtimes-*` and `normalization-*`). The other 7
//! cases in that same directory are pure and already covered by
//! `msc-domain`'s `java_runtime_guards.rs`.
//!
//! Test functions are prefixed `java_runtime_detection_` so the plan's
//! Verify command (a plain nextest substring filter, which matches on test
//! name, not file/binary name) selects all of them.

use msc_infrastructure::fs::FakeFileSystem;
use msc_infrastructure::java_runtime_detection::{
    detect_installed_java_runtimes, normalized_java_executable_path,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

struct Fixture {
    input: Value,
    expected: Value,
}

fn load(case: &str) -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/java-runtime-guards")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    let json: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()));
    Fixture {
        input: json["input"].clone(),
        expected: json["expected"].clone(),
    }
}

/// The fixtures stand in for a real temporary directory (MSC 1's own tests
/// create one with `FileManager.default.temporaryDirectory`) using the
/// literal token `<TMP>`. Every *other* fixture path embeds it as a prefix
/// followed by more path (`<TMP>/java`, `<TMP>/bin/java`, ...), which
/// already contains a `/` same as a real absolute path would. But one case
/// (`normalization-home-dir-to-bin-java`) passes the bare token itself as
/// the whole path — and unlike a real temp directory path, the literal
/// string `<TMP>` has no `/` in it at all, which would incorrectly trip
/// `normalized_java_executable_path`'s "no slash -> bare command, pass
/// through unchanged" fast path. Resolving the token to an actual
/// absolute-looking path before it reaches any fixture field (fsTree keys,
/// `path`, `searchRoots`, and every expected path) restores the real
/// temp-directory semantics the Swift test relied on, uniformly rather than
/// special-casing just the one case that happens to expose the gap.
const TMP_ROOT: &str = "/private/tmp/msc2-fixture-java-runtime";

fn resolve_tmp(s: &str) -> String {
    s.replace("<TMP>", TMP_ROOT)
}

fn build_fs(input: &Value) -> FakeFileSystem {
    match input.get("fsTree") {
        Some(tree) => {
            let object = tree.as_object().expect("fsTree must be a JSON object");
            let rewritten: Value = Value::Object(
                object
                    .iter()
                    .map(|(k, v)| (resolve_tmp(k), v.clone()))
                    .collect(),
            );
            FakeFileSystem::from_tree(&rewritten)
        }
        None => FakeFileSystem::new(),
    }
}

/// `PathBuf` equality rather than raw string equality, so a mixed `/`/`\`
/// join on Windows (`Path::join` only inserts the platform separator at the
/// join point, it doesn't rewrite the rest of the string) still compares
/// equal to the fixture's forward-slash literal — same reasoning
/// `path_safety.rs`'s tests already use.
fn assert_paths_eq(actual: &str, expected: &str, case: &str) {
    assert_eq!(
        PathBuf::from(actual),
        PathBuf::from(resolve_tmp(expected)),
        "case {case}"
    );
}

// --- normalizedJavaExecutablePath ---

fn assert_normalization(case: &str) {
    let fixture = load(case);
    let fs = build_fs(&fixture.input);
    let raw_path = resolve_tmp(fixture.input["path"].as_str().expect("path"));
    let actual = normalized_java_executable_path(&fs, &raw_path);

    match fixture.expected["path"].as_str() {
        Some(expected_path) => {
            let actual_path =
                actual.unwrap_or_else(|e| panic!("case {case}: expected Ok, got Err({e:?})"));
            assert_paths_eq(&actual_path, expected_path, case);
        }
        None => {
            let err = actual
                .err()
                .unwrap_or_else(|| panic!("case {case}: expected Err, got Ok"));
            let contains = fixture.expected["errContains"]
                .as_str()
                .expect("errContains");
            assert!(
                err.contains(contains),
                "case {case}: expected error to contain {contains:?}, got {err:?}"
            );
        }
    }
}

#[test]
fn java_runtime_detection_normalization_bare_command_passes_through() {
    assert_normalization("normalization-bare-command-passes-through");
}

#[test]
fn java_runtime_detection_normalization_already_executable_path_unchanged() {
    assert_normalization("normalization-already-executable-path-unchanged");
}

#[test]
fn java_runtime_detection_normalization_home_dir_to_bin_java() {
    assert_normalization("normalization-home-dir-to-bin-java");
}

#[test]
fn java_runtime_detection_normalization_nonexistent_path_returns_error() {
    assert_normalization("normalization-nonexistent-path-returns-error");
}

#[test]
fn java_runtime_detection_normalization_directory_without_bin_java_returns_error() {
    // This fixture's own `fsTree` is `{}` — its notes explain `<TMP>` is a
    // real, freshly created *empty* directory in the MSC 1 test, which the
    // fsTree schema has no way to spell (only "file"/"symlink" entries
    // exist). Seeded explicitly here via `with_dir`, rather than by
    // changing the fixture's frozen JSON or generalizing `from_tree`.
    let case = "normalization-directory-without-bin-java-returns-error";
    let fixture = load(case);
    let raw_path = resolve_tmp(fixture.input["path"].as_str().expect("path"));
    let fs = FakeFileSystem::new().with_dir(raw_path.clone());

    let actual = normalized_java_executable_path(&fs, &raw_path);

    let err = actual
        .err()
        .unwrap_or_else(|| panic!("case {case}: expected Err, got Ok"));
    let contains = fixture.expected["errContains"]
        .as_str()
        .expect("errContains");
    assert!(
        err.contains(contains),
        "case {case}: expected error to contain {contains:?}, got {err:?}"
    );
}

// --- detectInstalledJavaRuntimes ---

fn search_roots(input: &Value) -> Vec<String> {
    input["searchRoots"]
        .as_array()
        .expect("searchRoots array")
        .iter()
        .map(|v| resolve_tmp(v.as_str().expect("searchRoots entry")))
        .collect()
}

#[test]
fn java_runtime_detection_finds_macos_jdk_bundle() {
    let case = "detect-installed-java-runtimes-finds-macos-jdk-bundle";
    let fixture = load(case);
    let fs = build_fs(&fixture.input);
    let roots = search_roots(&fixture.input);

    let runtimes = detect_installed_java_runtimes(&fs, &roots);

    let expected = fixture.expected["runtimes"]
        .as_array()
        .expect("runtimes array");
    assert_eq!(runtimes.len(), expected.len(), "case {case}");
    let entry = &expected[0];
    assert_paths_eq(
        &runtimes[0].executable_path,
        entry["executablePath"].as_str().expect("executablePath"),
        case,
    );
    assert_eq!(
        runtimes[0].major_version,
        entry["majorVersion"].as_i64(),
        "case {case}"
    );
    assert_eq!(
        runtimes[0].name,
        entry["name"].as_str().expect("name"),
        "case {case}"
    );
}

#[test]
fn java_runtime_detection_finds_plain_java_home() {
    let case = "detect-installed-java-runtimes-finds-plain-java-home";
    let fixture = load(case);
    let fs = build_fs(&fixture.input);
    let roots = search_roots(&fixture.input);

    let runtimes = detect_installed_java_runtimes(&fs, &roots);

    let expected_set: BTreeSet<PathBuf> = fixture.expected["executablePathsSet"]
        .as_array()
        .expect("executablePathsSet array")
        .iter()
        .map(|v| PathBuf::from(resolve_tmp(v.as_str().expect("executablePathsSet entry"))))
        .collect();
    let actual_set: BTreeSet<PathBuf> = runtimes
        .iter()
        .map(|r| PathBuf::from(&r.executable_path))
        .collect();
    assert_eq!(actual_set, expected_set, "case {case}");

    let first_major = fixture.expected["firstMajorVersion"].as_i64();
    assert_eq!(
        runtimes.first().and_then(|r| r.major_version),
        first_major,
        "case {case}"
    );
}

#[test]
fn java_runtime_detection_ignores_invalid_candidates() {
    let case = "detect-installed-java-runtimes-ignores-invalid-candidates";
    let fixture = load(case);
    let fs = build_fs(&fixture.input);
    let roots = search_roots(&fixture.input);

    let runtimes = detect_installed_java_runtimes(&fs, &roots);

    assert!(
        runtimes.is_empty(),
        "case {case}: expected no runtimes, got {runtimes:?}"
    );
}
