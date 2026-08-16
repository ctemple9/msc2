//! Port of `fixtures/backups/`'s creation-token/trigger-reason/
//! association cases plus the offline half of P6.16's own characterization,
//! exercising `msc_application::backups::create_backup`'s offline path
//! (`console: None` throughout — the running-server save-pause protocol
//! is `backup_online_consistency.rs`'s job).
//!
//! Real on-disk server directories and real ZIP files, same "genuinely
//! disk-shaped" precedent `world_slot_crud.rs` already set. Test
//! functions are prefixed `backup_creation_` so the plan's Verify command
//! (`-E 'test(/backup_(creation|online_consistency)/)'`) selects them.

use msc_application::backups::{self, BackupError};
use msc_domain::identity::ServerType;
use msc_domain::world::{self, WorldSlot};
use std::fs;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

use msc_infrastructure::fs::StdFileSystem;

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-backup-creation-test-{label}-{}",
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

fn zip_entry_names(zip_path: &Path) -> Vec<String> {
    let file = fs::File::open(zip_path).unwrap();
    let archive = ZipArchive::new(file).unwrap();
    archive.file_names().map(str::to_string).collect()
}

fn slot(id: &str, name: &str, seed: Option<&str>) -> WorldSlot {
    WorldSlot {
        id: id.to_string(),
        name: name.to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_played_at: None,
        thumbnail_file_name: None,
        world_level_name: Some("world".to_string()),
        world_seed: seed.map(str::to_string),
        zip_size_bytes: None,
    }
}

/// `fixtures/backups/manual-backup-uses-manual-token-and-manual-trigger-reason.json`.
#[test]
fn backup_creation_manual_backup_uses_manual_token_and_reason() {
    let tmp = TempDir::new("manual");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");
    let association = world::BackupAssociation::default();

    let result = backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        false,
        true,
        None,
        None,
        "2026-02-14T15:30:45Z",
        None,
        || false,
        || false,
    )
    .unwrap();

    assert!(
        result
            .zip_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("_manual_")
    );
    assert_eq!(result.trigger_reason, "manual");
    assert!(result.sidecar_written);
}

/// `fixtures/backups/auto-backup-uses-auto-token-and-auto-trigger-reason.json`.
#[test]
fn backup_creation_auto_backup_uses_auto_token_and_reason() {
    let tmp = TempDir::new("auto");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");
    let association = world::BackupAssociation::default();

    let result = backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        true,
        true,
        None,
        None,
        "2026-02-14T15:30:45Z",
        None,
        || false,
        || false,
    )
    .unwrap();

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

/// `fixtures/backups/pre-replace-backup-has-no-token-and-is-excluded-from-pruning.json`
/// — `backupWorld(for:)`'s untokened shape via `tokened: false`.
#[test]
fn backup_creation_pre_replace_backup_has_no_token() {
    let tmp = TempDir::new("pre-replace");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");
    let association = world::BackupAssociation::default();

    let result = backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        false,
        false,
        Some("pre-replace"),
        None,
        "2026-02-14T15:30:45Z",
        None,
        || false,
        || false,
    )
    .unwrap();

    let filename = result
        .zip_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(!filename.contains("_auto_"));
    assert!(!filename.contains("_manual_"));
    assert_eq!(result.trigger_reason, "pre-replace");
    assert!(!msc_domain::backup::is_managed_backup_filename(&filename));
}

/// `fixtures/backups/backup-association-explicit-slot-id-overrides-active-slot.json`.
#[test]
fn backup_creation_association_explicit_slot_id_overrides_active_slot() {
    let tmp = TempDir::new("assoc-explicit");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");

    let slots = vec![
        slot("slot-a", "Overworld", None),
        slot("slot-b", "Nether Base", Some("9988")),
    ];
    let association = world::effective_backup_association(
        &slots,
        Some("slot-a"),
        Some("slot-b"),
        Some(" Nether Base "),
    );

    let result = backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        false,
        true,
        None,
        None,
        "2026-02-14T15:30:45Z",
        None,
        || false,
        || false,
    )
    .unwrap();

    let sidecar =
        msc_infrastructure::backup_store::read_sidecar(&StdFileSystem, &result.zip_path).unwrap();
    assert_eq!(sidecar.slot_id.as_deref(), Some("slot-b"));
    assert_eq!(sidecar.slot_name.as_deref(), Some("Nether Base"));
    assert_eq!(sidecar.world_seed.as_deref(), Some("9988"));
}

/// `fixtures/backups/backup-association-falls-back-to-active-slot-when-no-explicit-id.json`.
#[test]
fn backup_creation_association_falls_back_to_active_slot() {
    let tmp = TempDir::new("assoc-fallback");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");

    let slots = vec![slot("slot-a", "Overworld", None)];
    let association = world::effective_backup_association(&slots, Some("slot-a"), None, None);

    let result = backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        false,
        true,
        None,
        None,
        "2026-02-14T15:30:45Z",
        None,
        || false,
        || false,
    )
    .unwrap();

    let sidecar =
        msc_infrastructure::backup_store::read_sidecar(&StdFileSystem, &result.zip_path).unwrap();
    assert_eq!(sidecar.slot_id.as_deref(), Some("slot-a"));
    assert_eq!(sidecar.slot_name.as_deref(), Some("Overworld"));
    assert_eq!(sidecar.world_seed, None);
}

/// `worldFolderNames(for:)`'s empty guard, shared with `worlds::
/// create_slot_from_current_world` (`fixtures/world-mutations`'s own
/// `NoWorldFolders` case) — no fixture in `fixtures/backups` names this
/// directly, but source's own guard (`createBackup` line 207-213) is
/// identical.
#[test]
fn backup_creation_no_world_folders_returns_error() {
    let tmp = TempDir::new("no-folders");
    let association = world::BackupAssociation::default();

    let result = backups::create_backup(
        &StdFileSystem,
        tmp.path(),
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        false,
        true,
        None,
        None,
        "2026-02-14T15:30:45Z",
        None,
        || false,
        || false,
    );

    assert!(matches!(result, Err(BackupError::NoWorldFolders)));
}

/// Real Java dimension-folder capture: `world`, `world_nether`,
/// `world_the_end` are all zipped, matching `worldFolderNames(for:)`'s
/// Java candidate set.
#[test]
fn backup_creation_captures_every_java_dimension_folder() {
    let tmp = TempDir::new("dimensions");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");
    make_live_folder(server_dir, "world_nether", b"nether");
    make_live_folder(server_dir, "world_the_end", b"end");
    let association = world::BackupAssociation::default();

    let result = backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        false,
        true,
        None,
        None,
        "2026-02-14T15:30:45Z",
        None,
        || false,
        || false,
    )
    .unwrap();

    let names = zip_entry_names(&result.zip_path);
    for folder in ["world", "world_nether", "world_the_end"] {
        assert!(
            names.iter().any(|n| n.starts_with(&format!("{folder}/"))),
            "missing {folder} in {names:?}"
        );
    }
}

/// `createBackup`'s `isAutomatic` prune-before-create ordering (source
/// line 223-226): pruning uses the *old* file count, before the new
/// backup is written.
#[test]
fn backup_creation_auto_prune_runs_before_creating_new_backup() {
    let tmp = TempDir::new("prune-before-create");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");
    let backups_dir = server_dir.join("backups");
    fs::create_dir_all(&backups_dir).unwrap();
    for i in 1..=5 {
        let path = backups_dir.join(format!("world_auto_2026010{i}-000000.zip"));
        fs::write(&path, b"stub").unwrap();
        let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(i))
            .unwrap();
    }
    let association = world::BackupAssociation::default();

    // 5 pre-existing managed files + max_count 4 => prune deletes 2
    // before the new backup is written, leaving 3 old + 1 new = 4.
    backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        true,
        true,
        None,
        Some(4),
        "2026-02-14T15:30:45Z",
        None,
        || false,
        || false,
    )
    .unwrap();

    let remaining: Vec<_> = fs::read_dir(&backups_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("zip"))
        .collect();
    assert_eq!(remaining.len(), 4);
}

/// `fixtures/backups/same-second-backups-get-collision-proof-filenames-phase6-correction.json`
/// — two manual backups triggered with the identical `now` never collide
/// on disk, and both archives/sidecars survive with their own captured
/// content intact.
#[test]
fn backup_creation_same_second_backups_do_not_collide() {
    let tmp = TempDir::new("same-second");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"first");
    let association = world::BackupAssociation::default();

    let first = backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        false,
        true,
        None,
        None,
        "2026-02-14T15:30:45Z",
        None,
        || false,
        || false,
    )
    .unwrap();

    // Overwrite the live folder's content between calls so each archive
    // is independently verifiable -- a real overwrite-in-place bug would
    // make the second backup's content indistinguishable from the
    // first's even if the filename check were skipped.
    fs::write(server_dir.join("world").join("level.dat"), b"second").unwrap();

    let second = backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        false,
        true,
        None,
        None,
        "2026-02-14T15:30:45Z",
        None,
        || false,
        || false,
    )
    .unwrap();

    assert_ne!(first.zip_path, second.zip_path);
    assert!(first.zip_path.exists());
    assert!(second.zip_path.exists());

    let second_name = second.zip_path.file_name().unwrap().to_string_lossy();
    assert!(second_name.contains("_manual_"));
    assert!(second_name.ends_with("-2.zip"));

    // The first archive's content was never touched by the second call.
    let mut first_zip = ZipArchive::new(fs::File::open(&first.zip_path).unwrap()).unwrap();
    let mut contents = String::new();
    std::io::Read::read_to_string(
        &mut first_zip.by_name("world/level.dat").unwrap(),
        &mut contents,
    )
    .unwrap();
    assert_eq!(contents, "first");

    // Both sidecars are present, independently.
    let sidecar_first =
        msc_infrastructure::backup_store::read_sidecar(&StdFileSystem, &first.zip_path);
    let sidecar_second =
        msc_infrastructure::backup_store::read_sidecar(&StdFileSystem, &second.zip_path);
    assert!(sidecar_first.is_some());
    assert!(sidecar_second.is_some());
}

/// The untokened pre-replace shape collides the same way, and gets the
/// same `-2` disambiguation.
#[test]
fn backup_creation_same_second_untokened_backups_do_not_collide() {
    let tmp = TempDir::new("same-second-untokened");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");
    let association = world::BackupAssociation::default();

    let make = || {
        backups::create_backup(
            &StdFileSystem,
            server_dir,
            ServerType::Java,
            Some("world"),
            &association,
            None,
            None,
            false,
            false,
            Some("pre-replace"),
            None,
            "2026-02-14T15:30:45Z",
            None,
            || false,
            || false,
        )
        .unwrap()
    };

    let first = make();
    let second = make();

    assert_ne!(first.zip_path, second.zip_path);
    assert!(first.zip_path.exists());
    assert!(second.zip_path.exists());
    let second_name = second.zip_path.file_name().unwrap().to_string_lossy();
    assert!(!second_name.contains("_auto_"));
    assert!(!second_name.contains("_manual_"));
    assert!(second_name.ends_with("-2.zip"));
}

#[cfg(unix)]
#[test]
fn backup_creation_zip_write_failure_returns_archive_error() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new("zip-failure");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");
    let locked = server_dir.join("world").join("locked");
    fs::create_dir_all(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let association = world::BackupAssociation::default();

    let result = backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &association,
        None,
        None,
        false,
        true,
        None,
        None,
        "2026-02-14T15:30:45Z",
        None,
        || false,
        || false,
    );

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(matches!(result, Err(BackupError::Archive(_))));
}
