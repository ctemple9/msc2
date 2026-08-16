//! Port of `fixtures/world-import-reconciliation/`'s 8 fixtures (P6.4),
//! exercising `docs/msc2/worlds/phase6-scope.md`'s reconciliation rule as
//! implemented by `msc_application::worlds::reconcile_imported_worlds`.
//!
//! Each case builds a real server directory on disk (live world folders,
//! `world_slots/` entries, and — where a fixture's resolved slot carries
//! an archive — a real `world.zip`) since the function under test reads
//! real files and extracts/creates real zip archives via
//! `msc_infrastructure::archive`, the same "genuinely disk-shaped" real
//! temp-directory precedent P5.13/P5.14's own tests already set. Test
//! functions are prefixed `world_import_reconciliation_` so the plan's
//! Verify command (a plain nextest substring filter on test name) selects
//! them.

use msc_application::worlds::{ReconciliationOutcome, reconcile_imported_worlds};
use msc_domain::identity::ServerType;
use msc_infrastructure::fs::StdFileSystem;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-world-reconciliation-test-{label}-{}",
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

fn make_live_folder(server_dir: &Path, name: &str, content: &[u8]) {
    write_file(&server_dir.join(name).join("level.dat"), content);
}

fn write_slot_json(server_dir: &Path, slot_id: &str, created_at: &str) {
    let json =
        format!(r#"{{"id":"{slot_id}","name":"Slot {slot_id}","created_at":"{created_at}"}}"#);
    write_file(
        &server_dir
            .join("world_slots")
            .join(slot_id)
            .join("slot.json"),
        json.as_bytes(),
    );
}

fn write_slot_archive(server_dir: &Path, slot_id: &str, folder_name: &str, content: &[u8]) {
    let zip_path = server_dir
        .join("world_slots")
        .join(slot_id)
        .join("world.zip");
    fs::create_dir_all(zip_path.parent().unwrap()).unwrap();
    let file = fs::File::create(&zip_path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    zip.start_file(format!("{folder_name}/level.dat"), opts)
        .unwrap();
    zip.write_all(content).unwrap();
    zip.finish().unwrap();
}

fn write_corrupt_slot(server_dir: &Path, slot_id: &str) {
    write_file(
        &server_dir
            .join("world_slots")
            .join(slot_id)
            .join("slot.json"),
        b"{not valid json",
    );
}

#[test]
fn world_import_reconciliation_raw_live_folders_only() {
    let tmp = TempDir::new("state1");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld data");
    make_live_folder(server_dir, "world_nether", b"nether data");
    make_live_folder(server_dir, "world_the_end", b"end data");

    let outcome = reconcile_imported_worlds(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "2026-06-01T00:00:00Z",
    )
    .unwrap();

    let new_slot_id = match outcome {
        ReconciliationOutcome::LiveFoldersArchivedAsNewActiveSlot { new_slot_id } => new_slot_id,
        other => panic!("expected LiveFoldersArchivedAsNewActiveSlot, got {other:?}"),
    };

    let marker = fs::read_to_string(server_dir.join("world_slots/active_slot_id.txt")).unwrap();
    assert_eq!(marker.trim(), new_slot_id);
    assert!(
        server_dir
            .join("world_slots")
            .join(&new_slot_id)
            .join("world.zip")
            .is_file()
    );
    // Live folders are left untouched on disk.
    assert!(server_dir.join("world/level.dat").is_file());
    assert!(server_dir.join("world_nether/level.dat").is_file());
    assert!(server_dir.join("world_the_end/level.dat").is_file());
}

#[test]
fn world_import_reconciliation_copied_slots_only_archive_less_fresh() {
    let tmp = TempDir::new("state2-archive-less");
    let server_dir = tmp.path();
    write_slot_json(server_dir, "slot-fresh", "2026-01-01T00:00:00Z");

    let outcome = reconcile_imported_worlds(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "2026-06-01T00:00:00Z",
    )
    .unwrap();

    assert_eq!(
        outcome,
        ReconciliationOutcome::ArchiveLessSlotMarkedActive {
            slot_id: "slot-fresh".to_string()
        }
    );
    let marker = fs::read_to_string(server_dir.join("world_slots/active_slot_id.txt")).unwrap();
    assert_eq!(marker.trim(), "slot-fresh");
    assert!(!server_dir.join("world").exists());
}

#[test]
fn world_import_reconciliation_copied_slots_only_with_archive() {
    let tmp = TempDir::new("state2-archived");
    let server_dir = tmp.path();
    write_slot_json(server_dir, "slot-a", "2026-01-01T00:00:00Z");
    write_slot_archive(server_dir, "slot-a", "world", b"archived overworld");

    let outcome = reconcile_imported_worlds(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "2026-06-01T00:00:00Z",
    )
    .unwrap();

    assert_eq!(
        outcome,
        ReconciliationOutcome::ArchiveExtractedFromResolvedSlot {
            slot_id: "slot-a".to_string()
        }
    );
    let marker = fs::read_to_string(server_dir.join("world_slots/active_slot_id.txt")).unwrap();
    assert_eq!(marker.trim(), "slot-a");
    assert_eq!(
        fs::read(server_dir.join("world/level.dat")).unwrap(),
        b"archived overworld"
    );
}

#[test]
fn world_import_reconciliation_live_plus_matching_slot_proven_identical() {
    let tmp = TempDir::new("state3-identical");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"same content");
    write_slot_json(server_dir, "slot-recorded", "2026-01-01T00:00:00Z");
    write_slot_archive(server_dir, "slot-recorded", "world", b"same content");

    let outcome = reconcile_imported_worlds(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "2026-06-01T00:00:00Z",
    )
    .unwrap();

    assert_eq!(
        outcome,
        ReconciliationOutcome::LiveFoldersProvenIdenticalToRecordedSlot {
            slot_id: "slot-recorded".to_string()
        }
    );
    let marker = fs::read_to_string(server_dir.join("world_slots/active_slot_id.txt")).unwrap();
    assert_eq!(marker.trim(), "slot-recorded");
    // No new slot directory was created — exactly one slot dir exists.
    let slot_dirs: Vec<_> = fs::read_dir(server_dir.join("world_slots"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    assert_eq!(slot_dirs.len(), 1);
    // The recorded slot's own archive is untouched.
    assert!(fs::read(server_dir.join("world_slots/slot-recorded/world.zip")).is_ok());
}

#[test]
fn world_import_reconciliation_live_plus_stale_different_active_slot() {
    let tmp = TempDir::new("state3-different");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"freshly imported live content");
    write_slot_json(server_dir, "slot-recorded", "2026-01-01T00:00:00Z");
    write_slot_archive(
        server_dir,
        "slot-recorded",
        "world",
        b"old recorded content",
    );
    let recorded_archive_before =
        fs::read(server_dir.join("world_slots/slot-recorded/world.zip")).unwrap();

    let outcome = reconcile_imported_worlds(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "2026-06-01T00:00:00Z",
    )
    .unwrap();

    let new_slot_id = match outcome {
        ReconciliationOutcome::RecoverySnapshotCreated {
            new_slot_id,
            previous_slot_id,
        } => {
            assert_eq!(previous_slot_id, "slot-recorded");
            new_slot_id
        }
        other => panic!("expected RecoverySnapshotCreated, got {other:?}"),
    };

    let marker = fs::read_to_string(server_dir.join("world_slots/active_slot_id.txt")).unwrap();
    assert_eq!(marker.trim(), new_slot_id);
    assert_ne!(new_slot_id, "slot-recorded");
    // The previously-recorded slot survives untouched — same bytes, still present.
    let recorded_archive_after =
        fs::read(server_dir.join("world_slots/slot-recorded/world.zip")).unwrap();
    assert_eq!(recorded_archive_before, recorded_archive_after);
    assert!(server_dir.join("world_slots").join(&new_slot_id).is_dir());
}

#[test]
fn world_import_reconciliation_missing_or_corrupt_active_marker_with_live_and_slots() {
    let tmp = TempDir::new("state3-unresolvable");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"live content");
    write_corrupt_slot(server_dir, "slot-corrupt-1");
    write_corrupt_slot(server_dir, "slot-corrupt-2");

    let outcome = reconcile_imported_worlds(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "2026-06-01T00:00:00Z",
    )
    .unwrap();

    let new_slot_id = match outcome {
        ReconciliationOutcome::LiveFoldersArchivedAsNewActiveSlot { new_slot_id } => new_slot_id,
        other => panic!("expected LiveFoldersArchivedAsNewActiveSlot, got {other:?}"),
    };
    // The two unresolvable slot directories are left on disk, not deleted.
    assert!(server_dir.join("world_slots/slot-corrupt-1").is_dir());
    assert!(server_dir.join("world_slots/slot-corrupt-2").is_dir());
    assert!(server_dir.join("world_slots").join(&new_slot_id).is_dir());
}

#[test]
fn world_import_reconciliation_corrupt_slot_metadata_tolerated_but_one_valid() {
    let tmp = TempDir::new("state2-tolerant");
    let server_dir = tmp.path();
    write_corrupt_slot(server_dir, "slot-corrupt-1");
    write_corrupt_slot(server_dir, "slot-corrupt-2");
    write_slot_json(server_dir, "slot-valid", "2026-04-01T00:00:00Z");
    write_slot_archive(server_dir, "slot-valid", "world", b"valid archived content");

    let outcome = reconcile_imported_worlds(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "2026-06-01T00:00:00Z",
    )
    .unwrap();

    assert_eq!(
        outcome,
        ReconciliationOutcome::ArchiveExtractedFromResolvedSlot {
            slot_id: "slot-valid".to_string()
        }
    );
    assert!(server_dir.join("world_slots/slot-corrupt-1").is_dir());
    assert!(server_dir.join("world_slots/slot-corrupt-2").is_dir());
    assert_eq!(
        fs::read(server_dir.join("world/level.dat")).unwrap(),
        b"valid archived content"
    );
}

#[test]
fn world_import_reconciliation_no_world_data_neither_source() {
    let tmp = TempDir::new("no-data");
    let server_dir = tmp.path();
    fs::create_dir_all(server_dir).unwrap();

    let outcome = reconcile_imported_worlds(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "2026-06-01T00:00:00Z",
    )
    .unwrap();

    assert_eq!(outcome, ReconciliationOutcome::NoWorldData);
    assert!(!server_dir.join("world_slots/active_slot_id.txt").exists());
}

#[test]
fn world_import_reconciliation_corrupt_archive_extraction_leaves_no_partial_live_folder() {
    // P6.29: the archive-extraction branch stages into a scratch
    // directory before ever touching the live-folder location. A corrupt
    // archive must fail cleanly with no partial `world/` folder, no
    // active-slot marker, and no `.p6_reconciled` marker — so a later
    // startup still sees this server as needing reconciliation rather
    // than silently treating a half-extracted archive as done.
    let tmp = TempDir::new("corrupt-archive");
    let server_dir = tmp.path();
    write_slot_json(server_dir, "slot-corrupt-archive", "2026-01-01T00:00:00Z");
    write_file(
        &server_dir
            .join("world_slots")
            .join("slot-corrupt-archive")
            .join("world.zip"),
        b"this is not a valid zip archive",
    );

    let result = reconcile_imported_worlds(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "2026-06-01T00:00:00Z",
    );

    assert!(result.is_err(), "corrupt archive must not succeed");
    assert!(
        !server_dir.join("world").exists(),
        "no partial live folder may be created from a failed extraction"
    );
    assert!(
        !server_dir.join("world_slots/active_slot_id.txt").exists(),
        "the active marker must not be set when extraction failed"
    );
    assert!(
        !server_dir.join("world_slots/.p6_reconciled").exists(),
        "the reconciliation marker must not be written on failure"
    );
    assert!(
        !server_dir.join("world_slots/.p6_reconcile_staged").exists(),
        "the extraction scratch directory must be cleaned up on failure"
    );
}

#[test]
fn world_import_reconciliation_second_call_is_a_no_op() {
    let tmp = TempDir::new("idempotent");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld data");

    let first = reconcile_imported_worlds(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "2026-06-01T00:00:00Z",
    )
    .unwrap();
    let new_slot_id = match first {
        ReconciliationOutcome::LiveFoldersArchivedAsNewActiveSlot { new_slot_id } => new_slot_id,
        other => panic!("expected LiveFoldersArchivedAsNewActiveSlot, got {other:?}"),
    };

    let second = reconcile_imported_worlds(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "2026-06-02T00:00:00Z",
    )
    .unwrap();
    assert_eq!(second, ReconciliationOutcome::AlreadyReconciled);

    // No second slot was created, and the active marker is unchanged.
    let slot_dirs: Vec<_> = fs::read_dir(server_dir.join("world_slots"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    assert_eq!(slot_dirs.len(), 1);
    let marker = fs::read_to_string(server_dir.join("world_slots/active_slot_id.txt")).unwrap();
    assert_eq!(marker.trim(), new_slot_id);
}
