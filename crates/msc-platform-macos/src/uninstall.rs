//! Narrow native operations for the confirmed, host-local uninstall flow.
use crate::service::{self, MacosLaunchdServiceManager};
use msc_infrastructure::service::{
    ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceName, ServiceState,
    ServiceStatusReport,
};
use msc_infrastructure::uninstall::native::{self, Installation, LocalServices};
use std::path::Path;

pub struct MacUninstall;
impl LocalServices for MacUninstall {
    fn inspect(&self) -> Result<Vec<ServiceStatusReport>, String> {
        let manager = MacosLaunchdServiceManager::new();
        let agent = manager
            .execute(ServiceManagerCommand::Status {
                service_name: ServiceName::new("com.ctemple.msc2.agent"),
            })
            .map_err(|error| error.to_string())?;
        let helper = service::inspect_bedrock_helper().map_err(|error| error.to_string())?;
        let definition = (helper.state != ServiceState::NotInstalled).then(|| {
            ServiceInstallRequest::new(
                "com.ctemple.msc2.bedrock-helper",
                "/Library/Application Support/MSC 2/bedrock-helper/BedrockSidecar",
                "/Library/Application Support/MSC 2/bedrock-helper",
                "/Library/Logs/msc2-bedrock-helper.log",
                0,
            )
        });
        Ok(vec![
            agent,
            ServiceStatusReport {
                service_name: ServiceName::new("com.ctemple.msc2.bedrock-helper"),
                state: helper.state,
                pid: helper.pid,
                definition,
            },
        ])
    }
    fn remove(&self) -> Result<(), String> {
        let reports = self.inspect()?;
        if reports
            .iter()
            .any(|report| report.state != ServiceState::NotInstalled)
        {
            service::uninstall_elevated("com.ctemple.msc2.agent")
                .map_err(|error| error.to_string())?;
        }
        // A missing plist alone is not proof the launchd job has exited.
        for name in ["com.ctemple.msc2.agent", "com.ctemple.msc2.bedrock-helper"] {
            if native::capture("/bin/launchctl", &["print", &format!("system/{name}")]).is_ok() {
                return Err(format!(
                    "The launchd job {name} remains loaded; data was retained."
                ));
            }
        }
        let link = Path::new("/usr/local/bin/msc");
        if let Ok(target) = std::fs::read_link(link) {
            let owned = reports
                .iter()
                .filter_map(|report| report.definition.as_ref())
                .any(|definition| definition.binary_path == target)
                || (target.starts_with("/usr/local/lib/msc2")
                    && target.parent().is_some_and(|parent| {
                        std::fs::read_to_string(parent.join(".msc2-owned"))
                            .is_ok_and(|value| value.trim() == "msc2-headless-archive")
                    }));
            if owned {
                service::remove_desktop_cli_link_elevated(&target)
                    .map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }
    fn remove_installation(&self, installation: &Installation) -> Result<(), String> {
        let path = match installation {
            Installation::MacBundle { path } => {
                native::verify_mac_bundle(path)?;
                path
            }
            Installation::MarkedArchive { path } if path.starts_with("/usr/local/lib/msc2") => {
                verify_marker(path)?;
                path
            }
            _ => return Err("Unsupported macOS installation identity.".into()),
        };
        if path
            .symlink_metadata()
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err("Installation changed to a symlink.".into());
        }
        if native::remove_path(path).is_ok() {
            return Ok(());
        }
        // Elevation is limited to root-owned fixed installation parents. User
        // directories never become arbitrary elevated recursive-delete targets.
        if path.parent() != Some(Path::new("/Applications"))
            && !path.starts_with("/usr/local/lib/msc2")
        {
            return Err(format!(
                "Cannot remove {}; check its permissions.",
                path.display()
            ));
        }
        let quoted = quote(&path.display().to_string());
        let guard = match installation {
            Installation::MacBundle { .. } => format!(
                "[ \"$(/usr/bin/plutil -extract CFBundleIdentifier raw -o - {quoted}/Contents/Info.plist)\" = com.ctemple.msc2 ]"
            ),
            _ => format!("[ \"$(/bin/cat {quoted}/.msc2-owned)\" = msc2-headless-archive ]"),
        };
        service::run_as_administrator(&format!(
            "[ ! -L {quoted} ] && {guard} && /bin/rm -rf -- {quoted}"
        ))
        .map_err(|error| error.to_string())?;
        if path.exists() {
            return Err(format!("Installation remains at {}.", path.display()));
        }
        Ok(())
    }
    fn clear_credentials(&self) -> Result<(), String> {
        Ok(())
    }
}
fn verify_marker(path: &Path) -> Result<(), String> {
    if std::fs::read_to_string(path.join(".msc2-owned"))
        .is_ok_and(|value| value.trim() == "msc2-headless-archive")
    {
        Ok(())
    } else {
        Err("Headless ownership marker changed.".into())
    }
}
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
