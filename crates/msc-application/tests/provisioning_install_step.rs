//! P7.18: `msc_application::provisioning::create_install_step_server`
//! against the Forge/NeoForge half of `fixtures/server-creation/` (the
//! install-step branch, `install-step-branch-skips-jar-download-runs-
//! installer.json`, plus the shared tail every download-and-go case in
//! `crates/msc-application/tests/provisioning.rs` already exercises).
//!
//! `run_loader_installer`'s own polling/timeout/cancellation machinery
//! was already proven against a **real** `java` subprocess at P7.14
//! (`msc-infrastructure/tests/loader_installer.rs`) — these tests don't
//! re-prove that. They prove this step's own orchestration (version
//! resolution, installer-jar cleanup, the shared creation tail, and
//! directory rollback) using [`FakeProcessSupervisor`], driven from a
//! background thread since [`msc_application::provisioning::
//! create_install_step_server`] blocks synchronously on
//! `run_loader_installer`'s poll loop the same way a real caller would.

use msc_application::provisioning::{
    self, CreateServerError, NewServerRequest, WorldSource, real_copy_existing_world_folder,
    real_unzip_world_backup,
};
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
            "msc2-provisioning-install-step-test-{label}-{}-{}",
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

struct FakeTransport {
    responses: Mutex<HashMap<String, Vec<u8>>>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
        }
    }

    fn with(self, url: &str, bytes: impl Into<Vec<u8>>) -> Self {
        self.responses
            .lock()
            .unwrap()
            .insert(url.to_string(), bytes.into());
        self
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

const NEOFORGE_METADATA_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
const NEOFORGE_INSTALLER_URL: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge/20.4.237/neoforge-20.4.237-installer.jar";
const FORGE_PROMOTIONS_URL: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const FORGE_INSTALLER_URL: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.5/forge-1.20.1-47.4.5-installer.jar";

fn neoforge_transport() -> FakeTransport {
    FakeTransport::new()
        .with(
            NEOFORGE_METADATA_URL,
            br#"<metadata><versioning><versions><version>20.4.237</version></versions></versioning></metadata>"#.to_vec(),
        )
        .with(NEOFORGE_INSTALLER_URL, b"FAKE-NEOFORGE-INSTALLER".to_vec())
}

fn forge_transport() -> FakeTransport {
    FakeTransport::new()
        .with(
            FORGE_PROMOTIONS_URL,
            br#"{"promos":{"1.20.1-recommended":"47.4.5"}}"#.to_vec(),
        )
        .with(FORGE_INSTALLER_URL, b"FAKE-FORGE-INSTALLER".to_vec())
}

fn base_request<'a>(
    flavor: JavaServerFlavor,
    world_source: WorldSource<'a>,
) -> NewServerRequest<'a> {
    NewServerRequest {
        name: "Modded Server",
        initial_world_name: None,
        flavor,
        port: 25565,
        enable_cross_play: false,
        cross_play_bedrock_port: None,
        enable_playit: false,
        enable_xbox_broadcast: false,
        difficulty: "normal",
        gamemode: "survival",
        world_seed: None,
        world_source,
        save_downloaded_jars: false,
        default_banner_color_hex: "#3366FF",
    }
}

fn always_ok2(_a: &Path, _b: &Path) -> bool {
    true
}

fn always_ok3(_a: &Path, _b: &Path, _c: &str) -> bool {
    true
}

fn never_cancelled() -> bool {
    false
}

fn no_output(_stream: OutputStream, _bytes: &[u8]) {}

/// Bounds every spin-wait in this file: generous for a fake, in-memory
/// call, but finite — so a regression upstream (e.g. `spawn` never
/// called at all) fails this test loudly instead of hanging the whole
/// suite, the way an earlier, unbounded version of these loops once did.
/// 10s wasn't actually generous enough: P7.29's CI runs found this file's
/// own background `scope.spawn` thread repeatedly failed to get scheduled
/// within 10s under heavy concurrent nextest load on GitHub's hosted
/// runners (macOS reliably, Windows occasionally) — a real thread-
/// scheduling-starvation false failure, not a regression. 30s.
const SPIN_WAIT_DEADLINE: Duration = Duration::from_secs(30);

/// Waits (spinning briefly — this is a test double, not real I/O) for
/// `run_loader_installer`'s own `spawn` call to register.
fn wait_for_first_spawn(
    supervisor: &FakeProcessSupervisor,
) -> msc_infrastructure::process::ProcessId {
    let deadline = std::time::Instant::now() + SPIN_WAIT_DEADLINE;
    loop {
        if let Some((pid, _)) = supervisor.spawned_requests().into_iter().next() {
            return pid;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "no process was spawned within {SPIN_WAIT_DEADLINE:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// `after_spawn` runs once the fake installer has actually been spawned
/// (i.e. once `create_install_step_server` has already created `new_dir`
/// and invoked the installer) -- every caller uses it to write the args
/// file the installer would have produced. This ordering matters: writing
/// that file (which lives under `new_dir`) any earlier races
/// `create_install_step_server`'s own `new_dir`-already-exists check
/// (`provisioning.rs`, checked before anything else) against this
/// background thread's own filesystem write, and can lose that race under
/// heavy CI load -- found by P7.29's own CI runs, which had been
/// misdiagnosed as a too-short spin-wait deadline until the "no process
/// was spawned" panic was read closely enough to see it was reporting a
/// *symptom* (the main thread had already failed with
/// `FolderAlreadyExists` and so never spawned anything) rather than the
/// cause.
fn drive_fake_installer_to_success(supervisor: &FakeProcessSupervisor, after_spawn: impl FnOnce()) {
    let pid = wait_for_first_spawn(supervisor);
    after_spawn();
    supervisor.emit_stdout(pid, b"Installing...\n").unwrap();
    supervisor.exit_normally(pid).unwrap();
}

fn drive_fake_installer_to_crash(supervisor: &FakeProcessSupervisor, code: i32) {
    let pid = wait_for_first_spawn(supervisor);
    supervisor.crash(pid, code).unwrap();
}

// ---------------------------------------------------------------------
// fixtures/server-creation/install-step-branch-skips-jar-download-runs-installer.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_install_step_neoforge_end_to_end() {
    let tmp = TempDir::new("neoforge-success");
    let transport = neoforge_transport();
    let supervisor = FakeProcessSupervisor::new();
    let request = base_request(JavaServerFlavor::NeoForge, WorldSource::Fresh);

    let created = std::thread::scope(|scope| {
        let server_dir = tmp.path().join("java").join("modded_server");
        let args_dir = server_dir.join("libraries/net/neoforged/neoforge/20.4.237");
        let supervisor_ref = &supervisor;
        scope.spawn(move || {
            drive_fake_installer_to_success(supervisor_ref, || {
                // Real disk write so `run_loader_installer`'s post-exit
                // scan (a real `StdFileSystem`, same as the whole call
                // below) finds it once the fake installer "exits."
                fs::create_dir_all(&args_dir).unwrap();
                fs::write(
                    args_dir.join("unix_args.txt"),
                    b"@user_jvm_args.txt\nnogui\n",
                )
                .unwrap();
            });
        });

        provisioning::create_install_step_server(
            &StdFileSystem,
            &transport,
            &supervisor,
            tmp.path(),
            tmp.path(),
            &tmp.path().join("templates/plugin"),
            &request,
            "/usr/bin/java",
            Duration::from_secs(5),
            "2026-08-18T00:00:00Z",
            &never_cancelled,
            no_output,
            always_ok2,
            always_ok3,
        )
        .unwrap()
    });

    assert_eq!(created.config.paper_jar_path, "");
    assert_eq!(created.config.minecraft_version.as_deref(), Some("1.20.4"));
    assert_eq!(created.config.loader_version.as_deref(), Some("20.4.237"));
    assert_eq!(created.config.server_build.as_deref(), Some("20.4.237"));
    assert_eq!(created.config.java_flavor, JavaServerFlavor::NeoForge);
    assert!(created.should_record_loader_version);

    // "mods" add-on folder (modded category), not "plugins".
    let server_dir = PathBuf::from(&created.config.server_dir);
    assert!(server_dir.join("mods").is_dir());

    // Installer jar and log tidied up (source's own NeoForge-only
    // installer.log removal, P7.5's flagged asymmetry vs. Forge).
    assert!(!server_dir.join("neoforge-installer.jar").exists());
}

#[test]
fn provisioning_install_step_forge_end_to_end() {
    let tmp = TempDir::new("forge-success");
    let transport = forge_transport();
    let supervisor = FakeProcessSupervisor::new();
    let request = base_request(JavaServerFlavor::Forge, WorldSource::Fresh);

    let created = std::thread::scope(|scope| {
        let server_dir = tmp.path().join("java").join("modded_server");
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

        provisioning::create_install_step_server(
            &StdFileSystem,
            &transport,
            &supervisor,
            tmp.path(),
            tmp.path(),
            &tmp.path().join("templates/plugin"),
            &request,
            "/usr/bin/java",
            Duration::from_secs(5),
            "2026-08-18T00:00:00Z",
            &never_cancelled,
            no_output,
            always_ok2,
            always_ok3,
        )
        .unwrap()
    });

    assert_eq!(created.config.paper_jar_path, "");
    assert_eq!(created.config.minecraft_version.as_deref(), Some("1.20.1"));
    assert_eq!(created.config.loader_version.as_deref(), Some("47.4.5"));
    assert_eq!(created.config.server_build.as_deref(), Some("47.4.5"));
    assert_eq!(created.config.java_flavor, JavaServerFlavor::Forge);

    let server_dir = PathBuf::from(&created.config.server_dir);
    assert!(!server_dir.join("forge-installer.jar").exists());
}

// ---------------------------------------------------------------------
// Non-zero installer exit rolls the whole directory back — a Forge/
// NeoForge install writes a large `libraries/` tree, so a partial one
// is both large and unusable.
// ---------------------------------------------------------------------

#[test]
fn provisioning_install_step_non_zero_exit_rolls_back_directory() {
    let tmp = TempDir::new("neoforge-crash");
    let transport = neoforge_transport();
    let supervisor = FakeProcessSupervisor::new();
    let request = base_request(JavaServerFlavor::NeoForge, WorldSource::Fresh);

    let err = std::thread::scope(|scope| {
        scope.spawn(|| drive_fake_installer_to_crash(&supervisor, 1));

        provisioning::create_install_step_server(
            &StdFileSystem,
            &transport,
            &supervisor,
            tmp.path(),
            tmp.path(),
            &tmp.path().join("templates/plugin"),
            &request,
            "/usr/bin/java",
            Duration::from_secs(5),
            "2026-08-18T00:00:00Z",
            &never_cancelled,
            no_output,
            always_ok2,
            always_ok3,
        )
        .unwrap_err()
    });

    assert!(matches!(err, CreateServerError::LoaderInstaller(_)));
    assert!(!tmp.path().join("java").join("modded_server").exists());
}

// ---------------------------------------------------------------------
// Cancellation before the installer even starts — nothing long-running
// touched yet, matching `world_conversion::convert_world`'s own two-
// boundary cancellation shape.
// ---------------------------------------------------------------------

#[test]
fn provisioning_install_step_cancelled_before_installer_starts() {
    let tmp = TempDir::new("neoforge-cancel");
    let transport = neoforge_transport();
    let supervisor = FakeProcessSupervisor::new();
    let request = base_request(JavaServerFlavor::NeoForge, WorldSource::Fresh);

    fn always_cancelled() -> bool {
        true
    }

    let err = provisioning::create_install_step_server(
        &StdFileSystem,
        &transport,
        &supervisor,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/plugin"),
        &request,
        "/usr/bin/java",
        Duration::from_secs(5),
        "2026-08-18T00:00:00Z",
        &always_cancelled,
        no_output,
        always_ok2,
        always_ok3,
    )
    .unwrap_err();

    assert!(matches!(err, CreateServerError::Cancelled));
    assert!(!tmp.path().join("java").exists());
    // Never reached the network at all.
    assert!(supervisor.spawned_requests().is_empty());
}

// ---------------------------------------------------------------------
// A download-and-go flavor is refused by this function, and an
// install-step flavor is refused by `create_download_and_go_server` —
// each function only provisions its own half.
// ---------------------------------------------------------------------

#[test]
fn provisioning_install_step_refuses_download_and_go_flavor() {
    let tmp = TempDir::new("wrong-flavor");
    let transport = FakeTransport::new();
    let supervisor = FakeProcessSupervisor::new();
    let request = base_request(JavaServerFlavor::Paper, WorldSource::Fresh);

    let err = provisioning::create_install_step_server(
        &StdFileSystem,
        &transport,
        &supervisor,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/plugin"),
        &request,
        "/usr/bin/java",
        Duration::from_secs(5),
        "2026-08-18T00:00:00Z",
        &never_cancelled,
        no_output,
        always_ok2,
        always_ok3,
    )
    .unwrap_err();

    assert!(matches!(
        err,
        CreateServerError::UnsupportedFlavor(JavaServerFlavor::Paper)
    ));
}

// ---------------------------------------------------------------------
// The world-source dispatch, initial world slot, and cross-play skip
// (mods, not plugins) all reuse `finish_server_creation` — reusing
// `real_unzip_world_backup`/`real_copy_existing_world_folder` here
// proves the shared tail composes for real, not just against always-ok
// stub closures.
// ---------------------------------------------------------------------

#[test]
fn provisioning_install_step_fresh_world_slot_created() {
    let tmp = TempDir::new("neoforge-world-slot");
    let transport = neoforge_transport();
    let supervisor = FakeProcessSupervisor::new();
    let request = base_request(JavaServerFlavor::NeoForge, WorldSource::Fresh);

    let created = std::thread::scope(|scope| {
        let server_dir = tmp.path().join("java").join("modded_server");
        let args_dir = server_dir.join("libraries/net/neoforged/neoforge/20.4.237");
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

        provisioning::create_install_step_server(
            &StdFileSystem,
            &transport,
            &supervisor,
            tmp.path(),
            tmp.path(),
            &tmp.path().join("templates/plugin"),
            &request,
            "/usr/bin/java",
            Duration::from_secs(5),
            "2026-08-18T00:00:00Z",
            &never_cancelled,
            no_output,
            real_unzip_world_backup,
            real_copy_existing_world_folder,
        )
        .unwrap()
    });

    assert_eq!(created.world_slot.name, "Modded Server");
    assert!(created.world_slot.world_level_name.is_some());
    let server_dir = PathBuf::from(&created.config.server_dir);
    assert!(
        server_dir
            .join("world_slots")
            .join(&created.world_slot.id)
            .join("slot.json")
            .is_file()
    );
}
