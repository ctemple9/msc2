//! Exercises `fixtures/jar-templates/`'s seven store-owned cases (see
//! `template_store.rs`'s own module doc for which three of the ten stay
//! `msc-application`'s job) plus this module's own path-safety
//! composition, which no fixture covers.

use msc_domain::identity::JavaServerFlavor;
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::template_store::{
    ArchiveOutcome, ArchiveSkipReason, TemplateStoreError, archive_jar, copy_into_server_dir,
    latest_template, list_templates,
};
use serde_json::Value;
use std::path::Path;

struct Fixture {
    input: Value,
    expected: Value,
}

fn load(case: &str) -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/jar-templates")
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

const HOME_DIR: &str = "/home/msc";
const ARCHIVE_DIR: &str = "/servers/_paper_templates";
const SOURCE_JAR: &str = "/tmp/downloaded.jar";

// --- archiveServerJar ---

#[test]
fn template_store_archive_jar_paper_filename_pattern_uses_build_int() {
    let fixture = load("archive-jar-paper-filename-pattern-uses-build-int");
    let version = fixture.input["result"]["version"].as_str().unwrap();
    let build = fixture.input["result"]["build"].as_str().unwrap();
    let expected_filename = fixture.expected["archiveFilename"].as_str().unwrap();

    let fs = FakeFileSystem::new().with_file(SOURCE_JAR, b"paper-bytes".to_vec(), false);

    let outcome = archive_jar(
        &fs,
        Path::new(ARCHIVE_DIR),
        Path::new(HOME_DIR),
        JavaServerFlavor::Paper,
        version,
        build,
        Path::new(SOURCE_JAR),
    )
    .expect("archive_jar");

    assert_eq!(
        outcome,
        ArchiveOutcome::Archived {
            filename: expected_filename.to_string()
        }
    );
    let archived = fs
        .read(&Path::new(ARCHIVE_DIR).join(expected_filename))
        .expect("archived jar readable");
    assert_eq!(archived, b"paper-bytes");
}

#[test]
fn template_store_archive_jar_already_exists_in_archive_skipped_not_overwritten() {
    let fixture = load("archive-jar-already-exists-in-archive-skipped-not-overwritten");
    let expected_filename = fixture.input["archiveFilename"].as_str().unwrap();
    assert!(fixture.expected["fileCopied"].as_bool() == Some(false));

    // The fixture names the archive filename directly rather than a
    // version/build pair — "1.21.4"/"231" is the pair that produces it
    // (proved separately by the build-int case above), reconstructed
    // here only to drive `archive_jar` the same way a real caller would.
    let fs = FakeFileSystem::new()
        .with_file(SOURCE_JAR, b"new-bytes".to_vec(), false)
        .with_file(
            Path::new(ARCHIVE_DIR).join(expected_filename),
            b"already-here".to_vec(),
            false,
        );

    let outcome = archive_jar(
        &fs,
        Path::new(ARCHIVE_DIR),
        Path::new(HOME_DIR),
        JavaServerFlavor::Paper,
        "1.21.4",
        "231",
        Path::new(SOURCE_JAR),
    )
    .expect("archive_jar");

    assert_eq!(
        outcome,
        ArchiveOutcome::AlreadyArchived {
            filename: expected_filename.to_string()
        }
    );
    // Untouched — the pre-seeded bytes, not the "new" source bytes.
    let archived = fs
        .read(&Path::new(ARCHIVE_DIR).join(expected_filename))
        .expect("archived jar readable");
    assert_eq!(archived, b"already-here");
}

#[test]
fn template_store_archive_jar_purpur_vanilla_fabric_filename_patterns() {
    let fixture = load("archive-jar-purpur-vanilla-fabric-filename-patterns");
    let cases = fixture.input["cases"].as_array().unwrap();
    let expected_names = fixture.expected["archiveFilenames"].as_array().unwrap();
    assert_eq!(cases.len(), expected_names.len());

    for (i, case) in cases.iter().enumerate() {
        let flavor = JavaServerFlavor::from_raw_value(case["flavor"].as_str().unwrap())
            .expect("known flavor");
        let version = case["result"]["version"].as_str().unwrap();
        let build = case["result"]["build"].as_str().unwrap();
        let expected_filename = expected_names[i].as_str().unwrap();

        let fs = FakeFileSystem::new().with_file(SOURCE_JAR, b"bytes".to_vec(), false);
        let outcome = archive_jar(
            &fs,
            Path::new(ARCHIVE_DIR),
            Path::new(HOME_DIR),
            flavor,
            version,
            build,
            Path::new(SOURCE_JAR),
        )
        .unwrap_or_else(|e| panic!("case {i} ({flavor:?}): archive_jar failed: {e}"));

        assert_eq!(
            outcome,
            ArchiveOutcome::Archived {
                filename: expected_filename.to_string()
            },
            "case {i} ({flavor:?})"
        );
    }
}

#[test]
fn template_store_archive_jar_unsupported_flavor_silently_returns_no_op() {
    let fixture = load("archive-jar-unsupported-flavor-silently-returns-no-op");
    let flavor =
        JavaServerFlavor::from_raw_value(fixture.input["flavor"].as_str().unwrap()).unwrap();
    assert!(fixture.expected["archiveFilename"].is_null());
    assert!(fixture.expected["fileCopied"].as_bool() == Some(false));

    let fs = FakeFileSystem::new().with_file(SOURCE_JAR, b"bytes".to_vec(), false);
    let outcome = archive_jar(
        &fs,
        Path::new(ARCHIVE_DIR),
        Path::new(HOME_DIR),
        flavor,
        "1.21.4",
        "n/a",
        Path::new(SOURCE_JAR),
    )
    .expect("archive_jar");

    assert_eq!(
        outcome,
        ArchiveOutcome::Skipped(ArchiveSkipReason::UnsupportedFlavor)
    );
}

/// Not tied to a `fixtures/jar-templates` case on its own — it exercises
/// the `skippedIfBuildNotInt` flag the build-int fixture documents
/// (`archive-jar-paper-filename-pattern-uses-build-int.json`) but never
/// actually drives with a non-numeric build, since its own example
/// build ("231") is numeric throughout.
#[test]
fn template_store_archive_jar_paper_non_numeric_build_skips() {
    let fs = FakeFileSystem::new().with_file(SOURCE_JAR, b"bytes".to_vec(), false);
    let outcome = archive_jar(
        &fs,
        Path::new(ARCHIVE_DIR),
        Path::new(HOME_DIR),
        JavaServerFlavor::Paper,
        "1.21.4",
        "n/a",
        Path::new(SOURCE_JAR),
    )
    .expect("archive_jar");

    assert_eq!(
        outcome,
        ArchiveOutcome::Skipped(ArchiveSkipReason::NonNumericPaperBuild)
    );
}

// --- latestTemplate ---

fn seed_templates(fs: FakeFileSystem, dir: &str, files: &[&str]) -> FakeFileSystem {
    let mut fs = fs;
    for name in files {
        fs = fs.with_file(Path::new(dir).join(name), b"jar-bytes".to_vec(), false);
    }
    fs
}

#[test]
fn template_store_latest_template_no_matching_prefix_returns_nil() {
    let fixture = load("latest-template-no-matching-prefix-returns-nil");
    let prefix = fixture.input["prefixLowercased"].as_str().unwrap();
    let files: Vec<&str> = fixture.input["filesInDir"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(fixture.expected["picked"].is_null());

    let dir = "/servers/_paper_templates";
    let fs = seed_templates(FakeFileSystem::new(), dir, &files);

    let picked =
        latest_template(&fs, Path::new(dir), Path::new(HOME_DIR), prefix).expect("latest_template");
    assert!(picked.is_none());
}

#[test]
fn template_store_latest_template_picks_lexicographically_last_matching_prefix() {
    let fixture = load("latest-template-picks-lexicographically-last-matching-prefix");
    let prefix = fixture.input["prefixLowercased"].as_str().unwrap();
    let files: Vec<&str> = fixture.input["filesInDir"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let expected_picked = fixture.expected["picked"].as_str().unwrap();

    let dir = "/servers/_paper_templates";
    let fs = seed_templates(FakeFileSystem::new(), dir, &files);

    let picked = latest_template(&fs, Path::new(dir), Path::new(HOME_DIR), prefix)
        .expect("latest_template")
        .expect("a template should be picked");
    assert_eq!(picked.filename, expected_picked);
}

// --- template listing sort order ---

#[test]
fn template_store_template_listing_sorted_localized_case_insensitive_ascending_not_lexicographic() {
    let fixture =
        load("template-listing-sorted-localized-case-insensitive-ascending-not-lexicographic");
    let files: Vec<&str> = fixture.input["filesInDir"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let expected_order: Vec<&str> = fixture.expected["order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();

    let dir = "/servers/_paper_templates";
    let fs = seed_templates(FakeFileSystem::new(), dir, &files);

    let listed = list_templates(&fs, Path::new(dir), Path::new(HOME_DIR)).expect("list_templates");
    let names: Vec<&str> = listed.iter().map(|t| t.filename.as_str()).collect();
    assert_eq!(names, expected_order);
}

// --- path safety, not exercised by any fixtures/jar-templates case ---

#[test]
fn template_store_list_templates_refuses_root_filesystem_as_template_dir() {
    let fs = FakeFileSystem::new();
    let err = list_templates(&fs, Path::new("/"), Path::new(HOME_DIR))
        .expect_err("root is never a legitimate template dir");
    assert!(matches!(err, TemplateStoreError::PathSafety(_)));
}

#[test]
fn template_store_copy_into_server_dir_refuses_escaping_filename() {
    let fs = FakeFileSystem::new().with_file(SOURCE_JAR, b"paper.jar bytes".to_vec(), false);
    let err = copy_into_server_dir(
        &fs,
        Path::new(SOURCE_JAR),
        Path::new("/servers/java/my_server"),
        Path::new(HOME_DIR),
        "../../etc/escape.jar",
    )
    .expect_err("a dest filename that escapes server_dir must be refused");
    assert!(matches!(err, TemplateStoreError::PathSafety(_)));
}

#[test]
fn template_store_copy_into_server_dir_copies_bytes_to_resolved_destination() {
    let fs = FakeFileSystem::new().with_file(SOURCE_JAR, b"paper.jar bytes".to_vec(), false);
    let dest = copy_into_server_dir(
        &fs,
        Path::new(SOURCE_JAR),
        Path::new("/servers/java/my_server"),
        Path::new(HOME_DIR),
        "paper.jar",
    )
    .expect("copy_into_server_dir");

    assert_eq!(dest, Path::new("/servers/java/my_server/paper.jar"));
    assert_eq!(fs.read(&dest).unwrap(), b"paper.jar bytes");
}
