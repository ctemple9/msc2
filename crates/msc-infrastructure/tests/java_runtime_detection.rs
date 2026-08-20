//! Port of the filesystem-backed `JavaRuntimeGuardsTests.swift` cases P1.5
//! deferred to this phase (`fixtures/java-runtime-guards/`,
//! `detect-installed-java-runtimes-*` and `normalization-*`). The other 7
//! cases in that same directory are pure and already covered by
//! `msc-domain`'s `java_runtime_guards.rs`.
//!
//! Test functions are prefixed `java_runtime_detection_` so the plan's
//! Verify command (a plain nextest substring filter, which matches on test
//! name, not file/binary name) selects all of them.

use msc_domain::identity::ServerType;
use msc_domain::java_runtime::JavaVersionProbe;
use msc_infrastructure::fs::FakeFileSystem;
use msc_infrastructure::java_runtime_detection::{
    JavaOnPathStatus, check_java_on_path, detect_installed_java_runtimes,
    has_critical_missing_dependency, is_java_installed, java_on_path_field_autofill,
    normalized_java_executable_path, run_java_version_probe,
};
use msc_infrastructure::process::FakeProcessSupervisor;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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

// --- checkJavaOnPath / isJavaInstalled / hasCriticalMissingDependency ---
//
// P7.16's own 4 fixtures (the other 2 of the 6 P7.12 deferred here are
// `java_runtime_install.rs`'s Adoptium cases), under
// `fixtures/java-runtime-selection/` rather than `-guards/`.

struct SelectionFixture {
    input: Value,
    expected: Value,
}

fn load_selection(case: &str) -> SelectionFixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/java-runtime-selection")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    let json: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()));
    SelectionFixture {
        input: json["input"].clone(),
        expected: json["expected"].clone(),
    }
}

/// `run_which_java` spawns then blocks polling `drain_events` in a loop —
/// `FakeProcessSupervisor` has no automatic responder, so a test has to
/// feed it output/exit from a second thread once the spawn has actually
/// happened. Polls `spawned_requests()` rather than assuming a fixed pid,
/// since `run_which_java`'s pid comes from the fake's own counter.
fn drive_which_java(
    supervisor: &Arc<FakeProcessSupervisor>,
    stdout: impl Into<String>,
    exit_code: i32,
) {
    let supervisor = Arc::clone(supervisor);
    let stdout = stdout.into();
    thread::spawn(move || {
        loop {
            if let Some(pid) = supervisor.spawned_requests().first().map(|(pid, _)| *pid) {
                if !stdout.is_empty() {
                    let _ = supervisor.emit_stdout(pid, stdout.as_bytes().to_vec());
                }
                if exit_code == 0 {
                    let _ = supervisor.exit_normally(pid);
                } else {
                    let _ = supervisor.crash(pid, exit_code);
                }
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
    });
}

#[test]
fn java_runtime_detection_check_java_on_path_found_autofills_empty_preference_field() {
    let fixture = load_selection("check-java-on-path-found-autofills-empty-preference-field");
    let exit_code = fixture.input["whichJavaExitCode"].as_i64().unwrap() as i32;
    let output = fixture.input["whichJavaOutput"].as_str().unwrap().trim();
    let current_field = fixture.input["currentJavaPathField"].as_str().unwrap();
    let expected_path = fixture.expected["javaPathField"].as_str().unwrap();

    let supervisor = Arc::new(FakeProcessSupervisor::new());
    drive_which_java(&supervisor, output, exit_code);
    let status = check_java_on_path(supervisor.as_ref());

    assert_eq!(
        status,
        JavaOnPathStatus::Found {
            path: expected_path.to_string()
        }
    );
    assert_eq!(
        java_on_path_field_autofill(current_field, &status).as_deref(),
        Some(expected_path)
    );
}

#[test]
fn java_runtime_detection_check_java_on_path_not_found_sets_status() {
    let fixture = load_selection("check-java-on-path-not-found-sets-status");
    let exit_code = fixture.input["whichJavaExitCode"].as_i64().unwrap() as i32;
    let output = fixture.input["whichJavaOutput"].as_str().unwrap();
    let current_field = fixture.input["currentJavaPathField"].as_str().unwrap();

    let supervisor = Arc::new(FakeProcessSupervisor::new());
    drive_which_java(&supervisor, output, exit_code);
    let status = check_java_on_path(supervisor.as_ref());

    assert_eq!(status, JavaOnPathStatus::NotFound);
    // The stale field is left exactly as-is — the fixture's own point.
    assert_eq!(java_on_path_field_autofill(current_field, &status), None);
    assert_eq!(
        fixture.expected["javaPathField"].as_str().unwrap(),
        current_field
    );
}

#[test]
fn java_runtime_detection_has_critical_missing_dependency_blocks_when_java_configured_and_missing()
{
    let fixture = load_selection(
        "has-critical-missing-dependency-blocks-when-java-server-configured-and-missing",
    );
    let server_types: Vec<ServerType> = fixture.input["serverTypes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| ServerType::from_raw_value(v.as_str().unwrap()).unwrap())
        .collect();
    let exit_code = fixture.input["whichJavaExitCode"].as_i64().unwrap() as i32;
    let output = fixture.input["whichJavaOutput"].as_str().unwrap();

    let supervisor = Arc::new(FakeProcessSupervisor::new());
    drive_which_java(&supervisor, output, exit_code);
    let result = has_critical_missing_dependency(supervisor.as_ref(), &server_types);

    assert_eq!(
        result,
        fixture.expected["hasCriticalMissingDependency"]
            .as_bool()
            .unwrap()
    );
    // `is_java_installed` alone should agree — same probe, same input.
    let supervisor2 = Arc::new(FakeProcessSupervisor::new());
    drive_which_java(&supervisor2, output, exit_code);
    assert!(!is_java_installed(supervisor2.as_ref()));
}

#[test]
fn java_runtime_detection_has_critical_missing_dependency_skips_java_check_when_only_bedrock_configured()
 {
    let fixture = load_selection(
        "has-critical-missing-dependency-skips-java-check-when-only-bedrock-configured",
    );
    let server_types: Vec<ServerType> = fixture.input["serverTypes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| ServerType::from_raw_value(v.as_str().unwrap()).unwrap())
        .collect();

    // No `drive_which_java` puppeteer: source's own guard means
    // `isJavaInstalled()` (and so `run_which_java`) must never even be
    // called for a fleet with no Java servers configured. A supervisor
    // that would hang forever if actually polled proves the short-circuit
    // rather than merely asserting the returned value.
    let supervisor = FakeProcessSupervisor::new();
    let result = has_critical_missing_dependency(&supervisor, &server_types);

    assert_eq!(
        result,
        fixture.expected["hasCriticalMissingDependency"]
            .as_bool()
            .unwrap()
    );
    assert!(
        supervisor.spawned_requests().is_empty(),
        "which java must not be spawned when no Java server is configured"
    );
}

// ---------------------------------------------------------------------
// P7.31: `run_java_version_probe`, the create/start-time counterpart to
// `run_which_java` above -- proves the real spawn/poll/combine wiring
// `msc_domain::java_runtime::evaluate_java_runtime_guard` is fed from,
// not the guard's own pure decision logic (already covered by
// `msc-domain`'s inline `java_runtime::guard_tests`).
// ---------------------------------------------------------------------

#[test]
fn java_runtime_detection_run_java_version_probe_captures_combined_output() {
    let supervisor = Arc::new(FakeProcessSupervisor::new());
    drive_which_java(&supervisor, "openjdk version \"21.0.1\" 2023-10-17\n", 0);

    let probe = run_java_version_probe(supervisor.as_ref(), "/usr/bin/java");

    assert_eq!(
        probe,
        JavaVersionProbe::Captured {
            output: "openjdk version \"21.0.1\" 2023-10-17\n".to_string()
        }
    );
}

#[test]
fn java_runtime_detection_run_java_version_probe_not_found_on_spawn_failure() {
    let supervisor = FakeProcessSupervisor::new();
    supervisor.fail_next_spawn("no such file or directory");

    let probe = run_java_version_probe(&supervisor, "/nonexistent/java");

    assert_eq!(probe, JavaVersionProbe::NotFound);
}

#[test]
fn java_runtime_detection_run_java_version_probe_spawns_the_dash_version_flag() {
    let supervisor = Arc::new(FakeProcessSupervisor::new());
    drive_which_java(&supervisor, "openjdk version \"21.0.1\"\n", 0);

    let _ = run_java_version_probe(supervisor.as_ref(), "/usr/bin/java");

    let requests = supervisor.spawned_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].1.executable_path,
        PathBuf::from("/usr/bin/java")
    );
    assert_eq!(requests[0].1.arguments, vec!["-version".to_string()]);
}
