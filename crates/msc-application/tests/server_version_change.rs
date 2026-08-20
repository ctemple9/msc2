//! P7.19: `msc_application::server_versions::{change_version,
//! list_versions_for_server}` against `changeVersionProvider`
//! (`AppViewModel+APIWiringAddons.swift:358-573`) and `ServerJarProvider.
//! listVersions(for:)` (`ServerJarProviders.swift:68-77`). No dedicated
//! `fixtures/` directory was characterized for this behavior in P7.4-P7.9
//! (only catalog parsing and jar-filename parsing were) — these cases are
//! read directly from source with file:line citations, the same practice
//! P7.9/P7.16/P7.17 already established for an uncharacterized gap.
//!
//! Forge/NeoForge's own installer polling/timeout/cancellation machinery
//! was already proven against a real `java` subprocess at P7.14; the two
//! install-step tests here (driven by [`FakeProcessSupervisor`], same
//! pattern as `provisioning_install_step.rs`) prove this step's own
//! composition (version resolution + installer args + cleanup), not
//! `run_loader_installer` itself again.

use msc_application::server_versions::{self, ChangeVersionError, ChangeVersionRequest, LATEST};
use msc_domain::identity::JavaServerFlavor;
use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::jar_provider::{JarProviderError, Transport};
use msc_infrastructure::process::{FakeProcessSupervisor, OutputStream};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-server-version-change-test-{label}-{}-{}",
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

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/providers")
}

struct FakeTransport {
    responses: Mutex<HashMap<String, Vec<u8>>>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
        }
    }
    fn with_bytes(self, url: &str, bytes: impl Into<Vec<u8>>) -> Self {
        self.responses
            .lock()
            .unwrap()
            .insert(url.to_string(), bytes.into());
        self
    }
    fn with_file(self, url: &str, relative_corpus_path: &str) -> Self {
        let bytes = fs::read(corpus_dir().join(relative_corpus_path))
            .unwrap_or_else(|e| panic!("reading {relative_corpus_path}: {e}"));
        self.with_bytes(url, bytes)
    }
}

impl Transport for FakeTransport {
    fn get(&self, url: &str, what: &str, _max_bytes: u64) -> Result<Vec<u8>, JarProviderError> {
        self.responses
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or_else(|| {
                JarProviderError::Network(format!("{what}: no fake response registered for {url}"))
            })
    }
}

fn never_cancelled() -> bool {
    false
}
fn no_output(_stream: OutputStream, _bytes: &[u8]) {}
fn backup_ok() -> bool {
    true
}
fn backup_fails() -> bool {
    false
}

const INSTALLER_TIMEOUT: Duration = Duration::from_secs(30);

fn base_request<'a>(flavor: JavaServerFlavor, server_dir: &'a Path) -> ChangeVersionRequest<'a> {
    ChangeVersionRequest {
        flavor,
        version_id: LATEST,
        loader_version: None,
        current_minecraft_version: None,
        server_dir,
        paper_jar_path: "",
    }
}

// --- guards ---

#[test]
fn server_version_change_refuses_while_running() {
    let tmp = TempDir::new("running");
    let transport = FakeTransport::new();
    let supervisor = FakeProcessSupervisor::new();
    let request = base_request(JavaServerFlavor::Paper, tmp.path());
    let err = server_versions::change_version(
        &StdFileSystem,
        &transport,
        &supervisor,
        "java",
        INSTALLER_TIMEOUT,
        &request,
        true,
        false,
        backup_ok,
        &never_cancelled,
        no_output,
    )
    .expect_err("server running");
    assert!(matches!(err, ChangeVersionError::ServerRunning));
}

#[test]
fn server_version_change_refuses_download_in_progress() {
    let tmp = TempDir::new("downloading");
    let transport = FakeTransport::new();
    let supervisor = FakeProcessSupervisor::new();
    let request = base_request(JavaServerFlavor::Paper, tmp.path());
    let err = server_versions::change_version(
        &StdFileSystem,
        &transport,
        &supervisor,
        "java",
        INSTALLER_TIMEOUT,
        &request,
        false,
        true,
        backup_ok,
        &never_cancelled,
        no_output,
    )
    .expect_err("download in progress");
    assert!(matches!(err, ChangeVersionError::DownloadInProgress));
}

#[test]
fn server_version_change_refuses_unsupported_flavor_pufferfish() {
    let tmp = TempDir::new("pufferfish");
    let transport = FakeTransport::new();
    let supervisor = FakeProcessSupervisor::new();
    let request = base_request(JavaServerFlavor::Pufferfish, tmp.path());
    let err = server_versions::change_version(
        &StdFileSystem,
        &transport,
        &supervisor,
        "java",
        INSTALLER_TIMEOUT,
        &request,
        false,
        false,
        backup_ok,
        &never_cancelled,
        no_output,
    )
    .expect_err("pufferfish unsupported");
    assert!(matches!(
        err,
        ChangeVersionError::UnsupportedFlavor(JavaServerFlavor::Pufferfish)
    ));
}

#[test]
fn server_version_change_downgrade_triggers_backup_and_aborts_on_failure() {
    let tmp = TempDir::new("downgrade-abort");
    let transport = FakeTransport::new();
    let supervisor = FakeProcessSupervisor::new();
    let mut request = base_request(JavaServerFlavor::Paper, tmp.path());
    request.version_id = "1.20.1";
    request.current_minecraft_version = Some("1.21.4");

    let err = server_versions::change_version(
        &StdFileSystem,
        &transport,
        &supervisor,
        "java",
        INSTALLER_TIMEOUT,
        &request,
        false,
        false,
        backup_fails,
        &never_cancelled,
        no_output,
    )
    .expect_err("backup failed aborts before any download");
    assert!(matches!(err, ChangeVersionError::BackupFailed));
}

#[test]
fn server_version_change_upgrade_does_not_trigger_backup() {
    // A move to a *newer* version, or to "latest," must never call the
    // backup closure -- proven by making the closure panic if invoked.
    let tmp = TempDir::new("upgrade-no-backup");
    let transport = FakeTransport::new()
        .with_file(
            "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json",
            "vanilla/version-manifest-v2.json",
        )
        .with_file(
            "https://piston-meta.mojang.com/v1/packages/c75d82e7fa6eca5a043dab0c6cf77cb8317644f4/26.2.json",
            "vanilla/version-26.2.json",
        )
        .with_bytes(
            "https://piston-data.mojang.com/v1/objects/823e2250d24b3ddac457a60c92a6a941943fcd6a/server.jar",
            b"fake vanilla server jar bytes".to_vec(),
        );
    let supervisor = FakeProcessSupervisor::new();
    let mut request = base_request(JavaServerFlavor::Vanilla, tmp.path());
    request.current_minecraft_version = Some("1.20.1");
    let panicking_backup = || -> bool { panic!("backup should not be called for a non-downgrade") };

    let dest_dir = tmp.path();
    fs::create_dir_all(dest_dir).unwrap();
    let result = server_versions::change_version(
        &StdFileSystem,
        &transport,
        &supervisor,
        "java",
        INSTALLER_TIMEOUT,
        &request,
        false,
        false,
        panicking_backup,
        &never_cancelled,
        no_output,
    );
    assert!(result.is_ok());
}

// --- Paper / Purpur / Vanilla (download-and-go) ---

#[test]
fn server_version_change_paper_latest_download() {
    let tmp = TempDir::new("paper-latest");
    let transport = FakeTransport::new()
        .with_file(
            "https://fill.papermc.io/v3/projects/paper",
            "paper/projects-paper.json",
        )
        .with_file(
            "https://fill.papermc.io/v3/projects/paper/versions/1.21.11/builds",
            "paper/builds-1.21.11.json",
        )
        .with_bytes(
            "https://fill-data.papermc.io/v1/objects/5ffef465eeeb5f2a3c23a24419d97c51afd7dbb4923ff42df9a3f58bba1ccfba/paper-1.21.11-132.jar",
            b"fake paper jar".to_vec(),
        );
    let supervisor = FakeProcessSupervisor::new();
    // `paper_jar_path` left empty -> destination falls back to
    // `<serverDir>/paper.jar` (source line 438-441).
    let request = base_request(JavaServerFlavor::Paper, tmp.path());

    let changed = server_versions::change_version(
        &StdFileSystem,
        &transport,
        &supervisor,
        "java",
        INSTALLER_TIMEOUT,
        &request,
        false,
        false,
        backup_ok,
        &never_cancelled,
        no_output,
    )
    .expect("paper latest change");
    assert_eq!(changed.minecraft_version, "1.21.11");
    assert_eq!(changed.build, "132");
    assert_eq!(changed.loader_version, None);
    assert!(fs::read(tmp.path().join("paper.jar")).is_ok());
}

#[test]
fn server_version_change_paper_pinned_downloads_dest_url_when_paper_jar_path_empty() {
    let tmp = TempDir::new("paper-pinned-default-dest");
    let transport = FakeTransport::new()
        .with_file(
            "https://fill.papermc.io/v3/projects/paper/versions/1.21.11/builds",
            "paper/builds-1.21.11.json",
        )
        .with_bytes(
            "https://fill-data.papermc.io/v1/objects/5ffef465eeeb5f2a3c23a24419d97c51afd7dbb4923ff42df9a3f58bba1ccfba/paper-1.21.11-132.jar",
            b"fake paper jar".to_vec(),
        );
    let supervisor = FakeProcessSupervisor::new();
    let mut request = base_request(JavaServerFlavor::Paper, tmp.path());
    request.version_id = "1.21.11";
    // paper_jar_path left empty -> destination falls back to
    // `<serverDir>/paper.jar` (source line 438-441).

    let changed = server_versions::change_version(
        &StdFileSystem,
        &transport,
        &supervisor,
        "java",
        INSTALLER_TIMEOUT,
        &request,
        false,
        false,
        backup_ok,
        &never_cancelled,
        no_output,
    )
    .expect("paper pinned change");
    assert_eq!(changed.minecraft_version, "1.21.11");
    assert_eq!(changed.build, "132");
    assert!(fs::read(tmp.path().join("paper.jar")).is_ok());
}

#[test]
fn server_version_change_purpur_pinned_reports_build_latest_literal() {
    // `PurpurDownloader.downloadVersion(_:to:)` never resolves a real
    // build number -- it reports the literal string `"latest"`, unlike
    // the `downloadLatest` path (`ServerJarProviders.swift:327-333`).
    let tmp = TempDir::new("purpur-pinned");
    let transport = FakeTransport::new().with_bytes(
        "https://api.purpurmc.org/v2/purpur/1.21.4/latest/download",
        b"fake purpur jar".to_vec(),
    );
    let supervisor = FakeProcessSupervisor::new();
    let mut request = base_request(JavaServerFlavor::Purpur, tmp.path());
    request.version_id = "1.21.4";

    let changed = server_versions::change_version(
        &StdFileSystem,
        &transport,
        &supervisor,
        "java",
        INSTALLER_TIMEOUT,
        &request,
        false,
        false,
        backup_ok,
        &never_cancelled,
        no_output,
    )
    .expect("purpur pinned change");
    assert_eq!(changed.minecraft_version, "1.21.4");
    assert_eq!(changed.build, "latest");
}

#[test]
fn server_version_change_vanilla_pinned_release_id() {
    let tmp = TempDir::new("vanilla-pinned");
    let transport = FakeTransport::new()
        .with_file(
            "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json",
            "vanilla/version-manifest-v2.json",
        )
        .with_bytes(
            "https://piston-meta.mojang.com/v1/packages/e846101ba6cf0b548e8b71624c7351b6458c5349/1.20.1.json",
            br#"{"downloads":{"server":{"url":"https://piston-data.mojang.com/fake/1.20.1-server.jar"}}}"#.to_vec(),
        )
        .with_bytes(
            "https://piston-data.mojang.com/fake/1.20.1-server.jar",
            b"fake 1.20.1 server jar".to_vec(),
        );
    let supervisor = FakeProcessSupervisor::new();
    let mut request = base_request(JavaServerFlavor::Vanilla, tmp.path());
    request.version_id = "1.20.1";
    request.current_minecraft_version = Some("1.20"); // upgrade, no backup

    let changed = server_versions::change_version(
        &StdFileSystem,
        &transport,
        &supervisor,
        "java",
        INSTALLER_TIMEOUT,
        &request,
        false,
        false,
        backup_ok,
        &never_cancelled,
        no_output,
    )
    .expect("vanilla pinned change");
    assert_eq!(changed.minecraft_version, "1.20.1");
    assert_eq!(changed.build, "release");
}

// --- Fabric: the pinned-loader-is-unreachable oracle finding ---

#[test]
fn server_version_change_fabric_ignores_requested_pinned_loader() {
    let tmp = TempDir::new("fabric-ignores-pinned-loader");
    let transport = FakeTransport::new()
        .with_bytes(
            "https://meta.fabricmc.net/v2/versions/loader/1.20.1",
            br#"[{"loader":{"version":"0.15.11","stable":true}}]"#.to_vec(),
        )
        .with_bytes(
            "https://meta.fabricmc.net/v2/versions/installer",
            br#"[{"version":"1.0.1","stable":true}]"#.to_vec(),
        )
        .with_bytes(
            "https://meta.fabricmc.net/v2/versions/loader/1.20.1/0.15.11/1.0.1/server/jar",
            b"fake fabric server jar".to_vec(),
        );
    let supervisor = FakeProcessSupervisor::new();
    let mut request = base_request(JavaServerFlavor::Fabric, tmp.path());
    request.version_id = "1.20.1";
    // A caller-requested pinned loader that does NOT match what the
    // catalog resolves -- if this were honored, the download URL/result
    // would carry "9.9.9" instead of the real "0.15.11".
    request.loader_version = Some("9.9.9");

    let changed = server_versions::change_version(
        &StdFileSystem,
        &transport,
        &supervisor,
        "java",
        INSTALLER_TIMEOUT,
        &request,
        false,
        false,
        backup_ok,
        &never_cancelled,
        no_output,
    )
    .expect("fabric pinned mc version change");
    assert_eq!(changed.minecraft_version, "1.20.1");
    assert_eq!(changed.loader_version.as_deref(), Some("0.15.11"));
    assert_ne!(changed.loader_version.as_deref(), Some("9.9.9"));
}

// --- NeoForge / Forge: installer re-run into the existing server dir ---

const NEOFORGE_METADATA_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
const NEOFORGE_INSTALLER_URL: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge/20.4.237/neoforge-20.4.237-installer.jar";
const FORGE_PROMOTIONS_URL: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const FORGE_INSTALLER_URL: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.5/forge-1.20.1-47.4.5-installer.jar";

/// 30s, not 10s: the identical spin-wait in `provisioning_install_step.rs`
/// found 10s wasn't generous enough under heavy concurrent nextest load
/// on GitHub's hosted CI runners (P7.29) -- a thread-scheduling false
/// failure, not a real hang.
fn wait_for_first_spawn(
    supervisor: &FakeProcessSupervisor,
) -> msc_infrastructure::process::ProcessId {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        if let Some((pid, _)) = supervisor.spawned_requests().into_iter().next() {
            return pid;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "no process was spawned within the deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn drive_fake_installer_to_success(supervisor: &FakeProcessSupervisor) {
    let pid = wait_for_first_spawn(supervisor);
    supervisor.emit_stdout(pid, b"Installing...\n").unwrap();
    supervisor.exit_normally(pid).unwrap();
}

#[test]
fn server_version_change_neoforge_re_runs_installer_into_existing_server_dir() {
    let tmp = TempDir::new("neoforge-change");
    let server_dir = tmp.path().join("modded_server");
    fs::create_dir_all(&server_dir).unwrap();
    let transport = FakeTransport::new()
        .with_bytes(
            NEOFORGE_METADATA_URL,
            br#"<metadata><versioning><versions><version>20.4.237</version></versions></versioning></metadata>"#.to_vec(),
        )
        .with_bytes(NEOFORGE_INSTALLER_URL, b"FAKE-NEOFORGE-INSTALLER".to_vec());
    let supervisor = FakeProcessSupervisor::new();
    let mut request = base_request(JavaServerFlavor::NeoForge, &server_dir);
    request.version_id = LATEST;

    let changed = std::thread::scope(|scope| {
        let args_dir = server_dir.join("libraries/net/neoforged/neoforge/20.4.237");
        let supervisor_ref = &supervisor;
        scope.spawn(move || {
            fs::create_dir_all(&args_dir).unwrap();
            fs::write(
                args_dir.join("unix_args.txt"),
                b"@user_jvm_args.txt\nnogui\n",
            )
            .unwrap();
            drive_fake_installer_to_success(supervisor_ref);
        });

        server_versions::change_version(
            &StdFileSystem,
            &transport,
            &supervisor,
            "java",
            INSTALLER_TIMEOUT,
            &request,
            false,
            false,
            backup_ok,
            &never_cancelled,
            no_output,
        )
    })
    .expect("neoforge change");

    // NeoForge's own major.minor scheme ties to Minecraft differently
    // than a literal read might suggest: `20.4.237` derives Minecraft
    // `1.20.4`, not `1.20.1` (`neoforge_minecraft_version`, ported P7.10).
    assert_eq!(changed.minecraft_version, "1.20.4");
    assert_eq!(changed.loader_version.as_deref(), Some("20.4.237"));
    // Installer jar and log cleaned up afterward (source line 129-131).
    assert!(!server_dir.join("neoforge-installer.jar").exists());
}

#[test]
fn server_version_change_forge_pinned_end_to_end() {
    let tmp = TempDir::new("forge-change-pinned");
    let server_dir = tmp.path().join("modded_server");
    fs::create_dir_all(&server_dir).unwrap();
    let transport = FakeTransport::new().with_bytes(
        "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.5.0/forge-1.20.1-47.5.0-installer.jar",
        b"FAKE-FORGE-INSTALLER".to_vec(),
    );
    let supervisor = FakeProcessSupervisor::new();
    let mut request = base_request(JavaServerFlavor::Forge, &server_dir);
    request.version_id = "1.20.1";
    request.loader_version = Some("47.5.0");

    let changed = std::thread::scope(|scope| {
        // `findArgsFile`'s marker is always `unix_args.txt`, regardless
        // of host OS at this layer (`java_launch::find_forge_args_file`),
        // under `libraries/net/minecraftforge/forge/<mc>-<forge>/`.
        let args_dir = server_dir.join("libraries/net/minecraftforge/forge/1.20.1-47.5.0");
        let supervisor_ref = &supervisor;
        scope.spawn(move || {
            fs::create_dir_all(&args_dir).unwrap();
            fs::write(
                args_dir.join("unix_args.txt"),
                b"@user_jvm_args.txt\nnogui\n",
            )
            .unwrap();
            drive_fake_installer_to_success(supervisor_ref);
        });

        server_versions::change_version(
            &StdFileSystem,
            &transport,
            &supervisor,
            "java",
            INSTALLER_TIMEOUT,
            &request,
            false,
            false,
            backup_ok,
            &never_cancelled,
            no_output,
        )
    })
    .expect("forge pinned change");

    assert_eq!(changed.minecraft_version, "1.20.1");
    assert_eq!(changed.loader_version.as_deref(), Some("47.5.0"));
    assert!(!server_dir.join("forge-installer.jar").exists());
}

#[test]
fn server_version_change_forge_latest_when_pin_incomplete() {
    // Pinning requires BOTH a loader version AND a non-"__latest__"
    // versionId with a non-empty MC component -- a loader given without
    // a real versionId falls through to "latest" (source line 487-488).
    let tmp = TempDir::new("forge-change-incomplete-pin");
    let server_dir = tmp.path().join("modded_server");
    fs::create_dir_all(&server_dir).unwrap();
    let transport = FakeTransport::new()
        .with_bytes(
            FORGE_PROMOTIONS_URL,
            br#"{"promos":{"1.20.1-recommended":"47.4.5"}}"#.to_vec(),
        )
        .with_bytes(FORGE_INSTALLER_URL, b"FAKE-FORGE-INSTALLER".to_vec());
    let supervisor = FakeProcessSupervisor::new();
    let mut request = base_request(JavaServerFlavor::Forge, &server_dir);
    request.version_id = LATEST;
    request.loader_version = Some("47.5.0"); // ignored: versionId is "__latest__"

    let changed = std::thread::scope(|scope| {
        let args_dir = server_dir.join("libraries/net/minecraftforge/forge/1.20.1-47.4.5");
        let supervisor_ref = &supervisor;
        scope.spawn(move || {
            fs::create_dir_all(&args_dir).unwrap();
            fs::write(
                args_dir.join("unix_args.txt"),
                b"@user_jvm_args.txt\nnogui\n",
            )
            .unwrap();
            drive_fake_installer_to_success(supervisor_ref);
        });

        server_versions::change_version(
            &StdFileSystem,
            &transport,
            &supervisor,
            "java",
            INSTALLER_TIMEOUT,
            &request,
            false,
            false,
            backup_ok,
            &never_cancelled,
            no_output,
        )
    })
    .expect("forge latest change");

    assert_eq!(changed.loader_version.as_deref(), Some("47.4.5"));
}

// --- list_versions_for_server ---

#[test]
fn list_versions_for_server_vanilla_applies_1_20_floor_and_marks_current() {
    let transport = FakeTransport::new().with_file(
        "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json",
        "vanilla/version-manifest-v2.json",
    );
    let entries = server_versions::list_versions_for_server(
        &transport,
        JavaServerFlavor::Vanilla,
        Some("1.20.1"),
    )
    .expect("list versions");

    assert!(
        !entries.iter().any(|e| e.entry.mc_version == "1.19.4"),
        "below-floor version must be dropped"
    );
    assert!(entries.iter().any(|e| e.entry.mc_version == "1.20.1"));
    let current = entries
        .iter()
        .find(|e| e.entry.mc_version == "1.20.1")
        .unwrap();
    assert!(current.is_current);
    let not_current = entries
        .iter()
        .find(|e| e.entry.mc_version == "1.20")
        .unwrap();
    assert!(!not_current.is_current);
}

#[test]
fn list_versions_for_server_refuses_pufferfish_with_empty_list_not_error() {
    let transport = FakeTransport::new();
    let entries =
        server_versions::list_versions_for_server(&transport, JavaServerFlavor::Pufferfish, None)
            .expect("empty list, not an error");
    assert!(entries.is_empty());
}
