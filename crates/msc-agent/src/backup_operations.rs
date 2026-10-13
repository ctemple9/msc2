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

const MAP_SNAPSHOT_MAX_BYTES: u64 = 1024 * 1024 * 1024 * 1024;
const MAP_SNAPSHOT_LIVE_HOLD_LIMIT: Duration = Duration::from_secs(30);
const MAP_SNAPSHOT_STOPPED_COPY_LIMIT: Duration = Duration::from_secs(30 * 60);
const MAP_SNAPSHOT_MAX_DEPTH: usize = 32;

pub(crate) struct WorldMapSnapshot {
    pub(crate) staging: Option<crate::map_staging::Staging>,
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
    copy_snapshot_tree_with_budget(
        source,
        destination,
        bytes,
        deadline,
        should_cancel,
        depth,
        &|path| {
            msc_infrastructure::conversion_resources::require_space(
                path,
                0,
                "copying a map snapshot",
            )
        },
    )
}

fn copy_snapshot_tree_with_budget(
    source: &Path,
    destination: &Path,
    bytes: &mut u64,
    deadline: Instant,
    should_cancel: &impl Fn() -> bool,
    depth: usize,
    check_space: &impl Fn(&Path) -> io::Result<()>,
) -> io::Result<()> {
    if depth > MAP_SNAPSHOT_MAX_DEPTH {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "world directory exceeds snapshot depth limit",
        ));
    }
    if should_cancel() {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "snapshot cancelled",
        ));
    }
    if Instant::now() >= deadline {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "snapshot copy deadline reached",
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
            copy_snapshot_tree_with_budget(
                &entry.path(),
                &destination.join(entry.file_name()),
                bytes,
                deadline,
                should_cancel,
                depth + 1,
                check_space,
            )?;
        }
    } else if metadata.is_file() {
        let total = bytes.checked_add(metadata.len()).ok_or_else(|| {
            io::Error::new(io::ErrorKind::FileTooLarge, "snapshot byte count overflow")
        })?;
        if total > MAP_SNAPSHOT_MAX_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "snapshot exceeds 1 TiB safety limit",
            ));
        }
        check_space(destination.parent().unwrap())?;
        if clone_snapshot_file(source, destination)? {
            *bytes = total;
            if should_cancel() {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "snapshot cancelled",
                ));
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "snapshot copy deadline reached",
                ));
            }
            return Ok(());
        }
        let mut input = File::open(source)?;
        let mut output = File::create(destination)?;
        let mut since_space_check = 0usize;
        // A stack buffer here is reserved by every recursive directory frame.
        let mut buffer = vec![0u8; 1024 * 1024];
        loop {
            if should_cancel() {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "snapshot cancelled",
                ));
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "snapshot copy deadline reached",
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
                    "snapshot exceeds 1 TiB safety limit",
                ));
            }
            output.write_all(&buffer[..count])?;
            since_space_check += count;
            if since_space_check >= 64 * 1024 * 1024 {
                check_space(destination.parent().unwrap())?;
                since_space_check = 0;
            }
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "world contains a special file",
        ));
    }
    Ok(())
}

/// APFS clones preserve a stable snapshot without copying all data while saves
/// are held. Other filesystems use the same bounded streaming fallback.
fn clone_snapshot_file(source: &Path, destination: &Path) -> io::Result<bool> {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let source = CString::new(source.as_os_str().as_bytes())?;
        let destination = CString::new(destination.as_os_str().as_bytes())?;
        // sys/clonefile.h: reject symlinks and retain the creating user's owner.
        let flags = 0x0001 | 0x0002;
        if unsafe { libc::clonefile(source.as_ptr(), destination.as_ptr(), flags) } == 0 {
            return Ok(true);
        }
        let error = io::Error::last_os_error();
        if !matches!(
            error.raw_os_error(),
            Some(libc::ENOTSUP | libc::EXDEV | libc::EINVAL | libc::ENOSYS)
        ) {
            return Err(error);
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (source, destination);
    Ok(false)
}

/// Captures a disk-budgeted world copy while the same MSC-managed BDS run has
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
    let estimate = crate::map_staging::estimate_tree(&world).map_err(|error| error.to_string())?;
    if estimate > MAP_SNAPSHOT_MAX_BYTES {
        return Err("snapshot exceeds 1 TiB safety limit".into());
    }
    let staging = crate::map_staging::Staging::create("snapshot", estimate)
        .map_err(|error| error.to_string())?;
    let destination = staging.path().to_path_buf();
    let started = Instant::now();
    let console = LiveBackupConsole::new(lifecycle);
    let boundary = match console.lifecycle.send_backup_command("save hold") {
        Some(boundary) => boundary,
        None => {
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
        let resumed = held.resume();
        return Err(format!(
            "BDS did not confirm ready to be copied; save resume dispatched: {resumed}"
        ));
    }
    if should_cancel() {
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
        started + MAP_SNAPSHOT_LIVE_HOLD_LIMIT,
        &|| should_cancel() || !console.lifecycle.backup_run_matches(&held.boundary),
        0,
    );
    let hold_millis = started.elapsed().as_millis();
    let resume_sent = held.resume();
    drop(held);
    if let Err(error) = copied {
        return Err(format!(
            "BDS snapshot copy failed at {}: {error}; save resume dispatched: {resume_sent}. A live map snapshot has a 30-second save-pause budget. Stop the server and refresh terrain to copy a large world without holding saves.",
            destination.display()
        ));
    }
    if !resume_sent {
        return Err(
            "snapshot copied, but save resume was not dispatched to the same BDS run".to_string(),
        );
    }
    Ok(WorldMapSnapshot {
        staging: Some(staging),
        path: destination.join("world"),
        bytes,
        hold_millis,
    })
}

pub(crate) fn snapshot_stopped_bedrock_world(
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
    let world = server_dir.join("worlds").join(configured);
    if !world.join("level.dat").is_file() || !world.join("db").is_dir() {
        return Err("configured BDS world has no level.dat or db directory".to_string());
    }
    let estimate = crate::map_staging::estimate_tree(&world).map_err(|error| error.to_string())?;
    if estimate > MAP_SNAPSHOT_MAX_BYTES {
        return Err("snapshot exceeds 1 TiB safety limit".into());
    }
    let staging = crate::map_staging::Staging::create("snapshot", estimate)
        .map_err(|error| error.to_string())?;
    let destination = staging.path().to_path_buf();
    let mut bytes = 0;
    if let Err(error) = copy_snapshot_tree(
        &world,
        &destination.join("world"),
        &mut bytes,
        Instant::now() + MAP_SNAPSHOT_STOPPED_COPY_LIMIT,
        &should_cancel,
        0,
    ) {
        return Err(format!(
            "BDS stopped-world map copy failed at {}: {error}",
            destination.display()
        ));
    }
    if should_cancel() {
        return Err("Stopped-world map snapshot cancelled because the agent is stopping or the server started.".into());
    }
    Ok(WorldMapSnapshot {
        staging: Some(staging),
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
    let estimate = crate::map_staging::estimate_tree(&world).map_err(|error| error.to_string())?;
    if estimate > MAP_SNAPSHOT_MAX_BYTES {
        return Err("snapshot exceeds 1 TiB safety limit".into());
    }
    let staging = crate::map_staging::Staging::create("snapshot", estimate)
        .map_err(|error| error.to_string())?;
    let destination = staging.path().to_path_buf();
    let started = Instant::now();
    let console = LiveBackupConsole::new(lifecycle);
    let boundary = match console.lifecycle.send_backup_command("save-off") {
        Some(boundary) => boundary,
        None => {
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
        return Err(format!(
            "Java flush was not confirmed or snapshot cancelled; save-on dispatched: {resumed}"
        ));
    }
    let mut bytes = 0;
    let copied = copy_snapshot_tree(
        &world,
        &destination.join("world"),
        &mut bytes,
        started + MAP_SNAPSHOT_LIVE_HOLD_LIMIT,
        &should_cancel,
        0,
    );
    let hold_millis = started.elapsed().as_millis();
    let resume_sent = held.resume();
    drop(held);
    if let Err(error) = copied {
        return Err(format!(
            "Java snapshot copy failed at {}: {error}; save-on dispatched: {resume_sent}",
            destination.display()
        ));
    }
    if !resume_sent {
        return Err(
            "snapshot copied, but save-on was not dispatched to the same Java run".to_string(),
        );
    }
    Ok(WorldMapSnapshot {
        staging: Some(staging),
        path: destination.join("world"),
        bytes,
        hold_millis,
    })
}

struct LiveBackupConsole {
    lifecycle: LifecycleRoutesState,
    deadline: Instant,
    confirmation_boundary: Mutex<Option<(BackupBoundary, Duration)>>,
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

    fn confirmation_budget(command: &str) -> Duration {
        // BDS answers queries before save preparation finishes. Leave time
        // for another query instead of spending the overall deadline on one.
        if command == "save query" {
            Duration::from_millis(500)
        } else {
            Self::BUDGET
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
            *confirmation = Some((boundary, Self::confirmation_budget(command)));
        }
        true
    }

    fn wait_for_line(&self, matches: &dyn Fn(&str) -> bool) -> bool {
        let Some((boundary, budget)) = self.confirmation_boundary.lock().unwrap().clone() else {
            return false;
        };
        wait_for_confirmation(
            self.deadline,
            budget,
            || {
                self.lifecycle.backup_lines_after(&boundary).map(|lines| {
                    lines.iter().any(|line| {
                        matches!(line.source.as_str(), "stdout" | "stderr" | "bedrock")
                            && matches(&line.text)
                    })
                })
            },
            Instant::now,
            std::thread::sleep,
        )
    }

    fn deadline_reached(&self) -> bool {
        Instant::now() >= self.deadline
    }
}

fn wait_for_confirmation(
    overall_deadline: Instant,
    budget: Duration,
    mut observe: impl FnMut() -> Option<bool>,
    mut now: impl FnMut() -> Instant,
    mut pause: impl FnMut(Duration),
) -> bool {
    let deadline = overall_deadline.min(now() + budget);
    loop {
        match observe() {
            Some(true) => return true,
            None => return false,
            Some(false) => {}
        }
        let remaining = deadline.saturating_duration_since(now());
        if remaining.is_zero() {
            return false;
        }
        pause(remaining.min(Duration::from_millis(100)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct SnapshotFixture(PathBuf);
    impl SnapshotFixture {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("msc2-snapshot-check-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(root.join("source/db")).unwrap();
            fs::write(root.join("source/level.dat"), b"world metadata").unwrap();
            fs::write(root.join("source/db/CURRENT"), b"database").unwrap();
            Self(root)
        }
    }
    impl Drop for SnapshotFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn map_snapshot_crosses_old_size_limit_and_is_independent() {
        let fixture = SnapshotFixture::new();
        // Model earlier copied files without allocating or writing 2 GiB.
        let mut bytes = 2 * 1024 * 1024 * 1024;
        copy_snapshot_tree_with_budget(
            &fixture.0.join("source"),
            &fixture.0.join("snapshot"),
            &mut bytes,
            Instant::now() + Duration::from_secs(60),
            &|| false,
            0,
            &|_| Ok(()),
        )
        .unwrap();
        assert!(bytes > 2 * 1024 * 1024 * 1024);
        fs::write(fixture.0.join("source/db/CURRENT"), b"new data").unwrap();
        assert_eq!(
            fs::read(fixture.0.join("snapshot/db/CURRENT")).unwrap(),
            b"database"
        );
    }

    #[test]
    fn map_snapshot_cancellation_and_expired_deadline_stop_copy() {
        let fixture = SnapshotFixture::new();
        let visits = Cell::new(0);
        let cancel = || {
            visits.set(visits.get() + 1);
            visits.get() >= 2
        };
        let error = copy_snapshot_tree_with_budget(
            &fixture.0.join("source"),
            &fixture.0.join("cancelled"),
            &mut 0,
            Instant::now() + Duration::from_secs(60),
            &cancel,
            0,
            &|_| Ok(()),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert!(!fixture.0.join("cancelled/db/CURRENT").exists());
        let error = copy_snapshot_tree_with_budget(
            &fixture.0.join("source"),
            &fixture.0.join("expired"),
            &mut 0,
            Instant::now() - Duration::from_secs(1),
            &|| false,
            0,
            &|_| Ok(()),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(!fixture.0.join("expired").exists());
        let error = copy_snapshot_tree_with_budget(
            &fixture.0.join("source"),
            &fixture.0.join("full"),
            &mut 0,
            Instant::now() + Duration::from_secs(60),
            &|| false,
            0,
            &|_| Err(io::Error::other("simulated disk shortage")),
        )
        .unwrap_err();
        assert!(error.to_string().contains("simulated disk shortage"));
        assert!(!fixture.0.join("full/db/CURRENT").exists());
    }

    #[test]
    fn bedrock_queries_retry_before_deadline_without_shortening_java_flush() {
        let start = Instant::now();
        let clock = Cell::new(start);
        let overall_deadline = start + LiveBackupConsole::BUDGET;
        let ready_at = start + Duration::from_millis(750);
        let wait = |command: &str, ready_at: Instant| {
            wait_for_confirmation(
                overall_deadline,
                LiveBackupConsole::confirmation_budget(command),
                || Some(clock.get() >= ready_at),
                || clock.get(),
                |duration| clock.set(clock.get() + duration),
            )
        };

        // The first BDS response is not ready; a subsequent query can succeed
        // inside the same overall deadline, including a fast macOS response.
        assert!(!wait("save query", ready_at));
        assert!(clock.get() < overall_deadline);
        assert!(wait("save query", ready_at));
        assert!(wait("save query", clock.get()));

        clock.set(start);
        assert!(wait("save-all flush", ready_at));

        clock.set(start);
        let never_ready = overall_deadline + Duration::from_secs(1);
        while clock.get() < overall_deadline {
            assert!(!wait("save query", never_ready));
        }
        assert_eq!(clock.get(), overall_deadline);
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
