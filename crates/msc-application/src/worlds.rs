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
use msc_domain::world::{self, WorldSlot};
use msc_infrastructure::archive::{self, ArchiveError};
use msc_infrastructure::atomic_write::AtomicWriteError;
use msc_infrastructure::download_staging::sha1_hex;
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::world_store;
use std::collections::BTreeMap;
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
