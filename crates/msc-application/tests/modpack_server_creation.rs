//! P8.21's own tests: `msc_application::provisioning::create_server_from_pack`
//! against a real (in-memory-transport-driven) Fabric `.mrpack` create and
//! a real Forge CurseForge-pack create, plus the pre-claim rejection paths
//! (no pinned Minecraft version, an unsupported loader) and the two
//! rollback paths (cancellation mid-create, a hard pack-import failure)
//! that must leave no half-provisioned server or staged residue behind.
//!
//! Reuses `crates/msc-application/tests/provisioning_install_step.rs`'s own
//! `FakeProcessSupervisor`-driven-from-a-background-thread pattern for the
//! Forge/NeoForge installer half — see that file's own doc for why the
//! java-version-probe/installer spawn sequence needs a background thread
//! at all.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use msc_application::modpacks::{InspectedFormat, ModpackInspection};
use msc_application::provisioning::{
    self, CreateFromPackError, PackApplyReport, PackServerRequest, WorldSource,
    real_copy_existing_world_folder, real_unzip_world_backup,
};
use msc_domain::identity::JavaServerFlavor;
use msc_domain::modpack_manifest::{
    CurseForgeManifestFile, CurseForgeManifestMetadata, LoaderFlavor, MrpackFileEntry,
    MrpackFileHashes, MrpackManifest,
};
use msc_infrastructure::addon_provider::{AddonTransport, RawResponse, TransportError};
use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::jar_provider::{self, JarProviderError};
use msc_infrastructure::process::{FakeProcessSupervisor, OutputStream};
use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};

struct TempDir(PathBuf);
impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-pack-server-creation-test-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
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

fn never_cancelled() -> bool {
    false
}
fn no_output(_stream: OutputStream, _bytes: &[u8]) {}
fn always_ok2(_a: &Path, _b: &Path) -> bool {
    true
}
fn always_ok3(_a: &Path, _b: &Path, _c: &str) -> bool {
    true
}

fn base_pack_request() -> PackServerRequest<'static> {
    PackServerRequest {
        name: "Modpack Server",
        initial_world_name: None,
        port: 25565,
        enable_cross_play: false,
        cross_play_bedrock_port: None,
        enable_playit: false,
        enable_xbox_broadcast: false,
        difficulty: "normal",
        gamemode: "survival",
        world_seed: None,
        initial_world_profile: None,
        world_source: WorldSource::Fresh,
        default_banner_color_hex: "#3366FF",
    }
}

fn mrpack_manifest(
    minecraft: Option<&str>,
    loader_key: Option<(&str, &str)>,
    files: Vec<MrpackFileEntry>,
) -> MrpackManifest {
    let mut dependencies = HashMap::new();
    if let Some(mc) = minecraft {
        dependencies.insert("minecraft".to_string(), mc.to_string());
    }
    if let Some((key, version)) = loader_key {
        dependencies.insert(key.to_string(), version.to_string());
    }
    MrpackManifest {
        name: "Test Fabric Pack".to_string(),
        version_id: "1.2.0".to_string(),
        game: "minecraft".to_string(),
        dependencies,
        files,
    }
}

fn mrpack_mod_file(path: &str, download_url: &str) -> MrpackFileEntry {
    MrpackFileEntry {
        path: path.to_string(),
        hashes: MrpackFileHashes {
            sha1: None,
            sha512: None,
        },
        env: None,
        downloads: vec![download_url.to_string()],
        file_size: 0,
    }
}

fn curseforge_metadata(
    minecraft_version: &str,
    loader_flavor: Option<LoaderFlavor>,
    loader_version: Option<&str>,
    files: Vec<CurseForgeManifestFile>,
) -> CurseForgeManifestMetadata {
    CurseForgeManifestMetadata {
        name: "Test CF Pack".to_string(),
        version_id: "3.0.0".to_string(),
        minecraft_version: minecraft_version.to_string(),
        loader_flavor,
        loader_version: loader_version.map(str::to_string),
        overrides_folder: "overrides".to_string(),
        files,
    }
}

/// A staged, already-extracted directory an `inspect_staged_archive` call
/// would have produced — empty `overrides/`/`server-overrides/` unless a
/// caller populates them, since none of this file's cases need override
/// content beyond the manifest-declared downloads themselves.
fn staged_dir(tmp: &TempDir, label: &str) -> PathBuf {
    let dir = tmp.path().join("staged").join(label);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn inspection(format: InspectedFormat, staged_dir: PathBuf) -> ModpackInspection {
    ModpackInspection {
        format,
        pinned_version: None,
        manual_downloads: Vec::new(),
        curseforge_lookup_available: false,
        override_file_count: 0,
        staged_dir,
    }
}

// ---------------------------------------------------------------------
// Combined jar_provider::Transport + AddonTransport fake
// ---------------------------------------------------------------------

struct FakeTransport {
    get_responses: Mutex<HashMap<String, (u16, Vec<u8>)>>,
    post_responses: Mutex<HashMap<String, (u16, serde_json::Value)>>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            get_responses: Mutex::new(HashMap::new()),
            post_responses: Mutex::new(HashMap::new()),
        }
    }

    fn with_get(self, url: &str, status: u16, body: impl Into<Vec<u8>>) -> Self {
        self.get_responses
            .lock()
            .unwrap()
            .insert(url.to_string(), (status, body.into()));
        self
    }

    fn with_post(self, url_suffix: &str, status: u16, body: serde_json::Value) -> Self {
        self.post_responses
            .lock()
            .unwrap()
            .insert(url_suffix.to_string(), (status, body));
        self
    }
}

impl jar_provider::Transport for FakeTransport {
    fn get(&self, url: &str, what: &str, _max_bytes: u64) -> Result<Vec<u8>, JarProviderError> {
        self.get_responses
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .map(|(_, body)| body)
            .ok_or_else(|| {
                JarProviderError::Network(format!("{what}: no fake response registered for {url}"))
            })
    }
}

impl AddonTransport for FakeTransport {
    fn get(
        &self,
        url: &str,
        what: &str,
        _headers: &[(&str, &str)],
        _max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        self.get_responses
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .map(|(status, body)| RawResponse { status, body })
            .ok_or_else(|| TransportError::Network(format!("{what}: no fake GET for {url}")))
    }

    fn post_json(
        &self,
        url: &str,
        what: &str,
        _payload: &serde_json::Value,
        _headers: &[(&str, &str)],
        _max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        let responses = self.post_responses.lock().unwrap();
        for (suffix, (status, body)) in responses.iter() {
            if url.ends_with(suffix.as_str()) {
                return Ok(RawResponse {
                    status: *status,
                    body: serde_json::to_vec(body).unwrap(),
                });
            }
        }
        Err(TransportError::Network(format!(
            "{what}: no fake POST for {url}"
        )))
    }
}

/// No override jars ever get identified against Modrinth in this file's
/// fixtures (every manifest-declared mod is a synthetic, unpublished
/// fake) — every case registers this same empty `version_files` response
/// so `classify_override_jars` runs its real hash-identify call and finds
/// nothing, rather than skipping that call entirely.
fn no_modrinth_hits(t: FakeTransport) -> FakeTransport {
    t.with_post("/v2/version_files", 200, serde_json::json!({}))
}

// ---------------------------------------------------------------------
// Fabric + .mrpack: no installer subprocess involved.
// ---------------------------------------------------------------------

#[test]
fn modpack_server_creation_fabric_mrpack_end_to_end() {
    let tmp = TempDir::new("fabric-mrpack");
    let transport = no_modrinth_hits(
        FakeTransport::new()
            .with_get(
                "https://meta.fabricmc.net/v2/versions/installer",
                200,
                br#"[{"version":"1.0.1","stable":true}]"#.to_vec(),
            )
            .with_get(
                "https://meta.fabricmc.net/v2/versions/loader/1.20.1/0.15.11/1.0.1/server/jar",
                200,
                b"FAKE-FABRIC-SERVER-JAR".to_vec(),
            )
            .with_get(
                "https://cdn.modrinth.com/data/AAAA/versions/1.0/examplemod.jar",
                200,
                b"FAKE-MOD-JAR".to_vec(),
            ),
    );

    let manifest = mrpack_manifest(
        Some("1.20.1"),
        Some(("fabric-loader", "0.15.11")),
        vec![mrpack_mod_file(
            "mods/examplemod.jar",
            "https://cdn.modrinth.com/data/AAAA/versions/1.0/examplemod.jar",
        )],
    );
    let staged = staged_dir(&tmp, "fabric-ok");
    let inspection = inspection(InspectedFormat::Mrpack(manifest), staged.clone());
    let request = base_pack_request();

    let result = provisioning::create_server_from_pack(
        &StdFileSystem,
        &transport,
        &transport,
        &FakeSecretStore::new(),
        &FakeProcessSupervisor::new(),
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/plugin"),
        &request,
        &inspection,
        "/usr/bin/java",
        Duration::from_secs(5),
        "2026-08-21T00:00:00Z",
        &never_cancelled,
        no_output,
        real_unzip_world_backup,
        real_copy_existing_world_folder,
    )
    .unwrap();

    assert_eq!(result.created.config.java_flavor, JavaServerFlavor::Fabric);
    assert_eq!(
        result.created.config.minecraft_version.as_deref(),
        Some("1.20.1")
    );
    assert_eq!(
        result.created.config.loader_version.as_deref(),
        Some("0.15.11")
    );
    assert!(result.created.config.pack_managed);
    assert_eq!(
        result.created.config.pack_name.as_deref(),
        Some("Test Fabric Pack")
    );
    assert_eq!(result.created.config.pack_version.as_deref(), Some("1.2.0"));

    let server_dir = PathBuf::from(&result.created.config.server_dir);
    assert!(server_dir.join("mods/examplemod.jar").is_file());

    match result.pack_report {
        PackApplyReport::Mrpack(report) => {
            assert_eq!(report.installed_files.len(), 1);
            assert!(report.failed_files.is_empty());
        }
        PackApplyReport::CurseForge(_) => panic!("expected an mrpack report"),
    }

    // Staged directory is cleaned up once its content is either merged
    // into the new server or independently re-downloaded.
    assert!(!staged.exists());
}

// ---------------------------------------------------------------------
// Forge + CurseForge pack: exercises both the pinned-build-exists check
// (against real Maven metadata parsing) and the loader-installer
// subprocess path.
// ---------------------------------------------------------------------

const FORGE_METADATA_URL: &str =
    "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
const FORGE_INSTALLER_URL: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.5/forge-1.20.1-47.4.5-installer.jar";

fn forge_metadata_transport() -> FakeTransport {
    no_modrinth_hits(
        FakeTransport::new().with_get(
            FORGE_METADATA_URL,
            200,
            br#"<?xml version="1.0"?><metadata><versioning><versions>
              <version>1.20.1-47.4.5</version>
            </versions></versioning></metadata>"#
                .to_vec(),
        ),
    )
}

const SPIN_WAIT_DEADLINE: Duration = Duration::from_secs(30);

fn wait_for_spawn(
    supervisor: &FakeProcessSupervisor,
    matches: impl Fn(&msc_infrastructure::process::ProcessSpawnRequest) -> bool,
    what: &str,
) -> msc_infrastructure::process::ProcessId {
    let deadline = std::time::Instant::now() + SPIN_WAIT_DEADLINE;
    loop {
        if let Some((pid, _)) = supervisor
            .spawned_requests()
            .into_iter()
            .find(|(_, request)| matches(request))
        {
            return pid;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "no {what} process was spawned within {SPIN_WAIT_DEADLINE:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn is_version_probe(request: &msc_infrastructure::process::ProcessSpawnRequest) -> bool {
    request.arguments.iter().any(|arg| arg == "-version")
}

fn drive_fake_java_version_probe(supervisor: &FakeProcessSupervisor) {
    let pid = wait_for_spawn(supervisor, is_version_probe, "java -version probe");
    supervisor
        .emit_stdout(pid, b"openjdk version \"21.0.1\" 2023-10-17\n")
        .unwrap();
    supervisor.exit_normally(pid).unwrap();
}

fn wait_for_first_spawn(
    supervisor: &FakeProcessSupervisor,
) -> msc_infrastructure::process::ProcessId {
    wait_for_spawn(supervisor, |r| !is_version_probe(r), "installer")
}

fn drive_fake_installer_to_success(supervisor: &FakeProcessSupervisor, after_spawn: impl FnOnce()) {
    drive_fake_java_version_probe(supervisor);
    let pid = wait_for_first_spawn(supervisor);
    after_spawn();
    supervisor.emit_stdout(pid, b"Installing...\n").unwrap();
    supervisor.exit_normally(pid).unwrap();
}

fn cf_file(project_id: i64, file_id: i64) -> CurseForgeManifestFile {
    CurseForgeManifestFile {
        project_id,
        file_id,
        required: true,
    }
}

#[test]
fn modpack_server_creation_forge_curseforge_end_to_end() {
    let tmp = TempDir::new("forge-curseforge");
    let transport = forge_metadata_transport()
        .with_get(FORGE_INSTALLER_URL, 200, b"FAKE-FORGE-INSTALLER".to_vec())
        .with_get(
            "https://edge.forgecdn.net/files/1/1/examplemod.jar",
            200,
            b"FAKE-CF-MOD-JAR".to_vec(),
        )
        .with_post(
            "/v1/mods/files",
            200,
            serde_json::json!({
                "data": [{
                    "id": 111,
                    "modId": 222,
                    "fileName": "examplemod.jar",
                    "downloadUrl": "https://edge.forgecdn.net/files/1/1/examplemod.jar",
                    "fileLength": 15,
                }]
            }),
        );

    let secrets = FakeSecretStore::new();
    secrets
        .set(
            msc_infrastructure::addon_provider::CURSEFORGE_API_KEY_SECRET,
            "test-key",
        )
        .unwrap();

    let metadata = curseforge_metadata(
        "1.20.1",
        Some(LoaderFlavor::Forge),
        Some("47.4.5"),
        vec![cf_file(222, 111)],
    );
    let staged = staged_dir(&tmp, "forge-cf-ok");
    let inspection = inspection(InspectedFormat::CurseForge(metadata), staged.clone());
    let request = base_pack_request();
    let supervisor = FakeProcessSupervisor::new();

    let result = std::thread::scope(|scope| {
        let server_dir = tmp.path().join("java").join("modpack_server");
        let args_dir = server_dir.join("libraries/net/minecraftforge/forge/1.20.1-47.4.5");
        let supervisor_ref = &supervisor;
        scope.spawn(move || {
            drive_fake_installer_to_success(supervisor_ref, || {
                fs::create_dir_all(&args_dir).unwrap();
                fs::write(
                    args_dir.join("unix_args.txt"),
                    b"@user_jvm_args.txt\nnogui\n",
                )
                .unwrap();
            });
        });

        provisioning::create_server_from_pack(
            &StdFileSystem,
            &transport,
            &transport,
            &secrets,
            &supervisor,
            tmp.path(),
            tmp.path(),
            &tmp.path().join("templates/plugin"),
            &request,
            &inspection,
            "/usr/bin/java",
            Duration::from_secs(5),
            "2026-08-21T00:00:00Z",
            &never_cancelled,
            no_output,
            always_ok2,
            always_ok3,
        )
        .unwrap()
    });

    assert_eq!(result.created.config.java_flavor, JavaServerFlavor::Forge);
    assert_eq!(
        result.created.config.minecraft_version.as_deref(),
        Some("1.20.1")
    );
    assert_eq!(
        result.created.config.loader_version.as_deref(),
        Some("47.4.5")
    );
    assert_eq!(result.created.config.paper_jar_path, "");
    assert!(result.created.config.pack_managed);
    assert_eq!(
        result.created.config.pack_name.as_deref(),
        Some("Test CF Pack")
    );

    let server_dir = PathBuf::from(&result.created.config.server_dir);
    assert!(!server_dir.join("forge-installer.jar").exists());
    assert!(server_dir.join("mods/examplemod.jar").is_file());

    match result.pack_report {
        PackApplyReport::CurseForge(report) => {
            assert_eq!(report.installed_files.len(), 1);
            assert!(report.blocked_files.is_empty());
            assert!(report.failed_files.is_empty());
        }
        PackApplyReport::Mrpack(_) => panic!("expected a CurseForge report"),
    }
    assert!(!staged.exists());
}

#[test]
fn modpack_server_creation_forge_pinned_build_not_found_rolls_back() {
    let tmp = TempDir::new("forge-build-missing");
    // Maven metadata exists, but not for the pinned build this pack asks
    // for -- must fail before any download, and before the installer
    // ever spawns.
    let transport = no_modrinth_hits(
        FakeTransport::new().with_get(
            FORGE_METADATA_URL,
            200,
            br#"<?xml version="1.0"?><metadata><versioning><versions>
              <version>1.19.2-43.2.0</version>
            </versions></versioning></metadata>"#
                .to_vec(),
        ),
    );

    let metadata = curseforge_metadata(
        "1.20.1",
        Some(LoaderFlavor::Forge),
        Some("47.4.5"),
        Vec::new(),
    );
    let staged = staged_dir(&tmp, "forge-missing-build");
    let inspection = inspection(InspectedFormat::CurseForge(metadata), staged.clone());
    let request = base_pack_request();

    let err = provisioning::create_server_from_pack(
        &StdFileSystem,
        &transport,
        &transport,
        &FakeSecretStore::new(),
        &FakeProcessSupervisor::new(),
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/plugin"),
        &request,
        &inspection,
        "/usr/bin/java",
        Duration::from_secs(5),
        "2026-08-21T00:00:00Z",
        &never_cancelled,
        no_output,
        always_ok2,
        always_ok3,
    )
    .unwrap_err();

    assert!(matches!(
        err,
        CreateFromPackError::PinnedForgeBuildNotFound { .. }
    ));
    assert!(!tmp.path().join("java/modpack_server").exists());
    assert!(!staged.exists());
}

// ---------------------------------------------------------------------
// Pre-claim rejection: neither of these creates a server directory at
// all, since `pack_loader_pin` runs before `claim_new_server_directory`.
// ---------------------------------------------------------------------

#[test]
fn modpack_server_creation_missing_minecraft_version_rejected_before_claim() {
    let tmp = TempDir::new("no-mc-version");
    let manifest = mrpack_manifest(None, Some(("fabric-loader", "0.15.11")), Vec::new());
    let staged = staged_dir(&tmp, "no-mc");
    let inspection = inspection(InspectedFormat::Mrpack(manifest), staged.clone());
    let request = base_pack_request();
    let transport = FakeTransport::new();

    let err = provisioning::create_server_from_pack(
        &StdFileSystem,
        &transport,
        &transport,
        &FakeSecretStore::new(),
        &FakeProcessSupervisor::new(),
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/plugin"),
        &request,
        &inspection,
        "/usr/bin/java",
        Duration::from_secs(5),
        "2026-08-21T00:00:00Z",
        &never_cancelled,
        no_output,
        always_ok2,
        always_ok3,
    )
    .unwrap_err();

    assert!(matches!(err, CreateFromPackError::MissingMinecraftVersion));
    assert!(!tmp.path().join("java").exists());
}

#[test]
fn modpack_server_creation_quilt_loader_unsupported() {
    let tmp = TempDir::new("quilt-unsupported");
    let manifest = mrpack_manifest(Some("1.20.1"), Some(("quilt-loader", "0.23.0")), Vec::new());
    let staged = staged_dir(&tmp, "quilt");
    let inspection = inspection(InspectedFormat::Mrpack(manifest), staged.clone());
    let request = base_pack_request();
    let transport = FakeTransport::new();

    let err = provisioning::create_server_from_pack(
        &StdFileSystem,
        &transport,
        &transport,
        &FakeSecretStore::new(),
        &FakeProcessSupervisor::new(),
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/plugin"),
        &request,
        &inspection,
        "/usr/bin/java",
        Duration::from_secs(5),
        "2026-08-21T00:00:00Z",
        &never_cancelled,
        no_output,
        always_ok2,
        always_ok3,
    )
    .unwrap_err();

    assert!(matches!(err, CreateFromPackError::UnsupportedLoader));
    assert!(!tmp.path().join("java").exists());
}

// ---------------------------------------------------------------------
// Rollback paths: cancellation mid-create, and a hard pack-import
// failure -- both must remove the whole `new_dir`, not just their own
// partial writes.
// ---------------------------------------------------------------------

#[test]
fn modpack_server_creation_cancelled_before_pack_apply_rolls_back_new_dir() {
    let tmp = TempDir::new("cancel-before-apply");
    let transport = no_modrinth_hits(
        FakeTransport::new()
            .with_get(
                "https://meta.fabricmc.net/v2/versions/installer",
                200,
                br#"[{"version":"1.0.1","stable":true}]"#.to_vec(),
            )
            .with_get(
                "https://meta.fabricmc.net/v2/versions/loader/1.20.1/0.15.11/1.0.1/server/jar",
                200,
                b"FAKE-FABRIC-SERVER-JAR".to_vec(),
            ),
    );

    let manifest = mrpack_manifest(
        Some("1.20.1"),
        Some(("fabric-loader", "0.15.11")),
        vec![mrpack_mod_file(
            "mods/examplemod.jar",
            "https://cdn.modrinth.com/data/AAAA/versions/1.0/examplemod.jar",
        )],
    );
    let staged = staged_dir(&tmp, "cancel");
    let inspection = inspection(InspectedFormat::Mrpack(manifest), staged.clone());
    let request = base_pack_request();

    // Cancels only once the shared creation tail (jar download, eula,
    // properties, world slot) has already run -- proving cancellation is
    // observed right before pack apply, and that it still rolls the
    // *entire* new_dir back, not just the not-yet-attempted pack files.
    let calls = AtomicUsize::new(0);
    let should_cancel = || calls.fetch_add(1, Ordering::SeqCst) >= 1;

    let err = provisioning::create_server_from_pack(
        &StdFileSystem,
        &transport,
        &transport,
        &FakeSecretStore::new(),
        &FakeProcessSupervisor::new(),
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/plugin"),
        &request,
        &inspection,
        "/usr/bin/java",
        Duration::from_secs(5),
        "2026-08-21T00:00:00Z",
        &should_cancel,
        no_output,
        always_ok2,
        always_ok3,
    )
    .unwrap_err();

    assert!(matches!(err, CreateFromPackError::Cancelled));
    assert!(!tmp.path().join("java/modpack_server").exists());
    assert!(!staged.exists());
}

#[test]
fn modpack_server_creation_missing_curseforge_key_rolls_back_new_dir() {
    let tmp = TempDir::new("missing-cf-key");
    let transport = forge_metadata_transport().with_get(
        FORGE_INSTALLER_URL,
        200,
        b"FAKE-FORGE-INSTALLER".to_vec(),
    );
    // No CurseForge API key set on this SecretStore at all.
    let metadata = curseforge_metadata(
        "1.20.1",
        Some(LoaderFlavor::Forge),
        Some("47.4.5"),
        vec![cf_file(222, 111)],
    );
    let staged = staged_dir(&tmp, "no-key");
    let inspection = inspection(InspectedFormat::CurseForge(metadata), staged.clone());
    let request = base_pack_request();
    let supervisor = FakeProcessSupervisor::new();

    let err = std::thread::scope(|scope| {
        let server_dir = tmp.path().join("java").join("modpack_server");
        let args_dir = server_dir.join("libraries/net/minecraftforge/forge/1.20.1-47.4.5");
        let supervisor_ref = &supervisor;
        scope.spawn(move || {
            drive_fake_installer_to_success(supervisor_ref, || {
                fs::create_dir_all(&args_dir).unwrap();
                fs::write(
                    args_dir.join("unix_args.txt"),
                    b"@user_jvm_args.txt\nnogui\n",
                )
                .unwrap();
            });
        });

        provisioning::create_server_from_pack(
            &StdFileSystem,
            &transport,
            &transport,
            &FakeSecretStore::new(),
            &supervisor,
            tmp.path(),
            tmp.path(),
            &tmp.path().join("templates/plugin"),
            &request,
            &inspection,
            "/usr/bin/java",
            Duration::from_secs(5),
            "2026-08-21T00:00:00Z",
            &never_cancelled,
            no_output,
            always_ok2,
            always_ok3,
        )
        .unwrap_err()
    });

    assert!(matches!(err, CreateFromPackError::Import(_)));
    // The loader was fully provisioned before the pack-apply failure --
    // proving this really is "roll back a fully-created server", not
    // just "never created one".
    assert!(!tmp.path().join("java/modpack_server").exists());
    assert!(!staged.exists());
}
