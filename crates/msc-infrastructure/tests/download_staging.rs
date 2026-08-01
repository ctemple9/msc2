//! Port of `fixtures/download-staging/`: one test per case, each loading
//! its fixture, building a `FakeFileSystem` from its `existingFiles`, and
//! exercising `stage_download`. No MSC 1 test file exercises a
//! checksum-verified download (see each fixture's own `notes`), so these
//! fixtures were characterized directly from `msc2-engineering.md` §7 and
//! the three real download workflows it names.

use msc_infrastructure::download_staging::{sha1_hex, stage_download};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use serde_json::Value;
use std::path::{Path, PathBuf};

struct Fixture {
    input: Value,
    expected: Value,
}

fn load(case: &str) -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/download-staging")
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

fn temp_path_for(dest: &Path) -> PathBuf {
    let file_name = dest.file_name().expect("dest has a file name");
    dest.with_file_name(format!(".{}.tmp", file_name.to_string_lossy()))
}

/// Runs one case against `stage_download` and checks every field the
/// fixture's `expected` object carries. Every case in this domain shares
/// the same input/expected shape, unlike `config-lifecycle`'s three
/// distinct outcomes, so one runner covers all four.
fn run(case: &str) {
    let fixture = load(case);
    let fs = build_fs(&fixture.input);
    let dest = PathBuf::from(fixture.input["dest"].as_str().expect("dest"));
    let bytes = fixture.input["bytesUtf8"]
        .as_str()
        .expect("bytesUtf8")
        .as_bytes();
    let origin_url = fixture.input["originUrl"].as_str().expect("originUrl");
    let version = fixture.input["version"].as_str().expect("version");
    let expected_sha1 = fixture.input["expectedSha1Hex"].as_str();

    let result = stage_download(&fs, &dest, bytes, origin_url, version, expected_sha1);

    match fixture.expected["error"].as_str() {
        None => {
            let cached =
                result.unwrap_or_else(|e| panic!("case {case}: stage_download failed: {e}"));
            let expected_cached = &fixture.expected["cachedFile"];
            assert_eq!(
                cached.path,
                PathBuf::from(expected_cached["path"].as_str().expect("cachedFile.path")),
                "case {case}: cached path mismatch"
            );
            assert_eq!(
                cached.origin_url,
                expected_cached["originUrl"]
                    .as_str()
                    .expect("cachedFile.originUrl"),
                "case {case}: cached originUrl mismatch"
            );
            assert_eq!(
                cached.version,
                expected_cached["version"]
                    .as_str()
                    .expect("cachedFile.version"),
                "case {case}: cached version mismatch"
            );
        }
        Some("checksum_mismatch") => {
            assert!(
                matches!(
                    result,
                    Err(msc_infrastructure::download_staging::DownloadStagingError::ChecksumMismatch { .. })
                ),
                "case {case}: expected a checksum mismatch error, got {result:?}"
            );
        }
        Some(other) => panic!("case {case}: fixture names unknown expected error {other:?}"),
    }

    let expected_contents = fixture.expected["destinationContents"]
        .as_str()
        .expect("destinationContents");
    let actual_contents = fs
        .read(&dest)
        .map(|bytes| String::from_utf8(bytes).expect("destination is valid UTF-8"))
        .unwrap_or_else(|e| panic!("case {case}: could not read destination: {e}"));
    assert_eq!(
        actual_contents, expected_contents,
        "case {case}: destination contents mismatch"
    );

    let expected_temp_exists = fixture.expected["tempFileExists"]
        .as_bool()
        .expect("tempFileExists");
    let actual_temp_exists = fs.read(&temp_path_for(&dest)).is_ok();
    assert_eq!(
        actual_temp_exists, expected_temp_exists,
        "case {case}: temp file existence mismatch"
    );
}

#[test]
fn download_staging_successful_stage_and_move_with_matching_checksum() {
    run("successful-stage-and-move-with-matching-checksum");

    // Not from the fixture above — the fixture's own expected checksum
    // was computed out-of-band (Python's hashlib), which already proves
    // sha1_hex is correct for that one input. These are the published
    // FIPS 180-4 / RFC 3174 test vectors instead, checked directly
    // against sha1_hex here rather than as a separate #[test], so the
    // nextest count for this domain stays at the fixture count (4) — same
    // reasoning as P3.7's config_lifecycle.rs and P3.13's audit_log.rs.
    assert_eq!(sha1_hex(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
    assert_eq!(
        sha1_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
    );
}

#[test]
fn download_staging_checksum_mismatch_rejected_without_touching_destination() {
    run("checksum-mismatch-rejected-without-touching-destination");
}

#[test]
fn download_staging_no_checksum_published_still_stages_unverified() {
    run("no-checksum-published-still-stages-unverified");
}

#[test]
fn download_staging_interrupted_download_partial_temp_file_is_safely_retried() {
    run("interrupted-download-partial-temp-file-is-safely-retried");
}
