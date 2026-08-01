//! Port of `fixtures/atomic-write/`: one test per case, each loading its
//! fixture, building a `FakeFileSystem` from its `existingFiles`, and
//! exercising `atomic_write` (or, for the interrupted-write case, its
//! `temp_path_for` helper directly). No MSC 1 test file exercises this
//! pattern (see each fixture's own `notes`), so these fixtures were
//! characterized directly from `ConfigManager.save` and
//! `WorldSlotManager`'s temp-then-move call sites rather than pulled from
//! Swift assertions.

use msc_infrastructure::atomic_write::{AtomicWriteError, atomic_write, temp_path_for};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use serde_json::Value;
use std::path::{Path, PathBuf};

struct Fixture {
    input: Value,
    expected: Value,
}

fn load(case: &str) -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/atomic-write")
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

fn read_utf8(fs: &FakeFileSystem, path: &Path) -> Option<String> {
    fs.read(path)
        .ok()
        .map(|bytes| String::from_utf8(bytes).expect("fixture contents are valid UTF-8"))
}

/// Cases that call `atomic_write` itself and check the outcome: success
/// with the new content in place, or a typed error with the destination
/// left as it was found.
fn run(case: &str) {
    let fixture = load(case);
    let fs = build_fs(&fixture.input);
    let path = PathBuf::from(fixture.input["path"].as_str().expect("path"));
    let contents = fixture.input["contents"].as_str().unwrap_or("");

    let result = atomic_write(&fs, &path, contents.as_bytes());

    let expected_error = fixture.expected["error"].as_str();
    match (&result, expected_error) {
        (Ok(()), None) => {}
        (Err(AtomicWriteError::MissingParentDirectory(_)), Some("missing_parent_directory")) => {
            // No partial temp file should be left in the (nonexistent)
            // parent directory.
            let temp = temp_path_for(&path);
            assert!(
                fs.read(&temp).is_err(),
                "case {case}: partial temp file left behind at {}",
                temp.display()
            );
        }
        (actual, _) => panic!("case {case}: unexpected result {actual:?}"),
    }

    let expected_contents = fixture.expected["destinationContents"].as_str();
    assert_eq!(
        read_utf8(&fs, &path).as_deref(),
        expected_contents,
        "case {case}: destination contents mismatch"
    );
}

/// The interrupted-write case: writes straight to the primitive's own temp
/// path, without ever calling `atomic_write` or its rename step, and
/// checks the destination is untouched.
fn run_interrupted(case: &str) {
    let fixture = load(case);
    let fs = build_fs(&fixture.input);
    let path = PathBuf::from(fixture.input["path"].as_str().expect("path"));
    let contents = fixture.input["contents"].as_str().unwrap_or("");

    let temp = temp_path_for(&path);
    fs.write(&temp, contents.as_bytes())
        .expect("writing the temp file should not fail");

    let expected_contents = fixture.expected["destinationContents"].as_str();
    assert_eq!(
        read_utf8(&fs, &path).as_deref(),
        expected_contents,
        "case {case}: destination touched before rename"
    );
}

#[test]
fn atomic_write_successful_write_to_new_path() {
    run("successful-write-to-new-path");
}

#[test]
fn atomic_write_overwrite_existing_file_replaces_content() {
    run("overwrite-existing-file-replaces-content");
}

#[test]
fn atomic_write_missing_parent_directory_errors_without_partial_file() {
    run("missing-parent-directory-errors-without-partial-file");
}

#[test]
fn atomic_write_destination_untouched_before_rename() {
    run_interrupted("destination-untouched-before-rename");
}
