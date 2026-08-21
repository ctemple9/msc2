//! P8.19's own tests: `msc_application::modpacks::import_mrpack`. Real
//! on-disk directories via `StdFileSystem` (the same necessity
//! `modpack_inspection.rs`/`backup_restore.rs` already established —
//! `addon_store`/`archive` primitives this function composes work against
//! real paths).

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use msc_application::modpacks::{self, MrpackImportError};
use msc_domain::identity::JavaServerFlavor;
use msc_domain::modpack_manifest::{MrpackFileEntry, MrpackFileHashes, MrpackManifest};
use msc_infrastructure::addon_provider::{AddonTransport, RawResponse, TransportError};
use msc_infrastructure::download_staging::sha512_hex;
use msc_infrastructure::fs::StdFileSystem;

struct TempDir(PathBuf);
impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-mrpack-import-test-{label}-{}",
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

struct FakeTransport {
    downloads: HashMap<String, (u16, Vec<u8>)>,
    identify: Option<serde_json::Value>,
    projects: Option<serde_json::Value>,
}
impl FakeTransport {
    fn new() -> Self {
        Self {
            downloads: HashMap::new(),
            identify: None,
            projects: None,
        }
    }
    fn with_download(mut self, url: &str, status: u16, body: &[u8]) -> Self {
        self.downloads
            .insert(url.to_string(), (status, body.to_vec()));
        self
    }
    fn with_identify(mut self, v: serde_json::Value) -> Self {
        self.identify = Some(v);
        self
    }
    fn with_projects(mut self, v: serde_json::Value) -> Self {
        self.projects = Some(v);
        self
    }
}
impl AddonTransport for FakeTransport {
    fn get(
        &self,
        url: &str,
        what: &str,
        _: &[(&str, &str)],
        _: u64,
    ) -> Result<RawResponse, TransportError> {
        let (status, body) = self
            .downloads
            .get(url)
            .unwrap_or_else(|| panic!("{what}: no fake response registered for GET {url}"));
        Ok(RawResponse {
            status: *status,
            body: body.clone(),
        })
    }
    fn post_json(
        &self,
        url: &str,
        what: &str,
        _: &serde_json::Value,
        _: &[(&str, &str)],
        _: u64,
    ) -> Result<RawResponse, TransportError> {
        let body = if url.ends_with("/v2/version_files") {
            self.identify
                .clone()
                .unwrap_or_else(|| serde_json::json!({}))
        } else {
            panic!("{what}: unexpected POST {url}");
        };
        Ok(RawResponse {
            status: 200,
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

// modrinth_projects uses GET, not POST -- override get() to route it too.
struct FakeTransportWithProjects(FakeTransport);
impl AddonTransport for FakeTransportWithProjects {
    fn get(
        &self,
        url: &str,
        what: &str,
        h: &[(&str, &str)],
        m: u64,
    ) -> Result<RawResponse, TransportError> {
        if url.contains("/v2/projects") {
            let body = self
                .0
                .projects
                .clone()
                .unwrap_or_else(|| serde_json::json!([]));
            return Ok(RawResponse {
                status: 200,
                body: serde_json::to_vec(&body).unwrap(),
            });
        }
        self.0.get(url, what, h, m)
    }
    fn post_json(
        &self,
        url: &str,
        what: &str,
        b: &serde_json::Value,
        h: &[(&str, &str)],
        m: u64,
    ) -> Result<RawResponse, TransportError> {
        self.0.post_json(url, what, b, h, m)
    }
}

fn manifest(files: Vec<MrpackFileEntry>) -> MrpackManifest {
    MrpackManifest {
        name: "Test Pack".to_string(),
        version_id: "1.0".to_string(),
        game: "minecraft".to_string(),
        dependencies: HashMap::new(),
        files,
    }
}

fn file_entry(path: &str, sha512: &str, downloads: Vec<&str>) -> MrpackFileEntry {
    MrpackFileEntry {
        path: path.to_string(),
        hashes: MrpackFileHashes {
            sha1: None,
            sha512: Some(sha512.to_string()),
        },
        env: None,
        downloads: downloads.into_iter().map(str::to_string).collect(),
        file_size: 0,
    }
}

fn client_only_file_entry(path: &str) -> MrpackFileEntry {
    MrpackFileEntry {
        path: path.to_string(),
        hashes: MrpackFileHashes {
            sha1: None,
            sha512: None,
        },
        env: Some(msc_domain::modpack::MrpackEnv {
            client: Some("required".to_string()),
            server: Some("unsupported".to_string()),
        }),
        downloads: vec!["https://cdn.example.invalid/should-never-be-fetched.jar".to_string()],
        file_size: 0,
    }
}

fn write_file(dir: &Path, rel: &str, content: &[u8]) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    let mut f = fs::File::create(&p).unwrap();
    f.write_all(content).unwrap();
}

#[allow(clippy::too_many_arguments)]
fn run_import(
    transport: &dyn AddonTransport,
    server_dir: &Path,
    manifest: &MrpackManifest,
    staged_dir: &Path,
    home_dir: &Path,
    pack_managed: bool,
    explicit_replace: bool,
    should_cancel: &dyn Fn() -> bool,
) -> Result<modpacks::MrpackImportReport, MrpackImportError> {
    modpacks::import_mrpack(
        transport,
        &StdFileSystem,
        server_dir,
        JavaServerFlavor::Fabric,
        manifest,
        staged_dir,
        home_dir,
        pack_managed,
        explicit_replace,
        should_cancel,
    )
}

#[test]
fn import_downloads_and_verifies_manifest_files() {
    let tmp = TempDir::new("basic");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let bytes = b"real jar bytes";
    let hash = sha512_hex(bytes);
    let m = manifest(vec![file_entry(
        "mods/sodium.jar",
        &hash,
        vec!["https://cdn.example.invalid/sodium.jar"],
    )]);
    let transport =
        FakeTransport::new().with_download("https://cdn.example.invalid/sodium.jar", 200, bytes);

    let report = run_import(
        &transport,
        &server_dir,
        &m,
        &staged_dir,
        tmp.path(),
        false,
        false,
        &|| false,
    )
    .unwrap();

    assert_eq!(report.installed_files.len(), 1);
    assert!(report.failed_files.is_empty());
    assert_eq!(report.pack_name, "Test Pack");
    assert_eq!(report.pack_version, "1.0");
    assert_eq!(fs::read(server_dir.join("mods/sodium.jar")).unwrap(), bytes);
}

#[test]
fn import_skips_client_only_manifest_files_without_ever_fetching_them() {
    let tmp = TempDir::new("tier1-skip");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let m = manifest(vec![client_only_file_entry("resourcepacks/pack.zip")]);
    // No fake response registered -- a fetch attempt would panic.
    let transport = FakeTransport::new();

    let report = run_import(
        &transport,
        &server_dir,
        &m,
        &staged_dir,
        tmp.path(),
        false,
        false,
        &|| false,
    )
    .unwrap();

    assert!(report.installed_files.is_empty());
    assert!(report.failed_files.is_empty());
    assert!(!server_dir.join("resourcepacks/pack.zip").exists());
}

#[test]
fn import_hash_mismatch_is_recorded_as_failed_not_installed() {
    let tmp = TempDir::new("hash-mismatch");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let m = manifest(vec![file_entry(
        "mods/sodium.jar",
        "0".repeat(128).as_str(),
        vec!["https://cdn.example.invalid/sodium.jar"],
    )]);
    let transport = FakeTransport::new().with_download(
        "https://cdn.example.invalid/sodium.jar",
        200,
        b"real jar bytes",
    );

    let report = run_import(
        &transport,
        &server_dir,
        &m,
        &staged_dir,
        tmp.path(),
        false,
        false,
        &|| false,
    )
    .unwrap();

    assert!(report.installed_files.is_empty());
    assert_eq!(report.failed_files.len(), 1);
    assert!(!server_dir.join("mods/sodium.jar").exists());
}

#[test]
fn import_all_mirrors_fail_recorded_loop_continues_to_next_file() {
    let tmp = TempDir::new("all-mirrors-fail");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let good_bytes = b"good file bytes";
    let good_hash = sha512_hex(good_bytes);
    let m = manifest(vec![
        file_entry(
            "mods/bad.jar",
            &sha512_hex(b"irrelevant"),
            vec!["https://cdn.example.invalid/bad.jar"],
        ),
        file_entry(
            "mods/good.jar",
            &good_hash,
            vec!["https://cdn.example.invalid/good.jar"],
        ),
    ]);
    let transport = FakeTransport::new()
        .with_download("https://cdn.example.invalid/bad.jar", 500, b"")
        .with_download("https://cdn.example.invalid/good.jar", 200, good_bytes);

    let report = run_import(
        &transport,
        &server_dir,
        &m,
        &staged_dir,
        tmp.path(),
        false,
        false,
        &|| false,
    )
    .unwrap();

    assert_eq!(report.failed_files.len(), 1);
    assert_eq!(report.installed_files.len(), 1);
    assert!(server_dir.join("mods/good.jar").exists());
}

#[test]
fn import_merges_overrides_then_server_overrides_wins_conflict() {
    let tmp = TempDir::new("overrides-merge");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    write_file(
        &staged_dir,
        "overrides/config/shared.txt",
        b"from overrides",
    );
    write_file(
        &staged_dir,
        "overrides/config/only-overrides.txt",
        b"only overrides",
    );
    write_file(
        &staged_dir,
        "server-overrides/config/shared.txt",
        b"from server-overrides",
    );
    let m = manifest(vec![]);
    let transport = FakeTransport::new();

    let report = run_import(
        &transport,
        &server_dir,
        &m,
        &staged_dir,
        tmp.path(),
        false,
        false,
        &|| false,
    )
    .unwrap();
    let _ = report;

    assert_eq!(
        fs::read(server_dir.join("config/shared.txt")).unwrap(),
        b"from server-overrides"
    );
    assert_eq!(
        fs::read(server_dir.join("config/only-overrides.txt")).unwrap(),
        b"only overrides"
    );
}

#[test]
fn import_missing_overrides_folder_is_silently_skipped() {
    let tmp = TempDir::new("no-overrides");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged"); // exists, but has no overrides/ subfolder
    fs::create_dir_all(&staged_dir).unwrap();
    let m = manifest(vec![]);
    let transport = FakeTransport::new();

    let result = run_import(
        &transport,
        &server_dir,
        &m,
        &staged_dir,
        tmp.path(),
        false,
        false,
        &|| false,
    );
    assert!(result.is_ok());
}

#[test]
fn import_cancellation_rolls_back_files_already_written() {
    let tmp = TempDir::new("cancellation");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let first_bytes = b"first file bytes";
    let first_hash = sha512_hex(first_bytes);
    let m = manifest(vec![
        file_entry(
            "mods/first.jar",
            &first_hash,
            vec!["https://cdn.example.invalid/first.jar"],
        ),
        file_entry(
            "mods/second.jar",
            &sha512_hex(b"second"),
            vec!["https://cdn.example.invalid/second.jar"],
        ),
    ]);
    let transport = FakeTransport::new().with_download(
        "https://cdn.example.invalid/first.jar",
        200,
        first_bytes,
    );

    let cancel_after_first = AtomicBool::new(false);
    let should_cancel = || {
        if cancel_after_first.load(Ordering::SeqCst) {
            return true;
        }
        cancel_after_first.store(true, Ordering::SeqCst);
        false
    };

    let report = run_import(
        &transport,
        &server_dir,
        &m,
        &staged_dir,
        tmp.path(),
        false,
        false,
        &should_cancel,
    )
    .unwrap();

    assert!(report.cancelled);
    // The first file was written, then rolled back on cancellation before
    // the second file was ever attempted.
    assert!(!server_dir.join("mods/first.jar").exists());
}

#[test]
fn import_refused_on_pack_managed_server_without_explicit_replace_intent() {
    let tmp = TempDir::new("pack-managed-refused");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let m = manifest(vec![file_entry(
        "mods/x.jar",
        &sha512_hex(b"x"),
        vec!["https://cdn.example.invalid/x.jar"],
    )]);
    // No fake response registered -- refusal must happen before any fetch.
    let transport = FakeTransport::new();

    let result = run_import(
        &transport,
        &server_dir,
        &m,
        &staged_dir,
        tmp.path(),
        true,
        false,
        &|| false,
    );
    assert!(matches!(result, Err(MrpackImportError::PackManaged)));
}

#[test]
fn import_allowed_on_pack_managed_server_with_explicit_replace_intent() {
    let tmp = TempDir::new("pack-managed-explicit-replace");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let m = manifest(vec![]);
    let transport = FakeTransport::new();

    let result = run_import(
        &transport,
        &server_dir,
        &m,
        &staged_dir,
        tmp.path(),
        true,
        true,
        &|| false,
    );
    assert!(result.is_ok());
}

#[test]
fn import_tier0_disables_known_client_only_override_jar() {
    let tmp = TempDir::new("tier0-classify");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    write_file(
        &staged_dir,
        "overrides/mods/iris-1.7.jar",
        b"iris shader mod",
    );
    let m = manifest(vec![]);
    let transport = FakeTransport::new();

    let report = run_import(
        &transport,
        &server_dir,
        &m,
        &staged_dir,
        tmp.path(),
        false,
        false,
        &|| false,
    )
    .unwrap();

    assert_eq!(report.disabled_client_only_overrides.len(), 1);
    assert!(server_dir.join("mods/iris-1.7.jar.disabled").exists());
    assert!(!server_dir.join("mods/iris-1.7.jar").exists());
}

#[test]
fn import_tier2_disables_modrinth_server_unsupported_override_jar() {
    let tmp = TempDir::new("tier2-classify");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    let jar_bytes = b"client-only-decor-mod bytes";
    write_file(&staged_dir, "overrides/mods/decor.jar", jar_bytes);
    let hash = sha512_hex(jar_bytes);
    let m = manifest(vec![]);

    let identify = serde_json::json!({
        hash.clone(): {
            "id": "v1",
            "project_id": "decor-proj",
            "version_number": "1.0",
            "files": [],
            "dependencies": [],
        }
    });
    let projects = serde_json::json!([
        {"id": "decor-proj", "slug": "decor", "title": "Decor Mod", "server_side": "unsupported"}
    ]);
    let transport = FakeTransportWithProjects(
        FakeTransport::new()
            .with_identify(identify)
            .with_projects(projects),
    );

    let report = run_import(
        &transport,
        &server_dir,
        &m,
        &staged_dir,
        tmp.path(),
        false,
        false,
        &|| false,
    )
    .unwrap();

    assert_eq!(report.disabled_client_only_overrides.len(), 1);
    assert!(server_dir.join("mods/decor.jar.disabled").exists());
}
