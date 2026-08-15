//! Port of `fixtures/backups/`'s listing/sidecar/display-name/pruning
//! cases plus the `fixtures/backup-restore/` verification/retention
//! corrections (P6.6), exercising `msc_application::backups` and its
//! `msc_domain::backup`/`msc_infrastructure::backup_store` layers (P6.15).
//!
//! Real on-disk server directories and real ZIP files, same "genuinely
//! disk-shaped" precedent `world_slot_crud.rs` already set — necessary
//! here too since backup verification goes through
//! `msc_infrastructure::archive` against real files. Test functions are
//! prefixed `backup_inventory_` so the plan's Verify command (a plain
//! nextest substring filter on test name) selects them.

use msc_application::backups;
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::backup as domain_backup;
use msc_infrastructure::fs::StdFileSystem;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-backup-inventory-test-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_file(path: &Path, contents: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

/// A minimal, structurally valid zip — enough for
/// `archive::validate_archive_safety` to accept it.
fn write_real_zip(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    zip.start_file("world/level.dat", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"stub").unwrap();
    zip.finish().unwrap();
}

fn touch(path: &Path, when: SystemTime) {
    let file = OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(when).unwrap();
}

// ---------------------------------------------------------------------
// Listing
// ---------------------------------------------------------------------

/// `fixtures/backups/list-no-backups-directory-returns-empty.json`.
#[test]
fn backup_inventory_list_no_backups_directory_returns_empty() {
    let temp = TempDir::new("no-dir");
    let fs_impl = StdFileSystem;
    let entries = backups::list_backups(&fs_impl, temp.path());
    assert!(entries.is_empty());
}

/// `fixtures/backups/list-filters-zip-extension-and-sorts-newest-first.json`.
#[test]
fn backup_inventory_list_filters_zip_and_sorts_newest_first() {
    let temp = TempDir::new("list-sort");
    let backups_dir = temp.path().join("backups");
    let older = backups_dir.join("world_auto_20260101-000000.zip");
    let newer = backups_dir.join("world_manual_20260103-000000.zip");
    write_real_zip(&older);
    write_real_zip(&newer);
    write_file(
        &backups_dir.join("world_manual_20260103-000000.meta.json"),
        b"not a real sidecar object",
    );
    write_file(&backups_dir.join("notes.txt"), b"not a backup");

    let base = SystemTime::UNIX_EPOCH;
    touch(&older, base + Duration::from_secs(1));
    touch(&newer, base + Duration::from_secs(3));

    let fs_impl = StdFileSystem;
    let entries = backups::list_backups(&fs_impl, temp.path());
    let names: Vec<&str> = entries.iter().map(|e| e.filename.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "world_manual_20260103-000000.zip",
            "world_auto_20260101-000000.zip",
        ]
    );
}

/// `fixtures/backups/sidecar-present-overrides-filename-derived-trigger-reason-and-adds-slot-association.json`.
#[test]
fn backup_inventory_sidecar_present_overrides_filename_derived_defaults() {
    let temp = TempDir::new("sidecar-present");
    let zip_path = temp
        .path()
        .join("backups")
        .join("world_manual_20260101-000000.zip");
    write_real_zip(&zip_path);
    let meta = domain_backup::BackupMeta {
        server_id: Some("server_1".to_string()),
        server_display_name: Some("Main Server".to_string()),
        slot_id: Some("slot-a".to_string()),
        slot_name: Some("Base Camp".to_string()),
        world_seed: Some("12345".to_string()),
        trigger_reason: "pre-restore".to_string(),
    };
    write_file(
        &zip_path.with_extension("meta.json"),
        serde_json::to_vec_pretty(&meta.encode())
            .unwrap()
            .as_slice(),
    );

    let fs_impl = StdFileSystem;
    let entries = backups::list_backups(&fs_impl, temp.path());
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.trigger_reason, "pre-restore");
    assert_eq!(entry.slot_id.as_deref(), Some("slot-a"));
    assert_eq!(entry.slot_name.as_deref(), Some("Base Camp"));
    assert!(!domain_backup::is_automatic_trigger(&entry.trigger_reason));
}

/// `fixtures/backups/sidecar-missing-or-corrupt-leaves-filename-derived-defaults.json`
/// — both variants (no sidecar file at all, and an unparseable one).
#[test]
fn backup_inventory_sidecar_missing_or_corrupt_leaves_filename_derived_defaults() {
    let temp = TempDir::new("sidecar-absent-corrupt");
    let backups_dir = temp.path().join("backups");
    let missing = backups_dir.join("world_auto_20260101-000000.zip");
    let corrupt = backups_dir.join("world_auto_20260102-000000.zip");
    write_real_zip(&missing);
    write_real_zip(&corrupt);
    write_file(&corrupt.with_extension("meta.json"), b"{not valid json");

    let fs_impl = StdFileSystem;
    let entries = backups::list_backups(&fs_impl, temp.path());
    assert_eq!(entries.len(), 2);
    for entry in &entries {
        assert_eq!(entry.slot_id, None);
        assert_eq!(entry.slot_name, None);
        assert_eq!(entry.trigger_reason, "auto");
    }
}

/// `fixtures/backup-restore/backup-verification-required-before-listing-or-restore-eligibility-phase6-correction.json`
/// — a structurally invalid archive is still listed, just flagged
/// unverified, never hidden.
#[test]
fn backup_inventory_unverified_archive_still_listed_but_flagged() {
    let temp = TempDir::new("verify-flag");
    let backups_dir = temp.path().join("backups");
    let valid = backups_dir.join("world_manual_20260101-000000.zip");
    let invalid = backups_dir.join("world_manual_20260102-000000.zip");
    write_real_zip(&valid);
    write_file(&invalid, b"this is not a zip file");

    let fs_impl = StdFileSystem;
    let entries = backups::list_backups(&fs_impl, temp.path());
    assert_eq!(entries.len(), 2);
    let valid_entry = entries
        .iter()
        .find(|e| e.filename.contains("20260101"))
        .unwrap();
    let invalid_entry = entries
        .iter()
        .find(|e| e.filename.contains("20260102"))
        .unwrap();
    assert!(valid_entry.verified);
    assert!(!invalid_entry.verified);
}

// ---------------------------------------------------------------------
// Filename/trigger-reason/display-name (msc_domain::backup, pure)
// ---------------------------------------------------------------------

/// `fixtures/backups/auto-backup-uses-auto-token-and-auto-trigger-reason.json`.
#[test]
fn backup_inventory_auto_backup_uses_auto_token_and_reason() {
    assert_eq!(domain_backup::creation_token(true), "_auto_");
    assert_eq!(domain_backup::default_trigger_reason(true), "auto");
}

/// `fixtures/backups/manual-backup-uses-manual-token-and-manual-trigger-reason.json`.
#[test]
fn backup_inventory_manual_backup_uses_manual_token_and_reason() {
    assert_eq!(domain_backup::creation_token(false), "_manual_");
    assert_eq!(domain_backup::default_trigger_reason(false), "manual");
}

/// `fixtures/backups/pre-replace-backup-has-no-token-and-is-excluded-from-pruning.json`.
#[test]
fn backup_inventory_pre_replace_backup_excluded_from_pruning_filter() {
    let filename = "world-20260101-000000.zip";
    assert!(!domain_backup::is_managed_backup_filename(filename));
}

/// `fixtures/backups/display-name-auto-manual-token-parses-timestamp.json`.
#[test]
fn backup_inventory_display_name_token_parses_timestamp() {
    assert_eq!(
        domain_backup::make_display_name("world_auto_20260214-153045"),
        "2026-02-14 15:30:45"
    );
    assert_eq!(
        domain_backup::make_display_name("world_manual_20260214-153045"),
        "2026-02-14 15:30:45"
    );
}

/// `fixtures/backups/display-name-legacy-dash-timestamp-format.json`.
#[test]
fn backup_inventory_display_name_legacy_dash_format() {
    assert_eq!(
        domain_backup::make_display_name("myworld-20250601-120000"),
        "myworld — 2025-06-01 12:00:00"
    );
}

/// `fixtures/backups/display-name-unparseable-suffix-falls-back-to-raw-filename.json`.
#[test]
fn backup_inventory_display_name_unparseable_falls_back_to_raw() {
    assert_eq!(
        domain_backup::make_display_name("world_manual_notatimestamp"),
        "world_manual_notatimestamp"
    );
}

// ---------------------------------------------------------------------
// Deletion
// ---------------------------------------------------------------------

/// `AppViewModel+Backups.swift::deleteBackup` (source line 704-721):
/// removes the zip and its paired sidecar together.
#[test]
fn backup_inventory_delete_backup_removes_zip_and_sidecar() {
    let temp = TempDir::new("delete");
    let zip_path = temp
        .path()
        .join("backups")
        .join("world_manual_20260101-000000.zip");
    write_real_zip(&zip_path);
    write_file(&zip_path.with_extension("meta.json"), b"{}");

    let fs_impl = StdFileSystem;
    backups::delete_backup(&fs_impl, &zip_path).unwrap();
    assert!(!zip_path.exists());
    assert!(!zip_path.with_extension("meta.json").exists());
}

// ---------------------------------------------------------------------
// Pruning
// ---------------------------------------------------------------------

/// `fixtures/backups/pruning-deletes-oldest-managed-files-down-to-maxcount-minus-one-and-removes-orphaned-sidecar.json`.
#[test]
fn backup_inventory_pruning_deletes_oldest_down_to_maxcount_minus_one() {
    let temp = TempDir::new("prune");
    let backups_dir = temp.path().join("backups");
    let base = SystemTime::UNIX_EPOCH;
    let names = [
        "world_auto_1.zip",
        "world_auto_2.zip",
        "world_auto_3.zip",
        "world_auto_4.zip",
        "world_auto_5.zip",
    ];
    for (i, name) in names.iter().enumerate() {
        let path = backups_dir.join(name);
        write_real_zip(&path);
        touch(&path, base + Duration::from_secs(i as u64 + 1));
        if *name != "world_auto_3.zip" {
            write_file(&path.with_extension("meta.json"), b"{}");
        }
    }

    let fs_impl = StdFileSystem;
    let deleted = backups::prune_backups(&fs_impl, temp.path(), 4);
    let deleted_names: Vec<String> = deleted
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(deleted_names, vec!["world_auto_1.zip", "world_auto_2.zip"]);
    for kept in ["world_auto_3.zip", "world_auto_4.zip", "world_auto_5.zip"] {
        assert!(backups_dir.join(kept).exists());
    }
    assert!(!backups_dir.join("world_auto_1.meta.json").exists());
}

/// `pruneAutoBackupsIfNeeded`'s own guard (source line 550): below the
/// count threshold, nothing is deleted.
#[test]
fn backup_inventory_pruning_below_threshold_deletes_nothing() {
    let temp = TempDir::new("prune-below-threshold");
    let backups_dir = temp.path().join("backups");
    for name in ["world_auto_1.zip", "world_auto_2.zip", "world_auto_3.zip"] {
        write_real_zip(&backups_dir.join(name));
    }

    let fs_impl = StdFileSystem;
    let deleted = backups::prune_backups(&fs_impl, temp.path(), 4);
    assert!(deleted.is_empty());
}

/// `fixtures/backup-restore/retention-preserves-sole-remaining-verified-backup-even-at-maxcount-phase6-correction.json`.
#[test]
fn backup_inventory_pruning_never_deletes_sole_remaining_verified_backup() {
    let temp = TempDir::new("prune-floor");
    let backups_dir = temp.path().join("backups");
    let base = SystemTime::UNIX_EPOCH;

    // Two managed files: the oldest is a corrupt (unverified) archive,
    // the newest is the only verified one. Pruning to max_count=1 would
    // naively delete both down to zero remaining, but the floor must
    // preserve the sole verified backup even though it's newer than the
    // unverified one and wouldn't ordinarily be "the oldest".
    let unverified = backups_dir.join("world_auto_1.zip");
    let verified = backups_dir.join("world_auto_2.zip");
    write_file(&unverified, b"not a real zip");
    write_real_zip(&verified);
    touch(&unverified, base + Duration::from_secs(1));
    touch(&verified, base + Duration::from_secs(2));

    let fs_impl = StdFileSystem;
    let deleted = backups::prune_backups(&fs_impl, temp.path(), 1);
    let deleted_names: Vec<String> = deleted
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(deleted_names, vec!["world_auto_1.zip"]);
    assert!(backups_dir.join("world_auto_2.zip").exists());
}

// ---------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------

/// `fixtures/backups/auto-backup-max-count-editor-clamps-to-3-through-50.json`.
#[test]
fn backup_inventory_auto_backup_max_count_clamps_to_3_through_50() {
    assert_eq!(backups::clamp_auto_backup_max_count(1), 3);
    assert_eq!(backups::clamp_auto_backup_max_count(999), 50);
    assert_eq!(backups::clamp_auto_backup_max_count(12), 12);
}

/// `fixtures/backups/auto-backup-interval-minutes-defaults-to-30-when-config-field-absent.json`
/// — already-existing `ConfigServer::decode` behavior (P4), confirmed
/// here rather than re-implemented.
#[test]
fn backup_inventory_auto_backup_interval_minutes_defaults_to_30() {
    let json = serde_json::json!({
        "id": "server_1",
        "display_name": "Main",
        "server_dir": "/tmp/server",
        "paper_jar_path": "/tmp/server/paper.jar",
        "min_ram_gb": 1.0,
        "max_ram_gb": 2.0,
    });
    let server = ConfigServer::decode(&json).unwrap();
    assert_eq!(server.auto_backup_interval_minutes, 30);
}
