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
//! P6.16's half: [`create_backup`], the one authoritative backup path,
//! and the flush-consistent save-pause protocol
//! ([`BackupConsole`]/[`pause_saves_for_backup`]/
//! [`resume_saves_after_backup`]) it runs when the target is live.
//!
//! P6.17's half: [`scheduled_tick`] (the timer-fired policy real pacing
//! lives outside this crate for) and [`prune_orphan_sidecars`].
//!
//! P6.18's half (below): [`restore_backup`]/[`reconcile_interrupted_restore`],
//! transactional for the same reason `worlds::activate_slot`'s own
//! section doc explains — reused directly, not rebuilt, since restoring
//! a backup and activating a slot both boil down to "swap the live world
//! folders for a different, already-verified archive's contents."

use msc_domain::backup::{self as domain_backup, BackupMeta};
use msc_domain::identity::ServerType;
use msc_domain::world::{self, BackupAssociation};
use msc_infrastructure::archive::{self, ArchiveError};
use msc_infrastructure::backup_store::{self, BackupEntry};
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::world_store;
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

// =====================================================================
// P6.17 — scheduled backups and known-good retention
//
// `startAutoBackupTimer(for:)`'s closure body (source lines 774-787),
// minus the `Timer` construction itself: real wall-clock pacing is
// `msc-agent`'s job (`BackupScheduler`, tokio-driven, tested in
// `crates/msc-agent/tests/backup_scheduler.rs`), kept out of this crate
// the same way this phase keeps every other real-runtime concern
// (process supervision, console waiting) behind a caller-supplied
// signal rather than owning the clock itself. [`scheduled_tick`] is the
// one tick's worth of *policy* the timer's closure runs once it fires:
// skip if the backend isn't running (source stops the timer in that
// branch — a scheduler-level concern, not this function's), skip if no
// players are online (source's own guard, timer keeps running), else
// fire [`create_backup`] with `is_automatic: true`.
//
// Retention's "prune only MSC-managed backups and paired orphan
// sidecars, never delete the final verified recovery point" is already
// `backup_store::prune_managed_backups`'s job (P6.15) plus this step's
// own `backup_store::prune_orphan_sidecars` addition — [`scheduled_tick`]
// runs the former via `create_backup`'s own `auto_prune_max_count`
// (unchanged from P6.16); orphan-sidecar sweeping is a separate,
// explicit call this step adds since no fixture ties it to backup
// *creation* specifically.
// =====================================================================

#[derive(Debug)]
pub enum ScheduledTickOutcome {
    /// `startAutoBackupTimer`'s own guard (source line 777): the backend
    /// isn't running. Source stops the timer here; deciding whether to
    /// do the equivalent (or just skip this one tick) is the caller's
    /// call — see [`crate::backups`]'s module doc and
    /// `msc-agent::backup_scheduler` for how the real scheduler handles
    /// it.
    SkippedNotRunning,
    /// `fixtures/backups/scheduled-auto-backup-skipped-when-no-players-online.json`:
    /// re-evaluated on every tick: a quiet server just keeps skipping.
    SkippedNoPlayers,
    Fired(Result<BackupCreationResult, BackupError>),
}

/// One scheduled-timer tick's worth of policy — see the section doc
/// above. `backend_running`/`online_player_count` are the caller's
/// already-known snapshot for this tick (this function does no polling
/// of its own); `auto_prune_max_count` is always consulted (matching
/// `is_automatic: true` always pruning-eligible), and the save-pause
/// `console` is always `None` — no production `BackupConsole`
/// implementation is wired into the real scheduler yet (P6.21), so a
/// scheduled backup zips live files directly until then, exactly the
/// fallback source itself takes when `sendBackupCommand` can't reach a
/// backend at all.
#[allow(clippy::too_many_arguments)]
pub fn scheduled_tick(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    association: &BackupAssociation,
    server_id: Option<&str>,
    server_display_name: Option<&str>,
    auto_prune_max_count: i64,
    now: &str,
    backend_running: bool,
    online_player_count: usize,
) -> ScheduledTickOutcome {
    if !backend_running {
        return ScheduledTickOutcome::SkippedNotRunning;
    }
    if online_player_count == 0 {
        return ScheduledTickOutcome::SkippedNoPlayers;
    }

    let result = create_backup(
        fs,
        server_dir,
        server_type,
        raw_level_name,
        association,
        server_id,
        server_display_name,
        true,
        true,
        None,
        Some(auto_prune_max_count),
        now,
        None,
        || false,
    );
    ScheduledTickOutcome::Fired(result)
}

/// `msc_infrastructure::backup_store::prune_orphan_sidecars`, reached
/// through this crate's single backup entry point (see this module's own
/// P6.15 doc note for why every other backup mutation already does the
/// same).
pub fn prune_orphan_sidecars(fs: &dyn FileSystem, server_dir: &Path) -> Vec<PathBuf> {
    backup_store::prune_orphan_sidecars(fs, server_dir)
}

// =====================================================================
// P6.18 — transactional backup restore and restart recovery
//
// Ports `restoreBackup(_:)` (source `AppViewModel+Backups.swift:585-699`)
// merged with two Phase 6 corrections `fixtures/backup-restore/` already
// names:
//
//   - `restore-msc1-has-no-automatic-rollback-after-interrupted-
//     extraction-phase6-correction.json`: source removes the live world
//     folders *before* extracting the replacement (`removeWorldFolders`
//     then `unzip`, lines 668-684, no staging) — a failed `unzip` leaves
//     the server with no live world at all, recoverable only by an
//     operator manually finding and restoring the pre-restore safety
//     backup source itself already created two steps earlier. This is
//     the exact shape `worlds::activate_slot`'s own P6.13 correction
//     already fixed once for activation; restore gets the identical
//     three-phase on-disk transaction, under `world_slots/.restore/`
//     rather than `.activation/` (a sibling, not a collision — the two
//     transactions are independent state machines that happen to touch
//     the same live folders, never concurrently once P6.21 wires the
//     exclusivity this phase has flagged at every step so far).
//   - the backup-verification correction P6.15/P6.16 already built
//     (`archive::validate_archive_safety`) is restore's own "verify
//     source" gate (source's `validateZipArchive` call, line 656) —
//     reused, not reimplemented.
//
// Unlike activation, a restored backup carries no new world identity to
// commit (a backup ZIP's member paths already match *this* server's
// current level-name — that's what `backupWorld`/`createBackup` captured
// them as) — so this transaction's "installed" phase has no commit tail
// beyond removing `.restore/` itself, and its own manifest doesn't need
// to remember anything phase-3 recovery can't already re-derive from the
// directory layout alone (unlike activation's slot id/identity) — so
// restore's transaction carries no `manifest.json` at all.
//
// Deviation from this step's own `Files:` list, flagged rather than
// silent: `msc-application::operations` (`LifecycleOperations`) is not
// touched. No world- or backup-domain error type in this crate converts
// into `operations::OperationError`/routes through `LifecycleOperations`
// anywhere yet — every one of P6.13/14/16/17's own section docs already
// deferred that conversion to P6.21 (route wiring), once a route exists
// to own an operation's lifecycle across an async boundary; forcing it
// into this synchronous application function now would be a second,
// premature integration point with no caller yet. "Explain the outcome
// through the operation record" is satisfied here by
// [`RestoreOutcome`]/[`RestoreRecovery`] — structured, typed results a
// future route-layer `OperationError`/success-result mapping (P6.21) can
// translate directly, the same way `worlds::ActivationRecovery` already
// stands ready for `activate_slot`'s own eventual route wiring.
// =====================================================================

#[derive(Debug)]
pub enum RestoreError {
    /// `restoreBackup`'s server-type guard (source line 595-602): Java
    /// only, currently — checked first, before any other guard.
    BedrockNotSupported,
    /// Source line 604-609.
    ServerRunning,
    /// Source line 611-620: the backup carries a non-nil `slotId` that
    /// disagrees with the resolved active slot. A legacy backup with no
    /// slot association, or no resolvable active slot at all, always
    /// passes this guard regardless of `backup_slot_id`.
    CrossSlot {
        backup_slot_id: String,
        active_slot_id: String,
    },
    /// Source line 622-628.
    SourceMissing,
    /// The mandatory pre-restore safety backup itself failed — a hard
    /// abort (source line 645-654): nothing about the current world is
    /// touched.
    SafetyBackupFailed(BackupError),
    /// `validateZipArchive`'s structural check (source line 656) failed
    /// — the current world is never touched (source line 657-665; this
    /// port's own transaction hasn't started staging yet either way).
    ArchiveInvalid,
    Io(io::Error),
    Archive(ArchiveError),
}

impl fmt::Display for RestoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RestoreError::BedrockNotSupported => {
                write!(
                    f,
                    "live-world restore is currently supported for Java servers only"
                )
            }
            RestoreError::ServerRunning => write!(f, "server is running"),
            RestoreError::CrossSlot {
                backup_slot_id,
                active_slot_id,
            } => write!(
                f,
                "backup belongs to slot {backup_slot_id}, not the active slot {active_slot_id}"
            ),
            RestoreError::SourceMissing => write!(f, "backup source file is missing"),
            RestoreError::SafetyBackupFailed(e) => {
                write!(f, "pre-restore safety backup failed: {e}")
            }
            RestoreError::ArchiveInvalid => {
                write!(f, "backup archive failed structural verification")
            }
            RestoreError::Io(e) => write!(f, "{e}"),
            RestoreError::Archive(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RestoreError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreOutcome {
    /// The mandatory safety backup's own path — always created, always
    /// retained, regardless of how the restore itself concludes.
    pub safety_backup_zip_path: PathBuf,
}

fn restore_dir(server_dir: &Path) -> PathBuf {
    world_store::slots_directory(server_dir).join(".restore")
}

fn restore_staged_dir(server_dir: &Path) -> PathBuf {
    restore_dir(server_dir).join("staged")
}

fn restore_prior_dir(server_dir: &Path) -> PathBuf {
    restore_dir(server_dir).join("prior")
}

/// `restoreBackup(_:)`, transactional (see the section doc above).
/// `is_server_running`/`resolved_active_slot_id` are the caller's
/// already-known state (matching this file's established pattern of
/// folding an orchestration-layer guard into the same function, per
/// `worlds.rs`'s own module doc). `safety_backup_association`/
/// `safety_backup_server_id`/`safety_backup_server_display_name` feed
/// straight through to the mandatory safety backup's own
/// [`create_backup`] call — resolved by the caller
/// ([`world::effective_backup_association`]) exactly as every other
/// backup-creating call site in this module already requires.
#[allow(clippy::too_many_arguments)]
pub fn restore_backup(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    backup_zip_path: &Path,
    backup_slot_id: Option<&str>,
    resolved_active_slot_id: Option<&str>,
    is_server_running: bool,
    safety_backup_association: &BackupAssociation,
    safety_backup_server_id: Option<&str>,
    safety_backup_server_display_name: Option<&str>,
    now: &str,
) -> Result<RestoreOutcome, RestoreError> {
    if server_type == ServerType::Bedrock {
        return Err(RestoreError::BedrockNotSupported);
    }
    if is_server_running {
        return Err(RestoreError::ServerRunning);
    }
    if let (Some(backup_slot), Some(active_slot)) = (backup_slot_id, resolved_active_slot_id)
        && backup_slot != active_slot
    {
        return Err(RestoreError::CrossSlot {
            backup_slot_id: backup_slot.to_string(),
            active_slot_id: active_slot.to_string(),
        });
    }
    if !matches!(fs.stat(backup_zip_path), Ok(meta) if meta.is_file) {
        return Err(RestoreError::SourceMissing);
    }

    let safety_backup = create_backup(
        fs,
        server_dir,
        server_type,
        raw_level_name,
        safety_backup_association,
        safety_backup_server_id,
        safety_backup_server_display_name,
        false,
        true,
        Some("pre-restore"),
        None,
        now,
        None,
        || false,
    )
    .map_err(RestoreError::SafetyBackupFailed)?;

    if archive::validate_archive_safety(backup_zip_path).is_err() {
        return Err(RestoreError::ArchiveInvalid);
    }

    let level_name = world::current_level_name(server_type, raw_level_name);
    let current_folders =
        crate::worlds::existing_world_folders(fs, server_dir, server_type, &level_name);

    fs.create_dir_all(&restore_dir(server_dir))
        .map_err(RestoreError::Io)?;

    // Phase 1: stage the restored archive. The live world at the server
    // root is not touched by anything in this block — a failure here
    // (a corrupt/failing extraction) aborts with the live world
    // completely intact.
    let staged_dir = restore_staged_dir(server_dir);
    if let Err(e) = archive::extract_zip(backup_zip_path, &staged_dir) {
        let _ = fs.remove(&restore_dir(server_dir));
        return Err(RestoreError::Archive(e));
    }

    // Phase 2: move the current live folders aside.
    let prior_dir = restore_prior_dir(server_dir);
    fs.create_dir_all(&prior_dir).map_err(RestoreError::Io)?;
    for name in &current_folders {
        fs.rename(&server_dir.join(name), &prior_dir.join(name))
            .map_err(RestoreError::Io)?;
    }

    // Phase 3: install the staged restore, then remove the whole
    // transaction directory — `prior/` is discarded too, matching
    // source's own "restore succeeded" outcome (the safety backup, not
    // `prior/`, is the durable fallback from here on).
    crate::worlds::move_entries(fs, &staged_dir, server_dir).map_err(RestoreError::Io)?;
    let _ = fs.remove(&restore_dir(server_dir));

    Ok(RestoreOutcome {
        safety_backup_zip_path: safety_backup.zip_path,
    })
}

/// What [`reconcile_interrupted_restore`] did, if anything, on this call
/// — mirrors `worlds::ActivationRecovery`'s own shape exactly, since the
/// underlying transaction is the identical three-phase pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreRecovery {
    /// Phase 1 ("staged") or phase 2 ("prior_moved") was interrupted —
    /// the live world at the server root is (or has been restored to
    /// be) the complete, unmodified pre-restore world. The safety
    /// backup this restore attempt created is still on disk either way.
    RecoveredToOldWorld,
    /// Phase 3 ("installed") was interrupted after the restored world
    /// was already moved into place — nothing more to do beyond
    /// discarding the now-empty transaction directory.
    RecoveredToRestoredWorld,
}

/// Call once per server on agent startup, before any restore route is
/// reachable for it — the same "before routes are reachable" timing
/// [`crate::worlds::reconcile_imported_worlds`]/
/// [`crate::worlds::reconcile_interrupted_activation`] already
/// established. Driven purely by which of `.restore/{prior,staged}`
/// physically exist, per the section doc's own three-phase table.
pub fn reconcile_interrupted_restore(
    fs: &dyn FileSystem,
    server_dir: &Path,
) -> io::Result<Option<RestoreRecovery>> {
    let dir = restore_dir(server_dir);
    if fs.stat(&dir).is_err() {
        return Ok(None);
    }

    let prior_dir = restore_prior_dir(server_dir);
    let staged_dir = restore_staged_dir(server_dir);
    let prior_exists = fs.stat(&prior_dir).is_ok();
    let staged_exists = fs.stat(&staged_dir).is_ok();

    if !prior_exists {
        // Phase 1 ("staged"): nothing at the server root was ever
        // touched — discard the abandoned staging area outright.
        let _ = fs.remove(&dir);
        return Ok(Some(RestoreRecovery::RecoveredToOldWorld));
    }

    if staged_exists {
        // Phase 2 ("prior_moved"): the server root currently has no
        // live world at all — move the prior folders back.
        crate::worlds::move_entries(fs, &prior_dir, server_dir)?;
        let _ = fs.remove(&dir);
        return Ok(Some(RestoreRecovery::RecoveredToOldWorld));
    }

    // Phase 3 ("installed"): the restored world is already at the
    // server root; nothing left but to discard the transaction
    // directory.
    let _ = fs.remove(&dir);
    Ok(Some(RestoreRecovery::RecoveredToRestoredWorld))
}
