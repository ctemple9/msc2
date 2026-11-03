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

/// Try real format discovery rather than accepting a launcher merely because it exists.
pub fn select_runtime_with_formats(
    candidates: Vec<String>,
    mut query: impl FnMut(&str) -> Result<Vec<String>, String>,
) -> Result<(String, Vec<String>), String> {
    let mut failures = Vec::new();
    for candidate in candidates {
        match query(&candidate) {
            Ok(formats) if !formats.is_empty() => return Ok((candidate, formats)),
            Ok(_) => failures.push(format!("{candidate}: no conversion formats returned")),
            Err(error) => failures.push(error),
        }
    }
    Err(format!(
        "MSC could not run Chunker with the available Java runtimes. Chunker requires Java 17 or newer. Install or repair Java in MSC's Java settings.\n{}",
        failures.join("\n")
    ))
}

pub fn jar_path() -> PathBuf {
    std::env::var_os("MSC2_CHUNKER_JAR_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::config_repository::default_app_data_dir().join("chunker-cli.jar"))
}

/// Move only Chunker's own cache files; the legacy directory may hold unrelated state.
pub fn migrate_legacy_installation() -> Result<(), ChunkerError> {
    static MIGRATION: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = MIGRATION
        .lock()
        .map_err(|_| ChunkerError::Filesystem("Chunker migration lock failed".into()))?;
    if std::env::var_os("MSC2_CHUNKER_JAR_PATH").is_some() {
        return Ok(());
    }
    let old = legacy_app_support_directory().join("chunker-cli.jar");
    let new = jar_path();
    if old == new || new.exists() || !old.is_file() {
        return Ok(());
    }
    migrate_cache(&crate::fs::StdFileSystem, &old, &new)?;
    if let Some(parent) = old.parent() {
        // remove_dir only removes an empty directory, never other application data.
        let _ = std::fs::remove_dir(parent);
    }
    Ok(())
}

fn migrate_cache(fs: &dyn FileSystem, old: &Path, new: &Path) -> Result<(), ChunkerError> {
    let error =
        |error: std::io::Error| ChunkerError::Filesystem(format!("migrate Chunker cache: {error}"));
    let bytes = fs.read(old).map_err(error)?;
    validate_jar(&bytes)?;
    let old_metadata = old.with_file_name("chunker-cli.jar.metadata.json");
    let new_metadata = new.with_file_name("chunker-cli.jar.metadata.json");
    let metadata = match fs.read(&old_metadata) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(cause) => return Err(error(cause)),
    };
    fs.create_dir_all(
        new.parent()
            .ok_or_else(|| ChunkerError::Filesystem("Chunker cache has no parent".into()))?,
    )
    .map_err(error)?;
    if let Some(metadata) = &metadata {
        atomic_write(fs, &new_metadata, metadata)
            .map_err(|cause| ChunkerError::Filesystem(cause.to_string()))?;
    }
    atomic_write(fs, new, &bytes).map_err(|cause| ChunkerError::Filesystem(cause.to_string()))?;
    if fs.read(new).map_err(error)? != bytes {
        return Err(ChunkerError::Filesystem(
            "Migrated Chunker JAR readback mismatch; original retained".into(),
        ));
    }
    if let Some(metadata) = &metadata {
        if fs.read(&new_metadata).map_err(error)? != *metadata {
            return Err(ChunkerError::Filesystem(
                "Migrated Chunker metadata readback mismatch; original retained".into(),
            ));
        }
        fs.remove(&old_metadata).map_err(error)?;
    }
    fs.remove(old).map_err(error)?;
    Ok(())
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
    let release = latest_release(transport)?;
    let asset = jar_asset(&release)?;

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
        version: release.tag_name.clone(),
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

fn legacy_app_support_directory() -> PathBuf {
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

/// Read official release metadata without downloading or replacing the installed JAR.
pub fn latest_version(transport: &dyn Transport) -> Result<String, ChunkerError> {
    let release = latest_release(transport)?;
    jar_asset(&release)?;
    Ok(release.tag_name)
}

fn latest_release(transport: &dyn Transport) -> Result<ReleaseMetadata, ChunkerError> {
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
    Ok(release)
}

fn jar_asset(release: &ReleaseMetadata) -> Result<&ReleaseAsset, ChunkerError> {
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

    Ok(asset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::FakeFileSystem;
    use std::io::Write;

    #[test]
    fn runtime_selection_skips_broken_launchers_and_preserves_failure_details() {
        let candidates = vec![
            "managed-java".into(),
            "system-stub".into(),
            "working-java".into(),
        ];
        let chosen = select_runtime_with_formats(candidates.clone(), |path| match path {
            "managed-java" => Ok(Vec::new()),
            "system-stub" => Err("Unable to locate a Java Runtime".into()),
            _ => Ok(vec!["BEDROCK_1_21_0".into()]),
        })
        .unwrap();
        assert_eq!(chosen.0, "working-java");
        assert_eq!(chosen.1, vec!["BEDROCK_1_21_0"]);
        let failure =
            select_runtime_with_formats(candidates, |_| Err("UnsupportedClassVersionError".into()))
                .unwrap_err();
        assert!(failure.contains("UnsupportedClassVersionError"));
        assert!(failure.contains("Java 17"));
    }

    #[test]
    fn cache_migration_preserves_metadata_and_originals_on_failed_promotion() {
        let old = Path::new("/old/chunker-cli.jar");
        let new = Path::new("/data/chunker-cli.jar");
        let old_meta = Path::new("/old/chunker-cli.jar.metadata.json");
        let new_meta = Path::new("/data/chunker-cli.jar.metadata.json");
        let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        archive
            .start_file(
                "META-INF/MANIFEST.MF",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        archive.write_all(b"Manifest-Version: 1.0\n").unwrap();
        let jar = archive.finish().unwrap().into_inner();
        let metadata = br#"{"version":"example","sha256":"example"}"#;
        let fs = FakeFileSystem::new()
            .with_file(old, jar.clone(), false)
            .with_file(old_meta, metadata.to_vec(), false)
            .with_file("/old/unrelated.json", b"keep".to_vec(), false)
            .with_failing_rename(new);
        assert!(migrate_cache(&fs, old, new).is_err());
        assert_eq!(fs.read(old).unwrap(), jar);
        assert_eq!(fs.read(old_meta).unwrap(), metadata);
        migrate_cache(&fs, old, new).unwrap();
        assert_eq!(fs.read(new).unwrap(), jar);
        assert_eq!(fs.read(new_meta).unwrap(), metadata);
        assert!(fs.stat(old).is_err());
        assert!(fs.stat(old_meta).is_err());
        assert_eq!(fs.read(Path::new("/old/unrelated.json")).unwrap(), b"keep");
    }
}

/// Runs conversion with bounded logs, a memory budget and disk monitoring.
pub fn run_conversion(
    jar_path: &Path,
    input_dir: &Path,
    output_dir: &Path,
    target_format: &str,
    java_path: &str,
    progress: &mut dyn FnMut(&str),
) -> Result<(), String> {
    use std::io::BufReader;
    use std::process::{Command, Stdio};

    let heap = crate::conversion_resources::available_heap_mib()?;
    progress(&format!("Chunker memory budget: {heap} MiB Java heap"));
    let mut child = Command::new(java_path)
        .arg("-Xms128m")
        .arg(format!("-Xmx{heap}m"))
        .arg("-jar")
        .arg(jar_path)
        .arg("-i")
        .arg(input_dir)
        .arg("-f")
        .arg(target_format)
        .arg("-o")
        .arg(output_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to start Chunker: {error}"))?;

    // Drain both pipes concurrently: a verbose stderr must not block Java
    // while MSC waits for stdout. Retain only a bounded diagnostic tail.
    let stderr = child.stderr.take().expect("piped stderr");
    let errors = std::thread::spawn(move || {
        use std::io::Read;
        let mut tail = std::collections::VecDeque::with_capacity(8192);
        let mut reader = stderr;
        let mut buffer = [0_u8; 4096];
        while let Ok(count) = reader.read(&mut buffer) {
            if count == 0 {
                break;
            }
            for byte in &buffer[..count] {
                if tail.len() == 8192 {
                    tail.pop_front();
                }
                tail.push_back(*byte);
            }
        }
        String::from_utf8_lossy(&tail.into_iter().collect::<Vec<_>>()).into_owned()
    });
    let stdout = child.stdout.take().expect("piped stdout");
    let (sender, receiver) = std::sync::mpsc::sync_channel(32);
    let output = std::thread::spawn(move || {
        use std::io::Read;
        let mut line = Vec::new();
        for byte in BufReader::new(stdout).bytes().map_while(Result::ok) {
            if byte == b'\n' || line.len() >= 8192 {
                if sender
                    .send(String::from_utf8_lossy(&line).into_owned())
                    .is_err()
                {
                    return;
                }
                line.clear();
            }
            if byte != b'\n' {
                line.push(byte);
            }
        }
        if !line.is_empty() {
            let _ = sender.send(String::from_utf8_lossy(&line).into_owned());
        }
    });
    let mut resource_error = None;
    let mut next_disk_check = std::time::Instant::now();
    loop {
        if std::time::Instant::now() >= next_disk_check {
            if let Err(error) = crate::conversion_resources::require_space(
                output_dir,
                0,
                "running Chunker conversion",
            ) {
                let _ = child.kill();
                resource_error = Some(error.to_string());
                break;
            }
            next_disk_check = std::time::Instant::now() + std::time::Duration::from_secs(1);
        }
        match receiver.recv_timeout(std::time::Duration::from_millis(250)) {
            Ok(line) => {
                if !line.trim().is_empty() {
                    progress(line.trim());
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                Err(error) => {
                    let _ = child.kill();
                    resource_error = Some(error.to_string());
                    break;
                }
            },
        }
    }
    drop(receiver);
    let _ = output.join();
    let diagnostics = errors.join().unwrap_or_default();

    let status = child
        .wait()
        .map_err(|error| format!("Chunker process error: {error}"))?;
    if let Some(error) = resource_error {
        return Err(error);
    }
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Chunker exited with code {}: {}",
            status.code().unwrap_or(-1),
            diagnostics.trim()
        ))
    }
}
