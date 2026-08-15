//! `world_slots/{id}/{slot.json,world.zip,thumbnail.*}`: the I/O half of
//! `WorldSlotManager`'s directory layout, load/save, and archive-size stat
//! (P6.9's `msc_domain::world` owns the pure metadata/policy this module
//! calls into).
//!
//! Ported from `WorldSlotManager.swift`'s directory-helper functions
//! (`slotsDirectory`/`slotDirectory`/`zipURL`/`metadataURL`/
//! `activeSlotIDURL`, lines 73-103), `loadExplicitActiveSlotID`/
//! `setActiveSlotID` (lines 105-126), and `loadSlots`/`saveMetadata`
//! (lines 274-339). No destructive live-world swap yet — `activateSlot`'s
//! folder-removal/extraction step is P6.12+'s job, once running-server
//! guards exist at the application layer.
//!
//! `saveThumbnail` (source line 343-381) resizes and JPEG-encodes a real
//! image via AppKit — no fixture in `fixtures/world-slots`/
//! `fixtures/world-mutations` pins pixel output, and the field itself is
//! source's own "future use" (line 32's comment). This module ports only
//! the deterministic, testable half — `thumbnail_dest_size`'s aspect-
//! ratio-preserving bounding-box math — and stores whatever encoded bytes
//! the caller already produced verbatim; decoding/resizing a real image
//! is a client/UI-layer concern this crate has no fixture-backed reason
//! to take on yet. Flagged here rather than silently narrowed.

use crate::atomic_write::{AtomicWriteError, atomic_write};
use crate::fs::FileSystem;
use msc_domain::world::{self, WorldSlot};
use std::path::{Path, PathBuf};

pub fn slots_directory(server_dir: &Path) -> PathBuf {
    server_dir.join("world_slots")
}

pub fn slot_directory(server_dir: &Path, slot_id: &str) -> PathBuf {
    slots_directory(server_dir).join(slot_id)
}

pub fn zip_path(server_dir: &Path, slot_id: &str) -> PathBuf {
    slot_directory(server_dir, slot_id).join("world.zip")
}

pub fn metadata_path(server_dir: &Path, slot_id: &str) -> PathBuf {
    slot_directory(server_dir, slot_id).join("slot.json")
}

pub fn thumbnail_path(server_dir: &Path, slot_id: &str, file_name: &str) -> PathBuf {
    slot_directory(server_dir, slot_id).join(file_name)
}

pub fn active_marker_path(server_dir: &Path) -> PathBuf {
    slots_directory(server_dir).join("active_slot_id.txt")
}

/// `loadExplicitActiveSlotID(forServerDir:)` (source line 105-110): a
/// missing marker file, or one that's empty after trimming, is "no
/// explicit marker" — not an error.
pub fn load_explicit_active_slot_id(fs: &dyn FileSystem, server_dir: &Path) -> Option<String> {
    let bytes = fs.read(&active_marker_path(server_dir)).ok()?;
    let raw = String::from_utf8_lossy(&bytes);
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// `setActiveSlotID(_:forServerDir:)` (source line 112-126): `Some(id)`
/// creates `world_slots/` if needed and writes `"{id}\n"`; `None` removes
/// the marker file if present (already-absent is not an error, matching
/// source's own `fm.fileExists` guard before `removeItem`).
pub fn set_active_slot_id(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot_id: Option<&str>,
) -> std::io::Result<()> {
    let path = active_marker_path(server_dir);
    match slot_id {
        None => match fs.remove(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        },
        Some(id) => {
            fs.create_dir_all(&slots_directory(server_dir))?;
            fs.write(&path, format!("{id}\n").as_bytes())
        }
    }
}

/// `loadSlots(forServerDir:)` (source line 274-325): tolerant of a
/// non-directory entry, a missing `slot.json`, or an unparseable
/// `slot.json` in the same pass — each bad entry is skipped, not fatal to
/// the whole directory. Populates `zip_size_bytes` from a cheap stat of
/// `world.zip`, then sorts newest-`created_at`-first via
/// [`msc_domain::world::sort_newest_first`]. A missing `world_slots/`
/// directory returns an empty list, the ordinary state for any server
/// never yet stopped inside MSC 1/MSC 2.
pub fn load_slots(fs: &dyn FileSystem, server_dir: &Path) -> Vec<WorldSlot> {
    let slots_dir = slots_directory(server_dir);
    let Ok(entries) = fs.list(&slots_dir) else {
        return Vec::new();
    };

    let mut slots = Vec::new();
    for entry in entries {
        let Ok(meta) = fs.stat(&entry) else { continue };
        if !meta.is_dir {
            continue;
        }
        let Ok(bytes) = fs.read(&entry.join("slot.json")) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        let Ok(mut slot) = WorldSlot::decode(&value) else {
            continue;
        };
        if let Ok(zip_meta) = fs.stat(&entry.join("world.zip"))
            && zip_meta.is_file
        {
            slot.zip_size_bytes = zip_size_bytes(fs, &entry.join("world.zip"));
        }
        slots.push(slot);
    }

    world::sort_newest_first(&mut slots);
    slots
}

/// `loadSlots`'s zip-size stat (source line 314-318) reads the file
/// through [`FileSystem::read`] rather than a true byte-count-only stat —
/// this trait has no size-only `stat` field (see [`crate::fs::Metadata`]),
/// and every slot archive small enough for [`crate::archive`]'s own
/// [`crate::archive::MAX_TOTAL_UNCOMPRESSED_BYTES`] ceiling to allow is
/// small enough that reading it once to measure its length is not the
/// "extraction" cost this module's docs are careful to avoid — extraction
/// decompresses; this only measures the compressed file already on disk.
fn zip_size_bytes(fs: &dyn FileSystem, path: &Path) -> Option<i64> {
    fs.read(path).ok().map(|bytes| bytes.len() as i64)
}

/// `saveMetadata(_:serverDir:)` (source line 329-339): creates the slot
/// directory if needed, then writes `slot.json` — atomically, via
/// [`atomic_write`], strengthening source's plain `.atomic`-flagged
/// `Data.write` with the same stage-then-rename primitive every other
/// metadata writer in this crate already uses. Key order matches
/// source's `.sortedKeys` encoder option: `serde_json::Value`'s default
/// `Map` (no `preserve_order` feature enabled anywhere in this workspace)
/// is a `BTreeMap`, so `to_vec_pretty` already emits keys alphabetically.
pub fn save_metadata(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot: &WorldSlot,
) -> Result<(), AtomicWriteError> {
    let dir = slot_directory(server_dir, &slot.id);
    fs.create_dir_all(&dir).map_err(AtomicWriteError::Io)?;
    let bytes =
        serde_json::to_vec_pretty(&slot.encode()).expect("serde_json::Value always serializes");
    atomic_write(fs, &metadata_path(server_dir, &slot.id), &bytes)
}

/// `thumbnail dest size`, the deterministic half of `saveThumbnail`
/// (source line 353-356): `min(maxW / srcW, maxH / srcH, 1.0)`, applied to
/// both dimensions and rounded — never upscales (`1.0` caps the scale),
/// preserves aspect ratio (one scale factor for both axes).
pub fn thumbnail_dest_size(src_width: f64, src_height: f64) -> (f64, f64) {
    const MAX_W: f64 = 800.0;
    const MAX_H: f64 = 450.0;
    let scale = (MAX_W / src_width).min(MAX_H / src_height).min(1.0);
    ((src_width * scale).round(), (src_height * scale).round())
}

/// Stores `encoded_bytes` (already resized/JPEG-encoded by the caller —
/// see the module doc) as `thumbnail.jpg` in the slot's directory and
/// updates its metadata, mirroring `saveThumbnail`'s two-step "write the
/// file, then persist the updated slot" shape (source line 372-380).
pub fn save_thumbnail(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot: &WorldSlot,
    encoded_bytes: &[u8],
) -> Result<WorldSlot, AtomicWriteError> {
    let dir = slot_directory(server_dir, &slot.id);
    fs.create_dir_all(&dir).map_err(AtomicWriteError::Io)?;
    let file_name = "thumbnail.jpg";
    atomic_write(fs, &dir.join(file_name), encoded_bytes)?;

    let mut updated = slot.clone();
    updated.thumbnail_file_name = Some(file_name.to_string());
    save_metadata(fs, server_dir, &updated)?;
    Ok(updated)
}
