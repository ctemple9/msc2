//! Safe acquisition and provenance for the Chunker world-conversion CLI.
//!
//! Chunker is an external Java JAR, so it follows the same bounded download
//! and atomic-promotion rules as the other managed artifacts. The release
//! metadata and asset URL are accepted only from HiveGamesOSS' official
//! GitHub endpoints; a user must explicitly start the download from the
//! conversion wizard.

use crate::atomic_write::atomic_write;
use crate::download_staging::{self, DownloadStagingError, ExpectedChecksum};
use crate::fs::FileSystem;
use crate::jar_provider::{JarProviderError, Transport};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Cursor;
use std::path::{Path, PathBuf};

const RELEASE_METADATA_URL: &str =
    "https://api.github.com/repos/HiveGamesOSS/Chunker/releases/latest";
const RELEASE_DOWNLOAD_PREFIX: &str = "https://github.com/HiveGamesOSS/Chunker/releases/download/";
pub const CHUNKER_MAX_BYTES: u64 = 300 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkerMetadata {
    pub version: String,
    pub asset_name: String,
    pub asset_url: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquiredChunker {
    pub path: PathBuf,
    pub metadata: ChunkerMetadata,
}

#[derive(Debug)]
pub enum ChunkerError {
    Network(JarProviderError),
    InvalidRelease(String),
    InvalidArtifact(String),
    Staging(DownloadStagingError),
    Filesystem(String),
}

impl fmt::Display for ChunkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(error) => write!(f, "Chunker release download failed: {error}"),
            Self::InvalidRelease(message) => write!(f, "Chunker release is invalid: {message}"),
            Self::InvalidArtifact(message) => write!(f, "Chunker artifact is invalid: {message}"),
            Self::Staging(error) => write!(f, "Chunker staging failed: {error}"),
            Self::Filesystem(message) => {
                write!(f, "Chunker filesystem operation failed: {message}")
            }
        }
    }
}

impl std::error::Error for ChunkerError {}

#[derive(Debug, Deserialize)]
struct ReleaseMetadata {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

pub fn jar_path() -> PathBuf {
    std::env::var_os("MSC2_CHUNKER_JAR_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| app_support_directory().join("chunker-cli.jar"))
}

pub fn metadata_path() -> PathBuf {
    let path = jar_path();
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    path.with_file_name(format!("{name}.metadata.json"))
}

pub fn installed_version() -> Option<String> {
    let bytes = std::fs::read(metadata_path()).ok()?;
    serde_json::from_slice::<ChunkerMetadata>(&bytes)
        .ok()
        .map(|metadata| metadata.version)
}

pub fn is_installed() -> bool {
    std::fs::metadata(jar_path())
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

pub fn download_latest(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    progress: &mut dyn FnMut(&str),
) -> Result<AcquiredChunker, ChunkerError> {
    progress("Fetching the latest Chunker release…");
    let metadata_bytes = transport
        .get(
            RELEASE_METADATA_URL,
            "Chunker release metadata",
            2 * 1024 * 1024,
        )
        .map_err(ChunkerError::Network)?;
    let release: ReleaseMetadata = serde_json::from_slice(&metadata_bytes).map_err(|error| {
        ChunkerError::InvalidRelease(format!("invalid release metadata: {error}"))
    })?;
    if release.tag_name.trim().is_empty() || release.tag_name.contains('/') {
        return Err(ChunkerError::InvalidRelease(
            "release has no safe version tag".to_string(),
        ));
    }
    let asset = release
        .assets
        .iter()
        .find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.ends_with(".jar") && name.contains("cli")
        })
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|asset| asset.name.to_ascii_lowercase().ends_with(".jar"))
        })
        .ok_or_else(|| ChunkerError::InvalidRelease("release contains no JAR asset".to_string()))?;
    if !is_safe_asset_name(&asset.name)
        || !asset
            .browser_download_url
            .starts_with(RELEASE_DOWNLOAD_PREFIX)
    {
        return Err(ChunkerError::InvalidRelease(
            "release asset is not an official Chunker GitHub download".to_string(),
        ));
    }

    progress(&format!("Downloading Chunker {}…", release.tag_name));
    let bytes = transport
        .get(
            &asset.browser_download_url,
            "Chunker CLI asset",
            CHUNKER_MAX_BYTES,
        )
        .map_err(ChunkerError::Network)?;
    validate_jar(&bytes)?;

    let sha256 = download_staging::sha256_hex(&bytes);
    let expected = match asset.digest.as_deref() {
        None => None,
        Some(digest) => Some(parse_sha256_digest(digest).ok_or_else(|| {
            ChunkerError::InvalidRelease("release asset has an invalid SHA-256 digest".to_string())
        })?),
    };
    if let Some(expected) = expected.as_deref()
        && !expected.eq_ignore_ascii_case(&sha256)
    {
        return Err(ChunkerError::InvalidArtifact(format!(
            "SHA-256 mismatch: expected {expected}, got {sha256}"
        )));
    }

    let jar = jar_path();
    let parent = jar
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| {
            ChunkerError::Filesystem("Chunker path has no parent directory".to_string())
        })?;
    fs.create_dir_all(parent)
        .map_err(|error| ChunkerError::Filesystem(format!("create cache directory: {error}")))?;
    let staged_jar = jar.with_file_name(".chunker-cli.jar.download");
    let metadata = ChunkerMetadata {
        version: release.tag_name,
        asset_name: asset.name.clone(),
        asset_url: asset.browser_download_url.clone(),
        sha256,
    };
    let metadata_bytes = serde_json::to_vec_pretty(&metadata)
        .map_err(|error| ChunkerError::Filesystem(format!("serialize metadata: {error}")))?;
    let metadata_file = metadata_path();
    let staged_metadata = metadata_file.with_file_name(".chunker-cli.jar.metadata.json.download");

    let checksum = expected.map(ExpectedChecksum::sha256);
    download_staging::stage_download(
        fs,
        &staged_jar,
        &bytes,
        &metadata.asset_url,
        &metadata.version,
        checksum.as_ref(),
    )
    .map_err(ChunkerError::Staging)?;
    atomic_write(fs, &staged_metadata, &metadata_bytes).map_err(|error| {
        cleanup(fs, &staged_jar);
        ChunkerError::Filesystem(format!("stage metadata: {error}"))
    })?;

    if let Err(error) = fs.rename(&staged_jar, &jar) {
        cleanup(fs, &staged_jar);
        cleanup(fs, &staged_metadata);
        return Err(ChunkerError::Filesystem(format!("promote JAR: {error}")));
    }
    if let Err(error) = fs.rename(&staged_metadata, &metadata_file) {
        cleanup(fs, &staged_metadata);
        return Err(ChunkerError::Filesystem(format!(
            "promote metadata: {error}"
        )));
    }

    progress(&format!("Chunker {} is ready.", metadata.version));
    Ok(AcquiredChunker {
        path: jar,
        metadata,
    })
}

fn app_support_directory() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        #[cfg(target_os = "macos")]
        {
            return home.join("Library/Application Support/MSC2");
        }
        #[cfg(not(target_os = "macos"))]
        {
            return home.join(".msc2");
        }
    }
    std::env::temp_dir().join("msc2")
}

fn is_safe_asset_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
}

fn parse_sha256_digest(value: &str) -> Option<String> {
    let digest = value.strip_prefix("sha256:")?;
    (digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| digest.to_string())
}

fn validate_jar(bytes: &[u8]) -> Result<(), ChunkerError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| {
        ChunkerError::InvalidArtifact(format!("download is not a readable JAR archive: {error}"))
    })?;
    if archive.is_empty() {
        return Err(ChunkerError::InvalidArtifact(
            "downloaded JAR has no entries".to_string(),
        ));
    }
    let _ = archive.by_index(0).map_err(|error| {
        ChunkerError::InvalidArtifact(format!("downloaded JAR cannot be read: {error}"))
    })?;
    Ok(())
}

fn cleanup(fs: &dyn FileSystem, path: &Path) {
    let _ = fs.remove(path);
}
