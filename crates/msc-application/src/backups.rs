//! Backup application layer: thin pass-throughs over
//! `msc_infrastructure::backup_store` so every backup listing/deletion/
//! pruning route or CLI caller reaches its filesystem operation through
//! one module — the same precedent `msc_application::worlds::
//! set_slot_thumbnail` already set for a slot mutation with no
//! additional orchestration guard of its own — plus the one caller-facing
//! policy this step's own scope adds: the auto-backup max-count clamp.
//!
//! Backup *creation* (`createBackup`, the flush-consistent save-pause
//! protocol) and restore are P6.16/P6.18; this module covers only
//! inventory, deletion, pruning, and the max-count clamp.
//!
//! Two of this step's planned fixtures need no new production code at
//! all, since earlier phases already built what they characterize:
//! `auto-backup-interval-minutes-defaults-to-30-when-config-field-absent`
//! is `app_config_schema.rs`'s existing `opt_i64(v,
//! "auto_backup_interval_minutes", 30)` decode default (already tested in
//! `crates/msc-domain/tests/app_config_schema.rs`); `effective_backup_association`
//! (the active-slot-association rule) is `msc_domain::world`'s, ported
//! ahead of this step for P6.12's own use. `backup_inventory.rs`'s tests
//! cite both rather than re-proving them.

use msc_infrastructure::backup_store::{self, BackupEntry};
use msc_infrastructure::fs::FileSystem;
use std::io;
use std::path::{Path, PathBuf};

/// `loadBackupsForSelectedServer`'s data half (source
/// `AppViewModel+Backups.swift:24-98`) — the UI-only size-formatting and
/// background-thread directory-size computation stay with whichever
/// client renders them.
pub fn list_backups(fs: &dyn FileSystem, server_dir: &Path) -> Vec<BackupEntry> {
    backup_store::list_backups(fs, server_dir)
}

/// `deleteBackup(_:)` (source line 704-721): unconditional — MSC 1 has no
/// "don't delete the last backup" guard on a manual, single-backup
/// delete; that floor applies only to automatic pruning
/// ([`prune_backups`]'s own correction), matching source exactly.
pub fn delete_backup(fs: &dyn FileSystem, zip_path: &Path) -> io::Result<()> {
    backup_store::delete_backup(fs, zip_path)
}

/// `pruneAutoBackupsIfNeeded(in:maxCount:)` (source line 528-581), with
/// the retention floor already applied inside
/// `backup_store::prune_managed_backups` — see that function's own doc.
pub fn prune_backups(fs: &dyn FileSystem, server_dir: &Path, max_count: i64) -> Vec<PathBuf> {
    backup_store::prune_managed_backups(fs, server_dir, max_count)
}

/// `Stepper("", value: $autoBackupMaxCountLocal, in: 3...50)`
/// (`ServerEditorBackupsTab.swift:47`) — MSC 1 enforces this bound only
/// in the SwiftUI control itself; the model layer
/// (`setAutoBackupMaxCount`, `AppViewModel+ServerControls.swift:820-825`)
/// applies no clamp at all
/// (`fixtures/backups/auto-backup-max-count-editor-clamps-to-3-through-50.json`'s
/// own notes: "the model layer performs no validation or clamping").
/// MSC 2 has no editor control of its own yet (P6.20+ builds the routes;
/// a client builds the control), so this gives the 3...50 bound an
/// application-layer home a future settings route/CLI command can call
/// before persisting, rather than leaving it unenforced anywhere in the
/// port — a deliberate strengthening over source, not oracle parity.
pub fn clamp_auto_backup_max_count(requested: i64) -> i64 {
    requested.clamp(3, 50)
}
