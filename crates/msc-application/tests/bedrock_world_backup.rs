//! Phase 10 Bedrock world/backup integration cases.
//!
//! These tests keep the Phase 6 slot model visible at the Bedrock boundary:
//! the live tree is `worlds/<level-name>/`, a slot archive contains the
//! `worlds` container, and a running backup uses BDS's save-hold protocol.
//! The fixtures are loaded here so the expectations stay tied to the
//! extracted MSC 1 behavior rather than only to test names.

use msc_application::backups::{self, BackupConsole, BackupError};
use msc_application::worlds;
use msc_domain::identity::ServerType;
use msc_domain::world::{BackupAssociation, WorldSlot};
use msc_infrastructure::fs::StdFileSystem;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zip::ZipArchive;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "msc2-bedrock-world-backup-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
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

fn fixture(case: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/bedrock-backup")
        .join(format!("{case}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn write_file(path: &Path, contents: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn make_bedrock_world(server_dir: &Path, level_name: &str, contents: &[u8]) {
    write_file(
        &server_dir.join("worlds").join(level_name).join("level.dat"),
        contents,
    );
    write_file(
        &server_dir.join("server.properties"),
        format!("level-name={level_name}\n").as_bytes(),
    );
}

fn zip_entry_names(path: &Path) -> Vec<String> {
    let file = fs::File::open(path).unwrap();
    ZipArchive::new(file)
        .unwrap()
        .file_names()
        .map(str::to_owned)
        .collect()
}

fn write_slot_archive(server_dir: &Path, slot_id: &str, level_name: &str, contents: &[u8]) {
    let path = server_dir
        .join("world_slots")
        .join(slot_id)
        .join("world.zip");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    zip.start_file(
        format!("worlds/{level_name}/level.dat"),
        SimpleFileOptions::default(),
    )
    .unwrap();
    std::io::Write::write_all(&mut zip, contents).unwrap();
    zip.finish().unwrap();
}

fn archived_slot(id: &str, level_name: &str) -> WorldSlot {
    WorldSlot {
        id: id.to_owned(),
        name: level_name.to_owned(),
        created_at: "2026-08-23T12:00:00Z".to_owned(),
        last_played_at: None,
        thumbnail_file_name: None,
        world_level_name: Some(level_name.to_owned()),
        world_seed: None,
        zip_size_bytes: None,
    }
}

fn write_slot(server_dir: &Path, slot: &WorldSlot) {
    let path = server_dir
        .join("world_slots")
        .join(&slot.id)
        .join("slot.json");
    write_file(&path, &serde_json::to_vec(&slot.encode()).unwrap());
}

/// A deterministic runtime double for `save hold` / `save query` /
/// `save resume`. `wait_for_line` does not need to parse a real console line:
/// its result represents whether the matching readiness line arrived during
/// that poll.
struct ScriptedConsole {
    send_results: HashMap<String, bool>,
    ready_after: Option<usize>,
    deadline_after: usize,
    commands: Mutex<Vec<String>>,
    query_attempts: Mutex<usize>,
}

impl ScriptedConsole {
    fn new(send_results: HashMap<String, bool>) -> Self {
        Self {
            send_results,
            ready_after: None,
            deadline_after: usize::MAX,
            commands: Mutex::new(Vec::new()),
            query_attempts: Mutex::new(0),
        }
    }

    fn with_ready_after(mut self, attempts: usize) -> Self {
        self.ready_after = Some(attempts);
        self
    }

    fn with_deadline_after(mut self, attempts: usize) -> Self {
        self.deadline_after = attempts;
        self
    }

    fn commands(&self) -> Vec<String> {
        self.commands.lock().unwrap().clone()
    }
}

impl BackupConsole for ScriptedConsole {
    fn send(&self, command: &str) -> bool {
        self.commands.lock().unwrap().push(command.to_owned());
        if command == "save query" {
            *self.query_attempts.lock().unwrap() += 1;
        }
        self.send_results.get(command).copied().unwrap_or(true)
    }

    fn wait_for_line(&self, _matches: &dyn Fn(&str) -> bool) -> bool {
        self.ready_after
            .is_some_and(|ready_after| *self.query_attempts.lock().unwrap() >= ready_after)
    }

    fn deadline_reached(&self) -> bool {
        *self.query_attempts.lock().unwrap() >= self.deadline_after
    }
}

fn backup_association(slot_id: &str) -> BackupAssociation {
    BackupAssociation {
        slot_id: Some(slot_id.to_owned()),
        slot_name: Some("Realm".to_owned()),
        world_seed: None,
    }
}

fn create_bedrock_backup(
    server_dir: &Path,
    association: &BackupAssociation,
    console: Option<&dyn BackupConsole>,
    still_running: bool,
) -> Result<backups::BackupCreationResult, BackupError> {
    backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Bedrock,
        None,
        association,
        Some("bedrock-test"),
        Some("Bedrock test"),
        false,
        true,
        None,
        None,
        "2026-08-23T12:00:00Z",
        console,
        move || still_running,
        || false,
    )
}

#[test]
fn bedrock_slots_read_configured_level_and_archive_worlds_container() {
    let temp = TempDir::new("slots");
    make_bedrock_world(temp.path(), "Realm", b"realm");
    write_file(&temp.path().join("worlds/Other/level.dat"), b"other");

    let slot = worlds::create_slot_from_current_world(
        &StdFileSystem,
        temp.path(),
        ServerType::Bedrock,
        None,
        "Realm snapshot",
        None,
        "2026-08-23T12:00:00Z",
    )
    .unwrap();

    assert_eq!(slot.world_level_name.as_deref(), Some("Realm"));
    assert!(
        zip_entry_names(
            &temp
                .path()
                .join("world_slots")
                .join(&slot.id)
                .join("world.zip")
        )
        .iter()
        .all(|name| name.starts_with("worlds/"))
    );
}

#[test]
fn bedrock_slot_activation_preserves_flat_world_layout_transactionally() {
    let temp = TempDir::new("activation");
    make_bedrock_world(temp.path(), "Realm", b"old");
    let slot = archived_slot("OTHER", "Other");
    write_slot(temp.path(), &slot);
    write_slot_archive(temp.path(), &slot.id, "Other", b"new");

    let mut backup_called = false;
    worlds::activate_slot(
        &StdFileSystem,
        temp.path(),
        ServerType::Bedrock,
        &slot,
        false,
        "2026-08-23T12:00:00Z",
        || {
            backup_called = true;
            true
        },
        || false,
    )
    .unwrap();

    assert!(backup_called, "live Bedrock data requires a safety backup");
    assert_eq!(
        fs::read(temp.path().join("worlds/Other/level.dat")).unwrap(),
        b"new"
    );
    assert!(!temp.path().join("worlds/Realm").exists());
    assert!(
        fs::read_to_string(temp.path().join("server.properties"))
            .unwrap()
            .contains("level-name=Other")
    );
}

#[test]
fn bedrock_backup_success_uses_slot_association_and_save_resume() {
    let fixture = fixture("save-resume-runs-after-successful-zip");
    let temp = TempDir::new("backup-success");
    make_bedrock_world(temp.path(), "Realm", b"realm");
    let console = ScriptedConsole::new(HashMap::from([
        ("save hold".to_owned(), true),
        ("save query".to_owned(), true),
        ("save resume".to_owned(), true),
    ]))
    .with_ready_after(1);

    let result = create_bedrock_backup(
        temp.path(),
        &backup_association("REALM-SLOT"),
        Some(&console),
        true,
    )
    .unwrap();

    assert_eq!(
        console.commands(),
        fixture["expected"]["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    );
    assert!(
        zip_entry_names(&result.zip_path)
            .iter()
            .any(|name| name == "worlds/Realm/level.dat")
    );
    let sidecar = result.zip_path.with_extension("meta.json");
    let metadata: Value = serde_json::from_slice(&fs::read(sidecar).unwrap()).unwrap();
    assert_eq!(metadata["slotId"], "REALM-SLOT");
}

#[test]
fn bedrock_backup_query_failures_and_hold_failure_are_best_effort() {
    let timeout_fixture = fixture("save-query-timeout-still-proceeds");
    let temp = TempDir::new("backup-timeout");
    make_bedrock_world(temp.path(), "Realm", b"realm");
    let timeout_console = ScriptedConsole::new(HashMap::from([
        ("save hold".to_owned(), true),
        ("save query".to_owned(), true),
        ("save resume".to_owned(), true),
    ]))
    .with_deadline_after(3);
    let result = create_bedrock_backup(
        temp.path(),
        &backup_association("REALM-SLOT"),
        Some(&timeout_console),
        true,
    )
    .unwrap();
    assert!(result.zip_path.is_file());
    assert_eq!(
        timeout_console.commands(),
        [
            "save hold",
            "save query",
            "save query",
            "save query",
            "save resume"
        ]
    );
    assert_eq!(timeout_fixture["expected"]["backup_proceeds"], true);

    let hold_failure_fixture = fixture("save-hold-send-failure-backs-up-live-files");
    let hold_failure = ScriptedConsole::new(HashMap::from([("save hold".to_owned(), false)]));
    let result = create_bedrock_backup(
        temp.path(),
        &backup_association("REALM-SLOT"),
        Some(&hold_failure),
        true,
    )
    .unwrap();
    assert!(result.zip_path.is_file());
    assert_eq!(hold_failure.commands(), ["save hold"]);
    assert_eq!(hold_failure_fixture["expected"]["saves_paused"], false);
}

#[cfg(unix)]
#[test]
fn bedrock_backup_failure_resumes_saves_and_removes_partial_archive() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = fixture("save-resume-runs-after-failed-zip");
    let temp = TempDir::new("backup-failure");
    make_bedrock_world(temp.path(), "Realm", b"realm");
    let locked = temp.path().join("worlds/Realm/locked");
    fs::create_dir_all(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let console = ScriptedConsole::new(HashMap::from([
        ("save hold".to_owned(), true),
        ("save query".to_owned(), true),
        ("save resume".to_owned(), true),
    ]))
    .with_ready_after(1);

    let result = create_bedrock_backup(
        temp.path(),
        &backup_association("REALM-SLOT"),
        Some(&console),
        true,
    );
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(matches!(result, Err(BackupError::Archive(_))));
    assert_eq!(
        console.commands().last().map(String::as_str),
        Some(fixture["expected"]["final_command"].as_str().unwrap())
    );
    assert!(
        fs::read_dir(temp.path().join("backups"))
            .unwrap()
            .next()
            .is_none()
    );
}

#[test]
fn bedrock_live_restore_stays_on_world_slots() {
    let fixture = fixture("bedrock-live-restore-redirects-to-world-slots");
    let temp = TempDir::new("restore-boundary");
    make_bedrock_world(temp.path(), "Realm", b"realm");
    let backup = temp.path().join("backup.zip");
    write_slot_archive(temp.path(), "unused", "Realm", b"backup");
    fs::copy(temp.path().join("world_slots/unused/world.zip"), &backup).unwrap();

    let error = backups::restore_backup(
        &StdFileSystem,
        temp.path(),
        ServerType::Bedrock,
        None,
        &backup,
        None,
        None,
        false,
        &BackupAssociation::default(),
        None,
        None,
        "2026-08-23T12:00:00Z",
        || false,
    )
    .unwrap_err();

    assert!(matches!(error, backups::RestoreError::BedrockNotSupported));
    assert_eq!(fixture["expected"]["destination"], "Worlds tab");
}
