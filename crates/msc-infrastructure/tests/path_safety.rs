//! Port of `fixtures/path-safety/`: one test per case, each loading its
//! fixture and running it through `safe_path` against a `FakeFileSystem`
//! built from that fixture's `fsTree`. No MSC 1 test file exercises this
//! logic (see each fixture's own `notes`), so these fixtures were
//! characterized directly from `resolvedServerFileURL` and
//! `validateResetDeletionTarget` rather than pulled from Swift assertions.

use msc_infrastructure::fs::FakeFileSystem;
use msc_infrastructure::path_safety::{PathSafetyError, safe_path};
use serde_json::Value;
use std::path::{Path, PathBuf};

struct Fixture {
    input: Value,
    expected: Value,
}

fn load(case: &str) -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/path-safety")
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

fn run(case: &str) {
    let fixture = load(case);

    let root = PathBuf::from(fixture.input["root"].as_str().expect("root"));
    let relative_path = fixture.input["relativePath"]
        .as_str()
        .expect("relativePath");
    let requested = if relative_path.is_empty() {
        None
    } else {
        Some(relative_path)
    };
    let home_dir = PathBuf::from(fixture.input["homeDir"].as_str().expect("homeDir"));
    let fs = FakeFileSystem::from_tree(&fixture.input["fsTree"]);

    let actual = safe_path(&fs, &root, requested, &home_dir);

    let expected_path = fixture.expected["path"].as_str();
    let expected_error = fixture.expected["error"].as_str();

    match (actual, expected_path, expected_error) {
        (Ok(path), Some(expected_path), None) => {
            assert_eq!(
                path,
                PathBuf::from(expected_path),
                "case {case}: resolved path mismatch"
            );
        }
        (Err(PathSafetyError::Escape { .. }), None, Some("escape")) => {}
        (Err(PathSafetyError::ForbiddenRoot(_)), None, Some("forbidden_root")) => {}
        (actual, _, _) => panic!("case {case}: unexpected result {actual:?}"),
    }
}

#[test]
fn path_safety_plain_in_root_path() {
    run("plain-in-root-path");
}

#[test]
fn path_safety_dot_dot_escape_rejected() {
    run("dot-dot-escape-rejected");
}

#[test]
fn path_safety_symlink_inside_root_escapes() {
    run("symlink-inside-root-escapes");
}

#[test]
fn path_safety_empty_relative_path_returns_root() {
    run("empty-relative-path-returns-root");
}

#[test]
fn path_safety_forbidden_root_filesystem_root_rejected() {
    run("forbidden-root-filesystem-root-rejected");
}

#[test]
fn path_safety_forbidden_root_home_directory_rejected() {
    run("forbidden-root-home-directory-rejected");
}

#[test]
fn path_safety_sibling_name_prefix_is_still_an_escape() {
    run("sibling-name-prefix-is-still-an-escape");
}
