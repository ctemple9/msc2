//! Port of `fixtures/world-mutations/`'s 10 non-activation, non-direct-
//! rename/replace cases (P6.5), exercising `msc_application::worlds`'s
//! slot CRUD/copy/import/export functions (P6.12).
//!
//! Real on-disk server directories, same "genuinely disk-shaped"
//! precedent `world_import_reconciliation.rs` (P6.11) already set —
//! necessary here too since `msc_infrastructure::archive` requires real
//! files. Test functions are prefixed `world_slot_crud_` so the plan's
//! Verify command (a plain nextest substring filter on test name)
//! selects them.

use msc_application::worlds::{self, WorldError};
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
            "msc2-world-slot-crud-test-{label}-{}",
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

fn write_slot(server_dir: &Path, slot: &WorldSlot) {
    let bytes = serde_json::to_vec_pretty(&slot.encode()).unwrap();
    write_file(
        &server_dir
            .join("world_slots")
            .join(&slot.id)
            .join("slot.json"),
        &bytes,
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

fn plain_slot(id: &str, name: &str, created_at: &str) -> WorldSlot {
    WorldSlot {
        id: id.to_string(),
        name: name.to_string(),
        created_at: created_at.to_string(),
        last_played_at: None,
        thumbnail_file_name: None,
        world_level_name: Some("world".to_string()),
        world_seed: Some("1234".to_string()),
        zip_size_bytes: None,
    }
}

#[test]
fn world_slot_crud_create_slot_java_zips_main_nether_end() {
    let tmp = TempDir::new("create-java");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");
    make_live_folder(server_dir, "world_nether", b"nether");
    make_live_folder(server_dir, "world_the_end", b"end");
    make_live_folder(server_dir, "world_unrelated_leftover", b"leftover");

    let slot = worlds::create_slot_from_current_world(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "Before the Nether trip",
        None,
        "2026-06-01T00:00:00Z",
    )
    .unwrap();

    assert_eq!(slot.name, "Before the Nether trip");
    assert!(slot.zip_size_bytes.unwrap() > 0);

    let zip_bytes = fs::read(
        server_dir
            .join("world_slots")
            .join(&slot.id)
            .join("world.zip"),
    )
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).unwrap();
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect();
    assert!(names.iter().any(|n| n.starts_with("world/")));
    assert!(names.iter().any(|n| n.starts_with("world_nether/")));
    assert!(names.iter().any(|n| n.starts_with("world_the_end/")));
    assert!(
        !names
            .iter()
            .any(|n| n.starts_with("world_unrelated_leftover/"))
    );

    // Live world folders were only ever read from, never touched.
    assert!(server_dir.join("world_unrelated_leftover").is_dir());
}

#[test]
fn world_slot_crud_create_slot_bedrock_zips_worlds_folder() {
    let tmp = TempDir::new("create-bedrock");
    let server_dir = tmp.path();
    write_file(&server_dir.join("worlds").join("db").join("dummy"), b"db");

    let slot = worlds::create_slot_from_current_world(
        &StdFileSystem,
        server_dir,
        ServerType::Bedrock,
        None,
        "Realm export",
        None,
        "2026-06-01T00:00:00Z",
    )
    .unwrap();

    let zip_bytes = fs::read(
        server_dir
            .join("world_slots")
            .join(&slot.id)
            .join("world.zip"),
    )
    .unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).unwrap();
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect();
    assert!(names.iter().all(|n| n.starts_with("worlds/")));
}

// Unix-only: forcing this native (non-shell) zip writer to fail *after*
// the slot directory already exists — the case the fixture's own
// "directory removed" expectation is about — needs a real I/O failure
// inside `add_dir_recursive`'s directory walk. `archive::
// create_zip_from_folders` writes straight to `std::fs`, bypassing the
// injectable `FileSystem` trait entirely (the same real-disk-only
// convention P6.10's own archive module already established), so the
// only reliable, UUID-independent way to trigger that is a locked-down
// subdirectory — Windows has no equivalent permission primitive this
// port relies on elsewhere, so this one case has no Windows twin.
#[test]
#[cfg(unix)]
fn world_slot_crud_create_slot_zip_failure_cleans_up_slot_directory() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new("create-zip-failure");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld");
    let locked = server_dir.join("world").join("locked");
    fs::create_dir_all(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    let result = worlds::create_slot_from_current_world(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        "Ill-fated slot",
        None,
        "2026-06-01T00:00:00Z",
    );

    // Restore permissions so `TempDir`'s `Drop` can clean up regardless
    // of the assertions below.
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    let err = result.unwrap_err();
    assert!(matches!(err, WorldError::Archive(_)));
    // The just-created slot directory was removed — no half-written
    // `slot.json` or partial archive left behind.
    let remaining: Vec<_> = fs::read_dir(server_dir.join("world_slots"))
        .into_iter()
        .flatten()
        .collect();
    assert!(remaining.is_empty());
    // Live world folder was never touched.
    assert!(server_dir.join("world").join("level.dat").is_file());
}

// Unix-only, for the same reason `create_slot_zip_failure_cleans_up_
// slot_directory` is: the archive writer bypasses the injectable
// `FileSystem` trait entirely, so a real cross-platform-portable
// failure trigger doesn't exist for it — a locked-down slot directory
// (blocking the scratch file's own creation) is the reliable Unix
// equivalent of source's "zip process exits non-zero".
#[test]
#[cfg(unix)]
fn world_slot_crud_update_active_slot_zip_failure_preserves_previous_archive() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new("update-zip-failure");
    let server_dir = tmp.path();
    make_live_folder(server_dir, "world", b"overworld-new");
    let slot = plain_slot("slot-active", "Active slot", "2026-01-01T00:00:00Z");
    write_slot(server_dir, &slot);
    write_slot_archive(server_dir, &slot.id, "world", b"overworld-old");
    let previous_bytes = fs::read(
        server_dir
            .join("world_slots")
            .join(&slot.id)
            .join("world.zip"),
    )
    .unwrap();

    let slot_dir = server_dir.join("world_slots").join(&slot.id);
    fs::set_permissions(&slot_dir, fs::Permissions::from_mode(0o555)).unwrap();

    let result = worlds::update_active_slot_from_current_world(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        Some("world"),
        &slot,
    );

    fs::set_permissions(&slot_dir, fs::Permissions::from_mode(0o755)).unwrap();

    let err = result.unwrap_err();
    assert!(matches!(err, WorldError::Archive(_) | WorldError::Io(_)));

    let zip_path = slot_dir.join("world.zip");
    assert_eq!(fs::read(&zip_path).unwrap(), previous_bytes);
    assert!(!slot_dir.join("world.update.tmp.zip").exists());
}

#[test]
fn world_slot_crud_rename_slot_metadata_only_leaves_archive_untouched() {
    let tmp = TempDir::new("rename");
    let server_dir = tmp.path();
    let slot = plain_slot("slot-abc", "Old build", "2026-01-01T00:00:00Z");
    write_slot(server_dir, &slot);
    write_slot_archive(server_dir, &slot.id, "world", b"world bytes");
    let archive_before = fs::read(
        server_dir
            .join("world_slots")
            .join(&slot.id)
            .join("world.zip"),
    )
    .unwrap();

    let updated =
        worlds::rename_slot(&StdFileSystem, server_dir, &slot, "Redstone farm v2").unwrap();

    assert_eq!(updated.name, "Redstone farm v2");
    assert_eq!(updated.id, "slot-abc");
    assert_eq!(updated.world_level_name.as_deref(), Some("world"));
    let archive_after = fs::read(
        server_dir
            .join("world_slots")
            .join(&slot.id)
            .join("world.zip"),
    )
    .unwrap();
    assert_eq!(archive_before, archive_after);
}

#[test]
fn world_slot_crud_delete_active_slot_refused() {
    let tmp = TempDir::new("delete-active-refused");
    let server_dir = tmp.path();
    let slot = plain_slot("slot-active", "Active", "2026-01-01T00:00:00Z");
    write_slot(server_dir, &slot);

    let err =
        worlds::delete_slot(&StdFileSystem, server_dir, &slot, Some("slot-active")).unwrap_err();
    assert!(matches!(err, WorldError::ActiveSlotDeleteRefused));
    assert!(
        server_dir
            .join("world_slots")
            .join(&slot.id)
            .join("slot.json")
            .is_file()
    );
}

#[test]
fn world_slot_crud_delete_non_active_slot_succeeds() {
    let tmp = TempDir::new("delete-non-active");
    let server_dir = tmp.path();
    let slot = plain_slot("slot-other", "Other", "2026-01-01T00:00:00Z");
    write_slot(server_dir, &slot);

    worlds::delete_slot(&StdFileSystem, server_dir, &slot, Some("slot-active")).unwrap();
    assert!(!server_dir.join("world_slots").join(&slot.id).exists());
}

#[test]
fn world_slot_crud_duplicate_slot_fresh_uuid_source_untouched() {
    let tmp = TempDir::new("duplicate");
    let server_dir = tmp.path();
    let source = plain_slot("slot-src", "Base camp", "2026-01-01T00:00:00Z");
    write_slot(server_dir, &source);
    write_slot_archive(server_dir, &source.id, "world", b"base camp bytes");

    let new_slot = worlds::duplicate_slot(
        &StdFileSystem,
        server_dir,
        &source,
        "Base camp (copy)",
        "2026-06-01T00:00:00Z",
    )
    .unwrap();

    assert_ne!(new_slot.id, source.id);
    assert_eq!(new_slot.name, "Base camp (copy)");
    assert_eq!(new_slot.world_level_name, source.world_level_name);
    assert_eq!(new_slot.world_seed, source.world_seed);

    // Source untouched.
    assert!(
        server_dir
            .join("world_slots")
            .join(&source.id)
            .join("world.zip")
            .is_file()
    );
    assert!(
        server_dir
            .join("world_slots")
            .join(&new_slot.id)
            .join("world.zip")
            .is_file()
    );
}

// Unix-only: `copy_via_fs` writes through the injectable `FileSystem`
// trait, but `StdFileSystem` (the only implementation with real zip
// files to copy between) has no built-in write-failure injection, so a
// locked-down destination directory is the reliable trigger — same
// reasoning as this file's other two Unix-gated zip-failure cases,
// standing in for source's "source zip vanished mid-copy".
#[test]
#[cfg(unix)]
fn world_slot_crud_copy_into_existing_mid_copy_failure_preserves_destination() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new("copy-into-existing-failure");
    let server_dir = tmp.path();
    let source = plain_slot("slot-src", "Fresh build", "2026-01-01T00:00:00Z");
    write_slot(server_dir, &source);
    write_slot_archive(server_dir, &source.id, "world", b"fresh build bytes");
    let destination = plain_slot("slot-dst", "Old base", "2026-01-02T00:00:00Z");
    write_slot(server_dir, &destination);
    write_slot_archive(server_dir, &destination.id, "world", b"old base bytes");
    let dest_dir = server_dir.join("world_slots").join(&destination.id);
    let dest_zip_before = fs::read(dest_dir.join("world.zip")).unwrap();

    fs::set_permissions(&dest_dir, fs::Permissions::from_mode(0o555)).unwrap();

    let result = worlds::copy_slot_into_existing(
        &StdFileSystem,
        server_dir,
        &source,
        &destination,
        "2026-06-01T00:00:00Z",
    );

    fs::set_permissions(&dest_dir, fs::Permissions::from_mode(0o755)).unwrap();

    let err = result.unwrap_err();
    assert!(matches!(err, WorldError::Io(_)));

    let dest_zip_after = fs::read(dest_dir.join("world.zip")).unwrap();
    assert_eq!(dest_zip_before, dest_zip_after);
    assert!(!dest_dir.join("world.replace.tmp.zip").exists());
    // The destination's metadata (name preserved) is untouched too.
    let meta = fs::read_to_string(dest_dir.join("slot.json")).unwrap();
    assert!(meta.contains("Old base"));
}

#[test]
fn world_slot_crud_copy_into_existing_success_overwrites_destination() {
    let tmp = TempDir::new("copy-into-existing-success");
    let server_dir = tmp.path();
    let source = plain_slot("slot-src", "Fresh build", "2026-01-01T00:00:00Z");
    write_slot(server_dir, &source);
    write_slot_archive(server_dir, &source.id, "world", b"fresh build bytes");
    let destination = plain_slot("slot-dst", "Old base", "2026-01-02T00:00:00Z");
    write_slot(server_dir, &destination);
    write_slot_archive(server_dir, &destination.id, "world", b"old base bytes");

    let updated = worlds::copy_slot_into_existing(
        &StdFileSystem,
        server_dir,
        &source,
        &destination,
        "2026-06-01T00:00:00Z",
    )
    .unwrap();

    assert_eq!(updated.id, "slot-dst");
    assert_eq!(updated.name, "Old base");
    assert_eq!(updated.created_at, "2026-06-01T00:00:00Z");
    let dest_zip = fs::read(
        server_dir
            .join("world_slots")
            .join(&destination.id)
            .join("world.zip"),
    )
    .unwrap();
    let source_zip = fs::read(
        server_dir
            .join("world_slots")
            .join(&source.id)
            .join("world.zip"),
    )
    .unwrap();
    assert_eq!(dest_zip, source_zip);
}

#[test]
fn world_slot_crud_export_slot_zip_overwrites_destination() {
    let tmp = TempDir::new("export");
    let server_dir = tmp.path();
    let slot = plain_slot("slot-src", "Export me", "2026-01-01T00:00:00Z");
    write_slot(server_dir, &slot);
    write_slot_archive(server_dir, &slot.id, "world", b"export me bytes");

    let dest = tmp.path().join("outside").join("export-destination.zip");
    write_file(&dest, b"stale contents that must be overwritten");

    worlds::export_slot_zip(&StdFileSystem, server_dir, &slot, &dest).unwrap();

    let exported = fs::read(&dest).unwrap();
    let source_zip = fs::read(
        server_dir
            .join("world_slots")
            .join(&slot.id)
            .join("world.zip"),
    )
    .unwrap();
    assert_eq!(exported, source_zip);
    // Source slot untouched.
    assert!(
        server_dir
            .join("world_slots")
            .join(&slot.id)
            .join("world.zip")
            .is_file()
    );
}

#[test]
fn world_slot_crud_import_zip_as_new_slot_infers_level_name_and_seed_no_structural_validation() {
    let tmp = TempDir::new("import-zip");
    let server_dir = tmp.path();

    // A real, committed level.dat sample (P6.7) with a known seed ("0"),
    // packaged the way an exported MSC slot zip is shaped: a single
    // top-level folder named after the level.
    let level_dat =
        fs::read("../../fixtures/world-nbt/samples/java-real-legacy-fields-level.dat.gz")
            .expect("real NBT sample fixture is committed");
    let source_zip = tmp.path().join("outside").join("my-world.zip");
    fs::create_dir_all(source_zip.parent().unwrap()).unwrap();
    let file = fs::File::create(&source_zip).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    zip.start_file("campack/level.dat", opts).unwrap();
    zip.write_all(&level_dat).unwrap();
    zip.finish().unwrap();
    let raw_zip_bytes = fs::read(&source_zip).unwrap();

    let slot = worlds::import_zip_as_new_slot(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        None,
        &source_zip,
        "Imported from friend",
        "2026-06-01T00:00:00Z",
    )
    .unwrap();

    assert_eq!(slot.name, "Imported from friend");
    assert_eq!(slot.world_level_name.as_deref(), Some("campack"));
    assert_eq!(slot.world_seed.as_deref(), Some("0"));

    // Copied verbatim, byte-for-byte — no structural validation, no
    // re-zip.
    let copied = fs::read(
        server_dir
            .join("world_slots")
            .join(&slot.id)
            .join("world.zip"),
    )
    .unwrap();
    assert_eq!(copied, raw_zip_bytes);

    // The original source zip is untouched.
    assert_eq!(fs::read(&source_zip).unwrap(), raw_zip_bytes);
}

#[test]
fn world_slot_crud_import_zip_missing_source_rejected() {
    let tmp = TempDir::new("import-zip-missing");
    let server_dir = tmp.path();
    let missing = tmp.path().join("does-not-exist.zip");

    let err = worlds::import_zip_as_new_slot(
        &StdFileSystem,
        server_dir,
        ServerType::Java,
        None,
        &missing,
        "Imported",
        "2026-06-01T00:00:00Z",
    )
    .unwrap_err();
    assert!(matches!(err, WorldError::NoSourceZip));
}
