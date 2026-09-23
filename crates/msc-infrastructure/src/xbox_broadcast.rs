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
pub const XBOX_BROADCAST_ALT_PASSWORD_KEY: &str = "xbox-broadcast.alt-password";
pub const XBOX_BROADCAST_AUTH_TOKEN_KEY_PREFIX: &str = "xbox-broadcast.auth-token.";
pub const XBOX_BROADCAST_RELEASES_URL: &str =
    "https://api.github.com/repos/MCXboxBroadcast/Broadcaster/releases/latest";
pub const XBOX_BROADCAST_HELPER_NAME: &str = "xbox-broadcast";
pub const XBOX_BROADCAST_ASSET_NAME: &str = "MCXboxBroadcastStandalone.jar";

pub fn alt_password_secret_key(server_id: &str) -> String {
    format!("{XBOX_BROADCAST_ALT_PASSWORD_KEY_PREFIX}{server_id}")
}

pub const fn global_alt_password_secret_key() -> &'static str {
    XBOX_BROADCAST_ALT_PASSWORD_KEY
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
        "session:\n  update-interval: 30\n  query-server: true\n  session-info:\n    host-name: \"{}\"\n    world-name: \"{} World\"\n    ip: \"{}\"\n    port: {}\nfriend-sync:\n  auto-follow: true\n  auto-unfollow: true\n",
        yaml_quote(server_name),
        yaml_quote(server_name),
        yaml_quote(host.trim()),
        port
    )
}

/// Update only the session target in an existing standalone configuration.
///
/// MCXboxBroadcast keeps authentication and friend-sync settings in this
/// file, so replacing the whole file on every server start would discard
/// operator choices. The standalone config is intentionally simple enough
/// that a small indentation-aware patch is safer than adding a second YAML
/// parser dependency to the agent.
pub fn update_config_yaml(existing: &str, host: &str, port: u16, server_name: &str) -> String {
    let mut output = Vec::new();
    let mut in_session_info = false;
    let mut found_session_info = false;
    let mut replaced = [false; 4];
    let values = [
        ("host-name:", format!("\"{}\"", yaml_quote(server_name))),
        (
            "world-name:",
            format!("\"{} World\"", yaml_quote(server_name)),
        ),
        ("ip:", format!("\"{}\"", yaml_quote(host.trim()))),
        ("port:", port.to_string()),
    ];

    for line in existing.lines() {
        let trimmed = line.trim_start();
        if line.starts_with("  session-info:") {
            in_session_info = true;
            found_session_info = true;
            output.push(line.to_owned());
            continue;
        }
        if in_session_info
            && !trimmed.is_empty()
            && line.starts_with("  ")
            && !line.starts_with("    ")
        {
            in_session_info = false;
        }

        if in_session_info {
            let indent = &line[..line.len() - trimmed.len()];
            let mut replaced_line = false;
            for (index, (key, value)) in values.iter().enumerate() {
                if trimmed.starts_with(key) {
                    output.push(format!("{indent}{key} {value}"));
                    replaced[index] = true;
                    replaced_line = true;
                    break;
                }
            }
            if replaced_line {
                continue;
            }
        }
        output.push(line.to_owned());
    }

    if !found_session_info || replaced.iter().any(|value| !value) {
        return make_config_yaml(host, Some(port), server_name);
    }

    let mut config = output.join("\n");
    config.push('\n');
    config
}

/// Configure build 155's NetherNet broadcaster without relying on its legacy
/// RakNet status ping. The configured session details remain authoritative,
/// and the helper gets a small, predictable UDP range separate from BDS.
pub fn update_nethernet_config_yaml(
    existing: &str,
    host: &str,
    port: u16,
    server_name: &str,
) -> String {
    let config = update_config_yaml(existing, host, port, server_name);
    let (ice_min, ice_max) = nethernet_broadcast_ice_port_range(port);
    let has_query_server = config
        .lines()
        .any(|line| line.starts_with("  query-server:"));
    let has_config_fallback = config
        .lines()
        .any(|line| line.starts_with("  config-fallback:"));
    let has_ice_range = config
        .lines()
        .any(|line| line.starts_with("  ice-port-range:"));
    let mut output = Vec::new();

    for line in config.lines() {
        if line.starts_with("  query-server:") {
            output.push("  query-server: false".to_owned());
            continue;
        }
        if line.starts_with("  config-fallback:") {
            output.push("  config-fallback: true".to_owned());
            continue;
        }
        if line.starts_with("    min:") {
            output.push(format!("    min: {ice_min}"));
            continue;
        }
        if line.starts_with("    max:") {
            output.push(format!("    max: {ice_max}"));
            continue;
        }
        if line.starts_with("  session-info:") {
            if !has_query_server {
                output.push("  query-server: false".to_owned());
            }
            if !has_config_fallback {
                output.push("  config-fallback: true".to_owned());
            }
            if !has_ice_range {
                output.push("  ice-port-range:".to_owned());
                output.push(format!("    min: {ice_min}"));
                output.push(format!("    max: {ice_max}"));
            }
        }
        output.push(line.to_owned());
    }

    let mut config = output.join("\n");
    config.push('\n');
    config
}

/// BDS owns the first 32 UDP ports adjacent to the signaling port. Broadcast
/// uses the next 16 only while a console joins the advertised friend session.
pub fn nethernet_broadcast_ice_port_range(server_port: u16) -> (u16, u16) {
    const BDS_PORT_COUNT: u16 = 32;
    const BROADCAST_PORT_COUNT: u16 = 16;
    const TOTAL_PORT_COUNT: u16 = BDS_PORT_COUNT + BROADCAST_PORT_COUNT;

    if server_port <= u16::MAX - TOTAL_PORT_COUNT {
        (
            server_port + BDS_PORT_COUNT + 1,
            server_port + TOTAL_PORT_COUNT,
        )
    } else {
        (
            server_port - TOTAL_PORT_COUNT,
            server_port - BDS_PORT_COUNT - 1,
        )
    }
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
