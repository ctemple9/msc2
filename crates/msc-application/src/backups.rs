//! Backup application layer.
//!
//! P6.15's half: thin pass-throughs over `msc_infrastructure::backup_store`
//! so every backup listing/deletion/pruning route or CLI caller reaches
//! its filesystem operation through one module — the same precedent
//! `msc_application::worlds::set_slot_thumbnail` already set for a slot
//! mutation with no additional orchestration guard of its own — plus the
//! one caller-facing policy that step's own scope added: the auto-backup
//! max-count clamp.
//!
//! Two of P6.15's planned fixtures needed no new production code at all,
//! since earlier phases already built what they characterize:
//! `auto-backup-interval-minutes-defaults-to-30-when-config-field-absent`
//! is `app_config_schema.rs`'s existing `opt_i64(v,
//! "auto_backup_interval_minutes", 30)` decode default (already tested in
//! `crates/msc-domain/tests/app_config_schema.rs`); `effective_backup_association`
//! (the active-slot-association rule) is `msc_domain::world`'s, ported
//! ahead of that step for P6.12's own use. `backup_inventory.rs`'s tests
//! cite both rather than re-proving them.
//!
//! P6.16's half (below): [`create_backup`], the one authoritative backup
//! path, and the flush-consistent save-pause protocol
//! ([`BackupConsole`]/[`pause_saves_for_backup`]/
//! [`resume_saves_after_backup`]) it runs when the target is live.

use msc_domain::backup::{self as domain_backup, BackupMeta};
use msc_domain::identity::ServerType;
use msc_domain::world::{self, BackupAssociation};
use msc_infrastructure::archive::{self, ArchiveError};
use msc_infrastructure::backup_store::{self, BackupEntry};
use msc_infrastructure::fs::FileSystem;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// `loadBackupsForSelectedServer`'s data half (source
/// `AppViewModel+Backups.swift:24-98`) — the UI-only size-formatting and
/// background-thread directory-size computation stay with whichever
/// client renders them.
pub fn list_backups(fs: &dyn FileSystem, server_dir: &Path) -> Vec<BackupEntry> {
    backup_store::list_backups(fs, server_dir)
}

/// `deleteBackup(_:)` (source line 704-721): unconditional — MSC 1 has no
/// "don't delete the last backup" guard on a manual, single-backup
/// delete; that floor applies only to automatic pruning
/// ([`prune_backups`]'s own correction), matching source exactly.
pub fn delete_backup(fs: &dyn FileSystem, zip_path: &Path) -> io::Result<()> {
    backup_store::delete_backup(fs, zip_path)
}

/// `pruneAutoBackupsIfNeeded(in:maxCount:)` (source line 528-581), with
/// the retention floor already applied inside
/// `backup_store::prune_managed_backups` — see that function's own doc.
pub fn prune_backups(fs: &dyn FileSystem, server_dir: &Path, max_count: i64) -> Vec<PathBuf> {
    backup_store::prune_managed_backups(fs, server_dir, max_count)
}

/// `Stepper("", value: $autoBackupMaxCountLocal, in: 3...50)`
/// (`ServerEditorBackupsTab.swift:47`) — MSC 1 enforces this bound only
/// in the SwiftUI control itself; the model layer
/// (`setAutoBackupMaxCount`, `AppViewModel+ServerControls.swift:820-825`)
/// applies no clamp at all
/// (`fixtures/backups/auto-backup-max-count-editor-clamps-to-3-through-50.json`'s
/// own notes: "the model layer performs no validation or clamping").
/// MSC 2 has no editor control of its own yet (P6.20+ builds the routes;
/// a client builds the control), so this gives the 3...50 bound an
/// application-layer home a future settings route/CLI command can call
/// before persisting, rather than leaving it unenforced anywhere in the
/// port — a deliberate strengthening over source, not oracle parity.
pub fn clamp_auto_backup_max_count(requested: i64) -> i64 {
    requested.clamp(3, 50)
}

// =====================================================================
// P6.16 — create and verify offline and running-server backups
//
// One authoritative path, `create_backup`, unifying every MSC 1 trigger
// that produces a backup ZIP: `createBackup(for:isAutomatic:slotId:
// slotName:triggerReason:)` (manual button, the auto-backup timer, the
// stop-time trigger, and `restoreBackup`'s own "pre-restore" safety
// backup — all four go through this one Swift function already) *and*
// `backupWorld(for:)` (`replaceWorld`'s pre-replace safety backup — a
// second, separate Swift function with its own untokened filename shape
// and no save-pause step at all, since `replaceWorld`'s own running-
// server guard already means there's nothing to pause). This port folds
// both into one function, distinguished by the `tokened`/`console`
// parameters rather than by which function a caller happened to call —
// `fixtures/backups/pre-replace-backup-has-no-token-and-is-excluded-from-pruning.json`
// pins the untokened, unprunable filename shape either way produces.
//
// The flush-consistent save-pause protocol
// (`pauseSavesForBackup`/`resumeSavesAfterBackup`/
// `waitForBedrockSaveReady`, source lines 315-448) sits behind
// [`BackupConsole`], a fakeable runtime port per this step's own scope
// note ("Keep the Bedrock hold/query/resume protocol behind the same
// fakeable runtime port but unavailable in production until Phase 10").
// A real implementation — wiring `send`/`wait_for_line` to
// `LifecycleService::send_command` and an actual console-line wait with
// real wall-clock timing — is P6.21's job (route/agent wiring), the same
// deferred-wiring shape `worlds::activate_slot`'s own `backup: impl
// FnOnce() -> bool` closure parameter already established; nothing here
// depends on that wiring existing yet, since every fixture this step
// characterizes is exercised through [`FakeBackupConsole`].
// =====================================================================

#[derive(Debug)]
pub enum BackupError {
    /// `worldFolderNames(for:)` found nothing to archive — the same
    /// guard `worlds::create_slot_from_current_world` shares.
    NoWorldFolders,
    Io(io::Error),
    Archive(ArchiveError),
    /// The just-written archive failed the post-creation structural
    /// check, or is missing an entry for one of the folders it was
    /// supposed to capture — this step's own "verify before publishing"
    /// correction (see the module-level P6.16 doc), with no Swift
    /// counterpart: MSC 1 treats a zero `zip` exit status as sufficient.
    VerificationFailed,
}

impl fmt::Display for BackupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackupError::NoWorldFolders => write!(f, "no world folders found to back up"),
            BackupError::Io(e) => write!(f, "{e}"),
            BackupError::Archive(e) => write!(f, "{e}"),
            BackupError::VerificationFailed => {
                write!(f, "backup archive failed post-creation verification")
            }
        }
    }
}

impl std::error::Error for BackupError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupCreationResult {
    pub zip_path: PathBuf,
    pub trigger_reason: String,
    /// `writeBackupMeta`'s failure is logged-and-swallowed in source
    /// (`fixtures/backup-online-consistency/sidecar-meta-write-failure-logged-as-warning-backup-still-succeeds.json`)
    /// — the backup ZIP is the important artifact, so this field reports
    /// the sidecar outcome without making it fatal to the result.
    pub sidecar_written: bool,
}

/// `createBackup(for:isAutomatic:slotId:slotName:triggerReason:)` /
/// `backupWorld(for:)`, unified — see the section doc above for why one
/// function covers both Swift entry points.
///
/// - `association`: the caller's already-resolved
///   [`world::effective_backup_association`] result — slot lookup is
///   this function's caller's job, not this one's, keeping backup
///   creation decoupled from world-slot storage.
/// - `is_automatic`: selects [`domain_backup::AUTO_TOKEN`] vs
///   [`domain_backup::MANUAL_TOKEN`] when `tokened`, the default
///   trigger-reason fallback, and whether `auto_prune_max_count` is
///   consulted at all.
/// - `tokened`: `false` reproduces `backupWorld`'s untokened
///   `<base>-<ts>.zip` shape (pre-replace only — the caller must also
///   pass an explicit `trigger_reason` in that case, since there is no
///   sensible token-less default). `true` is every other trigger.
/// - `auto_prune_max_count`: `Some(max_count)` runs
///   [`backup_store::prune_managed_backups`] *before* this backup is
///   created (source's own ordering, `createBackup` line 223-226) —
///   only ever meaningful (and only ever consulted) when `is_automatic`.
/// - `console`: `None` when the target isn't the actively running
///   server (source's own `targetIsRunning` gate) — the world folders
///   are zipped live, no pause attempted. `Some(_)` runs
///   [`pause_saves_for_backup`] first.
/// - `still_running_at_resume`: evaluated once, only if a pause actually
///   happened, immediately before [`resume_saves_after_backup`] —
///   `resumeSavesAfterBackup`'s own guard re-checks liveness at *that*
///   point in time, not the `targetIsRunning` snapshot captured before
///   the zip started
///   (`fixtures/backup-online-consistency/resume-skipped-when-server-stopped-before-resume-runs.json`),
///   so this is a lazily-evaluated closure rather than a plain `bool`,
///   the same shape `worlds::activate_slot`'s `backup` parameter uses
///   for the same reason (a caller-observed condition this function
///   can't itself know without asking).
#[allow(clippy::too_many_arguments)]
pub fn create_backup(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    association: &BackupAssociation,
    server_id: Option<&str>,
    server_display_name: Option<&str>,
    is_automatic: bool,
    tokened: bool,
    trigger_reason: Option<&str>,
    auto_prune_max_count: Option<i64>,
    now: &str,
    console: Option<&dyn BackupConsole>,
    still_running_at_resume: impl FnOnce() -> bool,
) -> Result<BackupCreationResult, BackupError> {
    let level_name = world::current_level_name(server_type, raw_level_name);
    let folders = crate::worlds::existing_world_folders(fs, server_dir, server_type, &level_name);
    if folders.is_empty() {
        return Err(BackupError::NoWorldFolders);
    }

    let backups_dir = backup_store::backups_dir(server_dir);
    fs.create_dir_all(&backups_dir).map_err(BackupError::Io)?;

    if is_automatic && let Some(max_count) = auto_prune_max_count {
        let _ = backup_store::prune_managed_backups(fs, server_dir, max_count);
    }

    let archive_base_name = match server_type {
        ServerType::Bedrock => "worlds".to_string(),
        ServerType::Java => level_name,
    };
    let ts = domain_backup::filename_timestamp_from_iso8601(now);
    let filename = if tokened {
        format!(
            "{archive_base_name}{}{ts}.zip",
            domain_backup::creation_token(is_automatic)
        )
    } else {
        format!("{archive_base_name}-{ts}.zip")
    };
    let zip_path = backups_dir.join(filename);

    let is_bedrock = server_type == ServerType::Bedrock;
    let saves_paused = match console {
        Some(console) => pause_saves_for_backup(console, is_bedrock),
        None => false,
    };

    let zip_result = archive::create_zip_from_folders(&zip_path, server_dir, &folders);

    if saves_paused {
        let console = console.expect("saves_paused is only true when console was Some");
        resume_saves_after_backup(console, is_bedrock, still_running_at_resume());
    }

    zip_result.map_err(BackupError::Archive)?;

    if archive::validate_archive_safety(&zip_path).is_err()
        || !archive_contains_every_folder(&zip_path, &folders)
    {
        return Err(BackupError::VerificationFailed);
    }

    let reason = trigger_reason
        .map(str::to_string)
        .unwrap_or_else(|| domain_backup::default_trigger_reason(is_automatic).to_string());
    let meta = BackupMeta {
        server_id: server_id.map(str::to_string),
        server_display_name: server_display_name.map(str::to_string),
        slot_id: association.slot_id.clone(),
        slot_name: association.slot_name.clone(),
        world_seed: association.world_seed.clone(),
        trigger_reason: reason.clone(),
    };
    let sidecar_written = backup_store::write_sidecar(fs, &zip_path, &meta).is_ok();

    Ok(BackupCreationResult {
        zip_path,
        trigger_reason: reason,
        sidecar_written,
    })
}

/// This step's own post-creation verification (see [`BackupError::VerificationFailed`]):
/// every captured folder name must appear as a real entry (or entry
/// prefix) in the finished archive's own listing — not just "the zip
/// opens," but "the zip actually contains what we meant to back up."
fn archive_contains_every_folder(zip_path: &Path, folders: &[String]) -> bool {
    let Ok(names) = archive::list_entry_names(zip_path) else {
        return false;
    };
    folders.iter().all(|folder| {
        let prefix = format!("{folder}/");
        names.iter().any(|name| name.starts_with(&prefix))
    })
}

/// The flush-consistent save-pause protocol's runtime boundary —
/// `sendBackupCommand(_:)`/`waitForConsoleLine(timeout:matching:)`. See
/// the section doc above for why a real production implementation isn't
/// built in this crate yet.
pub trait BackupConsole {
    /// `sendBackupCommand(_:)`: sends `command` to the running server's
    /// console, returning whether the send itself succeeded.
    fn send(&self, command: &str) -> bool;
    /// Arms one fresh wait for a console line satisfying `matches`, up
    /// to whatever wall-clock budget the implementation enforces.
    /// Returns whether a match arrived before that budget elapsed. A
    /// caller polling in a loop (Bedrock's `save query`) calls this once
    /// per iteration, checking [`Self::deadline_reached`] between calls
    /// — see [`wait_for_bedrock_save_ready`].
    fn wait_for_line(&self, matches: &dyn Fn(&str) -> bool) -> bool;
    /// Whether this port's own overall ~10s budget (`waitForConsoleLine`'s
    /// `timeout: 10` at every one of source's call sites) has already
    /// elapsed. A fake can answer this without any real clock at all —
    /// see [`FakeBackupConsole`].
    fn deadline_reached(&self) -> bool;
}

/// `pauseSavesForBackup(isBedrock:)` (source line 329-364): sends the
/// pause command(s), waits (best-effort — a timeout is not a failure)
/// for confirmation, and reports whether a pause was actually
/// negotiated (so the caller knows whether [`resume_saves_after_backup`]
/// has anything to undo).
pub fn pause_saves_for_backup(console: &dyn BackupConsole, is_bedrock: bool) -> bool {
    if is_bedrock {
        if !console.send("save hold") {
            return false;
        }
        let _ = wait_for_bedrock_save_ready(console);
        true
    } else {
        let flush_sent = console.send("save-all flush");
        let off_sent = console.send("save-off");
        if !(flush_sent || off_sent) {
            return false;
        }
        let _ = wait_for_java_save_confirmation(console);
        off_sent
    }
}

/// `resumeSavesAfterBackup(for:isBedrock:)` (source line 370-388):
/// unconditional once `still_running` — no memory of whether the pause's
/// own confirmation wait ever succeeded.
pub fn resume_saves_after_backup(
    console: &dyn BackupConsole,
    is_bedrock: bool,
    still_running: bool,
) {
    if !still_running {
        return;
    }
    let _ = console.send(if is_bedrock { "save resume" } else { "save-on" });
}

/// `pauseSavesForBackup`'s Java branch's `waitForConsoleLine` call
/// (source line 351-354): matches "Saved the game" or "Saved the world",
/// case-insensitively.
pub fn wait_for_java_save_confirmation(console: &dyn BackupConsole) -> bool {
    console.wait_for_line(&|line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("saved the game") || lower.contains("saved the world")
    })
}

/// `waitForBedrockSaveReady(timeout:)` (source line 394-407): re-sends
/// `save query` and re-arms a fresh wait each iteration until either a
/// "ready to be copied" line arrives or the port's own deadline is
/// reached. Returns whether readiness was seen, and how many polls were
/// sent — the latter only for this step's own characterization (source
/// itself only surfaces the `Bool`).
pub fn wait_for_bedrock_save_ready(console: &dyn BackupConsole) -> (bool, u32) {
    let mut polls: u32 = 0;
    loop {
        console.send("save query");
        polls += 1;
        let ready =
            console.wait_for_line(&|line| line.to_ascii_lowercase().contains("ready to be copied"));
        if ready {
            return (true, polls);
        }
        if console.deadline_reached() {
            return (false, polls);
        }
    }
}
