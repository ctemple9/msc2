//! Port of `AppViewModel+ServerTransfer.swift`'s `exportServerTransfer`
//! (P5.13), `inspectTransferPackage` (P5.14), and the build-stage half of
//! `applyTransferImport` (P5.15) — the per-server filesystem restoration
//! that source's `Task.detached` closure performs, returning the
//! equivalent of its `(newServers, imported, skipped)` tuple. Source's
//! `MainActor.run` commit stage — merging/replacing `configManager.config`,
//! choosing `activeServerId`, calling `KeychainManager.deleteAllMSCSecrets`,
//! and saving config — is deliberately not ported here: this crate has no
//! loaded `AppConfig` or credential store to act on (this module has no
//! dependency beyond `msc-domain`'s types plus `zip`/`serde_json`), and the
//! replace-all backup orchestration (P5.16, in `msc-agent`) is what owns
//! calling this function only after a safety backup succeeds. P5.16's
//! files list (`msc-api`/`msc-agent`, not this module) is where that
//! commit-stage work — and the transfer mode itself — belongs.
//!
//! Format pinned in `docs/msc2/config-migration/transfer-package-format.md`
//! from the 7 fixtures in `fixtures/transfer-package/` (P5.12): no MSC 1
//! test exercises any of `exportServerTransfer`/`inspectTransferPackage`/
//! `applyTransferImport`, so both functions here are ported straight from
//! source, the same precedent `config_recovery` (P5.7) and
//! `secret_migration` (P5.8) set.
//!
//! `TransferManifest`/`TransferServerEntry`/`TransferPluginLink` mirror
//! `ConfigServer`'s own hand-rolled `decode`/`encode` shape (P5.4) rather
//! than deriving `serde::Serialize` — this crate has no existing precedent
//! for derive-based (de)serialization, and the manifest wrapper's camelCase
//! keys sitting alongside the embedded `server` object's snake_case keys
//! (source: no `CodingKeys` override on the wrapper types, but `ConfigServer`
//! has its own) is easiest to get right with two visibly distinct encoders
//! side by side rather than one derive macro straddling both conventions.

use msc_domain::app_config_schema::{ConfigServer, DecodeError, PluginSourceConfig};
use msc_domain::identity::{JavaServerFlavor, ServerType};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// `ServerTransfer.formatVersion` — the only value this build ever writes,
/// and the ceiling `inspectTransferPackage` accepts on read (equal is
/// fine, strictly greater is rejected). See "formatVersion rejection rule"
/// in the format doc.
pub const FORMAT_VERSION: i64 = 2;

/// `ServerTransfer.configFileExtensions` (source line 47-49), lowercased.
const CONFIG_FILE_EXTENSIONS: &[&str] =
    &["properties", "yml", "yaml", "json", "txt", "toml", "conf"];

/// The wholesale-copy subdirectory list shared between export and apply
/// (source line 169). `libraries` is not here — it bundles separately,
/// gated by `java_flavor`, not by existence alone.
const WHOLESALE_SUBDIRS: &[&str] = &[
    "world_slots",
    "backups",
    "plugins",
    "mods",
    "resource-packs",
];

// ---------- Manifest types ----------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferPluginLink {
    pub filename: String,
    pub url: String,
    pub plugin_type: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferServerEntry {
    /// Sanitized — see [`sanitize_for_manifest`]. Never the live server
    /// record.
    pub server: ConfigServer,
    pub folder_name: String,
    pub java_port: Option<i64>,
    pub paper_mc_version: Option<String>,
    pub paper_build: Option<i64>,
    pub bundled_paper_jar: bool,
    pub plugin_links: Vec<TransferPluginLink>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferManifest {
    pub format_version: i64,
    pub app_config_version: i64,
    pub created_at: String,
    pub source_machine_name: String,
    pub servers: Vec<TransferServerEntry>,
}

impl TransferPluginLink {
    fn decode(v: &Value) -> Result<Self, DecodeError> {
        Ok(Self {
            filename: req_str(v, "filename")?,
            url: req_str(v, "url")?,
            plugin_type: req_str(v, "type")?,
        })
    }

    fn encode(&self) -> Value {
        let mut m = Map::new();
        m.insert("filename".into(), Value::String(self.filename.clone()));
        m.insert("url".into(), Value::String(self.url.clone()));
        m.insert("type".into(), Value::String(self.plugin_type.clone()));
        Value::Object(m)
    }
}

impl TransferServerEntry {
    fn decode(v: &Value) -> Result<Self, DecodeError> {
        let server_value = v
            .get("server")
            .ok_or_else(|| DecodeError("missing field 'server'".to_string()))?;
        let server = ConfigServer::decode(server_value)?;
        let folder_name = req_str(v, "folderName")?;
        let java_port = opt_i64(v, "javaPort");
        let paper_mc_version = opt_str(v, "paperMCVersion");
        let paper_build = opt_i64(v, "paperBuild");
        let bundled_paper_jar = opt_bool(v, "bundledPaperJar", false);
        let plugin_links = match v.get("pluginLinks").and_then(Value::as_array) {
            Some(arr) => arr
                .iter()
                .map(TransferPluginLink::decode)
                .collect::<Result<Vec<_>, _>>()?,
            None => Vec::new(),
        };
        Ok(Self {
            server,
            folder_name,
            java_port,
            paper_mc_version,
            paper_build,
            bundled_paper_jar,
            plugin_links,
        })
    }

    fn encode(&self) -> Value {
        let mut m = Map::new();
        m.insert("server".into(), self.server.encode());
        m.insert("folderName".into(), Value::String(self.folder_name.clone()));
        m.insert(
            "javaPort".into(),
            self.java_port.map(Value::from).unwrap_or(Value::Null),
        );
        m.insert(
            "paperMCVersion".into(),
            self.paper_mc_version
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null),
        );
        m.insert(
            "paperBuild".into(),
            self.paper_build.map(Value::from).unwrap_or(Value::Null),
        );
        m.insert(
            "bundledPaperJar".into(),
            Value::Bool(self.bundled_paper_jar),
        );
        m.insert(
            "pluginLinks".into(),
            Value::Array(
                self.plugin_links
                    .iter()
                    .map(TransferPluginLink::encode)
                    .collect(),
            ),
        );
        Value::Object(m)
    }
}

impl TransferManifest {
    pub fn decode(v: &Value) -> Result<Self, DecodeError> {
        let format_version = req_i64(v, "formatVersion")?;
        let app_config_version = req_i64(v, "appConfigVersion")?;
        let created_at = req_str(v, "createdAt")?;
        let source_machine_name = req_str(v, "sourceMachineName")?;
        let servers = v
            .get("servers")
            .and_then(Value::as_array)
            .ok_or_else(|| DecodeError("missing field 'servers'".to_string()))?
            .iter()
            .map(TransferServerEntry::decode)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            format_version,
            app_config_version,
            created_at,
            source_machine_name,
            servers,
        })
    }

    pub fn encode(&self) -> Value {
        let mut m = Map::new();
        m.insert("formatVersion".into(), Value::from(self.format_version));
        m.insert(
            "appConfigVersion".into(),
            Value::from(self.app_config_version),
        );
        m.insert("createdAt".into(), Value::String(self.created_at.clone()));
        m.insert(
            "sourceMachineName".into(),
            Value::String(self.source_machine_name.clone()),
        );
        m.insert(
            "servers".into(),
            Value::Array(
                self.servers
                    .iter()
                    .map(TransferServerEntry::encode)
                    .collect(),
            ),
        );
        Value::Object(m)
    }
}

fn req_str(v: &Value, key: &str) -> Result<String, DecodeError> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| DecodeError(format!("missing or invalid string field '{key}'")))
}

fn req_i64(v: &Value, key: &str) -> Result<i64, DecodeError> {
    v.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| DecodeError(format!("missing or invalid integer field '{key}'")))
}

fn opt_str(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

fn opt_i64(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(Value::as_i64)
}

fn opt_bool(v: &Value, key: &str, default: bool) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(default)
}

// ---------- Export ----------

#[derive(Debug, Clone)]
pub struct TransferExportServerInput {
    pub server: ConfigServer,
    /// `PaperVersionSidecarManager` isn't ported (Phase 7 provisioning
    /// territory — see `phase5-scope.md`'s deferred list); this step
    /// carries the already-known sidecar values through rather than
    /// reading them off disk itself.
    pub paper_mc_version: Option<String>,
    pub paper_build: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct TransferExportRequest {
    pub servers: Vec<TransferExportServerInput>,
    /// ISO8601 export time. Caller-supplied rather than generated here, so
    /// this function stays a pure-ish transform over an explicit clock
    /// input instead of owning "now" — the same seam P5.7's
    /// `restore_servers_from_backup` draws around persistence.
    pub created_at: String,
    /// `Host.current().localizedName`, or `"Unknown Mac"` — resolving the
    /// real hostname is platform I/O the caller owns, not this function.
    pub source_machine_name: String,
    pub app_config_version: i64,
}

#[derive(Debug)]
pub enum TransferExportError {
    Io { path: PathBuf, message: String },
    UnsafeEntryName(String),
    Zip(String),
}

impl fmt::Display for TransferExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => write!(f, "{}: {message}", path.display()),
            Self::UnsafeEntryName(name) => {
                write!(f, "refusing to write unsafe archive entry {name:?}")
            }
            Self::Zip(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for TransferExportError {}

fn io_error(path: &Path, error: io::Error) -> TransferExportError {
    TransferExportError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

/// Ports `exportServerTransfer(to:)`. Stages every server's bundled files
/// straight into the zip (no separate on-disk staging directory — MSC 1
/// stages to a temp folder only because it shells out to `/usr/bin/zip`
/// afterward; writing zip entries directly is the equivalent with a real
/// Rust zip library) and returns the manifest that was written alongside
/// them, for a caller that wants to inspect what got exported without
/// re-reading the archive.
///
/// Never exposed as a public HTTP endpoint (`phase5-scope.md` "Deferred
/// and homeless") — P5.16's replace-all backup step is this function's
/// only caller.
pub fn export_server_transfer<W: Write + io::Seek>(
    request: &TransferExportRequest,
    writer: W,
) -> Result<TransferManifest, TransferExportError> {
    let mut zip = ZipWriter::new(writer);
    let mut used_folder_names = HashSet::new();
    let mut entries = Vec::with_capacity(request.servers.len());

    for input in &request.servers {
        let entry = export_one_server(&mut zip, input, &mut used_folder_names)?;
        entries.push(entry);
    }

    let manifest = TransferManifest {
        format_version: FORMAT_VERSION,
        app_config_version: request.app_config_version,
        created_at: request.created_at.clone(),
        source_machine_name: request.source_machine_name.clone(),
        servers: entries,
    };

    let manifest_json = serde_json::to_vec_pretty(&manifest.encode())
        .map_err(|e| TransferExportError::Zip(e.to_string()))?;
    add_file_entry(&mut zip, "manifest.json", &manifest_json)?;
    zip.finish()
        .map_err(|e| TransferExportError::Zip(e.to_string()))?;

    Ok(manifest)
}

fn export_one_server<W: Write + io::Seek>(
    zip: &mut ZipWriter<W>,
    input: &TransferExportServerInput,
    used_folder_names: &mut HashSet<String>,
) -> Result<TransferServerEntry, TransferExportError> {
    let server = &input.server;
    let folder_name = unique_transfer_folder_name(&server.display_name, used_folder_names);
    let server_dir = PathBuf::from(&server.server_dir);
    let prefix = format!("servers/{folder_name}");
    add_directory_entry(zip, &prefix)?;

    let is_java = server.server_type == ServerType::Java;
    let properties = if is_java {
        read_properties_map(&server_dir.join("server.properties"))
    } else {
        HashMap::new()
    };
    let java_port = properties
        .get("server-port")
        .and_then(|s| s.parse::<i64>().ok());

    let mut bundled_paper_jar = false;
    if is_java && !server.paper_jar_path.trim().is_empty() {
        let jar_path = PathBuf::from(&server.paper_jar_path);
        if jar_path.is_file() {
            let bytes = fs::read(&jar_path).map_err(|e| io_error(&jar_path, e))?;
            add_file_entry(zip, &format!("{prefix}/paper.jar"), &bytes)?;
            bundled_paper_jar = true;
        }
    }

    for name in WHOLESALE_SUBDIRS {
        let dir = server_dir.join(name);
        if dir.is_dir() {
            add_dir_recursive(zip, &dir, &format!("{prefix}/{name}"))?;
        }
    }

    if matches!(
        server.java_flavor,
        JavaServerFlavor::NeoForge | JavaServerFlavor::Forge
    ) {
        let dir = server_dir.join("libraries");
        if dir.is_dir() {
            add_dir_recursive(zip, &dir, &format!("{prefix}/libraries"))?;
        }
    }

    if is_java {
        let level = properties
            .get("level-name")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or("world")
            .to_string();
        for candidate in [
            level.clone(),
            format!("{level}_nether"),
            format!("{level}_the_end"),
        ] {
            let dir = server_dir.join(&candidate);
            if dir.is_dir() {
                add_dir_recursive(zip, &dir, &format!("{prefix}/{candidate}"))?;
            }
        }
    } else {
        let dir = server_dir.join("worlds");
        if dir.is_dir() {
            add_dir_recursive(zip, &dir, &format!("{prefix}/worlds"))?;
        }
    }

    let mut config_files = Vec::new();
    if let Ok(read_dir) = fs::read_dir(&server_dir) {
        for entry in read_dir.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_allowlisted_config_extension(&name) {
                config_files.push(name);
            }
        }
    }
    config_files.sort();
    if !config_files.is_empty() {
        add_directory_entry(zip, &format!("{prefix}/configs"))?;
        for name in &config_files {
            let path = server_dir.join(name);
            let bytes = fs::read(&path).map_err(|e| io_error(&path, e))?;
            add_file_entry(zip, &format!("{prefix}/configs/{name}"), &bytes)?;
        }
    }

    Ok(TransferServerEntry {
        server: sanitize_for_manifest(server),
        folder_name,
        java_port,
        paper_mc_version: input.paper_mc_version.clone(),
        paper_build: input.paper_build,
        bundled_paper_jar,
        plugin_links: plugin_links_from(&server.plugin_sources),
    })
}

/// Blanks the machine-specific / Xbox account fields before an entry's
/// `server` is written into the manifest (source line 241-248) — see
/// "Sanitization (export-time)" in the format doc.
fn sanitize_for_manifest(server: &ConfigServer) -> ConfigServer {
    let mut sanitized = server.clone();
    sanitized.server_dir = String::new();
    sanitized.paper_jar_path = String::new();
    sanitized.xbox_broadcast_config_path = None;
    sanitized.xbox_broadcast_alt_email = None;
    sanitized.xbox_broadcast_alt_gamertag = None;
    sanitized.xbox_broadcast_alt_password = None;
    sanitized.xbox_broadcast_alt_avatar_path = None;
    sanitized
}

fn plugin_links_from(
    plugin_sources: &Option<HashMap<String, PluginSourceConfig>>,
) -> Vec<TransferPluginLink> {
    let Some(map) = plugin_sources else {
        return Vec::new();
    };
    map.iter()
        .map(|(filename, config)| TransferPluginLink {
            filename: filename.clone(),
            url: config.url.clone(),
            plugin_type: config.source_type.raw_value().to_string(),
        })
        .collect()
}

fn is_allowlisted_config_extension(filename: &str) -> bool {
    Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| CONFIG_FILE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// `uniqueTransferFolderName` (source line 557-566): lowercase, spaces to
/// `_`, strip anything outside `[a-z0-9_-]`, truncate to 40 characters,
/// `"server"` if that leaves nothing, and `-2`/`-3`/… on collision within
/// one export run.
fn unique_transfer_folder_name(display_name: &str, used: &mut HashSet<String>) -> String {
    let base = sanitize_folder_name(display_name);
    let mut candidate = base.clone();
    let mut suffix = 2;
    while used.contains(&candidate) {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    used.insert(candidate.clone());
    candidate
}

fn sanitize_folder_name(display_name: &str) -> String {
    let mut result = String::new();
    for c in display_name.to_lowercase().chars() {
        if c == ' ' {
            result.push('_');
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' {
            result.push(c);
        }
    }
    // Every pushed char above is single-byte ASCII, so a byte-length
    // truncate can't land mid-character.
    result.truncate(40);
    if result.is_empty() {
        "server".to_string()
    } else {
        result
    }
}

fn read_properties_map(path: &Path) -> HashMap<String, String> {
    let Ok(contents) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    let mut map = HashMap::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}

fn add_directory_entry<W: Write + io::Seek>(
    zip: &mut ZipWriter<W>,
    name: &str,
) -> Result<(), TransferExportError> {
    if !is_safe_zip_entry_name(name) {
        return Err(TransferExportError::UnsafeEntryName(name.to_string()));
    }
    let opts = SimpleFileOptions::default().unix_permissions(0o755);
    zip.add_directory(format!("{name}/"), opts)
        .map_err(|e| TransferExportError::Zip(e.to_string()))
}

fn add_file_entry<W: Write + io::Seek>(
    zip: &mut ZipWriter<W>,
    name: &str,
    bytes: &[u8],
) -> Result<(), TransferExportError> {
    if !is_safe_zip_entry_name(name) {
        return Err(TransferExportError::UnsafeEntryName(name.to_string()));
    }
    let opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    zip.start_file(name, opts)
        .map_err(|e| TransferExportError::Zip(e.to_string()))?;
    zip.write_all(bytes)
        .map_err(|e| TransferExportError::Zip(e.to_string()))?;
    Ok(())
}

/// Recursively adds `disk_dir`'s contents under `zip_prefix`, including an
/// entry for `disk_dir` itself (so a subdirectory that happens to be
/// empty — a fresh `plugins/` folder, say — still round-trips as an empty
/// directory rather than vanishing from the archive entirely). Entries are
/// written in sorted-by-name order for deterministic output; MSC 1's own
/// `zip -r` has no such guarantee (directory-listing order is unspecified),
/// so this is a Rust-side improvement, not a parity requirement.
fn add_dir_recursive<W: Write + io::Seek>(
    zip: &mut ZipWriter<W>,
    disk_dir: &Path,
    zip_prefix: &str,
) -> Result<(), TransferExportError> {
    add_directory_entry(zip, zip_prefix)?;
    let mut entries: Vec<_> = fs::read_dir(disk_dir)
        .map_err(|e| io_error(disk_dir, e))?
        .collect::<Result<_, io::Error>>()
        .map_err(|e| io_error(disk_dir, e))?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let zip_path = format!("{zip_prefix}/{name}");
        let file_type = entry.file_type().map_err(|e| io_error(&path, e))?;
        if file_type.is_dir() {
            add_dir_recursive(zip, &path, &zip_path)?;
        } else if file_type.is_file() {
            let bytes = fs::read(&path).map_err(|e| io_error(&path, e))?;
            add_file_entry(zip, &zip_path, &bytes)?;
        }
        // Symlinks in a source server directory are neither bundled nor
        // rejected by MSC 1's shelled-out `zip -r` (it follows them); no
        // fixture exercises one, so this port leaves them untouched too
        // rather than inventing new source-side policy.
    }
    Ok(())
}

fn is_safe_zip_entry_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let path = Path::new(name);
    if path.is_absolute() {
        return false;
    }
    path.components()
        .all(|c| matches!(c, std::path::Component::Normal(_)))
}

// ---------- Inspect ----------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferServerConflict {
    pub folder_name: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferInspection {
    pub staging_root: PathBuf,
    pub manifest: TransferManifest,
    pub conflicts: Vec<TransferServerConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferInspectError {
    /// Package couldn't be opened/read as a zip archive at all, or a
    /// staging-directory I/O step failed. Not one of MSC 1's three named
    /// failure shapes — those all assume the archive itself opens fine —
    /// but a real filesystem can still fail here, so it needs a variant.
    OpenPackage(String),
    /// An entry's name escapes the staging root (path traversal or an
    /// absolute path) or is a symlink — Rust-side hardening MSC 1's own
    /// `/usr/bin/unzip -o` shell-out never had. See the format doc's
    /// "Where this connects downstream".
    UnsafeEntry(String),
    /// No `manifest.json` at the staging root after extraction (source
    /// line 314-317).
    MissingManifest,
    /// `manifest.json` failed to parse as JSON or decode into
    /// `TransferManifest` (source line 349-352, the outer catch block).
    Decode(String),
    /// `manifest.formatVersion` is strictly greater than
    /// [`FORMAT_VERSION`] (source line 324-327).
    UnsupportedFormatVersion { found: i64 },
}

impl fmt::Display for TransferInspectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenPackage(message) => write!(f, "could not open transfer package: {message}"),
            Self::UnsafeEntry(name) => {
                write!(f, "transfer package contains an unsafe entry: {name}")
            }
            Self::MissingManifest => write!(f, "transfer package is missing manifest.json"),
            Self::Decode(message) => write!(f, "could not read transfer package: {message}"),
            Self::UnsupportedFormatVersion { .. } => write!(
                f,
                "This transfer file was created by a newer version of MSC. Update the app and try again."
            ),
        }
    }
}

impl std::error::Error for TransferInspectError {}

/// Ports `inspectTransferPackage(at:)`. Extracts the whole archive into
/// `staging_root` (creating it first), rejecting any entry that would
/// escape it, then decodes `manifest.json` and compares every entry's
/// recorded port against `existing_java_ports`/`existing_bedrock_ports`.
///
/// `staging_root` is removed on every failure path, matching MSC 1's own
/// "remove staging, then return failure" shape shared by all three of its
/// named failure cases (see the format doc's "Inspect-time failure
/// shape"). On success `staging_root` is left in place — P5.15's apply
/// step is expected to read the already-extracted files from it.
pub fn inspect_transfer_package(
    package_path: &Path,
    staging_root: &Path,
    existing_java_ports: &[i64],
    existing_bedrock_ports: &[i64],
) -> Result<TransferInspection, TransferInspectError> {
    let result = inspect_transfer_package_inner(
        package_path,
        staging_root,
        existing_java_ports,
        existing_bedrock_ports,
    );
    if result.is_err() {
        let _ = fs::remove_dir_all(staging_root);
    }
    result
}

fn inspect_transfer_package_inner(
    package_path: &Path,
    staging_root: &Path,
    existing_java_ports: &[i64],
    existing_bedrock_ports: &[i64],
) -> Result<TransferInspection, TransferInspectError> {
    fs::create_dir_all(staging_root)
        .map_err(|e| TransferInspectError::OpenPackage(e.to_string()))?;

    let file = fs::File::open(package_path)
        .map_err(|e| TransferInspectError::OpenPackage(e.to_string()))?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| TransferInspectError::OpenPackage(e.to_string()))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| TransferInspectError::OpenPackage(e.to_string()))?;
        let raw_name = entry.name().to_string();
        if is_unsafe_raw_entry_name(&raw_name) || is_symlink_mode(entry.unix_mode()) {
            return Err(TransferInspectError::UnsafeEntry(raw_name));
        }
        let Some(enclosed) = entry.enclosed_name() else {
            return Err(TransferInspectError::UnsafeEntry(raw_name));
        };
        let dest = staging_root.join(&enclosed);

        if entry.is_dir() {
            fs::create_dir_all(&dest)
                .map_err(|e| TransferInspectError::OpenPackage(e.to_string()))?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| TransferInspectError::OpenPackage(e.to_string()))?;
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|e| TransferInspectError::OpenPackage(e.to_string()))?;
        fs::write(&dest, &bytes).map_err(|e| TransferInspectError::OpenPackage(e.to_string()))?;
    }

    let manifest_path = staging_root.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(TransferInspectError::MissingManifest);
    }
    let manifest_bytes =
        fs::read(&manifest_path).map_err(|e| TransferInspectError::Decode(e.to_string()))?;
    let manifest_value: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| TransferInspectError::Decode(e.to_string()))?;
    let manifest =
        TransferManifest::decode(&manifest_value).map_err(|e| TransferInspectError::Decode(e.0))?;

    if manifest.format_version > FORMAT_VERSION {
        return Err(TransferInspectError::UnsupportedFormatVersion {
            found: manifest.format_version,
        });
    }

    let mut conflicts = Vec::new();
    for entry in &manifest.servers {
        if entry.server.server_type == ServerType::Java {
            if let Some(port) = entry.java_port
                && existing_java_ports.contains(&port)
            {
                conflicts.push(TransferServerConflict {
                    folder_name: entry.folder_name.clone(),
                    message: format!("Java port {port} is already in use — edit below."),
                });
            }
        } else if let Some(port) = entry.server.bedrock_port
            && existing_bedrock_ports.contains(&port)
        {
            conflicts.push(TransferServerConflict {
                folder_name: entry.folder_name.clone(),
                message: format!("Bedrock port {port} is already in use — edit below."),
            });
        }
    }

    Ok(TransferInspection {
        staging_root: staging_root.to_path_buf(),
        manifest,
        conflicts,
    })
}

// ---------- Apply ----------

/// Inputs to [`apply_transfer_import`] beyond the already-inspected
/// [`TransferInspection`]. Deliberately has no `mode`/`backupPath` field —
/// those are P5.16's concern (transfer mode is a DTO-level input to the
/// HTTP import route, not a build-stage restoration input); see this
/// module's header comment.
#[derive(Debug, Clone, Default)]
pub struct TransferApplyRequest {
    /// `configManager.serversRootURL` on the target machine — the parent
    /// under which `java/`/`bedrock/` type directories live (source line
    /// 371, 386-387).
    pub servers_root: PathBuf,
    /// Keyed by the *source* server's `ConfigServer.id` — i.e. the id an
    /// entry carried in the manifest, before this function replaces it
    /// with a freshly generated one (source line 364-365, 450, 461).
    pub java_port_overrides: HashMap<String, i64>,
    pub bedrock_port_overrides: HashMap<String, i64>,
}

/// The build-stage result: source's `(newServers, imported, skipped)`
/// tuple (source line 517).
#[derive(Debug, Clone, PartialEq)]
pub struct TransferApplyResult {
    pub servers: Vec<ConfigServer>,
    pub imported: usize,
    pub skipped: usize,
}

/// Ports the build-stage half of `applyTransferImport` — see this module's
/// header comment for what's deliberately not ported. From
/// `inspection.staging_root`, for every manifest entry: choose a
/// noncolliding destination under `request.servers_root`, restore configs,
/// the wholesale subdirectories, Forge/NeoForge libraries, an optional
/// bundled `paper.jar`, and world data (preferring live Java/Bedrock world
/// folders bundled in the package; falling back to
/// [`restore_active_slot_world`] only when none exist), apply the caller's
/// port overrides, and produce a re-rooted, re-identified `ConfigServer`.
///
/// A per-entry failure — its `java`/`bedrock` type directory or its own
/// destination directory can't be created, or a wholesale subdirectory
/// fails to copy (the one copy in source's loop that isn't `try?`, source
/// line 423-428) — counts as skipped and removes any partial destination
/// (source line 510-514). Every other per-entry step (configs, libraries,
/// paper.jar, port rewrite, live-world/slot restoration) is best-effort in
/// source too (`try?`, or an inner do/catch that only logs) and never
/// fails the entry.
pub fn apply_transfer_import(
    inspection: &TransferInspection,
    request: &TransferApplyRequest,
) -> TransferApplyResult {
    let pkg_root = inspection.staging_root.join("servers");
    let mut servers = Vec::with_capacity(inspection.manifest.servers.len());
    let mut imported = 0usize;
    let mut skipped = 0usize;

    for entry in &inspection.manifest.servers {
        match apply_one_server(&pkg_root, entry, request) {
            Some(server) => {
                servers.push(server);
                imported += 1;
            }
            None => skipped += 1,
        }
    }

    TransferApplyResult {
        servers,
        imported,
        skipped,
    }
}

fn apply_one_server(
    pkg_root: &Path,
    entry: &TransferServerEntry,
    request: &TransferApplyRequest,
) -> Option<ConfigServer> {
    let is_java = entry.server.server_type == ServerType::Java;
    let type_root = request
        .servers_root
        .join(if is_java { "java" } else { "bedrock" });
    fs::create_dir_all(&type_root).ok()?;

    let dest = unique_destination(&type_root, &entry.folder_name);
    fs::create_dir_all(&dest).ok()?;
    let pkg_dir = pkg_root.join(&entry.folder_name);

    // configs/ -> dest top level, one file at a time, best-effort (source
    // line 413-420 uses `try?` per file).
    if let Ok(read_dir) = fs::read_dir(pkg_dir.join("configs")) {
        for item in read_dir.flatten() {
            if item.file_type().is_ok_and(|t| t.is_file()) {
                let _ = fs::copy(item.path(), dest.join(item.file_name()));
            }
        }
    }

    // world_slots/, backups/, plugins/, mods/, resource-packs/ — the one
    // copy in source's loop that is a hard failure, not `try?` (line
    // 423-428): a failure here removes the destination and skips the
    // whole entry.
    for sub in WHOLESALE_SUBDIRS {
        let src = pkg_dir.join(sub);
        if src.is_dir() && copy_dir_all(&src, &dest.join(sub)).is_err() {
            let _ = fs::remove_dir_all(&dest);
            return None;
        }
    }

    if matches!(
        entry.server.java_flavor,
        JavaServerFlavor::NeoForge | JavaServerFlavor::Forge
    ) {
        let src = pkg_dir.join("libraries");
        if src.is_dir() {
            let _ = copy_dir_all(&src, &dest.join("libraries"));
        }
    }

    let mut paper_jar_path = String::new();
    if is_java && entry.bundled_paper_jar {
        let jar = pkg_dir.join("paper.jar");
        let dest_jar = dest.join("paper.jar");
        if jar.is_file() && fs::copy(&jar, &dest_jar).is_ok() {
            paper_jar_path = dest_jar.to_string_lossy().into_owned();
        }
    }

    if is_java && let Some(port) = request.java_port_overrides.get(&entry.server.id) {
        rewrite_properties_line(&dest.join("server.properties"), "server-port=", *port);
    }

    let mut cfg_server = entry.server.clone();
    cfg_server.id = Uuid::new_v4().to_string().to_uppercase();
    cfg_server.server_dir = dest.to_string_lossy().into_owned();
    cfg_server.paper_jar_path = paper_jar_path;
    cfg_server.xbox_broadcast_config_path = None;
    if !is_java && let Some(port) = request.bedrock_port_overrides.get(&entry.server.id) {
        cfg_server.bedrock_port = Some(*port);
    }

    // Restore world data. Prefer live world folders bundled in the package
    // (source line 465-495) — the exact state the server was left in.
    let mut restored_live_world = false;
    if is_java {
        let props = read_properties_map(&dest.join("server.properties"));
        let raw = props
            .get("level-name")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let level = if raw.is_empty() {
            "world".to_string()
        } else {
            raw
        };
        for candidate in [
            level.clone(),
            format!("{level}_nether"),
            format!("{level}_the_end"),
        ] {
            let src = pkg_dir.join(&candidate);
            if src.is_dir() && copy_dir_all(&src, &dest.join(&candidate)).is_ok() {
                restored_live_world = true;
            }
        }
    } else {
        let src = pkg_dir.join("worlds");
        if src.is_dir() && copy_dir_all(&src, &dest.join("worlds")).is_ok() {
            restored_live_world = true;
        }
    }

    // Fall back only for an older package with no live world folders
    // (source line 497) — see `restore_active_slot_world`'s own doc for
    // the narrow compatibility adapter this substitutes for the real
    // `WorldSlotManager.activeSlot`/`activateSlot` (Phase 6 territory).
    if !restored_live_world {
        restore_active_slot_world(&dest);
    }

    Some(cfg_server)
}

/// Picks the same noncolliding destination folder source does (line
/// 395-403): try `folderName`, then `folderName-2`, `folderName-3`, …
fn unique_destination(type_root: &Path, folder_name: &str) -> PathBuf {
    let mut candidate = type_root.join(folder_name);
    let mut counter = 2;
    while candidate.exists() {
        candidate = type_root.join(format!("{folder_name}-{counter}"));
        counter += 1;
    }
    candidate
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// `ServerTransfer.updateServerPropertiesPort` (source line 569-577),
/// generalized to any `key=`-prefixed line: replaces an existing matching
/// line in place; does **not** add the key if absent, matching source
/// exactly. Best-effort — a read failure is silently a no-op, same as
/// source's `guard var content = try? ...`.
fn rewrite_properties_line(path: &Path, key_prefix: &str, value: i64) {
    let Ok(content) = fs::read_to_string(path) else {
        return;
    };
    let rewritten = content
        .split('\n')
        .map(|line| {
            if line.starts_with(key_prefix) {
                format!("{key_prefix}{value}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let _ = fs::write(path, rewritten);
}

/// Substitutes for MSC 1's `WorldSlotManager.activeSlot(forServerDir:)` /
/// `activateSlot(...)` (source line 497-505) without the formal slot model
/// — that stays Phase 6 (`phase5-scope.md` "Deferred and homeless"; see
/// also the format doc's "Apply-time world precedence"). Narrow and
/// read-only with respect to slot bookkeeping: it resolves
/// `world_slots/active_slot_id.txt` (already restored into `dest` by the
/// wholesale copy above — resolved against the package's *own* copied
/// `world_slots/`, not the source machine's, matching source) to a slot
/// id, and if that slot has a `world.zip`, extracts it directly into
/// `dest`. It never rewrites `active_slot_id.txt`, never updates
/// `slot.json`'s `lastPlayedAt`, and never infers a level name — source's
/// zip is created by zipping the live world folder(s) by name relative to
/// `serverDir` (`WorldSlotManager.createSlot`), so extracting it directly
/// into `dest` reproduces the same `<levelName>[_nether|_the_end]`/`worlds`
/// layout a live-world restore would have, with no extra bookkeeping
/// needed for a first-time import.
fn restore_active_slot_world(dest: &Path) {
    let marker = dest.join("world_slots").join("active_slot_id.txt");
    let Ok(raw) = fs::read_to_string(&marker) else {
        return;
    };
    let slot_id = raw.trim();
    if slot_id.is_empty() {
        return;
    }
    let zip_path = dest.join("world_slots").join(slot_id).join("world.zip");
    if !zip_path.is_file() {
        return;
    }
    extract_zip_into(&zip_path, dest);
}

/// Extracts every entry of the zip at `zip_path` into `dest_root`, reusing
/// [`is_unsafe_raw_entry_name`]/[`is_symlink_mode`] — `world.zip` is
/// data that arrived nested inside an already-hardened outer package, but
/// its own entries were never individually checked when the outer package
/// was extracted, and this extracts straight into an owned server
/// directory rather than disposable staging, so the same hardening applies
/// here, not less. Best-effort: any unsafe entry or I/O failure stops
/// extraction for this slot without touching anything else in `dest`
/// (whatever already extracted stays, matching source's `unzip -o`
/// failure just being logged, not rolled back).
fn extract_zip_into(zip_path: &Path, dest_root: &Path) {
    let Ok(file) = fs::File::open(zip_path) else {
        return;
    };
    let Ok(mut archive) = ZipArchive::new(file) else {
        return;
    };
    for i in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(i) else {
            return;
        };
        let raw_name = entry.name().to_string();
        if is_unsafe_raw_entry_name(&raw_name) || is_symlink_mode(entry.unix_mode()) {
            return;
        }
        let Some(enclosed) = entry.enclosed_name() else {
            return;
        };
        let dest = dest_root.join(&enclosed);
        if entry.is_dir() {
            let _ = fs::create_dir_all(&dest);
            continue;
        }
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut bytes = Vec::new();
        if entry.read_to_end(&mut bytes).is_err() {
            return;
        }
        if fs::write(&dest, &bytes).is_err() {
            return;
        }
    }
}

/// `enclosed_name()` alone isn't enough: the `zip` crate *relativizes* an
/// absolute entry (`/etc/passwd` -> `etc/passwd`, `C:\Windows\evil` ->
/// `Windows\evil`) instead of refusing it, which would silently defeat the
/// "absolute-path rejection" this step requires — proved against the real
/// crate before writing this, not assumed. `..` traversal is still caught
/// by `enclosed_name()` returning `None`, so that half doesn't need a
/// redundant check here.
fn is_unsafe_raw_entry_name(name: &str) -> bool {
    if name.starts_with('/') || name.starts_with('\\') {
        return true;
    }
    let mut chars = name.chars();
    matches!((chars.next(), chars.next()), (Some(drive), Some(':')) if drive.is_ascii_alphabetic())
}

fn is_symlink_mode(unix_mode: Option<u32>) -> bool {
    matches!(unix_mode, Some(mode) if mode & 0o170000 == 0o120000)
}
