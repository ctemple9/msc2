//! Host-local systemd/package cleanup; elevated operations use fixed commands.
use crate::service::LinuxSystemdServiceManager;
use msc_infrastructure::service::{
    ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceName, ServiceState,
    ServiceStatusReport,
};
use msc_infrastructure::uninstall::native::{self, Installation, LocalServices};
use std::path::Path;

pub struct LinuxUninstall;
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
        // Stop/unregister only fixed MSC units. An unrelated service can never
        // be named through inventory input or command-line arguments.
        for name in [
            "com.ctemple.msc2.agent.service",
            "msc2-credential-helper.service",
            "msc2-credential-helper.socket",
        ] {
            if Path::new("/etc/systemd/system").join(name).exists() {
                elevated("systemctl", &["stop", name])?;
                elevated("systemctl", &["disable", name])?;
                elevated("rm", &["-f", "--", &format!("/etc/systemd/system/{name}")])?;
            }
        }
        elevated("systemctl", &["daemon-reload"])?;
        for name in [
            "com.ctemple.msc2.agent.service",
            "msc2-credential-helper.service",
            "msc2-credential-helper.socket",
        ] {
            if native::capture("systemctl", &["is-active", name]).is_ok() {
                return Err(format!("{name} is still active; data was retained."));
            }
        }
        if std::fs::read_link("/usr/local/bin/msc")
            .is_ok_and(|target| target == Path::new("/usr/lib/msc2/msc"))
        {
            elevated("rm", &["-f", "--", "/usr/local/bin/msc"])?;
        }
        Ok(())
    }
    fn remove_data(&self, path: &Path) -> Result<(), String> {
        if path == Path::new("/var/lib/msc2") {
            elevated("rm", &["-rf", "--", "/var/lib/msc2"])
        } else {
            native::remove_path(path)
        }
    }
    fn remove_installation(&self, installation: &Installation) -> Result<(), String> {
        match installation {
            Installation::Deb { name } if known_package(name) => {
                elevated("dpkg", &["--purge", name])
            }
            Installation::Rpm { name } if known_package(name) => {
                elevated("rpm", &["--erase", name])
            }
            Installation::LinuxArchive { path }
                if path == Path::new("/usr/lib/msc2")
                    && std::fs::read_to_string(path.join(".msc2-installation-mode"))
                        .is_ok_and(|value| value.trim() == "standalone-archive") =>
            {
                // Do not recursively delete unmarked user content in the binary
                // installation directory; enumerate exactly the shipped files.
                for name in [
                    "msc",
                    "vantage",
                    "bedrock-map",
                    "VANTAGE-LICENSE.txt",
                    ".msc2-installation-mode",
                ] {
                    elevated("rm", &["-f", "--", &path.join(name).display().to_string()])?;
                }
                elevated("rm", &["-f", "--", "/usr/lib/tmpfiles.d/msc2.conf"])?;
                if path
                    .read_dir()
                    .map_err(|error| error.to_string())?
                    .next()
                    .is_some()
                {
                    return Err(
                        "Unrecognized content remains in /usr/lib/msc2; retained for review."
                            .into(),
                    );
                }
                elevated("rmdir", &["--", "/usr/lib/msc2"])
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
fn elevated(program: &str, args: &[&str]) -> Result<(), String> {
    // pkexec presents the OS authorization dialog; a CLI without a GUI uses
    // the terminal's sudo authentication. No shell interpolation is involved.
    let mut command = std::process::Command::new(
        if std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some() {
            "pkexec"
        } else {
            "sudo"
        },
    );
    let status = command
        .arg(format!("/usr/bin/{program}"))
        .args(args)
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} failed ({status})."))
    }
}
