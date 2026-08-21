//! Port of `fixtures/download-staging/`: one test per case, each loading
//! its fixture, building a `FakeFileSystem` from its `existingFiles`, and
//! exercising `stage_download`. No MSC 1 test file exercises a
//! checksum-verified download (see each fixture's own `notes`), so these
//! fixtures were characterized directly from `msc2-engineering.md` §7 and
//! the three real download workflows it names.

use msc_infrastructure::download_staging::{
    ChecksumAlgorithm, ExpectedChecksum, md5_hex, sha1_hex, sha256_hex, sha512_hex, stage_download,
};
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

/// P7.35 amendment: `expectedSha1Hex` (a bare string or JSON null) became
/// `expectedChecksum` (an `{algorithm, hex}` object or JSON null) — the
/// same shape [`stage_download`]'s own signature now takes.
fn parse_expected_checksum(value: &Value) -> Option<ExpectedChecksum> {
    if value.is_null() {
        return None;
    }
    let algorithm = match value["algorithm"]
        .as_str()
        .expect("expectedChecksum.algorithm")
    {
        "sha1" => ChecksumAlgorithm::Sha1,
        "sha256" => ChecksumAlgorithm::Sha256,
        "md5" => ChecksumAlgorithm::Md5,
        other => panic!("unknown expectedChecksum.algorithm {other:?}"),
    };
    let hex = value["hex"]
        .as_str()
        .expect("expectedChecksum.hex")
        .to_string();
    Some(ExpectedChecksum { algorithm, hex })
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
    let expected_checksum = parse_expected_checksum(&fixture.input["expectedChecksum"]);

    let result = stage_download(
        &fs,
        &dest,
        bytes,
        origin_url,
        version,
        expected_checksum.as_ref(),
    );

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

// --- P7.35: the algorithm-aware checksum contract, proven against every
// algorithm a real Phase 7 provider publishes (Mojang SHA-1 already
// covered above; Paper SHA-256 and Purpur MD5 below), not just SHA-1. ---

#[test]
fn download_staging_successful_stage_and_move_with_matching_sha256_checksum() {
    run("successful-stage-and-move-with-matching-sha256-checksum");

    // Same reasoning as the SHA-1 test's own folded-in assertions: these
    // are the published FIPS 180-4 test vectors, checked directly against
    // sha256_hex here rather than as a separate #[test].
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn download_staging_sha256_checksum_mismatch_rejected_without_touching_destination() {
    run("sha256-checksum-mismatch-rejected-without-touching-destination");
}

#[test]
fn download_staging_successful_stage_and_move_with_matching_md5_checksum() {
    run("successful-stage-and-move-with-matching-md5-checksum");

    // Same reasoning again — the published RFC 1321 test vectors, checked
    // directly against md5_hex here rather than as a separate #[test].
    assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
    assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
}

#[test]
fn download_staging_md5_checksum_mismatch_rejected_without_touching_destination() {
    run("md5-checksum-mismatch-rejected-without-touching-destination");
}

// --- P8.16 amendment: SHA-512, the algorithm Modrinth's exact-hash-identity
// endpoints key on, added to this same shared primitive rather than a
// second one. No fixture directory covers this (it postdates P3.14/P7.35's
// own fixture domains) — proven directly against the published FIPS 180-4
// test vectors, the same "no fixture, no fabricated one either" approach
// `config_lifecycle.rs`/`audit_log.rs` already established.

#[test]
fn download_staging_sha512_hex_matches_fips_180_4_test_vectors() {
    assert_eq!(
        sha512_hex(b""),
        "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
    );
    assert_eq!(
        sha512_hex(b"abc"),
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
    );
    // A 135-byte input spans two 128-byte SHA-512 blocks (the "" / "abc"
    // vectors above are both single-block), exercising the padding/carry
    // path across a block boundary. Expected digest computed independently
    // via Python's hashlib, the same "computed out-of-band" precedent this
    // file's own `run` fixtures already use for their expected checksums.
    let two_block_input = "The quick brown fox jumps over the lazy dog. ".repeat(3);
    assert_eq!(two_block_input.len(), 135);
    assert_eq!(
        sha512_hex(two_block_input.as_bytes()),
        "f5380dd66d7be3f28171b90a7f7b200e9f12fe8e8c3ac4c9952d5d97e9d3f4b00c50d6002a4e1d2ae9520af64a91e50b4355d20eb2473455ee2fd7051f380749"
    );
}

#[test]
fn download_staging_sha512_checksum_verified_before_write() {
    let fs = FakeFileSystem::new();
    fs.create_dir_all(Path::new("/server/mods")).unwrap();
    let bytes = b"pack override contents";
    let checksum = ExpectedChecksum::sha512(sha512_hex(bytes));
    let dest = PathBuf::from("/server/mods/override.jar");

    let result = stage_download(
        &fs,
        &dest,
        bytes,
        "https://example.invalid/override.jar",
        "1.0.0",
        Some(&checksum),
    );
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(fs.read(&dest).unwrap(), bytes);

    let wrong = ExpectedChecksum::sha512("0".repeat(128));
    let mismatch = stage_download(
        &fs,
        &dest,
        b"different bytes",
        "https://example.invalid/override.jar",
        "1.0.1",
        Some(&wrong),
    );
    assert!(mismatch.is_err());
    // The mismatch must not have overwritten the already-staged bytes.
    assert_eq!(fs.read(&dest).unwrap(), bytes);
}
