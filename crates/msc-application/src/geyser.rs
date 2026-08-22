//! Geyser's small, deliberately non-destructive configuration surface.
//!
//! Geyser owns its YAML.  MSC therefore finds the first top-level `bedrock`
//! block and changes only existing `address`/`port` lines inside it.  This
//! preserves comments and unfamiliar Geyser settings rather than rebuilding
//! the file from a partial model.

use std::path::{Path, PathBuf};

use msc_infrastructure::{atomic_write::atomic_write, fs::FileSystem};

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

fn bare_value(raw: &str) -> &str {
    raw.split('#')
        .next()
        .unwrap_or(raw)
        .trim()
        .trim_matches(['\"', '\''])
}
