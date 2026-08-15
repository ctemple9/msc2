//! Port of `fixtures/world-mutations/`'s 6 activation cases (P6.5),
//! exercising `msc_application::worlds::activate_slot`/
//! `reconcile_interrupted_activation` (P6.13) — the transactional,
//! restart-safe version of `WorldSlotManager.activateSlot`.
//!
//! Real on-disk server directories, same "genuinely disk-shaped"
//! precedent every other archive-touching test file in this phase
//! already set. Test functions are prefixed `world_activation_` so the
//! plan's Verify command (a plain nextest substring filter on test
//! name) selects them.

use msc_application::worlds::{self, ActivationError, ActivationRecovery};
use msc_domain::identity::ServerType;
use msc_domain::world::WorldSlot;
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
            "msc2-world-activation-test-{label}-{}",
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

fn write_slot_archive_folders(server_dir: &Path, slot_id: &str, folders: &[(&str, &[u8])]) {
    let zip_path = server_dir
        .join("world_slots")
        .join(slot_id)
        .join("world.zip");
    fs::create_dir_all(zip_path.parent().unwrap()).unwrap();
    let file = fs::File::create(&zip_path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    for (name, content) in folders {
        zip.start_file(format!("{name}/level.dat"), opts).unwrap();
        zip.write_all(content).unwrap();
    }
    zip.finish().unwrap();
}

fn write_server_properties(server_dir: &Path, level_name: &str) {
    write_file(
        &server_dir.join("server.properties"),
        format!("level-name={level_name}\n").as_bytes(),
    );
}

fn slot_with_archive(id: &str, level_name: &str) -> WorldSlot {
    WorldSlot {
        id: id.to_string(),
        name: "New adventure".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_played_at: None,
        thumbnail_file_name: None,
        world_level_name: Some(level_name.to_string()),
        world_seed: None,
        zip_size_bytes: None,
    }
}

fn fresh_slot(id: &str, level_name: &str, seed: &str) -> WorldSlot {
    WorldSlot {
        id: id.to_string(),
        name: "Brand new map".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_played_at: None,
        thumbnail_file_name: None,
        world_level_name: Some(level_name.to_string()),
        world_seed: Some(seed.to_string()),
        zip_size_bytes: None,
    }
}

#[test]
fn world_activation_refused_while_server_running() {
    let tmp = TempDir::new("running");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_live_folder(server_dir, "world", b"overworld");
    let slot = slot_with_archive("slot-b", "world");
    write_slot_archive_folders(server_dir, &slot.id, &[("world", b"new world")]);

    let err = worlds::activate_slot(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        &slot,
        true,
        "2026-06-01T00:00:00Z",
        || panic!("backup must not run when the server is refused for running"),
    )
    .unwrap_err();

    assert!(matches!(err, ActivationError::ServerRunning));
    assert!(server_dir.join("world").join("level.dat").is_file());
    assert!(!server_dir.join("world_slots").join(".activation").exists());
}

#[test]
fn world_activation_backup_failure_aborts_before_any_world_mutation() {
    let tmp = TempDir::new("backup-failure");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_live_folder(server_dir, "world", b"overworld");
    make_live_folder(server_dir, "world_nether", b"nether");
    let slot = slot_with_archive("slot-b", "world");
    write_slot_archive_folders(server_dir, &slot.id, &[("world", b"new world")]);

    let err = worlds::activate_slot(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        &slot,
        false,
        "2026-06-01T00:00:00Z",
        || false,
    )
    .unwrap_err();

    assert!(matches!(err, ActivationError::BackupFailed));
    assert!(server_dir.join("world").join("level.dat").is_file());
    assert!(server_dir.join("world_nether").join("level.dat").is_file());
    assert!(!server_dir.join("world_slots").join(".activation").exists());
}

#[test]
fn world_activation_mandatory_pre_activation_backup_runs_before_any_move() {
    let tmp = TempDir::new("backup-first");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_live_folder(server_dir, "world", b"overworld");
    let slot = slot_with_archive("slot-b", "world");
    write_slot_archive_folders(server_dir, &slot.id, &[("world", b"new world")]);

    let backup_saw_untouched_live_folder = std::cell::Cell::new(false);
    let updated = worlds::activate_slot(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        &slot,
        false,
        "2026-06-01T00:00:00Z",
        || {
            // At the moment the backup closure runs, the live folder is
            // still exactly as it was — nothing has moved yet.
            backup_saw_untouched_live_folder.set(
                fs::read(server_dir.join("world").join("level.dat"))
                    .ok()
                    .as_deref()
                    == Some(b"overworld".as_slice()),
            );
            true
        },
    )
    .unwrap();

    assert!(backup_saw_untouched_live_folder.get());
    assert_eq!(updated.id, "slot-b");
    assert_eq!(
        fs::read(server_dir.join("world").join("level.dat")).unwrap(),
        b"new world"
    );
}

#[test]
fn world_activation_extraction_failure_leaves_live_world_untouched() {
    let tmp = TempDir::new("extraction-failure");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_live_folder(server_dir, "world", b"overworld");
    let slot = slot_with_archive("slot-b", "world");
    // Corrupt archive: a zip file that isn't actually a zip.
    write_file(
        &server_dir
            .join("world_slots")
            .join(&slot.id)
            .join("world.zip"),
        b"not a real zip file at all",
    );

    let err = worlds::activate_slot(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        &slot,
        false,
        "2026-06-01T00:00:00Z",
        || true,
    )
    .unwrap_err();

    assert!(matches!(err, ActivationError::Archive(_)));
    // This is the P6.13 correction over MSC 1's own baseline
    // (`activate-extraction-failure-leaves-partial-state-for-safety-
    // backup-recovery.json`): the live world is still completely intact,
    // not gone.
    assert_eq!(
        fs::read(server_dir.join("world").join("level.dat")).unwrap(),
        b"overworld"
    );
    assert!(!server_dir.join("world_slots").join(".activation").exists());

    // No active marker was persisted.
    assert!(
        !server_dir
            .join("world_slots")
            .join("active_slot_id.txt")
            .exists()
    );
}

#[test]
fn world_activation_fresh_archive_less_slot_defers_world_generation() {
    let tmp = TempDir::new("fresh-slot");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_live_folder(server_dir, "world", b"overworld");
    make_live_folder(server_dir, "world_nether", b"nether");
    make_live_folder(server_dir, "world_the_end", b"end");
    let slot = fresh_slot("slot-fresh", "brand-new-map", "998877");

    let updated = worlds::activate_slot(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        &slot,
        false,
        "2026-06-01T00:00:00Z",
        || true,
    )
    .unwrap();

    assert_eq!(updated.world_level_name.as_deref(), Some("brand-new-map"));
    // Current folders were removed (moved aside), nothing generated yet.
    assert!(!server_dir.join("world").exists());
    assert!(!server_dir.join("world_nether").exists());
    assert!(!server_dir.join("world_the_end").exists());

    let props = fs::read_to_string(server_dir.join("server.properties")).unwrap();
    assert!(props.contains("level-name=brand-new-map"));
    assert!(props.contains("level-seed=998877"));

    let marker = fs::read_to_string(server_dir.join("world_slots/active_slot_id.txt")).unwrap();
    assert_eq!(marker.trim(), "slot-fresh");
    assert!(!server_dir.join("world_slots").join(".activation").exists());
}

#[test]
fn world_activation_legacy_zip_loose_worlds_root_relocated() {
    let tmp = TempDir::new("legacy-relocation");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "MyWorld");
    let slot = slot_with_archive("slot-old", "MyWorld");

    // Old Bedrock export: worlds/db, worlds/level.dat land loose at
    // worlds/ root, no worlds/MyWorld/ subfolder.
    let zip_path = server_dir
        .join("world_slots")
        .join(&slot.id)
        .join("world.zip");
    fs::create_dir_all(zip_path.parent().unwrap()).unwrap();
    let file = fs::File::create(&zip_path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    zip.start_file("worlds/db/dummy", opts).unwrap();
    zip.write_all(b"db bytes").unwrap();
    zip.start_file("worlds/level.dat", opts).unwrap();
    zip.write_all(b"level dat bytes").unwrap();
    zip.finish().unwrap();

    let updated = worlds::activate_slot(
        &StdFileSystem,
        server_dir,
        ServerType::Bedrock,
        &slot,
        false,
        "2026-06-01T00:00:00Z",
        || true,
    )
    .unwrap();

    assert_eq!(updated.id, "slot-old");
    assert!(
        server_dir
            .join("worlds")
            .join("MyWorld")
            .join("db")
            .join("dummy")
            .is_file()
    );
    assert!(
        server_dir
            .join("worlds")
            .join("MyWorld")
            .join("level.dat")
            .is_file()
    );
}

#[test]
fn world_activation_reconcile_no_transaction_in_flight_is_noop() {
    let tmp = TempDir::new("reconcile-noop");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");

    let outcome = worlds::reconcile_interrupted_activation(
        &StdFileSystem,
        server_dir,
        "2026-06-01T00:00:00Z",
    )
    .unwrap();
    assert!(outcome.is_none());
}

#[test]
fn world_activation_reconcile_prior_moved_restores_old_world() {
    let tmp = TempDir::new("reconcile-prior-moved");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");

    // Simulate a crash mid-transaction, after phase 2 (prior moved
    // aside, new content already staged) but before phase 3 (install).
    write_file(
        &server_dir
            .join("world_slots")
            .join(".activation")
            .join("manifest.json"),
        br#"{"slot_id":"slot-b","identity":{"level_name":"world","seed":null,"apply_seed":false}}"#,
    );
    write_file(
        &server_dir
            .join("world_slots")
            .join(".activation")
            .join("prior")
            .join("world")
            .join("level.dat"),
        b"old overworld",
    );
    write_file(
        &server_dir
            .join("world_slots")
            .join(".activation")
            .join("staged")
            .join("world")
            .join("level.dat"),
        b"new overworld, never installed",
    );
    // The server root currently has no live world at all — the
    // dangerous-looking window this transaction passes through safely.
    assert!(!server_dir.join("world").exists());

    let outcome = worlds::reconcile_interrupted_activation(
        &StdFileSystem,
        server_dir,
        "2026-06-01T00:00:00Z",
    )
    .unwrap()
    .unwrap();
    assert_eq!(outcome, ActivationRecovery::RecoveredToOldWorld);

    assert_eq!(
        fs::read(server_dir.join("world").join("level.dat")).unwrap(),
        b"old overworld"
    );
    assert!(!server_dir.join("world_slots").join(".activation").exists());
    // No active marker was persisted for the interrupted slot — the old
    // world was recovered, not the new one.
    assert!(
        !server_dir
            .join("world_slots")
            .join("active_slot_id.txt")
            .exists()
    );
}

#[test]
fn world_activation_reconcile_installed_finishes_committing_new_world() {
    let tmp = TempDir::new("reconcile-installed");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    let slot = slot_with_archive("slot-b", "world");
    let slot_dir = server_dir.join("world_slots").join(&slot.id);
    write_file(
        &slot_dir.join("slot.json"),
        serde_json::to_vec_pretty(&slot.encode())
            .unwrap()
            .as_slice(),
    );
    write_slot_archive_folders(server_dir, &slot.id, &[("world", b"new overworld")]);

    // Simulate a crash mid-transaction, after phase 3's install step
    // (new content already at the server root, `staged/` already
    // removed) but before the commit tail (identity/metadata/marker).
    make_live_folder(server_dir, "world", b"new overworld, already installed");
    write_file(
        &server_dir
            .join("world_slots")
            .join(".activation")
            .join("manifest.json"),
        br#"{"slot_id":"slot-b","identity":{"level_name":"world","seed":null,"apply_seed":false}}"#,
    );
    write_file(
        &server_dir
            .join("world_slots")
            .join(".activation")
            .join("prior")
            .join("world")
            .join("level.dat"),
        b"old overworld, safely discardable",
    );

    let outcome = worlds::reconcile_interrupted_activation(
        &StdFileSystem,
        server_dir,
        "2026-06-01T00:00:00Z",
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        outcome,
        ActivationRecovery::RecoveredToNewWorld {
            slot_id: "slot-b".to_string()
        }
    );

    assert_eq!(
        fs::read(server_dir.join("world").join("level.dat")).unwrap(),
        b"new overworld, already installed"
    );
    assert!(!server_dir.join("world_slots").join(".activation").exists());
    let marker = fs::read_to_string(server_dir.join("world_slots/active_slot_id.txt")).unwrap();
    assert_eq!(marker.trim(), "slot-b");
    let meta = fs::read_to_string(slot_dir.join("slot.json")).unwrap();
    assert!(meta.contains("2026-06-01T00:00:00Z"));
}
