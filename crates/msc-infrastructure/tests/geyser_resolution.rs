use msc_infrastructure::download_staging::sha256_hex;
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::geyser::{
    GEYSER_API_BASE_URL, GeyserProject, acquire_latest, latest_build_url, resolve_latest_build,
};
use msc_infrastructure::helper_acquisition::{ChecksumSource, HelperPlatform};
use msc_infrastructure::jar_provider::{JarProviderError, Transport};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

const GEYSER_BYTES: &[u8] = b"verified geyser jar";
const FLOODGATE_BYTES: &[u8] = b"verified floodgate jar";

struct FakeTransport {
    responses: Mutex<HashMap<String, Vec<u8>>>,
    requested: Mutex<Vec<String>>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
            requested: Mutex::new(Vec::new()),
        }
    }

    fn with_response(self, url: String, body: Vec<u8>) -> Self {
        self.responses.lock().unwrap().insert(url, body);
        self
    }

    fn requests(&self) -> Vec<String> {
        self.requested.lock().unwrap().clone()
    }
}

impl Transport for FakeTransport {
    fn get(&self, url: &str, what: &str, max_bytes: u64) -> Result<Vec<u8>, JarProviderError> {
        self.requested.lock().unwrap().push(url.to_string());
        let bytes = self
            .responses
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or_else(|| JarProviderError::Network(format!("{what}: no response for {url}")))?;
        if bytes.len() as u64 > max_bytes {
            return Err(JarProviderError::ResponseTooLarge {
                what: what.into(),
                max_bytes,
            });
        }
        Ok(bytes)
    }
}

fn metadata(version: &str, build: u64, sha256: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "version": version,
        "build": build,
        "downloads": { "spigot": { "sha256": sha256 } }
    }))
    .unwrap()
}

fn asset_url(project: GeyserProject, version: &str, build: u64) -> String {
    format!(
        "{GEYSER_API_BASE_URL}/projects/{}/versions/{version}/builds/{build}/downloads/spigot",
        project.api_name()
    )
}

#[test]
fn resolves_latest_spigot_build_and_builds_the_concrete_download_url() {
    let project = GeyserProject::Geyser;
    let url = latest_build_url(project);
    let transport =
        FakeTransport::new().with_response(url, metadata("2.11.2", 42, &sha256_hex(GEYSER_BYTES)));

    let build = resolve_latest_build(&transport, project).unwrap();

    assert_eq!(build.display_version(), "2.11.2 (build 42)");
    assert_eq!(build.download_url(), asset_url(project, "2.11.2", 42));
    assert_eq!(transport.requests(), vec![latest_build_url(project)]);
}

#[test]
fn acquires_geyser_and_floodgate_through_the_verified_shared_boundary() {
    let geyser = GeyserProject::Geyser;
    let floodgate = GeyserProject::Floodgate;
    let geyser_url = asset_url(geyser, "2.11.2", 42);
    let floodgate_url = asset_url(floodgate, "2.11.2", 9);
    let transport = FakeTransport::new()
        .with_response(
            latest_build_url(geyser),
            metadata("2.11.2", 42, &sha256_hex(GEYSER_BYTES)),
        )
        .with_response(geyser_url.clone(), GEYSER_BYTES.to_vec())
        .with_response(
            latest_build_url(floodgate),
            metadata("2.11.2", 9, &sha256_hex(FLOODGATE_BYTES)),
        )
        .with_response(floodgate_url.clone(), FLOODGATE_BYTES.to_vec());
    let fs = FakeFileSystem::new().with_dir("/cache");

    let (geyser_build, geyser_artifact) = acquire_latest(
        &transport,
        &fs,
        Path::new("/cache"),
        geyser,
        HelperPlatform::LinuxX86_64,
    )
    .unwrap();
    let (floodgate_build, floodgate_artifact) = acquire_latest(
        &transport,
        &fs,
        Path::new("/cache"),
        floodgate,
        HelperPlatform::LinuxX86_64,
    )
    .unwrap();

    assert_eq!(geyser_build.display_version(), "2.11.2 (build 42)");
    assert_eq!(floodgate_build.display_version(), "2.11.2 (build 9)");
    assert_eq!(
        geyser_artifact.metadata.checksum_source,
        ChecksumSource::UpstreamPublished
    );
    assert_eq!(
        floodgate_artifact.metadata.checksum_source,
        ChecksumSource::UpstreamPublished
    );
    assert_eq!(geyser_artifact.metadata.asset_url, geyser_url);
    assert_eq!(floodgate_artifact.metadata.asset_url, floodgate_url);
    assert_eq!(
        fs.read(&geyser_artifact.artifact.path).unwrap(),
        GEYSER_BYTES
    );
    assert_eq!(
        fs.read(&floodgate_artifact.artifact.path).unwrap(),
        FLOODGATE_BYTES
    );
}

#[test]
fn missing_upstream_sha256_is_a_resolution_failure_before_download() {
    let project = GeyserProject::Geyser;
    let metadata_url = latest_build_url(project);
    let transport = FakeTransport::new().with_response(
        metadata_url,
        serde_json::to_vec(&json!({
            "version": "2.11.2",
            "build": 42,
            "downloads": { "spigot": {} }
        }))
        .unwrap(),
    );

    let error = resolve_latest_build(&transport, project).unwrap_err();

    assert!(error.to_string().contains("no upstream sha256"));
    assert_eq!(transport.requests().len(), 1);
}
