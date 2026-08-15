//! Port of `fixtures/backup-restore/`'s 10 restore-guard/transaction
//! cases (P6.6), exercising `msc_application::backups::restore_backup`/
//! `reconcile_interrupted_restore` (P6.18).
//!
//! Real on-disk server directories and real ZIP files, same "genuinely
//! disk-shaped" precedent `world_activation.rs`/`world_slot_crud.rs`
//! already set. Test functions are prefixed `backup_restore_` so the
//! plan's Verify command (a plain nextest substring filter on test name)
//! selects them.

use msc_application::backups::{self, RestoreError, RestoreRecovery};
use msc_domain::identity::ServerType;
use msc_domain::world;
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
            "msc2-backup-restore-test-{label}-{}",
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

fn write_backup_zip(path: &Path, folder: &str, content: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    zip.start_file(format!("{folder}/level.dat"), SimpleFileOptions::default())
        .unwrap();
    zip.write_all(content).unwrap();
    zip.finish().unwrap();
}

fn default_association() -> world::BackupAssociation {
    world::BackupAssociation::default()
}

#[allow(clippy::too_many_arguments)]
fn restore(
    server_dir: &Path,
    server_type: ServerType,
    backup_zip_path: &Path,
    backup_slot_id: Option<&str>,
    resolved_active_slot_id: Option<&str>,
    is_server_running: bool,
) -> Result<backups::RestoreOutcome, RestoreError> {
    backups::restore_backup(
        &StdFileSystem,
        server_dir,
        server_type,
        Some("world"),
        backup_zip_path,
        backup_slot_id,
        resolved_active_slot_id,
        is_server_running,
        &default_association(),
        None,
        None,
        "2026-02-14T15:30:45Z",
    )
}

/// `fixtures/backup-restore/restore-refused-for-bedrock-server-java-only-currently-supported.json`.
#[test]
fn backup_restore_refused_for_bedrock_server() {
    let tmp = TempDir::new("bedrock");
    let backup_zip = tmp.path().join("backups").join("backup.zip");
    write_backup_zip(&backup_zip, "worlds", b"content");

    let result = restore(
        tmp.path(),
        ServerType::Bedrock,
        &backup_zip,
        None,
        None,
        false,
    );
    assert!(matches!(result, Err(RestoreError::BedrockNotSupported)));
}

/// `fixtures/backup-restore/restore-refused-while-target-server-running.json`.
#[test]
fn backup_restore_refused_while_target_server_running() {
    let tmp = TempDir::new("running");
    let backup_zip = tmp.path().join("backups").join("backup.zip");
    write_backup_zip(&backup_zip, "world", b"content");

    let result = restore(tmp.path(), ServerType::Java, &backup_zip, None, None, true);
    assert!(matches!(result, Err(RestoreError::ServerRunning)));
}

/// `fixtures/backup-restore/restore-refused-when-backup-belongs-to-different-slot-than-active.json`.
#[test]
fn backup_restore_refused_when_backup_belongs_to_different_slot() {
    let tmp = TempDir::new("cross-slot");
    let backup_zip = tmp.path().join("backups").join("backup.zip");
    write_backup_zip(&backup_zip, "world", b"content");

    let result = restore(
        tmp.path(),
        ServerType::Java,
        &backup_zip,
        Some("slot-b"),
        Some("slot-a"),
        false,
    );
    match result {
        Err(RestoreError::CrossSlot {
            backup_slot_id,
            active_slot_id,
        }) => {
            assert_eq!(backup_slot_id, "slot-b");
            assert_eq!(active_slot_id, "slot-a");
        }
        other => panic!("expected CrossSlot, got {other:?}"),
    }
}

/// A legacy backup with no slot association always passes the cross-slot
/// guard regardless of which slot is active.
#[test]
fn backup_restore_no_cross_slot_guard_when_backup_has_no_slot_association() {
    let tmp = TempDir::new("no-slot-association");
    make_live_folder(tmp.path(), "world", b"overworld");
    let backup_zip = tmp.path().join("backups").join("backup.zip");
    write_backup_zip(&backup_zip, "world", b"restored-content");

    let result = restore(
        tmp.path(),
        ServerType::Java,
        &backup_zip,
        None,
        Some("slot-a"),
        false,
    );
    assert!(result.is_ok());
}

/// `fixtures/backup-restore/restore-refused-when-source-file-missing-on-disk.json`.
#[test]
fn backup_restore_refused_when_source_file_missing() {
    let tmp = TempDir::new("missing-source");
    let backup_zip = tmp.path().join("backups").join("does-not-exist.zip");

    let result = restore(tmp.path(), ServerType::Java, &backup_zip, None, None, false);
    assert!(matches!(result, Err(RestoreError::SourceMissing)));
}

/// `fixtures/backup-restore/restore-creates-mandatory-safety-backup-before-any-restore-mutation.json`.
#[test]
fn backup_restore_creates_mandatory_safety_backup_with_pre_restore_token() {
    let tmp = TempDir::new("safety-backup");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"pre-restore-content");
    let backup_zip = server_dir.join("backups").join("restore-source.zip");
    write_backup_zip(&backup_zip, "world", b"restored-content");

    let result = restore(server_dir, ServerType::Java, &backup_zip, None, None, false).unwrap();

    let safety = &result.safety_backup_zip_path;
    assert!(safety.is_file());
    let filename = safety.file_name().unwrap().to_string_lossy().into_owned();
    // The safety backup goes through the ordinary `createBackup` path:
    // `_manual_` token, prunable — distinct from Replace World's own
    // untokened pre-replace safety backup.
    assert!(filename.contains("_manual_"));
    let sidecar = msc_infrastructure::backup_store::read_sidecar(&StdFileSystem, safety).unwrap();
    assert_eq!(sidecar.trigger_reason, "pre-restore");
}

/// `fixtures/backup-restore/restore-aborted-when-safety-backup-creation-fails-no-files-changed.json`.
#[cfg(unix)]
#[test]
fn backup_restore_aborted_when_safety_backup_creation_fails() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new("safety-backup-fails");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"pre-restore-content");
    let backup_zip = server_dir.join("backups").join("restore-source.zip");
    write_backup_zip(&backup_zip, "world", b"restored-content");

    let locked = server_dir.join("world").join("locked");
    fs::create_dir_all(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    let result = restore(server_dir, ServerType::Java, &backup_zip, None, None, false);

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(matches!(result, Err(RestoreError::SafetyBackupFailed(_))));
    // Nothing about the current world changed.
    assert_eq!(
        fs::read(server_dir.join("world").join("level.dat")).unwrap(),
        b"pre-restore-content"
    );
    assert!(!server_dir.join("world_slots").join(".restore").exists());
}

/// `fixtures/backup-restore/restore-aborted-when-archive-validation-fails-world-untouched.json`
/// and `restore-validates-archive-before-removing-existing-world-folders.json`: an invalid
/// source archive is rejected strictly before any folder is touched —
/// both the just-created safety backup and the still-intact live world
/// exist afterward.
#[test]
fn backup_restore_aborted_when_archive_validation_fails_world_untouched() {
    let tmp = TempDir::new("invalid-archive");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"pre-restore-content");
    let backup_zip = server_dir.join("backups").join("restore-source.zip");
    fs::create_dir_all(backup_zip.parent().unwrap()).unwrap();
    fs::write(&backup_zip, b"this is not a zip file").unwrap();

    let result = restore(server_dir, ServerType::Java, &backup_zip, None, None, false);
    assert!(matches!(result, Err(RestoreError::ArchiveInvalid)));

    // World untouched.
    assert_eq!(
        fs::read(server_dir.join("world").join("level.dat")).unwrap(),
        b"pre-restore-content"
    );
    // The mandatory safety backup was still created and retained before
    // the (failed) archive check.
    let backups_dir = server_dir.join("backups");
    let managed: Vec<_> = fs::read_dir(&backups_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().and_then(|x| x.to_str()) == Some("zip")
                && e.file_name().to_string_lossy().contains("_manual_")
        })
        .collect();
    assert_eq!(managed.len(), 1);
    assert!(!server_dir.join("world_slots").join(".restore").exists());
}

/// `fixtures/backup-restore/restore-success-extracts-into-server-directory-and-refreshes-list.json`.
#[test]
fn backup_restore_success_extracts_into_server_directory() {
    let tmp = TempDir::new("success");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"pre-restore-content");
    let backup_zip = server_dir.join("backups").join("restore-source.zip");
    write_backup_zip(&backup_zip, "world", b"restored-content");

    let result = restore(server_dir, ServerType::Java, &backup_zip, None, None, false).unwrap();

    assert_eq!(
        fs::read(server_dir.join("world").join("level.dat")).unwrap(),
        b"restored-content"
    );
    assert!(result.safety_backup_zip_path.is_file());
    assert!(!server_dir.join("world_slots").join(".restore").exists());
}

/// `fixtures/backup-restore/restore-msc1-has-no-automatic-rollback-after-interrupted-extraction-phase6-correction.json`
/// — this port's own correction: an interrupted extraction (staging
/// phase) leaves the live world completely untouched, not worldless.
#[cfg(unix)]
#[test]
fn backup_restore_interrupted_extraction_leaves_world_untouched() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new("interrupted-extraction");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"pre-restore-content");
    let backup_zip = server_dir.join("backups").join("restore-source.zip");
    write_backup_zip(&backup_zip, "world", b"restored-content");

    // Pre-create the staging directory read-only so `extract_zip`'s own
    // write phase fails partway through, after `validate_archive_safety`
    // already passed.
    let staged_dir = server_dir
        .join("world_slots")
        .join(".restore")
        .join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    fs::set_permissions(&staged_dir, fs::Permissions::from_mode(0o555)).unwrap();

    let result = restore(server_dir, ServerType::Java, &backup_zip, None, None, false);

    // The transaction directory's own cleanup may already have removed
    // `staged_dir` (its restrictive mode only ever blocked writes
    // *inside* it, not its parent unlinking it) — restoring permissions
    // is best-effort, not a correctness requirement of this test.
    let _ = fs::set_permissions(&staged_dir, fs::Permissions::from_mode(0o755));

    assert!(matches!(
        result,
        Err(RestoreError::Archive(_)) | Err(RestoreError::Io(_))
    ));
    // The live world is completely intact -- never even moved aside,
    // since staging (phase 1) failed before phase 2 starts.
    assert_eq!(
        fs::read(server_dir.join("world").join("level.dat")).unwrap(),
        b"pre-restore-content"
    );
    assert!(!server_dir.join("world_slots").join(".restore").exists());
}

// ---------------------------------------------------------------------
// Restart recovery
// ---------------------------------------------------------------------

fn restore_transaction_dir(server_dir: &Path) -> PathBuf {
    server_dir.join("world_slots").join(".restore")
}

/// No in-flight transaction: a no-op.
#[test]
fn backup_restore_reconcile_no_transaction_is_a_no_op() {
    let tmp = TempDir::new("reconcile-none");
    let outcome = backups::reconcile_interrupted_restore(&StdFileSystem, tmp.path()).unwrap();
    assert_eq!(outcome, None);
}

/// Phase 1 ("staged") interrupted: `prior/` was never created — discard
/// the abandoned staging area, the live world (never touched) is already
/// the complete old world.
#[test]
fn backup_restore_reconcile_staged_only_recovers_to_old_world() {
    let tmp = TempDir::new("reconcile-staged");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"live-content");
    let staged = restore_transaction_dir(server_dir)
        .join("staged")
        .join("world");
    fs::create_dir_all(&staged).unwrap();
    fs::write(staged.join("level.dat"), b"staged-content").unwrap();

    let outcome = backups::reconcile_interrupted_restore(&StdFileSystem, server_dir)
        .unwrap()
        .unwrap();
    assert_eq!(outcome, RestoreRecovery::RecoveredToOldWorld);
    assert!(!restore_transaction_dir(server_dir).exists());
    assert_eq!(
        fs::read(server_dir.join("world").join("level.dat")).unwrap(),
        b"live-content"
    );
}

/// Phase 2 ("prior_moved") interrupted: both `staged/` and `prior/`
/// exist, the server root has no live world at all — move `prior/` back.
#[test]
fn backup_restore_reconcile_prior_moved_recovers_to_old_world() {
    let tmp = TempDir::new("reconcile-prior-moved");
    let server_dir = tmp.path();
    let dir = restore_transaction_dir(server_dir);
    let staged = dir.join("staged").join("world");
    fs::create_dir_all(&staged).unwrap();
    fs::write(staged.join("level.dat"), b"staged-content").unwrap();
    let prior = dir.join("prior").join("world");
    fs::create_dir_all(&prior).unwrap();
    fs::write(prior.join("level.dat"), b"prior-content").unwrap();

    let outcome = backups::reconcile_interrupted_restore(&StdFileSystem, server_dir)
        .unwrap()
        .unwrap();
    assert_eq!(outcome, RestoreRecovery::RecoveredToOldWorld);
    assert!(!restore_transaction_dir(server_dir).exists());
    assert_eq!(
        fs::read(server_dir.join("world").join("level.dat")).unwrap(),
        b"prior-content"
    );
}

/// Phase 3 ("installed") interrupted: `prior/` exists but `staged/`
/// doesn't — the restored world is already at the server root; just
/// discard the transaction directory.
#[test]
fn backup_restore_reconcile_installed_recovers_to_restored_world() {
    let tmp = TempDir::new("reconcile-installed");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"restored-content");
    let prior = restore_transaction_dir(server_dir)
        .join("prior")
        .join("world");
    fs::create_dir_all(&prior).unwrap();
    fs::write(prior.join("level.dat"), b"prior-content").unwrap();

    let outcome = backups::reconcile_interrupted_restore(&StdFileSystem, server_dir)
        .unwrap()
        .unwrap();
    assert_eq!(outcome, RestoreRecovery::RecoveredToRestoredWorld);
    assert!(!restore_transaction_dir(server_dir).exists());
    assert_eq!(
        fs::read(server_dir.join("world").join("level.dat")).unwrap(),
        b"restored-content"
    );
}
