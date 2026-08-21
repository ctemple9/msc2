//! D-027's decided completion path for an author-blocked CurseForge file:
//! the client downloads the file itself and uploads it through MSC's
//! bounded staged-upload path; this module verifies the upload against
//! the exact pending file it's meant to complete and installs it.
//!
//! Ported from `CurseForgeManualDownloadSheet.swift`'s own three-tier
//! folder-watch filename matcher (P8.8's characterization,
//! `fixtures/curseforge-manual-download/`), translated to the staged-
//! upload contract `msc2-decisions.md`'s D-027 entry records: the same
//! tolerance (exact match, a macOS duplicate-download suffix, or a
//! single-remaining-candidate fallback) decides whether an uploaded file
//! is accepted as the completion for one specific pending operation —
//! never a generic "any file with a plausible name" acceptance.
//!
//! **Staged-upload redemption is the route layer's job (P8.24), the same
//! boundary every other module in this phase has already established** —
//! [`complete_pending_file`] takes an already-redeemed local path and the
//! filename the client's upload declared; it never touches
//! `POST /v1/staged-uploads` itself.
//!
//! **Size ceiling is per-file, not a flat cap** — `PendingManualFile::
//! expected_byte_size` is CurseForge's own reported file size for this
//! exact `fileID` (from `curseforge_files`'s response), per P8.9's own
//! contract note ("sized to that file's own CurseForge-reported byte
//! length, not a flat ceiling"). A mismatch is rejected outright — this
//! module never installs a file whose size doesn't match what CurseForge
//! itself reported for the pending file id, since a size match is the one
//! integrity signal available at all (CurseForge publishes no per-file
//! hash in this port's own model, confirmed absent from
//! `msc_domain::addon_provider::CurseForgeFile`).

use std::fmt;
use std::path::{Path, PathBuf};

use msc_infrastructure::archive;
use msc_infrastructure::fs::FileSystem;

/// One CurseForge file this pending pack operation is still waiting on a
/// manual upload for — bound to a specific `project_id`/`file_id`, never a
/// generic "any pending file" slot (`fixtures/curseforge-manual-download/
/// staged-upload-purpose-bound-to-one-specific-pending-operation-not-generic.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingManualFile {
    pub project_id: i64,
    pub file_id: i64,
    pub expected_file_name: String,
    pub expected_byte_size: u64,
    /// Where the file lands once accepted — inside the server's own
    /// add-on folder, resolved by the caller the same way every other
    /// install path in this phase resolves a destination.
    pub dest: PathBuf,
}

#[derive(Debug)]
pub enum ManualFileError {
    /// Neither an exact, duplicate-suffix, nor (when this is the only
    /// remaining pending file) fallback match — the upload is rejected
    /// rather than silently substituted for the wrong pending file
    /// (`fixtures/curseforge-manual-download/
    /// staged-upload-rejects-wrong-file-does-not-silently-substitute.json`).
    FilenameMismatch,
    SizeMismatch {
        expected: u64,
        actual: u64,
    },
    /// The uploaded bytes don't open as a real zip/jar archive at all.
    NotAValidJar,
    Io(String),
}

impl fmt::Display for ManualFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FilenameMismatch => write!(f, "uploaded file does not match the pending file"),
            Self::SizeMismatch { expected, actual } => {
                write!(f, "uploaded file is {actual} bytes, expected {expected}")
            }
            Self::NotAValidJar => write!(f, "uploaded file is not a valid jar/zip archive"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ManualFileError {}

/// Tier 1: an exact, case-insensitive filename match.
fn is_exact_match(expected: &str, actual: &str) -> bool {
    actual.eq_ignore_ascii_case(expected)
}

/// Tier 2: a macOS browser duplicate-download suffix — `"Foo (2).jar"`
/// matching an expected `"Foo.jar"` — requires the same extension on both
/// sides (`fixtures/curseforge-manual-download/
/// tier2-duplicate-suffix-match-requires-same-extension.json`).
fn strip_macos_duplicate_suffix(name: &str) -> Option<String> {
    let (stem, ext) = name.rsplit_once('.')?;
    let open = stem.rfind(" (")?;
    if !stem.ends_with(')') {
        return None;
    }
    let digits = &stem[open + 2..stem.len() - 1];
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(format!("{}.{ext}", &stem[..open]))
}

fn is_duplicate_suffix_match(expected: &str, actual: &str) -> bool {
    strip_macos_duplicate_suffix(actual)
        .map(|stripped| stripped.eq_ignore_ascii_case(expected))
        .unwrap_or(false)
}

/// Whether `actual` is accepted as a completion for a pending file whose
/// declared name is `expected`. `is_only_remaining_pending` is Tier 3: a
/// single-candidate fallback, only ever consulted when the caller has
/// exactly one pending file left waiting on exactly one uploaded file
/// (`fixtures/curseforge-manual-download/
/// staged-upload-one-file-fallback-when-exactly-one-file-uploaded-for-a-single-pending-file.json`)
/// — never applied when either side has more than one candidate.
pub fn filename_matches(expected: &str, actual: &str, is_only_remaining_pending: bool) -> bool {
    is_exact_match(expected, actual)
        || is_duplicate_suffix_match(expected, actual)
        || is_only_remaining_pending
}

/// Verifies an already-redeemed staged upload against `pending` and, if it
/// passes every check, installs it at `pending.dest`. Every check runs
/// before any byte is written: a rejected upload never touches the
/// destination.
pub fn complete_pending_file(
    fs: &dyn FileSystem,
    staged_local_path: &Path,
    staged_filename: &str,
    pending: &PendingManualFile,
    is_only_remaining_pending: bool,
) -> Result<PathBuf, ManualFileError> {
    if !filename_matches(
        &pending.expected_file_name,
        staged_filename,
        is_only_remaining_pending,
    ) {
        return Err(ManualFileError::FilenameMismatch);
    }
    let meta = fs
        .stat(staged_local_path)
        .map_err(|e| ManualFileError::Io(e.to_string()))?;
    if meta.size != pending.expected_byte_size {
        return Err(ManualFileError::SizeMismatch {
            expected: pending.expected_byte_size,
            actual: meta.size,
        });
    }
    if archive::list_entry_names(staged_local_path).is_err() {
        return Err(ManualFileError::NotAValidJar);
    }

    if let Some(parent) = pending.dest.parent() {
        fs.create_dir_all(parent)
            .map_err(|e| ManualFileError::Io(e.to_string()))?;
    }
    let bytes = fs
        .read(staged_local_path)
        .map_err(|e| ManualFileError::Io(e.to_string()))?;
    fs.write(&pending.dest, &bytes)
        .map_err(|e| ManualFileError::Io(e.to_string()))?;
    Ok(pending.dest.clone())
}
