//! P8.20's own tests: `msc_application::curseforge_manual`, the D-027
//! staged-upload completion path for an author-blocked CurseForge file.

use std::fs;
use std::path::{Path, PathBuf};

use msc_application::curseforge_manual::{self, ManualFileError, PendingManualFile};
use msc_infrastructure::fs::StdFileSystem;

struct TempDir(PathBuf);
impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-curseforge-manual-test-{label}-{}",
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

fn real_jar_bytes() -> Vec<u8> {
    // A minimal, genuinely valid (empty) zip archive.
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        zip.start_file("marker.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        use std::io::Write;
        zip.write_all(b"hi").unwrap();
        zip.finish().unwrap();
    }
    buf
}

fn pending(dest: PathBuf, name: &str, size: u64) -> PendingManualFile {
    PendingManualFile {
        project_id: 100,
        file_id: 5000001,
        expected_file_name: name.to_string(),
        expected_byte_size: size,
        dest,
    }
}

// --- filename_matches ---

#[test]
fn curseforge_manual_filename_matches_exact_case_insensitive() {
    assert!(curseforge_manual::filename_matches(
        "SomeMod-1.0.jar",
        "somemod-1.0.JAR",
        false
    ));
}

#[test]
fn curseforge_manual_filename_matches_macos_duplicate_suffix_same_extension() {
    assert!(curseforge_manual::filename_matches(
        "SomeMod-1.0.jar",
        "SomeMod-1.0 (1).jar",
        false
    ));
}

#[test]
fn curseforge_manual_filename_matches_duplicate_suffix_requires_same_extension() {
    assert!(!curseforge_manual::filename_matches(
        "SomeMod-1.0.jar",
        "SomeMod-1.0 (1).zip",
        false
    ));
}

#[test]
fn curseforge_manual_filename_matches_rejects_unrelated_name_without_fallback() {
    assert!(!curseforge_manual::filename_matches(
        "SomeMod-1.0.jar",
        "TotallyDifferent.jar",
        false
    ));
}

#[test]
fn curseforge_manual_filename_matches_single_remaining_fallback_when_flagged() {
    assert!(curseforge_manual::filename_matches(
        "SomeMod-1.0.jar",
        "TotallyDifferent.jar",
        true
    ));
}

// --- complete_pending_file ---

#[test]
fn curseforge_manual_complete_pending_file_accepts_exact_match_and_installs() {
    let tmp = TempDir::new("accept-exact");
    let jar = real_jar_bytes();
    let staged = tmp.path().join("staged").join("SomeMod-1.0.jar");
    fs::create_dir_all(staged.parent().unwrap()).unwrap();
    fs::write(&staged, &jar).unwrap();

    let dest = tmp.path().join("server/mods/SomeMod-1.0.jar");
    let p = pending(dest.clone(), "SomeMod-1.0.jar", jar.len() as u64);

    let result = curseforge_manual::complete_pending_file(
        &StdFileSystem,
        &staged,
        "SomeMod-1.0.jar",
        &p,
        false,
    );
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(fs::read(&dest).unwrap(), jar);
}

#[test]
fn curseforge_manual_complete_pending_file_rejects_wrong_file_does_not_substitute() {
    let tmp = TempDir::new("reject-wrong-file");
    let jar = real_jar_bytes();
    let staged = tmp.path().join("staged").join("Unrelated.jar");
    fs::create_dir_all(staged.parent().unwrap()).unwrap();
    fs::write(&staged, &jar).unwrap();

    let dest = tmp.path().join("server/mods/SomeMod-1.0.jar");
    let p = pending(dest.clone(), "SomeMod-1.0.jar", jar.len() as u64);

    let result = curseforge_manual::complete_pending_file(
        &StdFileSystem,
        &staged,
        "Unrelated.jar",
        &p,
        false,
    );
    assert!(matches!(result, Err(ManualFileError::FilenameMismatch)));
    assert!(!dest.exists());
}

#[test]
fn curseforge_manual_complete_pending_file_rejects_size_mismatch() {
    let tmp = TempDir::new("reject-size");
    let jar = real_jar_bytes();
    let staged = tmp.path().join("staged").join("SomeMod-1.0.jar");
    fs::create_dir_all(staged.parent().unwrap()).unwrap();
    fs::write(&staged, &jar).unwrap();

    let dest = tmp.path().join("server/mods/SomeMod-1.0.jar");
    // Wrong expected size on purpose.
    let p = pending(dest.clone(), "SomeMod-1.0.jar", jar.len() as u64 + 1000);

    let result = curseforge_manual::complete_pending_file(
        &StdFileSystem,
        &staged,
        "SomeMod-1.0.jar",
        &p,
        false,
    );
    assert!(matches!(result, Err(ManualFileError::SizeMismatch { .. })));
    assert!(!dest.exists());
}

#[test]
fn curseforge_manual_complete_pending_file_rejects_non_jar_content() {
    let tmp = TempDir::new("reject-not-a-jar");
    let bytes = b"not a real zip file at all".to_vec();
    let staged = tmp.path().join("staged").join("SomeMod-1.0.jar");
    fs::create_dir_all(staged.parent().unwrap()).unwrap();
    fs::write(&staged, &bytes).unwrap();

    let dest = tmp.path().join("server/mods/SomeMod-1.0.jar");
    let p = pending(dest.clone(), "SomeMod-1.0.jar", bytes.len() as u64);

    let result = curseforge_manual::complete_pending_file(
        &StdFileSystem,
        &staged,
        "SomeMod-1.0.jar",
        &p,
        false,
    );
    assert!(matches!(result, Err(ManualFileError::NotAValidJar)));
    assert!(!dest.exists());
}

#[test]
fn curseforge_manual_complete_pending_file_accepts_one_file_fallback_when_flagged() {
    let tmp = TempDir::new("accept-fallback");
    let jar = real_jar_bytes();
    let staged = tmp.path().join("staged").join("mystery-download.jar");
    fs::create_dir_all(staged.parent().unwrap()).unwrap();
    fs::write(&staged, &jar).unwrap();

    let dest = tmp.path().join("server/mods/SomeMod-1.0.jar");
    let p = pending(dest.clone(), "SomeMod-1.0.jar", jar.len() as u64);

    let result = curseforge_manual::complete_pending_file(
        &StdFileSystem,
        &staged,
        "mystery-download.jar",
        &p,
        true, // this is the only remaining pending file
    );
    assert!(result.is_ok(), "{result:?}");
    assert!(dest.exists());
}
