//! Exercises [`msc_infrastructure::loader_installer::run_loader_installer`]
//! against a real, small Java process -- not `FakeProcessSupervisor`, which
//! is driven by hand (`emit_stdout`/`exit_normally`) and so can't prove
//! this module's own polling, timeout, or cancellation code actually works
//! against a real subprocess. This is the same "test this crate's own
//! bounding code against something real and local" shape P7.13's
//! `jar_provider_http_transport_enforces_size_cap_and_timeout` used against
//! a real loopback server instead of a real external provider.
//!
//! `RealTestProcessSupervisor` below is a from-scratch, minimal
//! `ProcessSupervisor` built only for this test file: the real per-platform
//! supervisors (`msc-platform-macos`/`-linux`/`-windows`) depend on
//! `msc-infrastructure`, so this crate cannot depend back on them without a
//! cycle. It has none of those crates' process-group/signal-tree handling
//! (P7.14's own "What" line notes the kill-the-tree requirement belongs to
//! a real platform supervisor, which `run_loader_installer` only *calls
//! through* the trait) -- it exists only to prove `run_loader_installer`
//! itself polls, times out, and cancels correctly against a real `Child`.
//!
//! The fake installer JAR is compiled once with `javac`/`jar`, the same
//! technique `tools/phase6/phase6-gate-smoke.sh` already uses to avoid
//! committing a binary fixture. `MSC_TEST_MODE` selects its behavior so one
//! small `FakeInstaller.java` covers every case this step's own "What" line
//! asks for (success, non-zero exit, no-args-file-produced, timeout,
//! cancellation) instead of five near-duplicate Java sources.

use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::loader_installer::{
    LoaderFamily, LoaderInstallRequest, LoaderInstallerError, LoaderTarget, run_loader_installer,
};
use msc_infrastructure::process::{
    OutputStream, ProcessError, ProcessEvent, ProcessExitStatus, ProcessId, ProcessSpawnRequest,
    ProcessSupervisor,
};

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

// ---------------------------------------------------------------------
// A real, minimal ProcessSupervisor for these tests only.
// ---------------------------------------------------------------------

struct RealTestProcessSupervisor {
    next_id: AtomicU32,
    processes: Mutex<BTreeMap<ProcessId, RealTestProcess>>,
}

struct RealTestProcess {
    child: Mutex<Child>,
    events: Arc<Mutex<Vec<ProcessEvent>>>,
    exit_pushed: AtomicBool,
}

impl RealTestProcessSupervisor {
    fn new() -> Self {
        Self {
            next_id: AtomicU32::new(9000),
            processes: Mutex::new(BTreeMap::new()),
        }
    }
}

fn spawn_reader(
    mut stream: impl Read + Send + 'static,
    stream_kind: OutputStream,
    events: Arc<Mutex<Vec<ProcessEvent>>>,
) {
    thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => events.lock().unwrap().push(ProcessEvent::Output {
                    stream: stream_kind,
                    bytes: buffer[..n].to_vec(),
                }),
                Err(_) => break,
            }
        }
    });
}

impl ProcessSupervisor for RealTestProcessSupervisor {
    fn spawn(&self, request: ProcessSpawnRequest) -> Result<ProcessId, ProcessError> {
        let mut command = Command::new(&request.executable_path);
        command
            .args(&request.arguments)
            .current_dir(&request.working_directory)
            .envs(
                request
                    .environment
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str())),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|e| ProcessError::Spawn(format!("spawning test process: {e}")))?;

        let events: Arc<Mutex<Vec<ProcessEvent>>> = Arc::new(Mutex::new(Vec::new()));
        if let Some(stream) = child.stdout.take() {
            spawn_reader(stream, OutputStream::Stdout, Arc::clone(&events));
        }
        if let Some(stream) = child.stderr.take() {
            spawn_reader(stream, OutputStream::Stderr, Arc::clone(&events));
        }

        let pid = ProcessId::new(self.next_id.fetch_add(1, Ordering::SeqCst));
        self.processes.lock().unwrap().insert(
            pid,
            RealTestProcess {
                child: Mutex::new(child),
                events,
                exit_pushed: AtomicBool::new(false),
            },
        );
        Ok(pid)
    }

    fn write_stdin(&self, _pid: ProcessId, _bytes: &[u8]) -> Result<(), ProcessError> {
        Ok(())
    }

    fn force_terminate(&self, pid: ProcessId) -> Result<(), ProcessError> {
        let processes = self.processes.lock().unwrap();
        let process = processes.get(&pid).ok_or(ProcessError::NotFound(pid))?;
        let _ = process.child.lock().unwrap().kill();
        Ok(())
    }

    fn drain_events(&self, pid: ProcessId) -> Result<Vec<ProcessEvent>, ProcessError> {
        let processes = self.processes.lock().unwrap();
        let process = processes.get(&pid).ok_or(ProcessError::NotFound(pid))?;

        if !process.exit_pushed.load(Ordering::SeqCst)
            && let Ok(Some(status)) = process.child.lock().unwrap().try_wait()
        {
            process.exit_pushed.store(true, Ordering::SeqCst);
            process
                .events
                .lock()
                .unwrap()
                .push(ProcessEvent::Exited(ProcessExitStatus {
                    code: status.code(),
                    signal: None,
                }));
        }

        Ok(std::mem::take(&mut process.events.lock().unwrap()))
    }
}

// ---------------------------------------------------------------------
// The fake installer JAR, compiled once and reused by every test.
// ---------------------------------------------------------------------

const FAKE_INSTALLER_SOURCE: &str = r#"
import java.nio.file.*;

public class FakeInstaller {
    public static void main(String[] args) throws Exception {
        String mode = System.getenv("MSC_TEST_MODE");
        if (mode == null) mode = "success";
        System.out.println("[FakeInstaller] starting mode=" + mode);

        switch (mode) {
            case "fail":
                System.err.println("installer failed: could not find a supported Minecraft install");
                System.exit(1);
                break;
            case "no-args-file":
                System.out.println("[FakeInstaller] done, but not writing an args file");
                System.exit(0);
                break;
            case "sleep":
                for (int i = 0; i < 300; i++) {
                    Files.write(Paths.get("heartbeat.txt"), Integer.toString(i).getBytes());
                    Thread.sleep(100);
                }
                System.exit(0);
                break;
            default:
                Path dir = Paths.get("libraries/net/neoforged/neoforge/20.4.237");
                Files.createDirectories(dir);
                Files.write(dir.resolve("unix_args.txt"), "@user_jvm_args.txt\nnogui\n".getBytes());
                System.out.println("[FakeInstaller] wrote args file");
                System.exit(0);
        }
    }
}
"#;

fn fake_installer_jar() -> &'static Path {
    static JAR: OnceLock<PathBuf> = OnceLock::new();
    JAR.get_or_init(|| {
        let build_dir = std::env::temp_dir().join(format!(
            "msc2-loader-installer-test-build-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&build_dir);
        std::fs::create_dir_all(&build_dir).unwrap();

        std::fs::write(build_dir.join("FakeInstaller.java"), FAKE_INSTALLER_SOURCE).unwrap();
        let javac = Command::new("javac")
            .arg("FakeInstaller.java")
            .current_dir(&build_dir)
            .status()
            .expect("javac must be on PATH -- CI installs a JDK for exactly this reason");
        assert!(
            javac.success(),
            "javac failed to compile FakeInstaller.java"
        );

        std::fs::write(
            build_dir.join("manifest.txt"),
            "Main-Class: FakeInstaller\n",
        )
        .unwrap();
        let jar = Command::new("jar")
            .args([
                "cfm",
                "fake-installer.jar",
                "manifest.txt",
                "FakeInstaller.class",
            ])
            .current_dir(&build_dir)
            .status()
            .expect("jar must be on PATH alongside javac");
        assert!(jar.success(), "jar failed to package fake-installer.jar");

        build_dir.join("fake-installer.jar")
    })
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-loader-installer-test-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn base_request(server_dir: &Path, timeout: Duration) -> LoaderInstallRequest {
    let jar = fake_installer_jar();
    std::fs::copy(jar, server_dir.join("installer.jar")).unwrap();
    LoaderInstallRequest {
        java_executable_path: "java".to_string(),
        installer_jar_name: "installer.jar".to_string(),
        server_dir: server_dir.to_path_buf(),
        timeout,
        target: LoaderTarget::NeoForge {
            specific_version: None,
        },
    }
}

fn run(
    server_dir: &Path,
    mode: &str,
    timeout: Duration,
    cancelled: &dyn Fn() -> bool,
) -> Result<msc_infrastructure::loader_installer::LoaderInstallOutcome, LoaderInstallerError> {
    let supervisor = EnvInjectingSupervisor {
        inner: RealTestProcessSupervisor::new(),
        mode: mode.to_string(),
    };
    let fs = StdFileSystem;
    let request = base_request(server_dir, timeout);
    run_loader_installer(&supervisor, &fs, &request, cancelled, |_stream, _bytes| {})
}

/// Wraps [`RealTestProcessSupervisor`] to inject `MSC_TEST_MODE` into every
/// spawn -- `run_loader_installer` builds its own `ProcessSpawnRequest`
/// internally and has no per-test hook, so the mode travels through the
/// supervisor instead.
struct EnvInjectingSupervisor {
    inner: RealTestProcessSupervisor,
    mode: String,
}

impl ProcessSupervisor for EnvInjectingSupervisor {
    fn spawn(&self, request: ProcessSpawnRequest) -> Result<ProcessId, ProcessError> {
        self.inner
            .spawn(request.env("MSC_TEST_MODE", self.mode.clone()))
    }
    fn write_stdin(&self, pid: ProcessId, bytes: &[u8]) -> Result<(), ProcessError> {
        self.inner.write_stdin(pid, bytes)
    }
    fn force_terminate(&self, pid: ProcessId) -> Result<(), ProcessError> {
        self.inner.force_terminate(pid)
    }
    fn drain_events(&self, pid: ProcessId) -> Result<Vec<ProcessEvent>, ProcessError> {
        self.inner.drain_events(pid)
    }
}

#[test]
fn loader_installer_success_discovers_produced_args_file() {
    let tmp = TempDir::new("success");
    let outcome = run(tmp.path(), "success", Duration::from_secs(15), &|| false).unwrap();
    assert_eq!(
        outcome.args_file,
        "libraries/net/neoforged/neoforge/20.4.237/unix_args.txt"
    );
    assert!(outcome.output_tail.contains("wrote args file"));
}

#[test]
fn loader_installer_non_zero_exit_carries_output_tail() {
    let tmp = TempDir::new("fail");
    let err = run(tmp.path(), "fail", Duration::from_secs(15), &|| false).unwrap_err();
    match err {
        LoaderInstallerError::NonZeroExit { code, tail } => {
            assert_eq!(code, Some(1));
            assert!(tail.contains("could not find a supported Minecraft install"));
        }
        other => panic!("expected NonZeroExit, got {other:?}"),
    }
}

#[test]
fn loader_installer_success_exit_with_no_args_file_is_an_error() {
    let tmp = TempDir::new("no-args-file");
    let err = run(tmp.path(), "no-args-file", Duration::from_secs(15), &|| {
        false
    })
    .unwrap_err();
    match err {
        LoaderInstallerError::ArgsFileNotProduced { family } => {
            assert_eq!(family, LoaderFamily::NeoForge);
        }
        other => panic!("expected ArgsFileNotProduced, got {other:?}"),
    }
}

#[test]
fn loader_installer_timeout_kills_the_process() {
    let tmp = TempDir::new("timeout");
    let err = run(tmp.path(), "sleep", Duration::from_millis(300), &|| false).unwrap_err();
    assert!(matches!(err, LoaderInstallerError::Timeout { .. }));

    // Prove the process actually died rather than being left running:
    // its heartbeat file must stop advancing once `run_loader_installer`
    // has returned.
    let heartbeat_path = tmp.path().join("heartbeat.txt");
    let first = std::fs::read_to_string(&heartbeat_path).unwrap_or_default();
    thread::sleep(Duration::from_millis(500));
    let second = std::fs::read_to_string(&heartbeat_path).unwrap_or_default();
    assert_eq!(
        first, second,
        "heartbeat kept advancing after a reported timeout"
    );
}

#[test]
fn loader_installer_cancellation_kills_the_process() {
    let tmp = TempDir::new("cancel");
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_flag_writer = Arc::clone(&cancel_flag);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        cancel_flag_writer.store(true, Ordering::SeqCst);
    });

    let err = run(tmp.path(), "sleep", Duration::from_secs(30), &|| {
        cancel_flag.load(Ordering::SeqCst)
    })
    .unwrap_err();
    assert!(matches!(err, LoaderInstallerError::Cancelled { .. }));

    let heartbeat_path = tmp.path().join("heartbeat.txt");
    let first = std::fs::read_to_string(&heartbeat_path).unwrap_or_default();
    thread::sleep(Duration::from_millis(500));
    let second = std::fs::read_to_string(&heartbeat_path).unwrap_or_default();
    assert_eq!(first, second, "heartbeat kept advancing after cancellation");
}
