//! Installs the bundled position feed for the active Bedrock world while BDS is stopped.

use msc_infrastructure::atomic_write::atomic_write;
use msc_infrastructure::fs::StdFileSystem;
use serde_json::{Value, json};
use std::fs;
use std::io;
use std::path::{Component, Path};

const PACK_ID: &str = "98328b20-538c-47fd-96e1-9ed1d057d675";
const PACK_VERSION: [u8; 3] = [0, 0, 1];
const MANIFEST: &str = include_str!("../assets/bedrock-map-feed/manifest.json");
const SCRIPT: &str = include_str!("../assets/bedrock-map-feed/scripts/main.js");

pub fn ensure_active_world_feed(server_dir: &Path) -> io::Result<()> {
    let properties = fs::read_to_string(server_dir.join("server.properties"))?;
    let level_name = properties
        .lines()
        .filter_map(|line| line.strip_prefix("level-name="))
        .map(str::trim)
        .find(|value| !value.is_empty())
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Bedrock level-name is missing")
        })?;
    if Path::new(level_name).components().count() != 1
        || !matches!(
            Path::new(level_name).components().next(),
            Some(Component::Normal(_))
        )
        || level_name.contains(['/', '\\'])
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Bedrock level-name must name one world folder",
        ));
    }

    let world_dir = server_dir.join("worlds").join(level_name);
    fs::create_dir_all(&world_dir)?;
    let packs_dir = server_dir.join("behavior_packs");
    let legacy_dir = packs_dir.join("msc-map-player-feed-proof");
    let pack_dir = if legacy_dir.join("manifest.json").is_file() {
        legacy_dir
    } else {
        packs_dir.join("msc-map-player-feed")
    };
    match fs::read(pack_dir.join("manifest.json")) {
        Ok(existing) => {
            let manifest: Value = serde_json::from_slice(&existing).map_err(io::Error::other)?;
            if manifest["header"]["uuid"] != PACK_ID {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "Bedrock map feed folder contains a different behavior pack",
                ));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if pack_dir.is_dir() && fs::read_dir(&pack_dir)?.next().is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "Bedrock map feed folder contains unmanaged files",
                ));
            }
        }
        Err(error) => return Err(error),
    }
    fs::create_dir_all(pack_dir.join("scripts"))?;
    atomic_write(
        &StdFileSystem,
        &pack_dir.join("manifest.json"),
        MANIFEST.as_bytes(),
    )
    .map_err(io::Error::other)?;
    atomic_write(
        &StdFileSystem,
        &pack_dir.join("scripts/main.js"),
        SCRIPT.as_bytes(),
    )
    .map_err(io::Error::other)?;

    let config_path = world_dir.join("world_behavior_packs.json");
    let mut entries: Vec<Value> = match fs::read(&config_path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error),
    };
    let mut changed = false;
    if let Some(entry) = entries.iter_mut().find(|entry| entry["pack_id"] == PACK_ID) {
        if entry["version"] != json!(PACK_VERSION) {
            entry["version"] = json!(PACK_VERSION);
            changed = true;
        }
    } else {
        entries.push(json!({"pack_id": PACK_ID, "version": PACK_VERSION}));
        changed = true;
    }
    if changed {
        let mut bytes = serde_json::to_vec_pretty(&entries).map_err(io::Error::other)?;
        bytes.push(b'\n');
        atomic_write(&StdFileSystem, &config_path, &bytes).map_err(io::Error::other)?;
    }
    Ok(())
}
