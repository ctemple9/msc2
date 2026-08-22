use msc_infrastructure::download_staging::sha256_hex;
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::helper_acquisition::{
    AcquiredHelper, HelperAcquisitionError, HelperPlatform, PinnedHelperAsset, PinnedHelperRelease,
    acquire_pinned_helper, metadata_path_for,
};
use msc_infrastructure::jar_provider::{JarProviderError, Transport};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const RELEASE_URL: &str =
    "https://api.github.com/repos/example/playitd/releases/tags/playitd-v1.0.10";
const ASSET_URL: &str =
    "https://github.com/example/playitd/releases/download/playitd-v1.0.10/playitd-linux-x86_64";
const ASSET_BYTES: &[u8] = b"pinned playitd bytes";
const ASSET_SHA256: &str = "6d220b9914cafaccc949e466ec5935dea79ef413d0655cf14bd24baba58805f2";
const ARTIFACT: &str = "/cache/playitd/playitd-v1.0.10/playitd-linux-x86_64";

struct FakeTransport {
    responses: Mutex<HashMap<String, Vec<u8>>>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
        }
    }

    fn with_bytes(self, url: &str, bytes: impl Into<Vec<u8>>) -> Self {
        self.responses
            .lock()
            .unwrap()
            .insert(url.to_string(), bytes.into());
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
            .ok_or_else(|| {
                JarProviderError::Network(format!("{what}: no fake response for {url}"))
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

fn release_metadata() -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/networking/helper-acquisition-pinned-release.json");
    std::fs::read(path).expect("helper acquisition fixture")
}

fn pin(sha256: &str) -> PinnedHelperRelease {
    PinnedHelperRelease {
        helper: "playitd".into(),
        version: "playitd-v1.0.10".into(),
        release_metadata_url: RELEASE_URL.into(),
        assets: vec![
            PinnedHelperAsset {
                platform: HelperPlatform::LinuxX86_64,
                asset_name: "playitd-linux-x86_64".into(),
                sha256: sha256.into(),
            },
            PinnedHelperAsset {
                platform: HelperPlatform::WindowsX86_64,
                asset_name: "playitd-windows-x86_64.exe".into(),
                sha256: sha256.into(),
            },
        ],
    }
}

fn transport() -> FakeTransport {
    FakeTransport::new()
        .with_bytes(RELEASE_URL, release_metadata())
        .with_bytes(ASSET_URL, ASSET_BYTES)
}

#[test]
fn pinned_helper_resolves_exact_asset_verifies_sha256_and_records_metadata() {
    assert_eq!(sha256_hex(ASSET_BYTES), ASSET_SHA256);
    let transport = transport();
    let fs = FakeFileSystem::new().with_dir("/cache");

    let acquired = acquire_pinned_helper(
        &transport,
        &fs,
        Path::new("/cache"),
        &pin(ASSET_SHA256),
        HelperPlatform::LinuxX86_64,
    )
    .expect("pinned helper should acquire");

    assert_eq!(acquired.artifact.path, PathBuf::from(ARTIFACT));
    assert_eq!(fs.read(Path::new(ARTIFACT)).unwrap(), ASSET_BYTES);
    assert!(fs.stat(Path::new(ARTIFACT)).unwrap().executable);
    assert_eq!(
        acquired.metadata_path,
        metadata_path_for(Path::new(ARTIFACT))
    );
    let metadata: Value =
        serde_json::from_slice(&fs.read(&acquired.metadata_path).unwrap()).unwrap();
    assert_eq!(metadata["version"], "playitd-v1.0.10");
    assert_eq!(metadata["assetName"], "playitd-linux-x86_64");
    assert_eq!(metadata["sha256"], ASSET_SHA256);
}

#[test]
fn pinned_helper_rejects_latest_and_release_or_asset_identity_drift() {
    let fs = FakeFileSystem::new().with_dir("/cache");
    let latest = PinnedHelperRelease {
        release_metadata_url: "https://api.github.com/repos/example/playitd/releases/latest".into(),
        ..pin(ASSET_SHA256)
    };
    assert!(matches!(
        acquire_pinned_helper(&transport(), &fs, Path::new("/cache"), &latest, HelperPlatform::LinuxX86_64),
        Err(HelperAcquisitionError::ReleaseResolution(message)) if message.contains("latest")
    ));

    let wrong_version = PinnedHelperRelease {
        version: "playitd-v1.0.09".into(),
        ..pin(ASSET_SHA256)
    };
    assert!(matches!(
        acquire_pinned_helper(&transport(), &fs, Path::new("/cache"), &wrong_version, HelperPlatform::LinuxX86_64),
        Err(HelperAcquisitionError::ReleaseResolution(message)) if message.contains("resolved to release")
    ));

    let wrong_asset = PinnedHelperRelease {
        assets: vec![PinnedHelperAsset {
            platform: HelperPlatform::LinuxX86_64,
            asset_name: "playitd-not-present".into(),
            sha256: ASSET_SHA256.into(),
        }],
        ..pin(ASSET_SHA256)
    };
    assert!(matches!(
        acquire_pinned_helper(&transport(), &fs, Path::new("/cache"), &wrong_asset, HelperPlatform::LinuxX86_64),
        Err(HelperAcquisitionError::ReleaseResolution(message)) if message.contains("no exact asset")
    ));
}

#[test]
fn checksum_failure_keeps_previous_artifact_in_place() {
    let transport = transport();
    let fs = FakeFileSystem::new()
        .with_file(ARTIFACT, b"previous working bytes".to_vec(), true)
        .with_file(
            metadata_path_for(Path::new(ARTIFACT)),
            b"previous metadata".to_vec(),
            false,
        );
    let result = acquire_pinned_helper(
        &transport,
        &fs,
        Path::new("/cache"),
        &pin(&"0".repeat(64)),
        HelperPlatform::LinuxX86_64,
    );
    assert!(matches!(
        result,
        Err(HelperAcquisitionError::Checksum { .. })
    ));
    assert_eq!(
        fs.read(Path::new(ARTIFACT)).unwrap(),
        b"previous working bytes"
    );
    assert_eq!(
        fs.read(&metadata_path_for(Path::new(ARTIFACT))).unwrap(),
        b"previous metadata"
    );
}

#[test]
fn promotion_failure_keeps_previous_artifact_and_cleans_staging() {
    let transport = transport();
    let fs = FakeFileSystem::new()
        .with_file(ARTIFACT, b"previous working bytes".to_vec(), true)
        .with_failing_rename(ARTIFACT);
    let result: Result<AcquiredHelper, HelperAcquisitionError> = acquire_pinned_helper(
        &transport,
        &fs,
        Path::new("/cache"),
        &pin(ASSET_SHA256),
        HelperPlatform::LinuxX86_64,
    );
    assert!(
        matches!(result, Err(HelperAcquisitionError::Filesystem(message)) if message.contains("promote helper artifact"))
    );
    assert_eq!(
        fs.read(Path::new(ARTIFACT)).unwrap(),
        b"previous working bytes"
    );
    assert!(
        fs.read(Path::new(
            "/cache/playitd/playitd-v1.0.10/.playitd-linux-x86_64.staged"
        ))
        .is_err()
    );
}

#[test]
fn metadata_promotion_failure_restores_previous_artifact_pair() {
    let transport = transport();
    let metadata_path = metadata_path_for(Path::new(ARTIFACT));
    let fs = FakeFileSystem::new()
        .with_file(ARTIFACT, b"previous working bytes".to_vec(), true)
        .with_file(&metadata_path, b"previous metadata".to_vec(), false)
        .with_failing_rename(metadata_path.clone());
    let result = acquire_pinned_helper(
        &transport,
        &fs,
        Path::new("/cache"),
        &pin(ASSET_SHA256),
        HelperPlatform::LinuxX86_64,
    );
    assert!(matches!(
        result,
        Err(HelperAcquisitionError::Filesystem(message))
            if message.contains("promote helper metadata")
    ));
    assert_eq!(
        fs.read(Path::new(ARTIFACT)).unwrap(),
        b"previous working bytes"
    );
    assert!(fs.stat(Path::new(ARTIFACT)).unwrap().executable);
    assert_eq!(fs.read(&metadata_path).unwrap(), b"previous metadata");
}
