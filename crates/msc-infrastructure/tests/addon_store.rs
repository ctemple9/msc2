//! P8.14's own tests, direct unit coverage rather than a fixture-mapped
//! set (this step has no `fixtures/` directory of its own — the archive-
//! safety gaps it closes are already characterized in
//! `fixtures/modpack-archive-safety/`, cited from `addon_store.rs`'s own
//! module doc rather than re-asserted fixture-by-fixture here).

use msc_infrastructure::addon_provider::{AddonTransport, RawResponse, TransportError};
use msc_infrastructure::addon_store::{self, AddonStoreError, DisableOutcome};
use msc_infrastructure::download_staging::ExpectedChecksum;
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

struct FakeTransport {
    responses: Mutex<HashMap<String, (u16, Vec<u8>)>>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
        }
    }

    fn with_response(self, url: &str, status: u16, body: &[u8]) -> Self {
        self.responses
            .lock()
            .unwrap()
            .insert(url.to_string(), (status, body.to_vec()));
        self
    }
}

impl AddonTransport for FakeTransport {
    fn get(
        &self,
        url: &str,
        what: &str,
        _headers: &[(&str, &str)],
        max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        let responses = self.responses.lock().unwrap();
        let (status, body) = responses
            .get(url)
            .unwrap_or_else(|| panic!("{what}: no fake response registered for {url}"));
        if body.len() as u64 > max_bytes {
            return Err(TransportError::ResponseTooLarge {
                what: what.to_string(),
                max_bytes,
            });
        }
        Ok(RawResponse {
            status: *status,
            body: body.clone(),
        })
    }

    fn post_json(
        &self,
        url: &str,
        what: &str,
        _body: &serde_json::Value,
        _headers: &[(&str, &str)],
        max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        self.get(url, what, &[], max_bytes)
    }
}

// --- install_verified_file ---

#[test]
fn install_verified_file_writes_when_no_checksum_supplied() {
    let transport =
        FakeTransport::new().with_response("https://cdn.example/mod.jar", 200, b"jar-bytes");
    let fs = FakeFileSystem::new().with_dir("server/mods");
    let dest = PathBuf::from("server/mods/mod.jar");

    let cached = addon_store::install_verified_file(
        &transport,
        &fs,
        "https://cdn.example/mod.jar",
        "1.0.0",
        None,
        &dest,
    )
    .expect("install should succeed");

    assert_eq!(cached.path, dest);
    assert_eq!(cached.version, "1.0.0");
    assert_eq!(fs.read(&dest).unwrap(), b"jar-bytes");
}

#[test]
fn install_verified_file_matching_sha1_writes_the_file() {
    // sha1("jar-bytes")
    let expected_sha1 = msc_infrastructure::download_staging::sha1_hex(b"jar-bytes");
    let transport =
        FakeTransport::new().with_response("https://cdn.example/mod.jar", 200, b"jar-bytes");
    let fs = FakeFileSystem::new().with_dir("server/mods");
    let dest = PathBuf::from("server/mods/mod.jar");
    let checksum = ExpectedChecksum::sha1(expected_sha1);

    addon_store::install_verified_file(
        &transport,
        &fs,
        "https://cdn.example/mod.jar",
        "1.0.0",
        Some(&checksum),
        &dest,
    )
    .expect("matching checksum should install");

    assert_eq!(fs.read(&dest).unwrap(), b"jar-bytes");
}

#[test]
fn install_verified_file_mismatched_checksum_writes_nothing() {
    let transport =
        FakeTransport::new().with_response("https://cdn.example/mod.jar", 200, b"jar-bytes");
    let fs = FakeFileSystem::new().with_dir("server/mods");
    let dest = PathBuf::from("server/mods/mod.jar");
    let checksum = ExpectedChecksum::sha1("0000000000000000000000000000000000000000".to_string());

    let result = addon_store::install_verified_file(
        &transport,
        &fs,
        "https://cdn.example/mod.jar",
        "1.0.0",
        Some(&checksum),
        &dest,
    );

    assert!(matches!(result, Err(AddonStoreError::Staging(_))));
    assert!(
        fs.read(&dest).is_err(),
        "mismatched download must not be written"
    );
}

#[test]
fn install_verified_file_non_2xx_status_fails_without_writing() {
    let transport =
        FakeTransport::new().with_response("https://cdn.example/mod.jar", 404, b"not found");
    let fs = FakeFileSystem::new().with_dir("server/mods");
    let dest = PathBuf::from("server/mods/mod.jar");

    let result = addon_store::install_verified_file(
        &transport,
        &fs,
        "https://cdn.example/mod.jar",
        "1.0.0",
        None,
        &dest,
    );

    assert!(matches!(result, Err(AddonStoreError::DownloadFailed(404))));
    assert!(fs.read(&dest).is_err());
}

// --- toggle_addon_jar ---

#[test]
fn toggle_addon_jar_disables_an_active_jar() {
    let fs = FakeFileSystem::new().with_file("server/mods/foo.jar", b"bytes".to_vec(), false);
    let new_path = addon_store::toggle_addon_jar(&fs, Path::new("server/mods/foo.jar")).unwrap();
    assert_eq!(new_path, PathBuf::from("server/mods/foo.jar.disabled"));
    assert!(fs.read(&new_path).is_ok());
    assert!(fs.read(Path::new("server/mods/foo.jar")).is_err());
}

#[test]
fn toggle_addon_jar_enables_a_disabled_jar() {
    let fs =
        FakeFileSystem::new().with_file("server/mods/foo.jar.disabled", b"bytes".to_vec(), false);
    let new_path =
        addon_store::toggle_addon_jar(&fs, Path::new("server/mods/foo.jar.disabled")).unwrap();
    assert_eq!(new_path, PathBuf::from("server/mods/foo.jar"));
    assert!(fs.read(&new_path).is_ok());
}

#[test]
fn toggle_addon_jar_refuses_to_clobber_an_existing_target() {
    let fs = FakeFileSystem::new()
        .with_file("server/mods/foo.jar", b"active".to_vec(), false)
        .with_file("server/mods/foo.jar.disabled", b"disabled".to_vec(), false);

    let result = addon_store::toggle_addon_jar(&fs, Path::new("server/mods/foo.jar"));

    assert!(matches!(result, Err(AddonStoreError::AlreadyExists(_))));
    // Neither file moved.
    assert_eq!(
        fs.read(Path::new("server/mods/foo.jar")).unwrap(),
        b"active"
    );
    assert_eq!(
        fs.read(Path::new("server/mods/foo.jar.disabled")).unwrap(),
        b"disabled"
    );
}

// --- disable_for_classification ---

#[test]
fn disable_for_classification_no_op_when_no_active_jar() {
    let fs = FakeFileSystem::new();
    let outcome =
        addon_store::disable_for_classification(&fs, Path::new("server/mods/foo.jar")).unwrap();
    assert_eq!(outcome, DisableOutcome::NoOp);
}

#[test]
fn disable_for_classification_renames_when_no_disabled_sibling_exists() {
    let fs = FakeFileSystem::new().with_file("server/mods/foo.jar", b"active".to_vec(), false);
    let outcome =
        addon_store::disable_for_classification(&fs, Path::new("server/mods/foo.jar")).unwrap();
    assert_eq!(
        outcome,
        DisableOutcome::Disabled(PathBuf::from("server/mods/foo.jar.disabled"))
    );
    assert!(fs.read(Path::new("server/mods/foo.jar.disabled")).is_ok());
    assert!(fs.read(Path::new("server/mods/foo.jar")).is_err());
}

#[test]
fn disable_for_classification_drops_active_and_keeps_existing_disabled() {
    let fs = FakeFileSystem::new()
        .with_file("server/mods/foo.jar", b"fresh".to_vec(), false)
        .with_file("server/mods/foo.jar.disabled", b"original".to_vec(), false);

    let outcome =
        addon_store::disable_for_classification(&fs, Path::new("server/mods/foo.jar")).unwrap();

    assert_eq!(outcome, DisableOutcome::DroppedActiveKeptExistingDisabled);
    assert!(fs.read(Path::new("server/mods/foo.jar")).is_err());
    assert_eq!(
        fs.read(Path::new("server/mods/foo.jar.disabled")).unwrap(),
        b"original"
    );
}

// --- remove_addon_jar ---

#[test]
fn remove_addon_jar_removes_an_existing_file() {
    let fs = FakeFileSystem::new().with_file("server/mods/foo.jar", b"bytes".to_vec(), false);
    addon_store::remove_addon_jar(&fs, Path::new("server/mods/foo.jar")).unwrap();
    assert!(fs.read(Path::new("server/mods/foo.jar")).is_err());
}

#[test]
fn remove_addon_jar_missing_file_errors() {
    let fs = FakeFileSystem::new();
    let result = addon_store::remove_addon_jar(&fs, Path::new("server/mods/missing.jar"));
    assert!(matches!(result, Err(AddonStoreError::Io(_))));
}

// --- resolve_pack_file_dest ---

#[test]
fn resolve_pack_file_dest_accepts_a_legitimate_relative_path() {
    let fs = FakeFileSystem::new().with_dir("server");
    let dest = addon_store::resolve_pack_file_dest(
        &fs,
        Path::new("server"),
        "mods/example.jar",
        Path::new("/home/nobody"),
    )
    .unwrap();
    assert_eq!(dest, PathBuf::from("server/mods/example.jar"));
}

#[test]
fn resolve_pack_file_dest_rejects_traversal_outside_server_dir() {
    let fs = FakeFileSystem::new().with_dir("server");
    let result = addon_store::resolve_pack_file_dest(
        &fs,
        Path::new("server"),
        "../../../../etc/cron.d/evil",
        Path::new("/home/nobody"),
    );
    assert!(matches!(result, Err(AddonStoreError::PathSafety(_))));
}

#[test]
fn resolve_pack_file_dest_rejects_a_symlink_mediated_escape() {
    let fs = FakeFileSystem::new()
        .with_dir("server")
        .with_symlink("server/escape", "/etc");
    let result = addon_store::resolve_pack_file_dest(
        &fs,
        Path::new("server"),
        "escape/passwd",
        Path::new("/home/nobody"),
    );
    assert!(matches!(result, Err(AddonStoreError::PathSafety(_))));
}

// --- archive.rs's own P8.14 addition: executable-bit preservation ---
//
// A modpack override can legitimately bundle a script (e.g. `start.sh`)
// that needs to stay executable after extraction -- `fixtures/
// modpack-archive-safety/` never characterized this (MSC 1's `ditto`
// extraction preserves permissions as an OS-level side effect, not
// something any fixture documents as a Swift-level decision), so this is
// exercised directly against `archive::extract_zip` rather than mapped
// from a fixture, the same "no fixture, direct unit test" precedent this
// whole file follows.

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-addon-store-test-{label}-{}",
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

#[cfg(unix)]
#[test]
fn extract_zip_preserves_an_executable_entrys_permission_bit() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    let tmp = TempDir::new("executable-bit");
    let zip_path = tmp.path().join("override.zip");
    let dest = tmp.path().join("out");

    let file = fs::File::create(&zip_path).unwrap();
    let mut zip = ZipWriter::new(file);
    let exec_opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o755);
    zip.start_file("start.sh", exec_opts).unwrap();
    zip.write_all(b"#!/bin/sh\necho hi\n").unwrap();
    let plain_opts = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    zip.start_file("config.yml", plain_opts).unwrap();
    zip.write_all(b"key: value\n").unwrap();
    zip.finish().unwrap();

    msc_infrastructure::archive::extract_zip(&zip_path, &dest).unwrap();

    let script_mode = fs::metadata(dest.join("start.sh"))
        .unwrap()
        .permissions()
        .mode();
    assert_ne!(script_mode & 0o111, 0, "start.sh should stay executable");
    let config_mode = fs::metadata(dest.join("config.yml"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(
        config_mode & 0o111,
        0,
        "config.yml has no exec bit in the archive and should not gain one"
    );
}
