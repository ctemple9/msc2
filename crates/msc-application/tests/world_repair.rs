use msc_application::world_repair::{RepairServerControl, WorldRepairError, repair_world};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct FakeControl {
    events: Mutex<Vec<&'static str>>,
    running: Mutex<bool>,
}

impl RepairServerControl for FakeControl {
    fn start(&self) {
        self.events.lock().unwrap().push("start");
        *self.running.lock().unwrap() = true;
    }

    fn is_ready(&self) -> bool {
        self.events.lock().unwrap().push("is_ready");
        true
    }

    fn stop(&self) {
        self.events.lock().unwrap().push("stop");
        *self.running.lock().unwrap() = false;
    }

    fn is_running(&self) -> bool {
        self.events.lock().unwrap().push("is_running");
        *self.running.lock().unwrap()
    }
}

fn filesystem_with_world() -> FakeFileSystem {
    FakeFileSystem::new()
        .with_file(
            "/server/server.properties",
            b"level-name=survival\nunknown-setting=kept\n".to_vec(),
            false,
        )
        .with_file(
            "/server/worlds/survival/level.dat",
            b"old-level".to_vec(),
            false,
        )
        .with_file(
            "/server/worlds/_msc_repair_temp/level.dat",
            b"new-level".to_vec(),
            false,
        )
        .with_file(
            "/server/worlds/_msc_repair_temp/level.dat_old",
            b"new-old".to_vec(),
            false,
        )
        .with_file(
            "/server/worlds/_msc_repair_temp/levelname.txt",
            b"new-name".to_vec(),
            false,
        )
}

#[test]
fn repairs_format_files_in_source_order_and_restores_properties() {
    let fs = filesystem_with_world();
    let control = FakeControl::default();
    let events = Arc::new(Mutex::new(Vec::new()));
    let backup_events = Arc::clone(&events);
    let mut progress = Vec::new();

    let result = repair_world(
        &fs,
        &control,
        Path::new("/server"),
        || {
            backup_events.lock().unwrap().push("backup");
            true
        },
        |line| progress.push(line.to_owned()),
    );

    assert!(result.is_ok());
    assert_eq!(events.lock().unwrap().as_slice(), ["backup"]);
    assert_eq!(
        control.events.lock().unwrap().as_slice(),
        ["start", "is_ready", "stop", "is_running"]
    );
    assert_eq!(
        fs.read(Path::new("/server/worlds/survival/level.dat"))
            .unwrap(),
        b"new-level"
    );
    assert_eq!(
        fs.read(Path::new("/server/worlds/survival/level.dat_old"))
            .unwrap(),
        b"new-old"
    );
    assert_eq!(
        fs.read(Path::new("/server/worlds/survival/levelname.txt"))
            .unwrap(),
        b"new-name"
    );
    assert!(
        fs.stat(Path::new("/server/worlds/_msc_repair_temp"))
            .is_err()
    );
    let properties =
        String::from_utf8(fs.read(Path::new("/server/server.properties")).unwrap()).unwrap();
    assert!(properties.contains("level-name=survival"));
    assert!(properties.contains("unknown-setting=kept"));
    assert_eq!(progress.first().unwrap(), "World: \"survival\"");
}

#[test]
fn skips_missing_format_files_but_still_cleans_up_and_restores() {
    let fs = filesystem_with_world();
    fs.remove(Path::new("/server/worlds/_msc_repair_temp/level.dat_old"))
        .unwrap();
    fs.remove(Path::new("/server/worlds/_msc_repair_temp/levelname.txt"))
        .unwrap();
    let control = FakeControl::default();
    let result = repair_world(&fs, &control, Path::new("/server"), || true, |_| {});

    assert!(result.is_ok());
    assert!(
        fs.stat(Path::new("/server/worlds/survival/level.dat_old"))
            .is_err()
    );
    assert!(
        fs.stat(Path::new("/server/worlds/survival/levelname.txt"))
            .is_err()
    );
    assert!(
        fs.stat(Path::new("/server/worlds/_msc_repair_temp"))
            .is_err()
    );
}

#[test]
fn backup_failure_aborts_before_property_or_lifecycle_changes() {
    let fs = filesystem_with_world();
    let control = FakeControl::default();
    let before = fs.read(Path::new("/server/server.properties")).unwrap();

    let result = repair_world(&fs, &control, Path::new("/server"), || false, |_| {});

    assert!(matches!(result, Err(WorldRepairError::BackupFailed)));
    assert_eq!(
        fs.read(Path::new("/server/server.properties")).unwrap(),
        before
    );
    assert!(control.events.lock().unwrap().is_empty());
}

#[test]
fn missing_level_name_aborts_before_backup() {
    let fs = FakeFileSystem::new().with_file(
        "/server/server.properties",
        b"server-port=19132\n".to_vec(),
        false,
    );
    let control = FakeControl::default();
    let backup_called = Arc::new(Mutex::new(false));
    let backup_called_for_closure = Arc::clone(&backup_called);

    let result = repair_world(
        &fs,
        &control,
        Path::new("/server"),
        || {
            *backup_called_for_closure.lock().unwrap() = true;
            true
        },
        |_| {},
    );

    assert!(matches!(result, Err(WorldRepairError::NoLevelName)));
    assert!(!*backup_called.lock().unwrap());
}
