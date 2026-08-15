//! Port of `fixtures/world-mutations/`'s 4 direct world rename/replace
//! cases (P6.5), exercising `msc_application::worlds::rename_world`/
//! `replace_world` (P6.14).
//!
//! Real on-disk server directories, same precedent every other
//! archive-touching test file in this phase already set. Test functions
//! are prefixed `world_mutations_` so the plan's Verify command (a
//! plain nextest substring filter on test name) selects them.

use msc_application::worlds::{self, WorldError, WorldReplaceSource};
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
fn world_mutations_replace_world_folder_removal_failure_aborts_before_extraction() {
    let tmp = TempDir::new("replace-removal-failure");
    let server_dir = tmp.path();
    write_server_properties(server_dir, "world");
    make_folder(server_dir, "world", b"overworld");
    make_folder(server_dir, "world_nether", b"nether");
    make_folder(server_dir, "world_the_end", b"end");

    // Build a valid replacement backup zip.
    let backup_zip = tmp.path().join("outside").join("backup.zip");
    fs::create_dir_all(backup_zip.parent().unwrap()).unwrap();
    let file = fs::File::create(&backup_zip).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    zip.start_file("world/level.dat", opts).unwrap();
    zip.write_all(b"replacement overworld").unwrap();
    zip.finish().unwrap();

    // Force folder removal to fail, cleanly: lock down "world" itself
    // (not its parent) so deleting `level.dat` inside it — the first
    // thing a recursive removal does — is refused outright, leaving
    // "world" completely untouched rather than partially emptied. On
    // non-Unix this specific removal failure can't be forced without a
    // real locked file, so this test is Unix-only for the same reason
    // `world_slot_crud`'s zip-failure cases are.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let world_dir = server_dir.join("world");
        fs::set_permissions(&world_dir, fs::Permissions::from_mode(0o555)).unwrap();

        let result = worlds::replace_world(
            &StdFileSystem,
            server_dir,
            ServerType::Java,
            Some("world"),
            "newname",
            &WorldReplaceSource::BackupZip(backup_zip),
            false,
            false,
            || true,
        );

        fs::set_permissions(&world_dir, fs::Permissions::from_mode(0o755)).unwrap();

        let err = result.unwrap_err();
        assert!(matches!(err, WorldError::Io(_)));
        // Old world still there (removal aborted before completing),
        // and server.properties was never touched.
        assert!(server_dir.join("world").join("level.dat").is_file());
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
        false,
        || panic!("backup must not run when refused for running"),
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
        false,
        || true,
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
        false,
        || true,
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
        false,
        || panic!("backup must not run before source validation"),
    )
    .unwrap_err();

    assert!(matches!(err, WorldError::InvalidWorldSource));
    assert!(server_dir.join("world").join("level.dat").is_file());
}
