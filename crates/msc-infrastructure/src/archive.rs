//! `create_zip_from_folders`/`extract_zip`: the D-006-corrected world
//! archive primitives P6.10 builds ahead of any destructive live-world
//! swap (that's P6.12+'s `activateSlot` job).
//!
//! MSC 1's `WorldSlotManager` shells out to `/usr/bin/zip -r`/
//! `/usr/bin/unzip -o` with no entry-path, entry-type, or size inspection
//! at all (`createSlot`, `activateSlot`, `createSlotFromZIP`'s own doc
//! comment: "no structural validation is enforced here"). This module is
//! the D-006 correction `fixtures/world-archive-safety/` characterizes
//! (P6.5): every entry name is validated against traversal (both `/` and
//! `\`-separated, on every platform, not just when running on Windows),
//! absolute paths, and Windows drive-absolute paths before extraction;
//! every entry's type/mode is checked and any symlink is refused outright
//! regardless of target; the declared entry count and total declared
//! uncompressed size are checked against fixed ceilings using the zip's
//! own central directory metadata *before* a single byte is decompressed;
//! and the whole archive is dry-run decompressed to a sink (validating
//! every entry's data against its declared CRC/size) before any byte is
//! written to `dest_root` — so a corrupt archive (central directory and
//! local file data disagreeing) is refused with zero bytes written, not
//! discovered partway through extraction with the destination already
//! half-populated. Every rejection is all-or-nothing at the archive
//! level, matching `fixtures/path-safety`'s own escape-detection
//! precedent.
//!
//! No fixture pins an exact entry-count/size ceiling — the two "exceeded"
//! fixtures use values many orders of magnitude past any legitimate world
//! archive, and the positive-control fixture is many orders of magnitude
//! under — so [`MAX_ARCHIVE_ENTRIES`]/[`MAX_TOTAL_UNCOMPRESSED_BYTES`] are
//! this port's own fixed, documented ceilings, not values read off the
//! oracle.

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// A real Java world's region/entity/poi/data files number in the low
/// thousands at most (`fixtures/world-archive-safety/
/// extraction-entry-count-limit-exceeded-rejected.json`'s own reasoning);
/// this ceiling is generous headroom above that, not a tight fit.
pub const MAX_ARCHIVE_ENTRIES: u64 = 200_000;

/// 8 GiB: comfortably above any real world/backup archive this project's
/// own corpus (`corpus/worlds/`, `corpus/backups/`) has ever produced,
/// comfortably below "fills the disk."
pub const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 8 * 1024 * 1024 * 1024;

/// [`extract_zip`]'s ceilings, factored out so tests can exercise the
/// "exceeded" branch against a small real archive and a small limit
/// rather than needing to actually construct a multi-GB or
/// million-entry zip on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveLimits {
    pub max_entries: u64,
    pub max_total_uncompressed_bytes: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_entries: MAX_ARCHIVE_ENTRIES,
            max_total_uncompressed_bytes: MAX_TOTAL_UNCOMPRESSED_BYTES,
        }
    }
}

#[derive(Debug)]
pub enum ArchiveError {
    Open(io::Error),
    /// The zip's central directory failed to parse, or an entry's local
    /// file data disagrees with its central directory record (corrupt
    /// archive) — carries the underlying library's message.
    Corrupt(String),
    /// An entry's name escapes `dest_root` (traversal, absolute, or
    /// Windows drive-absolute) or is a symlink.
    UnsafeEntry(String),
    EntryCountExceeded {
        declared: u64,
        limit: u64,
    },
    TotalSizeExceeded {
        declared: u64,
        limit: u64,
    },
    Io(io::Error),
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveError::Open(e) => write!(f, "could not open archive: {e}"),
            ArchiveError::Corrupt(msg) => write!(f, "corrupt archive: {msg}"),
            ArchiveError::UnsafeEntry(name) => write!(f, "unsafe archive entry: {name}"),
            ArchiveError::EntryCountExceeded { declared, limit } => {
                write!(
                    f,
                    "extraction refused: {declared} entries exceeds limit of {limit}"
                )
            }
            ArchiveError::TotalSizeExceeded { declared, limit } => write!(
                f,
                "extraction refused: {declared} declared uncompressed bytes exceeds limit of {limit}"
            ),
            ArchiveError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ArchiveError {}

/// Whether `name` is a plain relative path fully confined under whatever
/// root it's extracted into — normalizing both `/` and `\` as path
/// separators regardless of host platform, since an archive can be built
/// on one platform and extracted on another
/// (`windows-backslash-traversal-rejected.json`).
fn is_safe_archive_entry_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    if name.starts_with('/') || name.starts_with('\\') {
        return false;
    }
    // Windows drive-absolute: "C:\..." or "C:/...".
    let mut chars = name.chars();
    if let (Some(first), Some(':')) = (chars.next(), chars.next())
        && first.is_ascii_alphabetic()
    {
        return false;
    }
    name.split(['/', '\\']).all(|part| part != "..")
}

/// A world folder produced by a real Minecraft server or by this port's
/// own [`create_zip_from_folders`] never legitimately contains a symlink
/// entry — any symlink entry is itself evidence of a hand-crafted or
/// corrupted input, so the whole class is refused regardless of where its
/// target points (`symlink-entry-any-target-rejected-outright.json`).
fn is_symlink_unix_mode(mode: Option<u32>) -> bool {
    matches!(mode, Some(mode) if mode & 0o170000 == 0o120000)
}

/// Builds `dest_root.join(name)` from `name`'s components, split on both
/// `/` and `\` regardless of host platform — never `Path::new(name)` or a
/// bare `dest_root.join(name)`, both of which parse separators using the
/// *host's* convention and would treat a backslash as a literal filename
/// character on Unix. Only called after [`is_safe_archive_entry_name`]
/// has already proven every component is a plain, non-`..` segment.
fn safe_join(dest_root: &Path, name: &str) -> PathBuf {
    let mut path = dest_root.to_path_buf();
    for part in name.split(['/', '\\']) {
        if !part.is_empty() {
            path.push(part);
        }
    }
    path
}

/// The read-only half of [`extract_zip_with_limits`] — declared-metadata
/// checks (entry count, name safety, symlink mode, total declared
/// uncompressed size) plus a no-write dry-run decompression proving every
/// entry's local file data matches its central directory record — without
/// writing a single byte anywhere. Factored out of extraction (rather than
/// extraction calling a would-be separate scan) so a caller that only
/// wants to know "would this archive be safe/complete to extract" — a
/// backup ZIP being listed, verified after creation, or checked for
/// restore-eligibility (P6.15's `backup_store::list_backups`, P6.16's
/// post-creation verification, P6.18's restore gate) — never has to
/// extract-to-a-scratch-dir just to answer that question.
pub fn validate_archive_safety(zip_path: &Path) -> Result<(), ArchiveError> {
    validate_archive_safety_with_limits(zip_path, ArchiveLimits::default()).map(|_| ())
}

/// Same as [`validate_archive_safety`], with caller-supplied ceilings.
/// Returns the already-opened archive handle so [`extract_zip_with_limits`]
/// can reuse it for its own write pass without reopening and rescanning
/// the same file.
fn validate_archive_safety_with_limits(
    zip_path: &Path,
    limits: ArchiveLimits,
) -> Result<ZipArchive<fs::File>, ArchiveError> {
    let file = fs::File::open(zip_path).map_err(ArchiveError::Open)?;
    let mut archive = ZipArchive::new(file).map_err(|e| ArchiveError::Corrupt(e.to_string()))?;

    let entry_count = archive.len() as u64;
    if entry_count > limits.max_entries {
        return Err(ArchiveError::EntryCountExceeded {
            declared: entry_count,
            limit: limits.max_entries,
        });
    }

    // Pass 0: declared-metadata-only checks (name safety, symlink mode,
    // total declared uncompressed size) — no decompression at all yet.
    let mut total_uncompressed: u64 = 0;
    for i in 0..archive.len() {
        let raw = archive
            .by_index_raw(i)
            .map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
        let name = raw.name().to_string();
        if !is_safe_archive_entry_name(&name) {
            return Err(ArchiveError::UnsafeEntry(name));
        }
        if is_symlink_unix_mode(raw.unix_mode()) {
            return Err(ArchiveError::UnsafeEntry(name));
        }
        total_uncompressed = total_uncompressed.saturating_add(raw.size());
        if total_uncompressed > limits.max_total_uncompressed_bytes {
            return Err(ArchiveError::TotalSizeExceeded {
                declared: total_uncompressed,
                limit: limits.max_total_uncompressed_bytes,
            });
        }
    }

    // Pass 1: dry-run decompression to a sink — proves every entry's
    // local file data matches its central directory record (catches a
    // corrupt/truncated archive) without writing anything to `dest_root`.
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
        io::copy(&mut entry, &mut io::sink()).map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
    }

    Ok(archive)
}

/// Extracts every entry of the zip at `zip_path` into `dest_root`.
/// All-or-nothing: any unsafe entry, any limit exceeded, or any corrupt
/// entry refuses the whole archive before writing anything to
/// `dest_root`. See the module doc for the three-pass shape (declared-
/// metadata checks, then a no-write dry-run decompression, then the real
/// extraction) that makes "corrupt archive" and "unsafe/oversized
/// archive" both zero-bytes-written outcomes, not partial ones.
pub fn extract_zip(zip_path: &Path, dest_root: &Path) -> Result<(), ArchiveError> {
    extract_zip_with_limits(zip_path, dest_root, ArchiveLimits::default())
}

/// Same as [`extract_zip`], with caller-supplied ceilings instead of the
/// crate's own [`ArchiveLimits::default`].
pub fn extract_zip_with_limits(
    zip_path: &Path,
    dest_root: &Path,
    limits: ArchiveLimits,
) -> Result<(), ArchiveError> {
    let mut archive = validate_archive_safety_with_limits(zip_path, limits)?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
        let name = entry.name().to_string();
        let dest = safe_join(dest_root, &name);
        if entry.is_dir() {
            fs::create_dir_all(&dest).map_err(ArchiveError::Io)?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(ArchiveError::Io)?;
        }
        let mut out = fs::File::create(&dest).map_err(ArchiveError::Io)?;
        io::copy(&mut entry, &mut out).map_err(ArchiveError::Io)?;
    }

    Ok(())
}

/// The zip's own member listing, in its internal (not sorted) order —
/// mirrors `unzip -Z -1`'s output, the listing MSC 1's
/// `inferJavaLevelName(fromSlotZIP:)`/`firstLevelDatPath(inZIP:)` both
/// shell out for (`WorldSlotManager.swift:192-199`, `1333-1345`). Used
/// only to make an import-time naming/seed *guess* — never to decide
/// what's safe to extract, so this deliberately skips
/// [`extract_zip`]'s traversal/symlink/size checks (P6.12 characterizes
/// import as accepting the zip verbatim; those checks apply once, at
/// activation time, per `fixtures/world-archive-safety`).
pub fn list_entry_names(zip_path: &Path) -> Result<Vec<String>, ArchiveError> {
    let file = fs::File::open(zip_path).map_err(ArchiveError::Open)?;
    let archive = ZipArchive::new(file).map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
    Ok(archive.file_names().map(str::to_string).collect())
}

/// One member's decompressed bytes, or `Ok(None)` if `entry_name` isn't
/// present — mirrors `unzip -p <zip> <member>`'s "read one file out of
/// the archive" shape, native rather than shelled out, same rationale as
/// [`list_entry_names`].
pub fn read_entry_bytes(
    zip_path: &Path,
    entry_name: &str,
) -> Result<Option<Vec<u8>>, ArchiveError> {
    let file = fs::File::open(zip_path).map_err(ArchiveError::Open)?;
    let mut archive = ZipArchive::new(file).map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
    match archive.by_name(entry_name) {
        Ok(mut entry) => {
            let mut buf = Vec::new();
            io::copy(&mut entry, &mut buf).map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
            Ok(Some(buf))
        }
        Err(zip::result::ZipError::FileNotFound) => Ok(None),
        Err(e) => Err(ArchiveError::Corrupt(e.to_string())),
    }
}

fn add_directory_entry<W: Write + io::Seek>(
    zip: &mut ZipWriter<W>,
    name: &str,
) -> Result<(), ArchiveError> {
    let opts = SimpleFileOptions::default().unix_permissions(0o755);
    zip.add_directory(format!("{name}/"), opts)
        .map_err(|e| ArchiveError::Corrupt(e.to_string()))
}

fn add_file_entry<W: Write + io::Seek>(
    zip: &mut ZipWriter<W>,
    name: &str,
    bytes: &[u8],
) -> Result<(), ArchiveError> {
    let opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    zip.start_file(name, opts)
        .map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
    zip.write_all(bytes).map_err(ArchiveError::Io)?;
    Ok(())
}

/// Recursively adds `disk_dir`'s contents under `zip_prefix`, including an
/// entry for `disk_dir` itself so an empty subdirectory still round-trips.
/// Entries are written in sorted-by-name order for deterministic output —
/// MSC 1's own `zip -r` has no such guarantee (directory-listing order is
/// unspecified), so this is a Rust-side improvement, not a parity
/// requirement, matching the same precedent already set by
/// `msc-application::transfer`'s own `add_dir_recursive`.
fn add_dir_recursive<W: Write + io::Seek>(
    zip: &mut ZipWriter<W>,
    disk_dir: &Path,
    zip_prefix: &str,
) -> Result<(), ArchiveError> {
    add_directory_entry(zip, zip_prefix)?;
    let mut entries: Vec<_> = fs::read_dir(disk_dir)
        .map_err(ArchiveError::Io)?
        .collect::<Result<_, io::Error>>()
        .map_err(ArchiveError::Io)?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let zip_path = format!("{zip_prefix}/{name}");
        let file_type = entry.file_type().map_err(ArchiveError::Io)?;
        if file_type.is_dir() {
            add_dir_recursive(zip, &path, &zip_path)?;
        } else if file_type.is_file() {
            let bytes = fs::read(&path).map_err(ArchiveError::Io)?;
            add_file_entry(zip, &zip_path, &bytes)?;
        }
        // A symlink inside a source world folder is neither expected nor
        // specially handled — the same "don't invent new source-side
        // policy with no fixture behind it" call `transfer.rs`'s own
        // `add_dir_recursive` already made.
    }
    Ok(())
}

/// Zips each of `folder_names` (relative to `base_dir`, e.g. Java's
/// `[level, level_nether, level_the_end]` or Bedrock's `["worlds"]") into
/// a new archive at `dest_zip_path`, top-level entries named after the
/// folder itself — mirroring `WorldSlotManager.createSlot`'s
/// `zip -r world.zip world world_nether world_the_end` shape. A named
/// folder that doesn't exist on disk is silently skipped (matches source:
/// `worldFolderNames` already filters to folders that exist before this
/// is ever called, so this is defense in depth, not a documented branch).
/// Does not create `dest_zip_path`'s parent directory — same "caller
/// already created it" convention as [`crate::atomic_write::atomic_write`].
pub fn create_zip_from_folders(
    dest_zip_path: &Path,
    base_dir: &Path,
    folder_names: &[String],
) -> Result<(), ArchiveError> {
    let file = fs::File::create(dest_zip_path).map_err(ArchiveError::Io)?;
    let mut zip = ZipWriter::new(file);
    for name in folder_names {
        let disk_dir = base_dir.join(name);
        if disk_dir.is_dir() {
            add_dir_recursive(&mut zip, &disk_dir, name)?;
        }
    }
    zip.finish()
        .map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
    Ok(())
}
