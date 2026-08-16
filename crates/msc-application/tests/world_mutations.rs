//! Port of `fixtures/world-mutations/`'s 4 direct world rename/replace
//! cases (P6.5), exercising `msc_application::worlds::rename_world`/
//! `replace_world` (P6.14), plus `replace_world`'s own P6.33
//! transactional-staging test (below) — `fixtures/world-mutations/
//! replace-world-folder-removal-failure-aborts-before-extraction.json`
//! stays as the MSC 1 baseline record it always was; the remove-then-
//! copy behavior it pins no longer exists in this port, so the case it
//! characterizes is replaced here by a staging-failure equivalent
//! against the corrected transaction.
//!
//! Real on-disk server directories, same precedent every other
//! archive-touching test file in this phase already set. Test functions
//! are prefixed `world_mutations_` so the plan's Verify command (a
//! plain nextest substring filter on test name) selects them.

use msc_application::backups;
use msc_application::worlds::{self, WorldError, WorldReplaceRecovery, WorldReplaceSource};
use msc_domain::identity::ServerType;
use msc_domain::world::BackupAssociation;
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
            "msc2-world-mutations-test-{label}-{}",
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

fn make_folder(server_dir: &Path, name: &str, content: &[u8]) {
    write_file(&server_dir.join(name).join("level.dat"), content);
}

fn write_server_properties(server_dir: &Path, level_name: &str) {
    write_file(
        &server_dir.join("server.properties"),
        format!("level-name={level_name}\n").as_bytes(),
    );
}

#[test]
fn world_mutations_rename_world_target_folder_exists_refused_before_any_move() {
    let tmp = TempDir::new("rename-target-exists");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_folder(server_dir, "world", b"overworld");
    make_folder(server_dir, "world_nether", b"nether");
    make_folder(server_dir, "world_the_end", b"end");
    // A stray folder already occupies one of the target names.
    make_folder(server_dir, "newname_nether", b"stray leftover");

    let err = worlds::rename_world(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "newname",
        false,
        false,
        || true,
    )
    .unwrap_err();

    assert!(matches!(err, WorldError::TargetFolderExists(name) if name == "newname_nether"));
    // Nothing moved.
    assert!(server_dir.join("world").join("level.dat").is_file());
    assert!(server_dir.join("world_nether").join("level.dat").is_file());
    assert!(server_dir.join("world_the_end").join("level.dat").is_file());
    assert!(!server_dir.join("newname").exists());
    let props = fs::read_to_string(server_dir.join("server.properties")).unwrap();
    assert!(props.contains("level-name=world"));
}

#[test]
fn world_mutations_rename_world_rollback_on_mid_sequence_move_failure() {
    let tmp = TempDir::new("rename-rollback");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_folder(server_dir, "world", b"overworld");
    make_folder(server_dir, "world_nether", b"nether");
    make_folder(server_dir, "world_the_end", b"end");

    // Force the third move (world_the_end -> newname_the_end) to fail:
    // pre-occupy the destination with a *file* (not a directory), so
    // `rename` onto it fails cross-platform without permission tricks.
    write_file(&server_dir.join("newname_the_end"), b"blocking file");

    let err = worlds::rename_world(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "newname",
        false,
        false,
        || true,
    )
    .unwrap_err();

    assert!(matches!(err, WorldError::Io(_)));
    // Rolled back: original folders restored under their original names.
    assert_eq!(
        fs::read(server_dir.join("world").join("level.dat")).unwrap(),
        b"overworld"
    );
    assert_eq!(
        fs::read(server_dir.join("world_nether").join("level.dat")).unwrap(),
        b"nether"
    );
    assert!(!server_dir.join("newname").exists());
    assert!(!server_dir.join("newname_nether").exists());
    let props = fs::read_to_string(server_dir.join("server.properties")).unwrap();
    assert!(props.contains("level-name=world"));
}

#[test]
fn world_mutations_rename_world_refuses_while_server_running() {
    let tmp = TempDir::new("rename-running");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_folder(server_dir, "world", b"overworld");

    let err = worlds::rename_world(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "newname",
        true,
        false,
        || panic!("backup must not run when refused for running"),
    )
    .unwrap_err();

    assert!(matches!(err, WorldError::ServerRunning));
    assert!(server_dir.join("world").join("level.dat").is_file());
}

#[test]
fn world_mutations_replace_world_staging_failure_leaves_live_world_untouched() {
    // P6.33's own headline improvement over the MSC 1 baseline
    // `fixtures/world-mutations/
    // replace-world-folder-removal-failure-aborts-before-extraction.json`
    // pins: source removes the live folders *before* installing the
    // replacement, so a failure there (or in extraction/copy right
    // after) can leave the server with no world at all. The corrected,
    // transactional `replace_world` stages the replacement first — the
    // live folders are never touched until staging has fully
    // succeeded — so a staging failure (here: an unreadable file inside
    // an `ExistingFolder` source, mid-copy) must leave every live
    // folder and `server.properties` completely untouched.
    let tmp = TempDir::new("replace-staging-failure");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_folder(server_dir, "world", b"overworld");
    make_folder(server_dir, "world_nether", b"nether");
    make_folder(server_dir, "world_the_end", b"end");

    let source_dir = tmp.path().join("outside").join("source_world");
    write_file(&source_dir.join("level.dat"), b"replacement overworld");
    let blocked_file = source_dir.join("blocked.dat");
    write_file(&blocked_file, b"unreadable");

    // Unix-only: forcing a mid-copy read failure without a real locked
    // file needs a permission trick unavailable on non-Unix, the same
    // constraint every other permission-based failure injection in this
    // phase already documents.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&blocked_file, fs::Permissions::from_mode(0o000)).unwrap();

        let result = worlds::replace_world(
            &StdFileSystem,
            server_dir,
            ServerType::Java,
            Some("world"),
            "newname",
            &WorldReplaceSource::ExistingFolder(source_dir.clone()),
            false,
            &BackupAssociation::default(),
            None,
            None,
            "2026-01-01T00:00:00Z",
            || false,
        );

        fs::set_permissions(&blocked_file, fs::Permissions::from_mode(0o644)).unwrap();

        let err = result.unwrap_err();
        assert!(matches!(err, WorldError::Io(_)));
        // Old world still there — staging aborted before phase 2 ever
        // moved a live folder — and server.properties was never touched.
        assert_eq!(
            fs::read(server_dir.join("world").join("level.dat")).unwrap(),
            b"overworld"
        );
        assert!(server_dir.join("world_nether").join("level.dat").is_file());
        assert!(server_dir.join("world_the_end").join("level.dat").is_file());
        let props = fs::read_to_string(server_dir.join("server.properties")).unwrap();
        assert!(props.contains("level-name=world"));
    }
}

#[test]
fn world_mutations_replace_world_refuses_while_server_running() {
    let tmp = TempDir::new("replace-running");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_folder(server_dir, "world", b"overworld");

    let err = worlds::replace_world(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "newname",
        &WorldReplaceSource::Fresh,
        true,
        &BackupAssociation::default(),
        None,
        None,
        "2026-01-01T00:00:00Z",
        || panic!("should_cancel/backup must not run when refused for running"),
    )
    .unwrap_err();

    assert!(matches!(err, WorldError::ServerRunning));
    assert!(server_dir.join("world").join("level.dat").is_file());
}

#[test]
fn world_mutations_replace_world_fresh_source_generates_on_next_start() {
    let tmp = TempDir::new("replace-fresh");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_folder(server_dir, "world", b"overworld");
    make_folder(server_dir, "world_nether", b"nether");

    worlds::replace_world(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "newname",
        &WorldReplaceSource::Fresh,
        false,
        &BackupAssociation::default(),
        None,
        None,
        "2026-01-01T00:00:00Z",
        || false,
    )
    .unwrap();

    assert!(!server_dir.join("world").exists());
    assert!(!server_dir.join("world_nether").exists());
    assert!(!server_dir.join("newname").exists());
    let props = fs::read_to_string(server_dir.join("server.properties")).unwrap();
    assert!(props.contains("level-name=newname"));
}

#[test]
fn world_mutations_replace_world_backup_zip_source_extracted_into_place() {
    let tmp = TempDir::new("replace-backup-zip");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_folder(server_dir, "world", b"overworld");

    let backup_zip = tmp.path().join("outside").join("backup.zip");
    fs::create_dir_all(backup_zip.parent().unwrap()).unwrap();
    let file = fs::File::create(&backup_zip).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    zip.start_file("newname/level.dat", opts).unwrap();
    zip.write_all(b"replacement overworld").unwrap();
    zip.finish().unwrap();

    worlds::replace_world(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "newname",
        &WorldReplaceSource::BackupZip(backup_zip),
        false,
        &BackupAssociation::default(),
        None,
        None,
        "2026-01-01T00:00:00Z",
        || false,
    )
    .unwrap();

    assert_eq!(
        fs::read(server_dir.join("newname").join("level.dat")).unwrap(),
        b"replacement overworld"
    );
    let props = fs::read_to_string(server_dir.join("server.properties")).unwrap();
    assert!(props.contains("level-name=newname"));
}

#[test]
fn world_mutations_replace_world_invalid_backup_zip_rejected() {
    let tmp = TempDir::new("replace-invalid-zip");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_folder(server_dir, "world", b"overworld");

    let bad_zip = tmp.path().join("outside").join("bad.zip");
    write_file(&bad_zip, b"not a zip file");

    let err = worlds::replace_world(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "newname",
        &WorldReplaceSource::BackupZip(bad_zip),
        false,
        &BackupAssociation::default(),
        None,
        None,
        "2026-01-01T00:00:00Z",
        || panic!("should_cancel/backup must not run before source validation"),
    )
    .unwrap_err();

    assert!(matches!(err, WorldError::InvalidWorldSource));
    assert!(server_dir.join("world").join("level.dat").is_file());
}

#[test]
fn world_mutations_replace_world_mandatory_safety_backup_created_before_live_world_touched() {
    let tmp = TempDir::new("replace-mandatory-backup");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_folder(server_dir, "world", b"overworld");

    let outcome = worlds::replace_world(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "newname",
        &WorldReplaceSource::Fresh,
        false,
        &BackupAssociation::default(),
        None,
        None,
        "2026-01-01T00:00:00Z",
        || false,
    )
    .unwrap();

    let backup_zip_path = outcome
        .safety_backup_zip_path
        .expect("a live world existed, so a safety backup must have been created");
    assert!(backup_zip_path.is_file());

    let entries = backups::list_backups(&StdFileSystem, server_dir);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].trigger_reason, "pre-replace");
    // Untokened, matching source's own separate, untokened pre-replace
    // backup function (`backupWorld`) — pruning never manages it.
    assert!(!entries[0].filename.contains("_manual_"));
    assert!(!entries[0].filename.contains("_auto_"));
}

#[test]
fn world_mutations_replace_world_skips_safety_backup_when_no_live_world_exists() {
    let tmp = TempDir::new("replace-no-backup-needed");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    // No live world folders at all yet — nothing to protect, matching
    // `activate_slot`'s own `!current_folders.is_empty()` backup gate.

    let outcome = worlds::replace_world(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "newname",
        &WorldReplaceSource::Fresh,
        false,
        &BackupAssociation::default(),
        None,
        None,
        "2026-01-01T00:00:00Z",
        || false,
    )
    .unwrap();

    assert!(outcome.safety_backup_zip_path.is_none());
    assert!(backups::list_backups(&StdFileSystem, server_dir).is_empty());
}

#[test]
fn world_mutations_replace_world_reconcile_no_transaction_in_flight_is_noop() {
    let tmp = TempDir::new("replace-reconcile-noop");
    let server_dir = tmp.path();
    make_folder(server_dir, "world", b"overworld");

    let outcome =
        worlds::reconcile_interrupted_world_replace(&StdFileSystem, server_dir, ServerType::Java)
            .unwrap();
    assert!(outcome.is_none());
}

#[test]
fn world_mutations_replace_world_reconcile_prior_moved_restores_old_world() {
    let tmp = TempDir::new("replace-reconcile-prior-moved");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");

    // Simulate a crash mid-transaction, after phase 2 (prior moved
    // aside, the replacement already staged) but before phase 3
    // (install).
    write_file(
        &server_dir
            .join("world_slots")
            .join(".replace")
            .join("manifest.json"),
        br#"{"level_name":"newname"}"#,
    );
    write_file(
        &server_dir
            .join("world_slots")
            .join(".replace")
            .join("prior")
            .join("world")
            .join("level.dat"),
        b"old overworld",
    );
    write_file(
        &server_dir
            .join("world_slots")
            .join(".replace")
            .join("staged")
            .join("newname")
            .join("level.dat"),
        b"new overworld, never installed",
    );
    // The live world currently has nothing at it — the dangerous-looking
    // window this transaction passes through safely.
    assert!(!server_dir.join("world").exists());

    let outcome =
        worlds::reconcile_interrupted_world_replace(&StdFileSystem, server_dir, ServerType::Java)
            .unwrap()
            .unwrap();
    assert_eq!(outcome, WorldReplaceRecovery::RecoveredToOldWorld);

    assert_eq!(
        fs::read(server_dir.join("world").join("level.dat")).unwrap(),
        b"old overworld"
    );
    assert!(!server_dir.join("world_slots").join(".replace").exists());
    // level-name was never committed — the old world was recovered, not
    // the new one.
    let props = fs::read_to_string(server_dir.join("server.properties")).unwrap();
    assert!(props.contains("level-name=world"));
}

#[test]
fn world_mutations_replace_world_reconcile_installed_finishes_committing_new_world() {
    let tmp = TempDir::new("replace-reconcile-installed");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");

    // Simulate a crash mid-transaction, after phase 3's install step
    // (new content already at the live location, `staged/` already
    // removed) but before the commit tail (the level-name write).
    make_folder(server_dir, "newname", b"new overworld, already installed");
    write_file(
        &server_dir
            .join("world_slots")
            .join(".replace")
            .join("manifest.json"),
        br#"{"level_name":"newname"}"#,
    );
    write_file(
        &server_dir
            .join("world_slots")
            .join(".replace")
            .join("prior")
            .join("world")
            .join("level.dat"),
        b"old overworld, safely discardable",
    );

    let outcome =
        worlds::reconcile_interrupted_world_replace(&StdFileSystem, server_dir, ServerType::Java)
            .unwrap()
            .unwrap();
    assert_eq!(outcome, WorldReplaceRecovery::RecoveredToNewWorld);

    assert_eq!(
        fs::read(server_dir.join("newname").join("level.dat")).unwrap(),
        b"new overworld, already installed"
    );
    let props = fs::read_to_string(server_dir.join("server.properties")).unwrap();
    assert!(props.contains("level-name=newname"));
    assert!(!server_dir.join("world_slots").join(".replace").exists());
}
