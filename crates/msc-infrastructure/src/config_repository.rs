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
//!
//! [`load_app_config`]/[`save_app_config`] (P5.6) compose the above with
//! P5.4/P5.5's typed `AppConfig` schema — the first real consumer of this
//! primitive. `AppConfig`'s own wire format carries MSC 1's real version
//! marker, `config_version` (`corpus/configs/server-config-*.json` has it,
//! not `schemaVersion`), so these two functions also stamp this module's
//! own [`SCHEMA_VERSION_FIELD`] onto the encoded value before it reaches
//! [`save_config`]. `AppConfig::decode` ignores unrecognized keys, so the
//! extra field is inert on read; it exists only to satisfy this
//! primitive's own invariant, not because MSC 1 ever wrote such a key.
//!
//! [`find_corrupt_backups`]/[`server_count_in_backup`]/
//! [`restore_servers_from_backup`] (P5.7) port
//! `AppViewModel+ConfigRecovery.swift`'s corrupt-backup discovery and
//! recovery merge — the two functions that read the `.corrupt-*` siblings
//! [`load_config`] itself writes. The source's second recovery path,
//! `rescanAndImportServers` (walking the servers root for untracked
//! folders), is a separate, unrelated mechanism the same file happens to
//! also define — P5.22's job, not this module's.

use crate::atomic_write::{AtomicWriteError, atomic_write};
use crate::fs::FileSystem;
use crate::path_safety::lexically_normalize;
use msc_domain::app_config_schema::{AppConfig, DecodeError};
use serde_json::Value;
use std::collections::HashSet;
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

/// Result of [`load_app_config`]: the decoded, port-clamped `AppConfig`
/// plus whatever [`load_config`] reported about a corrupt-file recovery.
#[derive(Debug)]
pub struct AppConfigLoadOutcome {
    pub config: AppConfig,
    pub corrupt_backup_path: Option<PathBuf>,
}

#[derive(Debug)]
pub enum AppConfigLoadError {
    Load(ConfigLoadError),
    Decode(DecodeError),
}

impl fmt::Display for AppConfigLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppConfigLoadError::Load(err) => write!(f, "{err}"),
            AppConfigLoadError::Decode(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for AppConfigLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppConfigLoadError::Load(err) => Some(err),
            // `DecodeError` (msc-domain) doesn't implement `std::error::Error`
            // yet -- nothing to chain to.
            AppConfigLoadError::Decode(_) => None,
        }
    }
}

/// Stamps this module's [`SCHEMA_VERSION_FIELD`] onto an encoded
/// `AppConfig` value — see this module's doc comment for why `AppConfig`
/// itself doesn't carry that literal key.
fn stamp_schema_version(mut value: Value, config_version: i64) -> Value {
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            SCHEMA_VERSION_FIELD.to_string(),
            Value::from(config_version),
        );
    }
    value
}

/// Loads `path` as a typed `AppConfig` through [`load_config`], then
/// clamps `remote_api_port` to [`AppConfig::DEFAULT_REMOTE_API_PORT`] when
/// it falls outside `1..=65535` — `ConfigManager.init`'s port-validation
/// step (source lines 101-104), which sits after decode and before
/// `populateSecretsFromKeychain()`/`save()`, not inside
/// `AppConfig.init(from:)` itself (P5.5 already scoped that boundary).
/// The clamp is in-memory only: MSC 1 immediately re-`save()`s afterward,
/// but that re-save exists there to durably persist Keychain-populated
/// fields (out of this step's scope — P5.8/P5.9), not specifically to
/// persist the clamp, so this function leaves persisting a clamped value
/// to whichever caller next calls [`save_app_config`].
pub fn load_app_config(
    fs: &dyn FileSystem,
    path: &Path,
    defaults: &AppConfig,
    now: SystemTime,
) -> Result<AppConfigLoadOutcome, AppConfigLoadError> {
    let defaults_value = stamp_schema_version(defaults.encode(), defaults.config_version);
    let outcome = load_config(fs, path, &defaults_value, now).map_err(AppConfigLoadError::Load)?;
    let mut config =
        AppConfig::decode(&outcome.config, defaults).map_err(AppConfigLoadError::Decode)?;
    if !(1..=65535).contains(&config.remote_api_port) {
        config.remote_api_port = AppConfig::DEFAULT_REMOTE_API_PORT;
    }
    Ok(AppConfigLoadOutcome {
        config,
        corrupt_backup_path: outcome.corrupt_backup_path,
    })
}

/// Saves `config` through [`save_config`], stamping
/// [`SCHEMA_VERSION_FIELD`] onto the encoded value first — see this
/// module's doc comment.
pub fn save_app_config(
    fs: &dyn FileSystem,
    path: &Path,
    config: &AppConfig,
) -> Result<(), ConfigSaveError> {
    let value = stamp_schema_version(config.encode(), config.config_version);
    save_config(fs, path, &value)
}

/// Every `.corrupt-<nanos>` backup sibling of `config_path`, newest first —
/// matches `findCorruptBackups` (`AppViewModel+ConfigRecovery.swift` lines
/// 19-33). MSC 1 sorts by each file's real filesystem creation date;
/// `FileSystem` (P3.4) exposes no such metadata (the same gap `AuditLog`'s
/// own retention logic hit — P3.13's doc comment explains it), so this
/// reads the same ordering key out of the filename instead:
/// [`corrupt_backup_path`] already derives a backup's name from `now` when
/// it's written, so sorting on that embedded nanosecond suffix numerically
/// is exactly equivalent to sorting on creation time for any backup this
/// crate ever wrote. A sibling whose suffix doesn't parse as a number
/// sorts as oldest, mirroring MSC 1's `.distantPast` fallback for a file
/// whose creation date can't be read.
pub fn find_corrupt_backups(fs: &dyn FileSystem, config_path: &Path) -> Vec<PathBuf> {
    let parent = config_path.parent().unwrap_or_else(|| Path::new(""));
    let file_name = config_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let prefix = format!("{file_name}.corrupt-");

    let mut backups: Vec<(u128, PathBuf)> = fs
        .list(parent)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_string_lossy().into_owned();
            let suffix = name.strip_prefix(&prefix)?;
            Some((suffix.parse::<u128>().unwrap_or(0), path))
        })
        .collect();
    backups.sort_by_key(|(nanos, _)| std::cmp::Reverse(*nanos));
    backups.into_iter().map(|(_, path)| path).collect()
}

/// A cheap peek at how many servers a `.corrupt-*` backup holds, without
/// fully decoding it as a typed `AppConfig` — matches `serverCountInBackup`
/// (lines 36-42). `None` covers both an unreadable file and JSON with no
/// (or non-array) `servers` field, the same as MSC 1's own
/// `guard let ... else { return nil }` chain.
pub fn server_count_in_backup(fs: &dyn FileSystem, path: &Path) -> Option<usize> {
    let bytes = fs.read(path).ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    value.get("servers")?.as_array().map(Vec::len)
}

/// What [`restore_servers_from_backup`] found, matching `BackupRestoreResult`
/// (lines 46-50).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupRestoreResult {
    pub restored: usize,
    pub skipped: usize,
    pub error: Option<String>,
}

/// Decodes `backup_path` as a full `AppConfig` and merges its `servers`
/// into `live`, matching `restoreServersFromBackup` (lines 55-91). A
/// backup entry is skipped when its lexically standardized `server_dir`
/// or its `id` was already present in `live` *before this restore began*
/// — those two sets are captured once, up front, and never updated as
/// entries get merged in. That's not an oversight to "fix": it's why two
/// backup entries that duplicate each other (but collide with nothing in
/// `live`) both restore, rather than the second being treated as a
/// duplicate of the first (see the `duplicate-entries-in-backup`
/// fixture) — `existingPaths`/`existingIDs` are plain `let` constants in
/// the source, computed once before its `for server in decoded.servers`
/// loop and never reassigned inside it.
///
/// Unlike MSC 1, this doesn't save or reload anything itself — it hands
/// back the merged `AppConfig` (identical to `live` when nothing restored
/// or the backup couldn't be used) and leaves persisting it to whichever
/// caller wants [`save_app_config`], the same split [`load_app_config`]
/// already draws between decoding and its own in-memory port clamp.
pub fn restore_servers_from_backup(
    fs: &dyn FileSystem,
    backup_path: &Path,
    defaults: &AppConfig,
    live: &AppConfig,
) -> (AppConfig, BackupRestoreResult) {
    let bytes = match fs.read(backup_path) {
        Ok(bytes) => bytes,
        Err(_) => {
            return (
                live.clone(),
                BackupRestoreResult {
                    restored: 0,
                    skipped: 0,
                    error: Some("Could not read backup file.".to_string()),
                },
            );
        }
    };

    let decoded = serde_json::from_slice::<Value>(&bytes)
        .map_err(|e| e.to_string())
        .and_then(|value| AppConfig::decode(&value, defaults).map_err(|e| e.to_string()));
    let decoded = match decoded {
        Ok(decoded) => decoded,
        Err(message) => {
            return (
                live.clone(),
                BackupRestoreResult {
                    restored: 0,
                    skipped: 0,
                    error: Some(format!("Backup could not be decoded: {message}")),
                },
            );
        }
    };

    let existing_paths: HashSet<PathBuf> = live
        .servers
        .iter()
        .map(|server| lexically_normalize(Path::new(&server.server_dir)))
        .collect();
    let existing_ids: HashSet<&str> = live
        .servers
        .iter()
        .map(|server| server.id.as_str())
        .collect();

    let mut merged = live.clone();
    let mut restored = 0usize;
    let mut skipped = 0usize;
    for server in decoded.servers {
        let normalized = lexically_normalize(Path::new(&server.server_dir));
        if existing_paths.contains(&normalized) || existing_ids.contains(server.id.as_str()) {
            skipped += 1;
            continue;
        }
        merged.servers.push(server);
        restored += 1;
    }

    (
        merged,
        BackupRestoreResult {
            restored,
            skipped,
            error: None,
        },
    )
}
