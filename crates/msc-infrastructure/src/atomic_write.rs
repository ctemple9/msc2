//! `atomic_write`: the shared temp-file-then-rename primitive.
//!
//! MSC 1 rebuilds this pattern at every call site instead of sharing it:
//! `ConfigManager.save` writes its JSON via `Data.write(options: [.atomic])`
//! (`ConfigManager.swift:215`); `WorldSlotManager.createSlot` and
//! `.updateSlotFromCurrentWorld` zip to a `*.tmp.zip` path and only
//! `moveItem` it over the real destination once the zip fully succeeds
//! (`WorldSlotManager.swift:391`, `:470`, `:486`, `:524`); the deferred
//! `AppViewModel+WorldSlots.restoreSlotBackup` (`:407`) does the same for
//! restoring a slot's `world.zip` from a backup. `atomic_write` is that one
//! pattern, built once on P3.4's `FileSystem` trait, for every later
//! config/metadata/world writer to call instead of reimplementing it.
//!
//! Unlike `ConfigManager.save`, this primitive does not create a missing
//! parent directory — MSC 1's other call sites (`WorldSlotManager`'s slot
//! writers) always write into a directory the caller already created, so a
//! missing parent here is a caller bug to surface, not paper over.

use crate::fs::FileSystem;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum AtomicWriteError {
    /// `path`'s parent directory doesn't exist (or isn't a directory).
    /// Carries the parent path, not `path` itself, since that's what's
    /// actually missing.
    MissingParentDirectory(PathBuf),
    /// The temp-file write or the rename step failed for some other
    /// reason (permissions, disk full, ...).
    Io(io::Error),
}

impl fmt::Display for AtomicWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtomicWriteError::MissingParentDirectory(parent) => {
                write!(f, "parent directory {} does not exist", parent.display())
            }
            AtomicWriteError::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for AtomicWriteError {}

/// Writes `contents` to `path` without ever leaving a partially-written
/// file at `path` itself: writes to a temp path in the same directory
/// first, then renames it over `path`. A rename within one directory is
/// what makes the swap atomic — both `std::fs::rename` and the fake
/// filesystem's rename replace the destination in a single step, so a
/// reader never observes a half-written `path`.
///
/// Requires `path`'s parent directory to already exist; see the module
/// docs for why this doesn't create it.
pub fn atomic_write(
    fs: &dyn FileSystem,
    path: &Path,
    contents: &[u8],
) -> Result<(), AtomicWriteError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| AtomicWriteError::MissingParentDirectory(PathBuf::new()))?;

    match fs.stat(parent) {
        Ok(meta) if meta.is_dir => {}
        _ => {
            return Err(AtomicWriteError::MissingParentDirectory(
                parent.to_path_buf(),
            ));
        }
    }

    let temp = temp_path_for(path);
    fs.write(&temp, contents).map_err(AtomicWriteError::Io)?;

    fs.rename(&temp, path).map_err(|err| {
        // The rename failed — don't leave the temp file behind for the
        // caller to trip over on a retry.
        let _ = fs.remove(&temp);
        AtomicWriteError::Io(err)
    })
}

/// The temp path `atomic_write` writes to before renaming over `path` —
/// exposed so tests (and any caller that wants to pre-clean a stale temp
/// file) can predict it without duplicating the naming rule.
pub fn temp_path_for(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{file_name}.tmp"))
}
