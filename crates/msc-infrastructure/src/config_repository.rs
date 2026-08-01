//! `load_config`/`save_config`: the versioned-configuration primitive.
//!
//! Ported from the *mechanism* `ConfigManager.swift` demonstrates (`init`,
//! `reload`, `save`, lines 40-236) — not MSC 1's `AppConfig` schema itself,
//! which is Phase 5's job once the historical `server_config_swift.json`
//! corpus exists to migrate from. Three policies generalize past that one
//! schema: every saved config carries a [`SCHEMA_VERSION_FIELD`]; a config
//! file that fails to parse is preserved as a timestamped `.corrupt-<ts>`
//! sibling before defaults overwrite it (`ConfigManager.init`'s R3
//! behavior, lines 111-141), rather than losing the evidence; and because
//! this operates on `serde_json::Value` rather than a fixed struct, a
//! field this code doesn't recognize survives a read-modify-write round
//! trip instead of being silently dropped on save — unlike `AppConfig`
//! itself, whose synthesized `Decodable` drops unrecognized keys.
//! `msc2-engineering.md` §7 names exactly this failure mode for
//! `server.properties`; this primitive generalizes the fix to any
//! versioned config.

use crate::atomic_write::{AtomicWriteError, atomic_write};
use crate::fs::FileSystem;
use serde_json::Value;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The field every saved config must carry. `save_config` refuses to
/// write a config missing it rather than silently produce an unversioned
/// file a later migration step has no way to identify.
pub const SCHEMA_VERSION_FIELD: &str = "schemaVersion";

#[derive(Debug)]
pub struct ConfigLoadOutcome {
    pub config: Value,
    /// Set when the on-disk file existed but failed to parse — the path
    /// its original bytes were preserved to before `path` was overwritten
    /// with defaults, mirroring `ConfigManager.corruptConfigCopyPath`.
    pub corrupt_backup_path: Option<PathBuf>,
}

#[derive(Debug)]
pub enum ConfigLoadError {
    Io(io::Error),
    Save(ConfigSaveError),
}

impl fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigLoadError::Io(err) => write!(f, "{err}"),
            ConfigLoadError::Save(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ConfigLoadError {}

#[derive(Debug)]
pub enum ConfigSaveError {
    /// `config`'s top level has no `schemaVersion` key (or isn't an
    /// object at all).
    MissingSchemaVersion,
    Write(AtomicWriteError),
}

impl fmt::Display for ConfigSaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigSaveError::MissingSchemaVersion => {
                write!(f, "config is missing the {SCHEMA_VERSION_FIELD:?} field")
            }
            ConfigSaveError::Write(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ConfigSaveError {}

/// Loads `path` as JSON. Three outcomes, matching `ConfigManager.init`:
///
/// - `path` doesn't exist: `defaults` is written to `path` (so the next
///   load finds a real file, matching `init`'s own `else` branch) and
///   returned. No backup — there was never a file to preserve.
/// - `path` exists but fails to parse: its bytes are preserved at a
///   `.corrupt-<ts>` sibling computed from `now`, then `defaults` is
///   written to `path` and returned.
/// - `path` exists and parses: returned as-is, unknown fields and all —
///   `load_config` never rewrites a file it read cleanly.
pub fn load_config(
    fs: &dyn FileSystem,
    path: &Path,
    defaults: &Value,
    now: SystemTime,
) -> Result<ConfigLoadOutcome, ConfigLoadError> {
    if fs.stat(path).is_err() {
        save_config(fs, path, defaults).map_err(ConfigLoadError::Save)?;
        return Ok(ConfigLoadOutcome {
            config: defaults.clone(),
            corrupt_backup_path: None,
        });
    }

    let bytes = fs.read(path).map_err(ConfigLoadError::Io)?;
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(config) => Ok(ConfigLoadOutcome {
            config,
            corrupt_backup_path: None,
        }),
        Err(_) => {
            let backup_path = corrupt_backup_path(path, now);
            fs.write(&backup_path, &bytes)
                .map_err(ConfigLoadError::Io)?;
            save_config(fs, path, defaults).map_err(ConfigLoadError::Save)?;
            Ok(ConfigLoadOutcome {
                config: defaults.clone(),
                corrupt_backup_path: Some(backup_path),
            })
        }
    }
}

/// Writes `config` to `path` via [`atomic_write`], refusing to write a
/// config that's missing [`SCHEMA_VERSION_FIELD`] rather than let an
/// unversioned file land on disk.
pub fn save_config(
    fs: &dyn FileSystem,
    path: &Path,
    config: &Value,
) -> Result<(), ConfigSaveError> {
    let has_schema_version = config
        .as_object()
        .is_some_and(|obj| obj.contains_key(SCHEMA_VERSION_FIELD));
    if !has_schema_version {
        return Err(ConfigSaveError::MissingSchemaVersion);
    }

    let bytes = serde_json::to_vec_pretty(config).expect("serde_json::Value always serializes");
    atomic_write(fs, path, &bytes).map_err(ConfigSaveError::Write)
}

/// The sibling path a corrupt `path` is preserved to before being
/// overwritten with defaults — exposed so tests can predict it from a
/// fixed `now` without duplicating the naming rule, the same reason
/// `atomic_write::temp_path_for` is public.
pub fn corrupt_backup_path(path: &Path, now: SystemTime) -> PathBuf {
    let nanos = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!("{file_name}.corrupt-{nanos}"))
}
