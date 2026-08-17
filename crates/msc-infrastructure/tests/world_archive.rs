//! Port of `fixtures/world-archive-safety/`'s 10 fixtures (P6.5).
//!
//! Each case builds a real zip file (via the `zip` crate directly — the
//! same library `extract_zip` itself uses, so a name/mode this test
//! writes is exactly what `extract_zip` reads back) matching the
//! fixture's described shape, then asserts on
//! [`msc_infrastructure::archive::extract_zip_with_limits`]'s outcome and
//! that nothing was written outside (or, for the refusal cases, at all
//! into) the destination directory.

use msc_infrastructure::archive::{
    ArchiveError, ArchiveLimits, create_zip_from_folders_cancellable, extract_zip,
    extract_zip_with_limits,
};
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

fn load(case: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/world-archive-safety")
        .join(format!("{case}.json"));
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()))
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-world-archive-test-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Writes a zip at `zip_path` with one entry per `(name, is_symlink)`
/// pair — a regular entry's content is a harmless string; a symlink
/// entry's content is a target path string, with Unix mode bits marking
/// it a symlink (`S_IFLNK`), the same shape a real symlink zip entry has.
fn write_zip(zip_path: &Path, entries: &[(&str, bool)]) {
    let file = fs::File::create(zip_path).unwrap();
    let mut zip = ZipWriter::new(file);
    for (name, is_symlink) in entries {
        if *is_symlink {
            let opts = SimpleFileOptions::default().unix_permissions(0o777);
            zip.add_symlink(*name, "../../../../etc/passwd", opts)
                .unwrap();
        } else {
            let opts = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .unix_permissions(0o644);
            zip.start_file(*name, opts).unwrap();
            zip.write_all(b"hello world").unwrap();
        }
    }
    zip.finish().unwrap();
}

fn assert_refused_entirely(result: Result<(), ArchiveError>, dest: &Path) {
    assert!(result.is_err(), "expected extraction to be refused");
    let written = fs::read_dir(dest).map(|d| d.count()).unwrap_or(0);
    assert_eq!(written, 0, "expected zero bytes written to the destination");
}

#[test]
fn world_archive_zip_slip_relative_traversal_rejected() {
    let fixture = load("zip-slip-relative-traversal-rejected");
    let names: Vec<&str> = fixture
        .pointer("/input/zip_entry_names")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let tmp = TempDir::new("zip-slip");
    let zip_path = tmp.path().join("archive.zip");
    let entries: Vec<(&str, bool)> = names.iter().map(|n| (*n, false)).collect();
    write_zip(&zip_path, &entries);
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    assert_refused_entirely(extract_zip(&zip_path, &dest), &dest);
}

#[test]
fn world_archive_absolute_path_entry_rejected() {
    let fixture = load("absolute-path-entry-rejected");
    let names: Vec<&str> = fixture
        .pointer("/input/zip_entry_names")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let tmp = TempDir::new("absolute-path");
    let zip_path = tmp.path().join("archive.zip");
    let entries: Vec<(&str, bool)> = names.iter().map(|n| (*n, false)).collect();
    write_zip(&zip_path, &entries);
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    assert_refused_entirely(extract_zip(&zip_path, &dest), &dest);
}

#[test]
fn world_archive_windows_backslash_traversal_rejected() {
    let fixture = load("windows-backslash-traversal-rejected");
    let names: Vec<&str> = fixture
        .pointer("/input/zip_entry_names")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let tmp = TempDir::new("backslash-traversal");
    let zip_path = tmp.path().join("archive.zip");
    let entries: Vec<(&str, bool)> = names.iter().map(|n| (*n, false)).collect();
    write_zip(&zip_path, &entries);
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    // This is the case a host-separator-only check would miss on Unix —
    // run on whatever platform CI happens to be, not conditionally.
    assert_refused_entirely(extract_zip(&zip_path, &dest), &dest);
}

#[test]
fn world_archive_windows_drive_absolute_path_entry_rejected() {
    let fixture = load("windows-drive-absolute-path-entry-rejected");
    let names: Vec<&str> = fixture
        .pointer("/input/zip_entry_names")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let tmp = TempDir::new("drive-absolute");
    let zip_path = tmp.path().join("archive.zip");
    let entries: Vec<(&str, bool)> = names.iter().map(|n| (*n, false)).collect();
    write_zip(&zip_path, &entries);
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    assert_refused_entirely(extract_zip(&zip_path, &dest), &dest);
}

#[test]
fn world_archive_symlink_entry_pointing_outside_root_rejected() {
    let tmp = TempDir::new("symlink-outside");
    let zip_path = tmp.path().join("archive.zip");
    write_zip(
        &zip_path,
        &[("world/level.dat", false), ("world/link", true)],
    );
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    let result = extract_zip(&zip_path, &dest);
    assert_refused_entirely(result, &dest);
    assert!(!dest.join("world/link").exists());
}

#[test]
fn world_archive_symlink_entry_any_target_rejected_outright() {
    let tmp = TempDir::new("symlink-any-target");
    let zip_path = tmp.path().join("archive.zip");
    write_zip(
        &zip_path,
        &[("world/level.dat", false), ("world/region/r.0.0.mca", true)],
    );
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    assert_refused_entirely(extract_zip(&zip_path, &dest), &dest);
}

#[test]
fn world_archive_extraction_entry_count_limit_exceeded_rejected() {
    let tmp = TempDir::new("entry-count-limit");
    let zip_path = tmp.path().join("archive.zip");
    // A small real archive proves the *enforcement*; a small local limit
    // (rather than constructing millions of real entries) proves it
    // against the same declared-entry-count check `extract_zip` uses in
    // production against `MAX_ARCHIVE_ENTRIES`.
    write_zip(
        &zip_path,
        &[
            ("world/level.dat", false),
            ("world/region/r.0.0.mca", false),
        ],
    );
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    let limits = ArchiveLimits {
        max_entries: 1,
        max_total_uncompressed_bytes: ArchiveLimits::default().max_total_uncompressed_bytes,
    };
    let result = extract_zip_with_limits(&zip_path, &dest, limits);
    assert!(matches!(
        result,
        Err(ArchiveError::EntryCountExceeded { .. })
    ));
    assert_refused_entirely(result, &dest);
}

#[test]
fn world_archive_extraction_size_limit_exceeded_rejected() {
    let tmp = TempDir::new("size-limit");
    let zip_path = tmp.path().join("archive.zip");
    write_zip(&zip_path, &[("world/level.dat", false)]);
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    let limits = ArchiveLimits {
        max_entries: ArchiveLimits::default().max_entries,
        max_total_uncompressed_bytes: 1,
    };
    let result = extract_zip_with_limits(&zip_path, &dest, limits);
    assert!(matches!(
        result,
        Err(ArchiveError::TotalSizeExceeded { .. })
    ));
    assert_refused_entirely(result, &dest);
}

#[test]
fn world_archive_corrupt_zip_central_directory_mismatch_rejected() {
    let tmp = TempDir::new("corrupt-cd");
    let zip_path = tmp.path().join("archive.zip");
    {
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = ZipWriter::new(file);
        // STORED so the "compressed" bytes are the raw payload, making a
        // predictable byte-level corruption straightforward.
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        zip.start_file("world/level.dat", opts).unwrap();
        zip.write_all(b"hello world, this is level.dat content")
            .unwrap();
        zip.finish().unwrap();
    }

    // Flip one byte inside the stored payload (well after the local file
    // header, well before the central directory) so the entry's actual
    // bytes no longer match the CRC32 the central directory/local header
    // both recorded — a structurally valid zip whose local file data
    // disagrees with its own declared checksum.
    let mut bytes = fs::read(&zip_path).unwrap();
    let marker = b"level.dat content";
    let pos = bytes
        .windows(marker.len())
        .position(|w| w == marker)
        .expect("marker bytes present in the stored entry");
    bytes[pos] ^= 0xFF;
    fs::write(&zip_path, &bytes).unwrap();

    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    let result = extract_zip(&zip_path, &dest);
    assert!(matches!(result, Err(ArchiveError::Corrupt(_))));
    assert_refused_entirely(result, &dest);
}

#[test]
fn world_archive_legitimate_nested_world_archive_extracts_normally() {
    let tmp = TempDir::new("positive-control");
    let zip_path = tmp.path().join("archive.zip");
    write_zip(
        &zip_path,
        &[
            ("world/level.dat", false),
            ("world/region/r.0.0.mca", false),
            ("world_nether/level.dat", false),
            ("world_nether/region/r.0.0.mca", false),
            ("world_the_end/level.dat", false),
        ],
    );
    let dest = tmp.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    extract_zip(&zip_path, &dest).expect("a legitimately-shaped archive extracts");
    assert!(dest.join("world/level.dat").is_file());
    assert!(dest.join("world/region/r.0.0.mca").is_file());
    assert!(dest.join("world_nether/level.dat").is_file());
    assert!(dest.join("world_nether/region/r.0.0.mca").is_file());
    assert!(dest.join("world_the_end/level.dat").is_file());
    assert_eq!(
        fs::read(dest.join("world/level.dat")).unwrap(),
        b"hello world"
    );
}

#[test]
fn world_archive_creation_cancellation_removes_partial_zip() {
    let tmp = TempDir::new("cancel-create");
    let world = tmp.path().join("world");
    fs::create_dir_all(&world).unwrap();
    fs::write(world.join("region.mca"), vec![0x5a; 256 * 1024]).unwrap();
    let zip_path = tmp.path().join("cancelled.zip");
    let polls = AtomicUsize::new(0);

    let result =
        create_zip_from_folders_cancellable(&zip_path, tmp.path(), &["world".to_string()], || {
            polls.fetch_add(1, Ordering::SeqCst) >= 4
        });

    assert!(matches!(result, Err(ArchiveError::Cancelled)));
    assert!(polls.load(Ordering::SeqCst) >= 5);
    assert!(
        !zip_path.exists(),
        "a cancelled archive must not leave a partial ZIP"
    );
}
