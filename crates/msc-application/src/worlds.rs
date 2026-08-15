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
use msc_domain::world::{self, WorldSlot};
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

/// The candidate-name half already lives in `msc_domain::world`
/// (`world_folder_candidates`); this is the existence-filtering half
/// `WorldSlotManager.worldFolderNames(for:)` mixes into the same
/// function in source, kept separate here per the module-boundary split
/// P6.9 already established.
fn existing_world_folders(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    level_name: &str,
) -> Vec<String> {
    world::world_folder_candidates(server_type, level_name)
        .into_iter()
        .filter(|name| matches!(fs.stat(&server_dir.join(name)), Ok(meta) if meta.is_dir))
        .collect()
}

/// `server.properties`' `level-name` value, for Java servers only — no
/// fixture in this domain names a Bedrock case (Bedrock's own runtime
/// stays unavailable until Phase 10 per this phase's own deferral), so
/// this reads only the one properties file every P6.11 fixture actually
/// needs. Flagged narrowing, not a silent one.
fn read_java_level_name(fs: &dyn FileSystem, server_dir: &Path) -> Option<String> {
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
            archive::extract_zip(&zip_path, server_dir).map_err(ReconciliationError::Archive)?;
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
/// Covers both server types via [`world::world_folder_candidates`]
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
