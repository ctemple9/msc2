//! Focused P10.10 coverage for verified Bedrock acquisition and promotion.

use msc_application::bedrock_provisioning::{
    BEDROCK_MANIFEST_URL, BedrockProvisioningError, ProvisionOutcome, ProvisionRequest,
    ensure_installed,
};
use msc_infrastructure::bedrock_distribution::{
    BedrockPlatform, BedrockVersionRequest, resolve_release, stage_archive,
};
use msc_infrastructure::download_staging::sha256_hex;
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::jar_provider::{JarProviderError, Transport};
use serde_json::json;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zip::write::SimpleFileOptions;

struct FakeTransport {
    responses: Mutex<HashMap<String, Vec<u8>>>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
        }
    }

    fn with_response(self, url: &str, bytes: Vec<u8>) -> Self {
        self.responses
            .lock()
            .unwrap()
            .insert(url.to_string(), bytes);
        self
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
            .ok_or_else(|| JarProviderError::Network(format!("{what}: no response for {url}")))?;
        if bytes.len() as u64 > max_bytes {
            return Err(JarProviderError::ResponseTooLarge {
                what: what.to_string(),
                max_bytes,
            });
        }
        Ok(bytes)
    }
}

fn archive(version: &str, executable_contents: &[u8], settings: &[u8]) -> Vec<u8> {
    archive_for_platform(
        version,
        executable_contents,
        settings,
        BedrockPlatform::Linux,
    )
}

fn archive_for_platform(
    version: &str,
    executable_contents: &[u8],
    settings: &[u8],
    platform: BedrockPlatform,
) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut bytes);
        let executable_options = SimpleFileOptions::default().unix_permissions(0o755);
        zip.start_file(platform.executable_name(), executable_options)
            .unwrap();
        zip.write_all(executable_contents).unwrap();
        zip.start_file("server.properties", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(settings).unwrap();
        zip.start_file(
            format!("resource_packs/{version}.mcpack"),
            SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"pack").unwrap();
        zip.start_file("worlds/main/level.dat", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"distribution-world").unwrap();
        zip.finish().unwrap();
    }
    bytes.into_inner()
}

#[test]
fn fresh_install_preserves_existing_server_settings_and_worlds() {
    let bytes = archive("1.26.32.2", b"new", b"distribution-settings");
    let server = PathBuf::from("servers/new-world");
    let fs = FakeFileSystem::new()
        .with_file(
            server.join("server.properties"),
            b"user-settings".to_vec(),
            false,
        )
        .with_file(
            server.join("worlds/main/level.dat"),
            b"world".to_vec(),
            false,
        );
    let transport = FakeTransport::new()
        .with_response(
            BEDROCK_MANIFEST_URL,
            manifest(&[(
                "1.26.32",
                "linux",
                "https://cdn/bedrock-server-1.26.32.2.zip",
                &bytes,
            )]),
        )
        .with_response("https://cdn/bedrock-server-1.26.32.2.zip", bytes);

    let outcome = ensure_installed(
        &fs,
        &transport,
        &request(&server, Some("1.26.32.2"), BedrockPlatform::Linux, false),
        || true,
    )
    .unwrap();

    assert_eq!(
        outcome,
        ProvisionOutcome::Installed {
            version: "1.26.32.2".into()
        }
    );
    assert_eq!(fs.read(&server.join("bedrock_server")).unwrap(), b"new");
    assert_eq!(
        fs.read(&server.join("server.properties")).unwrap(),
        b"user-settings"
    );
    assert_eq!(
        fs.read(&server.join("worlds/main/level.dat")).unwrap(),
        b"world"
    );
}

#[test]
fn windows_install_uses_the_windows_executable_name() {
    let bytes = archive_for_platform(
        "1.26.32.2",
        b"new-windows",
        b"settings",
        BedrockPlatform::Windows,
    );
    let server = PathBuf::from("servers/windows");
    let fs = FakeFileSystem::new();
    let transport = FakeTransport::new()
        .with_response(
            BEDROCK_MANIFEST_URL,
            manifest(&[(
                "1.26.32",
                "windows",
                "https://cdn/bedrock-server-1.26.32.2.zip",
                &bytes,
            )]),
        )
        .with_response("https://cdn/bedrock-server-1.26.32.2.zip", bytes);

    ensure_installed(
        &fs,
        &transport,
        &request(&server, Some("1.26.32.2"), BedrockPlatform::Windows, false),
        || true,
    )
    .unwrap();

    assert_eq!(
        fs.read(&server.join("bedrock_server.exe")).unwrap(),
        b"new-windows"
    );
    assert!(fs.read(&server.join("bedrock_server")).is_err());
}

fn manifest(releases: &[(&str, &str, &str, &[u8])]) -> Vec<u8> {
    let mut release = serde_json::Map::new();
    for (key, platform, url, bytes) in releases {
        let entry = release
            .entry((*key).to_string())
            .or_insert_with(|| json!({}));
        entry[platform] = json!({
            "url": url,
            "sha256": sha256_hex(bytes),
        });
    }
    serde_json::to_vec(&json!({ "release": release })).unwrap()
}

fn endstone_registry(version: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "release": {
            "latest": version,
            "versions": [version]
        }
    }))
    .unwrap()
}

fn endstone_metadata(version: &str, linux: &[u8], windows: &[u8]) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "version": version,
        "binary": {
            "linux": {
                "url": format!("https://cdn/bedrock-server-{version}.1.zip"),
                "sha256": sha256_hex(linux)
            },
            "windows": {
                "url": format!("https://cdn/bedrock-server-{version}.1.zip"),
                "sha256": sha256_hex(windows)
            }
        }
    }))
    .unwrap()
}

fn request<'a>(
    server_dir: &'a Path,
    version: Option<&'a str>,
    platform: BedrockPlatform,
    force: bool,
) -> ProvisionRequest<'a> {
    ProvisionRequest {
        server_dir,
        version,
        platform,
        force,
        manifest_url: BEDROCK_MANIFEST_URL,
    }
}

#[test]
fn endstone_metadata_verifies_linux_and_windows_archives() {
    let version = "1.26.45";
    let linux_archive = archive(version, b"linux", b"linux-settings");
    let windows_archive = archive_for_platform(
        version,
        b"windows",
        b"windows-settings",
        BedrockPlatform::Windows,
    );
    let index_url = BEDROCK_MANIFEST_URL;
    let metadata_url = format!(
        "https://raw.githubusercontent.com/EndstoneMC/bedrock-server-data/v2/release/{version}/metadata.json"
    );
    let transport = FakeTransport::new()
        .with_response(index_url, endstone_registry(version))
        .with_response(
            &metadata_url,
            endstone_metadata(version, &linux_archive, &windows_archive),
        )
        .with_response(
            "https://cdn/bedrock-server-1.26.45.1.zip",
            linux_archive.clone(),
        );
    let linux_server = PathBuf::from("servers/endstone-linux");
    let linux = ensure_installed(
        &FakeFileSystem::new(),
        &transport,
        &request(&linux_server, None, BedrockPlatform::Linux, false),
        || true,
    )
    .unwrap();
    assert!(matches!(linux, ProvisionOutcome::Installed { .. }));

    let windows_transport = FakeTransport::new()
        .with_response(index_url, endstone_registry(version))
        .with_response(
            &metadata_url,
            endstone_metadata(version, &linux_archive, &windows_archive),
        )
        .with_response("https://cdn/bedrock-server-1.26.45.1.zip", windows_archive);
    let windows_server = PathBuf::from("servers/endstone-windows");
    let windows = ensure_installed(
        &FakeFileSystem::new(),
        &windows_transport,
        &request(&windows_server, None, BedrockPlatform::Windows, false),
        || true,
    )
    .unwrap();
    assert!(matches!(windows, ProvisionOutcome::Installed { .. }));
}

fn fixture(case: &str) -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/bedrock-provisioning")
        .join(format!("{case}.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

use std::io::Cursor;

#[test]
fn provisioning_fixtures_are_the_complete_scoped_corpus() {
    let directory =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/bedrock-provisioning");
    let mut cases: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    cases.sort();
    assert_eq!(cases.len(), 16);
    assert!(cases.iter().any(|case| case.contains("checksum")));
    assert!(cases.iter().any(|case| case.contains("platform-manifest")));
    assert!(cases.iter().any(|case| case.contains("rolls-back")));
    assert_eq!(
        fixture("version-fetcher-offline-static-fallback")["expected"]["first_version"],
        "LATEST"
    );
}

#[test]
fn resolves_latest_semantically_and_dispatches_the_requested_platform() {
    let linux = archive("1.10.0.2", b"linux", b"linux-settings");
    let windows = archive("1.10.0.2", b"windows", b"windows-settings");
    let body = manifest(&[
        (
            "1.9.99",
            "linux",
            "https://cdn/bedrock-server-1.9.99.1.zip",
            &linux,
        ),
        (
            "1.10.0",
            "windows",
            "https://cdn/bedrock-server-1.10.0.2.zip",
            &windows,
        ),
    ]);
    let release = resolve_release(
        &body,
        BedrockVersionRequest::Latest,
        BedrockPlatform::Windows,
    )
    .unwrap();
    assert_eq!(release.version, "1.10.0.2");
    assert_eq!(release.platform, BedrockPlatform::Windows);
    assert_eq!(release.url, "https://cdn/bedrock-server-1.10.0.2.zip");
    assert_eq!(release.sha256, sha256_hex(&windows));
}

#[test]
fn refuses_unverified_archives_before_staging() {
    let body = serde_json::to_vec(&json!({
        "release": {
            "1.26.32": {
                "linux": {
                    "url": "https://cdn/bedrock-server-1.26.32.2.zip"
                }
            }
        }
    }))
    .unwrap();
    let error = resolve_release(
        &body,
        BedrockVersionRequest::Pinned("1.26.32.2"),
        BedrockPlatform::Linux,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        msc_infrastructure::bedrock_distribution::BedrockDistributionError::UnverifiedArchive
    ));
    assert_eq!(
        fixture("msc2-net-new-unverified-archive-is-not-runnable")["expected"]["staged"],
        false
    );
}

#[test]
fn checksum_mismatch_and_corrupt_zip_leave_staging_empty() {
    let bytes = archive("1.26.32.2", b"server", b"settings");
    let release = resolve_release(
        &manifest(&[(
            "1.26.32",
            "linux",
            "https://cdn/bedrock-server-1.26.32.2.zip",
            &bytes,
        )]),
        BedrockVersionRequest::Latest,
        BedrockPlatform::Linux,
    )
    .unwrap();
    let fs = FakeFileSystem::new().with_dir("servers");
    let bad = msc_infrastructure::bedrock_distribution::BedrockRelease {
        sha256: "0".repeat(64),
        ..release.clone()
    };
    assert!(stage_archive(&fs, Path::new("servers/staging"), &bad, &bytes).is_err());
    assert!(
        fs.read(Path::new("servers/staging/1.26.32.2/bedrock_server"))
            .is_err()
    );

    let corrupt = b"not a zip";
    let corrupt_release = msc_infrastructure::bedrock_distribution::BedrockRelease {
        sha256: sha256_hex(corrupt),
        ..release
    };
    assert!(stage_archive(&fs, Path::new("servers/staging"), &corrupt_release, corrupt).is_err());
    assert!(
        fs.read(Path::new("servers/staging/1.26.32.2/bedrock_server"))
            .is_err()
    );
    assert_eq!(
        fixture("msc2-net-new-corrupt-archive-rejected")["expected"]["runnable"],
        false
    );
}

#[test]
fn installs_and_updates_without_overwriting_user_state() {
    let old_archive = archive("1.26.31.1", b"old", b"new-server-settings");
    let new_archive = archive("1.26.32.2", b"new", b"distribution-settings");
    let manifest = manifest(&[(
        "1.26.32",
        "linux",
        "https://cdn/bedrock-server-1.26.32.2.zip",
        &new_archive,
    )]);
    let server = PathBuf::from("servers/world");
    let fs = FakeFileSystem::new()
        .with_file(server.join("bedrock_server"), b"old".to_vec(), true)
        .with_file(
            server.join(".msc_bds_version"),
            b"1.26.31.1".to_vec(),
            false,
        )
        .with_file(
            server.join("server.properties"),
            b"user-settings".to_vec(),
            false,
        )
        .with_file(
            server.join("allowlist.json"),
            b"user-allowlist".to_vec(),
            false,
        )
        .with_file(
            server.join("worlds/main/level.dat"),
            b"world".to_vec(),
            false,
        );
    let transport = FakeTransport::new()
        .with_response(BEDROCK_MANIFEST_URL, manifest)
        .with_response("https://cdn/bedrock-server-1.26.32.2.zip", new_archive);
    let outcome = ensure_installed(
        &fs,
        &transport,
        &request(&server, Some("1.26.32.2"), BedrockPlatform::Linux, false),
        || true,
    )
    .unwrap();
    assert_eq!(
        outcome,
        ProvisionOutcome::Updated {
            from: Some("1.26.31.1".into()),
            to: "1.26.32.2".into()
        }
    );
    assert_eq!(fs.read(&server.join("bedrock_server")).unwrap(), b"new");
    assert_eq!(
        fs.read(&server.join("server.properties")).unwrap(),
        b"user-settings"
    );
    assert_eq!(
        fs.read(&server.join("allowlist.json")).unwrap(),
        b"user-allowlist"
    );
    assert_eq!(
        fs.read(&server.join("worlds/main/level.dat")).unwrap(),
        b"world"
    );
    assert_eq!(
        fs.read(&server.join(".msc_bds_version")).unwrap(),
        b"1.26.32.2"
    );
    assert_eq!(
        fixture("update-excludes-user-state-files")["expected"]["worlds_touched"],
        false
    );
    let _ = old_archive;
}

#[test]
fn downgrade_requires_a_successful_safety_backup() {
    let archive = archive("1.20.0.0", b"old-target", b"settings");
    let server = PathBuf::from("servers/world");
    let fs = FakeFileSystem::new()
        .with_file(server.join("bedrock_server"), b"current".to_vec(), true)
        .with_file(
            server.join(".msc_bds_version"),
            b"1.26.32.2".to_vec(),
            false,
        );
    let transport = FakeTransport::new()
        .with_response(
            BEDROCK_MANIFEST_URL,
            manifest(&[(
                "1.20.0",
                "linux",
                "https://cdn/bedrock-server-1.20.0.0.zip",
                &archive,
            )]),
        )
        .with_response("https://cdn/bedrock-server-1.20.0.0.zip", archive);
    let error = ensure_installed(
        &fs,
        &transport,
        &request(&server, Some("1.20.0.0"), BedrockPlatform::Linux, false),
        || false,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        BedrockProvisioningError::DowngradeBackupFailed { .. }
    ));
    assert_eq!(fs.read(&server.join("bedrock_server")).unwrap(), b"current");
    assert_eq!(
        fixture("msc2-net-new-failed-update-rolls-back-atomically")["expected"]["old_installation_intact"],
        true
    );
}

#[test]
fn failed_promotion_restores_the_previous_installation() {
    let bytes = archive("1.26.32.2", b"new", b"settings");
    let server = PathBuf::from("servers/world");
    let fs = FakeFileSystem::new()
        .with_file(server.join("bedrock_server"), b"old".to_vec(), true)
        .with_file(
            server.join(".msc_bds_version"),
            b"1.26.31.1".to_vec(),
            false,
        )
        .with_failing_rename(&server);
    let transport = FakeTransport::new()
        .with_response(
            BEDROCK_MANIFEST_URL,
            manifest(&[(
                "1.26.32",
                "linux",
                "https://cdn/bedrock-server-1.26.32.2.zip",
                &bytes,
            )]),
        )
        .with_response("https://cdn/bedrock-server-1.26.32.2.zip", bytes);
    let result = ensure_installed(
        &fs,
        &transport,
        &request(&server, Some("1.26.32.2"), BedrockPlatform::Linux, false),
        || true,
    );
    assert!(matches!(
        result,
        Err(BedrockProvisioningError::Filesystem(_))
    ));
    assert_eq!(fs.read(&server.join("bedrock_server")).unwrap(), b"old");
    assert_eq!(
        fs.read(&server.join(".msc_bds_version")).unwrap(),
        b"1.26.31.1"
    );
}

#[test]
fn offline_nonforced_start_uses_existing_files() {
    let server = PathBuf::from("servers/world");
    let fs = FakeFileSystem::new()
        .with_file(server.join("bedrock_server"), b"current".to_vec(), true)
        .with_file(
            server.join(".msc_bds_version"),
            b"1.26.31.1".to_vec(),
            false,
        );
    let transport = FakeTransport::new();
    let outcome = ensure_installed(
        &fs,
        &transport,
        &request(&server, None, BedrockPlatform::Linux, false),
        || panic!("offline fallback must not request a backup"),
    )
    .unwrap();
    assert_eq!(
        outcome,
        ProvisionOutcome::UsedInstalledFiles {
            version: Some("1.26.31.1".into())
        }
    );
}
