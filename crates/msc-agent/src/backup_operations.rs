//! P6.31: the one authoritative agent-level backup operation.
//!
//! Before this step, a manual `POST /v1/backups/now` and a fired
//! scheduled tick took two different paths to the same
//! `msc_application::backups::create_backup`: the manual route
//! (`routes/backups.rs::now`, still true today for everything *except*
//! creation) journals a real operation through
//! `LifecycleOperations::begin_running` (ordinary per-server exclusivity,
//! shared with activation/restore/conversion/replacement), wires a real
//! [`LiveBackupConsole`] when the target is running, and hands
//! `create_backup` a real `should_cancel` from that same operation. The
//! scheduler's `LiveSchedulerBackend::run_scheduled_backup` instead called
//! `msc_application::backups::scheduled_tick`, which always passes
//! `console: None` and `should_cancel: || false` — no flush/pause
//! protocol on a live server, no cooperative cancellation, and gated by
//! `SchedulerBackend::admit_backup`, a stub that always returned `true`
//! (see this crate's prior `backup_scheduler.rs` module doc). A scheduled
//! backup could start while another operation already held that server's
//! exclusivity.
//!
//! [`start_backup`] is the fix: both `routes/backups.rs::now` and
//! `backup_scheduler.rs::LiveSchedulerBackend::run_scheduled_backup` now
//! call it directly. It performs the ordinary per-server operation
//! admission (`LifecycleOperations::begin_running`, the same call every
//! other Phase 6 mutation route already makes), builds a real
//! [`LiveBackupConsole`] whenever the target is running, and journals the
//! real outcome (`succeed`/`cancel`/`fail`) once `create_backup` returns.
//! Automatic retention is unchanged: `create_backup`'s own
//! `auto_prune_max_count` parameter already prunes before creating
//! whenever `is_automatic` is true (P6.16's ordering), which this
//! function simply forwards.
//!
//! `msc_application::backups::scheduled_tick` itself is untouched and
//! still exercised directly by
//! `crates/msc-application/tests/backup_retention.rs` — it remains a
//! valid, fixture-tested description of the timer-fired *policy*
//! (skip-when-not-running, skip-when-no-players); this module only stops
//! the real scheduler from reaching production backup creation through
//! it, since that path could never carry a live console or real
//! exclusivity.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use msc_application::backups::{self, BackupConsole, BackupError};
use msc_application::operations::LifecycleOperationError;
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::operation::OperationId;
use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::world_store;
use uuid::Uuid;

use crate::routes::lifecycle::{BackupBoundary, LifecycleRoutesState};

/// Reads the Java world's configured folder name at the agent boundary.
/// Bedrock keeps its distinct fixed backup-root rule, so its callers pass
/// no Java level name into the shared application services.
pub(crate) fn configured_java_level_name(
    server_type: msc_domain::identity::ServerType,
    server_dir: &Path,
) -> Option<String> {
    if server_type == msc_domain::identity::ServerType::Java {
        msc_application::worlds::read_java_level_name(&StdFileSystem, server_dir)
    } else {
        None
    }
}

/// `createBackup`'s manual button and `startAutoBackupTimer`'s fired
/// closure, unified — see this module's own doc for why one function now
/// covers both.
///
/// - `running`: the caller's already-known liveness of `server` (mirrors
///   every other guard this codebase resolves in the caller, e.g.
///   `backups::restore_backup`'s own `is_server_running` parameter).
///   `true` builds a real [`LiveBackupConsole`] and runs the flush/pause
///   protocol; `false` zips the live files directly, matching
///   `create_backup`'s own `console: None` contract.
/// - `is_automatic`/`auto_prune_max_count`: forwarded straight through to
///   `create_backup` — `false`/`None` for a manual backup (source has no
///   manual pruning), `true`/`Some(max_count)` for a scheduled one
///   (P6.16/P6.17's existing ordering: prune before creating).
pub fn start_backup(
    lifecycle: &LifecycleRoutesState,
    server: ConfigServer,
    running: bool,
    is_automatic: bool,
    auto_prune_max_count: Option<i64>,
) -> Result<OperationId, LifecycleOperationError> {
    let (operation_type, status_line) = if is_automatic {
        ("backup-scheduled", "Creating scheduled backup.")
    } else {
        ("backup-now", "Creating backup.")
    };
    let operation_id = lifecycle.operations().begin_lifecycle(
        operation_type,
        Some(server.id.clone()),
        status_line,
    )?;

    let server_dir = Path::new(&server.server_dir).to_path_buf();
    let server_type = server.server_type;
    let raw_level_name = configured_java_level_name(server_type, &server_dir);
    let server_id = server.id.clone();
    let server_name = server.display_name.clone();
    let task_lifecycle = lifecycle.clone();
    let task_operation_id = operation_id.clone();
    let should_cancel = lifecycle.operations().cancellation_check(&operation_id);

    tokio::spawn(async move {
        let now = iso8601_now();
        let backup_lifecycle = task_lifecycle.clone();
        let result = tokio::task::spawn_blocking(move || {
            let slots = world_store::load_slots(&StdFileSystem, &server_dir);
            let marker = world_store::load_explicit_active_slot_id(&StdFileSystem, &server_dir);
            let active_id = msc_domain::world::resolve_active_slot_id(&slots, marker.as_deref());
            let association = msc_domain::world::effective_backup_association(
                &slots,
                active_id.as_deref(),
                None,
                None,
            );
            let console: Option<LiveBackupConsole> = if running {
                Some(LiveBackupConsole::new(backup_lifecycle.clone()))
            } else {
                None
            };
            backups::create_backup(
                &StdFileSystem,
                &server_dir,
                server_type,
                raw_level_name.as_deref(),
                &association,
                Some(&server_id),
                Some(&server_name),
                is_automatic,
                true,
                None,
                auto_prune_max_count,
                &now,
                console.as_ref().map(|c| c as &dyn BackupConsole),
                || backup_lifecycle.status_snapshot().running,
                should_cancel,
            )
        })
        .await;
        match result {
            Ok(Ok(_)) => {
                let mut result = BTreeMap::new();
                result.insert("result".to_string(), "backup_created".to_string());
                let _ = task_lifecycle.operations().succeed(
                    &task_operation_id,
                    "Backup complete.",
                    result,
                );
            }
            Ok(Err(BackupError::Cancelled)) => {
                let _ = task_lifecycle
                    .operations()
                    .cancel(&task_operation_id, "Backup cancelled.");
            }
            Ok(Err(error)) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "backup_error",
                    error.to_string(),
                );
            }
            Err(_) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "internal_error",
                    "Backup task panicked.".to_string(),
                );
            }
        }
    });

    Ok(operation_id)
}

// =====================================================================
// Production `BackupConsole` — wires `send`/`wait_for_line` to
// `LifecycleService::send_command` and a real console-line wait. Moved
// here from `routes/backups.rs` (P6.21 originally built it there, for
// the manual route only) since [`start_backup`] is now the only caller,
// shared by both triggers.
// =====================================================================

const MAP_SNAPSHOT_MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAP_SNAPSHOT_COPY_LIMIT: Duration = Duration::from_secs(30);
const MAP_SNAPSHOT_MAX_DEPTH: usize = 32;

pub(crate) struct WorldMapSnapshot {
    pub(crate) path: PathBuf,
    pub(crate) bytes: u64,
    pub(crate) hold_millis: u128,
}

struct ResumeHeldSave<'a> {
    console: &'a LiveBackupConsole,
    boundary: BackupBoundary,
    command: &'static str,
    active: bool,
}

impl ResumeHeldSave<'_> {
    fn resume(&mut self) -> bool {
        if !self.console.lifecycle.backup_run_matches(&self.boundary) {
            return false;
        }
        let sent = self.console.send(self.command);
        if sent {
            self.active = false;
        }
        sent
    }
}

impl Drop for ResumeHeldSave<'_> {
    fn drop(&mut self) {
        // A stopped or replaced server must not receive the prior run's resume.
        if self.active {
            let _ = self.resume();
        }
    }
}

fn copy_snapshot_tree(
    source: &Path,
    destination: &Path,
    bytes: &mut u64,
    deadline: Instant,
    should_cancel: &impl Fn() -> bool,
    depth: usize,
) -> io::Result<()> {
    if depth > MAP_SNAPSHOT_MAX_DEPTH {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "world directory exceeds snapshot depth limit",
        ));
    }
    if should_cancel() || Instant::now() >= deadline {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "snapshot copy stopped",
        ));
    }
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "world contains a symlink",
        ));
    }
    if metadata.is_dir() {
        fs::create_dir(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_snapshot_tree(
                &entry.path(),
                &destination.join(entry.file_name()),
                bytes,
                deadline,
                should_cancel,
                depth + 1,
            )?;
        }
    } else if metadata.is_file() {
        let mut input = File::open(source)?;
        let mut output = File::create(destination)?;
        // A stack buffer here is reserved by every recursive directory frame.
        let mut buffer = vec![0u8; 1024 * 1024];
        loop {
            if should_cancel() || Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "snapshot copy stopped",
                ));
            }
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            *bytes += count as u64;
            if *bytes > MAP_SNAPSHOT_MAX_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::FileTooLarge,
                    "snapshot exceeds 2 GiB proof limit",
                ));
            }
            output.write_all(&buffer[..count])?;
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "world contains a special file",
        ));
    }
    Ok(())
}

/// Captures one proof-only world copy while the same MSC-managed BDS run has
/// confirmed that its files are ready. The normal backup path intentionally
/// retains its MSC 1 best-effort semantics; map publication requires a
/// stronger readiness gate and never consumes backup retention slots.
pub(crate) fn snapshot_bedrock_world(
    lifecycle: LifecycleRoutesState,
    server_dir: &Path,
    should_cancel: impl Fn() -> bool,
) -> Result<WorldMapSnapshot, String> {
    let configured =
        msc_application::worlds::read_configured_level_name(&StdFileSystem, server_dir)
            .ok_or("BDS server.properties has no level-name")?;
    if configured.is_empty()
        || configured == "."
        || configured == ".."
        || configured.contains('/')
        || configured.contains('\\')
    {
        return Err("BDS level-name is not a single safe folder name".to_string());
    }
    let world = server_dir.join("worlds").join(&configured);
    if !world.join("level.dat").is_file() || !world.join("db").is_dir() {
        return Err("configured BDS world has no level.dat or db directory".to_string());
    }
    let destination = std::env::temp_dir().join(format!("msc-world-map-proof-{}", Uuid::new_v4()));
    fs::create_dir(&destination).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    let started = Instant::now();
    let console = LiveBackupConsole::new(lifecycle);
    let boundary = match console.lifecycle.send_backup_command("save hold") {
        Some(boundary) => boundary,
        None => {
            let _ = fs::remove_dir_all(&destination);
            return Err("could not send save hold to the active BDS run".to_string());
        }
    };
    let mut held = ResumeHeldSave {
        console: &console,
        boundary,
        command: "save resume",
        active: true,
    };
    let (ready, _) = backups::wait_for_bedrock_save_ready(&console);
    if !ready {
        let _ = fs::remove_dir_all(&destination);
        let resumed = held.resume();
        return Err(format!(
            "BDS did not confirm ready to be copied; save resume dispatched: {resumed}"
        ));
    }
    if should_cancel() {
        let _ = fs::remove_dir_all(&destination);
        let resumed = held.resume();
        return Err(format!(
            "snapshot cancelled; save resume dispatched: {resumed}"
        ));
    }
    let mut bytes = 0;
    let copied = copy_snapshot_tree(
        &world,
        &destination.join("world"),
        &mut bytes,
        Instant::now() + MAP_SNAPSHOT_COPY_LIMIT,
        &should_cancel,
        0,
    );
    let hold_millis = started.elapsed().as_millis();
    let resume_sent = held.resume();
    drop(held);
    if let Err(error) = copied {
        let _ = fs::remove_dir_all(&destination);
        return Err(format!(
            "BDS snapshot copy failed: {error}; save resume dispatched: {resume_sent}"
        ));
    }
    if !resume_sent {
        let _ = fs::remove_dir_all(&destination);
        return Err(
            "snapshot copied, but save resume was not dispatched to the same BDS run".to_string(),
        );
    }
    Ok(WorldMapSnapshot {
        path: destination.join("world"),
        bytes,
        hold_millis,
    })
}

pub(crate) fn snapshot_stopped_bedrock_world(
    server_dir: &Path,
) -> Result<WorldMapSnapshot, String> {
    let configured =
        msc_application::worlds::read_configured_level_name(&StdFileSystem, server_dir)
            .ok_or("BDS server.properties has no level-name")?;
    if configured.is_empty()
        || configured == "."
        || configured == ".."
        || configured.contains('/')
        || configured.contains('\\')
    {
        return Err("BDS level-name is not a single safe folder name".to_string());
    }
    let world = server_dir.join("worlds").join(configured);
    if !world.join("level.dat").is_file() || !world.join("db").is_dir() {
        return Err("configured BDS world has no level.dat or db directory".to_string());
    }
    let destination = std::env::temp_dir().join(format!("msc-world-map-{}", Uuid::new_v4()));
    fs::create_dir(&destination).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    let mut bytes = 0;
    if let Err(error) = copy_snapshot_tree(
        &world,
        &destination.join("world"),
        &mut bytes,
        Instant::now() + MAP_SNAPSHOT_COPY_LIMIT,
        &|| false,
        0,
    ) {
        let _ = fs::remove_dir_all(&destination);
        return Err(format!("BDS stopped-world map copy failed: {error}"));
    }
    Ok(WorldMapSnapshot {
        path: destination.join("world"),
        bytes,
        hold_millis: 0,
    })
}

/// Captures a Java world only after a forced disk flush has been confirmed
/// behind save-off on the same MSC-managed run. A missing acknowledgement
/// fails closed; the guard still sends save-on before the operation ends.
pub(crate) fn snapshot_java_world(
    lifecycle: LifecycleRoutesState,
    server_dir: &Path,
    should_cancel: impl Fn() -> bool,
) -> Result<WorldMapSnapshot, String> {
    let configured = msc_application::worlds::read_java_level_name(&StdFileSystem, server_dir)
        .ok_or("Java server.properties has no level-name")?;
    if configured.is_empty()
        || configured == "."
        || configured == ".."
        || configured.contains('/')
        || configured.contains('\\')
    {
        return Err("Java level-name is not a single safe folder name".to_string());
    }
    let world = server_dir.join(&configured);
    if !world.join("level.dat").is_file() {
        return Err("configured Java world has no level.dat".to_string());
    }
    let destination = std::env::temp_dir().join(format!("msc-world-map-proof-{}", Uuid::new_v4()));
    fs::create_dir(&destination).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    let started = Instant::now();
    let console = LiveBackupConsole::new(lifecycle);
    let boundary = match console.lifecycle.send_backup_command("save-off") {
        Some(boundary) => boundary,
        None => {
            let _ = fs::remove_dir_all(&destination);
            return Err("could not send save-off to the active Java run".to_string());
        }
    };
    let mut held = ResumeHeldSave {
        console: &console,
        boundary,
        command: "save-on",
        active: true,
    };
    if !console.send("save-all flush") {
        let resumed = held.resume();
        let _ = fs::remove_dir_all(&destination);
        return Err(format!(
            "could not send Java save-all flush; save-on dispatched: {resumed}"
        ));
    }
    let saved = console.wait_for_line(&|line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("saved the game") || lower.contains("saved the world")
    });
    if !saved || should_cancel() {
        let resumed = held.resume();
        let _ = fs::remove_dir_all(&destination);
        return Err(format!(
            "Java flush was not confirmed or snapshot cancelled; save-on dispatched: {resumed}"
        ));
    }
    let mut bytes = 0;
    let copied = copy_snapshot_tree(
        &world,
        &destination.join("world"),
        &mut bytes,
        Instant::now() + MAP_SNAPSHOT_COPY_LIMIT,
        &should_cancel,
        0,
    );
    let hold_millis = started.elapsed().as_millis();
    let resume_sent = held.resume();
    drop(held);
    if let Err(error) = copied {
        let _ = fs::remove_dir_all(&destination);
        return Err(format!(
            "Java snapshot copy failed: {error}; save-on dispatched: {resume_sent}"
        ));
    }
    if !resume_sent {
        let _ = fs::remove_dir_all(&destination);
        return Err(
            "snapshot copied, but save-on was not dispatched to the same Java run".to_string(),
        );
    }
    Ok(WorldMapSnapshot {
        path: destination.join("world"),
        bytes,
        hold_millis,
    })
}

struct LiveBackupConsole {
    lifecycle: LifecycleRoutesState,
    deadline: Instant,
    confirmation_boundary: Mutex<Option<BackupBoundary>>,
}

impl LiveBackupConsole {
    /// `waitForConsoleLine(timeout:matching:)`'s own ~10s budget at
    /// every source call site.
    const BUDGET: std::time::Duration = std::time::Duration::from_secs(10);

    fn new(lifecycle: LifecycleRoutesState) -> Self {
        Self {
            lifecycle,
            deadline: Instant::now() + Self::BUDGET,
            confirmation_boundary: Mutex::new(None),
        }
    }
}

impl BackupConsole for LiveBackupConsole {
    fn send(&self, command: &str) -> bool {
        let Some(boundary) = self.lifecycle.send_backup_command(command) else {
            return false;
        };
        let mut confirmation = self.confirmation_boundary.lock().unwrap();
        if matches!(command, "save-all flush" | "save hold" | "save query")
            || command == "save-off" && confirmation.is_none()
        {
            *confirmation = Some(boundary);
        }
        true
    }

    fn wait_for_line(&self, matches: &dyn Fn(&str) -> bool) -> bool {
        let Some(boundary) = self.confirmation_boundary.lock().unwrap().clone() else {
            return false;
        };
        let start = Instant::now();
        loop {
            let Some(lines) = self.lifecycle.backup_lines_after(&boundary) else {
                return false;
            };
            if lines.iter().any(|line| {
                matches!(line.source.as_str(), "stdout" | "stderr" | "bedrock")
                    && matches(&line.text)
            }) {
                return true;
            }
            if self.deadline_reached() || start.elapsed() >= Self::BUDGET {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    fn deadline_reached(&self) -> bool {
        Instant::now() >= self.deadline
    }
}

fn iso8601_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = duration.as_secs() as i64;
    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}
