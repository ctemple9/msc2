//! Existing Paper server import for the Phase 4 Java lifecycle slice, plus
//! (P5.19/P5.20) the broader raw-directory scan and import MSC 1 actually
//! ships: any Java flavor or Bedrock, not just an already-registered Paper
//! folder. The Phase 4 half above intentionally does not copy, unzip,
//! create world slots, or write `server.properties` — it only registers
//! one already-existing Paper directory so lifecycle work could start
//! against real files before this broader path existed.
//!
//! The P5.19/P5.20 halves port `AppViewModel+ServerImport.swift`'s
//! `scanServerDirectory`/`detectJavaFlavor` (read-only) and
//! `importExistingServer` (mutating), per the fixtures and behavior
//! write-up P5.18 produced in `fixtures/raw-server-import/` and
//! `docs/msc2/config-migration/raw-import-behavior.md`. `importExistingServer`
//! itself has no fixture oracle (P5.18 scoped fixtures to the read-only
//! half only) — its own doc comment below cites the exact MSC 1 source
//! lines this step read directly instead.

use crate::lifecycle::{ImportedJavaServer, ServerId};
use msc_domain::identity::{JavaServerFlavor, ServerType};
use msc_domain::properties::ServerPropertiesModel;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use zip::ZipArchive;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: String,
    pub is_file: bool,
}

impl DirectoryEntry {
    pub fn file(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_file: true,
        }
    }

    pub fn directory(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_file: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaperImportRequest {
    pub display_name: String,
    pub server_dir: PathBuf,
}

impl PaperImportRequest {
    pub fn new(display_name: impl Into<String>, server_dir: impl Into<PathBuf>) -> Self {
        Self {
            display_name: display_name.into(),
            server_dir: server_dir.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedPaperServer {
    pub id: ServerId,
    pub display_name: String,
    pub server_dir: PathBuf,
    pub paper_jar_path: PathBuf,
    pub eula_accepted: Option<bool>,
    pub game_port: i64,
    pub max_players: i64,
    pub world_name: String,
    pub properties: ServerPropertiesModel,
}

impl ImportedPaperServer {
    pub fn lifecycle_server(&self) -> ImportedJavaServer {
        ImportedJavaServer {
            id: self.id.clone(),
            name: self.display_name.clone(),
            directory: self.server_dir.clone(),
            flavor: JavaServerFlavor::Paper,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaperImportError {
    EmptyDisplayName,
    ReadDirectory { path: PathBuf, message: String },
    NoJavaServerJar { path: PathBuf },
    Registry(String),
}

impl fmt::Display for PaperImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDisplayName => write!(f, "display name cannot be empty"),
            Self::ReadDirectory { path, message } => {
                write!(
                    f,
                    "could not read server directory {}: {message}",
                    path.display()
                )
            }
            Self::NoJavaServerJar { path } => {
                write!(f, "no Java server JAR found in {}", path.display())
            }
            Self::Registry(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for PaperImportError {}

pub trait PaperImportFileSystem {
    fn read_dir(&self, path: &Path) -> Result<Vec<DirectoryEntry>, PaperImportError>;
    fn read_to_string(&self, path: &Path) -> Result<Option<String>, PaperImportError>;
}

pub struct StdPaperImportFileSystem;

impl PaperImportFileSystem for StdPaperImportFileSystem {
    fn read_dir(&self, path: &Path) -> Result<Vec<DirectoryEntry>, PaperImportError> {
        let entries = std::fs::read_dir(path).map_err(|error| PaperImportError::ReadDirectory {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        let mut result = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| PaperImportError::ReadDirectory {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
            let file_type = entry
                .file_type()
                .map_err(|error| PaperImportError::ReadDirectory {
                    path: entry.path(),
                    message: error.to_string(),
                })?;
            result.push(DirectoryEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                is_file: file_type.is_file(),
            });
        }
        Ok(result)
    }

    fn read_to_string(&self, path: &Path) -> Result<Option<String>, PaperImportError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => Ok(Some(contents)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(PaperImportError::ReadDirectory {
                path: path.to_path_buf(),
                message: error.to_string(),
            }),
        }
    }
}

pub trait PaperServerRegistry {
    fn register(&mut self, server: ImportedPaperServer) -> Result<(), PaperImportError>;
}

pub fn import_existing_paper_server(
    fs: &dyn PaperImportFileSystem,
    registry: &mut dyn PaperServerRegistry,
    request: &PaperImportRequest,
) -> Result<ImportedPaperServer, PaperImportError> {
    let display_name = request.display_name.trim();
    if display_name.is_empty() {
        return Err(PaperImportError::EmptyDisplayName);
    }

    let entries = fs.read_dir(&request.server_dir)?;
    let paper_jar_path = detect_paper_jar(&request.server_dir, &entries).ok_or_else(|| {
        PaperImportError::NoJavaServerJar {
            path: request.server_dir.clone(),
        }
    })?;

    let raw_properties = read_server_properties(fs, &request.server_dir.join("server.properties"))?;
    let properties = ServerPropertiesModel::from_dict(&raw_properties, None);
    let world_name = raw_properties
        .get("level-name")
        .cloned()
        .unwrap_or_else(|| "world".to_string());
    let eula_accepted = read_eula(fs, &request.server_dir.join("eula.txt"))?;

    let server = ImportedPaperServer {
        id: stable_paper_server_id(&request.server_dir),
        display_name: display_name.to_string(),
        server_dir: request.server_dir.clone(),
        paper_jar_path,
        eula_accepted,
        game_port: properties.server_port,
        max_players: properties.max_players,
        world_name,
        properties,
    };

    registry.register(server.clone())?;
    Ok(server)
}

pub fn stable_paper_server_id(server_dir: &Path) -> ServerId {
    let normalized = normalized_path_string(server_dir);
    ServerId::new(format!("paper-{:016x}", fnv1a64(normalized.as_bytes())))
}

fn detect_paper_jar(server_dir: &Path, entries: &[DirectoryEntry]) -> Option<PathBuf> {
    let jars = entries
        .iter()
        .filter(|entry| entry.is_file && entry.name.to_lowercase().ends_with(".jar"))
        .collect::<Vec<_>>();
    let selected = jars
        .iter()
        .find(|entry| entry.name.to_lowercase().starts_with("paper"))
        .copied()
        .or_else(|| jars.first().copied())?;
    Some(server_dir.join(&selected.name))
}

fn read_server_properties(
    fs: &dyn PaperImportFileSystem,
    path: &Path,
) -> Result<HashMap<String, String>, PaperImportError> {
    let Some(contents) = fs.read_to_string(path)? else {
        return Ok(HashMap::new());
    };

    let mut properties = HashMap::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        properties.insert(key.trim().to_string(), value.trim().to_string());
    }
    Ok(properties)
}

fn read_eula(
    fs: &dyn PaperImportFileSystem,
    path: &Path,
) -> Result<Option<bool>, PaperImportError> {
    let Some(contents) = fs.read_to_string(path)? else {
        return Ok(None);
    };

    for line in contents.lines() {
        let raw = line.trim();
        let lower = raw.to_lowercase();
        if lower.starts_with("eula=") {
            return Ok(Some(lower.contains("true")));
        }
    }
    Ok(None)
}

fn normalized_path_string(path: &Path) -> String {
    let mut normalized = path.to_string_lossy().replace('\\', "/");
    while normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }
    normalized
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

// =====================================================================
// P5.19 — read-only raw server-directory scanning
//
// Ports `scanServerDirectory` and `detectJavaFlavor`
// (`AppViewModel+ServerImport.swift:235-437`) plus the single-root zip
// unwrap `AddServerWizardView.performScan` runs before calling
// `scanServerDirectory` (`AddServerWizardView.swift:2132-2201`). Oracle
// evidence is `fixtures/raw-server-import/` (P5.18) and
// `docs/msc2/config-migration/raw-import-behavior.md`; line numbers below
// were re-confirmed directly against MSC 1 source while writing this.
// =====================================================================

/// One directory entry as the scanner sees it — a name plus file/directory,
/// nothing else. Deliberately not [`DirectoryEntry`] (the Phase 4 Paper
/// import's own type): that one has no notion of file size, which this
/// scanner needs for world-size aggregation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScanEntry {
    pub name: String,
    pub is_file: bool,
}

/// The filesystem surface `scan_server_directory` and `detect_java_flavor`
/// need. Every real listing MSC 1's source performs — root contents,
/// version-directory listings under `libraries/...`, the Fabric loader
/// directory, world candidates under both search roots — is a *listing*,
/// not a single existence check, so this is shaped around `list_dir`
/// rather than `PaperImportFileSystem`'s flatter `read_dir`.
pub trait RawImportFileSystem {
    /// Non-recursive listing of `path`'s immediate children. Empty if
    /// `path` doesn't exist or isn't a directory — matches every call site
    /// here, which reads `try? contentsOfDirectory(...) ?? []`/`?? nil`.
    fn list_dir(&self, path: &Path) -> Vec<RawScanEntry>;
    fn is_dir(&self, path: &Path) -> bool;
    fn is_file(&self, path: &Path) -> bool;
    fn read_to_string(&self, path: &Path) -> Option<String>;
    fn file_size(&self, path: &Path) -> u64;
}

/// Real-filesystem [`RawImportFileSystem`], used by production scan/import
/// callers; fixture-driven tests use their own in-memory implementation.
pub struct StdRawImportFileSystem;

impl RawImportFileSystem for StdRawImportFileSystem {
    fn list_dir(&self, path: &Path) -> Vec<RawScanEntry> {
        let Ok(entries) = fs::read_dir(path) else {
            return Vec::new();
        };
        entries
            .filter_map(|entry| entry.ok())
            .map(|entry| {
                let is_file = entry.file_type().is_ok_and(|t| t.is_file());
                RawScanEntry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    is_file,
                }
            })
            .collect()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn read_to_string(&self, path: &Path) -> Option<String> {
        fs::read_to_string(path).ok()
    }

    fn file_size(&self, path: &Path) -> u64 {
        fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    }
}

/// One discovered world folder. `folder_path` is a `/`-joined path
/// relative to whichever directory `scan_server_directory` was called
/// with — e.g. `"creative"` or `"worlds/survival"` — not an absolute path,
/// so results are directly comparable across a scan-time zip's disposable
/// staging root and a real on-disk directory alike.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedWorld {
    pub name: String,
    pub folder_path: String,
    pub size_bytes: u64,
    pub has_nether: bool,
    pub has_end: bool,
}

/// Mirrors `ScannedServerInfo` (`AppViewModel+ServerImport.swift:14-25`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedServerInfo {
    pub server_type: ServerType,
    pub port: i64,
    pub max_players: i64,
    pub eula_accepted: bool,
    pub worlds: Vec<DetectedWorld>,
    pub default_world_name: String,
    /// `None` for Bedrock — `detectJavaFlavor` is only ever called for
    /// `.java` (source line 351).
    pub java_flavor: Option<JavaServerFlavor>,
    pub detected_mc_version: Option<String>,
    pub detected_loader_version: Option<String>,
    /// `/`-joined, relative to the scanned directory, like `folder_path`
    /// above — `None` for NeoForge (launches via `unix_args.txt`, no jar)
    /// and for a jar-less directory.
    pub primary_jar_path: Option<String>,
}

const WORLD_SKIP_DIRS: [&str; 11] = [
    "plugins",
    "logs",
    "cache",
    "crash-reports",
    "libraries",
    "versions",
    "mods",
    "config",
    "backups",
    "worlds",
    "__MACOSX",
];

/// Port of `scanServerDirectory` (source line 235-364). Read-only: never
/// copies, extracts, or registers anything — see P5.20's
/// `import_raw_server` for the mutating half.
pub fn scan_server_directory(fs: &dyn RawImportFileSystem, server_dir: &Path) -> ScannedServerInfo {
    let server_type = detect_server_type(fs, server_dir);

    let raw_props = read_raw_properties(fs, &server_dir.join("server.properties"));
    let default_port = if server_type == ServerType::Java {
        25565
    } else {
        19132
    };
    let port = raw_props
        .get("server-port")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(default_port);
    let max_players = raw_props
        .get("max-players")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(20);

    // Source line 253-256: a raw substring check against `?? ""`, not
    // `EULAManager`'s tri-state read — a missing/unreadable eula.txt reads
    // as `false`, not `None`. Preserved as-is; see
    // `fixtures/raw-server-import/eula-missing-file-defaults-to-false-not-null.json`.
    let eula_accepted = fs
        .read_to_string(&server_dir.join("eula.txt"))
        .unwrap_or_default()
        .contains("eula=true");

    let configured_level_name = raw_props
        .get("level-name")
        .cloned()
        .unwrap_or_else(|| "world".to_string());

    let (worlds, default_world_name) = discover_worlds(fs, server_dir, &configured_level_name);

    let (java_flavor, detected_mc_version, detected_loader_version, primary_jar_path) =
        if server_type == ServerType::Java {
            let flavor = detect_java_flavor(fs, server_dir);
            (
                Some(flavor.flavor),
                flavor.mc_version,
                flavor.loader_version,
                flavor.primary_jar_path,
            )
        } else {
            (None, None, None, None)
        };

    ScannedServerInfo {
        server_type,
        port,
        max_players,
        eula_accepted,
        worlds,
        default_world_name,
        java_flavor,
        detected_mc_version,
        detected_loader_version,
        primary_jar_path,
    }
}

/// Source line 240-246: `hasJar`/`hasBedrock` are plain-string checks
/// against the root listing (not filtered to files) — a directory that
/// happens to be named e.g. `bedrock_server` would still count. Ported
/// exactly, not hardened, since that's what the oracle does.
fn detect_server_type(fs: &dyn RawImportFileSystem, server_dir: &Path) -> ServerType {
    let contents = fs.list_dir(server_dir);
    let has_jar = contents
        .iter()
        .any(|e| e.name.to_lowercase().ends_with(".jar"));
    let has_bedrock = contents
        .iter()
        .any(|e| e.name == "bedrock_server" || e.name == "bedrock_server.exe");
    if has_bedrock && !has_jar {
        ServerType::Bedrock
    } else {
        ServerType::Java
    }
}

/// `ServerPropertiesManager.readProperties`/`BedrockPropertiesManager.readRawProperties`
/// — identical line-based `key=value` parsing in both (confirmed against
/// both source files), reading from the same relative `server.properties`
/// path regardless of platform.
fn read_raw_properties(fs: &dyn RawImportFileSystem, path: &Path) -> HashMap<String, String> {
    let Some(contents) = fs.read_to_string(path) else {
        return HashMap::new();
    };
    let mut props = HashMap::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some(idx) = trimmed.find('=') else {
            continue;
        };
        let key = trimmed[..idx].trim().to_string();
        let value = trimmed[idx + 1..].trim().to_string();
        props.insert(key, value);
    }
    props
}

/// Source line 261-350: search `<server_dir>/worlds/` then `<server_dir>`
/// itself, union the results (first occurrence of a name wins), require
/// `level.dat`, skip [`WORLD_SKIP_DIRS`] and dotfiles, fold `_nether`/
/// `_the_end` companions (standalone sibling **or** inline `DIM-1`/`DIM1`)
/// into their root world, then sort with the configured level-name first.
fn discover_worlds(
    fs: &dyn RawImportFileSystem,
    server_dir: &Path,
    configured_level_name: &str,
) -> (Vec<DetectedWorld>, String) {
    let mut raw: Vec<(String, String)> = Vec::new(); // (name, folder_path relative to server_dir)
    let mut seen = HashSet::new();

    for (root_prefix, root_abs) in [
        ("worlds".to_string(), server_dir.join("worlds")),
        (String::new(), server_dir.to_path_buf()),
    ] {
        let mut entries = fs.list_dir(&root_abs);
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        for entry in entries {
            if entry.is_file || seen.contains(&entry.name) {
                continue;
            }
            if WORLD_SKIP_DIRS.contains(&entry.name.as_str()) || entry.name.starts_with('.') {
                continue;
            }
            let folder_abs = root_abs.join(&entry.name);
            if !fs.is_file(&folder_abs.join("level.dat")) {
                continue;
            }
            seen.insert(entry.name.clone());
            let folder_rel = if root_prefix.is_empty() {
                entry.name.clone()
            } else {
                format!("{root_prefix}/{}", entry.name)
            };
            raw.push((entry.name, folder_rel));
        }
    }

    let all_names: HashSet<&str> = raw.iter().map(|(name, _)| name.as_str()).collect();
    let mut worlds = Vec::new();
    for (name, folder_rel) in &raw {
        let is_nether_of =
            name.ends_with("_nether") && all_names.contains(&name[..name.len() - "_nether".len()]);
        let is_end_of = name.ends_with("_the_end")
            && all_names.contains(&name[..name.len() - "_the_end".len()]);
        if is_nether_of || is_end_of {
            continue;
        }

        let folder_abs = server_dir.join(folder_rel);
        let nether_name = format!("{name}_nether");
        let end_name = format!("{name}_the_end");
        let has_nether_inline = fs.is_dir(&folder_abs.join("DIM-1"));
        let has_nether_companion = all_names.contains(nether_name.as_str());
        let has_end_inline = fs.is_dir(&folder_abs.join("DIM1"));
        let has_end_companion = all_names.contains(end_name.as_str());

        let mut size_bytes = directory_size(fs, &folder_abs);
        if has_nether_companion {
            size_bytes +=
                directory_size(fs, &server_dir.join(sibling_path(folder_rel, &nether_name)));
        }
        if has_end_companion {
            size_bytes += directory_size(fs, &server_dir.join(sibling_path(folder_rel, &end_name)));
        }

        worlds.push(DetectedWorld {
            name: name.clone(),
            folder_path: folder_rel.clone(),
            size_bytes,
            has_nether: has_nether_inline || has_nether_companion,
            has_end: has_end_inline || has_end_companion,
        });
    }

    // Source line 345-349: the configured level-name's world always sorts
    // first; everything else falls back to plain alphabetical order.
    worlds.sort_by(|a, b| {
        let a_first = a.name == configured_level_name;
        let b_first = b.name == configured_level_name;
        match (a_first, b_first) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    let default_world_name = worlds
        .first()
        .map(|w| w.name.clone())
        .unwrap_or_else(|| configured_level_name.to_string());
    (worlds, default_world_name)
}

/// A companion folder lives alongside its root world under the same
/// search-root prefix — `"creative"` -> `"creative_nether"`,
/// `"worlds/survival"` -> `"worlds/survival_nether"`.
fn sibling_path(folder_rel: &str, sibling_name: &str) -> String {
    match folder_rel.rsplit_once('/') {
        Some((prefix, _)) => format!("{prefix}/{sibling_name}"),
        None => sibling_name.to_string(),
    }
}

/// Port of `directorySizeBytes` (source line 496-510): recursively sums
/// file sizes under `path`, skipping dotfiles/dot-directories (source's
/// `.skipsHiddenFiles`; `.skipsPackageDescendants` has no Minecraft-world
/// equivalent here and is not modeled).
fn directory_size(fs: &dyn RawImportFileSystem, path: &Path) -> u64 {
    let mut total = 0u64;
    for entry in fs.list_dir(path) {
        if entry.name.starts_with('.') {
            continue;
        }
        let child = path.join(&entry.name);
        if entry.is_file {
            total += fs.file_size(&child);
        } else {
            total += directory_size(fs, &child);
        }
    }
    total
}

struct DetectedJavaFlavor {
    flavor: JavaServerFlavor,
    mc_version: Option<String>,
    loader_version: Option<String>,
    primary_jar_path: Option<String>,
}

/// Port of `detectJavaFlavor` (source line 370-437): NeoForge, then Forge,
/// then Fabric, then Purpur/Vanilla jar-name matches, then Paper as the
/// unconditional default. Every step returns on first match; only Paper's
/// step can hand back a `None` primary jar and version (an empty/jar-less
/// directory).
fn detect_java_flavor(fs: &dyn RawImportFileSystem, dir: &Path) -> DetectedJavaFlavor {
    let neo_base = dir.join("libraries/net/neoforged/neoforge");
    for name in list_names_sorted(fs, &neo_base) {
        if fs.is_file(&neo_base.join(&name).join("unix_args.txt")) {
            let mc_version = neoforge_minecraft_version(&name);
            return DetectedJavaFlavor {
                flavor: JavaServerFlavor::NeoForge,
                mc_version: Some(mc_version),
                loader_version: Some(name),
                primary_jar_path: None,
            };
        }
    }

    let forge_base = dir.join("libraries/net/minecraftforge/forge");
    for name in list_names_sorted(fs, &forge_base) {
        if fs.is_file(&forge_base.join(&name).join("unix_args.txt")) {
            // Source line 392-394: split on the *first* '-' only.
            let (mc_version, loader_version) = match name.split_once('-') {
                Some((mc, forge)) => (Some(mc.to_string()), forge.to_string()),
                None => (Some(name.clone()), name.clone()),
            };
            return DetectedJavaFlavor {
                flavor: JavaServerFlavor::Forge,
                mc_version,
                loader_version: Some(loader_version),
                primary_jar_path: None,
            };
        }
    }

    // Source line 402-404: `.skipsHiddenFiles` on this listing only.
    let root_files: Vec<String> = fs
        .list_dir(dir)
        .into_iter()
        .filter(|e| !e.name.starts_with('.'))
        .map(|e| e.name)
        .collect();

    if let Some(jar) = root_files.iter().find(|n| {
        let lower = n.to_lowercase();
        lower.starts_with("fabric-server-launch") && lower.ends_with(".jar")
    }) {
        let stem = strip_extension(jar);
        let mc_version = parse_fabric_mc_version(&stem);
        let loader_version = detect_fabric_loader_version(fs, dir);
        return DetectedJavaFlavor {
            flavor: JavaServerFlavor::Fabric,
            mc_version,
            loader_version,
            primary_jar_path: Some(jar.clone()),
        };
    }

    let jars: Vec<&String> = root_files
        .iter()
        .filter(|n| n.to_lowercase().ends_with(".jar"))
        .collect();

    if let Some(jar) = jars.iter().find(|n| n.to_lowercase().starts_with("purpur")) {
        return DetectedJavaFlavor {
            flavor: JavaServerFlavor::Purpur,
            mc_version: parse_jar_mc_version(&strip_extension(jar), "purpur-"),
            loader_version: None,
            primary_jar_path: Some((*jar).clone()),
        };
    }

    if let Some(jar) = jars
        .iter()
        .find(|n| n.to_lowercase().starts_with("minecraft_server"))
    {
        return DetectedJavaFlavor {
            flavor: JavaServerFlavor::Vanilla,
            mc_version: parse_jar_mc_version(&strip_extension(jar), "minecraft_server-"),
            loader_version: None,
            primary_jar_path: Some((*jar).clone()),
        };
    }

    // Source line 431-436: prefer a `paper*` jar, else whichever jar
    // exists, else `None` — Paper is always the fallback flavor.
    let paper_jar = jars
        .iter()
        .find(|n| n.to_lowercase().starts_with("paper"))
        .or_else(|| jars.first());
    let mc_version =
        paper_jar.and_then(|jar| parse_jar_mc_version(&strip_extension(jar), "paper-"));
    DetectedJavaFlavor {
        flavor: JavaServerFlavor::Paper,
        mc_version,
        loader_version: None,
        primary_jar_path: paper_jar.map(|jar| (*jar).clone()),
    }
}

/// Every version-directory/loader-directory listing `detectJavaFlavor`
/// walks uses an unfiltered, unsorted `contentsOfDirectory` and either
/// scans every entry (NeoForge/Forge) or sorts-and-takes-last (Fabric
/// loader). Sorting here first makes the scan deterministic without
/// changing which entry ultimately gets picked in either case.
fn list_names_sorted(fs: &dyn RawImportFileSystem, path: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs.list_dir(path).into_iter().map(|e| e.name).collect();
    names.sort();
    names
}

fn strip_extension(name: &str) -> String {
    match name.rfind('.') {
        Some(idx) => name[..idx].to_string(),
        None => name.to_string(),
    }
}

/// Source line 441-447.
fn parse_fabric_mc_version(stem: &str) -> Option<String> {
    let prefix = "fabric-server-launch-";
    if !stem.to_lowercase().starts_with(prefix) {
        return None;
    }
    let version = &stem[prefix.len()..];
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

/// Source line 450-456: lists `.fabric/server/libraries/net/fabricmc/fabric-loader/`,
/// sorts the entry names **lexicographically** (not semantically), and
/// takes the last. `"0.15.9"` sorts before `"0.9.0"` as strings, so a
/// directory tree with both picks the numerically older `"0.9.0"` — a
/// genuine MSC 1 quirk, preserved as-is per CLAUDE.md. See
/// `fixtures/raw-server-import/fabric-launcher-and-loader-version-lexicographic-quirk.json`.
fn detect_fabric_loader_version(fs: &dyn RawImportFileSystem, dir: &Path) -> Option<String> {
    let base = dir.join(".fabric/server/libraries/net/fabricmc/fabric-loader");
    let mut names: Vec<String> = fs.list_dir(&base).into_iter().map(|e| e.name).collect();
    names.sort();
    names.pop()
}

/// Source line 461-473: requires the stem to start with `prefix`
/// (case-insensitively); strips a trailing `-<numeric build>` if present.
/// A jar matched only via the looser `hasPrefix`/`?? jars.first` fallback
/// (e.g. an unmatched jar labelled Paper anyway) won't carry the exact
/// prefix here and yields `None`, not a guess.
fn parse_jar_mc_version(stem: &str, prefix: &str) -> Option<String> {
    let lower = stem.to_lowercase();
    if !lower.starts_with(&prefix.to_lowercase()) {
        return None;
    }
    let mut remainder = stem[prefix.len()..].to_string();
    if let Some(dash_idx) = remainder.rfind('-') {
        let after_dash = &remainder[dash_idx + 1..];
        if !after_dash.is_empty() && after_dash.chars().all(|c| c.is_ascii_digit()) {
            remainder.truncate(dash_idx);
        }
    }
    if remainder.is_empty() {
        None
    } else {
        Some(remainder)
    }
}

/// Port of `NeoForgeInstaller.minecraftVersion(forNeoForge:)`
/// (`NeoForgeInstaller.swift:224-231`). Splits on `-` first (a NeoForge
/// version directory name here is never suffixed, so `core` is normally
/// the whole string), parses `major.minor` from the first two dotted
/// components — components that don't parse as integers are silently
/// dropped, not just truncated at, matching Swift's `compactMap` exactly
/// — and renders `1.<major>.<minor>` unless `major >= 26` (Minecraft's
/// post-1.21 flat-year scheme), which renders `<major>.<minor>` directly.
fn neoforge_minecraft_version(version: &str) -> String {
    let core = version.split('-').next().unwrap_or(version);
    let comps: Vec<i64> = core
        .split('.')
        .filter_map(|s| s.parse::<i64>().ok())
        .collect();
    if comps.len() < 2 {
        return core.to_string();
    }
    let (major, minor) = (comps[0], comps[1]);
    if major >= 26 {
        if minor == 0 {
            major.to_string()
        } else {
            format!("{major}.{minor}")
        }
    } else if minor == 0 {
        format!("1.{major}")
    } else {
        format!("1.{major}.{minor}")
    }
}

/// Port of the single-root unwrap rule both `AddServerWizardView.performScan`
/// (line 2185-2193, the scan-time copy) and `resolvedImportDir`
/// (`AppViewModel+ServerImport.swift:478-494`, P5.20's mutating-import
/// copy) apply identically: if `root` contains exactly one subdirectory
/// and zero loose (non-hidden) files, scan/import from that subdirectory
/// instead of `root` itself.
pub fn resolve_unwrap_root(fs: &dyn RawImportFileSystem, root: &Path) -> PathBuf {
    let entries: Vec<RawScanEntry> = fs
        .list_dir(root)
        .into_iter()
        .filter(|e| !e.name.starts_with('.'))
        .collect();
    let dirs: Vec<&RawScanEntry> = entries.iter().filter(|e| !e.is_file).collect();
    let file_count = entries.iter().filter(|e| e.is_file).count();
    if dirs.len() == 1 && file_count == 0 {
        root.join(&dirs[0].name)
    } else {
        root.to_path_buf()
    }
}

/// Errors from the zip-extraction primitive P5.19's scan-time staging and
/// P5.20's mutating import share.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawImportError {
    EmptyDisplayName,
    EmptyDestinationName,
    PathSafety(String),
    DestinationExists { path: PathBuf },
    SourceNotFound { path: PathBuf },
    OpenZip(String),
    UnsafeZipEntry { name: String },
    UnsafeSymlink { path: PathBuf },
    Io(String),
}

impl fmt::Display for RawImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDisplayName => write!(f, "display name cannot be empty"),
            Self::EmptyDestinationName => {
                write!(f, "display name has no valid characters for a folder name")
            }
            Self::PathSafety(message) => write!(f, "{message}"),
            Self::DestinationExists { path } => {
                write!(f, "a server already exists at {}", path.display())
            }
            Self::SourceNotFound { path } => {
                write!(f, "import source not found: {}", path.display())
            }
            Self::OpenZip(message) => write!(f, "could not open zip archive: {message}"),
            Self::UnsafeZipEntry { name } => {
                write!(f, "zip entry escapes the destination directory: {name}")
            }
            Self::UnsafeSymlink { path } => {
                write!(f, "refusing to copy symlink: {}", path.display())
            }
            Self::Io(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for RawImportError {}

/// Extracts every entry of the zip at `zip_path` into `dest_root`,
/// rejecting (not silently skipping) any entry that is absolute, escapes
/// via `..`, or is a symlink — Rust-side hardening MSC 1's own
/// `/usr/bin/ditto -x -k` shell-out never had. `enclosed_name()` alone
/// isn't enough: the `zip` crate *relativizes* an absolute entry
/// (`/etc/passwd` -> `etc/passwd`) instead of refusing it, so
/// [`is_unsafe_zip_entry_name`] checks the raw name first.
fn extract_zip_traversal_safe(zip_path: &Path, dest_root: &Path) -> Result<(), RawImportError> {
    let file = fs::File::open(zip_path).map_err(|e| RawImportError::Io(e.to_string()))?;
    let mut archive = ZipArchive::new(file).map_err(|e| RawImportError::OpenZip(e.to_string()))?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| RawImportError::OpenZip(e.to_string()))?;
        let raw_name = entry.name().to_string();
        if is_unsafe_zip_entry_name(&raw_name) || is_symlink_zip_mode(entry.unix_mode()) {
            return Err(RawImportError::UnsafeZipEntry { name: raw_name });
        }
        let Some(enclosed) = entry.enclosed_name() else {
            return Err(RawImportError::UnsafeZipEntry { name: raw_name });
        };
        let dest = dest_root.join(&enclosed);
        if entry.is_dir() {
            fs::create_dir_all(&dest).map_err(|e| RawImportError::Io(e.to_string()))?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| RawImportError::Io(e.to_string()))?;
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|e| RawImportError::Io(e.to_string()))?;
        fs::write(&dest, &bytes).map_err(|e| RawImportError::Io(e.to_string()))?;
    }
    Ok(())
}

fn is_unsafe_zip_entry_name(name: &str) -> bool {
    if name.starts_with('/') || name.starts_with('\\') {
        return true;
    }
    let mut chars = name.chars();
    matches!((chars.next(), chars.next()), (Some(drive), Some(':')) if drive.is_ascii_alphabetic())
}

fn is_symlink_zip_mode(unix_mode: Option<u32>) -> bool {
    matches!(unix_mode, Some(mode) if mode & 0o170000 == 0o120000)
}

/// Scan-time zip source adapter: extracts `zip_path` into disposable
/// staging, applies [`resolve_unwrap_root`], scans the result, and always
/// removes staging — matching `performScan`'s `ditto` + unwrap + scan +
/// cleanup sequence (`AddServerWizardView.swift:2142-2199`), with real
/// zip-safety hardening in place of the oracle's un-hardened shell-out.
pub fn scan_zip_source(zip_path: &Path) -> Result<ScannedServerInfo, RawImportError> {
    let staging = std::env::temp_dir().join(format!("msc_raw_scan_{}", Uuid::new_v4()));
    fs::create_dir_all(&staging).map_err(|e| RawImportError::Io(e.to_string()))?;

    let result = extract_zip_traversal_safe(zip_path, &staging).map(|()| {
        let resolved = resolve_unwrap_root(&StdRawImportFileSystem, &staging);
        scan_server_directory(&StdRawImportFileSystem, &resolved)
    });

    let _ = fs::remove_dir_all(&staging);
    result
}
