//! Transactional world-slot activation and interrupted-activation recovery.

use super::*;

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

// The swap record is written before any live rename. A phase write precedes
// the first rename in that phase, so recovery can use the record and the
// physical location of each named folder together.
#[derive(serde::Serialize, serde::Deserialize)]
struct WorldSwap {
    phase: SwapPhase,
    old: Vec<String>,
    new: Vec<String>,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SwapPhase {
    Moving,
    Installing,
    RollingBack,
    Restoring,
    Committing,
}

pub(super) fn repair_error(reason: &str) -> io::Error {
    io::Error::other(format!("world swap needs manual repair: {reason}"))
}

fn swap_path(dir: &Path) -> PathBuf {
    dir.join("swap.json")
}

fn write_swap(fs: &dyn FileSystem, dir: &Path, swap: &WorldSwap) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(swap).map_err(io::Error::other)?;
    msc_infrastructure::atomic_write::atomic_write(fs, &swap_path(dir), &bytes)
        .map_err(io::Error::other)
}

fn valid_swap_names(names: &[String]) -> bool {
    let mut unique = BTreeSet::new();
    names
        .iter()
        .all(|name| safe_world_folder_name(name) && unique.insert(name))
}

pub(crate) fn begin_world_swap(
    fs: &dyn FileSystem,
    dir: &Path,
    live: &Path,
    staged: &Path,
    prior: &Path,
    old: Vec<String>,
    new: Vec<String>,
) -> io::Result<()> {
    if !valid_swap_names(&old) || !valid_swap_names(&new) {
        return Err(repair_error(
            "manifest contains an unsafe or duplicate folder name",
        ));
    }
    for name in &old {
        if !folder_exists(fs, &live.join(name)) || fs.stat(&prior.join(name)).is_ok() {
            return Err(repair_error(
                "old folder is missing or prior folder is occupied",
            ));
        }
    }
    for name in &new {
        if !folder_exists(fs, &staged.join(name)) {
            return Err(repair_error("staged folder is missing"));
        }
        if fs.stat(&live.join(name)).is_ok() && !old.contains(name) {
            return Err(repair_error("new folder destination is occupied"));
        }
    }
    let mut swap = WorldSwap {
        phase: SwapPhase::Moving,
        old,
        new,
    };
    write_swap(fs, dir, &swap)?;
    fs.create_dir_all(prior)?;
    for name in &swap.old {
        fs.rename(&live.join(name), &prior.join(name))?;
    }
    test_pause_after_world_move();
    swap.phase = SwapPhase::Installing;
    write_swap(fs, dir, &swap)?;
    for name in &swap.new {
        fs.rename(&staged.join(name), &live.join(name))?;
    }
    swap.phase = SwapPhase::Committing;
    write_swap(fs, dir, &swap)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwapRecovery {
    Old,
    New,
}

pub(crate) fn recover_world_swap(
    fs: &dyn FileSystem,
    dir: &Path,
    live: &Path,
    staged: &Path,
    prior: &Path,
) -> io::Result<SwapRecovery> {
    let bytes = match fs.read(&swap_path(dir)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // The swap record is created only after staging is complete.
            if fs.stat(prior).is_ok() {
                return Err(repair_error("prior folders exist without a swap manifest"));
            }
            fs.remove(dir)?;
            return Ok(SwapRecovery::Old);
        }
        Err(error) => return Err(error),
    };
    let mut swap: WorldSwap =
        serde_json::from_slice(&bytes).map_err(|_| repair_error("swap manifest is unreadable"))?;
    if !valid_swap_names(&swap.old) || !valid_swap_names(&swap.new) {
        return Err(repair_error("swap manifest contains unsafe folder names"));
    }
    if matches!(swap.phase, SwapPhase::Committing) {
        if swap
            .new
            .iter()
            .any(|name| !folder_exists(fs, &live.join(name)))
            || swap
                .old
                .iter()
                .any(|name| !folder_exists(fs, &prior.join(name)))
        {
            return Err(repair_error(
                "committed world folders cannot be proven complete",
            ));
        }
        return Ok(SwapRecovery::New);
    }
    if matches!(swap.phase, SwapPhase::Installing | SwapPhase::RollingBack)
        && swap
            .old
            .iter()
            .any(|name| !folder_exists(fs, &prior.join(name)))
    {
        return Err(repair_error("a prior world folder is missing"));
    }
    // In the installing phase, a missing staged root is an installed root.
    // Remove those destinations before restoring old roots, including names
    // shared by the old and new worlds. A missing root on both sides is an
    // error, never a reason to call the rollback complete.
    if matches!(swap.phase, SwapPhase::Installing) {
        for name in &swap.new {
            if folder_exists(fs, &staged.join(name)) == folder_exists(fs, &live.join(name)) {
                return Err(repair_error("new folder location is ambiguous"));
            }
        }
        swap.phase = SwapPhase::RollingBack;
        write_swap(fs, dir, &swap)?;
    }
    if matches!(swap.phase, SwapPhase::RollingBack) {
        for name in &swap.new {
            let source = folder_exists(fs, &staged.join(name));
            let destination = folder_exists(fs, &live.join(name));
            if source && destination {
                return Err(repair_error("new folder location is ambiguous"));
            }
            if !source && destination {
                fs.remove(&live.join(name))?;
            }
        }
    }
    swap.phase = SwapPhase::Restoring;
    write_swap(fs, dir, &swap)?;
    for name in &swap.old {
        let source = folder_exists(fs, &prior.join(name));
        let destination = folder_exists(fs, &live.join(name));
        if source == destination {
            return Err(repair_error("old folder location is ambiguous"));
        }
        if source {
            fs.rename(&prior.join(name), &live.join(name))?;
        }
    }
    fs.remove(dir)?;
    Ok(SwapRecovery::Old)
}

pub(super) fn move_approved_world_roots(
    fs: &dyn FileSystem,
    from_dir: &Path,
    to_dir: &Path,
    approved_roots: &[String],
) -> io::Result<()> {
    let entries = top_level_entries(fs, from_dir)?;
    if entries.len() != approved_roots.len() {
        return Err(io::Error::other(
            "staged world roots changed after validation",
        ));
    }
    for entry in entries {
        let name = entry
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| io::Error::other("staged world root has no portable name"))?;
        if !approved_roots.iter().any(|root| root == name) {
            return Err(io::Error::other("staged world root was not approved"));
        }
    }
    for root in approved_roots {
        fs.rename(&from_dir.join(root), &to_dir.join(root))?;
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
/// server root. P16.4 treats a failed relocation as a staging failure:
/// installing a partially normalized world would be unsafe.
pub(super) fn relocate_legacy_bedrock_layout(
    staged_dir: &Path,
    level_name: &str,
) -> io::Result<()> {
    if !safe_world_folder_name(level_name) {
        return Err(io::Error::other("Bedrock world folder name is unsafe"));
    }
    let worlds_dir = staged_dir.join("worlds");
    let expected_dir = worlds_dir.join(level_name);
    let loose_db_dir = worlds_dir.join("db");
    if expected_dir.is_dir() || !loose_db_dir.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(&worlds_dir)?;
    fs::create_dir_all(&expected_dir)?;
    for entry in entries {
        let entry = entry?;
        if entry.file_name() == level_name {
            continue;
        }
        let dest = expected_dir.join(entry.file_name());
        fs::rename(entry.path(), dest)?;
    }
    Ok(())
}

pub(super) fn safe_world_folder_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0'])
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
        let current_level_name = resolved_level_name(fs, server_dir, server_type, None);
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
    if fs.stat(&activation_dir(server_dir)).is_ok() {
        return Err(repair_error("activation transaction is still present").into());
    }
    if is_server_running {
        return Err(ActivationError::ServerRunning);
    }
    if should_cancel() {
        return Err(ActivationError::Cancelled);
    }

    let (has_archive, identity) = resolve_activation_identity(fs, server_dir, server_type, slot)?;
    if identity
        .as_ref()
        .is_some_and(|identity| !safe_world_folder_name(&identity.level_name))
    {
        return Err(ActivationError::Archive(ArchiveError::InvalidWorldLayout(
            "world folder name is unsafe".into(),
        )));
    }
    let mut approved_roots = if has_archive {
        archive::validate_world_archive(&world_store::zip_path(server_dir, &slot.id), server_type)?
    } else {
        Vec::new()
    };

    let current_level_name = resolved_level_name(fs, server_dir, server_type, None);
    let current_folders = existing_world_folders(fs, server_dir, server_type, &current_level_name);

    if !current_folders.is_empty() && !backup() {
        return if should_cancel() {
            Err(ActivationError::Cancelled)
        } else {
            Err(ActivationError::BackupFailed)
        };
    }

    if !current_folders.is_empty() {
        let slots = world_store::load_slots(fs, server_dir);
        let marker = world_store::load_explicit_active_slot_id(fs, server_dir);
        if let Some(active_id) = world::resolve_active_slot_id(&slots, marker.as_deref())
            && active_id != slot.id
            && let Some(old_slot) = slots.iter().find(|old| old.id == active_id)
        {
            update_active_slot_from_current_world(
                fs,
                server_dir,
                server_type,
                Some(&current_level_name),
                old_slot,
            )
            .map_err(|error| io::Error::other(error.to_string()))?;
        }
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
            if server_type == ServerType::Java {
                // A slot can choose a new folder name while its archive retains the old layout.
                let entries = std::fs::read_dir(&staged_dir)?;
                let roots: Vec<_> = entries
                    .filter_map(Result::ok)
                    .filter(|entry| {
                        entry.path().join("level.dat").is_file()
                            && !entry.file_name().to_string_lossy().ends_with("_nether")
                            && !entry.file_name().to_string_lossy().ends_with("_the_end")
                    })
                    .collect();
                if roots.len() != 1 {
                    return Err(io::Error::other(
                        "Java world archive must contain one main world folder",
                    )
                    .into());
                }
                let old = roots[0].file_name().to_string_lossy().into_owned();
                if old != identity.level_name {
                    for (old, new) in world::live_world_folder_candidates(ServerType::Java, &old)
                        .iter()
                        .zip(world::live_world_folder_candidates(
                            ServerType::Java,
                            &identity.level_name,
                        ))
                    {
                        let source = staged_dir.join(old);
                        let target = staged_dir.join(&new);
                        if fs.stat(&source).is_ok() {
                            if fs.stat(&target).is_ok() {
                                return Err(io::Error::other(
                                    "Java world archive has conflicting folder names",
                                )
                                .into());
                            }
                            fs.rename(&source, &target)?;
                            if let Some(root) = approved_roots.iter_mut().find(|root| *root == old)
                            {
                                *root = new.clone();
                            }
                        }
                    }
                }
            } else {
                relocate_legacy_bedrock_layout(&staged_dir, &identity.level_name)?;
            }
        }
    }

    // Last chance to cancel for free: staging is complete but nothing at
    // the server root has been touched yet, so backing out here is just
    // deleting the scratch directory this call itself just created.
    if should_cancel() {
        let _ = fs.remove(&activation_dir(server_dir));
        return Err(ActivationError::Cancelled);
    }

    let prior_dir = activation_prior_dir(server_dir);
    begin_world_swap(
        fs,
        &activation_dir(server_dir),
        server_dir,
        &staged_dir,
        &prior_dir,
        current_folders,
        approved_roots,
    )?;

    finish_activation_commit(
        fs,
        server_dir,
        server_type,
        slot,
        identity.as_ref(),
        if has_archive {
            WorldProfileApplyContext::Activation
        } else {
            WorldProfileApplyContext::Creation
        },
        now,
    )
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
    server_type: ServerType,
    slot: &WorldSlot,
    identity: Option<&WorldIdentity>,
    profile_context: WorldProfileApplyContext,
    now: &str,
) -> Result<WorldSlot, ActivationError> {
    let mut profile = world_store::load_profile(fs, server_dir, slot);
    if let Some(identity) = identity {
        profile.identity.level_name = Some(identity.level_name.clone());
        if identity.apply_seed {
            profile.identity.seed = identity.seed.clone();
        }
    }
    apply_world_profile(
        fs,
        server_dir,
        server_type,
        &profile,
        profile_context,
        false,
    )?;

    let mut updated = slot.clone();
    updated.last_played_at = Some(now.to_string());
    if let Some(identity) = identity {
        updated.world_level_name = Some(identity.level_name.clone());
    }
    world_store::save_profile(fs, server_dir, &updated, &profile)?;
    world_store::save_metadata(fs, server_dir, &updated)?;
    world_store::set_active_slot_id(fs, server_dir, Some(&updated.id))?;

    fs.remove(&activation_dir(server_dir))?;
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
/// the complete old world or the complete new world. The swap manifest
/// and each named folder's location establish which outcome is safe.
pub fn reconcile_interrupted_activation(
    fs: &dyn FileSystem,
    server_dir: &Path,
    now: &str,
) -> Result<Option<ActivationRecovery>, ActivationError> {
    let activation_dir = activation_dir(server_dir);
    if fs.stat(&activation_dir).is_err() {
        return Ok(None);
    }

    let outcome = recover_world_swap(
        fs,
        &activation_dir,
        server_dir,
        &activation_staged_dir(server_dir),
        &activation_prior_dir(server_dir),
    )?;
    if outcome == SwapRecovery::Old {
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

    let server_type = if read_properties_map(fs, &server_dir.join("server.properties"))
        .contains_key("server-portv6")
    {
        ServerType::Bedrock
    } else {
        ServerType::Java
    };
    let updated = finish_activation_commit(
        fs,
        server_dir,
        server_type,
        &slot,
        identity.as_ref(),
        if has_archive(fs, server_dir, &slot.id) {
            WorldProfileApplyContext::Activation
        } else {
            WorldProfileApplyContext::Creation
        },
        now,
    )?;
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
