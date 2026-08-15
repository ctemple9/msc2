//! Backup filesystem I/O: directory layout, listing, sidecar read/write,
//! deletion, and pruning — the file-facing half of
//! `AppViewModel+Backups.swift`'s `loadBackupsForSelectedServer`,
//! `readBackupMeta`/`writeBackupMeta`, `deleteBackup`, and
//! `pruneAutoBackupsIfNeeded` that `msc_domain::backup`'s pure
//! filename/sidecar rules never themselves touch a filesystem to
//! implement. Mirrors `world_store.rs`'s own split: this module is the
//! I/O half, `msc_domain::backup` is the policy half.
//!
//! `verified` (on [`BackupEntry`], and as this module's pruning floor) is
//! computed live via [`crate::archive::validate_archive_safety`] rather
//! than cached in the sidecar — a Phase 6 correction with no Swift
//! counterpart at all
//! (`fixtures/backup-restore/backup-verification-required-before-listing-or-restore-eligibility-phase6-correction.json`):
//! a failed or unverified archive is still listed, never hidden, but
//! never counted toward the "at least one verified backup survives
//! pruning" floor either. Computing it fresh on every call — rather than
//! persisting a flag that could go stale — is what "excluded ... until
//! re-verified" means literally: the check simply runs again next time.

use crate::archive;
use crate::atomic_write::atomic_write;
use crate::fs::FileSystem;
use msc_domain::backup::{self, BackupMeta};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// `ConfigManager.backupsDirectoryURL(forServerDirectory:)`
/// (`ConfigManager.swift:276-279`).
pub fn backups_dir(server_dir: &Path) -> PathBuf {
    server_dir.join("backups")
}

/// `sidecarURL(for:)` (source line 505-507):
/// `deletingPathExtension().appendingPathExtension("meta.json")`.
pub fn sidecar_path(zip_path: &Path) -> PathBuf {
    zip_path.with_extension("meta.json")
}

/// One backup ZIP as `loadBackupsForSelectedServer` assembles it
/// (`BackupItem`, `AppModels.swift:219-241`) plus this phase's own
/// `verified` field (see the module doc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupEntry {
    pub zip_path: PathBuf,
    pub filename: String,
    pub display_name: String,
    pub file_size: Option<u64>,
    pub modified: Option<SystemTime>,
    pub server_id: Option<String>,
    pub server_display_name: Option<String>,
    pub slot_id: Option<String>,
    pub slot_name: Option<String>,
    pub trigger_reason: String,
    pub verified: bool,
}

pub fn backups_directory_exists(fs: &dyn FileSystem, server_dir: &Path) -> bool {
    matches!(fs.stat(&backups_dir(server_dir)), Ok(m) if m.is_dir)
}

/// `readBackupMeta(forBackupURL:)` (source line 474-487): a missing
/// sidecar or one that fails to decode both return `None` silently —
/// distinguishing "missing" from "malformed" is the caller's log
/// message's job (this phase's ported callers have none; see
/// `msc-application::backups`'s own module doc), not this function's.
pub fn read_sidecar(fs: &dyn FileSystem, zip_path: &Path) -> Option<BackupMeta> {
    let bytes = fs.read(&sidecar_path(zip_path)).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    BackupMeta::decode(&value).ok()
}

/// `writeBackupMeta(_:forBackupURL:)` (source line 492-502): failures are
/// the caller's to swallow (source logs a warning and returns; this port
/// returns the `io::Result` instead of hiding it, since this crate has no
/// logging sink of its own — `msc-application::backups::create_backup`
/// (P6.16) is where that "log, don't fail the backup" policy actually
/// lives).
pub fn write_sidecar(fs: &dyn FileSystem, zip_path: &Path, meta: &BackupMeta) -> io::Result<()> {
    let dir = zip_path.parent().unwrap_or_else(|| Path::new("."));
    fs.create_dir_all(dir)?;
    let bytes = serde_json::to_vec_pretty(&meta.encode()).expect("BackupMeta always serializes");
    atomic_write(fs, &sidecar_path(zip_path), &bytes).map_err(|e| match e {
        crate::atomic_write::AtomicWriteError::Io(e) => e,
        crate::atomic_write::AtomicWriteError::MissingParentDirectory(p) => {
            io::Error::new(io::ErrorKind::NotFound, p.display().to_string())
        }
    })
}

fn is_zip(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
}

fn build_entry(fs: &dyn FileSystem, zip_path: &Path) -> Option<BackupEntry> {
    let filename = zip_path.file_name()?.to_string_lossy().into_owned();
    let base = zip_path.file_stem()?.to_string_lossy().into_owned();
    let stat = fs.stat(zip_path).ok();

    let mut entry = BackupEntry {
        zip_path: zip_path.to_path_buf(),
        display_name: backup::make_display_name(&base),
        file_size: stat.map(|m| m.size),
        modified: stat.map(|m| m.modified),
        server_id: None,
        server_display_name: None,
        slot_id: None,
        slot_name: None,
        trigger_reason: backup::filename_trigger_reason(&filename).to_string(),
        verified: archive::validate_archive_safety(zip_path).is_ok(),
        filename,
    };

    if let Some(meta) = read_sidecar(fs, zip_path) {
        entry.server_id = meta.server_id;
        entry.server_display_name = meta.server_display_name;
        entry.slot_id = meta.slot_id;
        entry.slot_name = meta.slot_name;
        entry.trigger_reason = meta.trigger_reason;
    }

    Some(entry)
}

/// `loadBackupsForSelectedServer`'s data half (source line 24-98): filters
/// to `.zip` entries, sorts newest-modified-first (a missing modification
/// time sorts as though it were `SystemTime::UNIX_EPOCH`, matching
/// source's own `.distantPast` fallback), and folds in each sidecar when
/// one decodes successfully. A missing or non-directory `backups/` (no
/// backup has ever been made for this server) returns an empty list, not
/// an error (`fixtures/backups/list-no-backups-directory-returns-empty.json`).
pub fn list_backups(fs: &dyn FileSystem, server_dir: &Path) -> Vec<BackupEntry> {
    if !backups_directory_exists(fs, server_dir) {
        return Vec::new();
    }
    let Ok(entries) = fs.list(&backups_dir(server_dir)) else {
        return Vec::new();
    };

    let mut items: Vec<BackupEntry> = entries
        .into_iter()
        .filter(|p| is_zip(p))
        .filter_map(|p| build_entry(fs, &p))
        .collect();

    items.sort_by(|a, b| {
        let a_modified = a.modified.unwrap_or(SystemTime::UNIX_EPOCH);
        let b_modified = b.modified.unwrap_or(SystemTime::UNIX_EPOCH);
        b_modified.cmp(&a_modified)
    });
    items
}

/// `deleteBackup(_:)` (source line 704-721): removes the ZIP, then
/// best-effort removes a paired sidecar if one exists — a missing sidecar
/// is not an error, matching source's own `fileExists` guard.
pub fn delete_backup(fs: &dyn FileSystem, zip_path: &Path) -> io::Result<()> {
    fs.remove(zip_path)?;
    let _ = fs.remove(&sidecar_path(zip_path));
    Ok(())
}

/// `pruneAutoBackupsIfNeeded(in:maxCount:)` (source line 528-581), plus
/// the D-006-style retention floor
/// (`fixtures/backup-restore/retention-preserves-sole-remaining-verified-backup-even-at-maxcount-phase6-correction.json`):
/// pruning only runs once the managed-file count reaches `max_count`
/// (source's own `guard managedFiles.count >= maxCount`), then deletes
/// the oldest `count - (max_count - 1)` — but never a `verified` entry
/// that would leave zero verified backups behind. An unverified/failed
/// entry is never protected by this floor (it isn't a known-good
/// recovery point to preserve), so it's always eligible for ordinary
/// oldest-first pruning.
pub fn prune_managed_backups(
    fs: &dyn FileSystem,
    server_dir: &Path,
    max_count: i64,
) -> Vec<PathBuf> {
    let Ok(entries) = fs.list(&backups_dir(server_dir)) else {
        return Vec::new();
    };

    let mut managed: Vec<(PathBuf, SystemTime, bool)> = entries
        .into_iter()
        .filter(|p| is_zip(p))
        .filter_map(|p| {
            let filename = p.file_name()?.to_string_lossy().into_owned();
            if !backup::is_managed_backup_filename(&filename) {
                return None;
            }
            let modified = fs.stat(&p).ok()?.modified;
            let verified = archive::validate_archive_safety(&p).is_ok();
            Some((p, modified, verified))
        })
        .collect();

    let max_count = max_count.max(0) as usize;
    if managed.len() < max_count {
        return Vec::new();
    }

    managed.sort_by_key(|(_, modified, _)| *modified);
    let delete_count = managed.len() - max_count.saturating_sub(1);
    let mut remaining_verified = managed.iter().filter(|(_, _, v)| *v).count();

    let mut deleted = Vec::new();
    for (path, _, verified) in managed.into_iter().take(delete_count) {
        if verified {
            if remaining_verified <= 1 {
                continue;
            }
            remaining_verified -= 1;
        }
        if delete_backup(fs, &path).is_ok() {
            deleted.push(path);
        }
    }
    deleted
}
