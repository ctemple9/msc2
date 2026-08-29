//! `world_slots/{id}/{slot.json,world.zip,thumbnail.*}`: the I/O half of
//! `WorldSlotManager`'s directory layout, load/save, and archive-size stat
//! (P6.9's `msc_domain::world` owns the pure metadata/policy this module
//! calls into).
//!
//! Ported from `WorldSlotManager.swift`'s directory-helper functions
//! (`slotsDirectory`/`slotDirectory`/`zipURL`/`metadataURL`/
//! `activeSlotIDURL`, lines 73-103), `loadExplicitActiveSlotID`/
//! `setActiveSlotID` (lines 105-126), and `loadSlots`/`saveMetadata`
//! (lines 274-339). No destructive live-world swap yet — `activateSlot`'s
//! folder-removal/extraction step is P6.12+'s job, once running-server
//! guards exist at the application layer.
//!
//! `saveThumbnail` (source line 343-381) resizes and JPEG-encodes a real
//! image via AppKit — no fixture in `fixtures/world-slots`/
//! `fixtures/world-mutations` pins pixel output, and the field itself is
//! source's own "future use" (line 32's comment). This module ports only
//! the deterministic, testable half — `thumbnail_dest_size`'s aspect-
//! ratio-preserving bounding-box math — and stores whatever encoded bytes
//! the caller already produced verbatim; decoding/resizing a real image
//! is a client/UI-layer concern this crate has no fixture-backed reason
//! to take on yet. Flagged here rather than silently narrowed.

use crate::atomic_write::{AtomicWriteError, atomic_write};
use crate::fs::FileSystem;
use msc_domain::world::{self, WorldSlot};
use msc_domain::world_profile::{
    WorldGameplay, WorldGeneration, WorldIdentity, WorldProfile, WorldSafety, WorldSafetyState,
};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub fn slots_directory(server_dir: &Path) -> PathBuf {
    server_dir.join("world_slots")
}

pub fn slot_directory(server_dir: &Path, slot_id: &str) -> PathBuf {
    slots_directory(server_dir).join(slot_id)
}

pub fn zip_path(server_dir: &Path, slot_id: &str) -> PathBuf {
    slot_directory(server_dir, slot_id).join("world.zip")
}

pub fn metadata_path(server_dir: &Path, slot_id: &str) -> PathBuf {
    slot_directory(server_dir, slot_id).join("slot.json")
}

pub fn thumbnail_path(server_dir: &Path, slot_id: &str, file_name: &str) -> PathBuf {
    slot_directory(server_dir, slot_id).join(file_name)
}

pub fn active_marker_path(server_dir: &Path) -> PathBuf {
    slots_directory(server_dir).join("active_slot_id.txt")
}

/// `loadExplicitActiveSlotID(forServerDir:)` (source line 105-110): a
/// missing marker file, or one that's empty after trimming, is "no
/// explicit marker" — not an error.
pub fn load_explicit_active_slot_id(fs: &dyn FileSystem, server_dir: &Path) -> Option<String> {
    let bytes = fs.read(&active_marker_path(server_dir)).ok()?;
    let raw = String::from_utf8_lossy(&bytes);
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// `setActiveSlotID(_:forServerDir:)` (source line 112-126): `Some(id)`
/// creates `world_slots/` if needed and writes `"{id}\n"`; `None` removes
/// the marker file if present (already-absent is not an error, matching
/// source's own `fm.fileExists` guard before `removeItem`).
pub fn set_active_slot_id(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot_id: Option<&str>,
) -> std::io::Result<()> {
    let path = active_marker_path(server_dir);
    match slot_id {
        None => match fs.remove(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        },
        Some(id) => {
            fs.create_dir_all(&slots_directory(server_dir))?;
            fs.write(&path, format!("{id}\n").as_bytes())
        }
    }
}

/// `loadSlots(forServerDir:)` (source line 274-325): tolerant of a
/// non-directory entry, a missing `slot.json`, or an unparseable
/// `slot.json` in the same pass — each bad entry is skipped, not fatal to
/// the whole directory. Populates `zip_size_bytes` from a cheap stat of
/// `world.zip`, then sorts newest-`created_at`-first via
/// [`msc_domain::world::sort_newest_first`]. A missing `world_slots/`
/// directory returns an empty list, the ordinary state for any server
/// never yet stopped inside MSC 1/MSC 2.
pub fn load_slots(fs: &dyn FileSystem, server_dir: &Path) -> Vec<WorldSlot> {
    let slots_dir = slots_directory(server_dir);
    let Ok(entries) = fs.list(&slots_dir) else {
        return Vec::new();
    };

    let mut slots = Vec::new();
    for entry in entries {
        let Ok(meta) = fs.stat(&entry) else { continue };
        if !meta.is_dir {
            continue;
        }
        let Ok(bytes) = fs.read(&entry.join("slot.json")) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        let Ok(mut slot) = WorldSlot::decode(&value) else {
            continue;
        };
        if value.get("profile").is_none() {
            // Legacy MSC 1/early MSC 2 metadata has no profile. Persisting
            // the identity-only migration here means every later reader sees
            // an explicit, honest profile instead of silently inventing
            // gameplay defaults.
            let migrated = migrated_profile(&slot);
            let _ =
                save_metadata_with_profile_value(fs, server_dir, &slot, &encode_profile(&migrated));
        }
        if let Ok(zip_meta) = fs.stat(&entry.join("world.zip"))
            && zip_meta.is_file
        {
            slot.zip_size_bytes = zip_size_bytes(fs, &entry.join("world.zip"));
        }
        slots.push(slot);
    }

    world::sort_newest_first(&mut slots);
    slots
}

/// `loadSlots`'s zip-size stat (source line 314-318) reads the file
/// through [`FileSystem::read`] rather than a true byte-count-only stat —
/// this trait has no size-only `stat` field (see [`crate::fs::Metadata`]),
/// and every slot archive small enough for [`crate::archive`]'s own
/// [`crate::archive::MAX_TOTAL_UNCOMPRESSED_BYTES`] ceiling to allow is
/// small enough that reading it once to measure its length is not the
/// "extraction" cost this module's docs are careful to avoid — extraction
/// decompresses; this only measures the compressed file already on disk.
fn zip_size_bytes(fs: &dyn FileSystem, path: &Path) -> Option<i64> {
    fs.read(path).ok().map(|bytes| bytes.len() as i64)
}

/// `saveMetadata(_:serverDir:)` (source line 329-339): creates the slot
/// directory if needed, then writes `slot.json` — atomically, via
/// [`atomic_write`], strengthening source's plain `.atomic`-flagged
/// `Data.write` with the same stage-then-rename primitive every other
/// metadata writer in this crate already uses. Key order matches
/// source's `.sortedKeys` encoder option: `serde_json::Value`'s default
/// `Map` (no `preserve_order` feature enabled anywhere in this workspace)
/// is a `BTreeMap`, so `to_vec_pretty` already emits keys alphabetically.
pub fn save_metadata(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot: &WorldSlot,
) -> Result<(), AtomicWriteError> {
    let dir = slot_directory(server_dir, &slot.id);
    fs.create_dir_all(&dir).map_err(AtomicWriteError::Io)?;
    let mut value = slot.encode();
    preserve_existing_profile(fs, server_dir, &slot.id, &mut value);
    let bytes = serde_json::to_vec_pretty(&value).expect("serde_json::Value always serializes");
    atomic_write(fs, &metadata_path(server_dir, &slot.id), &bytes)
}

/// Reads the typed portion of the slot-local profile. Unknown keys remain in
/// the on-disk JSON and are deliberately not represented by this older Rust
/// type; `save_metadata` preserves them when it rewrites the slot.
pub fn load_profile(fs: &dyn FileSystem, server_dir: &Path, slot: &WorldSlot) -> WorldProfile {
    fs.read(&metadata_path(server_dir, &slot.id))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| value.get("profile").cloned())
        .and_then(|value| decode_profile(&value))
        .unwrap_or_else(|| migrated_profile(slot))
}

/// Persists the profile inside the slot's existing metadata document. The
/// caller supplies a JSON value so forward-compatible profile properties can
/// be copied without decoding and re-encoding them through an older schema.
pub fn save_profile_value(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot: &WorldSlot,
    profile: &Value,
) -> Result<(), AtomicWriteError> {
    save_metadata_with_profile_value(fs, server_dir, slot, profile)
}

/// Persists a typed profile while retaining any unrelated unknown profile
/// properties that were already present in the metadata document.
pub fn save_profile(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot: &WorldSlot,
    profile: &WorldProfile,
) -> Result<(), AtomicWriteError> {
    let mut value = encode_profile(profile);
    if let Some(existing) = fs
        .read(&metadata_path(server_dir, &slot.id))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|existing| existing.get("profile").cloned())
    {
        preserve_unknown_profile_properties(&existing, &mut value);
    }
    save_metadata_with_profile_value(fs, server_dir, slot, &value)
}

/// Encodes a profile for callers that need to place it in another metadata
/// document without losing unknown properties through a typed round trip.
pub fn profile_value(profile: &WorldProfile) -> Value {
    encode_profile(profile)
}

/// Copies the raw profile object with a slot operation. This is intentionally
/// a metadata copy rather than a typed copy: fields added by a newer agent
/// must survive duplicate, restore, and cross-edition conversion.
pub fn copy_profile(
    fs: &dyn FileSystem,
    source_server_dir: &Path,
    source_slot: &WorldSlot,
    target_server_dir: &Path,
    target_slot: &WorldSlot,
) -> Result<(), AtomicWriteError> {
    let profile = fs
        .read(&metadata_path(source_server_dir, &source_slot.id))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| value.get("profile").cloned())
        .unwrap_or_else(|| encode_profile(&migrated_profile(source_slot)));
    save_profile_value(fs, target_server_dir, target_slot, &profile)
}

fn save_metadata_with_profile_value(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot: &WorldSlot,
    profile: &Value,
) -> Result<(), AtomicWriteError> {
    let dir = slot_directory(server_dir, &slot.id);
    fs.create_dir_all(&dir).map_err(AtomicWriteError::Io)?;
    let mut value = slot.encode();
    value
        .as_object_mut()
        .expect("WorldSlot::encode always returns an object")
        .insert("profile".to_string(), profile.clone());
    let bytes = serde_json::to_vec_pretty(&value).expect("serde_json::Value always serializes");
    atomic_write(fs, &metadata_path(server_dir, &slot.id), &bytes)
}

fn preserve_existing_profile(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot_id: &str,
    value: &mut Value,
) {
    let Some(existing) = fs
        .read(&metadata_path(server_dir, slot_id))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|existing| existing.get("profile").cloned())
    else {
        return;
    };
    value
        .as_object_mut()
        .expect("WorldSlot::encode always returns an object")
        .insert("profile".to_string(), existing);
}

fn preserve_unknown_profile_properties(existing: &Value, replacement: &mut Value) {
    let (Some(existing), Some(replacement)) = (existing.as_object(), replacement.as_object_mut())
    else {
        return;
    };
    for (key, value) in existing {
        match replacement.get_mut(key) {
            Some(replacement_value) => {
                preserve_unknown_profile_properties(value, replacement_value)
            }
            None => {
                replacement.insert(key.clone(), value.clone());
            }
        }
    }
}

fn migrated_profile(slot: &WorldSlot) -> WorldProfile {
    let mut profile = WorldProfile::new();
    profile.identity = WorldIdentity {
        name: Some(slot.name.clone()),
        level_name: slot.world_level_name.clone(),
        seed: slot.world_seed.clone(),
    };
    profile
}

fn encode_profile(profile: &WorldProfile) -> Value {
    let mut root = Map::new();
    root.insert(
        "schema_version".to_string(),
        Value::from(profile.schema_version),
    );
    root.insert("identity".to_string(), encode_identity(&profile.identity));
    root.insert(
        "generation".to_string(),
        encode_generation(&profile.generation),
    );
    root.insert("gameplay".to_string(), encode_gameplay(&profile.gameplay));
    root.insert("safety".to_string(), encode_safety(&profile.safety));
    Value::Object(root)
}

fn encode_identity(identity: &WorldIdentity) -> Value {
    let mut map = Map::new();
    insert_optional_string(&mut map, "name", identity.name.as_deref());
    insert_optional_string(&mut map, "level_name", identity.level_name.as_deref());
    insert_optional_string(&mut map, "seed", identity.seed.as_deref());
    Value::Object(map)
}

fn encode_generation(generation: &WorldGeneration) -> Value {
    let mut map = Map::new();
    insert_optional_string(&mut map, "world_type", generation.world_type.as_deref());
    insert_optional_string(&mut map, "flat_preset", generation.flat_preset.as_deref());
    insert_optional_bool(&mut map, "structures", generation.structures);
    insert_optional_string(&mut map, "biome_source", generation.biome_source.as_deref());
    insert_optional_string(
        &mut map,
        "generator_options",
        generation.generator_options.as_deref(),
    );
    insert_optional_bool(&mut map, "bonus_chest", generation.bonus_chest);
    map.insert(
        "data_packs".to_string(),
        Value::Array(
            generation
                .data_packs
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        ),
    );
    Value::Object(map)
}

fn encode_gameplay(gameplay: &WorldGameplay) -> Value {
    let mut map = Map::new();
    insert_optional_string(&mut map, "difficulty", gameplay.difficulty.as_deref());
    insert_optional_string(
        &mut map,
        "default_game_mode",
        gameplay.default_game_mode.as_deref(),
    );
    insert_optional_bool(&mut map, "hardcore", gameplay.hardcore);
    insert_optional_bool(&mut map, "commands", gameplay.commands);
    map.insert(
        "gamerules".to_string(),
        Value::Object(
            gameplay
                .gamerules
                .iter()
                .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                .collect(),
        ),
    );
    insert_optional_bool(&mut map, "cheats", gameplay.cheats);
    map.insert(
        "experiments".to_string(),
        Value::Object(
            gameplay
                .experiments
                .iter()
                .map(|(key, value)| (key.clone(), Value::Bool(*value)))
                .collect(),
        ),
    );
    insert_optional_bool(&mut map, "coordinates", gameplay.coordinates);
    insert_optional_bool(&mut map, "starting_map", gameplay.starting_map);
    map.insert(
        "supported_toggles".to_string(),
        Value::Object(
            gameplay
                .supported_toggles
                .iter()
                .map(|(key, value)| (key.clone(), Value::Bool(*value)))
                .collect(),
        ),
    );
    Value::Object(map)
}

fn encode_safety(safety: &WorldSafety) -> Value {
    serde_json::json!({
        "state": safety.state.raw_value(),
        "reasons": safety.reasons,
    })
}

fn decode_profile(value: &Value) -> Option<WorldProfile> {
    let root = value.as_object()?;
    let schema_version = root.get("schema_version")?.as_u64()? as u32;
    let identity = decode_identity(root.get("identity")?)?;
    let generation = decode_generation(root.get("generation").unwrap_or(&Value::Null));
    let gameplay = decode_gameplay(root.get("gameplay").unwrap_or(&Value::Null));
    let safety = decode_safety(root.get("safety").unwrap_or(&Value::Null));
    Some(WorldProfile {
        schema_version,
        identity,
        generation,
        gameplay,
        safety,
    })
}

fn decode_identity(value: &Value) -> Option<WorldIdentity> {
    let map = value.as_object()?;
    Some(WorldIdentity {
        name: optional_string(map.get("name")),
        level_name: optional_string(map.get("level_name")),
        seed: optional_string(map.get("seed")),
    })
}

fn decode_generation(value: &Value) -> WorldGeneration {
    let Some(map) = value.as_object() else {
        return WorldGeneration::default();
    };
    WorldGeneration {
        world_type: optional_string(map.get("world_type")),
        flat_preset: optional_string(map.get("flat_preset")),
        structures: optional_bool(map.get("structures")),
        biome_source: optional_string(map.get("biome_source")),
        generator_options: optional_string(map.get("generator_options")),
        bonus_chest: optional_bool(map.get("bonus_chest")),
        data_packs: string_array(map.get("data_packs")),
    }
}

fn decode_gameplay(value: &Value) -> WorldGameplay {
    let Some(map) = value.as_object() else {
        return WorldGameplay::default();
    };
    WorldGameplay {
        difficulty: optional_string(map.get("difficulty")),
        default_game_mode: optional_string(map.get("default_game_mode")),
        hardcore: optional_bool(map.get("hardcore")),
        commands: optional_bool(map.get("commands")),
        gamerules: string_map(map.get("gamerules")),
        cheats: optional_bool(map.get("cheats")),
        experiments: bool_map(map.get("experiments")),
        coordinates: optional_bool(map.get("coordinates")),
        starting_map: optional_bool(map.get("starting_map")),
        supported_toggles: bool_map(map.get("supported_toggles")),
    }
}

fn decode_safety(value: &Value) -> WorldSafety {
    let Some(map) = value.as_object() else {
        return WorldSafety::default();
    };
    let state = match map.get("state").and_then(Value::as_str) {
        Some("safe") => WorldSafetyState::Safe,
        Some("achievement_disabled") => WorldSafetyState::AchievementDisabled,
        Some("unsupported") => WorldSafetyState::Unsupported,
        _ => WorldSafetyState::Unknown,
    };
    WorldSafety {
        state,
        reasons: string_array(map.get("reasons")),
    }
}

fn insert_optional_string(map: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        map.insert(key.to_string(), Value::String(value.to_string()));
    }
}

fn insert_optional_bool(map: &mut Map<String, Value>, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        map.insert(key.to_string(), Value::Bool(value));
    }
}

fn optional_string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(str::to_string)
}

fn optional_bool(value: Option<&Value>) -> Option<bool> {
    value.and_then(Value::as_bool)
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn string_map(value: Option<&Value>) -> std::collections::BTreeMap<String, String> {
    value
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| value.as_str().map(|value| (key.clone(), value.to_string())))
        .collect()
}

fn bool_map(value: Option<&Value>) -> std::collections::BTreeMap<String, bool> {
    value
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| value.as_bool().map(|value| (key.clone(), value)))
        .collect()
}

/// `thumbnail dest size`, the deterministic half of `saveThumbnail`
/// (source line 353-356): `min(maxW / srcW, maxH / srcH, 1.0)`, applied to
/// both dimensions and rounded — never upscales (`1.0` caps the scale),
/// preserves aspect ratio (one scale factor for both axes).
pub fn thumbnail_dest_size(src_width: f64, src_height: f64) -> (f64, f64) {
    const MAX_W: f64 = 800.0;
    const MAX_H: f64 = 450.0;
    let scale = (MAX_W / src_width).min(MAX_H / src_height).min(1.0);
    ((src_width * scale).round(), (src_height * scale).round())
}

/// Stores `encoded_bytes` (already resized/JPEG-encoded by the caller —
/// see the module doc) as `thumbnail.jpg` in the slot's directory and
/// updates its metadata, mirroring `saveThumbnail`'s two-step "write the
/// file, then persist the updated slot" shape (source line 372-380).
pub fn save_thumbnail(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot: &WorldSlot,
    encoded_bytes: &[u8],
) -> Result<WorldSlot, AtomicWriteError> {
    let dir = slot_directory(server_dir, &slot.id);
    fs.create_dir_all(&dir).map_err(AtomicWriteError::Io)?;
    let file_name = "thumbnail.jpg";
    atomic_write(fs, &dir.join(file_name), encoded_bytes)?;

    let mut updated = slot.clone();
    updated.thumbnail_file_name = Some(file_name.to_string());
    save_metadata(fs, server_dir, &updated)?;
    Ok(updated)
}
