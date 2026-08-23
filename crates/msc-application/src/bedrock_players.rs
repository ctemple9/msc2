//! Bedrock player identity, allowlist, and permissions services.
//!
//! Bedrock stores player records in LevelDB and names separately in a small
//! JSON cache.  The service keeps those concerns separate: a corrupt player
//! record is never turned into a fake player, while a missing name cache is a
//! normal state that can be filled after the next connection.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io;
use std::path::Path;

use msc_domain::bedrock::{
    AllowlistEntry, BedrockPermissionLevel, PermissionEntry, add_allowlist_entry, parse_allowlist,
    parse_permissions, player_display_name, player_identity_from_key, remove_allowlist_entry,
    remove_permission, set_permission,
};
use msc_infrastructure::atomic_write::atomic_write;
use msc_infrastructure::bedrock_leveldb::read_player_data;
use msc_infrastructure::bedrock_nbt::read_player_nbt;
use msc_infrastructure::fs::FileSystem;

const ALLOWLIST_FILE: &str = "allowlist.json";
const PERMISSIONS_FILE: &str = "permissions.json";
const NAME_CACHE_FILE: &str = "bedrock_names.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockPlayerRecord {
    pub xuid: String,
    pub name: String,
    pub has_stats: bool,
    pub inventory_items: usize,
}

#[derive(Debug)]
pub enum BedrockPlayerError {
    Io(io::Error),
    LevelDb(String),
    AtomicWrite(String),
    InvalidPermission(String),
}

impl fmt::Display for BedrockPlayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::LevelDb(error) => write!(f, "{error}"),
            Self::AtomicWrite(error) => write!(f, "{error}"),
            Self::InvalidPermission(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for BedrockPlayerError {}

impl From<io::Error> for BedrockPlayerError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn read_allowlist(fs: &dyn FileSystem, server_dir: &Path) -> Vec<AllowlistEntry> {
    fs.read(&server_dir.join(ALLOWLIST_FILE))
        .ok()
        .map(|bytes| parse_allowlist(&String::from_utf8_lossy(&bytes)))
        .unwrap_or_default()
}

pub fn mutate_allowlist(
    fs: &dyn FileSystem,
    server_dir: &Path,
    action: &str,
    name: &str,
) -> Result<Vec<AllowlistEntry>, BedrockPlayerError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(BedrockPlayerError::InvalidPermission(
            "player name cannot be empty".to_owned(),
        ));
    }
    let mut entries = read_allowlist(fs, server_dir);
    let changed = match action {
        "add" => add_allowlist_entry(&mut entries, name, None),
        "remove" => remove_allowlist_entry(&mut entries, name),
        _ => {
            return Err(BedrockPlayerError::InvalidPermission(format!(
                "unknown allowlist action: {action}"
            )));
        }
    };
    if changed {
        write_json(fs, &server_dir.join(ALLOWLIST_FILE), &entries)?;
    }
    Ok(entries)
}

pub fn read_permissions(fs: &dyn FileSystem, server_dir: &Path) -> Vec<PermissionEntry> {
    fs.read(&server_dir.join(PERMISSIONS_FILE))
        .ok()
        .and_then(|bytes| parse_permissions(&String::from_utf8_lossy(&bytes)).ok())
        .unwrap_or_default()
}

pub fn set_player_permission(
    fs: &dyn FileSystem,
    server_dir: &Path,
    xuid: &str,
    permission: &str,
) -> Result<Vec<PermissionEntry>, BedrockPlayerError> {
    let permission = match permission {
        "visitor" => BedrockPermissionLevel::Visitor,
        "member" => BedrockPermissionLevel::Member,
        "operator" => BedrockPermissionLevel::Operator,
        other => return Err(BedrockPlayerError::InvalidPermission(other.to_owned())),
    };
    let xuid = xuid.trim();
    if xuid.is_empty() {
        return Err(BedrockPlayerError::InvalidPermission(
            "xuid cannot be empty".to_owned(),
        ));
    }
    let mut entries = read_permissions(fs, server_dir);
    set_permission(&mut entries, xuid, permission);
    write_json(fs, &server_dir.join(PERMISSIONS_FILE), &entries)?;
    Ok(entries)
}

pub fn remove_player_permission(
    fs: &dyn FileSystem,
    server_dir: &Path,
    xuid: &str,
) -> Result<Vec<PermissionEntry>, BedrockPlayerError> {
    let mut entries = read_permissions(fs, server_dir);
    remove_permission(&mut entries, xuid.trim());
    write_json(fs, &server_dir.join(PERMISSIONS_FILE), &entries)?;
    Ok(entries)
}

pub fn load_name_cache(fs: &dyn FileSystem, server_dir: &Path) -> BTreeMap<String, String> {
    fs.read(&server_dir.join(NAME_CACHE_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn record_name(
    fs: &dyn FileSystem,
    server_dir: &Path,
    xuid: &str,
    name: &str,
) -> Result<BTreeMap<String, String>, BedrockPlayerError> {
    let xuid = xuid.trim();
    let name = name.trim();
    let mut cache = load_name_cache(fs, server_dir);
    if !xuid.is_empty() && !name.is_empty() {
        cache.insert(xuid.to_owned(), name.to_owned());
        write_json(fs, &server_dir.join(NAME_CACHE_FILE), &cache)?;
    }
    Ok(cache)
}

pub fn load_hidden(fs: &dyn FileSystem, server_dir: &Path) -> BTreeSet<String> {
    fs.read(&server_dir.join("bedrock_hidden.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Vec<String>>(&bytes).ok())
        .unwrap_or_default()
        .into_iter()
        .collect()
}

/// Scan the active Bedrock world's LevelDB.  The reader is intentionally
/// read-only; the service reports corrupt input instead of mutating a live
/// world database to make it look healthy.
pub fn discover_players(
    server_dir: &Path,
    level_name: &str,
    cache: &BTreeMap<String, String>,
) -> Result<Vec<BedrockPlayerRecord>, BedrockPlayerError> {
    let db = server_dir.join("worlds").join(level_name).join("db");
    let data =
        read_player_data(&db).map_err(|error| BedrockPlayerError::LevelDb(error.to_string()))?;
    let mut players = Vec::new();
    for (key, bytes) in data {
        let Some(identity) = player_identity_from_key(&key) else {
            continue;
        };
        let nbt = read_player_nbt(&bytes)
            .map_err(|error| BedrockPlayerError::LevelDb(error.to_string()))?;
        if nbt.stats.is_none() && nbt.inventory.is_empty() {
            continue;
        }
        let xuid = match identity {
            msc_domain::bedrock::BedrockPlayerIdentity::Local => "local".to_owned(),
            msc_domain::bedrock::BedrockPlayerIdentity::NumericXuid(xuid)
            | msc_domain::bedrock::BedrockPlayerIdentity::ServerUuid { xuid, .. } => xuid,
        };
        let name = cache
            .get(&xuid)
            .cloned()
            .or_else(|| {
                player_display_name(&player_identity_from_key(&key).unwrap()).map(str::to_owned)
            })
            .unwrap_or_else(|| xuid.clone());
        players.push(BedrockPlayerRecord {
            xuid: xuid.clone(),
            name,
            has_stats: nbt.stats.is_some(),
            inventory_items: nbt.inventory.len(),
        });
    }
    Ok(players)
}

fn write_json<T: serde::Serialize>(
    fs: &dyn FileSystem,
    path: &Path,
    value: &T,
) -> Result<(), BedrockPlayerError> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| BedrockPlayerError::AtomicWrite(error.to_string()))?;
    atomic_write(fs, path, &bytes)
        .map_err(|error| BedrockPlayerError::AtomicWrite(error.to_string()))
}
