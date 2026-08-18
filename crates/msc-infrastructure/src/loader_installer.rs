//! Runs a Forge or NeoForge loader installer JAR as a supervised
//! subprocess through [`crate::process::ProcessSupervisor`] -- the second
//! boundary where MSC 2 hands control to a third party, after P7.13's
//! network fetch. P7.13 only downloads the installer jar; this module is
//! what actually runs `java -jar <installer> --installServer`
//! (`NeoForgeInstaller.swift`'s/`ForgeInstaller.swift`'s shared
//! `runJavaInstaller`, both installers' `install` after the download step).
//!
//! [`ProcessSupervisor`] is fully synchronous (`spawn` returns immediately,
//! `drain_events` polls), so [`run_loader_installer`] blocks the calling
//! thread, polling for output/exit at [`POLL_INTERVAL`] and checking a
//! caller-supplied `cancelled` predicate on every poll -- the same
//! "cooperative cancellation" shape a caller elsewhere in this codebase
//! would run on a `spawn_blocking` thread, which is not this step's job.
//! A timeout or a positive `cancelled` check both `force_terminate` the
//! process (killing the tree, not just the JVM's own pid, on every
//! platform supervisor this trait has) rather than leaving it orphaned.
//!
//! After a zero exit, the args file the installer was supposed to produce
//! is looked up with P7.11's pure selector
//! (`msc_domain::launch_shape::neoforge_select_args_file`/
//! `forge_select_args_file`) fed by a directory scan here -- the same
//! domain/I/O split `crates/msc-application/src/java_launch.rs`'s own
//! `find_neoforge_args_file`/`find_forge_args_file` already use against
//! the *launch*-time question "which installed version do we launch". This
//! module answers the narrower *install*-time question "did the installer
//! that just exited zero actually produce one" and cannot reuse that
//! application-layer code (`msc-application` depends on
//! `msc-infrastructure`, not the reverse), so the same small scan is
//! reimplemented here against this crate's own [`FileSystem`] trait rather
//! than promoted to a shared location this step wasn't asked to create.

use std::fmt;
use std::path::Path;
use std::time::{Duration, Instant};

use msc_domain::launch_shape;

use crate::fs::FileSystem;
use crate::process::{
    OutputStream, ProcessError, ProcessEvent, ProcessId, ProcessSpawnRequest, ProcessSupervisor,
};

/// How often the wait loop polls for new output/exit and re-checks the
/// timeout deadline and the `cancelled` predicate. Real installers run for
/// tens of seconds; this keeps progress/cancellation responsive without
/// busy-looping.
pub const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Trailing bytes of combined stdout+stderr kept for an error message --
/// enough to carry the real cause (an installer's own failure line, a
/// stack trace tail) without unbounded growth against a chatty installer.
pub const OUTPUT_TAIL_BYTES: usize = 4096;

/// Which family's installer is running -- determines both the `@<args-file>
/// nogui` directory this looks under afterwards and, via `target`, which
/// selection rule applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderFamily {
    Forge,
    NeoForge,
}

impl LoaderFamily {
    pub fn display_name(self) -> &'static str {
        match self {
            LoaderFamily::Forge => "Forge",
            LoaderFamily::NeoForge => "NeoForge",
        }
    }
}

/// Which installed version to prefer when more than one could satisfy the
/// post-install args-file lookup -- mirrors
/// `launch_shape::neoforge_select_args_file`'s/`forge_select_args_file`'s
/// own two shapes exactly (NeoForge selects on one version, Forge on an
/// `{mc}-{forge}` pair), since a fresh install directory ordinarily
/// contains exactly one candidate and `None` picks whichever one that is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoaderTarget {
    NeoForge {
        specific_version: Option<String>,
    },
    Forge {
        mc_version: Option<String>,
        forge_version: Option<String>,
    },
}

impl LoaderTarget {
    fn family(&self) -> LoaderFamily {
        match self {
            LoaderTarget::NeoForge { .. } => LoaderFamily::NeoForge,
            LoaderTarget::Forge { .. } => LoaderFamily::Forge,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderInstallRequest {
    pub java_executable_path: String,
    /// The installer jar's filename, relative to `server_dir` -- P7.13
    /// already staged it there.
    pub installer_jar_name: String,
    pub server_dir: std::path::PathBuf,
    pub timeout: Duration,
    pub target: LoaderTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoaderInstallOutcome {
    /// The args file's path, relative to `server_dir`, e.g.
    /// `libraries/net/neoforged/neoforge/20.4.237/unix_args.txt`.
    pub args_file: String,
    pub output_tail: String,
}

#[derive(Debug)]
pub enum LoaderInstallerError {
    Spawn(String),
    Timeout {
        tail: String,
    },
    Cancelled {
        tail: String,
    },
    NonZeroExit {
        code: Option<i32>,
        tail: String,
    },
    /// The installer exited zero but no args file matching `target` was
    /// found afterwards -- the installer claimed success but didn't
    /// produce the one thing this whole run exists to get.
    ArgsFileNotProduced {
        family: LoaderFamily,
    },
    Process(ProcessError),
}

impl fmt::Display for LoaderInstallerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoaderInstallerError::Spawn(message) => write!(f, "{message}"),
            LoaderInstallerError::Timeout { tail } => {
                write!(f, "installer timed out. Last output:\n{tail}")
            }
            LoaderInstallerError::Cancelled { tail } => {
                write!(f, "installer was cancelled. Last output:\n{tail}")
            }
            LoaderInstallerError::NonZeroExit { code, tail } => write!(
                f,
                "installer exited with code {}. Last output:\n{tail}",
                code.map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ),
            LoaderInstallerError::ArgsFileNotProduced { family } => write!(
                f,
                "{} installer reported success but produced no args file.",
                family.display_name()
            ),
            LoaderInstallerError::Process(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for LoaderInstallerError {}

struct TailBuffer {
    max_bytes: usize,
    buf: Vec<u8>,
}

impl TailBuffer {
    fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            buf: Vec::new(),
        }
    }

    fn push(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
        if self.buf.len() > self.max_bytes {
            let excess = self.buf.len() - self.max_bytes;
            self.buf.drain(0..excess);
        }
    }

    fn as_string(&self) -> String {
        String::from_utf8_lossy(&self.buf).into_owned()
    }
}

/// Runs the installer named by `request.installer_jar_name` (already
/// staged into `request.server_dir` by P7.13) as
/// `java -jar <installer> --installServer`, working directory pinned to
/// `server_dir` per `runJavaInstaller`'s own invocation. `on_output` is
/// called for every chunk of stdout/stderr as it arrives -- the "surfaced
/// as operation progress rather than swallowed" requirement; what a caller
/// does with those chunks (write them into an operation journal, drop
/// them) is not this function's concern. `cancelled` is polled on the same
/// cadence as the timeout deadline; both trigger `force_terminate` rather
/// than leaving the process (and, for a real supervisor, its process
/// group) running.
pub fn run_loader_installer(
    supervisor: &dyn ProcessSupervisor,
    fs: &dyn FileSystem,
    request: &LoaderInstallRequest,
    cancelled: &dyn Fn() -> bool,
    mut on_output: impl FnMut(OutputStream, &[u8]),
) -> Result<LoaderInstallOutcome, LoaderInstallerError> {
    let spawn_request = ProcessSpawnRequest::new(
        request.java_executable_path.clone(),
        request.server_dir.clone(),
    )
    .args([
        "-jar",
        request.installer_jar_name.as_str(),
        "--installServer",
    ]);

    let pid = supervisor
        .spawn(spawn_request)
        .map_err(|e| LoaderInstallerError::Spawn(e.to_string()))?;

    let mut tail = TailBuffer::new(OUTPUT_TAIL_BYTES);
    let deadline = Instant::now() + request.timeout;

    loop {
        let events = supervisor
            .drain_events(pid)
            .map_err(LoaderInstallerError::Process)?;
        for event in events {
            match event {
                ProcessEvent::Output { stream, bytes } => {
                    tail.push(&bytes);
                    on_output(stream, &bytes);
                }
                ProcessEvent::Exited(status) => {
                    return if status.success() {
                        finish_after_success(fs, request, tail.as_string())
                    } else {
                        Err(LoaderInstallerError::NonZeroExit {
                            code: status.code,
                            tail: tail.as_string(),
                        })
                    };
                }
            }
        }

        if cancelled() {
            let _ = supervisor.force_terminate(pid);
            drain_briefly(supervisor, pid, &mut tail, &mut on_output);
            return Err(LoaderInstallerError::Cancelled {
                tail: tail.as_string(),
            });
        }

        if Instant::now() >= deadline {
            let _ = supervisor.force_terminate(pid);
            drain_briefly(supervisor, pid, &mut tail, &mut on_output);
            return Err(LoaderInstallerError::Timeout {
                tail: tail.as_string(),
            });
        }

        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Best-effort collection of whatever output arrived in the moment around
/// a `force_terminate` call -- not waited on, since the process is being
/// killed precisely because this function is done waiting on it.
fn drain_briefly(
    supervisor: &dyn ProcessSupervisor,
    pid: ProcessId,
    tail: &mut TailBuffer,
    on_output: &mut impl FnMut(OutputStream, &[u8]),
) {
    if let Ok(events) = supervisor.drain_events(pid) {
        for event in events {
            if let ProcessEvent::Output { stream, bytes } = event {
                tail.push(&bytes);
                on_output(stream, &bytes);
            }
        }
    }
}

fn finish_after_success(
    fs: &dyn FileSystem,
    request: &LoaderInstallRequest,
    tail: String,
) -> Result<LoaderInstallOutcome, LoaderInstallerError> {
    let args_file = match &request.target {
        LoaderTarget::NeoForge { specific_version } => {
            find_neoforge_args_file(fs, &request.server_dir, specific_version.as_deref())
        }
        LoaderTarget::Forge {
            mc_version,
            forge_version,
        } => find_forge_args_file(
            fs,
            &request.server_dir,
            mc_version.as_deref(),
            forge_version.as_deref(),
        ),
    };

    args_file
        .map(|args_file| LoaderInstallOutcome {
            args_file,
            output_tail: tail,
        })
        .ok_or(LoaderInstallerError::ArgsFileNotProduced {
            family: request.target.family(),
        })
}

/// `NeoForgeInstaller.findArgsFile(in:specificVersion:)`'s I/O half: scans
/// `server_dir/libraries/net/neoforged/neoforge/` for version directories
/// containing `unix_args.txt`, then delegates selection to
/// [`launch_shape::neoforge_select_args_file`].
fn find_neoforge_args_file(
    fs: &dyn FileSystem,
    server_dir: &Path,
    specific_version: Option<&str>,
) -> Option<String> {
    let base = server_dir.join("libraries/net/neoforged/neoforge");
    let installed = installed_subdirs_containing(fs, &base, "unix_args.txt");
    launch_shape::neoforge_select_args_file(&installed, specific_version)
}

/// The Forge sibling of [`find_neoforge_args_file`]: scans
/// `server_dir/libraries/net/minecraftforge/forge/` for `{mc}-{forge}`
/// pair directories containing `unix_args.txt`.
fn find_forge_args_file(
    fs: &dyn FileSystem,
    server_dir: &Path,
    mc_version: Option<&str>,
    forge_version: Option<&str>,
) -> Option<String> {
    let base = server_dir.join("libraries/net/minecraftforge/forge");
    let installed = installed_subdirs_containing(fs, &base, "unix_args.txt");
    launch_shape::forge_select_args_file(&installed, mc_version, forge_version)
}

/// Lists `base`'s immediate subdirectories that contain a file named
/// `marker`, returning their basenames. An unreadable/absent `base` (the
/// installer failed before creating anything under `libraries/`) yields an
/// empty list, not an error.
fn installed_subdirs_containing(fs: &dyn FileSystem, base: &Path, marker: &str) -> Vec<String> {
    let Ok(entries) = fs.list(base) else {
        return Vec::new();
    };
    entries
        .into_iter()
        .filter(|entry| {
            fs.stat(&entry.join(marker))
                .map(|m| m.is_file)
                .unwrap_or(false)
        })
        .filter_map(|entry| entry.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect()
}
