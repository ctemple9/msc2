//! Checksum-verified acquisition for managed helper binaries.
//!
//! A helper is executable code that the agent downloads and later launches,
//! so it has a stricter contract than an ordinary provider download: the
//! release and asset identity are exact, and the bytes must match a
//! caller-supplied SHA-256 from either a repository pin or the upstream
//! provider. The acquisition boundary never resolves an unbounded `latest`
//! alias itself. The caller supplies a [`Transport`] so tests can exercise
//! every branch without contacting a public provider.

use crate::atomic_write::{AtomicWriteError, atomic_write};
use crate::download_staging::{self, CachedFile, DownloadStagingError, ExpectedChecksum};
use crate::fs::FileSystem;
use crate::jar_provider::{JarProviderError, Transport};
use crate::process::ProcessError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// The largest helper artifact the acquisition boundary will hold in memory.
/// Managed helpers are small binaries or JARs; this cap also prevents a bad
/// provider response from turning a helper operation into an unbounded read.
pub const HELPER_ASSET_MAX_BYTES: u64 = 300 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HelperPlatform {
    MacosX86_64,
    MacosAarch64,
    LinuxX86_64,
    LinuxAarch64,
    WindowsX86_64,
    WindowsAarch64,
}

impl HelperPlatform {
    /// Selects the platform identity compiled into the running agent. An
    /// unsupported target fails before any release request is attempted.
    pub fn current() -> Result<Self, HelperAcquisitionError> {
        let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
            ("macos", "x86_64") => Self::MacosX86_64,
            ("macos", "aarch64") => Self::MacosAarch64,
            ("linux", "x86_64") => Self::LinuxX86_64,
            ("linux", "aarch64") => Self::LinuxAarch64,
            ("windows", "x86_64") => Self::WindowsX86_64,
            ("windows", "aarch64") => Self::WindowsAarch64,
            (os, arch) => {
                return Err(HelperAcquisitionError::ReleaseResolution(format!(
                    "unsupported helper platform {os}/{arch}"
                )));
            }
        };
        Ok(platform)
    }
}

impl fmt::Display for HelperPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::MacosX86_64 => "macos-x86_64",
            Self::MacosAarch64 => "macos-aarch64",
            Self::LinuxX86_64 => "linux-x86_64",
            Self::LinuxAarch64 => "linux-aarch64",
            Self::WindowsX86_64 => "windows-x86_64",
            Self::WindowsAarch64 => "windows-aarch64",
        };
        write!(f, "{value}")
    }
}

/// An exact helper release and its platform-specific asset checksums.
///
/// The metadata URL and version must identify the same release. The expected
/// digest is caller-supplied: it may be a repository pin or a checksum
/// published by the upstream provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedHelperRelease {
    pub helper: String,
    pub version: String,
    pub release_metadata_url: String,
    pub assets: Vec<PinnedHelperAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedHelperAsset {
    pub platform: HelperPlatform,
    pub asset_name: String,
    pub sha256: String,
}

/// A provider-resolved release with one exact asset.
///
/// This uses the same stage, verify, and promote path as a repository pin,
/// while allowing providers such as Geyser to supply their own concrete
/// version/build identity and published checksum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHelperRelease {
    pub helper: String,
    pub version: String,
    pub platform: HelperPlatform,
    pub release_metadata_url: String,
    pub asset_name: String,
    pub asset_url: String,
    pub sha256: String,
    pub checksum_source: ChecksumSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChecksumSource {
    RepositoryPinned,
    UpstreamPublished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperArtifactMetadata {
    pub helper: String,
    pub version: String,
    pub platform: HelperPlatform,
    pub release_metadata_url: String,
    pub asset_name: String,
    pub asset_url: String,
    pub sha256: String,
    pub checksum_source: ChecksumSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquiredHelper {
    pub artifact: CachedFile,
    pub metadata_path: PathBuf,
    pub metadata: HelperArtifactMetadata,
}

#[derive(Debug)]
pub enum HelperAcquisitionError {
    ReleaseResolution(String),
    Download(JarProviderError),
    Checksum {
        expected: String,
        actual: String,
    },
    Staging(String),
    Permission(String),
    Filesystem(String),
    /// Included so the helper lifecycle can preserve spawn failures at this
    /// same operation boundary when acquisition and launch are composed.
    Spawn(String),
}

impl fmt::Display for HelperAcquisitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReleaseResolution(message) => {
                write!(f, "helper release resolution failed: {message}")
            }
            Self::Download(error) => write!(f, "helper download failed: {error}"),
            Self::Checksum { expected, actual } => {
                write!(
                    f,
                    "helper sha256 checksum mismatch: expected {expected}, got {actual}"
                )
            }
            Self::Staging(message) => write!(f, "helper staging failed: {message}"),
            Self::Permission(message) => write!(f, "helper permission failed: {message}"),
            Self::Filesystem(message) => write!(f, "helper filesystem failed: {message}"),
            Self::Spawn(message) => write!(f, "helper spawn failed: {message}"),
        }
    }
}

impl std::error::Error for HelperAcquisitionError {}

impl From<ProcessError> for HelperAcquisitionError {
    fn from(error: ProcessError) -> Self {
        Self::Spawn(error.to_string())
    }
}

#[derive(Debug, Deserialize)]
struct ReleaseMetadata {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

/// Acquires one pinned helper asset for `platform`.
///
/// The downloaded bytes first land at a versioned staging path through
/// [`stage_download`]. They are then marked executable and accompanied by a
/// metadata sidecar before either is promoted to the final versioned cache
/// path. A prior version therefore remains available throughout download,
/// checksum verification, staging, and metadata serialization.
pub fn acquire_pinned_helper(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    cache_directory: &Path,
    pin: &PinnedHelperRelease,
    platform: HelperPlatform,
) -> Result<AcquiredHelper, HelperAcquisitionError> {
    validate_pin(pin, platform)?;

    let metadata_bytes = transport
        .get(
            &pin.release_metadata_url,
            "pinned helper release metadata",
            2 * 1024 * 1024,
        )
        .map_err(HelperAcquisitionError::Download)?;
    let release: ReleaseMetadata = serde_json::from_slice(&metadata_bytes).map_err(|error| {
        HelperAcquisitionError::ReleaseResolution(format!("invalid release metadata: {error}"))
    })?;
    if release.tag_name != pin.version {
        return Err(HelperAcquisitionError::ReleaseResolution(format!(
            "pinned version {} resolved to release {}",
            pin.version, release.tag_name
        )));
    }

    let pinned_asset = pin
        .assets
        .iter()
        .find(|asset| asset.platform == platform)
        .expect("validate_pin guarantees a platform asset");
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == pinned_asset.asset_name)
        .ok_or_else(|| {
            HelperAcquisitionError::ReleaseResolution(format!(
                "release {} has no exact asset named {}",
                pin.version, pinned_asset.asset_name
            ))
        })?;
    if asset.browser_download_url.is_empty() {
        return Err(HelperAcquisitionError::ReleaseResolution(format!(
            "asset {} has no download URL",
            pinned_asset.asset_name
        )));
    }

    acquire_resolved_helper(
        transport,
        fs,
        cache_directory,
        &ResolvedHelperRelease {
            helper: pin.helper.clone(),
            version: pin.version.clone(),
            platform,
            release_metadata_url: pin.release_metadata_url.clone(),
            asset_name: pinned_asset.asset_name.clone(),
            asset_url: asset.browser_download_url.clone(),
            sha256: pinned_asset.sha256.clone(),
            checksum_source: ChecksumSource::RepositoryPinned,
        },
    )
}

/// Acquires a provider-resolved helper through the same verified staging
/// boundary as [`acquire_pinned_helper`]. The provider must already have
/// resolved the release to an exact version, asset URL, and SHA-256; this
/// function does not chase a mutable alias or perform another metadata lookup.
pub fn acquire_resolved_helper(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    cache_directory: &Path,
    release: &ResolvedHelperRelease,
) -> Result<AcquiredHelper, HelperAcquisitionError> {
    validate_resolved_release(release)?;

    let helper_directory = cache_directory.join(&release.helper).join(&release.version);
    fs.create_dir_all(&helper_directory)
        .map_err(|error| map_filesystem_error("create helper cache", error))?;
    let artifact_path = helper_directory.join(&release.asset_name);
    let metadata_path = metadata_path_for(&artifact_path);
    let staged_artifact = helper_directory.join(format!(".{}.staged", release.asset_name));
    let staged_metadata =
        helper_directory.join(format!(".{}.metadata.json.staged", release.asset_name));
    let previous_artifact = fs.read(&artifact_path).ok();
    let previous_artifact_executable = fs
        .stat(&artifact_path)
        .map(|metadata| metadata.executable)
        .unwrap_or(false);
    let previous_metadata = fs.read(&metadata_path).ok();

    let bytes = transport
        .get(&release.asset_url, "helper asset", HELPER_ASSET_MAX_BYTES)
        .map_err(HelperAcquisitionError::Download)?;
    let checksum = ExpectedChecksum::sha256(release.sha256.clone());
    if let Err(error) = download_staging::stage_download(
        fs,
        &staged_artifact,
        &bytes,
        &release.asset_url,
        &release.version,
        Some(&checksum),
    ) {
        cleanup(fs, &staged_artifact);
        return Err(map_staging_error(error));
    }
    if let Err(error) = fs.write_executable(&staged_artifact, &bytes) {
        cleanup(fs, &staged_artifact);
        return Err(map_filesystem_error("mark helper executable", error));
    }

    let metadata = HelperArtifactMetadata {
        helper: release.helper.clone(),
        version: release.version.clone(),
        platform: release.platform,
        release_metadata_url: release.release_metadata_url.clone(),
        asset_name: release.asset_name.clone(),
        asset_url: release.asset_url.clone(),
        sha256: release.sha256.clone(),
        checksum_source: release.checksum_source,
    };
    let metadata_json = serde_json::to_vec_pretty(&metadata).map_err(|error| {
        cleanup(fs, &staged_artifact);
        HelperAcquisitionError::Staging(format!("serialize helper metadata: {error}"))
    })?;
    if let Err(error) = atomic_write(fs, &staged_metadata, &metadata_json) {
        cleanup(fs, &staged_artifact);
        cleanup(fs, &staged_metadata);
        return Err(map_atomic_write_error(error));
    }

    if let Err(error) = fs.rename(&staged_artifact, &artifact_path) {
        cleanup(fs, &staged_artifact);
        cleanup(fs, &staged_metadata);
        return Err(map_filesystem_error("promote helper artifact", error));
    }
    if let Err(error) = fs.rename(&staged_metadata, &metadata_path) {
        // The artifact was already fully verified and promoted. Restore the
        // prior pair rather than leaving an artifact without the provenance
        // sidecar that makes it trustworthy to later callers.
        restore_file(
            fs,
            &artifact_path,
            previous_artifact.as_deref(),
            previous_artifact_executable,
        );
        restore_file(fs, &metadata_path, previous_metadata.as_deref(), false);
        cleanup(fs, &staged_metadata);
        return Err(map_filesystem_error("promote helper metadata", error));
    }

    Ok(AcquiredHelper {
        artifact: CachedFile {
            path: artifact_path,
            origin_url: release.asset_url.clone(),
            version: release.version.clone(),
        },
        metadata_path,
        metadata,
    })
}

pub fn metadata_path_for(artifact_path: &Path) -> PathBuf {
    let file_name = artifact_path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    artifact_path.with_file_name(format!("{file_name}.metadata.json"))
}

fn validate_pin(
    pin: &PinnedHelperRelease,
    platform: HelperPlatform,
) -> Result<(), HelperAcquisitionError> {
    if pin.helper.is_empty() || !safe_component(&pin.helper) {
        return Err(HelperAcquisitionError::ReleaseResolution(
            "helper name must be one path component".into(),
        ));
    }
    if pin.version.is_empty() || !safe_component(&pin.version) {
        return Err(HelperAcquisitionError::ReleaseResolution(
            "pinned version must be one path component".into(),
        ));
    }
    if contains_latest_alias(&pin.release_metadata_url) {
        return Err(HelperAcquisitionError::ReleaseResolution(
            "release metadata URL must identify a pinned release, not latest".into(),
        ));
    }
    let assets: Vec<&PinnedHelperAsset> = pin
        .assets
        .iter()
        .filter(|asset| asset.platform == platform)
        .collect();
    if assets.len() != 1 {
        return Err(HelperAcquisitionError::ReleaseResolution(format!(
            "expected exactly one asset pin for {platform}, found {}",
            assets.len()
        )));
    }
    let asset = assets[0];
    if asset.asset_name.is_empty() || !safe_component(&asset.asset_name) {
        return Err(HelperAcquisitionError::ReleaseResolution(
            "asset name must be one path component".into(),
        ));
    }
    if asset.sha256.len() != 64 || !asset.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HelperAcquisitionError::ReleaseResolution(format!(
            "asset {} does not have a 64-character SHA-256 pin",
            asset.asset_name
        )));
    }
    Ok(())
}

fn validate_resolved_release(
    release: &ResolvedHelperRelease,
) -> Result<(), HelperAcquisitionError> {
    if release.helper.is_empty() || !safe_component(&release.helper) {
        return Err(HelperAcquisitionError::ReleaseResolution(
            "helper name must be one path component".into(),
        ));
    }
    if release.version.is_empty() || !safe_component(&release.version) {
        return Err(HelperAcquisitionError::ReleaseResolution(
            "resolved version must be one path component".into(),
        ));
    }
    if release.release_metadata_url.is_empty() {
        return Err(HelperAcquisitionError::ReleaseResolution(
            "resolved release is missing its metadata URL".into(),
        ));
    }
    if release.asset_name.is_empty() || !safe_component(&release.asset_name) {
        return Err(HelperAcquisitionError::ReleaseResolution(
            "asset name must be one path component".into(),
        ));
    }
    if release.asset_url.is_empty() {
        return Err(HelperAcquisitionError::ReleaseResolution(
            "resolved asset is missing its download URL".into(),
        ));
    }
    if release.sha256.len() != 64 || !release.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HelperAcquisitionError::ReleaseResolution(format!(
            "asset {} does not have a 64-character SHA-256 digest",
            release.asset_name
        )));
    }
    Ok(())
}

fn safe_component(value: &str) -> bool {
    value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains('\0')
}

fn contains_latest_alias(url: &str) -> bool {
    url.split(['/', '?', '#'])
        .any(|segment| segment.eq_ignore_ascii_case("latest"))
}

fn map_staging_error(error: DownloadStagingError) -> HelperAcquisitionError {
    match error {
        DownloadStagingError::ChecksumMismatch {
            expected, actual, ..
        } => HelperAcquisitionError::Checksum { expected, actual },
        DownloadStagingError::Write(error) => map_atomic_write_error(error),
    }
}

fn map_atomic_write_error(error: AtomicWriteError) -> HelperAcquisitionError {
    match error {
        AtomicWriteError::Io(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            HelperAcquisitionError::Permission(error.to_string())
        }
        other => HelperAcquisitionError::Staging(other.to_string()),
    }
}

fn map_filesystem_error(context: &str, error: std::io::Error) -> HelperAcquisitionError {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        HelperAcquisitionError::Permission(format!("{context}: {error}"))
    } else {
        HelperAcquisitionError::Filesystem(format!("{context}: {error}"))
    }
}

fn cleanup(fs: &dyn FileSystem, path: &Path) {
    let _ = fs.remove(path);
}

fn restore_file(fs: &dyn FileSystem, path: &Path, contents: Option<&[u8]>, executable: bool) {
    match contents {
        Some(contents) if executable => {
            let _ = fs.write_executable(path, contents);
        }
        Some(contents) => {
            let _ = fs.write(path, contents);
        }
        None => cleanup(fs, path),
    }
}
