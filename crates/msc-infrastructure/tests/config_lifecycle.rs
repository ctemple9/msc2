//! Port of `fixtures/config-lifecycle/`: one test per case, each loading
//! its fixture, building a `FakeFileSystem` from its `existingFiles`, and
//! exercising `load_config`/`save_config`. No MSC 1 test file exercises
//! `ConfigManager`'s corruption-recovery or unknown-field behavior (see
//! each fixture's own `notes`), so these fixtures were characterized
//! directly from `ConfigManager.swift` and `msc2-engineering.md` §7
//! rather than pulled from Swift assertions.

use msc_infrastructure::config_repository::{corrupt_backup_path, load_config, save_config};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Fixture {
    input: Value,
    expected: Value,
}

fn load(case: &str) -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/config-lifecycle")
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

fn build_fs(input: &Value) -> FakeFileSystem {
    let mut fs = FakeFileSystem::new();
    if let Some(files) = input["existingFiles"].as_array() {
        for entry in files {
            let path = entry["path"].as_str().expect("existingFiles[].path");
            let contents = entry["contents"].as_str().unwrap_or("");
            fs = fs.with_file(path, contents.as_bytes().to_vec(), false);
        }
    }
    fs
}

fn now_from(input: &Value) -> SystemTime {
    let nanos = input["nowUnixNanos"].as_u64().expect("nowUnixNanos");
    UNIX_EPOCH + Duration::from_nanos(nanos)
}

fn read_json(fs: &FakeFileSystem, path: &Path) -> Option<Value> {
    fs.read(path)
        .ok()
        .map(|bytes| serde_json::from_slice(&bytes).expect("fixture destination is valid JSON"))
}

/// Cases that only exercise `load_config` — the three outcomes described
/// in `config_repository`'s module docs.
fn run(case: &str) {
    let fixture = load(case);
    let fs = build_fs(&fixture.input);
    let path = PathBuf::from(fixture.input["path"].as_str().expect("path"));
    let defaults = &fixture.input["defaults"];
    let now = now_from(&fixture.input);

    let outcome =
        load_config(&fs, &path, defaults, now).unwrap_or_else(|e| panic!("case {case}: {e}"));

    assert_eq!(
        outcome.config, fixture.expected["config"],
        "case {case}: config mismatch"
    );

    let expected_backup_path = fixture.expected["corruptBackupPath"]
        .as_str()
        .map(PathBuf::from);
    assert_eq!(
        outcome.corrupt_backup_path, expected_backup_path,
        "case {case}: corrupt backup path mismatch"
    );

    if let Some(expected_backup_contents) = fixture.expected["corruptBackupContents"].as_str() {
        let backup_path = outcome
            .corrupt_backup_path
            .as_ref()
            .unwrap_or_else(|| panic!("case {case}: expected a corrupt backup path"));
        let actual = fs
            .read(backup_path)
            .ok()
            .map(|bytes| String::from_utf8(bytes).expect("fixture backup is valid UTF-8"));
        assert_eq!(
            actual.as_deref(),
            Some(expected_backup_contents),
            "case {case}: corrupt backup contents mismatch"
        );
    }

    if let Some(expected_destination) = fixture.expected.get("destinationContents") {
        assert_eq!(
            read_json(&fs, &path).as_ref(),
            Some(expected_destination),
            "case {case}: destination contents mismatch"
        );
    }
}

#[test]
fn config_lifecycle_valid_file_loads_cleanly() {
    run("valid-file-loads-cleanly");
}

#[test]
fn config_lifecycle_missing_file_falls_back_to_defaults() {
    run("missing-file-falls-back-to-defaults");
}

#[test]
fn config_lifecycle_corrupted_json_preserved_and_defaults_returned() {
    run("corrupted-json-preserved-and-defaults-returned");

    // Also pin down the exposed naming rule itself, independent of the
    // fixture's own expected-value assertion above.
    let fixture = load("corrupted-json-preserved-and-defaults-returned");
    let path = PathBuf::from(fixture.input["path"].as_str().expect("path"));
    let now = now_from(&fixture.input);
    let expected = fixture.expected["corruptBackupPath"]
        .as_str()
        .map(PathBuf::from);
    assert_eq!(Some(corrupt_backup_path(&path, now)), expected);
}

#[test]
fn config_lifecycle_unknown_fields_survive_read_modify_write() {
    let case = "unknown-fields-survive-read-modify-write";
    let fixture = load(case);
    let fs = build_fs(&fixture.input);
    let path = PathBuf::from(fixture.input["path"].as_str().expect("path"));
    let defaults = &fixture.input["defaults"];
    let now = now_from(&fixture.input);

    let outcome =
        load_config(&fs, &path, defaults, now).unwrap_or_else(|e| panic!("case {case}: {e}"));
    assert_eq!(
        outcome.config, fixture.expected["config"],
        "case {case}: loaded config mismatch"
    );

    let mut mutated = outcome.config;
    let mutate_key = fixture.input["mutateKey"].as_str().expect("mutateKey");
    mutated[mutate_key] = fixture.input["mutateValue"].clone();

    save_config(&fs, &path, &mutated).unwrap_or_else(|e| panic!("case {case}: save failed: {e}"));

    assert_eq!(
        read_json(&fs, &path).as_ref(),
        Some(&fixture.expected["afterSaveContents"]),
        "case {case}: after-save contents mismatch"
    );
}
