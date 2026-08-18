//! Exercises `java_runtime_install.rs` against its two
//! `fixtures/java-runtime-selection/adoptium-*` cases: the query (real,
//! live-captured Adoptium response shapes) and the unpack/rollback
//! design (a pinned design, not an observed run -- MSC 1 has nothing to
//! observe here, see that fixture's own notes). Every test builds its
//! own tar.gz/zip archive with the `tar`/`flate2`/`zip` crates already
//! in this crate's dependency graph -- the same "build the real artifact
//! and round-trip it" technique `tools/phase6/phase6-gate-smoke.sh` and
//! P7.14's fake installer jar already use, rather than asserting against
//! a hand-simulated fake extractor.

use flate2::Compression;
use flate2::write::GzEncoder;
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::jar_provider::{JarProviderError, Transport};
use msc_infrastructure::java_runtime_detection::HostOs;
use msc_infrastructure::java_runtime_install::{
    AdoptiumAsset, ArchiveKind, JavaRuntimeInstallError, install_managed_runtime,
    query_adoptium_latest, sha256_hex,
};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

struct Fixture {
    expected: Value,
}

fn load(case: &str) -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/java-runtime-selection")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    let json: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()));
    Fixture {
        expected: json["expected"].clone(),
    }
}

struct FakeTransport {
    responses: Mutex<HashMap<String, Vec<u8>>>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
        }
    }

    fn set(&self, url: &str, bytes: impl Into<Vec<u8>>) {
        self.responses
            .lock()
            .unwrap()
            .insert(url.to_string(), bytes.into());
    }
}

impl Transport for FakeTransport {
    fn get(&self, url: &str, what: &str, max_bytes: u64) -> Result<Vec<u8>, JarProviderError> {
        let bytes = self
            .responses
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or_else(|| {
                JarProviderError::Network(format!("{what}: no fake response registered for {url}"))
            })?;
        if bytes.len() as u64 > max_bytes {
            return Err(JarProviderError::ResponseTooLarge {
                what: what.to_string(),
                max_bytes,
            });
        }
        Ok(bytes)
    }
}

/// Always returns a connection failure -- proves "the download itself
/// never happened" for the checksum-mismatch/transport-failure cases
/// without needing a real network.
struct FailingTransport;

impl Transport for FailingTransport {
    fn get(&self, _url: &str, what: &str, _max_bytes: u64) -> Result<Vec<u8>, JarProviderError> {
        Err(JarProviderError::Network(format!(
            "{what}: simulated connection failure"
        )))
    }
}

fn host_os_from_str(s: &str) -> HostOs {
    match s {
        "mac" => HostOs::Mac,
        "linux" => HostOs::Linux,
        "windows" => HostOs::Windows,
        other => panic!("unknown os {other}"),
    }
}

fn adoptium_query_url(major: u64, os: HostOs, arch: &str) -> String {
    format!(
        "https://api.adoptium.net/v3/assets/latest/{major}/hotspot?os={}&image_type=jdk&vendor=eclipse&architecture={arch}",
        os.adoptium_os_param(),
    )
}

fn adoptium_response_json(name: &str, link: &str, checksum: &str, size: u64) -> String {
    serde_json::json!([
        { "binary": { "package": { "name": name, "link": link, "checksum": checksum, "size": size } } }
    ])
    .to_string()
}

// --- query_adoptium_latest, against the real live-captured shapes ---

#[test]
fn java_runtime_install_adoptium_archive_url_per_os_architecture_with_checksum_and_no_asset_fallback()
 {
    let fixture =
        load("adoptium-archive-url-per-os-architecture-with-checksum-and-no-asset-fallback");
    let results = fixture.expected["results"].as_array().unwrap();

    for result in results {
        let req = &result["request"];
        let major = req["major"].as_u64().unwrap();
        let os = host_os_from_str(req["os"].as_str().unwrap());
        let arch = req["arch"].as_str().unwrap();
        let url = adoptium_query_url(major, os, arch);
        let transport = FakeTransport::new();

        if result["assetName"].is_null() {
            transport.set(&url, b"[]".to_vec());
            let err = query_adoptium_latest(&transport, major as u32, os, arch)
                .expect_err("empty asset array must be refused, not silently fall back");
            assert!(
                matches!(err, JavaRuntimeInstallError::NoAsset { .. }),
                "request {req:?}: expected NoAsset, got {err}"
            );
            continue;
        }

        let asset_name = result["assetName"].as_str().unwrap();
        let link = result["downloadLink"].as_str().unwrap();
        let sha256 = result["sha256"].as_str().unwrap();
        let size = result["sizeBytes"].as_u64().unwrap();
        transport.set(
            &url,
            adoptium_response_json(asset_name, link, sha256, size).into_bytes(),
        );

        let asset = query_adoptium_latest(&transport, major as u32, os, arch)
            .unwrap_or_else(|e| panic!("request {req:?}: query_adoptium_latest failed: {e}"));

        assert_eq!(asset.asset_name, asset_name, "request {req:?}");
        assert_eq!(asset.download_url, link, "request {req:?}");
        assert_eq!(asset.sha256, sha256, "request {req:?}");
        assert_eq!(asset.size_bytes, size, "request {req:?}");
        let expected_kind = match result["archiveKind"].as_str().unwrap() {
            "tar.gz" => ArchiveKind::TarGz,
            "zip" => ArchiveKind::Zip,
            other => panic!("unexpected archiveKind {other}"),
        };
        assert_eq!(asset.archive_kind, expected_kind, "request {req:?}");
    }
}

// --- install_managed_runtime: real tar.gz/zip round trips ---

fn build_tar_gz(top_dir: &str, files: &[(&str, &[u8], bool)]) -> Vec<u8> {
    let mut tar_bytes = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut tar_bytes);
        for (rel, contents, executable) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(if *executable { 0o755 } else { 0o644 });
            header.set_cksum();
            let path = format!("{top_dir}/{rel}");
            builder.append_data(&mut header, &path, *contents).unwrap();
        }
        builder.finish().unwrap();
    }
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&tar_bytes).unwrap();
    encoder.finish().unwrap()
}

fn build_zip(top_dir: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buf);
        let options = zip::write::SimpleFileOptions::default();
        for (rel, contents) in files {
            let path = format!("{top_dir}/{rel}");
            writer.start_file(path, options).unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
    }
    buf.into_inner()
}

#[test]
fn java_runtime_install_successful_mac_install_stages_verifies_extracts_strips_and_cleans_up() {
    let archive = build_tar_gz(
        "jdk-25.0.4+7",
        &[
            ("Contents/Home/bin/java", b"fake-java-binary", true),
            ("Contents/Home/release", b"JAVA_VERSION=\"25.0.4\"", false),
        ],
    );
    let sha256 = sha256_hex(&archive);
    let asset = AdoptiumAsset {
        asset_name: "OpenJDK25U-jdk_aarch64_mac_hotspot_25.0.4_7.tar.gz".to_string(),
        download_url: "https://example.invalid/temurin25-mac-aarch64.tar.gz".to_string(),
        sha256,
        size_bytes: archive.len() as u64,
        archive_kind: ArchiveKind::TarGz,
    };

    let transport = FakeTransport::new();
    transport.set(&asset.download_url, archive);
    let fs = FakeFileSystem::new();
    let runtimes_root = Path::new("/runtimes");

    let dest = install_managed_runtime(
        &fs,
        &transport,
        runtimes_root,
        "temurin-25-mac-aarch64",
        &asset,
    )
    .expect("install_managed_runtime");

    assert_eq!(dest, runtimes_root.join("temurin-25-mac-aarch64"));

    let java_path = dest.join("Contents/Home/bin/java");
    assert_eq!(fs.read(&java_path).unwrap(), b"fake-java-binary");
    assert!(
        fs.stat(&java_path).unwrap().executable,
        "bin/java must come out of extraction executable"
    );

    let release_path = dest.join("Contents/Home/release");
    assert_eq!(fs.read(&release_path).unwrap(), b"JAVA_VERSION=\"25.0.4\"");
    assert!(!fs.stat(&release_path).unwrap().executable);

    // The staged archive is deleted, and no `.extracting` swap directory
    // is left behind, on the success path.
    assert!(
        fs.read(
            &runtimes_root
                .join(".staging")
                .join("temurin-25-mac-aarch64")
        )
        .is_err()
    );
    assert!(
        fs.stat(&runtimes_root.join("temurin-25-mac-aarch64.extracting"))
            .is_err()
    );
}

#[test]
fn java_runtime_install_successful_windows_install_extracts_zip_and_strips_top_level() {
    let archive = build_zip(
        "jdk-21.0.12+8",
        &[
            ("bin/java.exe", b"fake-java-exe"),
            ("release", b"JAVA_VERSION=\"21.0.12\""),
        ],
    );
    let sha256 = sha256_hex(&archive);
    let asset = AdoptiumAsset {
        asset_name: "OpenJDK21U-jdk_x64_windows_hotspot_21.0.12_8.zip".to_string(),
        download_url: "https://example.invalid/temurin21-windows-x64.zip".to_string(),
        sha256,
        size_bytes: archive.len() as u64,
        archive_kind: ArchiveKind::Zip,
    };

    let transport = FakeTransport::new();
    transport.set(&asset.download_url, archive);
    let fs = FakeFileSystem::new();
    let runtimes_root = Path::new("/runtimes");

    let dest = install_managed_runtime(
        &fs,
        &transport,
        runtimes_root,
        "temurin-21-windows-x64",
        &asset,
    )
    .expect("install_managed_runtime");

    assert_eq!(
        fs.read(&dest.join("bin/java.exe")).unwrap(),
        b"fake-java-exe"
    );
    assert_eq!(
        fs.read(&dest.join("release")).unwrap(),
        b"JAVA_VERSION=\"21.0.12\""
    );
    assert!(
        fs.read(
            &runtimes_root
                .join(".staging")
                .join("temurin-21-windows-x64")
        )
        .is_err()
    );
}

#[test]
fn java_runtime_install_checksum_mismatch_writes_nothing_and_cleans_staging() {
    let archive = build_tar_gz("jdk-1", &[("bin/java", b"bytes", true)]);
    let asset = AdoptiumAsset {
        asset_name: "bad.tar.gz".to_string(),
        download_url: "https://example.invalid/bad.tar.gz".to_string(),
        sha256: "0".repeat(64), // deliberately wrong
        size_bytes: archive.len() as u64,
        archive_kind: ArchiveKind::TarGz,
    };

    let transport = FakeTransport::new();
    transport.set(&asset.download_url, archive);
    let fs = FakeFileSystem::new();
    let runtimes_root = Path::new("/runtimes");

    let err = install_managed_runtime(&fs, &transport, runtimes_root, "bad-runtime", &asset)
        .expect_err("checksum mismatch must be refused");
    assert!(matches!(
        err,
        JavaRuntimeInstallError::ChecksumMismatch { .. }
    ));

    // The invariant this fixture pins: the final runtime directory is
    // never written when the checksum doesn't verify, and nothing is
    // left staged either.
    assert!(fs.stat(&runtimes_root.join("bad-runtime")).is_err());
    assert!(
        fs.read(&runtimes_root.join(".staging").join("bad-runtime"))
            .is_err()
    );
}

/// Stands in for the fixture's "interrupted-mid-download" scenario: this
/// module's own doc explains why a real streaming interruption can't
/// leave a partial file in this design (bytes are fully buffered by
/// `Transport::get` before anything touches disk) — a hard transport
/// failure is the strongest version of "the download never completed"
/// this design can produce, and it must leave exactly as much behind as
/// a checksum mismatch does: nothing.
#[test]
fn java_runtime_install_transport_failure_leaves_nothing_on_disk() {
    let asset = AdoptiumAsset {
        asset_name: "unreachable.tar.gz".to_string(),
        download_url: "https://example.invalid/unreachable.tar.gz".to_string(),
        sha256: "irrelevant".to_string(),
        size_bytes: 0,
        archive_kind: ArchiveKind::TarGz,
    };
    let fs = FakeFileSystem::new();
    let runtimes_root = Path::new("/runtimes");

    let err = install_managed_runtime(
        &fs,
        &FailingTransport,
        runtimes_root,
        "unreachable-runtime",
        &asset,
    )
    .expect_err("a transport failure must propagate");
    assert!(matches!(err, JavaRuntimeInstallError::Transport(_)));
    assert!(fs.stat(&runtimes_root.join("unreachable-runtime")).is_err());
    assert!(
        fs.read(&runtimes_root.join(".staging").join("unreachable-runtime"))
            .is_err()
    );
}

#[test]
fn java_runtime_install_stale_staging_leftover_from_a_prior_attempt_is_discarded_not_resumed() {
    let archive = build_tar_gz("jdk-1", &[("bin/java", b"real-bytes", true)]);
    let sha256 = sha256_hex(&archive);
    let asset = AdoptiumAsset {
        asset_name: "ok.tar.gz".to_string(),
        download_url: "https://example.invalid/ok.tar.gz".to_string(),
        sha256,
        size_bytes: archive.len() as u64,
        archive_kind: ArchiveKind::TarGz,
    };

    let transport = FakeTransport::new();
    transport.set(&asset.download_url, archive);
    let runtimes_root = Path::new("/runtimes");
    // A stale, unrelated leftover from a prior interrupted attempt.
    let fs = FakeFileSystem::new().with_file(
        runtimes_root.join(".staging").join("some-runtime"),
        b"orphaned-partial-bytes".to_vec(),
        false,
    );

    let dest = install_managed_runtime(&fs, &transport, runtimes_root, "some-runtime", &asset)
        .expect("install_managed_runtime");

    // The stale file was discarded and replaced by a real, verified
    // install -- never "resumed" from the orphaned bytes.
    assert_eq!(fs.read(&dest.join("bin/java")).unwrap(), b"real-bytes");
}

#[test]
fn java_runtime_install_sha256_hex_matches_fips_180_4_test_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
