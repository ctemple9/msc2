//! Infrastructure primitives for MCXboxBroadcastStandalone.
//!
//! The helper is a Java process, so its account state lives in its working
//! directory.  MSC still treats credentials as secrets: this module exposes
//! stable SecretStore keys and never puts their values in a launch request.

use crate::download_staging::{self, CachedFile, DownloadStagingError};
use crate::fs::FileSystem;
use crate::jar_provider::{JarProviderError, Transport};
use crate::process::ProcessSpawnRequest;
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const XBOX_BROADCAST_ALT_PASSWORD_KEY_PREFIX: &str = "xbox-broadcast.alt-password.";
pub const XBOX_BROADCAST_AUTH_TOKEN_KEY_PREFIX: &str = "xbox-broadcast.auth-token.";
pub const XBOX_BROADCAST_JAR_URL: &str = "https://github.com/MCXboxBroadcast/Broadcaster/releases/latest/download/MCXboxBroadcastStandalone.jar";
pub const XBOX_BROADCAST_RELEASES_URL: &str =
    "https://api.github.com/repos/MCXboxBroadcast/Broadcaster/releases/latest";
pub const XBOX_BROADCAST_MAX_BYTES: u64 = 100 * 1024 * 1024;

pub fn alt_password_secret_key(server_id: &str) -> String {
    format!("{XBOX_BROADCAST_ALT_PASSWORD_KEY_PREFIX}{server_id}")
}

pub fn auth_token_secret_key(server_id: &str) -> String {
    format!("{XBOX_BROADCAST_AUTH_TOKEN_KEY_PREFIX}{server_id}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XboxBroadcastLaunch {
    pub java_path: PathBuf,
    pub jar_path: PathBuf,
    pub working_directory: PathBuf,
}

impl XboxBroadcastLaunch {
    pub fn process_request(&self) -> ProcessSpawnRequest {
        ProcessSpawnRequest::new(&self.java_path, &self.working_directory).args([
            "-jar".to_string(),
            self.jar_path.to_string_lossy().into_owned(),
        ])
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
}

/// Downloads the latest standalone JAR into the caller's library directory.
/// GitHub release metadata supplies the version; the JAR is staged and moved
/// only after the bounded download succeeds.
pub fn download_latest_jar(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    library_directory: &Path,
) -> Result<CachedFile, XboxBroadcastDownloadError> {
    fs.create_dir_all(library_directory)
        .map_err(|error| XboxBroadcastDownloadError::Filesystem(error.to_string()))?;
    let metadata = transport
        .get(
            XBOX_BROADCAST_RELEASES_URL,
            "MCXboxBroadcast release metadata",
            2 * 1024 * 1024,
        )
        .map_err(XboxBroadcastDownloadError::Provider)?;
    let release: LatestRelease = serde_json::from_slice(&metadata)
        .map_err(|error| XboxBroadcastDownloadError::InvalidMetadata(error.to_string()))?;
    let version = safe_version(&release.tag_name);
    let url = release
        .assets
        .iter()
        .find(|asset| asset.name == "MCXboxBroadcastStandalone.jar")
        .map(|asset| asset.browser_download_url.as_str())
        .unwrap_or(XBOX_BROADCAST_JAR_URL);
    let bytes = transport
        .get(url, "MCXboxBroadcast JAR", XBOX_BROADCAST_MAX_BYTES)
        .map_err(XboxBroadcastDownloadError::Provider)?;
    let destination = library_directory.join(format!("MCXboxBroadcastStandalone-{version}.jar"));
    download_staging::stage_download(fs, &destination, &bytes, url, &version, None)
        .map_err(XboxBroadcastDownloadError::Staging)
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

fn safe_version(raw: &str) -> String {
    let value: String = raw
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if value.is_empty() {
        "unknown".into()
    } else {
        value
    }
}

#[derive(Debug)]
pub enum XboxBroadcastDownloadError {
    Provider(JarProviderError),
    InvalidMetadata(String),
    Filesystem(String),
    Staging(DownloadStagingError),
}

impl std::fmt::Display for XboxBroadcastDownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provider(error) => write!(f, "{error}"),
            Self::InvalidMetadata(error) => {
                write!(f, "invalid MCXboxBroadcast release metadata: {error}")
            }
            Self::Filesystem(error) => write!(f, "MCXboxBroadcast library: {error}"),
            Self::Staging(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for XboxBroadcastDownloadError {}
