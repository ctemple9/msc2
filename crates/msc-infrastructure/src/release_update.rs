//! Signed MSC application release retrieval and local staging.
//!
//! GitHub is used only as a transport. The release manifest is the authority:
//! it is signed, canonical, platform-specific, and names every byte that may
//! be downloaded. This module stops at an immutable staging directory; the
//! caller must make a separate local installation decision.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::Duration,
};
use ureq::ResponseExt;

pub const MANIFEST_MAX_BYTES: u64 = 256 * 1024;
pub const SIGNATURE_MAX_BYTES: u64 = 4 * 1024;
pub const RELEASE_NOTES_MAX_BYTES: u64 = 256 * 1024;
pub const ASSET_MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const TOTAL_DOWNLOAD_MAX_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Selects the release artifact family for a local updater.
///
/// A headless agent must not accidentally select a desktop installer. Linux
/// package installs are the one deliberate exception: the package manager
/// owns the files, so the CLI stages the matching package and reports the
/// command the operator must run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateChannel {
    Desktop,
    Headless,
    LinuxPackageDeb,
    LinuxPackageRpm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxPackageFormat {
    Deb,
    Rpm,
}

const CURRENT_API_MAJOR: u32 = 1;
const RELEASE_SET: &str = "msc-application";
const MANIFEST_FILE: &str = "msc2-update-manifest.json";
const SIGNATURE_FILE: &str = "msc2-update-manifest.sig";
const RELEASE_NOTES_FILE: &str = "RELEASE-NOTES.md";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const USER_AGENT: &str = "msc2-update-client/1";

#[derive(Debug, Clone)]
pub struct UpdateClientConfig {
    pub repository: String,
    pub current_version: String,
    pub api_major: u32,
    pub api_minor: u32,
    pub target: String,
    pub trusted_key: [u8; 32],
    pub channel: UpdateChannel,
    pub linux_package_format: Option<LinuxPackageFormat>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    pub state: &'static str,
    pub release_id: Option<String>,
    pub release_notes: String,
    pub install_mode: Option<String>,
    pub target: Option<String>,
    pub artifact_filename: Option<String>,
    pub staged_directory: Option<PathBuf>,
    pub detail: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseManifest {
    pub schema_version: u32,
    pub release_set: String,
    pub release_id: String,
    pub tag: String,
    pub api: ApiCompatibility,
    pub platforms: std::collections::BTreeMap<String, PlatformRelease>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiCompatibility {
    pub major: u32,
    pub min_minor: u32,
    pub max_minor: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformRelease {
    pub target: String,
    pub install_mode: String,
    pub assets: Vec<ReleaseAsset>,
    #[serde(default)]
    pub included_components: Vec<String>,
    #[serde(default)]
    pub forbidden_artifacts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseAsset {
    pub role: String,
    pub filename: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone)]
pub struct StagedUpdate {
    pub manifest: ReleaseManifest,
    pub install_mode: String,
    pub asset_role: String,
    pub artifact_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
}

/// Fetches, verifies, and atomically stages the current local platform's
/// signed release set beneath data_directory/updates/<release-id>.
pub fn check_and_stage(
    config: &UpdateClientConfig,
    data_directory: &Path,
) -> Result<UpdateResult, String> {
    validate_repository(&config.repository)?;
    let agent = http_agent();
    let latest = latest_release(&agent, &config.repository)?;
    let latest_id = release_id_from_tag(&latest.tag_name)?;

    if compare_versions(&latest_id, &config.current_version)? != std::cmp::Ordering::Greater {
        return Ok(UpdateResult {
            state: "current",
            release_id: Some(latest_id),
            release_notes: String::new(),
            install_mode: None,
            target: None,
            artifact_filename: None,
            staged_directory: None,
            detail: "MSC 2 is already up to date.".to_string(),
        });
    }

    let base = release_base_url(&config.repository, &latest_id);
    let manifest_bytes = download_bytes(
        &agent,
        &format!("{base}/{MANIFEST_FILE}"),
        MANIFEST_MAX_BYTES,
        &github_hosts(true),
        MANIFEST_FILE,
    )?;
    let signature_bytes = download_bytes(
        &agent,
        &format!("{base}/{SIGNATURE_FILE}"),
        SIGNATURE_MAX_BYTES,
        &github_hosts(true),
        SIGNATURE_FILE,
    )?;
    let release_notes = download_bytes(
        &agent,
        &format!("{base}/{RELEASE_NOTES_FILE}"),
        RELEASE_NOTES_MAX_BYTES,
        &github_hosts(true),
        RELEASE_NOTES_FILE,
    )?;

    let manifest_value: serde_json::Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("Update manifest is invalid JSON: {error}"))?;
    let canonical = serde_json::to_vec(&manifest_value)
        .map_err(|error| format!("Could not canonicalize the update manifest: {error}"))?;
    if canonical != manifest_bytes {
        return Err("Update manifest is not in canonical form.".to_string());
    }
    let manifest: ReleaseManifest = serde_json::from_value(manifest_value)
        .map_err(|error| format!("Update manifest has an invalid shape: {error}"))?;
    verify_manifest(&manifest, &signature_bytes, &canonical, config, &latest_id)?;
    let platform_key = platform_key(&config.target, config.channel, config.linux_package_format)?;
    let platform = manifest
        .platforms
        .get(platform_key)
        .ok_or_else(|| format!("This release has no asset for {platform_key}."))?;
    verify_platform(platform, &config.target, platform_key)?;
    let asset = platform
        .assets
        .first()
        .ok_or_else(|| "This release has no downloadable asset.".to_string())?;

    let total_metadata = manifest_bytes
        .len()
        .checked_add(signature_bytes.len())
        .and_then(|value| value.checked_add(release_notes.len()))
        .ok_or_else(|| "Update download size overflowed.".to_string())?
        as u64;
    let total = total_metadata
        .checked_add(asset.bytes)
        .ok_or_else(|| "Update download size overflowed.".to_string())?;
    if total > TOTAL_DOWNLOAD_MAX_BYTES {
        return Err("The signed update set exceeds the total download limit.".to_string());
    }

    let updates_directory = data_directory.join("updates");
    fs::create_dir_all(&updates_directory)
        .map_err(|error| format!("Could not create the update directory: {error}"))?;
    let staged_directory = updates_directory.join(&manifest.release_id);
    if staged_directory.exists() {
        return Err("This release is already staged; it will not be overwritten.".to_string());
    }
    let temporary_directory = updates_directory.join(format!(".{}.staging", manifest.release_id));
    if temporary_directory.exists() {
        fs::remove_dir_all(&temporary_directory)
            .map_err(|error| format!("Could not discard the previous staging attempt: {error}"))?;
    }
    fs::create_dir(&temporary_directory)
        .map_err(|error| format!("Could not create the temporary staging directory: {error}"))?;

    let result = stage_release(
        &agent,
        &base,
        &temporary_directory,
        asset,
        &manifest_bytes,
        &signature_bytes,
        &release_notes,
    );
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&temporary_directory);
        return Err(error);
    }
    fs::rename(&temporary_directory, &staged_directory)
        .map_err(|error| format!("Could not finalize the staged update: {error}"))?;

    Ok(UpdateResult {
        state: "staged",
        release_id: Some(manifest.release_id),
        release_notes: String::from_utf8_lossy(&release_notes).into_owned(),
        install_mode: Some(platform.install_mode.clone()),
        target: Some(platform.target.clone()),
        artifact_filename: Some(asset.filename.clone()),
        staged_directory: Some(staged_directory),
        detail: "Release verified and staged. Installation still requires explicit approval."
            .to_string(),
    })
}

/// Re-checks the exact staged release before a local installer is launched.
/// This makes the second confirmation safe even if the staging directory has
/// been modified or partially written since the check operation returned.
pub fn verify_staged(
    config: &UpdateClientConfig,
    data_directory: &Path,
    release_id: &str,
) -> Result<StagedUpdate, String> {
    if !safe_version(release_id) {
        return Err("The staged release ID is unsafe.".to_string());
    }
    let staged_directory = data_directory.join("updates").join(release_id);
    let manifest_bytes = fs::read(staged_directory.join(MANIFEST_FILE))
        .map_err(|error| format!("Could not read the staged update manifest: {error}"))?;
    let signature_bytes = fs::read(staged_directory.join(SIGNATURE_FILE))
        .map_err(|error| format!("Could not read the staged update signature: {error}"))?;
    let notes = fs::read(staged_directory.join(RELEASE_NOTES_FILE))
        .map_err(|error| format!("Could not read the staged release notes: {error}"))?;
    if notes.len() as u64 > RELEASE_NOTES_MAX_BYTES {
        return Err("The staged release notes exceed the response limit.".to_string());
    }
    let manifest_value: serde_json::Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("The staged update manifest is invalid JSON: {error}"))?;
    let canonical = serde_json::to_vec(&manifest_value)
        .map_err(|error| format!("Could not canonicalize the staged manifest: {error}"))?;
    if canonical != manifest_bytes {
        return Err("The staged update manifest is not canonical.".to_string());
    }
    let manifest: ReleaseManifest = serde_json::from_value(manifest_value)
        .map_err(|error| format!("The staged update manifest has an invalid shape: {error}"))?;
    verify_manifest(&manifest, &signature_bytes, &canonical, config, release_id)?;
    let key = platform_key(&config.target, config.channel, config.linux_package_format)?;
    let platform = manifest
        .platforms
        .get(key)
        .ok_or_else(|| format!("The staged update has no asset for {key}."))?;
    verify_platform(platform, &config.target, key)?;
    let install_mode = platform.install_mode.clone();
    let asset_role = platform
        .assets
        .first()
        .expect("verify_platform checks one asset")
        .role
        .clone();
    let asset = platform
        .assets
        .first()
        .expect("verify_platform checks one asset");
    let artifact_path = staged_directory.join(&asset.filename);
    verify_local_artifact(&artifact_path, asset)?;
    Ok(StagedUpdate {
        manifest,
        install_mode,
        asset_role,
        artifact_path,
    })
}

fn stage_release(
    agent: &ureq::Agent,
    base: &str,
    temporary_directory: &Path,
    asset: &ReleaseAsset,
    manifest_bytes: &[u8],
    signature_bytes: &[u8],
    release_notes: &[u8],
) -> Result<(), String> {
    let artifact_path = temporary_directory.join(&asset.filename);
    download_to_file(
        agent,
        &format!("{base}/{}", asset.filename),
        &artifact_path,
        asset.bytes,
        &asset.sha256,
        &github_hosts(true),
        &asset.filename,
    )?;
    write_bytes(temporary_directory.join(MANIFEST_FILE), manifest_bytes)?;
    write_bytes(temporary_directory.join(SIGNATURE_FILE), signature_bytes)?;
    write_bytes(temporary_directory.join(RELEASE_NOTES_FILE), release_notes)?;
    Ok(())
}

fn verify_manifest(
    manifest: &ReleaseManifest,
    signature_bytes: &[u8],
    canonical: &[u8],
    config: &UpdateClientConfig,
    latest_id: &str,
) -> Result<(), String> {
    if manifest.schema_version != 1 || manifest.release_set != RELEASE_SET {
        return Err("This update manifest shape is not supported.".to_string());
    }
    if manifest.release_id != latest_id || manifest.tag != format!("v{latest_id}") {
        return Err("The signed manifest does not match the immutable release tag.".to_string());
    }
    if manifest.api.major != CURRENT_API_MAJOR
        || manifest.api.major != config.api_major
        || manifest.api.min_minor > manifest.api.max_minor
        || config.api_minor < manifest.api.min_minor
        || config.api_minor > manifest.api.max_minor
    {
        return Err("This release is outside the supported API compatibility range.".to_string());
    }
    if manifest.platforms.is_empty() {
        return Err("The signed update manifest contains no platform entries.".to_string());
    }
    let signature = decode_manifest_signature(signature_bytes)?;
    let key = VerifyingKey::from_bytes(&config.trusted_key)
        .map_err(|_| "The configured release-signing key is invalid.".to_string())?;
    key.verify(canonical, &signature)
        .map_err(|_| "Update manifest signature did not verify.".to_string())
}

fn verify_platform(
    platform: &PlatformRelease,
    target: &str,
    platform_key: &str,
) -> Result<(), String> {
    if platform.target != target {
        return Err("The signed update target does not match this machine.".to_string());
    }
    let expected_mode = if platform_key.starts_with("linux-desktop-") {
        "authorized-package-install"
    } else if platform_key.contains("-headless-") {
        "standalone-archive"
    } else {
        "tauri-coordinated"
    };
    if platform.install_mode != expected_mode {
        return Err(
            "The signed update installation mode is not valid for this platform.".to_string(),
        );
    }
    if platform.assets.len() != 1 {
        return Err("The selected platform must contain exactly one release asset.".to_string());
    }
    let asset = &platform.assets[0];
    let expected_role = if platform_key.ends_with("-deb-x86_64") {
        "package-deb"
    } else if platform_key.ends_with("-rpm-x86_64") {
        "package-rpm"
    } else if platform_key.contains("-headless-") {
        "archive"
    } else {
        "desktop"
    };
    if asset.role != expected_role
        || !safe_file_name(&asset.filename)
        || asset.bytes == 0
        || asset.bytes > ASSET_MAX_BYTES
        || !is_lowercase_sha256(&asset.sha256)
    {
        return Err("The signed update asset identity is invalid.".to_string());
    }
    if platform
        .forbidden_artifacts
        .iter()
        .any(|value| value == "sidecar")
        && platform
            .included_components
            .iter()
            .any(|value| value == "sidecar")
    {
        return Err("The signed platform both forbids and includes a sidecar.".to_string());
    }
    Ok(())
}

fn decode_manifest_signature(signature_bytes: &[u8]) -> Result<Signature, String> {
    // The release signer writes a final newline; it is file formatting, not signature data.
    let encoded = signature_bytes.trim_ascii();
    let decoded = BASE64
        .decode(encoded)
        .map_err(|_| "Update manifest signature has invalid base64.".to_string())?;
    Signature::from_slice(&decoded).map_err(|_| "Update manifest signature is invalid.".to_string())
}

fn latest_release(agent: &ureq::Agent, repository: &str) -> Result<GithubRelease, String> {
    let url = format!("https://api.github.com/repos/{repository}/releases?per_page=100");
    let bytes = download_bytes_with_accept(
        agent,
        &url,
        MANIFEST_MAX_BYTES,
        &github_hosts(false),
        "GitHub release",
        "application/vnd.github+json",
    )?;
    let releases: Vec<GithubRelease> = serde_json::from_slice(&bytes)
        .map_err(|error| format!("GitHub release metadata is invalid: {error}"))?;
    releases
        .into_iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            release_id_from_tag(&release.tag_name)
                .ok()
                .map(|release_id| (release_id, release))
        })
        .max_by(|(left_id, _), (right_id, _)| {
            compare_versions(left_id, right_id).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, release)| release)
        .ok_or_else(|| "GitHub has no published MSC release with a supported v tag.".to_string())
}

fn http_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .max_redirects(3)
        .max_redirects_will_error(true)
        .save_redirect_history(true)
        .https_only(true)
        .http_status_as_error(false)
        .build()
        .into()
}

fn download_bytes(
    agent: &ureq::Agent,
    url: &str,
    max_bytes: u64,
    allowed_hosts: &[&str],
    what: &str,
) -> Result<Vec<u8>, String> {
    download_bytes_with_accept(
        agent,
        url,
        max_bytes,
        allowed_hosts,
        what,
        "application/octet-stream",
    )
}

fn download_bytes_with_accept(
    agent: &ureq::Agent,
    url: &str,
    max_bytes: u64,
    allowed_hosts: &[&str],
    what: &str,
    accept: &str,
) -> Result<Vec<u8>, String> {
    let mut response = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", accept)
        .call()
        .map_err(|error| format!("Could not fetch {what}: {error}"))?;
    verify_response_url(&response, allowed_hosts, what)?;
    if !response.status().is_success() {
        return Err(format!(
            "Could not fetch {what}: HTTP {}",
            response.status()
        ));
    }
    response
        .body_mut()
        .with_config()
        .limit(max_bytes)
        .read_to_vec()
        .map_err(|error| format!("Could not read {what}: {error}"))
}

fn download_to_file(
    agent: &ureq::Agent,
    url: &str,
    destination: &Path,
    expected_bytes: u64,
    expected_sha256: &str,
    allowed_hosts: &[&str],
    what: &str,
) -> Result<(), String> {
    let mut response = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/octet-stream")
        .call()
        .map_err(|error| format!("Could not fetch {what}: {error}"))?;
    verify_response_url(&response, allowed_hosts, what)?;
    if !response.status().is_success() {
        return Err(format!(
            "Could not fetch {what}: HTTP {}",
            response.status()
        ));
    }
    let mut output = File::create(destination)
        .map_err(|error| format!("Could not create staged {what}: {error}"))?;
    let mut reader = response
        .body_mut()
        .with_config()
        .limit(ASSET_MAX_BYTES)
        .reader();
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut actual_bytes = 0_u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("Could not read {what}: {error}"))?;
        if read == 0 {
            break;
        }
        actual_bytes = actual_bytes
            .checked_add(read as u64)
            .ok_or_else(|| format!("{what} is too large."))?;
        if actual_bytes > expected_bytes || actual_bytes > ASSET_MAX_BYTES {
            return Err(format!("{what} exceeded its signed byte count."));
        }
        digest.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .map_err(|error| format!("Could not write staged {what}: {error}"))?;
    }
    if actual_bytes != expected_bytes || format!("{:x}", digest.finalize()) != expected_sha256 {
        return Err(format!(
            "{what} did not match its signed size or SHA-256 digest."
        ));
    }
    output
        .flush()
        .map_err(|error| format!("Could not flush staged {what}: {error}"))
}

fn verify_local_artifact(path: &Path, asset: &ReleaseAsset) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Could not inspect staged {}: {error}", asset.filename))?;
    if metadata.len() != asset.bytes {
        return Err(format!(
            "Staged {} has the wrong byte count.",
            asset.filename
        ));
    }
    let mut file = File::open(path)
        .map_err(|error| format!("Could not read staged {}: {error}", asset.filename))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Could not hash staged {}: {error}", asset.filename))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    if format!("{:x}", digest.finalize()) != asset.sha256 {
        return Err(format!(
            "Staged {} has the wrong SHA-256 digest.",
            asset.filename
        ));
    }
    Ok(())
}

fn verify_response_url(
    response: &ureq::http::Response<ureq::Body>,
    allowed_hosts: &[&str],
    what: &str,
) -> Result<(), String> {
    let history = response
        .get_redirect_history()
        .ok_or_else(|| format!("Could not verify the GitHub redirect chain for {what}."))?;
    for uri in history {
        let uri = uri.to_string();
        let host = uri
            .strip_prefix("https://")
            .and_then(|value| value.split('/').next())
            .and_then(|value| value.split(':').next())
            .unwrap_or_default();
        if !allowed_hosts.contains(&host) {
            return Err(format!("GitHub redirected {what} to an untrusted host."));
        }
    }
    Ok(())
}

fn github_hosts(allow_asset_cdn: bool) -> Vec<&'static str> {
    if allow_asset_cdn {
        vec![
            "github.com",
            "api.github.com",
            "release-assets.githubusercontent.com",
            "objects.githubusercontent.com",
        ]
    } else {
        vec!["api.github.com", "github.com"]
    }
}

fn release_base_url(repository: &str, release_id: &str) -> String {
    format!("https://github.com/{repository}/releases/download/v{release_id}")
}

fn platform_key(
    target: &str,
    channel: UpdateChannel,
    linux_package_format: Option<LinuxPackageFormat>,
) -> Result<&'static str, String> {
    match (std::env::consts::OS, target, channel) {
        ("macos", "x86_64-apple-darwin", UpdateChannel::Desktop) => Ok("macos-desktop-x86_64"),
        ("macos", "x86_64-apple-darwin", UpdateChannel::Headless) => Ok("macos-headless-x86_64"),
        ("macos", "aarch64-apple-darwin", UpdateChannel::Desktop) => Ok("macos-desktop-aarch64"),
        ("macos", "aarch64-apple-darwin", UpdateChannel::Headless) => Ok("macos-headless-aarch64"),
        ("windows", "x86_64-pc-windows-msvc", UpdateChannel::Desktop) => {
            Ok("windows-desktop-x86_64")
        }
        ("windows", "x86_64-pc-windows-msvc", UpdateChannel::Headless) => {
            Ok("windows-headless-x86_64")
        }
        ("linux", "x86_64-unknown-linux-gnu", UpdateChannel::Desktop) => match linux_package_format
        {
            Some(LinuxPackageFormat::Deb) => Ok("linux-desktop-deb-x86_64"),
            Some(LinuxPackageFormat::Rpm) => Ok("linux-desktop-rpm-x86_64"),
            None => Err(
                "Could not determine whether this Linux desktop uses DEB or RPM packages."
                    .to_string(),
            ),
        },
        ("linux", "x86_64-unknown-linux-gnu", UpdateChannel::Headless) => {
            Ok("linux-headless-x86_64")
        }
        ("linux", "x86_64-unknown-linux-gnu", UpdateChannel::LinuxPackageDeb) => {
            Ok("linux-desktop-deb-x86_64")
        }
        ("linux", "x86_64-unknown-linux-gnu", UpdateChannel::LinuxPackageRpm) => {
            Ok("linux-desktop-rpm-x86_64")
        }
        _ => Err("This platform is not supported by the MSC updater.".to_string()),
    }
}

/// Extracts a verified standalone headless archive into a fresh directory.
///
/// The archive has already passed the signed byte-count and SHA-256 checks.
/// We still validate every path and reject links because archive contents are
/// an installation input, not trusted merely because the container is signed.
pub fn extract_standalone_archive(archive_path: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        return Err(format!(
            "The temporary update directory already exists: {}",
            destination.display()
        ));
    }
    fs::create_dir_all(destination)
        .map_err(|error| format!("Could not create the update extraction directory: {error}"))?;

    let result = if archive_path.extension().and_then(|value| value.to_str()) == Some("zip") {
        crate::archive::extract_zip(archive_path, destination)
            .map_err(|error| format!("Could not extract the Windows headless archive: {error}"))
    } else {
        extract_tar_gz(archive_path, destination)
    };
    if let Err(error) = result {
        let _ = fs::remove_dir_all(destination);
        return Err(error);
    }
    Ok(())
}

fn extract_tar_gz(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|error| format!("Could not open the macOS/Linux headless archive: {error}"))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|error| format!("Could not read the headless archive: {error}"))?;
    for entry in entries {
        let mut entry = entry.map_err(|error| format!("Could not read archive entry: {error}"))?;
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err(
                "The headless archive contains an unsupported link or special file.".into(),
            );
        }
        let relative = entry
            .path()
            .map_err(|error| format!("Could not read archive path: {error}"))?;
        validate_archive_relative_path(&relative)?;
        let destination_path = destination.join(&relative);
        if entry_type.is_dir() {
            fs::create_dir_all(&destination_path)
                .map_err(|error| format!("Could not create archive directory: {error}"))?;
        } else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("Could not create archive parent: {error}"))?;
            }
            let mut output = File::create(&destination_path)
                .map_err(|error| format!("Could not create extracted archive file: {error}"))?;
            std::io::copy(&mut entry, &mut output)
                .map_err(|error| format!("Could not extract archive file: {error}"))?;
            #[cfg(unix)]
            if let Ok(mode) = entry.header().mode() {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&destination_path, fs::Permissions::from_mode(mode))
                    .map_err(|error| format!("Could not preserve archive permissions: {error}"))?;
            }
        }
    }
    Ok(())
}

fn validate_archive_relative_path(path: &Path) -> Result<(), String> {
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "The headless archive contains an unsafe path: {}",
            path.display()
        ));
    }
    Ok(())
}

fn validate_repository(repository: &str) -> Result<(), String> {
    let mut pieces = repository.split('/');
    let owner = pieces.next().unwrap_or_default();
    let name = pieces.next().unwrap_or_default();
    if pieces.next().is_some()
        || owner.is_empty()
        || name.is_empty()
        || !owner.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
        || !name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err("The configured GitHub repository is invalid.".to_string());
    }
    Ok(())
}

fn release_id_from_tag(tag: &str) -> Result<String, String> {
    let release_id = tag
        .strip_prefix('v')
        .ok_or_else(|| "The latest GitHub release has no supported v tag.".to_string())?;
    if !safe_version(release_id) {
        return Err("The latest GitHub release tag is unsafe.".to_string());
    }
    Ok(release_id.to_string())
}

fn compare_versions(left: &str, right: &str) -> Result<std::cmp::Ordering, String> {
    let left = parse_version(left)?;
    let right = parse_version(right)?;
    Ok(left.cmp(&right))
}

fn parse_version(value: &str) -> Result<(u64, u64, u64, bool, String), String> {
    if !safe_version(value) {
        return Err("The release version is not valid SemVer.".to_string());
    }
    let value_without_build = value.split_once('+').map_or(value, |(value, _)| value);
    let (core, prerelease) = value_without_build
        .split_once('-')
        .unwrap_or((value_without_build, ""));
    let mut parts = core.split('.');
    let major = parts.next().and_then(|part| part.parse().ok());
    let minor = parts.next().and_then(|part| part.parse().ok());
    let patch = parts.next().and_then(|part| part.parse().ok());
    if parts.next().is_some() || major.is_none() || minor.is_none() || patch.is_none() {
        return Err("The release version is not valid SemVer.".to_string());
    }
    Ok((
        major.unwrap(),
        minor.unwrap(),
        patch.unwrap(),
        prerelease.is_empty(),
        prerelease.to_string(),
    ))
}

fn safe_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '+')
        })
}

fn safe_file_name(value: &str) -> bool {
    let path = Path::new(value);
    path.components().count() == 1
        && matches!(path.components().next(), Some(Component::Normal(_)))
        && !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | '+')
        })
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn write_bytes(path: PathBuf, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes)
        .map_err(|error| format!("Could not write staged update metadata: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{BASE64, decode_manifest_signature};
    use base64::Engine as _;

    #[test]
    fn accepts_signer_signature_file_with_trailing_newline() {
        let encoded = BASE64.encode([0_u8; 64]);
        let signature_file = format!("{encoded}\n");

        assert!(decode_manifest_signature(signature_file.as_bytes()).is_ok());
    }
}
