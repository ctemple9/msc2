//! Ports `AppViewModel+WorldRepair.swift`'s Bedrock world-format repair.
//!
//! The repair keeps the world's database untouched while asking Bedrock to
//! generate a fresh set of format files with the current server version. The
//! safety backup is deliberately supplied by the caller: this module owns
//! the ordering rule (backup before any property change), but not the backup
//! policy or its many-argument filesystem implementation.
//!
//! Bedrock lifecycle calls stay behind [`RepairServerControl`]. This keeps the
//! application-layer orchestration testable and leaves the real start/stop
//! adapter for the agent layer, just as `WorldConverter` does for Chunker.

use crate::worlds;
use msc_infrastructure::fs::FileSystem;
use std::fmt;
use std::io;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

const REPAIR_TEMP_LEVEL_NAME: &str = "_msc_repair_temp";
const START_TIMEOUT: Duration = Duration::from_secs(180);
const STOP_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const POST_STOP_SETTLE: Duration = Duration::from_secs(1);

/// The four lifecycle signals used by the source repair loop:
/// `startServer`, `serverReadyForAutoMetrics`, `stopServer`, and
/// `isServerRunning`.
pub trait RepairServerControl {
    fn start(&self);
    fn is_ready(&self) -> bool;
    fn stop(&self);
    fn is_running(&self) -> bool;
}

#[derive(Debug)]
pub enum WorldRepairError {
    NoLevelName,
    BackupFailed,
    StartTimedOut,
    Io(io::Error),
    RestoreFailed(io::Error),
}

impl fmt::Display for WorldRepairError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoLevelName => write!(f, "level-name is missing from server.properties"),
            Self::BackupFailed => write!(f, "pre-repair backup failed"),
            Self::StartTimedOut => write!(f, "timed out waiting for Bedrock to become ready"),
            Self::Io(error) => write!(f, "{error}"),
            Self::RestoreFailed(error) => {
                write!(
                    f,
                    "could not restore level-name in server.properties: {error}"
                )
            }
        }
    }
}

impl std::error::Error for WorldRepairError {}

impl From<io::Error> for WorldRepairError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// `repairWorldLevelDat(logLine:)` (source lines 19-137).
///
/// `pre_repair_backup` is called after a valid `level-name` has been read and
/// before `server.properties` is changed. A `false` result aborts with no
/// further filesystem or lifecycle work. Progress messages mirror the source
/// because the eventual route/client can surface them as operation output.
pub fn repair_world(
    fs: &dyn FileSystem,
    control: &dyn RepairServerControl,
    server_dir: &Path,
    pre_repair_backup: impl FnOnce() -> bool,
    mut progress: impl FnMut(&str),
) -> Result<(), WorldRepairError> {
    let properties_path = server_dir.join("server.properties");
    let mut original_properties = worlds::read_properties_map(fs, &properties_path);
    let original_level_name = original_properties
        .get("level-name")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or(WorldRepairError::NoLevelName)?;

    progress(&format!("World: \"{original_level_name}\""));
    progress("Creating backup of current world...");
    if !pre_repair_backup() {
        progress("Backup failed — aborting repair.");
        return Err(WorldRepairError::BackupFailed);
    }
    progress("Backup created successfully.");

    let temp_name = REPAIR_TEMP_LEVEL_NAME;
    original_properties.insert("level-name".to_owned(), temp_name.to_owned());
    if let Err(error) = worlds::write_properties_map(fs, &properties_path, &original_properties) {
        return restore_after_failure(fs, &properties_path, &original_level_name, error.into());
    }

    progress("Starting server briefly to generate updated world format...");
    control.start();
    let start_deadline = Instant::now() + START_TIMEOUT;
    while !control.is_ready() {
        if Instant::now() > start_deadline {
            progress("Timed out waiting for server to start. Aborting.");
            control.stop();
            return restore_after_failure(
                fs,
                &properties_path,
                &original_level_name,
                WorldRepairError::StartTimedOut,
            );
        }
        thread::sleep(POLL_INTERVAL);
    }

    progress("Server reached ready state — stopping...");
    control.stop();
    let stop_deadline = Instant::now() + STOP_TIMEOUT;
    while control.is_running() {
        if Instant::now() > stop_deadline {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    // BDS may still be flushing its world files after its running signal
    // drops; preserve the source's settling window before copying them.
    thread::sleep(POST_STOP_SETTLE);

    progress("Applying updated world format files...");
    let temp_world_dir = server_dir.join("worlds").join(temp_name);
    let real_world_dir = server_dir.join("worlds").join(&original_level_name);
    if !is_directory(fs, &real_world_dir) {
        progress("Original world folder not found at expected path — restoring server.properties.");
        return restore_after_failure(
            fs,
            &properties_path,
            &original_level_name,
            io_error("original world folder not found").into(),
        );
    }

    for file_name in ["level.dat", "level.dat_old", "levelname.txt"] {
        let source = temp_world_dir.join(file_name);
        let source_metadata = match fs.stat(&source) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => {
                return restore_after_failure(
                    fs,
                    &properties_path,
                    &original_level_name,
                    error.into(),
                );
            }
        };
        if !source_metadata.is_file {
            return restore_after_failure(
                fs,
                &properties_path,
                &original_level_name,
                io_error("repair source is not a file").into(),
            );
        }

        let destination = real_world_dir.join(file_name);
        let bytes = match fs.read(&source) {
            Ok(bytes) => bytes,
            Err(error) => {
                return restore_after_failure(
                    fs,
                    &properties_path,
                    &original_level_name,
                    error.into(),
                );
            }
        };
        if fs.stat(&destination).is_ok()
            && let Err(error) = fs.remove(&destination)
        {
            return restore_after_failure(fs, &properties_path, &original_level_name, error.into());
        }
        if let Err(error) = fs.write(&destination, &bytes) {
            return restore_after_failure(fs, &properties_path, &original_level_name, error.into());
        }
    }
    progress("Format files updated.");

    progress("Removing temporary files...");
    let _ = fs.remove(&temp_world_dir);

    original_properties.insert("level-name".to_owned(), original_level_name);
    worlds::write_properties_map(fs, &properties_path, &original_properties)
        .map_err(WorldRepairError::RestoreFailed)
}

fn restore_after_failure<T>(
    fs: &dyn FileSystem,
    properties_path: &Path,
    original_level_name: &str,
    error: WorldRepairError,
) -> Result<T, WorldRepairError> {
    let mut properties = worlds::read_properties_map(fs, properties_path);
    properties.insert("level-name".to_owned(), original_level_name.to_owned());
    match worlds::write_properties_map(fs, properties_path, &properties) {
        Ok(()) => Err(error),
        Err(restore_error) => Err(WorldRepairError::RestoreFailed(restore_error)),
    }
}

fn is_directory(fs: &dyn FileSystem, path: &Path) -> bool {
    matches!(fs.stat(path), Ok(metadata) if metadata.is_dir)
}

fn io_error(message: &str) -> io::Error {
    io::Error::other(message)
}
