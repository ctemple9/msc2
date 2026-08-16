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
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use msc_application::backups::{self, BackupConsole, BackupError};
use msc_application::operations::LifecycleOperationError;
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::operation::OperationId;
use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::world_store;

use crate::routes::lifecycle::LifecycleRoutesState;

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
                None,
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

struct LiveBackupConsole {
    lifecycle: LifecycleRoutesState,
    deadline: Instant,
}

impl LiveBackupConsole {
    /// `waitForConsoleLine(timeout:matching:)`'s own ~10s budget at
    /// every source call site.
    const BUDGET: std::time::Duration = std::time::Duration::from_secs(10);

    fn new(lifecycle: LifecycleRoutesState) -> Self {
        Self {
            lifecycle,
            deadline: Instant::now() + Self::BUDGET,
        }
    }
}

impl BackupConsole for LiveBackupConsole {
    fn send(&self, command: &str) -> bool {
        self.lifecycle.send_command(command).is_ok()
    }

    fn wait_for_line(&self, matches: &dyn Fn(&str) -> bool) -> bool {
        let start = Instant::now();
        loop {
            let lines = self.lifecycle.recent_console_lines(50);
            if lines.iter().any(|line| matches(&line.text)) {
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
