//! Geyser's small, deliberately non-destructive configuration surface.
//!
//! Geyser owns its YAML.  MSC therefore finds the first top-level `bedrock`
//! block and changes only existing `address`/`port` lines inside it.  This
//! preserves comments and unfamiliar Geyser settings rather than rebuilding
//! the file from a partial model.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use msc_infrastructure::{
    atomic_write::atomic_write,
    download_staging::sha256_hex,
    fs::FileSystem,
    geyser::{self as geyser_provider, GeyserAcquisitionError, GeyserBuild, GeyserProject},
    helper_acquisition::{AcquiredHelper, HelperArtifactMetadata, HelperPlatform},
    jar_provider::Transport,
};
use zip::ZipArchive;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeyserConfig {
    pub address: String,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossPlayInstallation {
    pub geyser_installed: bool,
    pub floodgate_installed: bool,
    pub geyser_path: Option<PathBuf>,
    pub floodgate_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedPluginInstallation {
    pub build: GeyserBuild,
    pub plugin_path: PathBuf,
    pub acquired: AcquiredHelper,
}

/// Version information read from a plugin's own descriptor inside its JAR.
/// This is intentionally independent from the download resolver: users can
/// also place a plugin JAR in `plugins/` manually, and Components should show
/// the version that is actually installed in that case too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledPluginVersion {
    pub version: String,
    pub build: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallError {
    Acquisition(String),
    Filesystem(String),
    InvalidConfiguration(String),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Acquisition(message) => write!(f, "Geyser acquisition failed: {message}"),
            Self::Filesystem(message) => write!(f, "Geyser installation failed: {message}"),
            Self::InvalidConfiguration(message) => {
                write!(f, "Geyser configuration is invalid: {message}")
            }
        }
    }
}

impl std::error::Error for InstallError {}

/// Resolves and installs one latest Paper-family cross-play plugin.
///
/// The JAR is acquired and checksum-verified before this function touches
/// the server's `plugins/` directory. If an existing Geyser configuration is
/// present, it must still be parseable; a failed validation restores the
/// previous JAR so an update never leaves a known-good installation disabled.
pub fn install_latest(
    fs: &dyn FileSystem,
    transport: &dyn Transport,
    cache_directory: &Path,
    server_dir: &Path,
    project: GeyserProject,
    platform: HelperPlatform,
) -> Result<ManagedPluginInstallation, InstallError> {
    let (build, acquired) =
        geyser_provider::acquire_latest(transport, fs, cache_directory, project, platform)
            .map_err(map_acquisition_error)?;
    let plugins_dir = server_dir.join("plugins");
    if !fs
        .stat(&plugins_dir)
        .map(|metadata| metadata.is_dir)
        .unwrap_or(false)
    {
        return Err(InstallError::Filesystem(format!(
            "plugin directory {} does not exist",
            plugins_dir.display()
        )));
    }

    let current = installation(fs, server_dir);
    let plugin_path = match project {
        GeyserProject::Geyser => current
            .geyser_path
            .unwrap_or_else(|| plugins_dir.join(project.jar_name())),
        GeyserProject::Floodgate => current
            .floodgate_path
            .unwrap_or_else(|| plugins_dir.join(project.jar_name())),
    };
    let previous =
        if fs.stat(&plugin_path).is_ok() {
            Some(fs.read(&plugin_path).map_err(|error| {
                InstallError::Filesystem(format!("read existing plugin: {error}"))
            })?)
        } else {
            None
        };

    let config_was_present = matches!(project, GeyserProject::Geyser)
        && fs
            .stat(&config_path(server_dir))
            .map(|metadata| metadata.is_file)
            .unwrap_or(false);
    if config_was_present && read_config(fs, server_dir).is_none() {
        return Err(InstallError::InvalidConfiguration(
            "existing config.yml cannot be parsed".into(),
        ));
    }

    let bytes = fs.read(&acquired.artifact.path).map_err(|error| {
        InstallError::Filesystem(format!("read verified helper artifact: {error}"))
    })?;
    atomic_write(fs, &plugin_path, &bytes)
        .map_err(|error| InstallError::Filesystem(error.to_string()))?;

    let valid = match project {
        GeyserProject::Geyser => !config_was_present || read_config(fs, server_dir).is_some(),
        GeyserProject::Floodgate => installation(fs, server_dir).floodgate_installed,
    };
    if !valid {
        restore_plugin(fs, &plugin_path, previous.as_deref())?;
        return Err(InstallError::InvalidConfiguration(
            "configuration validation failed after staging the new plugin".into(),
        ));
    }

    Ok(ManagedPluginInstallation {
        build,
        plugin_path,
        acquired,
    })
}

/// Installs the complete Paper-family cross-play pair. GeyserMC publishes
/// separate release streams for the two plugins, so each artifact is resolved
/// and checksum-verified independently before creation reports success.
pub fn install_latest_pair(
    fs: &dyn FileSystem,
    transport: &dyn Transport,
    cache_directory: &Path,
    server_dir: &Path,
    platform: HelperPlatform,
) -> Result<(ManagedPluginInstallation, ManagedPluginInstallation), InstallError> {
    let geyser = install_latest(
        fs,
        transport,
        cache_directory,
        server_dir,
        GeyserProject::Geyser,
        platform,
    )?;
    let floodgate = install_latest(
        fs,
        transport,
        cache_directory,
        server_dir,
        GeyserProject::Floodgate,
        platform,
    )?;
    Ok((geyser, floodgate))
}

/// Creates or updates the small part of Geyser's configuration that must be
/// ready before the first server start. Geyser fills in the rest of its
/// defaults on startup; MSC owns the Bedrock listener and Floodgate auth mode.
pub fn configure_for_floodgate(
    fs: &dyn FileSystem,
    server_dir: &Path,
    java_port: u16,
    bedrock_port: u16,
) -> Result<GeyserConfig, InstallError> {
    let path = config_path(server_dir);
    if fs
        .stat(&path)
        .map(|metadata| metadata.is_file)
        .unwrap_or(false)
    {
        update_config(fs, server_dir, None, Some(i64::from(bedrock_port)))
            .map_err(InstallError::InvalidConfiguration)?;
        let original = String::from_utf8(
            fs.read(&path)
                .map_err(|error| InstallError::Filesystem(error.to_string()))?,
        )
        .map_err(|_| InstallError::InvalidConfiguration("config.yml is not UTF-8".into()))?;
        let patched = patch_floodgate_auth_type(&original)?;
        if patched != original {
            atomic_write(fs, &path, patched.as_bytes())
                .map_err(|error| InstallError::Filesystem(error.to_string()))?;
        }
    } else {
        let parent = path
            .parent()
            .ok_or_else(|| InstallError::Filesystem("Geyser config has no parent".into()))?;
        fs.create_dir_all(parent)
            .map_err(|error| InstallError::Filesystem(error.to_string()))?;
        let contents = format!(
            "bedrock:\n  address: 0.0.0.0\n  port: {bedrock_port}\nremote:\n  address: 127.0.0.1\n  port: {java_port}\n  auth-type: floodgate\n"
        );
        atomic_write(fs, &path, contents.as_bytes())
            .map_err(|error| InstallError::Filesystem(error.to_string()))?;
    }
    read_config(fs, server_dir)
        .ok_or_else(|| InstallError::InvalidConfiguration("no Bedrock listener block".into()))
}

fn map_acquisition_error(error: GeyserAcquisitionError) -> InstallError {
    InstallError::Acquisition(error.to_string())
}

fn restore_plugin(
    fs: &dyn FileSystem,
    path: &Path,
    previous: Option<&[u8]>,
) -> Result<(), InstallError> {
    match previous {
        Some(bytes) => atomic_write(fs, path, bytes)
            .map_err(|error| InstallError::Filesystem(error.to_string())),
        None => fs.remove(path).map_err(|error| {
            InstallError::Filesystem(format!("remove failed plugin replacement: {error}"))
        }),
    }
}

pub fn installation(fs: &dyn FileSystem, server_dir: &Path) -> CrossPlayInstallation {
    let plugins = server_dir.join("plugins");
    let mut result = CrossPlayInstallation {
        geyser_installed: false,
        floodgate_installed: false,
        geyser_path: None,
        floodgate_path: None,
    };
    let Ok(entries) = fs.list(&plugins) else {
        return result;
    };
    for path in entries {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if !lower.ends_with(".jar") || !fs.stat(&path).map(|meta| meta.is_file).unwrap_or(false) {
            continue;
        }
        if lower.contains("geyser") {
            result.geyser_installed = true;
            result.geyser_path.get_or_insert(path.clone());
        }
        if lower.contains("floodgate") {
            result.floodgate_installed = true;
            result.floodgate_path.get_or_insert(path);
        }
    }
    result
}

/// Reads the installed version from the standard Paper/Bukkit plugin
/// descriptor inside a plugin JAR. Malformed or non-JAR files return `None`,
/// allowing callers to keep their existing "installed" fallback.
pub fn installed_plugin_version(
    fs: &dyn FileSystem,
    plugin_path: &Path,
) -> Option<InstalledPluginVersion> {
    let bytes = fs.read(plugin_path).ok()?;
    let installed_hash = sha256_hex(&bytes);
    let mut archive = ZipArchive::new(Cursor::new(bytes)).ok()?;
    for descriptor_name in ["plugin.yml", "paper-plugin.yml"] {
        let Ok(mut descriptor) = archive.by_name(descriptor_name) else {
            continue;
        };
        let mut contents = String::new();
        descriptor.read_to_string(&mut contents).ok()?;
        if let Some(version) = descriptor_version(&contents) {
            return Some(InstalledPluginVersion {
                build: descriptor_build(&version)
                    .or_else(|| cached_build(fs, plugin_path, &installed_hash)),
                version,
            });
        }
    }
    None
}

fn cached_build(fs: &dyn FileSystem, plugin_path: &Path, installed_hash: &str) -> Option<i64> {
    let filename = plugin_path.file_name()?.to_str()?;
    let lower = filename.to_ascii_lowercase();
    let helper = if lower.contains("floodgate") {
        "floodgate"
    } else if lower.contains("geyser") {
        "geyser"
    } else {
        return None;
    };
    let cache_dir = plugin_path.ancestors().find_map(|ancestor| {
        let candidate = ancestor.join("_addon_cache").join(helper);
        fs.stat(&candidate)
            .ok()
            .filter(|metadata| metadata.is_dir)
            .map(|_| candidate)
    })?;
    for release_dir in fs.list(&cache_dir).ok()? {
        if !fs
            .stat(&release_dir)
            .map(|metadata| metadata.is_dir)
            .unwrap_or(false)
        {
            continue;
        }
        let metadata_path = release_dir.join(format!("{filename}.metadata.json"));
        let Ok(metadata_bytes) = fs.read(&metadata_path) else {
            continue;
        };
        let Ok(metadata) = serde_json::from_slice::<HelperArtifactMetadata>(&metadata_bytes) else {
            continue;
        };
        if metadata.sha256 != installed_hash {
            continue;
        }
        let (_, build) = metadata.version.rsplit_once("-build-")?;
        return build.parse().ok();
    }
    None
}

fn descriptor_version(contents: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        if line.starts_with([' ', '\t']) {
            return None;
        }
        let value = line.strip_prefix("version:")?;
        let value = bare_value(value);
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn descriptor_build(version: &str) -> Option<i64> {
    version.match_indices('b').rev().find_map(|(index, _)| {
        let preceding = version[..index].chars().next_back()?;
        if !matches!(preceding, '-' | '(' | ' ') {
            return None;
        }
        let digits = version[index + 1..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>();
        (!digits.is_empty()).then(|| digits.parse().ok()).flatten()
    })
}

pub fn config_path(server_dir: &Path) -> PathBuf {
    server_dir
        .join("plugins")
        .join("Geyser-Spigot")
        .join("config.yml")
}

pub fn read_config(fs: &dyn FileSystem, server_dir: &Path) -> Option<GeyserConfig> {
    let text = String::from_utf8(fs.read(&config_path(server_dir)).ok()?).ok()?;
    let (_, body) = top_level_bedrock_block(&text)?;
    let mut address = "0.0.0.0".to_string();
    let mut port = None;
    for line in body {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("address:") {
            let value = bare_value(value);
            if !value.is_empty() {
                address = value.to_string();
            }
        } else if let Some(value) = trimmed.strip_prefix("port:") {
            port = bare_value(value).parse().ok();
        }
    }
    Some(GeyserConfig { address, port })
}

pub fn update_config(
    fs: &dyn FileSystem,
    server_dir: &Path,
    address: Option<&str>,
    port: Option<i64>,
) -> Result<GeyserConfig, String> {
    let path = config_path(server_dir);
    let original = String::from_utf8(
        fs.read(&path)
            .map_err(|_| "Geyser configuration is unavailable.")?,
    )
    .map_err(|_| "Geyser configuration is not valid UTF-8.")?;
    let address = address.map(str::trim).filter(|value| !value.is_empty());
    if address.is_some_and(|value| value.contains(['\n', '\r'])) {
        return Err("Geyser address must be one line.".into());
    }
    let port = match port {
        Some(value @ 1..=65535) => Some(value as u16),
        Some(_) => return Err("Geyser port must be between 1 and 65535.".into()),
        None => None,
    };
    if address.is_none() && port.is_none() {
        return Err("Provide an address or port to change.".into());
    }
    let patched = patch_bedrock_block(&original, address, port)?;
    if patched != original {
        atomic_write(fs, &path, patched.as_bytes()).map_err(|error| error.to_string())?;
    }
    read_config(fs, server_dir)
        .ok_or_else(|| "Geyser configuration has no top-level bedrock block.".into())
}

fn top_level_bedrock_block(text: &str) -> Option<(usize, Vec<&str>)> {
    let lines: Vec<_> = text.lines().collect();
    let start = lines.iter().position(|line| {
        !line.starts_with([' ', '\t']) && line.trim_start().starts_with("bedrock:")
    })?;
    let end = lines[start + 1..]
        .iter()
        .position(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#') && !line.starts_with([' ', '\t'])
        })
        .map(|offset| start + 1 + offset)
        .unwrap_or(lines.len());
    Some((start, lines[start + 1..end].to_vec()))
}

fn patch_bedrock_block(
    text: &str,
    address: Option<&str>,
    port: Option<u16>,
) -> Result<String, String> {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let (start, _) = top_level_bedrock_block(text)
        .ok_or_else(|| "Geyser configuration has no top-level bedrock block.".to_string())?;
    let end = lines[start + 1..]
        .iter()
        .position(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#') && !line.starts_with([' ', '\t'])
        })
        .map(|offset| start + 1 + offset)
        .unwrap_or(lines.len());
    for line in &mut lines[start + 1..end] {
        let trimmed = line.trim_start();
        let indent = &line[..line.len() - trimmed.len()];
        let comment = trimmed
            .find('#')
            .map(|index| format!(" {}", &trimmed[index..]))
            .unwrap_or_default();
        if let Some(value) = address.filter(|_| trimmed.starts_with("address:")) {
            *line = format!("{indent}address: \"{value}\"{comment}");
        } else if let Some(value) = port.filter(|_| trimmed.starts_with("port:")) {
            *line = format!("{indent}port: {value}{comment}");
        }
    }
    Ok(lines.join("\n") + if text.ends_with('\n') { "\n" } else { "" })
}

fn patch_floodgate_auth_type(text: &str) -> Result<String, InstallError> {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let remote_start = lines
        .iter()
        .position(|line| !line.starts_with([' ', '\t']) && line.trim_start() == "remote:")
        .ok_or_else(|| InstallError::InvalidConfiguration("no top-level remote block".into()))?;
    let remote_end = lines[remote_start + 1..]
        .iter()
        .position(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#') && !line.starts_with([' ', '\t'])
        })
        .map(|offset| remote_start + 1 + offset)
        .unwrap_or(lines.len());
    for line in &mut lines[remote_start + 1..remote_end] {
        let trimmed = line.trim_start();
        if trimmed.starts_with("auth-type:") {
            let indent = &line[..line.len() - trimmed.len()];
            *line = format!("{indent}auth-type: floodgate");
            return Ok(lines.join("\n") + if text.ends_with('\n') { "\n" } else { "" });
        }
    }
    lines.insert(remote_start + 1, "  auth-type: floodgate".into());
    Ok(lines.join("\n") + if text.ends_with('\n') { "\n" } else { "" })
}

fn bare_value(raw: &str) -> &str {
    raw.split('#')
        .next()
        .unwrap_or(raw)
        .trim()
        .trim_matches(['\"', '\''])
}
