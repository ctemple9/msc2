//! Bedrock distribution resolution, verification, and safe promotion.
//!
//! The infrastructure layer owns manifest parsing and isolated ZIP staging.
//! This application layer owns server state: the offline/no-op behavior,
//! downgrade safety-backup gate, preservation of user files, and the final
//! directory swap. Keeping those responsibilities separate means Linux,
//! Windows, and the macOS sidecar can all consume one verified installation.

use msc_infrastructure::bedrock_distribution::{
    self, BedrockDistributionError, BedrockPlatform, BedrockVersionRequest,
};
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::jar_provider::{JarProviderError, Transport};
use std::fmt;
use std::path::Path;

pub const BEDROCK_MANIFEST_URL: &str = "https://raw.githubusercontent.com/kittizz/bedrock-server-downloads/refs/heads/main/bedrock-server-downloads.json";
const VERSION_MARKER: &str = ".msc_bds_version";
const PRESERVED_FILES: [&str; 4] = [
    "server.properties",
    "allowlist.json",
    "permissions.json",
    "whitelist.json",
];

#[derive(Debug, Clone)]
pub struct ProvisionRequest<'a> {
    pub server_dir: &'a Path,
    pub version: Option<&'a str>,
    pub platform: BedrockPlatform,
    pub force: bool,
    pub manifest_url: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvisionOutcome {
    NoOp { version: String },
    UsedInstalledFiles { version: Option<String> },
    RecordedLegacyMarker { version: String },
    Installed { version: String },
    Updated { from: Option<String>, to: String },
}

#[derive(Debug)]
pub enum BedrockProvisioningError {
    Download(JarProviderError),
    Distribution(BedrockDistributionError),
    Filesystem(String),
    DowngradeBackupRequired { from: String, to: String },
    DowngradeBackupFailed { from: String, to: String },
}

impl fmt::Display for BedrockProvisioningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Download(error) => write!(f, "Bedrock download failed: {error}"),
            Self::Distribution(error) => write!(f, "{error}"),
            Self::Filesystem(message) => write!(f, "Bedrock installation failed: {message}"),
            Self::DowngradeBackupRequired { from, to } => write!(
                f,
                "a safety backup is required before downgrading Bedrock from {from} to {to}"
            ),
            Self::DowngradeBackupFailed { from, to } => write!(
                f,
                "the safety backup failed before downgrading Bedrock from {from} to {to}"
            ),
        }
    }
}

impl std::error::Error for BedrockProvisioningError {}

/// Ensures one server directory contains a verified BDS distribution. The
/// backup closure is called only for a real downgrade and must complete before
/// the old directory is moved aside.
pub fn ensure_installed(
    fs: &dyn FileSystem,
    transport: &dyn Transport,
    request: &ProvisionRequest<'_>,
    pre_downgrade_backup: impl FnOnce() -> bool,
) -> Result<ProvisionOutcome, BedrockProvisioningError> {
    let executable = request.server_dir.join("bedrock_server");
    let already_installed = fs
        .stat(&executable)
        .map(|metadata| metadata.is_file && metadata.executable)
        .unwrap_or(false);
    let installed_version = read_marker(fs, request.server_dir);

    let manifest = match transport.get(
        request.manifest_url,
        "Bedrock version manifest",
        bedrock_distribution::BEDROCK_MANIFEST_MAX_BYTES,
    ) {
        Ok(manifest) => manifest,
        Err(_error) if already_installed && !request.force => {
            return Ok(ProvisionOutcome::UsedInstalledFiles {
                version: installed_version,
            });
        }
        Err(error) => return Err(BedrockProvisioningError::Download(error)),
    };

    let target = bedrock_distribution::resolve_release(
        &manifest,
        BedrockVersionRequest::parse(request.version),
        request.platform,
    )
    .map_err(BedrockProvisioningError::Distribution)?;

    if already_installed && !request.force {
        if installed_version.as_deref() == Some(target.version.as_str()) {
            return Ok(ProvisionOutcome::NoOp {
                version: target.version,
            });
        }
        if installed_version.is_none() && BedrockVersionRequest::parse(request.version).is_latest()
        {
            write_marker(fs, request.server_dir, &target.version)?;
            return Ok(ProvisionOutcome::RecordedLegacyMarker {
                version: target.version,
            });
        }
    }

    let old_version = installed_version.clone();
    if let Some(from) = installed_version.as_deref()
        && bedrock_distribution::requires_downgrade_backup(Some(from), &target.version)
        && !pre_downgrade_backup()
    {
        return Err(BedrockProvisioningError::DowngradeBackupFailed {
            from: from.to_string(),
            to: target.version,
        });
    }

    let bytes = transport
        .get(
            &target.url,
            "Bedrock server archive",
            bedrock_distribution::BEDROCK_ARCHIVE_MAX_BYTES,
        )
        .map_err(BedrockProvisioningError::Download)?;
    let staging_root = request.server_dir.join(".msc_bds_staging");
    let staged = bedrock_distribution::stage_archive(fs, &staging_root, &target, &bytes)
        .map_err(BedrockProvisioningError::Distribution)?;

    let result = promote(
        fs,
        request.server_dir,
        &staged.root,
        &target.version,
        already_installed,
    );
    let _ = fs.remove(&staging_root);
    result?;

    Ok(if already_installed {
        ProvisionOutcome::Updated {
            from: old_version,
            to: target.version,
        }
    } else {
        ProvisionOutcome::Installed {
            version: target.version,
        }
    })
}

fn read_marker(fs: &dyn FileSystem, server_dir: &Path) -> Option<String> {
    let bytes = fs.read(&server_dir.join(VERSION_MARKER)).ok()?;
    let value = String::from_utf8(bytes).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn write_marker(
    fs: &dyn FileSystem,
    server_dir: &Path,
    version: &str,
) -> Result<(), BedrockProvisioningError> {
    msc_infrastructure::atomic_write::atomic_write(
        fs,
        &server_dir.join(VERSION_MARKER),
        version.as_bytes(),
    )
    .map_err(|error| BedrockProvisioningError::Filesystem(error.to_string()))
}

fn promote(
    fs: &dyn FileSystem,
    server_dir: &Path,
    staged_root: &Path,
    version: &str,
    already_installed: bool,
) -> Result<(), BedrockProvisioningError> {
    let parent = server_dir.parent().ok_or_else(|| {
        BedrockProvisioningError::Filesystem("server directory has no parent".into())
    })?;
    fs.create_dir_all(parent)
        .map_err(|error| BedrockProvisioningError::Filesystem(error.to_string()))?;

    let name = server_dir
        .file_name()
        .ok_or_else(|| BedrockProvisioningError::Filesystem("server directory has no name".into()))?
        .to_string_lossy();
    let candidate = parent.join(format!(".{name}.bedrock-installing"));
    let backup = parent.join(format!(".{name}.bedrock-backup"));
    let _ = fs.remove(&candidate);
    let _ = fs.remove(&backup);
    fs.create_dir_all(&candidate)
        .map_err(|error| BedrockProvisioningError::Filesystem(error.to_string()))?;

    if already_installed {
        copy_tree(fs, server_dir, &candidate, Path::new(""), false)?;
    }
    copy_tree(
        fs,
        staged_root,
        &candidate,
        Path::new(""),
        already_installed,
    )?;
    write_marker(fs, &candidate, version)?;

    if !already_installed {
        fs.rename(&candidate, server_dir)
            .map_err(|error| BedrockProvisioningError::Filesystem(error.to_string()))?;
        return Ok(());
    }

    fs.rename(server_dir, &backup)
        .map_err(|error| BedrockProvisioningError::Filesystem(error.to_string()))?;
    if let Err(error) = fs.rename(&candidate, server_dir) {
        let _ = fs.rename(&backup, server_dir);
        return Err(BedrockProvisioningError::Filesystem(error.to_string()));
    }
    let _ = fs.remove(&backup);
    Ok(())
}

fn copy_tree(
    fs: &dyn FileSystem,
    source: &Path,
    destination: &Path,
    relative: &Path,
    preserve_user_files: bool,
) -> Result<(), BedrockProvisioningError> {
    for child in fs
        .list(&source.join(relative))
        .map_err(|error| BedrockProvisioningError::Filesystem(error.to_string()))?
    {
        let name = child
            .file_name()
            .ok_or_else(|| BedrockProvisioningError::Filesystem("entry has no name".into()))?;
        let child_relative = relative.join(name);
        if child_relative == Path::new(".msc_bds_staging") {
            continue;
        }
        let child_destination = destination.join(&child_relative);
        let metadata = fs
            .stat(&child)
            .map_err(|error| BedrockProvisioningError::Filesystem(error.to_string()))?;
        if metadata.is_dir {
            fs.create_dir_all(&child_destination)
                .map_err(|error| BedrockProvisioningError::Filesystem(error.to_string()))?;
            copy_tree(
                fs,
                source,
                destination,
                &child_relative,
                preserve_user_files,
            )?;
            continue;
        }
        if preserve_user_files
            && child_relative
                .parent()
                .is_some_and(|parent| parent.as_os_str().is_empty())
            && PRESERVED_FILES.contains(&name.to_string_lossy().as_ref())
        {
            continue;
        }
        if let Some(parent) = child_destination.parent() {
            fs.create_dir_all(parent)
                .map_err(|error| BedrockProvisioningError::Filesystem(error.to_string()))?;
        }
        let bytes = fs
            .read(&child)
            .map_err(|error| BedrockProvisioningError::Filesystem(error.to_string()))?;
        if metadata.executable {
            fs.write_executable(&child_destination, &bytes)
        } else {
            fs.write(&child_destination, &bytes)
        }
        .map_err(|error| BedrockProvisioningError::Filesystem(error.to_string()))?;
    }
    Ok(())
}
