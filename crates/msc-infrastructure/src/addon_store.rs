//! Verified add-on staging and atomic replacement: the download-and-write
//! half of a jar install/update, and the on-disk enable/disable/remove
//! primitives every add-on mutation (P8.17), dependency install (P8.15),
//! and modpack import (P8.18-P8.20) needs.
//!
//! `docs/msc2/rolling-plan.md`'s own P8.14 working text: "stage into an
//! operation-owned temporary area, enforce the publisher hash when
//! present, reject hostile archive paths/symlinks, preserve executable/
//! read permission bits where the pack supplies them, and atomically
//! install/replace/remove/toggle JARs without clobbering an existing
//! `.disabled` target." Each half of that sentence has its own home:
//!
//! - **Staging + hash + atomic write**: [`install_verified_file`] composes
//!   [`crate::addon_provider::AddonTransport::get`] with
//!   [`crate::download_staging::stage_download`] the same way
//!   `jar_provider.rs`'s per-family download functions do (P7.13/P7.35) —
//!   `stage_download` IS the "operation-owned temporary area" (a
//!   dot-prefixed sibling of `dest`, atomically renamed over it) this
//!   phase's own `fixtures/modpack-archive-safety/
//!   corrupt-or-truncated-download-written-directly-to-final-path-...`
//!   finding says MSC 1 never had.
//! - **Hostile archive paths/symlinks**: already closed by two existing
//!   primitives this step composes rather than reimplements —
//!   [`crate::archive::extract_zip`] (traversal/symlink/size-capped
//!   extraction, P6.5) for zip-entry-driven override trees, and
//!   [`crate::path_safety::safe_path`] (traversal/symlink-escape
//!   resolution, P3.5) for a manifest-declared per-file relative path.
//!   [`resolve_pack_file_dest`] is the one new wrapper this step adds, so
//!   P8.18/P8.19's `.mrpack`/CurseForge per-file downloads get the
//!   `no-path-traversal-guard-on-manifest-declared-file-path` fixture's
//!   gap closed without re-deriving the check.
//! - **Executable/read permission bits**: `archive.rs`'s own
//!   `apply_executable_bit`, added by this same step (see that file's own
//!   doc comment) — extraction, not this module's job.
//! - **Install/replace/remove/toggle without clobbering `.disabled`**: the
//!   rest of this module. "Replace" has no function of its own: writing a
//!   new version to the SAME `dest` is just [`install_verified_file`]
//!   again (`stage_download`'s rename already overwrites atomically); a
//!   version-bump that changes the filename is P8.17's rekey policy
//!   (already characterized in P8.11's `plugin_source_mapping` port) —
//!   this module only gives it [`remove_addon_jar`] to drop the stale
//!   name with.
//!
//! Two distinct disable/toggle call sites exist in MSC 1, with two
//! genuinely different collision policies, and this module keeps both
//! rather than collapsing them into one shared function:
//!
//! - [`toggle_addon_jar`] ports `toggleMod`'s rename
//!   (`AppViewModel+ModManagement.swift:192-211`): a plain
//!   `.jar` <-> `.jar.disabled` rename that **errors** if the target
//!   already exists — MSC 1's own `FileManager.moveItem` throws on
//!   collision there, and nothing catches it into a silent drop.
//! - [`disable_for_classification`] ports `ModpackClientOnlyClassifier.
//!   disableJar` (`ModpackClientOnlyClassifier.swift:131-146`) via
//!   `msc_domain::modpack::decide_disable_jar_action` (P8.12): the
//!   modpack-classification path, which **silently drops** a
//!   freshly-downloaded active duplicate rather than clobbering an
//!   already-existing `.disabled` sibling.

use std::fmt;
use std::path::{Path, PathBuf};

use msc_domain::modpack::{self, DisableJarAction};

use crate::addon_provider::{AddonTransport, TransportError};
use crate::download_staging::{self, CachedFile, DownloadStagingError, ExpectedChecksum};
use crate::fs::FileSystem;
use crate::path_safety::{self, PathSafetyError};

/// Real add-on jars run a few KB to low tens of MB; modpack override files
/// (configs, resource packs bundled as an override) can run larger. 300 MB
/// matches `jar_provider::JAR_MAX_BYTES`'s own headroom rationale rather
/// than inventing a second number.
pub const ADDON_FILE_MAX_BYTES: u64 = 300 * 1024 * 1024;

/// The suffix `toggleMod`/`disableJar` both use to mark a jar disabled —
/// appended, never substituted for `.jar`'s own extension (`disabledURL`'s
/// own `appendingPathExtension` semantics, `msc_domain::modpack::
/// disabled_url`).
pub const DISABLED_SUFFIX: &str = ".disabled";

#[derive(Debug)]
pub enum AddonStoreError {
    Transport(TransportError),
    /// The download itself completed, but with a non-2xx status — a jar
    /// download has no provider-specific status meaning the way a
    /// metadata endpoint does (P8.13's own doc comment), so this is
    /// always a plain failure.
    DownloadFailed(u16),
    Staging(DownloadStagingError),
    PathSafety(PathSafetyError),
    /// [`toggle_addon_jar`]'s target already exists — the collision
    /// [`toggleMod`] surfaces as a thrown (and merely logged) error rather
    /// than ever overwriting.
    AlreadyExists(PathBuf),
    /// `current`'s file name doesn't end in `.jar`/`.jar.disabled` at all,
    /// so no sibling toggle path exists to compute.
    NotAJarPath(PathBuf),
    Io(std::io::Error),
}

impl fmt::Display for AddonStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddonStoreError::Transport(e) => write!(f, "{e}"),
            AddonStoreError::DownloadFailed(status) => {
                write!(f, "add-on download returned status {status}.")
            }
            AddonStoreError::Staging(e) => write!(f, "{e}"),
            AddonStoreError::PathSafety(e) => write!(f, "{e}"),
            AddonStoreError::AlreadyExists(path) => {
                write!(f, "{} already exists.", path.display())
            }
            AddonStoreError::NotAJarPath(path) => {
                write!(f, "{} is not a .jar/.jar.disabled path.", path.display())
            }
            AddonStoreError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AddonStoreError {}

/// Downloads `download_url` through `transport`, verifying
/// `expected_checksum` when the caller has one (Modrinth publishes a
/// `sha1`/`sha512` per file; CurseForge/Hangar/GitHub/direct publish none
/// in this port's own model — `None` there, same as `jar_provider.rs`'s
/// own Fabric/NeoForge/Forge callers), and atomically stages the result at
/// `dest`. One function serves both a single add-on jar install/update
/// (P8.15/P8.17) and one modpack-manifest-declared file download
/// (P8.18-P8.20) — the shape is identical, only `dest`/`expected_checksum`
/// differ per caller.
pub fn install_verified_file(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    download_url: &str,
    version_label: &str,
    expected_checksum: Option<&ExpectedChecksum>,
    dest: &Path,
) -> Result<CachedFile, AddonStoreError> {
    let response = transport
        .get(download_url, "add-on download", &[], ADDON_FILE_MAX_BYTES)
        .map_err(AddonStoreError::Transport)?;
    if !(200..300).contains(&response.status) {
        return Err(AddonStoreError::DownloadFailed(response.status));
    }
    download_staging::stage_download(
        fs,
        dest,
        &response.body,
        download_url,
        version_label,
        expected_checksum,
    )
    .map_err(AddonStoreError::Staging)
}

/// Removes one add-on jar (enabled or disabled) at `path`.
pub fn remove_addon_jar(fs: &dyn FileSystem, path: &Path) -> Result<(), AddonStoreError> {
    fs.remove(path).map_err(AddonStoreError::Io)
}

/// `current`'s sibling with enabled/disabled state flipped: `foo.jar` <->
/// `foo.jar.disabled`. `None` when `current`'s file name is empty or not
/// valid UTF-8.
pub fn toggled_path(current: &Path) -> Option<PathBuf> {
    let name = current.file_name()?.to_str()?;
    if let Some(stripped) = name.strip_suffix(DISABLED_SUFFIX) {
        Some(current.with_file_name(stripped))
    } else {
        Some(current.with_file_name(format!("{name}{DISABLED_SUFFIX}")))
    }
}

/// `toggleMod`'s rename (`AppViewModel+ModManagement.swift:205`): renames
/// `current` to its toggled sibling, refusing (rather than clobbering) if
/// something is already there. Returns the new path on success.
pub fn toggle_addon_jar(fs: &dyn FileSystem, current: &Path) -> Result<PathBuf, AddonStoreError> {
    let target =
        toggled_path(current).ok_or_else(|| AddonStoreError::NotAJarPath(current.to_path_buf()))?;
    if fs.stat(&target).is_ok() {
        return Err(AddonStoreError::AlreadyExists(target));
    }
    fs.rename(current, &target).map_err(AddonStoreError::Io)?;
    Ok(target)
}

/// What [`disable_for_classification`] actually did, mirroring
/// `msc_domain::modpack::DisableJarAction`'s three outcomes with the
/// filesystem effect each one produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisableOutcome {
    /// No active jar existed at that path; nothing changed.
    NoOp,
    /// The active jar was renamed to `.disabled`.
    Disabled(PathBuf),
    /// A `.disabled` sibling already existed; the active duplicate was
    /// dropped and the existing `.disabled` file was left untouched.
    DroppedActiveKeptExistingDisabled,
}

/// `ModpackClientOnlyClassifier.disableJar(at:fm:)`
/// (`ModpackClientOnlyClassifier.swift:131-146`) via
/// `msc_domain::modpack::decide_disable_jar_action` (P8.12): the
/// modpack-classification disable path, distinct from [`toggle_addon_jar`]
/// — see this module's own doc comment for why the two collision policies
/// differ.
pub fn disable_for_classification(
    fs: &dyn FileSystem,
    active_path: &Path,
) -> Result<DisableOutcome, AddonStoreError> {
    let disabled_path = PathBuf::from(modpack::disabled_url(&active_path.to_string_lossy()));
    let active_exists = fs.stat(active_path).map(|m| m.is_file).unwrap_or(false);
    let disabled_exists = fs.stat(&disabled_path).map(|m| m.is_file).unwrap_or(false);

    match modpack::decide_disable_jar_action(active_exists, disabled_exists) {
        DisableJarAction::NoOp => Ok(DisableOutcome::NoOp),
        DisableJarAction::Rename => {
            fs.rename(active_path, &disabled_path)
                .map_err(AddonStoreError::Io)?;
            Ok(DisableOutcome::Disabled(disabled_path))
        }
        DisableJarAction::DropActiveKeepExistingDisabled => {
            fs.remove(active_path).map_err(AddonStoreError::Io)?;
            Ok(DisableOutcome::DroppedActiveKeptExistingDisabled)
        }
    }
}

/// Resolves one manifest-declared relative file path (a `.mrpack`
/// `MrpackFile.path`, or a CurseForge override entry) against `server_dir`,
/// refusing traversal/absolute paths and a symlink-mediated escape —
/// `fixtures/modpack-archive-safety/
/// no-path-traversal-guard-on-manifest-declared-file-path.json`'s own gap,
/// closed by composing the existing `path_safety::safe_path` primitive
/// rather than re-deriving traversal checking here.
pub fn resolve_pack_file_dest(
    fs: &dyn FileSystem,
    server_dir: &Path,
    manifest_relative_path: &str,
    home_dir: &Path,
) -> Result<PathBuf, AddonStoreError> {
    path_safety::safe_path(fs, server_dir, Some(manifest_relative_path), home_dir)
        .map_err(AddonStoreError::PathSafety)
}
