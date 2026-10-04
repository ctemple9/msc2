//! Host-local systemd/package cleanup; elevated operations use fixed commands.
use crate::service::LinuxSystemdServiceManager;
use msc_infrastructure::service::{
    ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceName, ServiceState,
    ServiceStatusReport,
};
use msc_infrastructure::uninstall::native::{self, Installation, LocalServices};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

#[derive(Default)]
pub struct LinuxUninstall {
    privileged: Mutex<Option<PrivilegedSession>>,
}

impl LinuxUninstall {
    pub fn new() -> Self {
        Self::default()
    }

    fn privileged(&self, action: &str) -> Result<(), String> {
        let mut session = self.privileged.lock().map_err(|error| error.to_string())?;
        if session.is_none() {
            *session = Some(PrivilegedSession::start()?);
        }
        session.as_mut().expect("session started").send(action)
    }
}

struct PrivilegedSession {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl PrivilegedSession {
    fn start() -> Result<Self, String> {
        let binary = installed_helper()?;
        let graphical =
            std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some();
        let mut command = if graphical {
            Command::new("/usr/bin/pkexec")
        } else {
            Command::new("/usr/bin/sudo")
        };
        let mut child = command
            .arg(binary)
            .arg("uninstall-privileged")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Could not start MSC's uninstall authorization: {error}"))?;
        let input = child
            .stdin
            .take()
            .ok_or("Uninstall helper has no input channel")?;
        let mut output = BufReader::new(
            child
                .stdout
                .take()
                .ok_or("Uninstall helper has no output channel")?,
        );
        let mut greeting = String::new();
        output
            .read_line(&mut greeting)
            .map_err(|error| error.to_string())?;
        if greeting.trim_end() != "READY" {
            let _ = child.wait();
            return Err("MSC's privileged uninstall helper did not start. The installed package must contain this uninstall update.".into());
        }
        Ok(Self {
            child,
            input,
            output,
        })
    }

    fn send(&mut self, action: &str) -> Result<(), String> {
        writeln!(self.input, "{action}").map_err(|error| error.to_string())?;
        self.input.flush().map_err(|error| error.to_string())?;
        let mut answer = String::new();
        self.output
            .read_line(&mut answer)
            .map_err(|error| error.to_string())?;
        if answer.trim_end() == "OK" {
            Ok(())
        } else {
            Err(answer
                .trim_end()
                .strip_prefix("ERROR ")
                .unwrap_or("Privileged uninstall helper stopped unexpectedly.")
                .to_owned())
        }
    }
}

impl Drop for PrivilegedSession {
    fn drop(&mut self) {
        let _ = writeln!(self.input, "QUIT");
        let _ = self.input.flush();
        let _ = self.child.wait();
    }
}

fn installed_helper() -> Result<PathBuf, String> {
    for path in [
        "/usr/lib/MSC 2/agent/msc",
        "/usr/lib/msc2-desktop-web/agent/msc",
        "/usr/lib/msc2/msc",
    ] {
        let path = Path::new(path);
        if let Ok(metadata) = fs::symlink_metadata(path)
            && metadata.is_file()
            && !metadata.file_type().is_symlink()
            && metadata.uid() == 0
            && metadata.mode() & 0o022 == 0
            && fs::canonicalize(path).is_ok_and(|resolved| resolved == path)
            && path
                .parent()
                .and_then(|parent| fs::metadata(parent).ok())
                .is_some_and(|parent| {
                    parent.is_dir() && parent.uid() == 0 && parent.mode() & 0o022 == 0
                })
        {
            return Ok(path.to_path_buf());
        }
    }
    Err("Could not find a protected system-installed MSC uninstall helper.".into())
}
impl LocalServices for LinuxUninstall {
    fn inspect(&self) -> Result<Vec<ServiceStatusReport>, String> {
        let agent = LinuxSystemdServiceManager::new()
            .execute(ServiceManagerCommand::Status {
                service_name: ServiceName::new("com.ctemple.msc2.agent"),
            })
            .map_err(|error| error.to_string())?;
        let helper_present = Path::new("/etc/systemd/system/msc2-credential-helper.service")
            .exists()
            || Path::new("/etc/systemd/system/msc2-credential-helper.socket").exists();
        let helper = ServiceStatusReport {
            service_name: ServiceName::new("msc2-credential-helper"),
            state: if helper_present {
                ServiceState::Stopped
            } else {
                ServiceState::NotInstalled
            },
            pid: None,
            definition: helper_present.then(|| {
                ServiceInstallRequest::new(
                    "msc2-credential-helper",
                    "/usr/lib/msc2/msc",
                    "/var/lib/msc2/credentials",
                    "/var/log/msc2/credentials.log",
                    0,
                )
            }),
        };
        Ok(vec![agent, helper])
    }
    fn remove(&self) -> Result<(), String> {
        self.privileged("STOP")
    }
    fn remove_data(&self, path: &Path) -> Result<(), String> {
        if path == Path::new("/var/lib/msc2") {
            self.privileged("DATA")
        } else {
            native::remove_path(path)
        }
    }
    fn remove_installation(&self, installation: &Installation) -> Result<(), String> {
        match installation {
            Installation::Deb { name } if known_package(name) => {
                self.privileged(&format!("DEB {name}"))
            }
            Installation::Rpm { name } if known_package(name) => {
                self.privileged(&format!("RPM {name}"))
            }
            Installation::LinuxArchive { path }
                if path == Path::new("/usr/lib/msc2")
                    && std::fs::read_to_string(path.join(".msc2-installation-mode"))
                        .is_ok_and(|value| value.trim() == "standalone-archive") =>
            {
                self.privileged("ARCHIVE")
            }
            _ => Err("Unsupported Linux installation identity.".into()),
        }
    }
    fn clear_credentials(&self) -> Result<(), String> {
        Ok(())
    }
}
fn known_package(name: &str) -> bool {
    matches!(name, "msc2" | "msc2-desktop-web" | "msc-2")
}

/// Runs only from a protected installed executable after one OS authorization.
/// The protocol has fixed verbs and never accepts a path from the caller.
pub fn run_privileged_uninstall() -> Result<(), String> {
    if unsafe { libc::geteuid() } != 0 {
        return Err("Privileged uninstall requires OS authorization.".into());
    }
    let binary = fs::canonicalize(std::env::current_exe().map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    if installed_helper()? != binary {
        return Err("Privileged uninstall must run from the protected installed MSC agent.".into());
    }
    let mut output = std::io::stdout().lock();
    writeln!(output, "READY").map_err(|error| error.to_string())?;
    output.flush().map_err(|error| error.to_string())?;
    let input = std::io::stdin().lock();
    let mut stopped = false;
    for line in input.lines() {
        let line = line.map_err(|error| error.to_string())?;
        if line == "QUIT" {
            break;
        }
        let result = if line == "STOP" {
            let result = stop_fixed_services();
            if result.is_ok() {
                stopped = true;
            }
            result
        } else if !stopped {
            Err("MSC services must stop before privileged cleanup.".into())
        } else if line == "DATA" {
            remove_system_data()
        } else if line == "ARCHIVE" {
            remove_marked_archive()
        } else if let Some(name) = line.strip_prefix("RPM ").filter(|name| known_package(name)) {
            remove_verified_package("rpm", name, &binary)
        } else if let Some(name) = line.strip_prefix("DEB ").filter(|name| known_package(name)) {
            remove_verified_package("deb", name, &binary)
        } else {
            Err("Unsupported privileged uninstall action.".into())
        };
        match &result {
            Ok(()) => writeln!(output, "OK"),
            Err(error) => writeln!(output, "ERROR {}", error.replace(['\r', '\n'], " ")),
        }
        .map_err(|error| error.to_string())?;
        output.flush().map_err(|error| error.to_string())?;
        if result.is_err() {
            break;
        }
    }
    Ok(())
}

fn stop_fixed_services() -> Result<(), String> {
    for name in [
        "com.ctemple.msc2.agent.service",
        "msc2-credential-helper.service",
        "msc2-credential-helper.socket",
    ] {
        let unit = Path::new("/etc/systemd/system").join(name);
        if unit.exists() {
            run_fixed("/usr/bin/systemctl", &["stop", name])?;
            run_fixed("/usr/bin/systemctl", &["disable", name])?;
            fs::remove_file(&unit).map_err(|error| format!("Could not remove {name}: {error}"))?;
        }
    }
    run_fixed("/usr/bin/systemctl", &["daemon-reload"])?;
    for name in [
        "com.ctemple.msc2.agent.service",
        "msc2-credential-helper.service",
        "msc2-credential-helper.socket",
    ] {
        if Command::new("/usr/bin/systemctl")
            .args(["is-active", name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
        {
            return Err(format!("{name} is still active; data was retained."));
        }
    }
    let link = Path::new("/usr/local/bin/msc");
    if fs::read_link(link).is_ok_and(|target| target == Path::new("/usr/lib/msc2/msc")) {
        fs::remove_file(link).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn remove_system_data() -> Result<(), String> {
    let path = Path::new("/var/lib/msc2");
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path).map_err(|error| error.to_string())
        }
        Ok(_) => Err("MSC system data is not a directory; retained for review.".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn remove_verified_package(format: &str, name: &str, binary: &Path) -> Result<(), String> {
    if !known_package(name) {
        return Err("Unrecognized MSC package.".into());
    }
    let binary = binary.to_str().ok_or("Installed MSC path is not UTF-8")?;
    let owner = if format == "rpm" {
        Command::new("/usr/bin/rpm")
            .args(["-qf", "--qf", "%{NAME}", binary])
            .output()
            .map_err(|error| error.to_string())?
    } else {
        Command::new("/usr/bin/dpkg-query")
            .args(["-S", binary])
            .output()
            .map_err(|error| error.to_string())?
    };
    let actual = String::from_utf8_lossy(&owner.stdout);
    let actual = if format == "rpm" {
        actual.trim()
    } else {
        actual.split_once(": ").map_or("", |(package, _)| package)
    };
    if !owner.status.success() || actual != name {
        return Err("Installed MSC binary is not owned by the reviewed package.".into());
    }
    if format == "rpm" {
        run_fixed("/usr/bin/rpm", &["--erase", name])
    } else {
        run_fixed("/usr/bin/dpkg", &["--purge", name])
    }
}

fn remove_marked_archive() -> Result<(), String> {
    let path = Path::new("/usr/lib/msc2");
    if !fs::read_to_string(path.join(".msc2-installation-mode"))
        .is_ok_and(|value| value.trim() == "standalone-archive")
    {
        return Err("MSC standalone archive marker is missing.".into());
    }
    for name in [
        "msc",
        "vantage",
        "bedrock-map",
        "VANTAGE-LICENSE.txt",
        ".msc2-installation-mode",
    ] {
        let target = path.join(name);
        match fs::remove_file(&target) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("Could not remove {}: {error}", target.display())),
        }
    }
    match fs::remove_file("/usr/lib/tmpfiles.d/msc2.conf") {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("Could not remove MSC tmpfiles rule: {error}")),
    }
    if fs::read_dir(path)
        .map_err(|error| error.to_string())?
        .next()
        .is_some()
    {
        return Err("Unrecognized content remains in /usr/lib/msc2; retained for review.".into());
    }
    fs::remove_dir(path).map_err(|error| error.to_string())
}

fn run_fixed(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| format!("Could not run {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} failed ({status})."))
    }
}
