//! Ports `AppViewModel+WorldConversion.swift::performWorldConversion` and
//! the pieces of `ChunkerManager.swift` it drives directly:
//! `findInputWorldFolder`/`packageOutput` (pure-ish path logic over a
//! real, already-unzipped scratch directory) and `convert` (the actual
//! `java -jar chunker-cli.jar` process invocation).
//!
//! Flow, matching source's own comment at the top of
//! `AppViewModel+WorldConversion.swift`: unzip source slot → locate the
//! nested world folder → run Chunker → package output into a
//! slot-compatible zip → create or replace the target slot → back up the
//! target server's current world (warn-only) → activate the new slot →
//! clean up the temp working directory.
//!
//! **The Chunker process boundary is a fakeable port**
//! ([`WorldConverter`]), the same "policy vs. runtime mechanism" split
//! this phase already drew for `backups::BackupConsole` (P6.16). No
//! production implementation exists yet — building the real
//! `java -jar chunker-cli.jar -i … -f … -o …` invocation (source
//! `ChunkerManager.swift:222-267`), plus GitHub release download and
//! `~/Library/Application Support` jar-path resolution, is deferred to
//! whichever later step first needs a running Chunker (P6.21 route
//! wiring, at the earliest) — this step only needs the boundary and the
//! orchestration logic around it, matching `BackupConsole`'s own
//! precedent of shipping the port and a fake, not a production adapter.
//! Every fixture this step characterizes (`fixtures/world-conversion/`,
//! P6.7) is exercised through a scripted `FakeWorldConverter` in
//! `tests/world_conversion.rs`.
//!
//! **One real MSC 1 gap is preserved, not corrected**, per its fixture's
//! own notes (P6.7) — raised as a question and left as-is on Cameron's
//! call: a later activation failure (this function's own final step)
//! does not revert the slot already written in the placement step — the
//! new/replaced slot is left on disk, inactive, exactly as source leaves
//! it.
//!
//! **One real MSC 1 gap *is* corrected**, also on Cameron's call:
//! source's `replaceSlotWithConvertedZip` removes the destination's
//! existing archive *before* copying the new one in — a plain
//! remove-then-copy, unlike every other overwrite in this phase
//! (`worlds::update_active_slot_from_current_world`, `worlds::
//! copy_slot_into_existing`), which all stage the write to a temp file
//! first and only then atomically replace the destination. See
//! [`replace_slot_with_converted_zip`]'s own doc for the fix.
//!
//! One precondition this step's own plan text adds beyond the oracle:
//! "validate stopped source/target." Source itself never checks this
//! inside `performWorldConversion` — the running-server guard lives
//! entirely in `WorldConversionWizardView`'s own UI code
//! (`viewModel.isRunning(server)`, checked before the sheet even lets a
//! user start a conversion). Folded into this function per the same
//! "orchestration-layer guard, one layer down" pattern already applied
//! to `worlds::activate_slot`'s and `worlds::rename_world`'s running-
//! server guards.

use crate::worlds;
use msc_domain::identity::ServerType;
use msc_domain::world::{self, WorldSlot};
use msc_infrastructure::archive::{self, ArchiveError};
use msc_infrastructure::atomic_write::AtomicWriteError;
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::world_store;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// `ChunkerManager`'s installation-state/Java-resolution/`convert`
/// surface, narrowed to exactly what [`convert_world`] needs — see the
/// module doc for why no production implementation lives in this crate
/// yet.
pub trait WorldConverter {
    /// `ChunkerManager.isInstalled` (source line 87-89).
    fn is_installed(&self) -> bool;
    /// `ChunkerManager.resolveJavaPath(appConfigJavaPath:)` (source line
    /// 101-127): configured path, then common system locations, then
    /// `which java`. A real implementation performs that search; this
    /// port only needs the result.
    fn resolve_java_path(&self, configured_java_path: &str) -> Option<String>;
    /// `ChunkerManager.convert(inputDir:outputDir:targetFormat:javaPath:
    /// progressHandler:)` (source line 222-267): runs Chunker against an
    /// already-located `input_dir`, writing into `output_dir` (which must
    /// not exist beforehand — source's own doc comment), streaming each
    /// stdout/stderr line to `progress` as it arrives. `Err(message)`
    /// mirrors `ChunkerError.conversionFailed("Chunker exited with code
    /// \(status)")` — a non-zero exit, not a Rust-level I/O failure.
    fn convert(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        target_format: &str,
        java_path: &str,
        progress: &mut dyn FnMut(&str),
    ) -> Result<(), String>;
    /// `ChunkerManager.supportedFormats(javaPath:)` (source line
    /// 181-216): every format string the installed jar supports, for
    /// validating a caller-supplied `targetFormat` against reality
    /// rather than trusting an unchecked string. `resolved_java_path` is
    /// the *already-resolved* path (this method's own caller runs after
    /// `resolve_java_path` already succeeded), matching source's own
    /// call shape (`chunker.supportedFormats(javaPath: java)`, where
    /// `java` is `resolveJavaPath`'s own result, not the raw config
    /// setting).
    fn supported_formats(&self, resolved_java_path: &str) -> Vec<String>;
}

/// `ConversionSlotPlacement` (source `AppViewModel+WorldConversion.swift:
/// 26-29`).
#[derive(Debug, Clone)]
pub enum ConversionPlacement {
    NewSlot { name: String },
    ReplaceExisting { slot: WorldSlot },
}

#[derive(Debug)]
pub enum ConversionError {
    /// This step's own addition — see the module doc's closing note.
    ServerRunning,
    /// `ChunkerError.javaNotFound`.
    JavaNotFound,
    /// `ChunkerError.jarNotInstalled`.
    ChunkerNotInstalled,
    /// `ChunkerError.conversionFailed("Slot name cannot be empty.")`.
    EmptyName,
    /// `ChunkerError.conversionFailed("Source slot archive not found at
    /// \(sourceZip.path)")`.
    NoSourceZip,
    /// `ChunkerError.worldFolderNotFound` — either
    /// `findInputWorldFolder` found nothing inside the unzipped source,
    /// or `packageOutput` found nothing in Chunker's output directory.
    WorldFolderNotFound,
    /// `ChunkerError.conversionFailed(_)` — a non-zero Chunker exit, a
    /// `zip`-equivalent packaging failure, or `activateSlot` returning
    /// failure (source's `guard activated else { throw
    /// .conversionFailed("Failed to activate converted world slot.") }`,
    /// collapsing every one of [`worlds::ActivationError`]'s variants
    /// into this one message, matching source exactly).
    ConversionFailed(String),
    Io(io::Error),
    Archive(ArchiveError),
    AtomicWrite(AtomicWriteError),
    /// `should_cancel` (P6.30) reported true at one of this function's own
    /// checkpoints (entry, or immediately before the Chunker process
    /// starts) — the temp working directory ([`TempRootGuard`]) is
    /// cleaned up either way, and neither the target slot nor the target
    /// server's live world has been touched. Not returned once step 5
    /// (placement) has started: from there the same "finish the current
    /// atomic filesystem action safely" rule applies, and the nested
    /// [`worlds::activate_slot`] call at step 7 makes its own independent
    /// cancellation check at its own boundary before it touches the
    /// target's live world.
    Cancelled,
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionError::ServerRunning => write!(f, "server is running"),
            ConversionError::JavaNotFound => write!(f, "java executable not found"),
            ConversionError::ChunkerNotInstalled => write!(f, "chunker CLI is not installed"),
            ConversionError::EmptyName => write!(f, "slot name cannot be empty"),
            ConversionError::NoSourceZip => write!(f, "source slot archive not found"),
            ConversionError::WorldFolderNotFound => {
                write!(f, "could not locate the world folder inside the archive")
            }
            ConversionError::ConversionFailed(msg) => write!(f, "conversion failed: {msg}"),
            ConversionError::Io(e) => write!(f, "{e}"),
            ConversionError::Archive(e) => write!(f, "{e}"),
            ConversionError::AtomicWrite(e) => write!(f, "{e}"),
            ConversionError::Cancelled => write!(f, "conversion was cancelled"),
        }
    }
}

impl std::error::Error for ConversionError {}

impl From<io::Error> for ConversionError {
    fn from(e: io::Error) -> Self {
        ConversionError::Io(e)
    }
}

impl From<ArchiveError> for ConversionError {
    fn from(e: ArchiveError) -> Self {
        ConversionError::Archive(e)
    }
}

impl From<AtomicWriteError> for ConversionError {
    fn from(e: AtomicWriteError) -> Self {
        ConversionError::AtomicWrite(e)
    }
}

/// A temp working directory unique to one conversion run — source's own
/// `tempRoot` (line 102-103), namespaced under this project's own prefix
/// rather than MSC 1's `msc_conversion_`, matching every other scratch
/// directory this phase already creates
/// (`msc2-world-reconcile-*`, `msc2-world-activation-test-*`).
fn temp_conversion_root() -> PathBuf {
    std::env::temp_dir().join(format!("msc2-world-conversion-{}", Uuid::new_v4()))
}

/// Removes [`Self::0`] on drop, regardless of which return path got
/// there — source's own `cleanup()` is called from exactly two mutually
/// exclusive sites (the success path and the `catch` block); a `Drop`
/// guard reproduces "exactly once, on both success and every
/// mid-pipeline failure" more directly than duplicating that call at
/// every one of this function's early-return points. A failure to
/// remove the directory (e.g. permissions) is silently swallowed, same
/// as source's own `try?` — no retry or leftover-tempdir reporting here
/// either.
struct TempRootGuard(PathBuf);

impl Drop for TempRootGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `ChunkerManager.findInputWorldFolder(in:isBedrock:slotLevelName:)`
/// (source line 273-316), over a real, already-unzipped directory.
fn find_input_world_folder(
    unzip_dir: &Path,
    is_bedrock: bool,
    slot_level_name: Option<&str>,
) -> Option<PathBuf> {
    fn is_dir(path: &Path) -> bool {
        fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false)
    }
    fn entry_name(path: &Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }
    // source's `firstSubdir` (line 280-289): the first directory entry,
    // excluding `__`/`.`-prefixed names, in `contentsOfDirectory`'s own
    // (unspecified, not sorted) enumeration order.
    fn first_subdir(parent: &Path) -> Option<PathBuf> {
        let entries = fs::read_dir(parent).ok()?;
        entries.flatten().map(|e| e.path()).find(|p| {
            let name = entry_name(p);
            !name.starts_with("__") && !name.starts_with('.') && is_dir(p)
        })
    }

    let level_name = slot_level_name.map(str::trim).filter(|s| !s.is_empty());

    if is_bedrock {
        let worlds_dir = unzip_dir.join("worlds");
        if let Some(name) = level_name {
            let candidate = worlds_dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        first_subdir(&worlds_dir)
    } else {
        if let Some(name) = level_name {
            let candidate = unzip_dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        // source line 304-312: filter out `_nether`/`_the_end`/`__`/`.`,
        // then `.sorted { $0.lastPathComponent < $1.lastPathComponent }`.
        let mut candidates: Vec<PathBuf> = fs::read_dir(unzip_dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                let name = entry_name(p);
                is_dir(p)
                    && !name.ends_with("_nether")
                    && !name.ends_with("_the_end")
                    && !name.starts_with("__")
                    && !name.starts_with('.')
            })
            .collect();
        candidates.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        candidates.into_iter().next()
    }
}

/// `ChunkerManager.packageOutput(chunkerOutputDir:isBedrockTarget:
/// targetLevelName:)` (source line 323-383). The Java/Bedrock branches
/// are structurally identical except the extra `worlds/` path component
/// and the zip's own root folder name — source's own doc comment on this
/// step flags that duplication as collapsible; here both branches share
/// one call into [`archive::create_zip_from_folders`], which already
/// zips a named folder (relative to a base directory) with a top-level
/// entry named after itself, exactly the shape this needs.
fn package_output(
    chunker_output_dir: &Path,
    is_bedrock_target: bool,
    target_level_name: &str,
) -> Result<PathBuf, ConversionError> {
    let package_dir = chunker_output_dir.join("_package");
    fs::create_dir_all(&package_dir)?;

    let world_entries: Vec<PathBuf> = fs::read_dir(chunker_output_dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name != "_package" && !name.starts_with('.')
        })
        .collect();
    if world_entries.is_empty() {
        return Err(ConversionError::WorldFolderNotFound);
    }

    let (world_dir, zip_folder_name) = if is_bedrock_target {
        (
            package_dir.join("worlds").join(target_level_name),
            "worlds".to_string(),
        )
    } else {
        (
            package_dir.join(target_level_name),
            target_level_name.to_string(),
        )
    };
    fs::create_dir_all(&world_dir)?;
    for entry in world_entries {
        let name = entry.file_name().expect("directory entries are named");
        fs::rename(&entry, world_dir.join(name))?;
    }

    let zip_path = package_dir.join("converted.zip");
    archive::create_zip_from_folders(&zip_path, &package_dir, &[zip_folder_name])?;
    Ok(zip_path)
}

fn zip_size_bytes(fs: &dyn FileSystem, path: &Path) -> Option<i64> {
    fs.read(path).ok().map(|bytes| bytes.len() as i64)
}

/// Copies a real-disk file (the packaged conversion output, always a
/// real file per the module doc — archive work bypasses the injectable
/// [`FileSystem`] the same way every other archive-touching function in
/// this phase already does) into a slot's zip location through `fs`, so
/// at least the destination half stays behind the same abstraction as
/// the metadata write beside it — the same convention `worlds::
/// copy_via_fs` established for a same-filesystem-on-both-sides copy,
/// adapted here for a source that's always real disk.
fn copy_packaged_zip_into_slot(
    fs: &dyn FileSystem,
    from_real_disk: &Path,
    to: &Path,
) -> io::Result<()> {
    let bytes = fs::read(from_real_disk)?;
    fs.write(to, &bytes)
}

/// `createConvertedSlot(name:zipURL:targetServer:targetLevelName:)`
/// (source line 226-256): a brand-new slot, `worldSeed` left unset
/// (source never infers or carries one through for a freshly converted
/// world). No cleanup on a mid-write failure — source has none either;
/// unlike `worlds::create_slot_from_current_world` (P6.12), no P6.7
/// fixture calls for one here, so this stays literal parity rather than
/// an uncalled-for correction.
fn create_converted_slot(
    fs: &dyn FileSystem,
    target_server_dir: &Path,
    name: &str,
    converted_zip: &Path,
    target_level_name: &str,
    now: &str,
) -> Result<WorldSlot, ConversionError> {
    let id = Uuid::new_v4().to_string().to_uppercase();
    let dir = world_store::slot_directory(target_server_dir, &id);
    fs.create_dir_all(&dir)?;
    let dest_zip = world_store::zip_path(target_server_dir, &id);
    copy_packaged_zip_into_slot(fs, converted_zip, &dest_zip)?;

    let slot = WorldSlot {
        id,
        name: name.to_string(),
        created_at: now.to_string(),
        last_played_at: None,
        thumbnail_file_name: None,
        world_level_name: Some(target_level_name.to_string()),
        world_seed: None,
        zip_size_bytes: zip_size_bytes(fs, &dest_zip),
    };
    world_store::save_metadata(fs, target_server_dir, &slot)?;
    Ok(slot)
}

/// `replaceSlotWithConvertedZip(existingSlot:zipURL:targetServer:
/// targetLevelName:)` (source line 258-283), **corrected** on Cameron's
/// call: source is a plain remove-then-copy straight to the destination
/// (`fixtures/world-conversion/
/// replace-existing-slot-overwrite-is-not-atomic-unlike-other-slot-mutations.json`),
/// so a write failure after the `remove` already succeeded leaves the
/// slot with no archive at all. This port instead stages the copy to a
/// scratch file in the same slot directory first — a failure there
/// leaves `dest_zip` (the destination's real archive) completely
/// untouched — and only removes/replaces the real destination once that
/// staged copy has already fully succeeded, matching the temp-file-
/// then-atomic-replace pattern every other overwrite in this phase uses
/// (`worlds::update_active_slot_from_current_world`, `worlds::
/// copy_slot_into_existing`), including their same remove-before-rename
/// shape (`fs::rename` doesn't overwrite an existing destination on
/// Windows the way it does on POSIX, so the explicit `remove` first is
/// still required even though the copy itself is now crash-safe).
fn replace_slot_with_converted_zip(
    fs: &dyn FileSystem,
    target_server_dir: &Path,
    existing_slot: &WorldSlot,
    converted_zip: &Path,
    target_level_name: &str,
) -> Result<WorldSlot, ConversionError> {
    let dir = world_store::slot_directory(target_server_dir, &existing_slot.id);
    fs.create_dir_all(&dir)?;
    let temp_zip = dir.join("world.convert.tmp.zip");
    let _ = fs.remove(&temp_zip);

    if let Err(e) = copy_packaged_zip_into_slot(fs, converted_zip, &temp_zip) {
        let _ = fs.remove(&temp_zip);
        return Err(e.into());
    }

    let dest_zip = world_store::zip_path(target_server_dir, &existing_slot.id);
    let _ = fs.remove(&dest_zip);
    fs.rename(&temp_zip, &dest_zip)?;

    let mut updated = existing_slot.clone();
    updated.world_level_name = Some(target_level_name.to_string());
    updated.zip_size_bytes = zip_size_bytes(fs, &dest_zip);
    world_store::save_metadata(fs, target_server_dir, &updated)?;
    Ok(updated)
}

/// `performWorldConversion(sourceSlot:sourceServer:targetServer:
/// targetFormat:placement:progressHandler:)` (source line 68-202). See
/// the module doc for the fakeable [`WorldConverter`] boundary, the two
/// preserved (not corrected) MSC 1 gaps, and the one precondition this
/// step's own plan text adds beyond the oracle.
///
/// - `is_source_running`/`is_target_running`: this step's own addition —
///   source enforces "stop the server before converting" only in its UI
///   layer (`WorldConversionWizardView`), never inside
///   `performWorldConversion` itself; folded in here per this phase's
///   established "orchestration-layer guard, one layer down" pattern.
/// - `target_raw_level_name`: the target server's own `server.properties`
///   `level-name` (Java) — `None` for Bedrock, matching every other
///   caller of [`world::current_level_name`] in this phase
///   (`worlds::read_java_level_name`'s own doc).
/// - `pre_conversion_backup`: the caller's already-performed target
///   safety backup (source's own step 6, `createBackup(for:targetServer,
///   isAutomatic:false,triggerReason:"pre-conversion")`) — a closure
///   rather than this function calling `backups::create_backup` itself,
///   the same decoupling `worlds::activate_slot`'s own `backup` parameter
///   already established, so this module stays independent of the
///   backups module's own many-argument surface. A `false` result is a
///   warning, not an abort — matching source exactly.
/// - `should_cancel` (P6.30): checked at entry and again immediately
///   before the Chunker process starts (the longest-running step) — see
///   [`ConversionError::Cancelled`].
#[allow(clippy::too_many_arguments)]
pub fn convert_world(
    fs: &dyn FileSystem,
    converter: &dyn WorldConverter,
    java_path_setting: &str,
    source_server_dir: &Path,
    source_slot: &WorldSlot,
    source_server_type: ServerType,
    is_source_running: bool,
    target_server_dir: &Path,
    target_server_type: ServerType,
    target_raw_level_name: Option<&str>,
    target_format: &str,
    placement: ConversionPlacement,
    is_target_running: bool,
    now: &str,
    pre_conversion_backup: impl FnOnce() -> bool,
    mut progress: impl FnMut(&str),
    should_cancel: impl Fn() -> bool,
) -> Result<WorldSlot, ConversionError> {
    if is_source_running || is_target_running {
        return Err(ConversionError::ServerRunning);
    }
    if should_cancel() {
        return Err(ConversionError::Cancelled);
    }

    // Source lines 79-82: java resolution is checked strictly before
    // jar-installed — preserve that exact precedence
    // (`fixtures/world-conversion/guard-order-java-path-checked-before-jar-installed.json`).
    let java_path = converter
        .resolve_java_path(java_path_setting)
        .ok_or(ConversionError::JavaNotFound)?;
    if !converter.is_installed() {
        return Err(ConversionError::ChunkerNotInstalled);
    }

    let target_level_name = world::current_level_name(target_server_type, target_raw_level_name);

    // Source lines 88-93: the placement's name is validated (trimmed)
    // before the source-zip existence check or any temp-directory setup
    // (`fixtures/world-conversion/guard-empty-new-slot-name-rejected-before-any-file-work.json`).
    if let ConversionPlacement::NewSlot { name } = &placement
        && name.trim().is_empty()
    {
        return Err(ConversionError::EmptyName);
    }

    // Source lines 96-99: still before the temp root is computed
    // (`fixtures/world-conversion/guard-missing-source-slot-archive-aborts-before-temp-dir.json`).
    let source_zip = world_store::zip_path(source_server_dir, &source_slot.id);
    if !matches!(fs.stat(&source_zip), Ok(meta) if meta.is_file) {
        return Err(ConversionError::NoSourceZip);
    }

    let temp_root = temp_conversion_root();
    let _cleanup = TempRootGuard(temp_root.clone());
    let unzip_dir = temp_root.join("source");
    let chunker_output_dir = temp_root.join("chunker_output");

    // Step 1: unzip the source slot. `source_zip` is read through `fs`
    // (which may be a fake in tests); the archive engine itself always
    // operates on real disk, the same bridge P6.11's
    // `live_folders_proven_identical_to_archive` already uses — so the
    // bytes are staged to a real scratch file first.
    progress("Extracting source world…");
    let source_zip_bytes = fs.read(&source_zip)?;
    fs::create_dir_all(&temp_root)?;
    let real_source_zip = temp_root.join("source.zip");
    fs::write(&real_source_zip, &source_zip_bytes)?;
    archive::extract_zip(&real_source_zip, &unzip_dir)?;
    let _ = fs::remove_file(&real_source_zip);

    // Step 2: locate the world folder inside the unzipped content.
    progress("Locating world data…");
    let input_world_dir = find_input_world_folder(
        &unzip_dir,
        source_server_type == ServerType::Bedrock,
        source_slot.world_level_name.as_deref(),
    )
    .ok_or(ConversionError::WorldFolderNotFound)?;
    if let Some(name) = input_world_dir.file_name().and_then(|n| n.to_str()) {
        progress(&format!("Found world: {name}"));
    }

    // Last chance to cancel before the longest-running step: nothing at
    // either server has been touched yet, only the temp working
    // directory, which `TempRootGuard` cleans up either way.
    if should_cancel() {
        return Err(ConversionError::Cancelled);
    }

    // Step 3: run Chunker.
    progress("Running Chunker conversion…");
    converter
        .convert(
            &input_world_dir,
            &chunker_output_dir,
            target_format,
            &java_path,
            &mut progress,
        )
        .map_err(ConversionError::ConversionFailed)?;

    // Step 4: package Chunker's output into a slot-compatible zip.
    progress("Packaging converted world…");
    let converted_zip = package_output(
        &chunker_output_dir,
        target_server_type == ServerType::Bedrock,
        &target_level_name,
    )?;

    // Step 5: place the zip into the target server's slot directory.
    progress("Placing converted world into target server…");
    let new_slot = match &placement {
        ConversionPlacement::NewSlot { name } => create_converted_slot(
            fs,
            target_server_dir,
            name.trim(),
            &converted_zip,
            &target_level_name,
            now,
        )?,
        ConversionPlacement::ReplaceExisting { slot } => replace_slot_with_converted_zip(
            fs,
            target_server_dir,
            slot,
            &converted_zip,
            &target_level_name,
        )?,
    };

    // Step 6: back up the target server's current world — warning-only
    // on failure, matching source exactly
    // (`fixtures/world-conversion/pre-conversion-backup-failure-only-warns-while-activation-failure-aborts-after-slot-already-written.json`).
    progress("Backing up target server's current world…");
    if !pre_conversion_backup() {
        progress("Warning: pre-conversion backup failed. Proceeding with activation.");
    }

    // Step 7: activate the new slot. Source calls `activateSlot` with
    // `backupCurrent: false, backupWorld: { _ in true }` since step 6
    // already (attempted to) back up — this port's `activate_slot`
    // folds "should I back up" into its own `backup` closure, so
    // `|| true` reproduces the same "already handled, report success"
    // shape. Every `ActivationError` variant collapses into the same
    // `ConversionFailed` message source itself uses, regardless of
    // cause — the already-written slot from step 5 is left on disk,
    // unactivated, exactly as source leaves it (see the module doc).
    progress("Activating converted world…");
    let activated = worlds::activate_slot(
        fs,
        target_server_dir,
        target_server_type,
        &new_slot,
        is_target_running,
        now,
        || true,
        &should_cancel,
    )
    .map_err(|error| {
        if matches!(error, worlds::ActivationError::Cancelled) {
            ConversionError::Cancelled
        } else {
            ConversionError::ConversionFailed(
                "Failed to activate converted world slot.".to_string(),
            )
        }
    })?;

    progress("Conversion complete.");
    Ok(activated)
}
