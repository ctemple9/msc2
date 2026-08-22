//! Infrastructure primitives for MCXboxBroadcastStandalone.
//!
//! The helper is a Java process, so its account state lives in its working
//! directory.  MSC still treats credentials as secrets: this module exposes
//! stable SecretStore keys and never puts their values in a launch request.

use crate::download_staging::CachedFile;
use crate::fs::FileSystem;
use crate::helper_acquisition::{
    AcquiredHelper, ChecksumSource, HelperAcquisitionError, HelperPlatform, ResolvedHelperRelease,
    acquire_resolved_helper,
};
use crate::jar_provider::{JarProviderError, Transport};
use crate::process::ProcessSpawnRequest;
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const XBOX_BROADCAST_ALT_PASSWORD_KEY_PREFIX: &str = "xbox-broadcast.alt-password.";
pub const XBOX_BROADCAST_AUTH_TOKEN_KEY_PREFIX: &str = "xbox-broadcast.auth-token.";
pub const XBOX_BROADCAST_RELEASES_URL: &str =
    "https://api.github.com/repos/MCXboxBroadcast/Broadcaster/releases/latest";
pub const XBOX_BROADCAST_HELPER_NAME: &str = "xbox-broadcast";
pub const XBOX_BROADCAST_ASSET_NAME: &str = "MCXboxBroadcastStandalone.jar";

pub fn alt_password_secret_key(server_id: &str) -> String {
    format!("{XBOX_BROADCAST_ALT_PASSWORD_KEY_PREFIX}{server_id}")
}

pub fn auth_token_secret_key(server_id: &str) -> String {
    format!("{XBOX_BROADCAST_AUTH_TOKEN_KEY_PREFIX}{server_id}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XboxBroadcastLaunch {
    pub java_path: PathBuf,
    pub working_directory: PathBuf,
}

impl XboxBroadcastLaunch {
    pub fn process_request(&self, jar_path: &Path) -> ProcessSpawnRequest {
        ProcessSpawnRequest::new(&self.java_path, &self.working_directory)
            .args(["-jar".to_string(), jar_path.to_string_lossy().into_owned()])
    }
}

#[derive(Debug, Deserialize)]
struct LatestRelease {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
}

/// Testable inputs for Broadcast's latest-release acquisition. The metadata
/// request still follows MSC 1's latest-release behavior; only the concrete
/// asset and its upstream digest cross into the shared acquisition boundary.
pub struct XboxBroadcastJarAcquisition<'a> {
    transport: &'a dyn Transport,
    fs: &'a dyn FileSystem,
    cache_directory: &'a Path,
    platform: HelperPlatform,
}

impl<'a> XboxBroadcastJarAcquisition<'a> {
    pub fn new(
        transport: &'a dyn Transport,
        fs: &'a dyn FileSystem,
        cache_directory: &'a Path,
        platform: HelperPlatform,
    ) -> Self {
        Self {
            transport,
            fs,
            cache_directory,
            platform,
        }
    }

    pub fn for_current_platform(
        transport: &'a dyn Transport,
        fs: &'a dyn FileSystem,
        cache_directory: &'a Path,
    ) -> Result<Self, HelperAcquisitionError> {
        Ok(Self::new(
            transport,
            fs,
            cache_directory,
            HelperPlatform::current()?,
        ))
    }

    pub fn acquire(&self) -> Result<AcquiredHelper, XboxBroadcastDownloadError> {
        let metadata = self
            .transport
            .get(
                XBOX_BROADCAST_RELEASES_URL,
                "MCXboxBroadcast release metadata",
                2 * 1024 * 1024,
            )
            .map_err(XboxBroadcastDownloadError::Provider)?;
        let release: LatestRelease = serde_json::from_slice(&metadata)
            .map_err(|error| XboxBroadcastDownloadError::InvalidMetadata(error.to_string()))?;
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == XBOX_BROADCAST_ASSET_NAME)
            .ok_or_else(|| {
                XboxBroadcastDownloadError::InvalidMetadata(format!(
                    "release {} has no asset named {XBOX_BROADCAST_ASSET_NAME}",
                    release.tag_name
                ))
            })?;
        let digest = asset.digest.as_deref().ok_or_else(|| {
            XboxBroadcastDownloadError::InvalidMetadata(format!(
                "asset {} has no upstream sha256 digest",
                asset.name
            ))
        })?;
        let sha256 = digest.strip_prefix("sha256:").ok_or_else(|| {
            XboxBroadcastDownloadError::InvalidMetadata(format!(
                "asset {} has an unsupported digest format",
                asset.name
            ))
        })?;
        if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(XboxBroadcastDownloadError::InvalidMetadata(format!(
                "asset {} sha256 is not a 64-character hex digest",
                asset.name
            )));
        }
        if release.tag_name.is_empty()
            || release.tag_name == "."
            || release.tag_name == ".."
            || release.tag_name.contains(['/', '\\', '\0'])
        {
            return Err(XboxBroadcastDownloadError::InvalidMetadata(
                "release tag is not a safe version".into(),
            ));
        }
        let resolved = ResolvedHelperRelease {
            helper: XBOX_BROADCAST_HELPER_NAME.into(),
            version: release.tag_name,
            platform: self.platform,
            release_metadata_url: XBOX_BROADCAST_RELEASES_URL.into(),
            asset_name: asset.name.clone(),
            asset_url: asset.browser_download_url.clone(),
            sha256: sha256.into(),
            checksum_source: ChecksumSource::UpstreamPublished,
        };
        acquire_resolved_helper(self.transport, self.fs, self.cache_directory, &resolved)
            .map_err(XboxBroadcastDownloadError::Acquisition)
    }
}

/// Downloads the latest standalone JAR into the caller's library directory.
/// GitHub release metadata supplies the version and checksum; the shared
/// helper acquisition boundary verifies and records both before promotion.
pub fn download_latest_jar(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    library_directory: &Path,
) -> Result<CachedFile, XboxBroadcastDownloadError> {
    XboxBroadcastJarAcquisition::for_current_platform(transport, fs, library_directory)
        .map_err(XboxBroadcastDownloadError::Acquisition)?
        .acquire()
        .map(|acquired| acquired.artifact)
}

/// The helper's config is intentionally plain text: it contains the player
/// destination and display name, never a Microsoft password or auth token.
pub fn make_config_yaml(host: &str, port: Option<u16>, server_name: &str) -> String {
    let port = port.map_or_else(String::new, |port| port.to_string());
    format!(
        "session:\n  update-interval: 30\n  query-server: true\n  session-info:\n    host-name: \"{}\"\n    world-name: \"{} World\"\n    ip: {}\n    port: {}\nfriend-sync:\n  auto-follow: true\n  auto-unfollow: true\n",
        yaml_quote(server_name),
        yaml_quote(server_name),
        host.trim(),
        port
    )
}

fn yaml_quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[derive(Debug)]
pub enum XboxBroadcastDownloadError {
    Provider(JarProviderError),
    InvalidMetadata(String),
    Acquisition(HelperAcquisitionError),
}

impl std::fmt::Display for XboxBroadcastDownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provider(error) => write!(f, "{error}"),
            Self::InvalidMetadata(error) => {
                write!(f, "invalid MCXboxBroadcast release metadata: {error}")
            }
            Self::Acquisition(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for XboxBroadcastDownloadError {}
