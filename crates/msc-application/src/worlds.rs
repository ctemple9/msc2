//! Reconciles whatever world state Phase 5's two import paths (raw import,
//! transfer import) can leave on a server directory into the formal
//! world-slot model, before any world-mutation route becomes reachable for
//! that server.
//!
//! Implements the rule `docs/msc2/worlds/phase6-scope.md` (P6.1) fixed for
//! the three starting states raw/transfer import can produce — live world
//! folders only, `world_slots/` only, or both together — reusing
//! `WorldSlotManager`'s own active-slot resolution chain
//! (`msc_domain::world::resolve_active_slot_id`) and archiving mechanism
//! (`msc_domain::world::build_bootstrap_slot` +
//! `msc_infrastructure::archive::create_zip_from_folders`, mirroring
//! `AppViewModel+WorldSlots.createInitialWorldSlotIfNeeded`'s own
//! `WorldSlotManager.createSlot` call plus its `lastPlayedAt = Date()`
//! finalization — the same bootstrap shape every "archive live folders as
//! a brand-new slot" branch in this module uses, State 1 and State 3's
//! recovery-snapshot case alike, since both are "this data just became the
//! active slot" moments in the same sense the post-first-stop bootstrap is).
//!
//! **Idempotency, per phase6-scope.md's own "Ordering and crash safety"
//! section:** a dedicated marker (`world_slots/.p6_reconciled`), distinct
//! from `WorldSlotManager`'s own `active_slot_id.txt`, records that this
//! reconciliation has already run for a server — so a copied-in,
//! MSC-1-native `active_slot_id.txt` (which can legitimately already
//! resolve to something the moment Phase 5 finishes importing) is never
//! mistaken for proof that Phase 6's own live-vs-archive comparison
//! already happened. That marker is the last write of a successful
//! reconciliation, after every required archive/metadata write for that
//! server has already succeeded, and is checked first on every call — a
//! second call against an already-reconciled server is a no-op.

use msc_domain::identity::ServerType;
use msc_domain::nbt;
use msc_domain::world::{self, BackupAssociation, WorldSlot};
use msc_infrastructure::archive::{self, ArchiveError};
use msc_infrastructure::atomic_write::AtomicWriteError;
use msc_infrastructure::download_staging::sha1_hex;
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::world_store;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationOutcome {
    /// The dedicated marker was already present — no-op, matching the
    /// "a second startup makes no additional changes" requirement.
    AlreadyReconciled,
    /// Neither live world folders nor a resolvable `world_slots/` exist —
    /// nothing to inventory, nothing to protect.
    NoWorldData,
    /// State 1 (live folders only, or live folders plus unresolvable
    /// `world_slots/` data): the live folders were archived into a new
    /// slot, which became active. Any pre-existing unresolvable slot data
    /// is left on disk, untouched.
    LiveFoldersArchivedAsNewActiveSlot { new_slot_id: String },
    /// State 2, archived branch: the resolved active slot's `world.zip`
    /// was extracted into the live-folder location, and the active
    /// marker was persisted (Phase 6 persists it; Phase 5's own
    /// `restore_active_slot_world` deliberately did not).
    ArchiveExtractedFromResolvedSlot { slot_id: String },
    /// State 2, archive-less branch: the resolved active slot has no
    /// backing archive, so no live data is materialized — the active
    /// marker is still persisted so activation state is well-defined.
    ArchiveLessSlotMarkedActive { slot_id: String },
    /// State 3, proven-identical branch: the live folders are proven
    /// (file-by-file: presence, size, content hash) identical to the
    /// recorded active slot's archive. The active marker is persisted to
    /// the existing slot; no new slot is created.
    LiveFoldersProvenIdenticalToRecordedSlot { slot_id: String },
    /// State 3, different-or-unproven branch: the live folders were
    /// archived into a new "recovery snapshot" slot, distinct from the
    /// previously-recorded slot, which becomes active. The previously-
    /// recorded slot survives untouched as an ordinary, non-active,
    /// selectable slot.
    RecoverySnapshotCreated {
        new_slot_id: String,
        previous_slot_id: String,
    },
}

#[derive(Debug)]
pub enum ReconciliationError {
    Io(io::Error),
    Archive(ArchiveError),
    AtomicWrite(AtomicWriteError),
}

impl fmt::Display for ReconciliationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReconciliationError::Io(e) => write!(f, "{e}"),
            ReconciliationError::Archive(e) => write!(f, "{e}"),
            ReconciliationError::AtomicWrite(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ReconciliationError {}

fn reconciliation_marker_path(server_dir: &Path) -> PathBuf {
    world_store::slots_directory(server_dir).join(".p6_reconciled")
}

/// Scratch location for [`reconcile_imported_worlds`]'s own "extract an
/// archive into the live-folder location" branch — distinct from
/// [`activation_staged_dir`], which belongs to a different transaction
/// (`activate_slot`/[`reconcile_interrupted_activation`]) that never runs
/// concurrently with startup reconciliation but shouldn't share a
/// directory with it regardless. Extraction lands here first so a
/// corrupt archive or a mid-extraction crash never leaves a partially
/// populated live folder at `server_dir` — nothing at the live location
/// is touched until the full archive has extracted successfully.
fn reconciliation_staged_dir(server_dir: &Path) -> PathBuf {
    world_store::slots_directory(server_dir).join(".p6_reconcile_staged")
}

/// The candidate-name half already lives in `msc_domain::world`
/// (`backup_root_folder_candidates`); this is the existence-filtering half
/// `WorldSlotManager.worldFolderNames(for:)` mixes into the same
/// function in source, kept separate here per the module-boundary split
/// P6.9 already established.
pub(crate) fn existing_world_folders(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    level_name: &str,
) -> Vec<String> {
    world::backup_root_folder_candidates(server_type, level_name)
        .into_iter()
        .filter(|name| matches!(fs.stat(&server_dir.join(name)), Ok(meta) if meta.is_dir))
        .collect()
}

/// `server.properties`' `level-name` value, for Java servers only — no
/// fixture in this domain names a Bedrock case (Bedrock's own runtime
/// stays unavailable until Phase 10 per this phase's own deferral), so
/// this reads only the one properties file every P6.11 fixture actually
/// needs. Flagged narrowing, not a silent one.
pub fn read_java_level_name(fs: &dyn FileSystem, server_dir: &Path) -> Option<String> {
    let bytes = fs.read(&server_dir.join("server.properties")).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=')
            && key.trim() == "level-name"
        {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn has_archive(fs: &dyn FileSystem, server_dir: &Path, slot_id: &str) -> bool {
    matches!(fs.stat(&world_store::zip_path(server_dir, slot_id)), Ok(meta) if meta.is_file)
}

/// Archives `live_folders` into a brand-new slot and makes it active —
/// the one operation State 1, State 3's "resolution finds nothing"
/// sub-case, and State 3's recovery-snapshot case all share. Mirrors
/// `createInitialWorldSlotIfNeeded`'s shape: `defaultPersistentSlotName`
/// for the name, `lastPlayedAt` set to `created_at` (not left `None`,
/// unlike a plain `createSlot` snapshot) since this slot is becoming
/// active the moment it's created.
fn archive_live_folders_as_new_active_slot(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    live_folders: &[String],
    created_at: &str,
) -> Result<WorldSlot, ReconciliationError> {
    let id = Uuid::new_v4().to_string().to_uppercase();
    let slot = world::build_bootstrap_slot(
        id.clone(),
        server_type,
        raw_level_name,
        created_at.to_string(),
    );

    let dir = world_store::slot_directory(server_dir, &id);
    fs.create_dir_all(&dir).map_err(ReconciliationError::Io)?;
    if !live_folders.is_empty() {
        let zip_path = world_store::zip_path(server_dir, &id);
        archive::create_zip_from_folders(&zip_path, server_dir, live_folders)
            .map_err(ReconciliationError::Archive)?;
    }

    world_store::save_metadata(fs, server_dir, &slot).map_err(ReconciliationError::AtomicWrite)?;
    world_store::set_active_slot_id(fs, server_dir, Some(&slot.id))
        .map_err(ReconciliationError::Io)?;
    Ok(slot)
}

/// One file's identity for the purposes of the file-by-file comparison
/// phase6-scope.md's State 3 requires: presence (an entry existing in
/// this map at all), size, and content hash. A cheap check that could
/// produce a false "identical" (matching only names or sizes) is
/// explicitly disallowed there, so this always hashes full content —
/// reusing `msc-infrastructure`'s existing from-scratch SHA1 rather than
/// adding a new hashing dependency for this one comparison.
fn collect_world_file_fingerprints(
    root: &Path,
    folder_names: &[String],
) -> io::Result<BTreeMap<PathBuf, (u64, String)>> {
    let mut out = BTreeMap::new();
    for name in folder_names {
        collect_files_into(&root.join(name), Path::new(name), &mut out)?;
    }
    Ok(out)
}

fn collect_files_into(
    dir: &Path,
    rel_prefix: &Path,
    out: &mut BTreeMap<PathBuf, (u64, String)>,
) -> io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, io::Error>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let rel = rel_prefix.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_files_into(&path, &rel, out)?;
        } else if file_type.is_file() {
            let bytes = fs::read(&path)?;
            out.insert(rel, (bytes.len() as u64, sha1_hex(&bytes)));
        }
    }
    Ok(())
}

/// State 3's file-by-file proof. Extracts `zip_path` to a scratch
/// location outside `server_dir` (never touching the live folders or the
/// recorded slot's own archive), fingerprints both trees, and compares
/// for exact equality. Any failure along the way (corrupt archive,
/// unreadable file) is "equality cannot be established" — `false`, per
/// phase6-scope.md, not a hard error that would abort reconciliation.
fn live_folders_proven_identical_to_archive(
    server_dir: &Path,
    zip_path: &Path,
    live_folders: &[String],
) -> bool {
    let scratch = std::env::temp_dir().join(format!("msc2-world-reconcile-{}", Uuid::new_v4()));
    let result = (|| -> io::Result<bool> {
        fs::create_dir_all(&scratch)?;
        archive::extract_zip(zip_path, &scratch).map_err(io::Error::other)?;
        let live = collect_world_file_fingerprints(server_dir, live_folders)?;
        let archived = collect_world_file_fingerprints(&scratch, live_folders)?;
        Ok(live == archived)
    })();
    let _ = fs::remove_dir_all(&scratch);
    result.unwrap_or(false)
}

/// The idempotent P6.1 handoff. Call once per server before any world-
/// mutation route is reachable for it — see the module doc for the
/// dedicated-marker mechanism that makes a repeated call a no-op.
/// `raw_level_name` is `server.properties`' `level-name` value for Java
/// servers (`None` for Bedrock — see [`read_java_level_name`]'s doc);
/// callers may pass `None` to have this function read it itself via
/// [`read_java_level_name`], or supply an already-read value.
pub fn reconcile_imported_worlds(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    now: &str,
) -> Result<ReconciliationOutcome, ReconciliationError> {
    if fs.stat(&reconciliation_marker_path(server_dir)).is_ok() {
        return Ok(ReconciliationOutcome::AlreadyReconciled);
    }

    let owned_level_name = if raw_level_name.is_none() && server_type == ServerType::Java {
        read_java_level_name(fs, server_dir)
    } else {
        None
    };
    let raw_level_name = raw_level_name.or(owned_level_name.as_deref());
    let level_name = world::current_level_name(server_type, raw_level_name);
    let live_folders = existing_world_folders(fs, server_dir, server_type, &level_name);

    let slots = world_store::load_slots(fs, server_dir);
    let explicit_marker = world_store::load_explicit_active_slot_id(fs, server_dir);
    let resolved_active_id = world::resolve_active_slot_id(&slots, explicit_marker.as_deref());
    let resolved_active_slot = resolved_active_id
        .as_deref()
        .and_then(|id| slots.iter().find(|s| s.id == id));

    let outcome = match (live_folders.is_empty(), resolved_active_slot) {
        (true, None) => ReconciliationOutcome::NoWorldData,

        (true, Some(slot)) if has_archive(fs, server_dir, &slot.id) => {
            let zip_path = world_store::zip_path(server_dir, &slot.id);
            let staged_dir = reconciliation_staged_dir(server_dir);
            let _ = fs.remove(&staged_dir);
            if let Err(e) = archive::extract_zip(&zip_path, &staged_dir) {
                let _ = fs.remove(&staged_dir);
                return Err(ReconciliationError::Archive(e));
            }
            // The live-folder location is not touched until every entry
            // has already extracted successfully into `staged_dir`.
            if let Err(e) = move_entries(fs, &staged_dir, server_dir) {
                let _ = fs.remove(&staged_dir);
                return Err(ReconciliationError::Io(e));
            }
            let _ = fs.remove(&staged_dir);
            world_store::set_active_slot_id(fs, server_dir, Some(&slot.id))
                .map_err(ReconciliationError::Io)?;
            ReconciliationOutcome::ArchiveExtractedFromResolvedSlot {
                slot_id: slot.id.clone(),
            }
        }
        (true, Some(slot)) => {
            world_store::set_active_slot_id(fs, server_dir, Some(&slot.id))
                .map_err(ReconciliationError::Io)?;
            ReconciliationOutcome::ArchiveLessSlotMarkedActive {
                slot_id: slot.id.clone(),
            }
        }

        (false, None) => {
            let slot = archive_live_folders_as_new_active_slot(
                fs,
                server_dir,
                server_type,
                raw_level_name,
                &live_folders,
                now,
            )?;
            ReconciliationOutcome::LiveFoldersArchivedAsNewActiveSlot {
                new_slot_id: slot.id,
            }
        }
        (false, Some(slot)) if !has_archive(fs, server_dir, &slot.id) => {
            let new_slot = archive_live_folders_as_new_active_slot(
                fs,
                server_dir,
                server_type,
                raw_level_name,
                &live_folders,
                now,
            )?;
            ReconciliationOutcome::LiveFoldersArchivedAsNewActiveSlot {
                new_slot_id: new_slot.id,
            }
        }
        (false, Some(slot)) => {
            let zip_path = world_store::zip_path(server_dir, &slot.id);
            if live_folders_proven_identical_to_archive(server_dir, &zip_path, &live_folders) {
                world_store::set_active_slot_id(fs, server_dir, Some(&slot.id))
                    .map_err(ReconciliationError::Io)?;
                ReconciliationOutcome::LiveFoldersProvenIdenticalToRecordedSlot {
                    slot_id: slot.id.clone(),
                }
            } else {
                let new_slot = archive_live_folders_as_new_active_slot(
                    fs,
                    server_dir,
                    server_type,
                    raw_level_name,
                    &live_folders,
                    now,
                )?;
                ReconciliationOutcome::RecoverySnapshotCreated {
                    new_slot_id: new_slot.id,
                    previous_slot_id: slot.id.clone(),
                }
            }
        }
    };

    fs.create_dir_all(&world_store::slots_directory(server_dir))
        .map_err(ReconciliationError::Io)?;
    fs.write(&reconciliation_marker_path(server_dir), b"1")
        .map_err(ReconciliationError::Io)?;

    Ok(outcome)
}

// =====================================================================
// P6.12 — slot CRUD, copy, import, export, and thumbnails
//
// Ports `WorldSlotManager`'s eight slot-mutation verbs (`createSlot`,
// `updateSlotFromCurrentWorld`, `renameSlot`, `deleteSlot`,
// `duplicateSlot`, `copySlotIntoExisting`, `exportSlotZIP`,
// `createSlotFromZIP`) plus `saveThumbnail`'s application-layer entry
// point, merged with the orchestration-layer guards
// `AppViewModel+WorldSlots.swift` applies at each matching call site
// (name trimming/empty checks, the active-slot delete refusal) — the
// same "pure port plus its own orchestration guard, one layer" shape
// P6.11 already established for reconciliation, per
// `docs/msc2/worlds/phase6-scope.md`'s own module-boundary note that
// `msc-infrastructure` stays as ignorant of caller-level policy as
// `WorldSlotManager` is.
//
// `fixtures/world-mutations/`'s 10 non-activation, non-direct-rename/
// replace cases (P6.5) are this section's characterization; each
// function's doc comment cites the specific case and MSC 1 source lines
// it ports.
// =====================================================================

#[derive(Debug)]
pub enum WorldError {
    Io(io::Error),
    Archive(ArchiveError),
    AtomicWrite(AtomicWriteError),
    /// `worldFolderNames(for:)` found nothing on disk to archive —
    /// `createSlot`/`updateSlotFromCurrentWorld`'s shared "nothing to
    /// save" guard.
    NoWorldFolders,
    /// The operation's source slot has no `world.zip` on disk yet.
    NoSourceZip,
    /// A caller-supplied name was empty (or all whitespace) where source
    /// requires a non-blank one.
    EmptyName,
    /// `deleteWorldSlot`'s active-slot refusal
    /// (`fixtures/world-mutations/delete-active-slot-refused.json`).
    ActiveSlotDeleteRefused,
    /// `activateSlot`'s guard: neither a real archive nor fresh-world
    /// generation metadata exists for this slot.
    NoArchiveOrFreshMetadata,
    /// The mandatory pre-activation/pre-rename/pre-replace safety backup
    /// failed or was refused.
    BackupFailed,
    /// `renameWorld`'s all-or-nothing pre-check: a folder already exists
    /// at one of the target names.
    TargetFolderExists(String),
    /// A running server refused an operation that touches live world
    /// data (`activateWorldSlot`/`renameWorld`/`replaceWorld`'s
    /// identically-shaped guard).
    ServerRunning,
    /// The caller-supplied replacement world source failed validation
    /// (unreadable backup ZIP, or a source folder that doesn't exist).
    InvalidWorldSource,
    /// P6.33: the mandatory pre-replace safety backup itself failed —
    /// `replace_world`'s own hard-abort guard, distinct from
    /// [`WorldError::BackupFailed`] (`rename_world`'s caller-optional
    /// backup closure, unchanged by this correction).
    SafetyBackupFailed(crate::backups::BackupError),
    /// P6.33: an interrupted-replace manifest under `world_slots/.replace/`
    /// is missing or unreadable — the same "can't trust a half-written
    /// journal" case [`ActivationError::Manifest`] documents.
    Manifest,
    /// P6.30-style cooperative cancellation, reported at one of the two
    /// "nothing at the live world touched yet" boundaries
    /// `replace_world`'s own doc comment names — the same two-boundary
    /// shape [`ActivationError::Cancelled`]/`RestoreError::Cancelled`
    /// already use. The safety backup this attempt already created (if
    /// any) is left on disk regardless.
    Cancelled,
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldError::Io(e) => write!(f, "{e}"),
            WorldError::Archive(e) => write!(f, "{e}"),
            WorldError::AtomicWrite(e) => write!(f, "{e}"),
            WorldError::NoWorldFolders => write!(f, "no world folders found to save"),
            WorldError::NoSourceZip => write!(f, "source slot has no saved world archive"),
            WorldError::EmptyName => write!(f, "name is empty"),
            WorldError::ActiveSlotDeleteRefused => {
                write!(f, "cannot delete the active world slot")
            }
            WorldError::NoArchiveOrFreshMetadata => write!(
                f,
                "slot has no saved world archive and no fresh-world generation metadata"
            ),
            WorldError::BackupFailed => write!(f, "pre-operation safety backup failed"),
            WorldError::TargetFolderExists(name) => {
                write!(f, "a folder named {name} already exists")
            }
            WorldError::ServerRunning => write!(f, "server is running"),
            WorldError::InvalidWorldSource => write!(f, "replacement world source is invalid"),
            WorldError::SafetyBackupFailed(e) => {
                write!(f, "pre-replace safety backup failed: {e}")
            }
            WorldError::Manifest => write!(f, "interrupted world replace manifest is unreadable"),
            WorldError::Cancelled => write!(f, "world replace was cancelled"),
        }
    }
}

impl std::error::Error for WorldError {}

impl From<io::Error> for WorldError {
    fn from(e: io::Error) -> Self {
        WorldError::Io(e)
    }
}

impl From<ArchiveError> for WorldError {
    fn from(e: ArchiveError) -> Self {
        WorldError::Archive(e)
    }
}

impl From<AtomicWriteError> for WorldError {
    fn from(e: AtomicWriteError) -> Self {
        WorldError::AtomicWrite(e)
    }
}

/// A copy through the [`FileSystem`] trait (`write(read(from))`) rather
/// than reaching for `std::fs::copy` directly — every other CRUD
/// operation in this section touches `slot.json`/the active marker
/// through `fs`, so the zip-copy half stays behind the same
/// abstraction instead of silently depending on both being backed by
/// the same real disk.
fn copy_via_fs(fs: &dyn FileSystem, from: &Path, to: &Path) -> io::Result<()> {
    let bytes = fs.read(from)?;
    fs.write(to, &bytes)
}

/// `loadSlots`'s zip-size stat, duplicated from `world_store`'s private
/// equivalent rather than made `pub` across the crate boundary for this
/// module's own handful of call sites — the same small-duplicate call
/// P6.11's `iso8601_now` already made for `audit_log`'s calendar math.
fn zip_size_bytes(fs: &dyn FileSystem, path: &Path) -> Option<i64> {
    fs.read(path).ok().map(|bytes| bytes.len() as i64)
}

fn slot_zip_exists(fs: &dyn FileSystem, server_dir: &Path, slot_id: &str) -> bool {
    has_archive(fs, server_dir, slot_id)
}

/// `createSlot(name:for:worldSeed:logLine:)` (source line 391-461):
/// zips whatever world folders currently exist into a brand-new slot.
/// On a zip failure, the just-created slot directory is removed before
/// returning — no half-written `slot.json` or partial archive is left
/// behind
/// (`fixtures/world-mutations/create-slot-zip-failure-cleans-up-slot-directory.json`).
/// Covers both server types via [`world::backup_root_folder_candidates`]
/// (`create-slot-java-zips-main-nether-end.json`,
/// `create-slot-bedrock-zips-worlds-folder.json`).
pub fn create_slot_from_current_world(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    name: &str,
    seed: Option<&str>,
    now: &str,
) -> Result<WorldSlot, WorldError> {
    let level_name = world::current_level_name(server_type, raw_level_name);
    let folders = existing_world_folders(fs, server_dir, server_type, &level_name);
    if folders.is_empty() {
        return Err(WorldError::NoWorldFolders);
    }

    let id = Uuid::new_v4().to_string().to_uppercase();
    let mut slot = world::build_archived_slot(
        id.clone(),
        name,
        seed,
        server_type,
        raw_level_name,
        now.to_string(),
    );

    let dir = world_store::slot_directory(server_dir, &id);
    fs.create_dir_all(&dir)?;
    let zip_path = world_store::zip_path(server_dir, &id);
    if let Err(e) = archive::create_zip_from_folders(&zip_path, server_dir, &folders) {
        let _ = fs.remove(&dir);
        return Err(e.into());
    }
    slot.zip_size_bytes = zip_size_bytes(fs, &zip_path);

    if let Err(e) = world_store::save_metadata(fs, server_dir, &slot) {
        let _ = fs.remove(&dir);
        return Err(e.into());
    }
    Ok(slot)
}

/// `updateSlotFromCurrentWorld(_:for:logLine:)` (source line 466-546):
/// re-zips the current world into `slot`'s *existing* archive via a
/// scratch-file-then-atomic-replace, so a zip failure never touches the
/// previous archive
/// (`fixtures/world-mutations/update-active-slot-zip-failure-preserves-previous-archive.json`).
/// Unlike [`create_slot_from_current_world`], `created_at`/`name`/
/// `last_played_at` are left untouched — only `world_level_name` and
/// `zip_size_bytes` change.
pub fn update_active_slot_from_current_world(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    slot: &WorldSlot,
) -> Result<WorldSlot, WorldError> {
    let level_name = world::current_level_name(server_type, raw_level_name);
    let folders = existing_world_folders(fs, server_dir, server_type, &level_name);
    if folders.is_empty() {
        return Err(WorldError::NoWorldFolders);
    }

    let dir = world_store::slot_directory(server_dir, &slot.id);
    fs.create_dir_all(&dir)?;
    let temp_zip = dir.join("world.update.tmp.zip");
    let _ = fs.remove(&temp_zip);

    if let Err(e) = archive::create_zip_from_folders(&temp_zip, server_dir, &folders) {
        let _ = fs.remove(&temp_zip);
        return Err(e.into());
    }

    let zip_path = world_store::zip_path(server_dir, &slot.id);
    let _ = fs.remove(&zip_path);
    fs.rename(&temp_zip, &zip_path)?;

    let mut updated = slot.clone();
    updated.world_level_name = Some(level_name);
    updated.zip_size_bytes = zip_size_bytes(fs, &zip_path);

    world_store::save_metadata(fs, server_dir, &updated)?;
    Ok(updated)
}

/// `renameSlot(_:newName:serverDir:)` (source line 786-791):
/// metadata-only, no file is moved or renamed on disk — the slot's
/// on-disk directory is keyed by its UUID, never its display name, and
/// `world.zip` is never opened
/// (`fixtures/world-mutations/rename-slot-metadata-only-leaves-archive-untouched.json`).
/// The empty-name guard lives in the orchestration layer in source
/// (`renameWorldSlot`); folded in here per this section's module doc.
pub fn rename_slot(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot: &WorldSlot,
    new_name: &str,
) -> Result<WorldSlot, WorldError> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(WorldError::EmptyName);
    }
    let mut updated = slot.clone();
    updated.name = trimmed.to_string();
    world_store::save_metadata(fs, server_dir, &updated)?;
    Ok(updated)
}

/// `deleteWorldSlot(_:)`'s active-slot guard (source
/// `AppViewModel+WorldSlots.swift:297-318`) plus `WorldSlotManager
/// .deleteSlot(_:serverDir:)` (source line 795-798) — the guard lives in
/// the orchestration layer, not the repository, matching source exactly
/// (`fixtures/world-mutations/delete-active-slot-refused.json`).
/// `resolved_active_slot_id` is the caller's already-resolved value
/// (`world::resolve_active_slot_id`), not re-derived here.
pub fn delete_slot(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot: &WorldSlot,
    resolved_active_slot_id: Option<&str>,
) -> Result<(), WorldError> {
    if resolved_active_slot_id == Some(slot.id.as_str()) {
        return Err(WorldError::ActiveSlotDeleteRefused);
    }
    let dir = world_store::slot_directory(server_dir, &slot.id);
    fs.remove(&dir)?;
    Ok(())
}

/// `duplicateSlot(_:newName:for:logLine:)` (source line 805-865): a
/// fresh UUID, never the source id; only reads from the source zip, so
/// the source slot is left completely untouched
/// (`fixtures/world-mutations/duplicate-slot-fresh-uuid-source-untouched.json`).
pub fn duplicate_slot(
    fs: &dyn FileSystem,
    server_dir: &Path,
    source: &WorldSlot,
    new_name: &str,
    now: &str,
) -> Result<WorldSlot, WorldError> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err(WorldError::EmptyName);
    }
    if !slot_zip_exists(fs, server_dir, &source.id) {
        return Err(WorldError::NoSourceZip);
    }

    let new_id = Uuid::new_v4().to_string().to_uppercase();
    let mut new_slot = WorldSlot {
        id: new_id.clone(),
        name: trimmed.to_string(),
        created_at: now.to_string(),
        last_played_at: None,
        thumbnail_file_name: None,
        world_level_name: source.world_level_name.clone(),
        world_seed: source.world_seed.clone(),
        zip_size_bytes: None,
    };

    let new_dir = world_store::slot_directory(server_dir, &new_id);
    fs.create_dir_all(&new_dir)?;
    let source_zip = world_store::zip_path(server_dir, &source.id);
    let dest_zip = world_store::zip_path(server_dir, &new_id);
    if let Err(e) = copy_via_fs(fs, &source_zip, &dest_zip) {
        let _ = fs.remove(&new_dir);
        return Err(e.into());
    }
    new_slot.zip_size_bytes = zip_size_bytes(fs, &dest_zip);

    if let Err(e) = world_store::save_metadata(fs, server_dir, &new_slot) {
        let _ = fs.remove(&new_dir);
        return Err(e.into());
    }
    Ok(new_slot)
}

/// `copySlotIntoExisting(_:into:for:logLine:)` (source line 875-937):
/// destructive by design (overwrites `destination`'s world data), but
/// never touches `destination`'s real archive until the source has
/// already been copied into a scratch file inside `destination`'s own
/// slot directory — a mid-copy failure leaves `destination` completely
/// untouched and removes the orphaned scratch file
/// (`fixtures/world-mutations/copy-into-existing-mid-copy-failure-preserves-destination.json`).
/// A metadata-save failure afterward is non-fatal, matching source's own
/// comment: world data is already in place by that point.
pub fn copy_slot_into_existing(
    fs: &dyn FileSystem,
    server_dir: &Path,
    source: &WorldSlot,
    destination: &WorldSlot,
    now: &str,
) -> Result<WorldSlot, WorldError> {
    if !slot_zip_exists(fs, server_dir, &source.id) {
        return Err(WorldError::NoSourceZip);
    }

    let dest_dir = world_store::slot_directory(server_dir, &destination.id);
    fs.create_dir_all(&dest_dir)?;
    let temp_zip = dest_dir.join("world.replace.tmp.zip");
    let _ = fs.remove(&temp_zip);

    let source_zip = world_store::zip_path(server_dir, &source.id);
    if let Err(e) = copy_via_fs(fs, &source_zip, &temp_zip) {
        let _ = fs.remove(&temp_zip);
        return Err(e.into());
    }

    let dest_zip = world_store::zip_path(server_dir, &destination.id);
    let _ = fs.remove(&dest_zip);
    fs.rename(&temp_zip, &dest_zip)?;

    let mut updated = destination.clone();
    updated.created_at = now.to_string();
    updated.world_level_name = source.world_level_name.clone();
    updated.world_seed = source.world_seed.clone();
    updated.zip_size_bytes = zip_size_bytes(fs, &dest_zip);

    let _ = world_store::save_metadata(fs, server_dir, &updated);
    Ok(updated)
}

/// `exportSlotZIP(_:from:to:logLine:)` (source line 960-989): a plain
/// overwrite-at-destination copy — if a file already exists at
/// `destination_path` it's removed first so the copy doesn't fail with
/// "file already exists"
/// (`fixtures/world-mutations/export-slot-zip-overwrites-destination.json`).
/// `destination_path` is a caller-resolved staged-download path
/// (`docs/msc2/worlds/phase6-api.md`'s bounded staging convention), not
/// an arbitrary host path this function accepts unchecked.
pub fn export_slot_zip(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot: &WorldSlot,
    destination_path: &Path,
) -> Result<(), WorldError> {
    if !slot_zip_exists(fs, server_dir, &slot.id) {
        return Err(WorldError::NoSourceZip);
    }
    if fs.stat(destination_path).is_ok() {
        fs.remove(destination_path)?;
    }
    let source_zip = world_store::zip_path(server_dir, &slot.id);
    copy_via_fs(fs, &source_zip, destination_path)?;
    Ok(())
}

/// `inferJavaLevelName(fromSlotZIP:)` (source line 187-221): the "root
/// entry name minus a `_nether`/`_the_end` suffix" heuristic MSC 1 uses
/// to guess a just-imported Java slot's level-name — distinct from
/// [`nbt::first_level_dat_path`]'s "which member is `level.dat`"
/// selection (P6.9), and not folded into `msc-domain` since it needs a
/// real zip listing (I/O); kept here rather than added to
/// `msc-infrastructure` since it's a naming *guess*, not a general
/// archive primitive.
fn infer_java_level_name_from_zip(zip_path: &Path) -> Option<String> {
    let listing = archive::list_entry_names(zip_path).ok()?;
    let roots: BTreeSet<String> = listing
        .iter()
        .filter_map(|entry| {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                return None;
            }
            let first = trimmed.split('/').next().unwrap_or(trimmed);
            (!first.is_empty() && first != "__MACOSX").then(|| first.to_string())
        })
        .collect();

    let plain = roots
        .iter()
        .find(|r| !r.ends_with("_nether") && !r.ends_with("_the_end"));
    if let Some(best) = plain {
        return Some(best.clone());
    }
    let suffixed = roots.iter().next()?;
    suffixed
        .strip_suffix("_nether")
        .or_else(|| suffixed.strip_suffix("_the_end"))
        .map(str::to_string)
}

/// The adjacent `<name>.meta.json` sidecar's `worldSeed` field (source's
/// `readAdjacentBackupMetadata`, which decodes the full `BackupMeta` —
/// not ported until P6.15 — but only this one field is ever read back
/// out at this call site, so this reads it directly via
/// `serde_json::Value` rather than waiting on that port).
fn read_sidecar_world_seed(zip_path: &Path) -> Option<String> {
    let sidecar = zip_path.with_extension("meta.json");
    let bytes = std::fs::read(sidecar).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    nbt::trimmed_sidecar_seed(value.get("worldSeed").and_then(|v| v.as_str()))
}

/// `importedWorldMetadata(fromZIP:serverType:)`'s seed half (source line
/// 1260-1269): a non-blank sidecar seed always wins over a parsed
/// `level.dat` seed, via [`nbt::merge_sidecar_metadata`].
fn infer_imported_world_seed(zip_path: &Path, server_type: ServerType) -> Option<String> {
    let sidecar_seed = read_sidecar_world_seed(zip_path);
    let parsed = archive::list_entry_names(zip_path)
        .ok()
        .and_then(|listing| {
            let refs: Vec<&str> = listing.iter().map(String::as_str).collect();
            nbt::first_level_dat_path(&refs)
        })
        .and_then(|member| archive::read_entry_bytes(zip_path, &member).ok().flatten())
        .map(|bytes| nbt::imported_world_metadata_from_level_dat(&bytes, server_type))
        .unwrap_or_default();
    nbt::merge_sidecar_metadata(sidecar_seed, parsed).seed
}

/// `createSlotFromZIP(zipURL:name:for:logLine:)` (source line 1008-
/// 1077): copies the external zip verbatim into a new slot — no
/// structural validation is performed here, matching source's own
/// documented baseline exactly
/// (`fixtures/world-mutations/import-zip-as-new-slot-copies-verbatim-no-structural-validation.json`).
/// The D-006 correction for unsafe archive content lives uniformly at
/// every *extraction* point (`msc_infrastructure::archive::extract_zip`,
/// `fixtures/world-archive-safety`), applied once this slot is later
/// activated — not duplicated here.
pub fn import_zip_as_new_slot(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    source_zip_path: &Path,
    name: &str,
    now: &str,
) -> Result<WorldSlot, WorldError> {
    if !matches!(fs.stat(source_zip_path), Ok(m) if m.is_file) {
        return Err(WorldError::NoSourceZip);
    }
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(WorldError::EmptyName);
    }

    let new_id = Uuid::new_v4().to_string().to_uppercase();
    let dir = world_store::slot_directory(server_dir, &new_id);
    fs.create_dir_all(&dir)?;
    let dest_zip = world_store::zip_path(server_dir, &new_id);

    if let Err(e) = copy_via_fs(fs, source_zip_path, &dest_zip) {
        let _ = fs.remove(&dir);
        return Err(e.into());
    }

    let slot = WorldSlot {
        id: new_id,
        name: trimmed.to_string(),
        created_at: now.to_string(),
        last_played_at: None,
        thumbnail_file_name: None,
        world_level_name: match server_type {
            ServerType::Java => infer_java_level_name_from_zip(&dest_zip),
            ServerType::Bedrock => Some(world::current_level_name(server_type, raw_level_name)),
        },
        world_seed: infer_imported_world_seed(source_zip_path, server_type),
        zip_size_bytes: zip_size_bytes(fs, &dest_zip),
    };

    if let Err(e) = world_store::save_metadata(fs, server_dir, &slot) {
        let _ = fs.remove(&dir);
        return Err(e.into());
    }
    Ok(slot)
}

/// `setSlotThumbnail(_:image:)`'s application-layer entry point — the
/// deterministic half (resize math, atomic write, metadata update)
/// already lives in [`world_store::save_thumbnail`] (P6.10); this is a
/// thin pass-through so route/CLI callers reach every slot mutation
/// through this one module.
pub fn set_slot_thumbnail(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot: &WorldSlot,
    encoded_bytes: &[u8],
) -> Result<WorldSlot, WorldError> {
    Ok(world_store::save_thumbnail(
        fs,
        server_dir,
        slot,
        encoded_bytes,
    )?)
}

// =====================================================================
// P6.13 — transactional world activation and restart recovery
//
// Ports `WorldSlotManager.activateSlot(_:for:backupCurrent:logLine:
// backupWorld:)` (source line 643-778) merged with
// `AppViewModel.activateWorldSlot(_:)`'s running-server guard (source
// `AppViewModel+WorldSlots.swift:212-260`), corrected against the one
// gap `fixtures/world-mutations/
// activate-extraction-failure-leaves-partial-state-for-safety-backup-recovery.json`
// pins as MSC 1's own baseline: source removes the current live folders
// *before* extracting the replacement, so a corrupt/failing archive
// leaves the server with no world at all and only the (also-taken)
// safety backup to recover from — recovery there is manual, not
// automatic.
//
// This port closes that window with a three-phase on-disk transaction
// under `world_slots/.activation/`:
//
//   1. **staged** — the replacement is fully extracted into
//      `.activation/staged/` (or, for a fresh/archive-less slot, this
//      phase is trivially already true — nothing to stage). The live
//      folders at the server root are untouched. A failure here (a
//      corrupt archive, an I/O error) aborts with the live world
//      completely intact — the specific improvement over source.
//   2. **prior_moved** — the current live folders are moved (not
//      copied) into `.activation/prior/`. The server root now has no
//      live world at all — the same dangerous-looking window source
//      has, except every archive/legitimacy check already passed in
//      phase 1, so what's left is only a plain filesystem move.
//   3. **installed** — every entry staged in phase 1 is moved into the
//      server root, then `.activation/staged/` itself is removed
//      (`.activation/prior/` is deliberately left in place a moment
//      longer — see below). World identity, slot metadata, and the
//      active marker are then committed; `.activation/` is removed
//      last, only once every one of those has succeeded.
//
// The three phases are distinguished purely by which of
// `.activation/{prior,staged}` exist on disk — no separate journaled
// "current phase" field to trust or fall out of sync with reality:
//
//   | `prior/` | `staged/` | phase        | restart recovery            |
//   |----------|-----------|--------------|------------------------------|
//   | absent   | n/a       | staged       | delete `.activation/` — old world already complete |
//   | present  | present   | prior_moved  | move `prior/*` back to the server root, delete `.activation/` — old world restored |
//   | present  | absent    | installed    | re-run the commit tail (identity/metadata/marker — each idempotent), delete `.activation/` — new world completed |
//
// So a restart mid-transaction always reconciles to either the fully
// old or the fully new world, never a mixture — [`reconcile_interrupted_activation`]
// is that reconciler, driven only by this physical layout, not by
// trusting an in-memory or journaled "what was I doing" flag.
//
// A small `manifest.json` (slot id, and the identity to apply) is
// written once, atomically, at the very start of phase 1 — the only
// piece phase-3 recovery can't re-derive from the directory layout
// alone. Deviation from this step's planned `Files:` list, flagged
// rather than silent: this transaction does not route through
// `msc-infrastructure::operation_journal`/`msc-application::operations`
// (`LifecycleOperations`) — that substrate models an abstract
// queued/running/succeeded/failed *operation*, with no notion of a
// multi-step filesystem transaction's own phase, and forcing this
// three-phase move-based recovery through it would add a second,
// redundant source of truth alongside the directory layout itself
// rather than reuse one. Per-target exclusivity (so a concurrent
// backup/replace can't race an in-flight activation) is exactly the
// kind of cross-domain concern `OperationJournal::admit` already solves
// well — left for the route layer (P6.21) to wire once backups (P6.15+)
// exist to conflict with.
// =====================================================================
/// `ServerPropertiesManager.readProperties`/
/// `BedrockPropertiesManager.readRawProperties`'s shared parse shape —
/// both are plain `key=value` text files at this level, so one reader
/// serves both server types (the type-specific halves never actually
/// diverge in shape, only in which file they read).
fn read_properties_map(fs: &dyn FileSystem, path: &Path) -> BTreeMap<String, String> {
    let Ok(bytes) = fs.read(path) else {
        return BTreeMap::new();
    };
    let text = String::from_utf8_lossy(&bytes);
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}

/// `ServerPropertiesManager.writeProperties`/
/// `BedrockPropertiesManager.writeRawProperties`: a full rewrite (header
/// comment plus one sorted `key=value` line per entry) — comments and
/// blank lines from the original file don't survive, matching both
/// source functions exactly. Best-effort is the caller's choice, not
/// this function's — it returns the write's real result.
fn write_properties_map(
    fs: &dyn FileSystem,
    path: &Path,
    props: &BTreeMap<String, String>,
) -> io::Result<()> {
    let mut out = String::from("# Modified via MSC 2\n");
    for (key, value) in props {
        out.push_str(&format!("{key}={value}\n"));
    }
    fs.write(path, out.as_bytes())
}

/// `applyWorldIdentity(levelName:seed:applySeed:for:logLine:)` (source
/// `WorldSlotManager.swift:596-636`) once the caller has resolved which
/// level-name/seed to apply and whether the seed half applies at all
/// (the archived-slot activation branch calls this with `apply_seed:
/// false`; the fresh-slot branch and direct world replace/rename call it
/// with the seed half live).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldIdentity {
    pub level_name: String,
    pub seed: Option<String>,
    pub apply_seed: bool,
}

fn apply_world_identity(
    fs: &dyn FileSystem,
    server_dir: &Path,
    identity: &WorldIdentity,
) -> io::Result<()> {
    let path = server_dir.join("server.properties");
    let mut props = read_properties_map(fs, &path);
    props.insert("level-name".to_string(), identity.level_name.clone());
    if identity.apply_seed {
        match identity
            .seed
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(seed) => {
                props.insert("level-seed".to_string(), seed.to_string());
            }
            None => {
                props.remove("level-seed");
            }
        }
    }
    write_properties_map(fs, &path, &props)
}

fn activation_dir(server_dir: &Path) -> PathBuf {
    world_store::slots_directory(server_dir).join(".activation")
}

fn activation_manifest_path(server_dir: &Path) -> PathBuf {
    activation_dir(server_dir).join("manifest.json")
}

fn activation_staged_dir(server_dir: &Path) -> PathBuf {
    activation_dir(server_dir).join("staged")
}

fn activation_prior_dir(server_dir: &Path) -> PathBuf {
    activation_dir(server_dir).join("prior")
}

fn activation_manifest_value(slot_id: &str, identity: Option<&WorldIdentity>) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert(
        "slot_id".to_string(),
        serde_json::Value::String(slot_id.to_string()),
    );
    obj.insert(
        "identity".to_string(),
        match identity {
            None => serde_json::Value::Null,
            Some(identity) => {
                let mut i = serde_json::Map::new();
                i.insert(
                    "level_name".to_string(),
                    serde_json::Value::String(identity.level_name.clone()),
                );
                i.insert(
                    "seed".to_string(),
                    identity
                        .seed
                        .clone()
                        .map(serde_json::Value::String)
                        .unwrap_or(serde_json::Value::Null),
                );
                i.insert(
                    "apply_seed".to_string(),
                    serde_json::Value::Bool(identity.apply_seed),
                );
                serde_json::Value::Object(i)
            }
        },
    );
    serde_json::Value::Object(obj)
}

fn parse_activation_manifest(bytes: &[u8]) -> Option<(String, Option<WorldIdentity>)> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let slot_id = value.get("slot_id")?.as_str()?.to_string();
    let identity = match value.get("identity") {
        None | Some(serde_json::Value::Null) => None,
        Some(obj) => Some(WorldIdentity {
            level_name: obj.get("level_name")?.as_str()?.to_string(),
            seed: obj.get("seed").and_then(|v| v.as_str()).map(str::to_string),
            apply_seed: obj.get("apply_seed")?.as_bool()?,
        }),
    };
    Some((slot_id, identity))
}

/// Every top-level entry name directly under `dir` (not recursive) — the
/// unit both the "move current live folders aside" and "move staged
/// content into place" steps operate on, and what
/// [`reconcile_interrupted_activation`] replays without needing to have
/// remembered the names anywhere else.
pub(crate) fn top_level_entries(fs: &dyn FileSystem, dir: &Path) -> io::Result<Vec<PathBuf>> {
    match fs.list(dir) {
        Ok(entries) => Ok(entries),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

pub(crate) fn move_entries(fs: &dyn FileSystem, from_dir: &Path, to_dir: &Path) -> io::Result<()> {
    fs.create_dir_all(to_dir)?;
    for entry in top_level_entries(fs, from_dir)? {
        let name = entry
            .file_name()
            .expect("directory listing entries are named");
        fs.rename(&entry, &to_dir.join(name))?;
    }
    Ok(())
}

/// Freezes the calling thread indefinitely once the current live world
/// has been moved aside but before its replacement is installed --
/// giving `phase6-gate-smoke.sh`'s restart-race checks a stable,
/// arbitrarily-wide window to catch and kill a real agent process,
/// rather than racing a poll against a real handful-of-`rename()`-
/// syscalls window (which turned out to be too narrow to reliably
/// observe on Windows CI runners regardless of kill speed). A no-op
/// unless `MSC2_TEST_PAUSE_AFTER_WORLD_MOVE` is set; the smoke script
/// only ever sets it for an agent process it starts specifically to
/// serve one racy call before killing it, never for a process handling
/// any other operation.
pub(crate) fn test_pause_after_world_move() {
    if std::env::var_os("MSC2_TEST_PAUSE_AFTER_WORLD_MOVE").is_some() {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }
}

/// `worldFolderNames(for:)`'s Bedrock legacy-layout relocation (source
/// line 737-758), applied inside the staging directory rather than the
/// server root — a failure here is non-fatal either way, matching
/// source's own warning-only handling
/// (`fixtures/world-mutations/activate-legacy-zip-loose-worlds-root-relocated.json`),
/// but staging it first means a relocation failure never risks leaving
/// half-relocated files at the live server root.
fn relocate_legacy_bedrock_layout(staged_dir: &Path, level_name: &str) {
    let worlds_dir = staged_dir.join("worlds");
    let expected_dir = worlds_dir.join(level_name);
    let loose_db_dir = worlds_dir.join("db");
    if expected_dir.is_dir() || !loose_db_dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(&worlds_dir) else {
        return;
    };
    let _ = fs::create_dir_all(&expected_dir);
    for entry in entries.flatten() {
        if entry.file_name() == level_name {
            continue;
        }
        let dest = expected_dir.join(entry.file_name());
        let _ = fs::rename(entry.path(), dest);
    }
}

#[derive(Debug)]
pub enum ActivationError {
    ServerRunning,
    NoArchiveOrFreshMetadata,
    BackupFailed,
    Archive(ArchiveError),
    Io(io::Error),
    AtomicWrite(AtomicWriteError),
    Manifest,
    /// `should_cancel` reported true at a boundary where nothing at the
    /// server root had been touched yet (before staging began, or after
    /// staging but before the live folders were moved aside) — see
    /// [`activate_slot`]'s own doc. The live world is untouched, and any
    /// scratch staging this attempt created has already been cleaned up.
    Cancelled,
}

impl fmt::Display for ActivationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActivationError::ServerRunning => write!(f, "server is running"),
            ActivationError::NoArchiveOrFreshMetadata => write!(
                f,
                "slot has no saved world archive and no fresh-world generation metadata"
            ),
            ActivationError::BackupFailed => write!(f, "pre-activation safety backup failed"),
            ActivationError::Archive(e) => write!(f, "{e}"),
            ActivationError::Io(e) => write!(f, "{e}"),
            ActivationError::AtomicWrite(e) => write!(f, "{e}"),
            ActivationError::Manifest => write!(f, "interrupted activation manifest is unreadable"),
            ActivationError::Cancelled => write!(f, "activation was cancelled"),
        }
    }
}

impl std::error::Error for ActivationError {}

impl From<io::Error> for ActivationError {
    fn from(e: io::Error) -> Self {
        ActivationError::Io(e)
    }
}

impl From<ArchiveError> for ActivationError {
    fn from(e: ArchiveError) -> Self {
        ActivationError::Archive(e)
    }
}

impl From<AtomicWriteError> for ActivationError {
    fn from(e: AtomicWriteError) -> Self {
        ActivationError::AtomicWrite(e)
    }
}

/// The level-name/seed identity to apply for `slot`, and whether it has
/// a real archive — `inferredWorldLevelName`'s primary branch only
/// (`slot.world_level_name`, trimmed); the Java-only zip-listing
/// fallback that branch also has is narrowed out here since no P6.13
/// fixture exercises activating a legacy-imported, name-less archived
/// slot, and it can be added if a real one turns up. Flagged narrowing,
/// not a silent one.
fn resolve_activation_identity(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    slot: &WorldSlot,
) -> Result<(bool, Option<WorldIdentity>), ActivationError> {
    let has_archive = has_archive(fs, server_dir, &slot.id);
    let stored_level_name = slot
        .world_level_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let will_generate_fresh = !has_archive && stored_level_name.is_some();

    if !has_archive && !will_generate_fresh {
        return Err(ActivationError::NoArchiveOrFreshMetadata);
    }

    let identity = if has_archive {
        stored_level_name.map(|level_name| WorldIdentity {
            level_name: level_name.to_string(),
            seed: None,
            apply_seed: false,
        })
    } else {
        let current_level_name = read_java_level_name(fs, server_dir)
            .unwrap_or_else(|| world::current_level_name(server_type, None));
        let candidate = stored_level_name.unwrap_or(slot.name.as_str());
        Some(WorldIdentity {
            level_name: world::sanitized_world_level_name(candidate, &current_level_name),
            seed: slot.world_seed.clone(),
            apply_seed: true,
        })
    };

    Ok((has_archive, identity))
}

/// `activateSlot(_:for:backupCurrent:logLine:backupWorld:)`, transactional
/// (see the section doc above). `is_server_running` is the caller's
/// already-known process state (`activateWorldSlot`'s guard, folded in
/// here per this file's established pattern); `backup` is called only
/// when live folders currently exist, matching source's own
/// `!currentFolders.isEmpty` condition, and aborts the whole activation
/// before any folder is touched if it returns `false`.
///
/// `should_cancel` (P6.30) is cooperative-cancellation support: polled
/// only at the two boundaries where the live world at the server root
/// has not yet been touched — before the pre-activation backup/staging
/// begins at all, and again once staging (phase 1) has finished but
/// before phase 2 starts moving the current live folders aside. A `true`
/// observed at either point cleans up any scratch staging this call
/// created and returns [`ActivationError::Cancelled`] with the live
/// world completely untouched. Once phase 2 begins, the transaction runs
/// to completion unconditionally — the same "finish the current atomic
/// filesystem action safely" rule [`reconcile_interrupted_activation`]'s
/// own restart recovery already depends on, since an activation that
/// stopped mid-phase-2/3 needs that recovery path regardless of why it
/// stopped.
#[allow(clippy::too_many_arguments)]
pub fn activate_slot(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    slot: &WorldSlot,
    is_server_running: bool,
    now: &str,
    backup: impl FnOnce() -> bool,
    should_cancel: impl Fn() -> bool,
) -> Result<WorldSlot, ActivationError> {
    if is_server_running {
        return Err(ActivationError::ServerRunning);
    }
    if should_cancel() {
        return Err(ActivationError::Cancelled);
    }

    let (has_archive, identity) = resolve_activation_identity(fs, server_dir, server_type, slot)?;

    let current_level_name = read_java_level_name(fs, server_dir)
        .unwrap_or_else(|| world::current_level_name(server_type, None));
    let current_folders = existing_world_folders(fs, server_dir, server_type, &current_level_name);

    if !current_folders.is_empty() && !backup() {
        return if should_cancel() {
            Err(ActivationError::Cancelled)
        } else {
            Err(ActivationError::BackupFailed)
        };
    }

    let manifest_path = activation_manifest_path(server_dir);
    fs.create_dir_all(&activation_dir(server_dir))?;
    let manifest_bytes =
        serde_json::to_vec_pretty(&activation_manifest_value(&slot.id, identity.as_ref()))
            .expect("activation manifest always serializes");
    fs.write(&manifest_path, &manifest_bytes)?;

    // Phase 1: stage the replacement. The live world at the server root
    // is not touched by anything in this block.
    let staged_dir = activation_staged_dir(server_dir);
    if has_archive {
        let zip_path = world_store::zip_path(server_dir, &slot.id);
        if let Err(e) = archive::extract_zip(&zip_path, &staged_dir) {
            let _ = fs.remove(&activation_dir(server_dir));
            return Err(e.into());
        }
        if let Some(identity) = &identity {
            relocate_legacy_bedrock_layout(&staged_dir, &identity.level_name);
        }
    }

    // Last chance to cancel for free: staging is complete but nothing at
    // the server root has been touched yet, so backing out here is just
    // deleting the scratch directory this call itself just created.
    if should_cancel() {
        let _ = fs.remove(&activation_dir(server_dir));
        return Err(ActivationError::Cancelled);
    }

    // Phase 2: move the current live folders aside.
    let prior_dir = activation_prior_dir(server_dir);
    fs.create_dir_all(&prior_dir)?;
    for name in &current_folders {
        fs.rename(&server_dir.join(name), &prior_dir.join(name))?;
    }
    test_pause_after_world_move();

    // Phase 3: install the staged replacement (if any), then commit.
    if has_archive {
        move_entries(fs, &staged_dir, server_dir)?;
        let _ = fs.remove(&staged_dir);
    }

    finish_activation_commit(fs, server_dir, slot, identity.as_ref(), now)
}

/// The tail shared by a normal [`activate_slot`] call and
/// [`reconcile_interrupted_activation`]'s "installed" recovery: apply
/// identity, persist slot metadata (`last_played_at` refreshed,
/// `world_level_name` updated if an identity was applied), persist the
/// active marker, then remove the whole `.activation/` transaction
/// directory. Every one of these is idempotent, so replaying it after a
/// restart is always safe even if some of it already ran before the
/// crash.
fn finish_activation_commit(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot: &WorldSlot,
    identity: Option<&WorldIdentity>,
    now: &str,
) -> Result<WorldSlot, ActivationError> {
    if let Some(identity) = identity {
        apply_world_identity(fs, server_dir, identity)?;
    }

    let mut updated = slot.clone();
    updated.last_played_at = Some(now.to_string());
    if let Some(identity) = identity {
        updated.world_level_name = Some(identity.level_name.clone());
    }
    world_store::save_metadata(fs, server_dir, &updated)?;
    world_store::set_active_slot_id(fs, server_dir, Some(&updated.id))?;

    let _ = fs.remove(&activation_dir(server_dir));
    Ok(updated)
}

/// What [`reconcile_interrupted_activation`] did, if anything, on this
/// call — `None` means there was no in-flight transaction to recover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationRecovery {
    /// Phase 1 ("staged") or phase 2 ("prior_moved") was interrupted —
    /// the live world at the server root is (or has been restored to
    /// be) the complete, unmodified old world.
    RecoveredToOldWorld,
    /// Phase 3 ("installed") was interrupted after the new world was
    /// already moved into place — the commit tail was replayed to
    /// completion.
    RecoveredToNewWorld { slot_id: String },
}

/// Call once per server on agent startup, before any world-mutation
/// route is reachable for it (the same "before routes are reachable"
/// timing [`reconcile_imported_worlds`] already established) — reconciles
/// an [`activate_slot`] call interrupted by a crash/restart to either
/// the complete old world or the complete new world, driven only by
/// which of `.activation/{prior,staged}` physically exist. See the
/// section doc above for the three-phase table this implements.
pub fn reconcile_interrupted_activation(
    fs: &dyn FileSystem,
    server_dir: &Path,
    now: &str,
) -> Result<Option<ActivationRecovery>, ActivationError> {
    let activation_dir = activation_dir(server_dir);
    if fs.stat(&activation_dir).is_err() {
        return Ok(None);
    }

    let prior_dir = activation_prior_dir(server_dir);
    let staged_dir = activation_staged_dir(server_dir);
    let prior_exists = fs.stat(&prior_dir).is_ok();
    let staged_exists = fs.stat(&staged_dir).is_ok();

    if !prior_exists {
        // Phase 1 ("staged"): nothing at the server root was ever
        // touched — discard the abandoned staging area outright.
        let _ = fs.remove(&activation_dir);
        return Ok(Some(ActivationRecovery::RecoveredToOldWorld));
    }

    if staged_exists {
        // Phase 2 ("prior_moved"): the server root currently has no
        // live world at all — move the prior folders back.
        move_entries(fs, &prior_dir, server_dir)?;
        let _ = fs.remove(&activation_dir);
        return Ok(Some(ActivationRecovery::RecoveredToOldWorld));
    }

    // Phase 3 ("installed"): the new world is already at the server
    // root; replay the commit tail to completion.
    let manifest_bytes = fs
        .read(&activation_manifest_path(server_dir))
        .map_err(|_| ActivationError::Manifest)?;
    let (slot_id, identity) =
        parse_activation_manifest(&manifest_bytes).ok_or(ActivationError::Manifest)?;
    let slots = world_store::load_slots(fs, server_dir);
    let slot = slots
        .iter()
        .find(|s| s.id == slot_id)
        .cloned()
        .ok_or(ActivationError::Manifest)?;

    let updated = finish_activation_commit(fs, server_dir, &slot, identity.as_ref(), now)?;
    Ok(Some(ActivationRecovery::RecoveredToNewWorld {
        slot_id: updated.id,
    }))
}
// =====================================================================
// P6.14 — transactional direct world rename and replacement
//
// Ports `AppViewModel+WorldManagement.swift`'s `renameWorld(for:
// newLevelName:backupFirst:)` (source line 178-247) and `replaceWorld(
// for:newLevelName:worldSource:backupFirst:)` (source line 45-152) —
// the *direct* live-folder operations the public compatibility routes
// use (`docs/msc2/worlds/phase6-api.md`'s naming-trap note: these are
// distinct from `rename_slot`'s slot-metadata-only rename above). Both
// share the identically-shaped running-server guard three call sites in
// source re-derive independently (`fixtures/world-mutations/
// activate-refused-while-server-running.json`'s own note); this port
// implements it once ([`WorldError::ServerRunning`], checked first in
// both functions) rather than a third time.
// =====================================================================

/// `renameWorld`'s all-or-nothing move set: Java's three level-name-
/// derived folders, or Bedrock's single `worlds/<level-name>` folder —
/// the same base-directory split `replace_world` also uses.
fn world_base_dir(server_dir: &Path, server_type: ServerType) -> PathBuf {
    match server_type {
        ServerType::Bedrock => server_dir.join("worlds"),
        ServerType::Java => server_dir.to_path_buf(),
    }
}

fn folder_exists(fs: &dyn FileSystem, path: &Path) -> bool {
    matches!(fs.stat(path), Ok(m) if m.is_dir)
}

/// `renameWorld(for:newLevelName:backupFirst:)` (source line 178-247).
/// A no-op success if `new_level_name` already equals the current
/// level-name (source line 187). Otherwise: an all-or-nothing pre-check
/// across every target name before any folder moves
/// (`fixtures/world-mutations/rename-world-target-folder-exists-refused-before-any-move.json`),
/// then a move loop that rolls back every already-moved folder in
/// reverse order on either a mid-sequence move failure or a trailing
/// `server.properties` write failure
/// (`fixtures/world-mutations/rename-world-rollback-on-mid-sequence-move-failure.json`).
/// `backup` is called only when `backup_first` is set, mirroring
/// `activate_slot`'s own backup-hook shape (backups aren't ported until
/// P6.15 — this function takes the safety net as a caller-supplied
/// closure rather than depending on that port directly).
#[allow(clippy::too_many_arguments)]
pub fn rename_world(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    new_level_name: &str,
    is_server_running: bool,
    backup_first: bool,
    backup: impl FnOnce() -> bool,
) -> Result<(), WorldError> {
    let trimmed = new_level_name.trim();
    if trimmed.is_empty() {
        return Err(WorldError::EmptyName);
    }
    if is_server_running {
        return Err(WorldError::ServerRunning);
    }

    let old_level_name = world::current_level_name(server_type, raw_level_name);
    if trimmed == old_level_name {
        return Ok(());
    }

    if backup_first && !backup() {
        return Err(WorldError::BackupFailed);
    }

    let base = world_base_dir(server_dir, server_type);
    let target_names = world::live_world_folder_candidates(server_type, trimmed);
    for name in &target_names {
        if folder_exists(fs, &base.join(name)) {
            return Err(WorldError::TargetFolderExists(name.clone()));
        }
    }

    let old_names = world::live_world_folder_candidates(server_type, &old_level_name);
    let mut moved_pairs: Vec<(PathBuf, PathBuf)> = Vec::new();
    let rollback = |fs: &dyn FileSystem, moved_pairs: &[(PathBuf, PathBuf)]| {
        for (old_path, new_path) in moved_pairs.iter().rev() {
            if folder_exists(fs, new_path) {
                let _ = fs.rename(new_path, old_path);
            }
        }
    };

    for (old_name, new_name) in old_names.iter().zip(target_names.iter()) {
        let old_path = base.join(old_name);
        let new_path = base.join(new_name);
        if !folder_exists(fs, &old_path) {
            continue;
        }
        // `fs.rename` onto an existing path is meant to fail (Unix
        // refuses a directory-over-non-directory rename), but Windows
        // can silently replace a stray file at the destination instead
        // of erroring. Check explicitly so a leftover file is refused
        // and rolled back the same way on every platform.
        if fs.stat(&new_path).is_ok() {
            rollback(fs, &moved_pairs);
            return Err(WorldError::Io(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} already exists", new_path.display()),
            )));
        }
        if let Err(e) = fs.rename(&old_path, &new_path) {
            rollback(fs, &moved_pairs);
            return Err(e.into());
        }
        moved_pairs.push((old_path, new_path));
    }

    let identity = WorldIdentity {
        level_name: trimmed.to_string(),
        seed: None,
        apply_seed: false,
    };
    if let Err(e) = apply_world_identity(fs, server_dir, &identity) {
        rollback(fs, &moved_pairs);
        return Err(e.into());
    }

    Ok(())
}

/// The three ways `replaceWorld`'s `WorldSource` enum can supply a
/// replacement world (source line 8-14 of the same file's `WorldSource`
/// declaration, referenced from `replaceWorld`'s own `switch`).
#[derive(Debug, Clone)]
pub enum WorldReplaceSource {
    /// No source data — the world folders are cleared and a new world
    /// generates on next start.
    Fresh,
    /// A backup ZIP, extracted into place — validated
    /// ([`archive::validate_archive_safety`]'s traversal/symlink/size
    /// checks, P6.33) before anything else is touched.
    BackupZip(PathBuf),
    /// An existing world folder, copied into place under the new
    /// level-name.
    ExistingFolder(PathBuf),
}

fn copy_dir_recursive(fs: &dyn FileSystem, from: &Path, to: &Path) -> io::Result<()> {
    fs.create_dir_all(to)?;
    for entry in top_level_entries(fs, from)? {
        let name = entry
            .file_name()
            .expect("directory listing entries are named");
        let dest = to.join(name);
        if folder_exists(fs, &entry) {
            copy_dir_recursive(fs, &entry, &dest)?;
        } else {
            copy_via_fs(fs, &entry, &dest)?;
        }
    }
    Ok(())
}

// =====================================================================
// P6.33 — make active-world replacement transactional
//
// `replaceWorld(for:newLevelName:worldSource:backupFirst:)` (source line
// 45-152) removed the live world folders *before* installing the new
// source, with only the (also caller-optional) safety backup as a manual
// recovery path if installation then failed — flagged as baseline parity
// at P6.14, not a correction, since `phase6-scope.md` hadn't named this
// window yet. The Phase 6 gate review did: this is the exact
// remove-then-copy shape `activate_slot` (P6.13) and `restore_backup`
// (P6.18) were already corrected away from, so [`replace_world`] gets
// the identical three-phase on-disk transaction, under
// `world_slots/.replace/{manifest.json,staged/,prior/}`, plus a
// *mandatory* (no longer caller-optional) verified safety backup —
// matching `restore_backup`'s own unconditional pre-restore backup
// rather than `rename_world`'s caller-supplied `backup_first` flag,
// since "a safety backup alone is not a substitute for automatic
// rollback/reconciliation" is this correction's own point: both now
// exist together.
//
//   1. **staged** — the replacement source (a validated backup ZIP, an
//      existing folder, or nothing at all for a fresh world) is fully
//      staged into `.replace/staged/`. The live world is untouched. A
//      failure here (a corrupt archive, an unreadable source folder)
//      aborts with the live world completely intact — the safety backup
//      has already been secured either way.
//   2. **prior_moved** — the current live folders (Java's full
//      main/nether/end set, or Bedrock's single folder — the same set
//      source removed outright) are moved, not deleted, into
//      `.replace/prior/`.
//   3. **installed** — the staged replacement is moved into place,
//      `staged/` is removed, the new level-name is committed to
//      `server.properties`, then `.replace/` itself is removed last.
//
// The three phases are distinguished purely by which of
// `.replace/{prior,staged}` physically exist — the same journal-free
// recovery shape `activate_slot`/`restore_backup` already use — so
// [`reconcile_interrupted_world_replace`] always resolves an interrupted
// transaction to either the complete old world or the complete new one.
// `manifest.json` (just the new level-name) is the one piece phase-3
// recovery can't re-derive from the directory layout alone.
// =====================================================================

fn replace_dir(server_dir: &Path) -> PathBuf {
    world_store::slots_directory(server_dir).join(".replace")
}

fn replace_manifest_path(server_dir: &Path) -> PathBuf {
    replace_dir(server_dir).join("manifest.json")
}

fn replace_staged_dir(server_dir: &Path) -> PathBuf {
    replace_dir(server_dir).join("staged")
}

fn replace_prior_dir(server_dir: &Path) -> PathBuf {
    replace_dir(server_dir).join("prior")
}

fn parse_replace_manifest(bytes: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    value.get("level_name")?.as_str().map(str::to_string)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplaceOutcome {
    /// The mandatory pre-replace safety backup's own path — created only
    /// when live world folders existed to protect, the same
    /// `!current_folders.is_empty()` gate [`activate_slot`]'s own backup
    /// hook already uses. `None` when there was nothing yet to back up
    /// (a first-time replace against a server with no world yet).
    pub safety_backup_zip_path: Option<PathBuf>,
}

/// The tail shared by a normal [`replace_world`] call and
/// [`reconcile_interrupted_world_replace`]'s "installed" recovery:
/// commit the new level-name to `server.properties`, then remove the
/// whole `.replace/` transaction directory. Both steps are idempotent,
/// so replaying this after a restart is always safe even if the identity
/// write already happened before the crash.
fn finish_replace_commit(
    fs: &dyn FileSystem,
    server_dir: &Path,
    level_name: &str,
) -> Result<(), WorldError> {
    let identity = WorldIdentity {
        level_name: level_name.to_string(),
        seed: None,
        apply_seed: false,
    };
    apply_world_identity(fs, server_dir, &identity)?;
    let _ = fs.remove(&replace_dir(server_dir));
    Ok(())
}

/// `replaceWorld(for:newLevelName:worldSource:backupFirst:)`,
/// transactional (see the section doc above). Guard order matches
/// source: empty name, then running-server, then source validation
/// (a backup ZIP source now runs [`archive::validate_archive_safety`] —
/// the same D-006 traversal/symlink/zip-bomb check `restore_backup`
/// already gates on — rather than source's own bare structural-open
/// check). `should_cancel` (P6.30) is polled at the same two
/// "nothing at the live world touched yet" boundaries `activate_slot`/
/// `restore_backup` already use: before the mandatory safety backup
/// begins at all, and again once staging has finished but before the
/// live folders move. Once phase 2 begins, the transaction runs to
/// completion unconditionally, matching every other P6.30 worker.
#[allow(clippy::too_many_arguments)]
pub fn replace_world(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    new_level_name: &str,
    world_source: &WorldReplaceSource,
    is_server_running: bool,
    safety_backup_association: &BackupAssociation,
    safety_backup_server_id: Option<&str>,
    safety_backup_server_display_name: Option<&str>,
    now: &str,
    should_cancel: impl Fn() -> bool,
) -> Result<WorldReplaceOutcome, WorldError> {
    let trimmed = new_level_name.trim();
    if trimmed.is_empty() {
        return Err(WorldError::EmptyName);
    }
    if is_server_running {
        return Err(WorldError::ServerRunning);
    }

    match world_source {
        WorldReplaceSource::Fresh => {}
        WorldReplaceSource::BackupZip(path) => {
            if archive::validate_archive_safety(path).is_err() {
                return Err(WorldError::InvalidWorldSource);
            }
        }
        WorldReplaceSource::ExistingFolder(path) => {
            if !folder_exists(fs, path) {
                return Err(WorldError::InvalidWorldSource);
            }
        }
    }

    if should_cancel() {
        return Err(WorldError::Cancelled);
    }

    // `world_base_dir`/`live_world_folder_candidates` — the same base and
    // candidate-name computation `rename_world` uses — decide both which
    // live folders exist to protect and which ones phase 2 moves aside.
    let base = world_base_dir(server_dir, server_type);
    let current_level_name = world::current_level_name(server_type, raw_level_name);
    let current_names = world::live_world_folder_candidates(server_type, &current_level_name);
    let current_folders_exist = current_names
        .iter()
        .any(|name| folder_exists(fs, &base.join(name)));

    let safety_backup_zip_path = if current_folders_exist {
        let result = crate::backups::create_backup(
            fs,
            server_dir,
            server_type,
            raw_level_name,
            safety_backup_association,
            safety_backup_server_id,
            safety_backup_server_display_name,
            false,
            false,
            Some("pre-replace"),
            None,
            now,
            None,
            || false,
            &should_cancel,
        );
        let result = match result {
            Ok(result) => result,
            Err(crate::backups::BackupError::Cancelled) => return Err(WorldError::Cancelled),
            Err(error) => return Err(WorldError::SafetyBackupFailed(error)),
        };
        Some(result.zip_path)
    } else {
        None
    };

    fs.create_dir_all(&replace_dir(server_dir))?;
    let manifest_bytes = serde_json::to_vec_pretty(&serde_json::json!({ "level_name": trimmed }))
        .expect("replace manifest always serializes");
    fs.write(&replace_manifest_path(server_dir), &manifest_bytes)?;

    // Phase 1: stage the replacement. The live world is not touched by
    // anything in this block.
    let staged_dir = replace_staged_dir(server_dir);
    let staged_base = world_base_dir(&staged_dir, server_type);
    fs.create_dir_all(&staged_base)?;
    match world_source {
        WorldReplaceSource::Fresh => {}
        WorldReplaceSource::BackupZip(path) => {
            if let Err(e) = archive::extract_zip(path, &staged_dir) {
                let _ = fs.remove(&replace_dir(server_dir));
                return Err(e.into());
            }
        }
        WorldReplaceSource::ExistingFolder(source_path) => {
            let dest = staged_base.join(trimmed);
            if let Err(e) = copy_dir_recursive(fs, source_path, &dest) {
                let _ = fs.remove(&replace_dir(server_dir));
                return Err(e.into());
            }
        }
    }

    // Last chance to cancel for free: staging is complete but nothing at
    // the live world has been touched yet.
    if should_cancel() {
        let _ = fs.remove(&replace_dir(server_dir));
        return Err(WorldError::Cancelled);
    }

    // Phase 2: move the current live folders aside.
    let prior_dir = replace_prior_dir(server_dir);
    fs.create_dir_all(&prior_dir)?;
    for name in &current_names {
        let path = base.join(name);
        if folder_exists(fs, &path) {
            fs.rename(&path, &prior_dir.join(name))?;
        }
    }
    test_pause_after_world_move();

    // Phase 3: install the staged replacement (if any), then commit.
    move_entries(fs, &staged_base, &base)?;
    let _ = fs.remove(&staged_dir);
    finish_replace_commit(fs, server_dir, trimmed)?;

    Ok(WorldReplaceOutcome {
        safety_backup_zip_path,
    })
}

/// What [`reconcile_interrupted_world_replace`] did, if anything, on this
/// call — mirrors `ActivationRecovery`/`RestoreRecovery`'s own shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldReplaceRecovery {
    /// Phase 1 ("staged") or phase 2 ("prior_moved") was interrupted —
    /// the live world is (or has been restored to be) the complete,
    /// unmodified pre-replace world. The safety backup this attempt
    /// created (if any) is still on disk either way.
    RecoveredToOldWorld,
    /// Phase 3 ("installed") was interrupted after the new world was
    /// already moved into place — the commit tail was replayed to
    /// completion.
    RecoveredToNewWorld,
}

/// Call once per server on agent startup, before any world-replace route
/// is reachable for it — the same "before routes are reachable" timing
/// [`reconcile_interrupted_activation`]/`reconcile_interrupted_restore`
/// already establish. Driven purely by which of `.replace/{prior,staged}`
/// physically exist, per the section doc's own three-phase table.
pub fn reconcile_interrupted_world_replace(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
) -> Result<Option<WorldReplaceRecovery>, WorldError> {
    let dir = replace_dir(server_dir);
    if fs.stat(&dir).is_err() {
        return Ok(None);
    }

    let prior_dir = replace_prior_dir(server_dir);
    let staged_dir = replace_staged_dir(server_dir);
    let prior_exists = fs.stat(&prior_dir).is_ok();
    let staged_exists = fs.stat(&staged_dir).is_ok();

    if !prior_exists {
        // Phase 1 ("staged"): nothing at the live world was ever
        // touched — discard the abandoned staging area outright.
        let _ = fs.remove(&dir);
        return Ok(Some(WorldReplaceRecovery::RecoveredToOldWorld));
    }

    if staged_exists {
        // Phase 2 ("prior_moved"): the live world currently has nothing
        // at it — move the prior folders back.
        let base = world_base_dir(server_dir, server_type);
        move_entries(fs, &prior_dir, &base)?;
        let _ = fs.remove(&dir);
        return Ok(Some(WorldReplaceRecovery::RecoveredToOldWorld));
    }

    // Phase 3 ("installed"): the new world is already in place; replay
    // the commit tail (apply identity, remove `.replace/`).
    let manifest_bytes = fs.read(&replace_manifest_path(server_dir))?;
    let level_name = parse_replace_manifest(&manifest_bytes).ok_or(WorldError::Manifest)?;
    finish_replace_commit(fs, server_dir, &level_name)?;
    Ok(Some(WorldReplaceRecovery::RecoveredToNewWorld))
}
