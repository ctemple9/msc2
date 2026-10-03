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
use msc_domain::properties::LevelType;
use msc_domain::world::{self, BackupAssociation, WorldSlot};
use msc_domain::world_profile::{
    SettingApplyPolicy, WorldGameplay as ProfileGameplay, WorldGeneration as ProfileGeneration,
    WorldIdentity as ProfileIdentity, WorldProfile, WorldProfileField,
    WorldSafety as ProfileSafety, WorldSafetyState,
};
use msc_infrastructure::archive::{self, ArchiveError};
use msc_infrastructure::atomic_write::AtomicWriteError;
use msc_infrastructure::download_staging::sha1_hex;
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::metrics::directory_size_mb;
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

pub(crate) fn mark_fresh_world_reconciled(
    fs: &dyn FileSystem,
    server_dir: &Path,
) -> io::Result<()> {
    // Creation already owns the initial slot and any prepared data packs.
    // Import recovery must not turn that pre-generation folder into a new
    // world slot. Call only after the creation profile and active ID are saved.
    fs.write(&reconciliation_marker_path(server_dir), b"1")
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

/// The shared `server.properties` `level-name` value. Java and Bedrock use
/// the same key, but Bedrock's `worlds/<level-name>/` directory makes it
/// especially important that callers do not silently fall back to the
/// generic Bedrock name when a real server directory already declares one.
pub fn read_configured_level_name(fs: &dyn FileSystem, server_dir: &Path) -> Option<String> {
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

/// Compatibility-named Java entry point retained for callers that already
/// know they are handling a Java server. The file format is shared with
/// Bedrock; [`read_configured_level_name`] is the type-neutral entry point.
pub fn read_java_level_name(fs: &dyn FileSystem, server_dir: &Path) -> Option<String> {
    read_configured_level_name(fs, server_dir)
}

/// Measures the world the server is configured to load, not an assumed
/// `world/` directory or archived world-slot ZIPs. Java stores its main,
/// Nether, and End folders beside the server properties; Bedrock stores
/// its active level under `worlds/<level-name>`.
pub fn active_world_size_mb(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
) -> Option<f64> {
    let configured_level_name = read_configured_level_name(fs, server_dir);
    let level_name = world::current_level_name(server_type, configured_level_name.as_deref());
    let world_base = world_base_dir(server_dir, server_type);
    let mut total_mb = 0.0;
    let mut found_world_folder = false;

    for folder in world::live_world_folder_candidates(server_type, &level_name) {
        match directory_size_mb(&world_base.join(folder)) {
            Ok(size_mb) => {
                total_mb += size_mb;
                found_world_folder = true;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return None,
        }
    }

    found_world_folder.then_some(total_mb)
}

fn resolved_level_name(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
) -> String {
    let configured = if raw_level_name.is_none() {
        read_configured_level_name(fs, server_dir)
    } else {
        None
    };
    world::current_level_name(server_type, raw_level_name.or(configured.as_deref()))
}

/// Builds the profile for a newly-created world. These values are explicit
/// user choices, so they are stored as configured rather than reported as
/// detected from a world file that does not exist yet.
pub fn fresh_world_profile(
    slot: &WorldSlot,
    difficulty: Option<&str>,
    gamemode: Option<&str>,
) -> WorldProfile {
    let mut profile = WorldProfile::new();
    profile.identity = ProfileIdentity {
        name: Some(slot.name.clone()),
        level_name: slot.world_level_name.clone(),
        seed: slot.world_seed.clone(),
    };
    profile.gameplay = ProfileGameplay {
        difficulty: normalized_profile_string(difficulty),
        default_game_mode: normalized_profile_string(gamemode),
        ..ProfileGameplay::default()
    };
    profile.safety = ProfileSafety {
        state: WorldSafetyState::Safe,
        reasons: Vec::new(),
    };
    profile
}

/// Builds a profile from values actually recovered from a Java or Bedrock
/// level.dat. Missing values remain `None`; the safety state makes a failed
/// read visible instead of allowing callers to mistake a default for fact.
pub fn detected_profile(
    slot: &WorldSlot,
    server_type: ServerType,
    metadata: &nbt::ImportedWorldMetadata,
) -> WorldProfile {
    let mut profile = WorldProfile::new();
    profile.identity = ProfileIdentity {
        name: Some(slot.name.clone()),
        level_name: slot.world_level_name.clone(),
        seed: metadata.seed.clone().or_else(|| slot.world_seed.clone()),
    };
    profile.generation = ProfileGeneration {
        world_type: metadata.world_type.clone(),
        flat_preset: metadata.flat_preset.clone(),
        structures: metadata.structures,
        biome_source: metadata.biome_source.clone(),
        generator_options: metadata.generator_options.clone(),
        bonus_chest: metadata.bonus_chest,
        data_packs: metadata.data_packs.clone(),
    };
    profile.gameplay = ProfileGameplay {
        difficulty: metadata.difficulty.clone(),
        default_game_mode: metadata.gamemode.clone(),
        hardcore: metadata.hardcore,
        commands: metadata.commands,
        gamerules: metadata.gamerules.clone(),
        cheats: metadata.cheats,
        experiments: metadata.experiments.clone(),
        coordinates: metadata.coordinates,
        starting_map: metadata.starting_map,
        supported_toggles: metadata.supported_toggles.clone(),
    };
    let (state, reasons) = if !metadata.parsed {
        (
            WorldSafetyState::Unknown,
            vec!["The world's level.dat could not be read.".to_string()],
        )
    } else if server_type == ServerType::Bedrock && metadata.cheats == Some(true) {
        (
            WorldSafetyState::AchievementDisabled,
            vec!["Bedrock cheats are enabled; achievements are disabled.".to_string()],
        )
    } else {
        (WorldSafetyState::Safe, Vec::new())
    };
    profile.safety = ProfileSafety { state, reasons };
    profile
}

fn normalized_profile_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn metadata_from_live_world(
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
) -> nbt::ImportedWorldMetadata {
    let level_name = resolved_level_name_from_disk(server_dir, server_type, raw_level_name);
    let level_dat = match server_type {
        ServerType::Java => server_dir.join(&level_name).join("level.dat"),
        ServerType::Bedrock => server_dir
            .join("worlds")
            .join(&level_name)
            .join("level.dat"),
    };
    std::fs::read(level_dat)
        .ok()
        .map(|bytes| nbt::imported_world_metadata_from_level_dat(&bytes, server_type))
        .unwrap_or_default()
}

fn resolved_level_name_from_disk(
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
) -> String {
    if let Some(raw) = raw_level_name {
        return world::current_level_name(server_type, Some(raw));
    }
    let configured = std::fs::read(server_dir.join("server.properties"))
        .ok()
        .and_then(|bytes| {
            String::from_utf8_lossy(&bytes).lines().find_map(|line| {
                line.trim().split_once('=').and_then(|(key, value)| {
                    (key.trim() == "level-name").then(|| value.trim().to_string())
                })
            })
        });
    world::current_level_name(server_type, configured.as_deref())
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

    let metadata = metadata_from_live_world(server_dir, server_type, raw_level_name);
    let profile = detected_profile(&slot, server_type, &metadata);
    world_store::save_profile(fs, server_dir, &slot, &profile)
        .map_err(ReconciliationError::AtomicWrite)?;
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
/// `raw_level_name` is the already-read `server.properties` `level-name`;
/// callers may pass `None` to have this function read it itself from the
/// shared properties file, or supply an already-read value.
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

    let owned_level_name = if raw_level_name.is_none() {
        read_configured_level_name(fs, server_dir)
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
            let approved_roots = archive::validate_world_archive(&zip_path, server_type)
                .map_err(ReconciliationError::Archive)?;
            let staged_dir = reconciliation_staged_dir(server_dir);
            let _ = fs.remove(&staged_dir);
            if let Err(e) = archive::extract_zip(&zip_path, &staged_dir) {
                let _ = fs.remove(&staged_dir);
                return Err(ReconciliationError::Archive(e));
            }
            if server_type == ServerType::Bedrock {
                let level_name = slot.world_level_name.as_deref().unwrap_or(&level_name);
                relocate_legacy_bedrock_layout(&staged_dir, level_name)
                    .map_err(ReconciliationError::Io)?;
            }
            // The live-folder location is not touched until every entry
            // has already extracted successfully into `staged_dir`.
            if let Err(e) = move_approved_world_roots(fs, &staged_dir, server_dir, &approved_roots)
            {
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

/// Packs selected before terrain generation live separately from a saved world.
/// Keeping them out of world.zip preserves fresh-world seed/generation semantics.
pub fn world_pack_archive_path(server_dir: &Path, slot_id: &str) -> PathBuf {
    let world = world_store::zip_path(server_dir, slot_id);
    if world.is_file() {
        world
    } else {
        world.with_file_name("packs.zip")
    }
}

pub fn prepare_world_pack_archive(
    server_dir: &Path,
    server_type: ServerType,
    slot: &WorldSlot,
) -> Result<PathBuf, WorldError> {
    let path = world_pack_archive_path(server_dir, &slot.id);
    if path.is_file() {
        return Ok(path);
    }
    let profile =
        world_store::load_profile(&msc_infrastructure::fs::StdFileSystem, server_dir, slot);
    let level = profile
        .identity
        .level_name
        .as_deref()
        .or(slot.world_level_name.as_deref())
        .ok_or(WorldError::NoArchiveOrFreshMetadata)?;
    if !safe_world_folder_name(level) {
        return Err(WorldError::InvalidWorldSource);
    }
    let root = if server_type == ServerType::Java {
        format!("{level}/")
    } else {
        format!("worlds/{level}/")
    };
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    let mut zip = zip::ZipWriter::new(file);
    zip.add_directory(root, zip::write::SimpleFileOptions::default())
        .map_err(|error| io::Error::other(error.to_string()))?;
    zip.finish()
        .map_err(|error| io::Error::other(error.to_string()))?
        .sync_all()?;
    Ok(path)
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
    let configured_level_name = if raw_level_name.is_none() {
        read_configured_level_name(fs, server_dir)
    } else {
        None
    };
    let raw_level_name = raw_level_name.or(configured_level_name.as_deref());
    let level_name = world::current_level_name(server_type, raw_level_name);
    let folders = existing_world_folders(fs, server_dir, server_type, &level_name);
    if folders.is_empty() {
        return Err(WorldError::NoWorldFolders);
    }

    let imported_metadata = metadata_from_live_world(server_dir, server_type, Some(&level_name));

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

    let profile = detected_profile(&slot, server_type, &imported_metadata);
    if let Err(e) = world_store::save_profile(fs, server_dir, &slot, &profile) {
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
    update_active_slot_from_current_world_with_progress(
        fs,
        server_dir,
        server_type,
        raw_level_name,
        slot,
        None,
    )
}

/// Saves the outgoing slot with optional measured compression progress.
pub fn update_active_slot_from_current_world_with_progress(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    slot: &WorldSlot,
    progress: Option<&mut dyn FnMut(u64, u64)>,
) -> Result<WorldSlot, WorldError> {
    let configured_level_name = if raw_level_name.is_none() {
        read_configured_level_name(fs, server_dir)
    } else {
        None
    };
    let raw_level_name = raw_level_name.or(configured_level_name.as_deref());
    let level_name = world::current_level_name(server_type, raw_level_name);
    let folders = existing_world_folders(fs, server_dir, server_type, &level_name);
    if folders.is_empty() {
        return Err(WorldError::NoWorldFolders);
    }
    let imported_metadata = metadata_from_live_world(server_dir, server_type, Some(&level_name));

    let dir = world_store::slot_directory(server_dir, &slot.id);
    fs.create_dir_all(&dir)?;
    let temp_zip = dir.join("world.update.tmp.zip");
    let _ = fs.remove(&temp_zip);

    let creation = match progress {
        Some(progress) => archive::create_zip_from_folders_with_progress(
            &temp_zip,
            server_dir,
            &folders,
            || false,
            progress,
        ),
        None => archive::create_zip_from_folders(&temp_zip, server_dir, &folders),
    };
    if let Err(e) = creation {
        let _ = fs.remove(&temp_zip);
        return Err(e.into());
    }

    let zip_path = world_store::zip_path(server_dir, &slot.id);
    let _ = fs.remove(&zip_path);
    fs.rename(&temp_zip, &zip_path)?;

    let mut updated = slot.clone();
    updated.world_level_name = Some(level_name);
    updated.zip_size_bytes = zip_size_bytes(fs, &zip_path);

    let mut profile = world_store::load_profile(fs, server_dir, slot);
    let detected = detected_profile(&updated, server_type, &imported_metadata);
    // Saved choices remain authoritative, including changes awaiting a restart.
    macro_rules! fill_missing {
        ($($section:ident.$field:ident),+ $(,)?) => { $(
            if profile.$section.$field.is_none() { profile.$section.$field = detected.$section.$field.clone(); }
        )+ };
    }
    fill_missing!(
        identity.seed,
        identity.level_name,
        generation.world_type,
        generation.flat_preset,
        generation.structures,
        generation.biome_source,
        generation.generator_options,
        generation.bonus_chest,
        gameplay.difficulty,
        gameplay.default_game_mode,
        gameplay.hardcore,
        gameplay.commands,
        gameplay.cheats,
        gameplay.coordinates,
        gameplay.starting_map
    );
    profile.safety = detected.safety;
    for pack in detected.packs {
        if !profile.packs.iter().any(|saved| saved.id == pack.id) {
            profile.packs.push(pack);
        }
    }
    world_store::save_profile(fs, server_dir, &updated, &profile)?;
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
    let mut profile = world_store::load_profile(fs, server_dir, slot);
    profile.identity.name = Some(trimmed.to_string());
    world_store::save_profile(fs, server_dir, &updated, &profile)?;
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
    if let Err(e) = world_store::copy_profile(fs, server_dir, source, server_dir, &new_slot) {
        let _ = fs.remove(&new_dir);
        return Err(e.into());
    }
    Ok(new_slot)
}

/// `copySlotIntoExisting(_:into:for:logLine:)` (source line 875-937):
/// destructive by design (overwrites `destination`'s world data), but
/// never touches `destination`'s real archive until the source has
/// already been copied into a scratch file inside `destination`'s own
/// slot directory. The old archive stays under a unique recovery name until
/// both the archive swap and the combined slot/profile metadata write succeed;
/// a failed metadata write restores the old archive before returning.
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
    let old_archive = match fs.stat(&dest_zip) {
        Ok(metadata) if metadata.is_file => {
            let recovery_zip =
                dest_dir.join(format!("world.replace.{}.rollback.zip", Uuid::new_v4()));
            if let Err(error) = fs.rename(&dest_zip, &recovery_zip) {
                let _ = fs.remove(&temp_zip);
                return Err(error.into());
            }
            Some(recovery_zip)
        }
        Ok(_) => {
            let _ = fs.remove(&temp_zip);
            return Err(io::Error::other("destination world archive is not a file").into());
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            let _ = fs.remove(&temp_zip);
            return Err(error.into());
        }
    };

    if let Err(error) = fs.rename(&temp_zip, &dest_zip) {
        if let Some(recovery_zip) = &old_archive
            && let Err(rollback_error) = fs.rename(recovery_zip, &dest_zip)
        {
            return Err(io::Error::other(format!(
                "could not install copied world ({error}); prior archive remains at {} because rollback failed ({rollback_error})",
                recovery_zip.display()
            ))
            .into());
        }
        let _ = fs.remove(&temp_zip);
        return Err(error.into());
    }

    let mut updated = destination.clone();
    updated.created_at = now.to_string();
    updated.world_level_name = source.world_level_name.clone();
    updated.world_seed = source.world_seed.clone();
    updated.zip_size_bytes = zip_size_bytes(fs, &dest_zip);

    let source_profile = world_store::load_profile_value(fs, server_dir, source);
    if let Err(error) = world_store::save_profile_value(fs, server_dir, &updated, &source_profile) {
        if let Err(remove_error) = fs.remove(&dest_zip) {
            if let Some(recovery_zip) = &old_archive {
                return match fs.rename(recovery_zip, &dest_zip) {
                    Ok(()) => Err(io::Error::other(format!(
                        "could not save copied slot metadata ({error}); removing the new archive failed ({remove_error}), but the prior archive was restored"
                    ))
                    .into()),
                    Err(rollback_error) => Err(io::Error::other(format!(
                        "could not save copied slot metadata ({error}); new archive removal failed ({remove_error}); prior archive remains at {} because rollback failed ({rollback_error})",
                        recovery_zip.display()
                    ))
                    .into()),
                };
            }
            return Err(io::Error::other(format!(
                "could not save copied slot metadata ({error}); new archive could not be removed ({remove_error})"
            ))
            .into());
        }

        if let Some(recovery_zip) = &old_archive
            && let Err(rollback_error) = fs.rename(recovery_zip, &dest_zip)
        {
            return Err(io::Error::other(format!(
                "could not save copied slot metadata ({error}); prior archive remains at {} because rollback failed ({rollback_error})",
                recovery_zip.display()
            ))
            .into());
        }
        return Err(error.into());
    }

    if let Some(recovery_zip) = old_archive {
        let _ = fs.remove(&recovery_zip);
    }
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
    let profile = world_store::load_profile_value(fs, server_dir, slot);
    if let Err(error) = archive::copy_with_world_profile(&source_zip, destination_path, &profile) {
        let _ = fs.remove(destination_path);
        return Err(error.into());
    }
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
pub(crate) fn read_sidecar_world_seed(zip_path: &Path) -> Option<String> {
    let sidecar = zip_path.with_extension("meta.json");
    let bytes = std::fs::read(sidecar).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    nbt::trimmed_sidecar_seed(value.get("worldSeed").and_then(|v| v.as_str()))
}

fn read_sidecar_world_profile(zip_path: &Path) -> Option<serde_json::Value> {
    let sidecar = zip_path.with_extension("meta.json");
    std::fs::read(sidecar)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|value| value.get("worldProfile").cloned())
        .or_else(|| archive::read_world_profile(zip_path).ok().flatten())
}

/// Reads world metadata from an archive without changing the archive. The
/// sidecar seed retains MSC 1's precedence rule; all other values come from
/// the first usable level.dat member.
pub(crate) fn imported_world_metadata_from_zip(
    zip_path: &Path,
    server_type: ServerType,
) -> nbt::ImportedWorldMetadata {
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
    nbt::merge_sidecar_metadata(sidecar_seed, parsed)
}

/// `createSlotFromZIP(zipURL:name:for:logLine:)` (source line 1008-
/// 1077): imports the external ZIP into a new slot. P16.4 adds a
/// world-layout check. External folder wrappers and packaging metadata are
/// normalized in the new slot archive; activation retains its strict layout
/// and safety checks. The original archive is never modified.
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

    let imported_level_name =
        match archive::normalize_world_import(source_zip_path, &dest_zip, server_type) {
            Ok(level_name) => level_name,
            Err(e) => {
                let _ = fs.remove(&dir);
                return Err(e.into());
            }
        };

    let parsed_metadata = imported_world_metadata_from_zip(&dest_zip, server_type);
    let slot = WorldSlot {
        id: new_id,
        name: trimmed.to_string(),
        created_at: now.to_string(),
        last_played_at: None,
        thumbnail_file_name: None,
        world_level_name: match server_type {
            ServerType::Java => infer_java_level_name_from_zip(&dest_zip),
            ServerType::Bedrock => Some(imported_level_name.unwrap_or_else(|| {
                resolved_level_name(fs, server_dir, server_type, raw_level_name)
            })),
        },
        world_seed: parsed_metadata.seed.clone(),
        zip_size_bytes: zip_size_bytes(fs, &dest_zip),
    };

    if let Err(e) = world_store::save_metadata(fs, server_dir, &slot) {
        let _ = fs.remove(&dir);
        return Err(e.into());
    }
    let profile = read_sidecar_world_profile(source_zip_path).unwrap_or_else(|| {
        world_store::profile_value(&detected_profile(&slot, server_type, &parsed_metadata))
    });
    // The sidecar profile is copied as raw JSON when present, preserving
    // fields from a newer agent. A normal world ZIP has no profile sidecar,
    // so imported level.dat values receive explicit detected/unknown state.
    if let Err(e) = world_store::save_profile_value(fs, server_dir, &slot, &profile) {
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
// The replacement is staged before touching live folders. A durable
// `swap.json` records the exact old and new folder names and the current
// move phase before each batch of renames. Recovery rolls back partial
// moves, or finishes a fully installed world and its metadata commit.
// An ambiguous or missing folder leaves the transaction in place for
// repair; the agent keeps world mutation unavailable meanwhile.
// =====================================================================
/// `ServerPropertiesManager.readProperties`/
/// `BedrockPropertiesManager.readRawProperties`'s shared parse shape —
/// both are plain `key=value` text files at this level, so one reader
/// serves both server types (the type-specific halves never actually
/// diverge in shape, only in which file they read).
pub(crate) fn read_properties_map(fs: &dyn FileSystem, path: &Path) -> BTreeMap<String, String> {
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
pub(crate) fn write_properties_map(
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

pub(crate) fn apply_world_identity(
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

/// The result of projecting a saved world profile onto the active runtime.
/// The profile remains the source of truth; this report says what the
/// runtime-facing files accepted now and what must wait for a later lifecycle
/// boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldProfileApplyStatus {
    Live,
    PendingRestart,
    Blocked,
}

impl WorldProfileApplyStatus {
    pub const fn raw_value(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::PendingRestart => "pending_restart",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldProfileChange {
    pub key: String,
    pub status: WorldProfileApplyStatus,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorldProfileApplicationReport {
    pub changes: Vec<WorldProfileChange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldProfileApplyContext {
    Creation,
    Activation,
}

fn profile_field_present(profile: &WorldProfile, field: WorldProfileField) -> bool {
    match field {
        WorldProfileField::IdentityName => profile.identity.name.is_some(),
        WorldProfileField::IdentityLevelName => profile.identity.level_name.is_some(),
        WorldProfileField::IdentitySeed => profile.identity.seed.is_some(),
        WorldProfileField::GenerationWorldType => profile.generation.world_type.is_some(),
        WorldProfileField::GenerationFlatPreset => profile.generation.flat_preset.is_some(),
        WorldProfileField::GenerationStructures => profile.generation.structures.is_some(),
        WorldProfileField::GenerationBiomeSource => profile.generation.biome_source.is_some(),
        WorldProfileField::GenerationGeneratorOptions => {
            profile.generation.generator_options.is_some()
        }
        WorldProfileField::GenerationBonusChest => profile.generation.bonus_chest.is_some(),
        WorldProfileField::GenerationDataPacks => !profile.generation.data_packs.is_empty(),
        WorldProfileField::GameplayDifficulty => profile.gameplay.difficulty.is_some(),
        WorldProfileField::GameplayDefaultGameMode => profile.gameplay.default_game_mode.is_some(),
        WorldProfileField::GameplayHardcore => profile.gameplay.hardcore.is_some(),
        WorldProfileField::GameplayCommands => profile.gameplay.commands.is_some(),
        WorldProfileField::GameplayGamerules => !profile.gameplay.gamerules.is_empty(),
        WorldProfileField::GameplayCheats => profile.gameplay.cheats.is_some(),
        WorldProfileField::GameplayExperiments => !profile.gameplay.experiments.is_empty(),
        WorldProfileField::GameplayCoordinates => profile.gameplay.coordinates.is_some(),
        WorldProfileField::GameplayStartingMap => profile.gameplay.starting_map.is_some(),
        WorldProfileField::GameplaySupportedToggles => {
            !profile.gameplay.supported_toggles.is_empty()
        }
        WorldProfileField::SafetyState => false,
    }
}

fn profile_property_value(
    profile: &WorldProfile,
    server_type: ServerType,
    context: WorldProfileApplyContext,
    field: WorldProfileField,
) -> Option<(&'static str, String)> {
    match field {
        WorldProfileField::IdentityLevelName => profile
            .identity
            .level_name
            .clone()
            .map(|value| ("level-name", value)),
        WorldProfileField::IdentitySeed if context == WorldProfileApplyContext::Creation => profile
            .identity
            .seed
            .clone()
            .map(|value| ("level-seed", value)),
        WorldProfileField::GenerationWorldType if context == WorldProfileApplyContext::Creation => {
            profile.generation.world_type.clone().map(|value| {
                (
                    "level-type",
                    LevelType::from_legacy_or_namespaced(&value)
                        .raw_value()
                        .to_string(),
                )
            })
        }
        WorldProfileField::GenerationStructures => profile
            .generation
            .structures
            .map(|value| ("generate-structures", value.to_string())),
        WorldProfileField::GenerationGeneratorOptions
            if server_type == ServerType::Java && context == WorldProfileApplyContext::Creation =>
        {
            profile
                .generation
                .generator_options
                .clone()
                .map(|value| ("generator-settings", value))
        }
        WorldProfileField::GenerationBonusChest
            if context == WorldProfileApplyContext::Creation =>
        {
            profile
                .generation
                .bonus_chest
                .map(|value| ("bonus-chest", value.to_string()))
        }
        WorldProfileField::GameplayDifficulty => profile
            .gameplay
            .difficulty
            .clone()
            .map(|value| ("difficulty", value)),
        WorldProfileField::GameplayDefaultGameMode => profile
            .gameplay
            .default_game_mode
            .clone()
            .map(|value| ("gamemode", value)),
        WorldProfileField::GameplayHardcore if context == WorldProfileApplyContext::Creation => {
            profile
                .gameplay
                .hardcore
                .map(|value| ("hardcore", value.to_string()))
        }
        WorldProfileField::GameplayCommands
            if server_type == ServerType::Java && context == WorldProfileApplyContext::Creation =>
        {
            profile
                .gameplay
                .commands
                .map(|value| ("enable-command-block", value.to_string()))
        }
        WorldProfileField::GameplayCheats if server_type == ServerType::Bedrock => profile
            .gameplay
            .cheats
            .map(|value| ("allow-cheats", value.to_string())),
        WorldProfileField::GameplayCoordinates if server_type == ServerType::Bedrock => profile
            .gameplay
            .coordinates
            .map(|value| ("show-coordinates", value.to_string())),
        WorldProfileField::GameplayStartingMap if server_type == ServerType::Bedrock => profile
            .gameplay
            .starting_map
            .map(|value| ("starting-map", value.to_string())),
        WorldProfileField::GameplaySupportedToggles if server_type == ServerType::Bedrock => {
            profile
                .gameplay
                .supported_toggles
                .get("require-resource-packs")
                .map(|value| ("texturepack-required", value.to_string()))
        }
        _ => None,
    }
}

fn profile_change_status(
    field: WorldProfileField,
    context: WorldProfileApplyContext,
    is_server_running: bool,
) -> (WorldProfileApplyStatus, Option<String>) {
    let policy = field.apply_policy();
    if context == WorldProfileApplyContext::Activation && policy == SettingApplyPolicy::CreationOnly
    {
        return (
            WorldProfileApplyStatus::Blocked,
            Some("creation_only".to_string()),
        );
    }
    if is_server_running {
        return match policy {
            SettingApplyPolicy::CreationOnly | SettingApplyPolicy::ApplyOnActivation => (
                WorldProfileApplyStatus::Blocked,
                Some("server_running".to_string()),
            ),
            SettingApplyPolicy::RestartRequired => (
                WorldProfileApplyStatus::PendingRestart,
                Some("restart_required".to_string()),
            ),
            SettingApplyPolicy::LiveSafe => (WorldProfileApplyStatus::Live, None),
        };
    }
    (WorldProfileApplyStatus::Live, None)
}

/// Applies the shared world-profile projection used by initial creation,
/// activation, and profile updates. Only keys represented by a world profile
/// are changed. Bedrock gameplay and generation settings are written into
/// that world's `level.dat`; BDS server properties are projected separately.
pub fn apply_world_profile(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
    profile: &WorldProfile,
    context: WorldProfileApplyContext,
    is_server_running: bool,
) -> io::Result<WorldProfileApplicationReport> {
    let path = server_dir.join("server.properties");
    let mut properties = read_properties_map(fs, &path);
    let mut expected = BTreeMap::new();
    let mut changes = Vec::new();
    if server_type == ServerType::Java {
        crate::java_world_settings::validate(profile)?;
        if !is_server_running {
            if let (Some(old), Some(new)) = (
                properties.get("level-name"),
                profile.identity.level_name.as_deref(),
            ) && old != new
                && fs.stat(&server_dir.join(old).join("level.dat")).is_ok()
                && context != WorldProfileApplyContext::Creation
            {
                rename_world(
                    fs,
                    server_dir,
                    server_type,
                    Some(old),
                    new,
                    false,
                    false,
                    || true,
                )
                .map_err(|error| io::Error::other(error.to_string()))?;
                properties.insert("level-name".into(), new.into());
            }
            let level_name = profile
                .identity
                .level_name
                .as_deref()
                .or_else(|| properties.get("level-name").map(String::as_str))
                .ok_or_else(|| io::Error::other("Java world folder name is missing"))?;
            if context == WorldProfileApplyContext::Creation
                && fs
                    .stat(&server_dir.join(level_name).join("level.dat"))
                    .is_err()
            {
                let generation = crate::java_world_settings::generation_properties(
                    fs, server_dir, level_name, profile,
                )?;
                expected.extend(generation.clone());
                properties.extend(generation);
                // Absent seed and defaults must not leak from the previous world.
                properties.insert(
                    "level-seed".into(),
                    profile.identity.seed.clone().unwrap_or_default(),
                );
                properties.insert(
                    "generate-structures".into(),
                    profile.generation.structures.unwrap_or(true).to_string(),
                );
                properties.insert(
                    "hardcore".into(),
                    profile.gameplay.hardcore.unwrap_or(false).to_string(),
                );
                properties.insert(
                    "enable-command-block".into(),
                    profile.gameplay.commands.unwrap_or(false).to_string(),
                );
                expected.extend(
                    properties
                        .iter()
                        .filter(|(key, _)| {
                            [
                                "level-seed",
                                "generate-structures",
                                "hardcore",
                                "enable-command-block",
                            ]
                            .contains(&key.as_str())
                        })
                        .map(|(key, value)| (key.clone(), value.clone())),
                );
            } else {
                crate::java_world_settings::apply_existing_world(
                    fs, server_dir, level_name, profile,
                )?;
                if let Some(enabled) = profile.gameplay.commands {
                    properties.insert("enable-command-block".into(), enabled.to_string());
                    expected.insert("enable-command-block".into(), enabled.to_string());
                }
                if let Some(enabled) = profile.gameplay.hardcore {
                    properties.insert("hardcore".into(), enabled.to_string());
                    expected.insert("hardcore".into(), enabled.to_string());
                }
            }
        }
    }
    // This BDS property is server-level, but MSC owns its preference per world.
    // A world without an explicit preference must not inherit the previous one.
    if server_type == ServerType::Bedrock && !is_server_running {
        let required = profile
            .gameplay
            .supported_toggles
            .get("require-resource-packs")
            .copied()
            .unwrap_or(false);
        properties.insert("texturepack-required".into(), required.to_string());
        expected.insert("texturepack-required".into(), required.to_string());
    }

    for field in WorldProfileField::ALL {
        if !field.applies_to(server_type) || !profile_field_present(profile, field) {
            continue;
        }
        let (mut status, mut reason) = profile_change_status(field, context, is_server_running);
        if field == WorldProfileField::IdentityName {
            changes.push(WorldProfileChange {
                key: field.key().into(),
                status: WorldProfileApplyStatus::Live,
                reason: None,
            });
            continue;
        }
        if server_type == ServerType::Java
            && matches!(
                field,
                WorldProfileField::GenerationWorldType
                    | WorldProfileField::GenerationFlatPreset
                    | WorldProfileField::GenerationBiomeSource
                    | WorldProfileField::GenerationGeneratorOptions
                    | WorldProfileField::GenerationBonusChest
                    | WorldProfileField::GenerationDataPacks
                    | WorldProfileField::GameplayGamerules
            )
        {
            if matches!(
                field,
                WorldProfileField::GameplayGamerules | WorldProfileField::GenerationDataPacks
            ) {
                status = WorldProfileApplyStatus::PendingRestart;
                reason = Some(
                    if is_server_running {
                        "restart_required"
                    } else {
                        "applies_on_start"
                    }
                    .into(),
                );
            }
            changes.push(WorldProfileChange {
                key: field.key().into(),
                status,
                reason,
            });
            continue;
        }
        if server_type == ServerType::Bedrock && bedrock_level_data_field(field) {
            if is_server_running && status == WorldProfileApplyStatus::Live {
                status = WorldProfileApplyStatus::PendingRestart;
                reason = Some("restart_required".into());
            }
            // Difficulty and game mode also seed new players through BDS properties.
            if !is_server_running
                && status != WorldProfileApplyStatus::Blocked
                && let Some((key, value)) =
                    profile_property_value(profile, server_type, context, field)
                && matches!(
                    field,
                    WorldProfileField::GameplayDifficulty
                        | WorldProfileField::GameplayDefaultGameMode
                        | WorldProfileField::GameplayCheats
                )
            {
                properties.insert(key.into(), value.clone());
                expected.insert(key.into(), value);
            }
            changes.push(WorldProfileChange {
                key: field.key().into(),
                status,
                reason,
            });
            continue;
        }
        if server_type == ServerType::Bedrock
            && is_server_running
            && field == WorldProfileField::GameplaySupportedToggles
            && profile
                .gameplay
                .supported_toggles
                .contains_key("require-resource-packs")
        {
            status = WorldProfileApplyStatus::PendingRestart;
            reason = Some("restart_required".into());
        }
        if let Some((key, value)) = profile_property_value(profile, server_type, context, field) {
            if status != WorldProfileApplyStatus::Blocked
                && !(server_type == ServerType::Bedrock && is_server_running)
            {
                properties.insert(key.to_string(), value.clone());
                expected.insert(key.to_string(), value);
            }
        } else {
            status = WorldProfileApplyStatus::Blocked;
            reason = Some("runtime_projection_unavailable".to_string());
        }
        changes.push(WorldProfileChange {
            key: field.key().to_string(),
            status,
            reason,
        });
    }

    if server_type == ServerType::Bedrock && !is_server_running {
        apply_bedrock_world_data(fs, server_dir, &properties, profile, context)?;
    }

    if !expected.is_empty() {
        write_properties_map(fs, &path, &properties)?;
        let accepted = read_properties_map(fs, &path);
        for change in &mut changes {
            if change.status == WorldProfileApplyStatus::Blocked {
                continue;
            }
            let field = WorldProfileField::ALL
                .into_iter()
                .find(|field| field.key() == change.key)
                .expect("every profile change key comes from WorldProfileField::ALL");
            if server_type == ServerType::Java
                && matches!(
                    field,
                    WorldProfileField::GenerationWorldType
                        | WorldProfileField::GenerationGeneratorOptions
                        | WorldProfileField::GenerationBonusChest
                )
            {
                continue;
            }
            if let Some((property, value)) =
                profile_property_value(profile, server_type, context, field)
                && expected.contains_key(property)
                && accepted.get(property).map(String::as_str) != Some(value.as_str())
            {
                change.status = WorldProfileApplyStatus::Blocked;
                change.reason = Some("runtime_readback_mismatch".to_string());
            }
        }
    }

    Ok(WorldProfileApplicationReport { changes })
}

fn bedrock_level_data_field(field: WorldProfileField) -> bool {
    matches!(
        field,
        WorldProfileField::IdentitySeed
            | WorldProfileField::GenerationWorldType
            | WorldProfileField::GenerationBonusChest
            | WorldProfileField::GameplayDifficulty
            | WorldProfileField::GameplayDefaultGameMode
            | WorldProfileField::GameplayGamerules
            | WorldProfileField::GameplayCheats
            | WorldProfileField::GameplayExperiments
            | WorldProfileField::GameplayCoordinates
            | WorldProfileField::GameplayStartingMap
    )
}

fn apply_bedrock_world_data(
    fs: &dyn FileSystem,
    server_dir: &Path,
    properties: &BTreeMap<String, String>,
    profile: &WorldProfile,
    context: WorldProfileApplyContext,
) -> io::Result<()> {
    use msc_domain::nbt::NbtValue;
    let level_name = properties
        .get("level-name")
        .filter(|name| !name.is_empty())
        .ok_or_else(|| io::Error::other("Bedrock world folder name is missing"))?;
    let world_dir = server_dir.join("worlds").join(level_name);
    let level_dat = world_dir.join("level.dat");
    let raw = match fs.read(&level_dat) {
        Ok(raw) => Some(raw),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let creating = context == WorldProfileApplyContext::Creation;
    if raw.is_none() && !creating {
        return Err(io::Error::other(
            "The active Bedrock world's level.dat is missing",
        ));
    }
    let mut tags = BTreeMap::new();
    if raw.is_none() {
        // A metadata-only fresh world has no chunks, player records, or elapsed time.
        tags.extend([
            ("StorageVersion".into(), NbtValue::Int(10)),
            ("Generator".into(), NbtValue::Int(1)),
            ("GameType".into(), NbtValue::Int(0)),
            ("Difficulty".into(), NbtValue::Int(2)),
            ("SpawnX".into(), NbtValue::Int(0)),
            ("SpawnY".into(), NbtValue::Int(32767)),
            ("SpawnZ".into(), NbtValue::Int(0)),
            ("Time".into(), NbtValue::Long(0)),
            ("currentTick".into(), NbtValue::Long(0)),
            ("worldStartCount".into(), NbtValue::Long(0xFFFF_FFFE)),
            (
                "RandomSeed".into(),
                NbtValue::Long(Uuid::new_v4().as_u128() as i64),
            ),
        ]);
        for name in [
            "dodaylightcycle",
            "doweathercycle",
            "domobspawning",
            "domobloot",
            "doentitydrops",
            "dotiledrops",
            "dofiretick",
            "naturalregeneration",
            "pvp",
            "falldamage",
            "firedamage",
            "drowningdamage",
            "sendcommandfeedback",
            "commandblockoutput",
            "commandblocksenabled",
            "showdeathmessages",
            "tntexplodes",
            "respawnblocksexplode",
        ] {
            tags.insert(name.into(), NbtValue::Byte(1));
        }
        for name in [
            "commandsEnabled",
            "showcoordinates",
            "startWithMapEnabled",
            "bonusChestEnabled",
            "bonusChestSpawned",
            "keepinventory",
        ] {
            tags.insert(name.into(), NbtValue::Byte(0));
        }
        tags.insert("randomtickspeed".into(), NbtValue::Int(1));
        tags.insert("spawnradius".into(), NbtValue::Int(5));
    }
    tags.insert(
        "LevelName".into(),
        NbtValue::String(
            profile
                .identity
                .name
                .clone()
                .unwrap_or_else(|| level_name.clone()),
        ),
    );
    if creating {
        if let Some(seed) = profile
            .identity
            .seed
            .as_deref()
            .filter(|seed| !seed.is_empty())
        {
            let value = seed.parse::<i64>().unwrap_or_else(|_| {
                seed.encode_utf16().fold(0i32, |hash, ch| {
                    hash.wrapping_mul(31).wrapping_add(i32::from(ch))
                }) as i64
            });
            tags.insert("RandomSeed".into(), NbtValue::Long(value));
        }
        if let Some(kind) = profile.generation.world_type.as_deref() {
            let generator = match kind {
                "default" | "normal" | "minecraft:normal" => 1,
                "flat" | "minecraft:flat" => 2,
                _ => {
                    return Err(io::Error::other(
                        "BDS supports Default and Flat world generation",
                    ));
                }
            };
            tags.insert("Generator".into(), NbtValue::Int(generator));
            if generator == 2 {
                tags.insert("FlatWorldLayers".into(), NbtValue::String(r#"{"biome_id":1,"block_layers":[{"block_name":"minecraft:bedrock","count":1},{"block_name":"minecraft:dirt","count":2},{"block_name":"minecraft:grass_block","count":1}],"encoding_version":6,"structure_options":null,"world_version":"version.post_1_18"}"#.into()));
            }
        }
        if let Some(enabled) = profile.generation.bonus_chest {
            tags.insert(
                "bonusChestEnabled".into(),
                NbtValue::Byte(i8::from(enabled)),
            );
        }
        if let Some(enabled) = profile.gameplay.starting_map {
            tags.insert(
                "startWithMapEnabled".into(),
                NbtValue::Byte(i8::from(enabled)),
            );
        }
    }
    if let Some(difficulty) = profile.gameplay.difficulty.as_deref() {
        let value = match difficulty {
            "peaceful" => 0,
            "easy" => 1,
            "normal" => 2,
            "hard" => 3,
            _ => return Err(io::Error::other("Invalid Bedrock difficulty")),
        };
        tags.insert("Difficulty".into(), NbtValue::Int(value));
    }
    if let Some(mode) = profile.gameplay.default_game_mode.as_deref() {
        let value = match mode {
            "survival" => 0,
            "creative" => 1,
            "adventure" => 2,
            _ => return Err(io::Error::other("Invalid Bedrock game mode")),
        };
        tags.insert("GameType".into(), NbtValue::Int(value));
    }
    if let Some(enabled) = profile.gameplay.cheats {
        tags.insert("commandsEnabled".into(), NbtValue::Byte(i8::from(enabled)));
    }
    if profile.gameplay.cheats == Some(true)
        || profile.gameplay.default_game_mode.as_deref() == Some("creative")
    {
        tags.insert("hasBeenLoadedInCreative".into(), NbtValue::Byte(1));
    }
    for (name, value) in &profile.gameplay.gamerules {
        if name.is_empty() || !name.chars().all(|ch| ch.is_ascii_alphanumeric()) {
            return Err(io::Error::other(
                "Gamerule names must contain only letters and digits",
            ));
        }
        // Locator bar is a native command alias; apply it through BDS on ready.
        if name.eq_ignore_ascii_case("locatorbar") {
            if value != "true" && value != "false" {
                return Err(io::Error::other("Locator bar must be true or false"));
            }
            continue;
        }
        let tag = if name.eq_ignore_ascii_case("playerwaypoints") {
            NbtValue::Int(match value.as_str() {
                "everyone" => 1,
                "off" => 0,
                _ => return Err(io::Error::other("Player waypoints must be everyone or off")),
            })
        } else {
            match value.as_str() {
                "true" => NbtValue::Byte(1),
                "false" => NbtValue::Byte(0),
                _ => NbtValue::Int(value.parse().map_err(|_| {
                    io::Error::other(format!(
                        "Gamerule {name} must be true, false, or an integer"
                    ))
                })?),
            }
        };
        tags.insert(name.to_ascii_lowercase(), tag);
    }
    // The dedicated control takes precedence over a duplicate gamerule entry.
    if let Some(enabled) = profile.gameplay.coordinates {
        tags.insert("showcoordinates".into(), NbtValue::Byte(i8::from(enabled)));
    }
    if !profile.gameplay.experiments.is_empty() {
        let mut flags: BTreeMap<String, NbtValue> = profile
            .gameplay
            .experiments
            .iter()
            .map(|(name, value)| (name.clone(), NbtValue::Byte(i8::from(*value))))
            .collect();
        if profile.gameplay.experiments.iter().any(|(name, value)| {
            *value && name != "experiments_ever_used" && name != "saved_with_toggled_experiments"
        }) {
            flags.insert("experiments_ever_used".into(), NbtValue::Byte(1));
            flags.insert("saved_with_toggled_experiments".into(), NbtValue::Byte(1));
        } else {
            // Historical flags must never be reset by a later settings save.
            flags.remove("experiments_ever_used");
            flags.remove("saved_with_toggled_experiments");
        }
        tags.insert("experiments".into(), NbtValue::Compound(flags));
    }
    let bytes = match raw.as_deref() {
        Some(raw) => msc_infrastructure::bedrock_nbt::update_level_dat(raw, &tags),
        None => msc_infrastructure::bedrock_nbt::new_level_dat(&tags),
    }
    .map_err(|error| io::Error::other(error.to_string()))?;
    fs.create_dir_all(&world_dir)?;
    msc_infrastructure::atomic_write::atomic_write(fs, &level_dat, &bytes)
        .map_err(|error| io::Error::other(error.to_string()))?;
    if fs.read(&level_dat)? != bytes {
        return Err(io::Error::other("Bedrock world settings readback mismatch"));
    }
    Ok(())
}

#[path = "worlds/activation.rs"]
mod activation;
pub use activation::*;
use activation::{
    move_approved_world_roots, relocate_legacy_bedrock_layout, repair_error, safe_world_folder_name,
};

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
    if !safe_world_folder_name(trimmed) {
        return Err(WorldError::InvalidWorldSource);
    }
    if is_server_running {
        return Err(WorldError::ServerRunning);
    }

    let old_level_name = resolved_level_name(fs, server_dir, server_type, raw_level_name);
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
// Staging completes before `swap.json` records the old and new folder
// names. The shared swap reconciler handles every partial move and the
// replacement commit replays the new level-name when installation finished.
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
    fs.remove(&replace_dir(server_dir))?;
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
    if fs.stat(&replace_dir(server_dir)).is_ok() {
        return Err(WorldError::Io(repair_error(
            "replacement transaction is still present",
        )));
    }
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
            if archive::validate_world_archive(path, server_type).is_err() {
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
    let current_level_name = resolved_level_name(fs, server_dir, server_type, raw_level_name);
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
    let approved_roots = match world_source {
        WorldReplaceSource::BackupZip(path) => archive::validate_world_archive(path, server_type)?,
        WorldReplaceSource::Fresh => Vec::new(),
        WorldReplaceSource::ExistingFolder(_) => vec![trimmed.to_string()],
    };
    match world_source {
        WorldReplaceSource::Fresh => {}
        WorldReplaceSource::BackupZip(path) => {
            if let Err(e) = archive::extract_zip(path, &staged_dir) {
                let _ = fs.remove(&replace_dir(server_dir));
                return Err(e.into());
            }
            if server_type == ServerType::Bedrock {
                relocate_legacy_bedrock_layout(&staged_dir, trimmed)?;
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

    let prior_dir = replace_prior_dir(server_dir);
    let install_roots = if server_type == ServerType::Bedrock {
        match world_source {
            WorldReplaceSource::BackupZip(_) => top_level_entries(fs, &staged_base)?
                .into_iter()
                .filter_map(|path| {
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .collect::<Vec<_>>(),
            _ => approved_roots,
        }
    } else {
        approved_roots
    };
    let old_roots = current_names
        .into_iter()
        .filter(|name| folder_exists(fs, &base.join(name)))
        .collect();
    begin_world_swap(
        fs,
        &replace_dir(server_dir),
        &base,
        &staged_base,
        &prior_dir,
        old_roots,
        install_roots,
    )?;
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
/// already establish. The durable swap manifest lists every folder and
/// phase; incomplete moves roll back, while a complete install commits.
pub fn reconcile_interrupted_world_replace(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_type: ServerType,
) -> Result<Option<WorldReplaceRecovery>, WorldError> {
    let dir = replace_dir(server_dir);
    if fs.stat(&dir).is_err() {
        return Ok(None);
    }

    let base = world_base_dir(server_dir, server_type);
    let outcome = recover_world_swap(
        fs,
        &dir,
        &base,
        &world_base_dir(&replace_staged_dir(server_dir), server_type),
        &replace_prior_dir(server_dir),
    )?;
    if outcome == SwapRecovery::Old {
        return Ok(Some(WorldReplaceRecovery::RecoveredToOldWorld));
    }

    // Phase 3 ("installed"): the new world is already in place; replay
    // the commit tail (apply identity, remove `.replace/`).
    let manifest_bytes = fs.read(&replace_manifest_path(server_dir))?;
    let level_name = parse_replace_manifest(&manifest_bytes).ok_or(WorldError::Manifest)?;
    finish_replace_commit(fs, server_dir, &level_name)?;
    Ok(Some(WorldReplaceRecovery::RecoveredToNewWorld))
}
