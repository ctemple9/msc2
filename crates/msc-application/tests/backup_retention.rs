//! Port of `fixtures/backups/scheduled-auto-backup-skipped-when-no-players-online.json`
//! plus P6.17's own retention characterization (orphan-sidecar sweeping,
//! the never-delete-the-last-verified-backup floor holding across
//! repeated scheduled ticks), exercising `msc_application::backups::
//! scheduled_tick`/`prune_orphan_sidecars`.
//!
//! Real on-disk server directories and real ZIP files, same "genuinely
//! disk-shaped" precedent this phase's other test files already set.
//! Test functions are prefixed `backup_retention_` so the plan's Verify
//! command (a plain nextest substring filter on test name) selects them.

use msc_application::backups::{self, BackupError, ScheduledTickOutcome};
use msc_domain::identity::ServerType;
use msc_domain::world;
use msc_infrastructure::fs::StdFileSystem;
use std::fs;
use std::path::{Path, PathBuf};

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-backup-retention-test-{label}-{}",
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

fn make_live_folder(server_dir: &Path, name: &str, content: &[u8]) {
    let dir = server_dir.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("level.dat"), content).unwrap();
}

fn tick(
    server_dir: &Path,
    now: &str,
    backend_running: bool,
    online_player_count: usize,
) -> ScheduledTickOutcome {
    let association = world::BackupAssociation::default();
    backups::scheduled_tick(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        4,
        now,
        backend_running,
        online_player_count,
    )
}

/// `startAutoBackupTimer`'s own guard (source line 777): the backend
/// isn't running.
#[test]
fn backup_retention_skipped_when_backend_not_running() {
    let tmp = TempDir::new("not-running");
    make_live_folder(tmp.path(), "world", b"overworld");

    let outcome = tick(tmp.path(), "2026-01-01T00:00:00Z", false, 3);
    assert!(matches!(outcome, ScheduledTickOutcome::SkippedNotRunning));
    assert!(!tmp.path().join("backups").exists());
}

/// `fixtures/backups/scheduled-auto-backup-skipped-when-no-players-online.json`.
#[test]
fn backup_retention_skipped_when_no_players_online() {
    let tmp = TempDir::new("no-players");
    make_live_folder(tmp.path(), "world", b"overworld");

    let outcome = tick(tmp.path(), "2026-01-01T00:00:00Z", true, 0);
    assert!(matches!(outcome, ScheduledTickOutcome::SkippedNoPlayers));
    assert!(!tmp.path().join("backups").exists());
}

/// Re-evaluated every tick — a quiet server just keeps skipping, no
/// state carried between calls.
#[test]
fn backup_retention_no_players_skip_is_re_evaluated_every_tick() {
    let tmp = TempDir::new("no-players-repeat");
    make_live_folder(tmp.path(), "world", b"overworld");

    for _ in 0..3 {
        let outcome = tick(tmp.path(), "2026-01-01T00:00:00Z", true, 0);
        assert!(matches!(outcome, ScheduledTickOutcome::SkippedNoPlayers));
    }
    assert!(!tmp.path().join("backups").exists());
}

/// A running server with players online fires a real, automatic,
/// prunable backup.
#[test]
fn backup_retention_fires_automatic_backup_when_running_with_players() {
    let tmp = TempDir::new("fires");
    make_live_folder(tmp.path(), "world", b"overworld");

    let outcome = tick(tmp.path(), "2026-02-14T15:30:45Z", true, 2);
    let ScheduledTickOutcome::Fired(Ok(result)) = outcome else {
        panic!("expected a fired, successful backup: {outcome:?}");
    };
    assert!(
        result
            .zip_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("_auto_")
    );
    assert_eq!(result.trigger_reason, "auto");
}

/// The retention floor (P6.15's own `prune_managed_backups` correction)
/// holds across repeated scheduled ticks: max_count clamps the managed
/// set, but the sole verified backup is never the one pruned away.
#[test]
fn backup_retention_floor_holds_across_repeated_scheduled_ticks() {
    let tmp = TempDir::new("floor-repeated");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");

    let timestamps = [
        "2026-01-01T00:00:00Z",
        "2026-01-01T01:00:00Z",
        "2026-01-01T02:00:00Z",
        "2026-01-01T03:00:00Z",
        "2026-01-01T04:00:00Z",
        "2026-01-01T05:00:00Z",
    ];
    for now in timestamps {
        let outcome = tick(server_dir, now, true, 1);
        assert!(matches!(outcome, ScheduledTickOutcome::Fired(Ok(_))));
    }

    let backups_dir = server_dir.join("backups");
    let managed: Vec<_> = fs::read_dir(&backups_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("zip"))
        .collect();
    // max_count is 4 in `tick`'s own fixed `auto_prune_max_count`; 6
    // ticks against that ceiling always leaves at least one managed
    // backup, and never zero.
    assert!(!managed.is_empty());
    assert!(managed.len() <= 4);
}

/// No world folders yet (a fresh, never-started server) — the tick
/// still fires (backend running, players online) but `create_backup`
/// itself refuses.
#[test]
fn backup_retention_fired_tick_can_still_fail_with_no_world_folders() {
    let tmp = TempDir::new("no-world-folders");
    let outcome = tick(tmp.path(), "2026-01-01T00:00:00Z", true, 1);
    assert!(matches!(
        outcome,
        ScheduledTickOutcome::Fired(Err(BackupError::NoWorldFolders))
    ));
}

// ---------------------------------------------------------------------
// Orphan sidecar sweeping
// ---------------------------------------------------------------------

#[test]
fn backup_retention_prune_orphan_sidecars_removes_only_unpaired_meta_files() {
    let tmp = TempDir::new("orphan-sidecars");
    let backups_dir = tmp.path().join("backups");
    fs::create_dir_all(&backups_dir).unwrap();

    // A real, paired backup: zip + sidecar both present.
    fs::write(backups_dir.join("world_auto_1.zip"), b"stub").unwrap();
    fs::write(backups_dir.join("world_auto_1.meta.json"), b"{}").unwrap();
    // An orphaned sidecar: the zip is gone (hand-deleted, or a crash
    // between the two removals), but the sidecar survived.
    fs::write(backups_dir.join("world_auto_2.meta.json"), b"{}").unwrap();
    // A non-sidecar JSON file that happens to live in the backups
    // directory — must never be touched.
    fs::write(backups_dir.join("notes.json"), b"{}").unwrap();

    let removed = backups::prune_orphan_sidecars(&StdFileSystem, tmp.path());
    let removed_names: Vec<String> = removed
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();

    assert_eq!(removed_names, vec!["world_auto_2.meta.json"]);
    assert!(backups_dir.join("world_auto_1.zip").exists());
    assert!(backups_dir.join("world_auto_1.meta.json").exists());
    assert!(backups_dir.join("notes.json").exists());
    assert!(!backups_dir.join("world_auto_2.meta.json").exists());
}

#[test]
fn backup_retention_prune_orphan_sidecars_missing_directory_is_a_no_op() {
    let tmp = TempDir::new("orphan-sidecars-missing-dir");
    let removed = backups::prune_orphan_sidecars(&StdFileSystem, tmp.path());
    assert!(removed.is_empty());
}
