//! P8.20's own tests: `msc_application::modpacks::import_curseforge`.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use msc_application::modpacks::{self, CurseForgeImportError};
use msc_domain::identity::JavaServerFlavor;
use msc_domain::modpack_manifest::{CurseForgeManifestFile, CurseForgeManifestMetadata};
use msc_infrastructure::addon_provider::{AddonTransport, RawResponse, TransportError};
use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};

struct TempDir(PathBuf);
impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-curseforge-import-test-{label}-{}",
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

fn write_file(dir: &Path, rel: &str, content: &[u8]) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    let mut f = fs::File::create(&p).unwrap();
    f.write_all(content).unwrap();
}

fn metadata(
    overrides_folder: &str,
    files: Vec<CurseForgeManifestFile>,
) -> CurseForgeManifestMetadata {
    CurseForgeManifestMetadata {
        name: "Test CF Pack".to_string(),
        version_id: "1".to_string(),
        minecraft_version: "1.20.1".to_string(),
        loader_flavor: None,
        loader_version: None,
        overrides_folder: overrides_folder.to_string(),
        files,
    }
}

fn cf_file(project_id: i64, file_id: i64) -> CurseForgeManifestFile {
    CurseForgeManifestFile {
        project_id,
        file_id,
        required: true,
    }
}

struct FakeTransport {
    files_response: serde_json::Value,
}
impl AddonTransport for FakeTransport {
    fn get(
        &self,
        url: &str,
        what: &str,
        _: &[(&str, &str)],
        _: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!("{what}: unexpected GET {url}");
    }
    fn post_json(
        &self,
        url: &str,
        what: &str,
        _: &serde_json::Value,
        _: &[(&str, &str)],
        _: u64,
    ) -> Result<RawResponse, TransportError> {
        if url.ends_with("/v1/mods/files") {
            return Ok(RawResponse {
                status: 200,
                body: serde_json::to_vec(&self.files_response).unwrap(),
            });
        }
        panic!("{what}: unexpected POST {url}");
    }
}

/// Combined GET-download + POST-files transport for the "resolvable file
/// actually downloads" tests.
struct DownloadTransport {
    files_response: serde_json::Value,
    downloads: HashMap<String, (u16, Vec<u8>)>,
}
impl AddonTransport for DownloadTransport {
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
            .unwrap_or_else(|| panic!("{what}: no fake download response for {url}"));
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
        if url.ends_with("/v1/mods/files") {
            return Ok(RawResponse {
                status: 200,
                body: serde_json::to_vec(&self.files_response).unwrap(),
            });
        }
        if url.ends_with("/v2/version_files") {
            // Post-download override-jar classification (`classify_override_jars`)
            // hash-identifies every jar in the add-on folder, CurseForge-
            // resolved ones included -- the manifest carries no per-file
            // env the way `.mrpack` does, so Tier 0/Tier 2 classification
            // runs over the whole folder, per this phase's own
            // "CurseForge-imported jar... checks known_client_only_reason
            // (Tier 0) first" finding. No hits here -- nothing to disable.
            return Ok(RawResponse {
                status: 200,
                body: b"{}".to_vec(),
            });
        }
        panic!("{what}: unexpected POST {url}");
    }
}

fn secrets_with_key() -> FakeSecretStore {
    let s = FakeSecretStore::new();
    s.set("curseforge.api-key", "test-key").unwrap();
    s
}

#[allow(clippy::too_many_arguments)]
fn run_import(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    server_dir: &Path,
    metadata: &CurseForgeManifestMetadata,
    staged_dir: &Path,
    pack_managed: bool,
    explicit_replace: bool,
    should_cancel: &dyn Fn() -> bool,
) -> Result<modpacks::CurseForgeImportReport, CurseForgeImportError> {
    modpacks::import_curseforge(
        transport,
        secrets,
        &StdFileSystem,
        server_dir,
        JavaServerFlavor::Forge,
        metadata,
        staged_dir,
        pack_managed,
        explicit_replace,
        should_cancel,
    )
}

#[test]
fn curseforge_pack_import_import_missing_api_key_stops_before_any_file_resolution() {
    let tmp = TempDir::new("missing-key");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let m = metadata("overrides", vec![cf_file(100, 5000001)]);
    let transport = FakeTransport {
        files_response: serde_json::json!({"data": []}),
    };
    let no_key = FakeSecretStore::new();

    let result = run_import(
        &transport,
        &no_key,
        &server_dir,
        &m,
        &staged_dir,
        false,
        false,
        &|| false,
    );
    assert!(matches!(result, Err(CurseForgeImportError::MissingApiKey)));
}

#[test]
fn curseforge_pack_import_import_downloads_resolvable_file() {
    let tmp = TempDir::new("resolvable");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let m = metadata("overrides", vec![cf_file(306612, 5000001)]);
    let bytes = b"real jar bytes";
    let files_response = serde_json::json!({
        "data": [
            {"id": 5000001, "modId": 306612, "fileName": "SomeMod-1.0.jar", "downloadUrl": "https://edge.forgecdn.net/SomeMod-1.0.jar", "fileLength": bytes.len()}
        ]
    });
    let mut downloads = HashMap::new();
    downloads.insert(
        "https://edge.forgecdn.net/SomeMod-1.0.jar".to_string(),
        (200u16, bytes.to_vec()),
    );
    let transport = DownloadTransport {
        files_response,
        downloads,
    };
    let secrets = secrets_with_key();

    let report = run_import(
        &transport,
        &secrets,
        &server_dir,
        &m,
        &staged_dir,
        false,
        false,
        &|| false,
    )
    .unwrap();

    assert_eq!(report.installed_files.len(), 1);
    assert!(report.blocked_files.is_empty());
    assert!(report.failed_files.is_empty());
    assert_eq!(report.pack_name, "Test CF Pack");
    assert_eq!(
        fs::read(server_dir.join("mods/SomeMod-1.0.jar")).unwrap(),
        bytes
    );
}

#[test]
fn curseforge_pack_import_import_blocked_file_collected_separately_not_a_failure() {
    let tmp = TempDir::new("blocked-file");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let m = metadata("overrides", vec![cf_file(448233, 8287121)]);
    let files_response = serde_json::json!({
        "data": [
            {"id": 8287121, "modId": 448233, "fileName": "entityculling-forge.jar", "downloadUrl": null, "fileLength": 1589078}
        ]
    });
    let transport = FakeTransport { files_response };
    let secrets = secrets_with_key();

    let report = run_import(
        &transport,
        &secrets,
        &server_dir,
        &m,
        &staged_dir,
        false,
        false,
        &|| false,
    )
    .unwrap();

    assert!(report.installed_files.is_empty());
    assert!(
        report.failed_files.is_empty(),
        "a blocked file must never be counted as a failure"
    );
    assert_eq!(report.blocked_files.len(), 1);
    assert_eq!(report.blocked_files[0].file_id, 8287121);
    assert_eq!(report.blocked_files[0].project_id, 448233);
    assert_eq!(
        report.blocked_files[0].expected_file_name,
        "entityculling-forge.jar"
    );
    assert_eq!(report.blocked_files[0].expected_byte_size, 1589078);
}

#[test]
fn curseforge_pack_import_import_merges_overrides_folder_named_from_manifest() {
    let tmp = TempDir::new("overrides-named");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    write_file(
        &staged_dir,
        "custom-overrides/config/shared.txt",
        b"cf override",
    );
    let m = metadata("custom-overrides", vec![]);
    let transport = FakeTransport {
        files_response: serde_json::json!({"data": []}),
    };
    let secrets = secrets_with_key();

    let report = run_import(
        &transport,
        &secrets,
        &server_dir,
        &m,
        &staged_dir,
        false,
        false,
        &|| false,
    )
    .unwrap();
    let _ = report;

    assert_eq!(
        fs::read(server_dir.join("config/shared.txt")).unwrap(),
        b"cf override"
    );
}

#[test]
fn curseforge_pack_import_import_cancellation_rolls_back_files_already_written() {
    let tmp = TempDir::new("cancellation");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let m = metadata(
        "overrides",
        vec![cf_file(100, 5000001), cf_file(200, 5000002)],
    );
    let files_response = serde_json::json!({
        "data": [
            {"id": 5000001, "modId": 100, "fileName": "First.jar", "downloadUrl": "https://edge.forgecdn.net/First.jar", "fileLength": 5},
            {"id": 5000002, "modId": 200, "fileName": "Second.jar", "downloadUrl": "https://edge.forgecdn.net/Second.jar", "fileLength": 6}
        ]
    });
    let mut downloads = HashMap::new();
    downloads.insert(
        "https://edge.forgecdn.net/First.jar".to_string(),
        (200u16, b"first".to_vec()),
    );
    downloads.insert(
        "https://edge.forgecdn.net/Second.jar".to_string(),
        (200u16, b"second".to_vec()),
    );
    let transport = DownloadTransport {
        files_response,
        downloads,
    };
    let secrets = secrets_with_key();

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
        &secrets,
        &server_dir,
        &m,
        &staged_dir,
        false,
        false,
        &should_cancel,
    )
    .unwrap();

    assert!(report.cancelled);
    assert!(!server_dir.join("mods/First.jar").exists());
    assert!(!server_dir.join("mods/Second.jar").exists());
}

struct PanicTransport;
impl AddonTransport for PanicTransport {
    fn get(
        &self,
        url: &str,
        what: &str,
        _: &[(&str, &str)],
        _: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!(
            "{what}: unexpected GET {url} -- pack-managed refusal must happen before any network call"
        );
    }
    fn post_json(
        &self,
        url: &str,
        what: &str,
        _: &serde_json::Value,
        _: &[(&str, &str)],
        _: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!(
            "{what}: unexpected POST {url} -- pack-managed refusal must happen before any network call"
        );
    }
}

#[test]
fn curseforge_pack_import_import_refused_on_pack_managed_server_without_explicit_replace_intent() {
    let tmp = TempDir::new("pack-managed");
    let server_dir = tmp.path().join("server");
    let staged_dir = tmp.path().join("staged");
    fs::create_dir_all(&staged_dir).unwrap();
    let m = metadata("overrides", vec![cf_file(100, 5000001)]);
    // Any transport call at all would panic -- proves refusal happens
    // before even the CurseForge file-resolution step.
    let transport = PanicTransport;
    let secrets = secrets_with_key();

    let result = run_import(
        &transport,
        &secrets,
        &server_dir,
        &m,
        &staged_dir,
        true,
        false,
        &|| false,
    );
    assert!(matches!(result, Err(CurseForgeImportError::PackManaged)));
}
