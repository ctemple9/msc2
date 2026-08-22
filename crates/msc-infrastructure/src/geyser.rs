//! GeyserMC's latest Paper-plugin resolver.
//!
//! MSC 1 asks the Geyser API for the latest version and build, then downloads
//! the `spigot` artifact for that exact result. MSC 2 keeps that moving
//! resolution behavior, but carries the API's SHA-256 into the shared helper
//! acquisition boundary before the JAR can be staged or used.

use crate::fs::FileSystem;
use crate::helper_acquisition::{
    AcquiredHelper, ChecksumSource, HelperAcquisitionError, HelperPlatform, ResolvedHelperRelease,
    acquire_resolved_helper,
};
use crate::jar_provider::{JarProviderError, Transport};
use serde::Deserialize;
use std::fmt;

pub const GEYSER_API_BASE_URL: &str = "https://download.geysermc.org/v2";
const METADATA_MAX_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeyserProject {
    Geyser,
    Floodgate,
}

impl GeyserProject {
    pub const fn api_name(self) -> &'static str {
        match self {
            Self::Geyser => "geyser",
            Self::Floodgate => "floodgate",
        }
    }

    pub const fn helper_name(self) -> &'static str {
        self.api_name()
    }

    pub const fn jar_name(self) -> &'static str {
        match self {
            Self::Geyser => "Geyser-Spigot.jar",
            Self::Floodgate => "floodgate-spigot.jar",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeyserBuild {
    pub project: GeyserProject,
    pub version: String,
    pub build: u64,
    pub sha256: String,
}

impl GeyserBuild {
    pub fn display_version(&self) -> String {
        format!("{} (build {})", self.version, self.build)
    }

    pub fn download_url(&self) -> String {
        format!(
            "{GEYSER_API_BASE_URL}/projects/{}/versions/{}/builds/{}/downloads/spigot",
            self.project.api_name(),
            self.version,
            self.build
        )
    }

    pub fn resolved_release(&self, platform: HelperPlatform) -> ResolvedHelperRelease {
        ResolvedHelperRelease {
            helper: self.project.helper_name().into(),
            version: format!("{}-build-{}", self.version, self.build),
            platform,
            release_metadata_url: latest_build_url(self.project),
            asset_name: self.project.jar_name().into(),
            asset_url: self.download_url(),
            sha256: self.sha256.clone(),
            checksum_source: ChecksumSource::UpstreamPublished,
        }
    }
}

#[derive(Debug)]
pub enum GeyserResolutionError {
    Download(JarProviderError),
    InvalidResponse(String),
}

impl fmt::Display for GeyserResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Download(error) => write!(f, "Geyser metadata download failed: {error}"),
            Self::InvalidResponse(message) => {
                write!(f, "Geyser metadata response is invalid: {message}")
            }
        }
    }
}

impl std::error::Error for GeyserResolutionError {}

#[derive(Debug)]
pub enum GeyserAcquisitionError {
    Resolution(GeyserResolutionError),
    Acquisition(HelperAcquisitionError),
}

impl fmt::Display for GeyserAcquisitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resolution(error) => error.fmt(f),
            Self::Acquisition(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for GeyserAcquisitionError {}

#[derive(Debug, Deserialize)]
struct LatestBuildResponse {
    version: String,
    build: BuildNumber,
    downloads: Downloads,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BuildNumber {
    Number(u64),
    Text(String),
}

impl BuildNumber {
    fn parse(self) -> Result<u64, String> {
        match self {
            Self::Number(value) => Ok(value),
            Self::Text(value) => value
                .parse()
                .map_err(|_| format!("build is not an integer: {value}")),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Downloads {
    spigot: Option<SpigotDownload>,
}

#[derive(Debug, Deserialize)]
struct SpigotDownload {
    sha256: Option<String>,
}

pub fn latest_build_url(project: GeyserProject) -> String {
    format!(
        "{GEYSER_API_BASE_URL}/projects/{}/versions/latest/builds/latest",
        project.api_name()
    )
}

/// Resolves exactly the latest Paper-family `spigot` build that MSC 1 uses.
/// The injected transport makes resolution deterministic in tests and keeps
/// the public network outside the test process.
pub fn resolve_latest_build(
    transport: &dyn Transport,
    project: GeyserProject,
) -> Result<GeyserBuild, GeyserResolutionError> {
    let url = latest_build_url(project);
    let bytes = transport
        .get(&url, "Geyser latest build metadata", METADATA_MAX_BYTES)
        .map_err(GeyserResolutionError::Download)?;
    let response: LatestBuildResponse = serde_json::from_slice(&bytes).map_err(|error| {
        GeyserResolutionError::InvalidResponse(format!("invalid JSON: {error}"))
    })?;
    let version = response.version.trim();
    if version.is_empty() || version.contains(['/', '\\']) {
        return Err(GeyserResolutionError::InvalidResponse(
            "version is empty or contains a path separator".into(),
        ));
    }
    let build = response
        .build
        .parse()
        .map_err(GeyserResolutionError::InvalidResponse)?;
    let sha256 = response
        .downloads
        .spigot
        .and_then(|download| download.sha256)
        .ok_or_else(|| {
            GeyserResolutionError::InvalidResponse(
                "the spigot artifact has no upstream sha256".into(),
            )
        })?;
    if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GeyserResolutionError::InvalidResponse(
            "the spigot artifact sha256 is not a 64-character hex digest".into(),
        ));
    }

    Ok(GeyserBuild {
        project,
        version: version.to_string(),
        build,
        sha256,
    })
}

pub fn acquire_latest(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    cache_directory: &std::path::Path,
    project: GeyserProject,
    platform: HelperPlatform,
) -> Result<(GeyserBuild, AcquiredHelper), GeyserAcquisitionError> {
    let build =
        resolve_latest_build(transport, project).map_err(GeyserAcquisitionError::Resolution)?;
    let release = build.resolved_release(platform);
    let acquired = acquire_resolved_helper(transport, fs, cache_directory, &release)
        .map_err(GeyserAcquisitionError::Acquisition)?;
    Ok((build, acquired))
}
