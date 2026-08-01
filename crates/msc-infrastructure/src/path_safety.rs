//! `safe_path`: the approved-server-root path-safety primitive.
//!
//! Ported from two MSC 1 functions that never shared code but enforce the
//! same idea: `resolvedServerFileURL`
//! (`AppViewModel+APIWiringContent.swift:41`), which resolves a relative
//! path against a server's root and refuses anything that escapes it, and
//! `validateResetDeletionTarget` (`AppViewModel+ConfigHelpers.swift:169`),
//! which refuses to treat `/`, the home directory, or similar catch-all
//! paths as a legitimate root at all. `safe_path` folds both checks into
//! one call so a future caller gets both for free instead of remembering
//! to apply the second one only for destructive operations.

use crate::fs::FileSystem;
use std::fmt;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSafetyError {
    /// `root` itself is a filesystem root or the caller's home directory —
    /// never a legitimate server root or deletion target, no matter what
    /// request produced it.
    ForbiddenRoot(PathBuf),
    /// The resolved candidate falls outside `root` once symlinks are
    /// followed and `.`/`..` are collapsed.
    Escape { root: PathBuf, candidate: PathBuf },
    /// Following symlinks didn't terminate within a sane number of hops.
    /// Not exercised by any of P3.5's fixtures (none describe a loop) —
    /// present because `resolve` walks the real filesystem in production
    /// and an unguarded symlink loop there is an infinite loop, not a
    /// hypothetical.
    SymlinkLoop(PathBuf),
}

impl fmt::Display for PathSafetyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathSafetyError::ForbiddenRoot(root) => {
                write!(f, "{} is not a valid server root", root.display())
            }
            PathSafetyError::Escape { root, candidate } => {
                write!(f, "{} escapes root {}", candidate.display(), root.display())
            }
            PathSafetyError::SymlinkLoop(path) => {
                write!(
                    f,
                    "too many levels of symbolic links resolving {}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for PathSafetyError {}

const MAX_SYMLINK_DEPTH: u8 = 32;

/// Resolves `requested` (a relative path, or `None`/empty for the root
/// itself) against `root`, the way `resolvedServerFileURL` did, and also
/// refuses a `root` that is itself a filesystem root or `home_dir`, the
/// way `validateResetDeletionTarget` did for deletion targets.
///
/// Both checks apply on every call: browsing files needs the escape check
/// and can't legitimately have `/` as a root anyway, and validating a
/// deletion target needs the forbidden-root check but is naturally an
/// empty-request call (candidate == root) into the same escape logic — one
/// primitive serves both call sites.
pub fn safe_path(
    fs: &dyn FileSystem,
    root: &Path,
    requested: Option<&str>,
    home_dir: &Path,
) -> Result<PathBuf, PathSafetyError> {
    let root_resolved = resolve(fs, &lexically_normalize(root))?;
    if is_forbidden_root(&root_resolved, home_dir) {
        return Err(PathSafetyError::ForbiddenRoot(root_resolved));
    }

    let trimmed = requested.unwrap_or("").trim();
    let candidate_raw = if trimmed.is_empty() {
        root.to_path_buf()
    } else {
        root.join(trimmed)
    };
    let candidate_resolved = resolve(fs, &lexically_normalize(&candidate_raw))?;

    if candidate_resolved == root_resolved || candidate_resolved.starts_with(&root_resolved) {
        Ok(candidate_resolved)
    } else {
        Err(PathSafetyError::Escape {
            root: root_resolved,
            candidate: candidate_resolved,
        })
    }
}

/// A path with no parent — `/` on Unix, a bare drive root like `C:\` on
/// Windows — is never a legitimate server root or deletion target, the
/// same guard `validateResetDeletionTarget` hardcoded as `path != "/"`.
/// Checking "has no parent" instead of the literal string `"/"` covers the
/// same case without assuming Unix.
fn is_forbidden_root(root: &Path, home_dir: &Path) -> bool {
    root.parent().is_none() || root == home_dir
}

/// Collapses `.` and `..` components without touching the filesystem —
/// Foundation's `standardizedFileURL`, run before `resolvingSymlinksInPath`
/// in both source functions.
fn lexically_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    // No parent left to pop (already at a root, or
                    // relative and empty) — keep the `..` rather than
                    // silently dropping it.
                    out.push(component);
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// Walks `path` component by component, following any symlink found along
/// the way — mirroring Foundation's `resolvingSymlinksInPath()`. A
/// component that doesn't exist, or isn't a symlink, is kept as-is rather
/// than treated as an error: the candidate a caller asks about need not
/// exist yet (e.g. a file about to be written).
fn resolve(fs: &dyn FileSystem, path: &Path) -> Result<PathBuf, PathSafetyError> {
    resolve_inner(fs, path, 0)
}

fn resolve_inner(fs: &dyn FileSystem, path: &Path, depth: u8) -> Result<PathBuf, PathSafetyError> {
    if depth > MAX_SYMLINK_DEPTH {
        return Err(PathSafetyError::SymlinkLoop(path.to_path_buf()));
    }
    let mut resolved = PathBuf::new();
    for component in path.components() {
        resolved.push(component);
        if let Ok(target) = fs.read_link(&resolved) {
            let joined = if target.is_absolute() {
                target
            } else {
                resolved
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("/"))
                    .join(target)
            };
            resolved = resolve_inner(fs, &lexically_normalize(&joined), depth + 1)?;
        }
    }
    Ok(resolved)
}
