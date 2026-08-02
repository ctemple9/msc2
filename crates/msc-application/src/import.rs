//! Existing Paper server import for the Phase 4 Java lifecycle slice.
//!
//! This intentionally does not copy, unzip, create world slots, or write
//! `server.properties`. Phase 4 only needs to register one already-existing
//! Paper directory so lifecycle work can start against real files.

use crate::lifecycle::{ImportedJavaServer, ServerId};
use msc_domain::identity::JavaServerFlavor;
use msc_domain::properties::ServerPropertiesModel;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

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
