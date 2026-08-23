//! Verified acquisition and isolated staging for the official Bedrock server.
//!
//! MSC 1 selected a Linux entry from the public manifest and extracted the ZIP
//! directly into the live server directory. That path had no checksum or
//! archive-identity check. This module keeps the useful manifest/version
//! behavior while making verification and staging explicit: bytes are checked
//! before they are written, the ZIP is parsed before it becomes runnable, and
//! extraction happens under a separate directory owned by the caller.

use crate::download_staging::sha256_hex;
use crate::fs::FileSystem;
use msc_domain::version::is_downgrade;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

pub const BEDROCK_MANIFEST_MAX_BYTES: u64 = 20 * 1024 * 1024;
pub const BEDROCK_ARCHIVE_MAX_BYTES: u64 = 500 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedrockPlatform {
    Linux,
    Windows,
    Macos,
}

impl BedrockPlatform {
    pub fn manifest_key(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Windows => "windows",
            Self::Macos => "macos",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedrockVersionRequest<'a> {
    Latest,
    Pinned(&'a str),
}

impl<'a> BedrockVersionRequest<'a> {
    pub fn parse(value: Option<&'a str>) -> Self {
        match value.map(str::trim) {
            None | Some("") | Some("LATEST") | Some("latest") => Self::Latest,
            Some(version) => Self::Pinned(version),
        }
    }

    pub fn is_latest(self) -> bool {
        matches!(self, Self::Latest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockRelease {
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub platform: BedrockPlatform,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedBedrockDistribution {
    pub root: PathBuf,
    pub release: BedrockRelease,
}

#[derive(Debug)]
pub enum BedrockDistributionError {
    Manifest(String),
    VersionNotFound(String),
    NoPlatformRelease(BedrockPlatform),
    UnverifiedArchive,
    InvalidChecksum(String),
    ChecksumMismatch { expected: String, actual: String },
    ArchiveCorrupt(String),
    ArchiveMissingExecutable,
    Filesystem(String),
}

impl fmt::Display for BedrockDistributionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(message) => write!(f, "Bedrock manifest is invalid: {message}"),
            Self::VersionNotFound(version) => {
                write!(
                    f,
                    "Bedrock version {version} was not found in the download manifest."
                )
            }
            Self::NoPlatformRelease(platform) => {
                write!(
                    f,
                    "no Bedrock {} download found in the manifest",
                    platform.manifest_key()
                )
            }
            Self::UnverifiedArchive => {
                write!(f, "Bedrock archive has no published SHA-256 identity")
            }
            Self::InvalidChecksum(value) => {
                write!(f, "Bedrock manifest SHA-256 is invalid: {value}")
            }
            Self::ChecksumMismatch { expected, actual } => {
                write!(
                    f,
                    "Bedrock SHA-256 mismatch: expected {expected}, got {actual}"
                )
            }
            Self::ArchiveCorrupt(message) => write!(f, "Bedrock archive is corrupt: {message}"),
            Self::ArchiveMissingExecutable => {
                f.write_str("Bedrock archive did not contain a bedrock_server executable")
            }
            Self::Filesystem(message) => write!(f, "Bedrock staging filesystem error: {message}"),
        }
    }
}

impl std::error::Error for BedrockDistributionError {}

#[derive(Debug, Deserialize)]
struct Manifest {
    release: BTreeMap<String, BTreeMap<String, ManifestEntry>>,
}

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    url: String,
    #[serde(alias = "sha256sum", alias = "checksum")]
    sha256: Option<String>,
}

impl BedrockPlatform {
    fn release<'a>(&self, manifest: &'a Manifest, key: &str) -> Option<&'a ManifestEntry> {
        manifest.release.get(key)?.get(self.manifest_key())
    }
}

/// Resolves a pinned or latest release from the same public manifest shape as
/// MSC 1. The platform is selected before the version is selected, so a
/// Windows or macOS backend can never accidentally consume the Linux asset.
pub fn resolve_release(
    manifest_bytes: &[u8],
    request: BedrockVersionRequest<'_>,
    platform: BedrockPlatform,
) -> Result<BedrockRelease, BedrockDistributionError> {
    let manifest: Manifest = serde_json::from_slice(manifest_bytes)
        .map_err(|error| BedrockDistributionError::Manifest(error.to_string()))?;

    let (key, entry) = match request {
        BedrockVersionRequest::Pinned(version) => manifest
            .release
            .keys()
            .filter_map(|key| platform.release(&manifest, key).map(|entry| (key, entry)))
            .find(|(_, entry)| version_from_url(&entry.url).as_deref() == Some(version))
            .or_else(|| {
                manifest
                    .release
                    .keys()
                    .filter_map(|key| platform.release(&manifest, key).map(|entry| (key, entry)))
                    .find(|(_, entry)| entry.url.contains(version))
            })
            .ok_or_else(|| BedrockDistributionError::VersionNotFound(version.to_string()))?,
        BedrockVersionRequest::Latest => manifest
            .release
            .keys()
            .filter_map(|key| platform.release(&manifest, key).map(|entry| (key, entry)))
            .filter(|(key, _)| numeric_version(key).is_some())
            .max_by(|(left, _), (right, _)| compare_numeric_versions(left, right))
            .ok_or(BedrockDistributionError::NoPlatformRelease(platform))?,
    };

    let sha256 = entry
        .sha256
        .clone()
        .ok_or(BedrockDistributionError::UnverifiedArchive)?;
    if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(BedrockDistributionError::InvalidChecksum(sha256));
    }

    Ok(BedrockRelease {
        version: version_from_url(&entry.url).unwrap_or_else(|| key.clone()),
        url: entry.url.clone(),
        sha256,
        platform,
    })
}

/// Verifies and extracts one release into `staging_root/version`. Existing
/// staging is removed first so a retry cannot accidentally reuse a partial
/// extraction. The returned directory is never the live server directory.
pub fn stage_archive(
    fs: &dyn FileSystem,
    staging_root: &Path,
    release: &BedrockRelease,
    bytes: &[u8],
) -> Result<StagedBedrockDistribution, BedrockDistributionError> {
    if bytes.len() as u64 > BEDROCK_ARCHIVE_MAX_BYTES {
        return Err(BedrockDistributionError::ArchiveCorrupt(
            "archive exceeds the size limit".into(),
        ));
    }
    let actual = sha256_hex(bytes);
    if !actual.eq_ignore_ascii_case(&release.sha256) {
        return Err(BedrockDistributionError::ChecksumMismatch {
            expected: release.sha256.clone(),
            actual,
        });
    }

    let root = staging_root.join(&release.version);
    let _ = fs.remove(&root);
    fs.create_dir_all(&root)
        .map_err(|error| BedrockDistributionError::Filesystem(error.to_string()))?;

    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| BedrockDistributionError::ArchiveCorrupt(error.to_string()))?;
    let mut has_executable = false;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| BedrockDistributionError::ArchiveCorrupt(error.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let path = entry
            .enclosed_name()
            .ok_or_else(|| BedrockDistributionError::ArchiveCorrupt("unsafe entry path".into()))?
            .to_path_buf();
        if path.as_os_str().is_empty() {
            continue;
        }
        let destination = root.join(&path);
        if let Some(parent) = destination.parent() {
            fs.create_dir_all(parent)
                .map_err(|error| BedrockDistributionError::Filesystem(error.to_string()))?;
        }
        let mut contents = Vec::new();
        entry
            .read_to_end(&mut contents)
            .map_err(|error| BedrockDistributionError::ArchiveCorrupt(error.to_string()))?;
        let executable = entry.unix_mode().is_some_and(|mode| mode & 0o111 != 0)
            || path == Path::new("bedrock_server");
        if path == Path::new("bedrock_server") {
            has_executable = executable;
        }
        if executable {
            fs.write_executable(&destination, &contents)
        } else {
            fs.write(&destination, &contents)
        }
        .map_err(|error| BedrockDistributionError::Filesystem(error.to_string()))?;
    }
    if !has_executable {
        let _ = fs.remove(&root);
        return Err(BedrockDistributionError::ArchiveMissingExecutable);
    }

    Ok(StagedBedrockDistribution {
        root,
        release: release.clone(),
    })
}

/// The Phase 7 downgrade rule in one place, allowing the application layer
/// to decide how a server's safety backup is actually made.
pub fn requires_downgrade_backup(installed: Option<&str>, target: &str) -> bool {
    is_downgrade(installed, target)
}

fn version_from_url(url: &str) -> Option<String> {
    let start = url.find("bedrock-server-")? + "bedrock-server-".len();
    let rest = &url[start..];
    let end = rest.find(".zip")?;
    let version = &rest[..end];
    (!version.is_empty() && numeric_version(version).is_some()).then(|| version.to_string())
}

fn numeric_version(value: &str) -> Option<Vec<u64>> {
    let parts: Vec<u64> = value
        .split('.')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    (!parts.is_empty()).then_some(parts)
}

fn compare_numeric_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let left = numeric_version(left).unwrap_or_default();
    let right = numeric_version(right).unwrap_or_default();
    let count = left.len().max(right.len());
    (0..count)
        .map(|index| {
            left.get(index)
                .copied()
                .unwrap_or(0)
                .cmp(&right.get(index).copied().unwrap_or(0))
        })
        .find(|order| *order != std::cmp::Ordering::Equal)
        .unwrap_or(std::cmp::Ordering::Equal)
}
