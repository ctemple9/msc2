//! Java player-profile discovery and the small sidecar store used to hide
//! profiles from the client list.
//!
//! The Java player data files are read from both layouts used by Minecraft
//! servers.  This module owns the filesystem orchestration; NBT field
//! extraction remains in `msc_domain::player_nbt`.

use msc_domain::identity::ServerType;
use msc_domain::player_nbt::{InventoryItem, PlayerStats, read_all};
use msc_infrastructure::atomic_write::atomic_write;
use msc_infrastructure::fs::FileSystem;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use uuid::Uuid;

const HIDDEN_FILE: &str = "java_hidden.json";

#[derive(Debug)]
pub enum PlayerProfileError {
    ProfileNotFound,
    UsernameUnknown,
    Io(io::Error),
}

impl fmt::Display for PlayerProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProfileNotFound => write!(f, "player profile was not found"),
            Self::UsernameUnknown => write!(f, "player username is unknown"),
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for PlayerProfileError {}

#[derive(Debug, Clone, PartialEq)]
pub struct JavaPlayerProfile {
    pub uuid: Uuid,
    pub username: Option<String>,
    pub dat_file_path: PathBuf,
    pub last_modified: SystemTime,
    pub is_online: bool,
    pub is_op: bool,
    pub is_hidden: bool,
    pub stats: Option<PlayerStats>,
    pub inventory: Vec<InventoryItem>,
}

#[derive(Debug, Deserialize)]
struct UsercacheEntry {
    name: String,
    uuid: String,
}

#[derive(Debug, Deserialize)]
struct OpsEntry {
    uuid: String,
}

/// Loads every Java profile in the active world, including its eagerly-read
/// stats and inventory.  Sidecar JSON is deliberately best-effort because
/// Minecraft may not have created it yet, or it may be left malformed by a
/// server/plugin update just as MSC 1's `try?` reads tolerated.
pub fn load_player_profiles(
    fs: &dyn FileSystem,
    server_dir: &Path,
    output_reducer: &crate::output_reducer::JavaOutputReducer,
) -> Result<Vec<JavaPlayerProfile>, PlayerProfileError> {
    let raw_level_name = crate::worlds::read_java_level_name(fs, server_dir);
    let level_name =
        msc_domain::world::current_level_name(ServerType::Java, raw_level_name.as_deref());
    let username_by_uuid = read_usercache(fs, server_dir);
    let op_uuids = read_ops(fs, server_dir);
    let hidden_uuids = read_hidden(fs, server_dir);
    let online_names = output_reducer.online_players();

    let mut profiles = Vec::new();
    let mut seen = BTreeSet::new();
    for directory in player_data_dirs(server_dir, &level_name) {
        let exists = match fs.stat(&directory) {
            Ok(metadata) => metadata.is_dir,
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => return Err(PlayerProfileError::Io(error)),
        };
        if !exists {
            continue;
        }

        let entries = match fs.list(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(PlayerProfileError::Io(error)),
        };
        for path in entries {
            let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !filename.ends_with(".dat") || filename.ends_with(".dat_old") {
                continue;
            }
            let Some(stem) = filename.strip_suffix(".dat") else {
                continue;
            };
            let Ok(uuid) = Uuid::parse_str(stem) else {
                continue;
            };
            if !seen.insert(uuid) {
                continue;
            }

            let dat_bytes = fs.read(&path).map_err(PlayerProfileError::Io)?;
            let (stats, inventory) = read_all(&dat_bytes);
            let username = username_by_uuid.get(&uuid).cloned();
            let is_online = username
                .as_deref()
                .is_some_and(|name| online_names.iter().any(|online| online == name));
            let last_modified = fs
                .stat(&path)
                .map(|metadata| metadata.modified)
                .unwrap_or(SystemTime::UNIX_EPOCH);

            profiles.push(JavaPlayerProfile {
                uuid,
                username,
                dat_file_path: path,
                last_modified,
                is_online,
                is_op: op_uuids.contains(&uuid),
                is_hidden: hidden_uuids.contains(&uuid.to_string()),
                stats,
                inventory,
            });
        }
    }

    Ok(profiles)
}

/// Returns whether the UUID is in the server-root hidden-profile sidecar.
pub fn is_hidden(fs: &dyn FileSystem, server_dir: &Path, uuid: &Uuid) -> bool {
    read_hidden(fs, server_dir).contains(&uuid.to_string())
}

/// Adds a UUID to the server-root hidden-profile sidecar atomically.
pub fn hide(fs: &dyn FileSystem, server_dir: &Path, uuid: &Uuid) -> Result<(), PlayerProfileError> {
    let mut hidden = read_hidden(fs, server_dir);
    hidden.insert(uuid.to_string());
    write_hidden(fs, server_dir, &hidden)
}

/// Removes a UUID from the server-root hidden-profile sidecar atomically.
pub fn unhide(
    fs: &dyn FileSystem,
    server_dir: &Path,
    uuid: &Uuid,
) -> Result<(), PlayerProfileError> {
    let mut hidden = read_hidden(fs, server_dir);
    hidden.remove(&uuid.to_string());
    write_hidden(fs, server_dir, &hidden)
}

/// Resolves the single player-data directory used by a file mutation.
/// Unlike profile discovery, which scans both layouts, mutations follow the
/// first directory that exists so the source and destination stay together.
pub fn resolve_player_data_dir(
    fs: &dyn FileSystem,
    server_dir: &Path,
    level_name: &str,
) -> PathBuf {
    let candidates = player_data_dirs(server_dir, level_name);
    candidates
        .into_iter()
        .find(|candidate| fs.list(candidate).is_ok())
        .unwrap_or_else(|| server_dir.join(level_name).join("playerdata"))
}

/// Builds the lowercase Minecraft player-data filename for a UUID.
pub fn dat_path(uuid: &Uuid, player_data_dir: &Path) -> PathBuf {
    player_data_dir.join(format!("{}.dat", uuid.to_string().to_lowercase()))
}

/// Copies one player's data to another UUID, overwriting the destination.
pub fn copy_player_data(
    source_uuid: Uuid,
    dest_uuid: Uuid,
    player_data_dir: &Path,
    fs: &dyn FileSystem,
) -> Result<(), PlayerProfileError> {
    let source = fs
        .read(&dat_path(&source_uuid, player_data_dir))
        .map_err(map_profile_file_error)?;
    fs.write(&dat_path(&dest_uuid, player_data_dir), &source)
        .map_err(PlayerProfileError::Io)
}

/// Deletes one player's data file.
pub fn delete_player_data(
    uuid: Uuid,
    player_data_dir: &Path,
    fs: &dyn FileSystem,
) -> Result<(), PlayerProfileError> {
    fs.remove(&dat_path(&uuid, player_data_dir))
        .map_err(map_profile_file_error)
}

/// Copies one player's data to a new random UUID and returns that UUID.
pub fn duplicate_player_data(
    uuid: Uuid,
    player_data_dir: &Path,
    fs: &dyn FileSystem,
) -> Result<Uuid, PlayerProfileError> {
    let new_uuid = Uuid::new_v4();
    copy_player_data(uuid, new_uuid, player_data_dir, fs)?;
    Ok(new_uuid)
}

/// Copies a player's data to the UUID Minecraft derives for offline mode.
pub fn migrate_to_offline_uuid(
    profile: &JavaPlayerProfile,
    player_data_dir: &Path,
    fs: &dyn FileSystem,
) -> Result<Uuid, PlayerProfileError> {
    let username = profile
        .username
        .as_deref()
        .filter(|username| !username.is_empty())
        .ok_or(PlayerProfileError::UsernameUnknown)?;
    let target_uuid = msc_domain::player_nbt::offline_uuid(username);
    copy_player_data(profile.uuid, target_uuid, player_data_dir, fs)?;
    Ok(target_uuid)
}

/// Copies a player's data to an explicitly chosen UUID.
pub fn migrate_to_uuid(
    profile: &JavaPlayerProfile,
    target_uuid: Uuid,
    player_data_dir: &Path,
    fs: &dyn FileSystem,
) -> Result<(), PlayerProfileError> {
    copy_player_data(profile.uuid, target_uuid, player_data_dir, fs)
}

fn player_data_dirs(server_dir: &Path, level_name: &str) -> [PathBuf; 2] {
    let world_dir = server_dir.join(level_name);
    [
        world_dir.join("playerdata"),
        world_dir.join("players").join("data"),
    ]
}

fn read_usercache(fs: &dyn FileSystem, server_dir: &Path) -> BTreeMap<Uuid, String> {
    let Ok(bytes) = fs.read(&server_dir.join("usercache.json")) else {
        return BTreeMap::new();
    };
    let Ok(entries) = serde_json::from_slice::<Vec<UsercacheEntry>>(&bytes) else {
        return BTreeMap::new();
    };
    entries
        .into_iter()
        .filter_map(|entry| {
            Uuid::parse_str(&entry.uuid)
                .ok()
                .map(|uuid| (uuid, entry.name))
        })
        .collect()
}

fn read_ops(fs: &dyn FileSystem, server_dir: &Path) -> BTreeSet<Uuid> {
    let Ok(bytes) = fs.read(&server_dir.join("ops.json")) else {
        return BTreeSet::new();
    };
    let Ok(entries) = serde_json::from_slice::<Vec<OpsEntry>>(&bytes) else {
        return BTreeSet::new();
    };
    entries
        .into_iter()
        .filter_map(|entry| Uuid::parse_str(&entry.uuid).ok())
        .collect()
}

fn read_hidden(fs: &dyn FileSystem, server_dir: &Path) -> BTreeSet<String> {
    let Ok(bytes) = fs.read(&server_dir.join(HIDDEN_FILE)) else {
        return BTreeSet::new();
    };
    serde_json::from_slice::<Vec<String>>(&bytes)
        .map(|entries| entries.into_iter().collect())
        .unwrap_or_default()
}

fn write_hidden(
    fs: &dyn FileSystem,
    server_dir: &Path,
    hidden: &BTreeSet<String>,
) -> Result<(), PlayerProfileError> {
    let bytes = serde_json::to_vec_pretty(hidden)
        .map_err(|error| PlayerProfileError::Io(io::Error::other(error)))?;
    atomic_write(fs, &server_dir.join(HIDDEN_FILE), &bytes)
        .map_err(|error| PlayerProfileError::Io(io::Error::other(error.to_string())))
}

fn map_profile_file_error(error: io::Error) -> PlayerProfileError {
    if error.kind() == io::ErrorKind::NotFound {
        PlayerProfileError::ProfileNotFound
    } else {
        PlayerProfileError::Io(error)
    }
}
