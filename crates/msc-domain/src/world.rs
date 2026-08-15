//! World-slot domain model: records, identity/level-name rules, active-slot
//! resolution, and backup association policy.
//!
//! Ported from `WorldSlotManager.swift`'s pure metadata/resolution rules
//! (P6.4's fixtures, `fixtures/world-slots/`) — the directory listing, zip/
//! unzip, and process work MSC 1 mixes into the same functions stays out of
//! this crate per `msc2-engineering.md`'s module-boundary rule.
//! `msc-infrastructure::world_store` (P6.10) owns the I/O half of
//! `loadSlots`/`saveMetadata`/`createSlot`'s archive step; `msc-application`
//! (P6.11+) owns orchestration (running-server guards, reconciliation).
//!
//! Three "build a slot's metadata" constructors exist because MSC 1 itself
//! has three, not because this port invented variety:
//! [`build_archived_slot`] mirrors `WorldSlotManager.createSlot` (name
//! untrimmed, `worldLevelName` via [`current_level_name`], `lastPlayedAt`
//! starts `nil`); [`build_fresh_slot`] mirrors `createFreshWorldSlot` (name
//! trimmed, `worldLevelName` via [`sanitized_world_level_name`], no
//! archive ever); [`build_bootstrap_slot`] mirrors
//! `AppViewModel+WorldSlots.ensureActiveWorldSlotExists`'s from-nothing
//! path, the one slot-creation path where `lastPlayedAt` is set at
//! creation instead of left `nil`.

use crate::identity::ServerType;
use serde_json::{Map, Value};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError(pub String);

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DecodeError {}

fn err(msg: impl Into<String>) -> DecodeError {
    DecodeError(msg.into())
}

fn present<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.get(key).filter(|x| !x.is_null())
}

fn req_str(v: &Value, key: &str) -> Result<String, DecodeError> {
    match present(v, key) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(err(format!("missing or invalid required field \"{key}\""))),
    }
}

fn opt_str(v: &Value, key: &str) -> Result<Option<String>, DecodeError> {
    match present(v, key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(err(format!("field \"{key}\" is not a string"))),
    }
}

fn insert_opt_str(m: &mut Map<String, Value>, key: &str, value: &Option<String>) {
    if let Some(v) = value {
        m.insert(key.to_string(), Value::String(v.clone()));
    }
}

/// `WorldSlot.swift`'s stored properties. `created_at`/`last_played_at` are
/// kept as ISO-8601 strings (never a `chrono` timestamp type): every
/// existing domain module already compares timestamps this way
/// (`app_config_schema.rs`), the workspace carries no date-arithmetic
/// dependency, and ISO-8601 UTC (`Z`-suffixed, zero-padded) strings compare
/// lexicographically in the same order they compare chronologically, which
/// is all [`sort_newest_first`]/[`resolve_active_slot_id`] need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSlot {
    pub id: String,
    pub name: String,
    pub created_at: String,
    /// `nil` until the slot has been activated at least once.
    pub last_played_at: Option<String>,
    pub thumbnail_file_name: Option<String>,
    /// `nil` for legacy imported slots until inferred or resaved.
    pub world_level_name: Option<String>,
    pub world_seed: Option<String>,
    /// Not stored in `slot.json` — computed on load from the zip file's
    /// size on disk (source line 46-47, 59). `decode`/`encode` never touch
    /// this field; `msc-infrastructure`'s repository sets it after a `stat`.
    pub zip_size_bytes: Option<i64>,
}

impl WorldSlot {
    /// `id`/`name`/`created_at` are the only fields the Swift struct has no
    /// `= nil` default for — a `slot.json` missing any of them, or any
    /// field carrying the wrong JSON type, fails the whole decode, matching
    /// `JSONDecoder`'s synthesized `Codable` conformance (not a `try?`
    /// swallow — the caller's `loadSlots` tolerance is what skips a failed
    /// entry, characterized in `fixtures/world-slots/
    /// load-slots-corrupt-entry-skipped.json`, not this function).
    pub fn decode(v: &Value) -> Result<WorldSlot, DecodeError> {
        Ok(WorldSlot {
            id: req_str(v, "id")?,
            name: req_str(v, "name")?,
            created_at: req_str(v, "created_at")?,
            last_played_at: opt_str(v, "last_played_at")?,
            thumbnail_file_name: opt_str(v, "thumbnail_file_name")?,
            world_level_name: opt_str(v, "world_level_name")?,
            world_seed: opt_str(v, "world_seed")?,
            zip_size_bytes: None,
        })
    }

    /// `zip_size_bytes` is intentionally excluded, matching source's
    /// `CodingKeys` (line 59).
    pub fn encode(&self) -> Value {
        let mut m = Map::new();
        m.insert("id".to_string(), Value::String(self.id.clone()));
        m.insert("name".to_string(), Value::String(self.name.clone()));
        m.insert(
            "created_at".to_string(),
            Value::String(self.created_at.clone()),
        );
        insert_opt_str(&mut m, "last_played_at", &self.last_played_at);
        insert_opt_str(&mut m, "thumbnail_file_name", &self.thumbnail_file_name);
        insert_opt_str(&mut m, "world_level_name", &self.world_level_name);
        insert_opt_str(&mut m, "world_seed", &self.world_seed);
        Value::Object(m)
    }
}

/// `loadSlots`'s final sort (source line 324): newest-`created_at` first,
/// independent of whatever order the caller discovered entries on disk.
pub fn sort_newest_first(slots: &mut [WorldSlot]) {
    slots.sort_by(|a, b| b.created_at.cmp(&a.created_at));
}

/// `resolvedActiveSlotID`'s fallback chain (source line 128-144):
/// 1. `explicit_marker`, if it names a slot present in `slots`.
/// 2. Otherwise the most-recently-played slot (`last_played_at`), if any.
/// 3. Otherwise the newest-created slot.
/// 4. `None` if `slots` is empty.
///
/// Takes already-loaded `slots` and an already-read marker rather than
/// touching disk itself — `msc-infrastructure` supplies both.
pub fn resolve_active_slot_id(
    slots: &[WorldSlot],
    explicit_marker: Option<&str>,
) -> Option<String> {
    if slots.is_empty() {
        return None;
    }

    if let Some(explicit) = explicit_marker
        && slots.iter().any(|s| s.id == explicit)
    {
        return Some(explicit.to_string());
    }

    if let Some(best) = slots
        .iter()
        .filter(|s| s.last_played_at.is_some())
        .max_by(|a, b| a.last_played_at.cmp(&b.last_played_at))
    {
        return Some(best.id.clone());
    }

    slots
        .iter()
        .max_by(|a, b| a.created_at.cmp(&b.created_at))
        .map(|s| s.id.clone())
}

/// `currentLevelName(for:)` (source line 154-165): the *unsanitized*
/// current level-name identity — Java reads `server.properties`
/// `level-name` (falls back to the literal `"world"`), Bedrock reads its
/// own properties model's `levelName` (falls back to `"Bedrock level"`).
/// `raw` is whatever the caller already read from the relevant properties
/// file/model; reading that file is `msc-infrastructure`'s job.
pub fn current_level_name(server_type: ServerType, raw: Option<&str>) -> String {
    let trimmed = raw.unwrap_or("").trim();
    let fallback = match server_type {
        ServerType::Java => "world",
        ServerType::Bedrock => "Bedrock level",
    };
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

/// `sanitizedWorldLevelName(_:fallback:)` (source line 168-185). Exists to
/// undo BDS's habit of stripping `=` when it turns a level-name into a
/// folder name — see `fixtures/world-slots/
/// sanitized-level-name-strips-invalid-characters.json` for the Realm-
/// export case this specifically fixes.
pub fn sanitized_world_level_name(raw: &str, fallback: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return fallback.to_string();
    }

    let is_invalid = |c: char| "/\\:\n\r\t=".contains(c);
    let collapsed: String = trimmed
        .split(is_invalid)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let collapsed = collapsed.trim_matches(|c| c == '.' || c == ' ');

    if collapsed.is_empty() {
        fallback.to_string()
    } else {
        collapsed.to_string()
    }
}

/// `worldFolderNames(for:)`'s *candidate* half (source line 244-270):
/// which folder names are relevant for a server type/level-name, before
/// filtering to what actually exists on disk. Java: the level-name folder
/// plus its `_nether`/`_the_end` siblings. Bedrock: the fixed `worlds`
/// directory name. Filtering these candidates down to ones that exist is
/// `msc-infrastructure`'s job (source's own `fm.fileExists` checks).
pub fn world_folder_candidates(server_type: ServerType, level_name: &str) -> Vec<String> {
    match server_type {
        ServerType::Bedrock => vec!["worlds".to_string()],
        ServerType::Java => vec![
            level_name.to_string(),
            format!("{level_name}_nether"),
            format!("{level_name}_the_end"),
        ],
    }
}

fn normalized_seed(seed: Option<&str>) -> Option<String> {
    seed.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `WorldSlotManager.createSlot`'s metadata construction (source line
/// 391-417), minus the zip/write steps `msc-infrastructure` owns. Unlike
/// [`build_fresh_slot`], `name` is stored exactly as given — source never
/// trims it here (`name: name`, line 412) — and `world_level_name` comes
/// from [`current_level_name`] (unsanitized), not
/// [`sanitized_world_level_name`].
pub fn build_archived_slot(
    id: String,
    name: &str,
    seed: Option<&str>,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    created_at: String,
) -> WorldSlot {
    WorldSlot {
        id,
        name: name.to_string(),
        created_at,
        last_played_at: None,
        thumbnail_file_name: None,
        world_level_name: Some(current_level_name(server_type, raw_level_name)),
        world_seed: normalized_seed(seed),
        zip_size_bytes: None,
    }
}

/// `createFreshWorldSlot`'s metadata construction (source line 549-575):
/// no world data is ever written for a fresh slot (it generates on first
/// activation), so this has no I/O-bearing counterpart — the whole
/// function lives here. `name` is trimmed; `world_level_name` runs through
/// [`sanitized_world_level_name`], which can differ from the trimmed name
/// when it contains characters BDS/Java can't use in a folder name.
pub fn build_fresh_slot(
    id: String,
    name: &str,
    seed: Option<&str>,
    server_type: ServerType,
    created_at: String,
) -> WorldSlot {
    let trimmed_name = name.trim().to_string();
    let fallback = match server_type {
        ServerType::Java => "world",
        ServerType::Bedrock => "Bedrock level",
    };
    let level_name = sanitized_world_level_name(&trimmed_name, fallback);
    WorldSlot {
        id,
        name: trimmed_name,
        created_at,
        last_played_at: None,
        thumbnail_file_name: None,
        world_level_name: Some(level_name),
        world_seed: normalized_seed(seed),
        zip_size_bytes: None,
    }
}

/// `defaultPersistentSlotName(for:)` (source line 51-61): the display name
/// [`build_bootstrap_slot`] uses. Reads its own independent property value
/// (not routed through [`current_level_name`]'s fallback) with its own
/// fallback string, `"World 1"` — and only Java capitalizes the first
/// letter; Bedrock passes a non-blank value through unchanged.
pub fn default_persistent_slot_name(
    server_type: ServerType,
    raw_level_name: Option<&str>,
) -> String {
    let trimmed = raw_level_name.unwrap_or("").trim();
    if trimmed.is_empty() {
        return "World 1".to_string();
    }
    match server_type {
        ServerType::Bedrock => trimmed.to_string(),
        ServerType::Java => {
            let mut chars = trimmed.chars();
            match chars.next() {
                None => trimmed.to_string(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }
    }
}

/// `AppViewModel+WorldSlots.ensureActiveWorldSlotExists`'s from-nothing
/// bootstrap path (source line 63-101): the one slot-creation path where
/// `last_played_at` is set to the creation time instead of left `None` —
/// this bootstrap slot is being marked as already-active/already-played on
/// creation. `raw_level_name` is the same underlying properties read used
/// for both [`default_persistent_slot_name`] (the slot's display name) and
/// [`current_level_name`] (its `world_level_name`) — source reads it twice
/// through two different property-manager calls that resolve to the same
/// value, so this pure half takes it once.
pub fn build_bootstrap_slot(
    id: String,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    created_at: String,
) -> WorldSlot {
    WorldSlot {
        id,
        name: default_persistent_slot_name(server_type, raw_level_name),
        created_at: created_at.clone(),
        last_played_at: Some(created_at),
        thumbnail_file_name: None,
        world_level_name: Some(current_level_name(server_type, raw_level_name)),
        world_seed: None,
        zip_size_bytes: None,
    }
}

/// `effectiveBackupAssociation`'s result (source
/// `AppViewModel+Backups.swift` line 143-163): which slot, if any, a
/// backup is associated with.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackupAssociation {
    pub slot_id: Option<String>,
    pub slot_name: Option<String>,
    pub world_seed: Option<String>,
}

/// `effectiveBackupAssociation(for:explicitSlotId:explicitSlotName:)`
/// (source line 143-163): an explicit, non-blank slot id always wins —
/// its name/seed are looked up (and trimmed-to-`None`-if-blank) from
/// `slots` rather than trusted from the caller. Otherwise falls back to
/// `active_slot_id` (the already-resolved active slot, source's
/// `activeWorldSlotMetadata`), and finally to no association at all.
pub fn effective_backup_association(
    slots: &[WorldSlot],
    active_slot_id: Option<&str>,
    explicit_slot_id: Option<&str>,
    explicit_slot_name: Option<&str>,
) -> BackupAssociation {
    let trimmed_explicit_id = explicit_slot_id.map(str::trim).filter(|s| !s.is_empty());
    if let Some(explicit_id) = trimmed_explicit_id {
        let trimmed_seed = slots
            .iter()
            .find(|s| s.id == explicit_id)
            .and_then(|s| s.world_seed.as_deref());
        return BackupAssociation {
            slot_id: Some(explicit_id.to_string()),
            slot_name: normalized_seed(explicit_slot_name),
            world_seed: normalized_seed(trimmed_seed),
        };
    }

    if let Some(active_id) = active_slot_id
        && let Some(slot) = slots.iter().find(|s| s.id == active_id)
    {
        return BackupAssociation {
            slot_id: Some(slot.id.clone()),
            slot_name: Some(slot.name.clone()),
            world_seed: slot.world_seed.clone(),
        };
    }

    BackupAssociation::default()
}
