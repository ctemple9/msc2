//! `launchd` service management for the macOS headless agent.
//!
//! Phase 4's service proof uses a root-owned LaunchDaemon plist under
//! `/Library/LaunchDaemons`, with `UserName` set to the installing user
//! (`docs/msc2/substrate/service-identity.md` §2). This module keeps that
//! behavior explicit: it writes the plist, registers it with `launchctl`,
//! and reconstructs the shared `ServiceInstallRequest` back out of the
//! installed plist so `status` reports the same cross-platform shape
//! P4.21 defined.

use msc_infrastructure::service::{
    ServiceError, ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceState,
    ServiceStatusReport,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use security_framework::os::macos::code_signing::{
    Flags, GuestAttributes, SecCode, SecRequirement,
};

const EXPECTED_PORT_ENV: &str = "MSC2_EXPECTED_PORT";

/// The installation key lives as a plain 0600 file next to the rest of the
/// secret store's encrypted files, not in the System keychain — see
/// `secret_store.rs`'s module doc for why keychain-rooted secrets were
/// dropped. The agent and its desktop shell always run as the same regular
/// user, so both sides just read/write this same file directly; nothing
/// needs an elevated install step to provision it.
pub const LOCAL_BOOTSTRAP_KEY_FILE_NAME: &str = "local-bootstrap.key";

pub fn local_bootstrap_key_path(secrets_dir: &Path) -> PathBuf {
    secrets_dir.join(LOCAL_BOOTSTRAP_KEY_FILE_NAME)
}

/// Verifies the live process behind a local socket against the designated
/// requirement recorded when the desktop package was installed. The kernel
/// supplies the PID; the agent never trusts a PID or executable path supplied
/// by the client.
pub fn verify_process_code_identity(pid: u32, requirement: &str) -> Result<(), String> {
    let mut attributes = GuestAttributes::new();
    attributes.set_pid(pid as libc::pid_t);
    let code = SecCode::copy_guest_with_attribues(None, &attributes, Flags::NONE)
        .map_err(|error| format!("looking up bootstrap peer code: {error}"))?;
    let requirement: SecRequirement = requirement
        .parse()
        .map_err(|error| format!("invalid installed desktop requirement: {error}"))?;
    code.check_validity(Flags::NONE, &requirement)
        .map_err(|error| format!("bootstrap peer failed code-identity validation: {error}"))
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemLaunchctl;

pub trait Launchctl: Send + Sync {
    fn bootstrap(&self, plist_path: &Path) -> Result<(), ServiceError>;
    fn bootout(&self, plist_path: &Path) -> Result<(), ServiceError>;
    /// `label` is the bare `Label` from the plist — `start`/`stop` are
    /// the legacy launchctl subcommand family and, unlike
    /// `bootstrap`/`bootout`/`print`, do not take a `<domain>/<label>`
    /// target (`launchctl start system/<label>` fails silently with
    /// exit 3/ESRCH against real launchd; `launchctl start <label>` on
    /// the identical job succeeds).
    fn start(&self, label: &str) -> Result<(), ServiceError>;
    fn stop(&self, label: &str) -> Result<(), ServiceError>;
    fn print(&self, service_target: &str) -> Result<String, ServiceError>;
}

impl Launchctl for SystemLaunchctl {
    fn bootstrap(&self, plist_path: &Path) -> Result<(), ServiceError> {
        run_launchctl(&["bootstrap", "system"], Some(plist_path)).map(|_| ())
    }

    fn bootout(&self, plist_path: &Path) -> Result<(), ServiceError> {
        run_launchctl(&["bootout", "system"], Some(plist_path)).map(|_| ())
    }

    fn start(&self, label: &str) -> Result<(), ServiceError> {
        run_launchctl(&["start", label], None).map(|_| ())
    }

    fn stop(&self, label: &str) -> Result<(), ServiceError> {
        run_launchctl(&["stop", label], None).map(|_| ())
    }

    fn print(&self, service_target: &str) -> Result<String, ServiceError> {
        run_launchctl(&["print", service_target], None)
    }
}

#[derive(Debug)]
pub struct MacosLaunchdServiceManager<L = SystemLaunchctl> {
    plist_root: PathBuf,
    launchctl: L,
}

impl MacosLaunchdServiceManager<SystemLaunchctl> {
    pub fn new() -> Self {
        Self {
            plist_root: PathBuf::from("/Library/LaunchDaemons"),
            launchctl: SystemLaunchctl,
        }
    }
}

impl Default for MacosLaunchdServiceManager<SystemLaunchctl> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L> MacosLaunchdServiceManager<L> {
    pub fn with_launchctl(plist_root: impl Into<PathBuf>, launchctl: L) -> Self {
        Self {
            plist_root: plist_root.into(),
            launchctl,
        }
    }
}

impl<L: Launchctl> ServiceManager for MacosLaunchdServiceManager<L> {
    fn execute(&self, command: ServiceManagerCommand) -> Result<ServiceStatusReport, ServiceError> {
        match command {
            ServiceManagerCommand::Install(request) => self.install(request),
            ServiceManagerCommand::Uninstall { service_name } => {
                self.uninstall(service_name.as_str())
            }
            ServiceManagerCommand::Start { service_name } => self.start(service_name.as_str()),
            ServiceManagerCommand::Stop { service_name } => self.stop(service_name.as_str()),
            ServiceManagerCommand::Status { service_name } => self.status(service_name.as_str()),
        }
    }
}

impl<L: Launchctl> MacosLaunchdServiceManager<L> {
    fn install(&self, request: ServiceInstallRequest) -> Result<ServiceStatusReport, ServiceError> {
        validate_request(&request)?;
        let plist_path = self.plist_path(request.service_name.as_str());
        let plist = LaunchDaemonPlist::from_request(&request);

        if plist_path.exists() {
            let _ = self.launchctl.bootout(&plist_path);
        }

        if let Some(parent) = plist_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                ServiceError::Platform(format!(
                    "creating LaunchDaemon directory {}: {err}",
                    parent.display()
                ))
            })?;
        }

        if let Some(parent) = request.log_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                ServiceError::Platform(format!(
                    "creating log directory {}: {err}",
                    parent.display()
                ))
            })?;
        }

        fs::write(&plist_path, plist.to_xml()).map_err(|err| {
            ServiceError::Platform(format!(
                "writing LaunchDaemon plist {}: {err}",
                plist_path.display()
            ))
        })?;
        fs::set_permissions(&plist_path, fs::Permissions::from_mode(0o644)).map_err(|err| {
            ServiceError::Platform(format!(
                "setting LaunchDaemon plist permissions on {}: {err}",
                plist_path.display()
            ))
        })?;
        self.launchctl.bootstrap(&plist_path)?;
        Ok(ServiceStatusReport::stopped(request))
    }

    fn uninstall(&self, service_name: &str) -> Result<ServiceStatusReport, ServiceError> {
        let plist_path = self.plist_path(service_name);
        if !plist_path.exists() {
            return Ok(ServiceStatusReport::not_installed(service_name.to_string()));
        }

        let _ = self.launchctl.bootout(&plist_path);
        fs::remove_file(&plist_path).map_err(|err| {
            ServiceError::Platform(format!(
                "removing LaunchDaemon plist {}: {err}",
                plist_path.display()
            ))
        })?;

        Ok(ServiceStatusReport::not_installed(service_name.to_string()))
    }

    fn start(&self, service_name: &str) -> Result<ServiceStatusReport, ServiceError> {
        let plist_path = self.require_installed(service_name)?;
        // `start`/`stop` are the legacy launchctl subcommand family and
        // take a bare label, unlike `bootstrap`/`bootout`/`print`'s
        // `<domain>/<label>` target syntax — confirmed against real
        // launchd: `launchctl start system/<label>` fails silently with
        // exit 3 (ESRCH), `launchctl start <label>` on the identical job
        // succeeds. Passing the domain-prefixed target here was never
        // exercised against real launchd before (only against the fake
        // in tests), so it shipped broken.
        self.launchctl.start(service_name)?;
        let definition = LaunchDaemonPlist::from_plist_file(&plist_path)?.into_request()?;
        match self.status(service_name)? {
            ServiceStatusReport {
                state: ServiceState::Running,
                pid: Some(pid),
                ..
            } => Ok(ServiceStatusReport::running(definition, pid)),
            _ => Ok(ServiceStatusReport::stopped(definition)),
        }
    }

    fn stop(&self, service_name: &str) -> Result<ServiceStatusReport, ServiceError> {
        let plist_path = self.require_installed(service_name)?;
        self.launchctl.stop(service_name)?;
        let definition = LaunchDaemonPlist::from_plist_file(&plist_path)?.into_request()?;
        Ok(ServiceStatusReport::stopped(definition))
    }

    fn status(&self, service_name: &str) -> Result<ServiceStatusReport, ServiceError> {
        let plist_path = self.plist_path(service_name);
        if !plist_path.exists() {
            return Ok(ServiceStatusReport::not_installed(service_name.to_string()));
        }

        let definition = LaunchDaemonPlist::from_plist_file(&plist_path)?.into_request()?;
        match self.launchctl.print(&service_target(service_name)) {
            Ok(output) => match parse_pid(&output) {
                Some(pid) => Ok(ServiceStatusReport::running(definition, pid)),
                None => Ok(ServiceStatusReport::stopped(definition)),
            },
            Err(ServiceError::Platform(message)) if is_missing_service_output(&message) => {
                Ok(ServiceStatusReport::stopped(definition))
            }
            Err(err) => Err(err),
        }
    }

    fn require_installed(&self, service_name: &str) -> Result<PathBuf, ServiceError> {
        let plist_path = self.plist_path(service_name);
        if plist_path.exists() {
            Ok(plist_path)
        } else {
            Err(ServiceError::NotInstalled(
                msc_infrastructure::service::ServiceName::new(service_name),
            ))
        }
    }

    fn plist_path(&self, service_name: &str) -> PathBuf {
        self.plist_root.join(format!("{service_name}.plist"))
    }
}

/// Installs and starts the real LaunchDaemon through macOS's administrator
/// prompt. The desktop process remains unprivileged; only the plist copy and
/// launchd registration run in the elevated shell. Neither secret this
/// bootstrap path uses needs privileged provisioning any more (see
/// `secret_store.rs`'s module doc): the caller is expected to have already
/// written the installation key file itself, unprivileged, at
/// `local_bootstrap_key_path` under `MSC2_MACOS_SECRET_STORE_DIR`, before
/// calling this — checked here so a missing key fails closed with a clear
/// error instead of installing a service that can never complete bootstrap.
pub fn install_and_start_elevated(
    request: ServiceInstallRequest,
) -> Result<ServiceStatusReport, ServiceError> {
    let secrets_dir = request
        .environment
        .get("MSC2_MACOS_SECRET_STORE_DIR")
        .ok_or_else(|| {
            ServiceError::InvalidDefinition(
                "MSC2_MACOS_SECRET_STORE_DIR is missing from the install request".to_string(),
            )
        })?;
    let bootstrap_key_path = local_bootstrap_key_path(Path::new(secrets_dir));
    if !bootstrap_key_path.is_file() {
        return Err(ServiceError::InvalidDefinition(format!(
            "the local bootstrap installation key is missing at {}",
            bootstrap_key_path.display()
        )));
    }
    validate_request(&request)?;
    let plist_path = PathBuf::from(format!(
        "/Library/LaunchDaemons/{}.plist",
        request.service_name.as_str()
    ));
    let temporary_plist = std::env::temp_dir().join(format!(
        "msc2-{}.{}.plist",
        request.service_name.as_str(),
        std::process::id()
    ));
    let plist = LaunchDaemonPlist::from_request(&request).to_xml();
    fs::write(&temporary_plist, plist).map_err(|error| {
        ServiceError::Platform(format!(
            "writing temporary LaunchDaemon plist {}: {error}",
            temporary_plist.display()
        ))
    })?;

    let destination = shell_quote(&plist_path.display().to_string());
    let temporary = shell_quote(&temporary_plist.display().to_string());
    let service_target = shell_quote(&format!("system/{}", request.service_name.as_str()));
    let command = format!(
        "if [ -e {destination} ]; then /bin/launchctl bootout system {destination} >/dev/null 2>&1 || true; fi; \
/usr/bin/install -o root -g wheel -m 644 {temporary} {destination}; \
/bin/launchctl bootstrap system {destination}; \
/bin/launchctl kickstart -k {service_target}"
    );

    let result = run_as_administrator(&command);
    let _ = fs::remove_file(&temporary_plist);
    result?;

    wait_for_service_state(&request.service_name, ServiceState::Running)
}

/// Starts an already-installed LaunchDaemon through the same OS-owned
/// elevation boundary used for installation.
pub fn start_elevated(service_name: &str) -> Result<ServiceStatusReport, ServiceError> {
    run_as_administrator(&format!(
        "/bin/launchctl kickstart -k {}",
        shell_quote(&format!("system/{service_name}"))
    ))?;
    wait_for_service_state(
        &msc_infrastructure::service::ServiceName::new(service_name),
        ServiceState::Running,
    )
}

/// Stops an already-installed LaunchDaemon through the same elevation boundary
/// as start. A GUI process's `launchctl stop <label>` targets its own launchd
/// context, not the system LaunchDaemon domain that owns the agent.
pub fn stop_elevated(service_name: &str) -> Result<ServiceStatusReport, ServiceError> {
    run_as_administrator(&elevated_stop_command(service_name))?;
    wait_for_service_state(
        &msc_infrastructure::service::ServiceName::new(service_name),
        ServiceState::Stopped,
    )
}

/// Removes the local LaunchDaemon through the administrator boundary. The
/// desktop may do this only for its own local agent after a full host reset.
pub fn uninstall_elevated(service_name: &str) -> Result<ServiceStatusReport, ServiceError> {
    run_as_administrator(&elevated_uninstall_command(service_name))?;
    wait_for_service_state(
        &msc_infrastructure::service::ServiceName::new(service_name),
        ServiceState::NotInstalled,
    )
}

fn wait_for_service_state(
    service_name: &msc_infrastructure::service::ServiceName,
    expected_state: ServiceState,
) -> Result<ServiceStatusReport, ServiceError> {
    let manager = MacosLaunchdServiceManager::new();
    let mut latest = ServiceStatusReport::not_installed(service_name.as_str().to_string());
    for _ in 0..40 {
        latest = manager.execute(ServiceManagerCommand::Status {
            service_name: service_name.clone(),
        })?;
        if latest.state == expected_state {
            return Ok(latest);
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    Ok(latest)
}

fn run_as_administrator(command: &str) -> Result<(), ServiceError> {
    let script =
        "on run argv\n  do shell script (item 1 of argv) with administrator privileges\nend run";
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", script, "--", command])
        .output()
        .map_err(|error| {
            ServiceError::Platform(format!("starting macOS elevation prompt: {error}"))
        })?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(ServiceError::Platform(if detail.is_empty() {
        format!(
            "macOS administrator authorization failed: {}",
            output.status
        )
    } else {
        format!("macOS administrator authorization failed: {detail}")
    }))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn elevated_stop_command(service_name: &str) -> String {
    format!("/bin/launchctl stop {}", shell_quote(service_name))
}

fn elevated_uninstall_command(service_name: &str) -> String {
    let plist_path = format!("/Library/LaunchDaemons/{service_name}.plist");
    format!(
        "if [ -e {path} ]; then /bin/launchctl bootout system {path} >/dev/null 2>&1 || true; /bin/rm -f {path}; fi",
        path = shell_quote(&plist_path),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevated_stop_uses_the_bare_launchd_label() {
        let command = elevated_stop_command("com.ctemple.msc2.agent");
        assert_eq!(command, "/bin/launchctl stop 'com.ctemple.msc2.agent'");
    }

    #[test]
    fn elevated_uninstall_removes_the_system_launchdaemon() {
        let command = elevated_uninstall_command("com.ctemple.msc2.agent");
        assert_eq!(
            command,
            "if [ -e '/Library/LaunchDaemons/com.ctemple.msc2.agent.plist' ]; then /bin/launchctl bootout system '/Library/LaunchDaemons/com.ctemple.msc2.agent.plist' >/dev/null 2>&1 || true; /bin/rm -f '/Library/LaunchDaemons/com.ctemple.msc2.agent.plist'; fi"
        );
    }
}

fn validate_request(request: &ServiceInstallRequest) -> Result<(), ServiceError> {
    if request.service_name.as_str().trim().is_empty() {
        return Err(ServiceError::InvalidDefinition(
            "LaunchDaemon service name cannot be empty".to_string(),
        ));
    }
    if request.service_name.as_str().contains('/') {
        return Err(ServiceError::InvalidDefinition(format!(
            "LaunchDaemon service name must be a label, not a path: {}",
            request.service_name.as_str()
        )));
    }
    if !request.binary_path.is_absolute() {
        return Err(ServiceError::InvalidDefinition(format!(
            "LaunchDaemon binary path must be absolute: {}",
            request.binary_path.display()
        )));
    }
    if !request.working_directory.is_absolute() {
        return Err(ServiceError::InvalidDefinition(format!(
            "LaunchDaemon working directory must be absolute: {}",
            request.working_directory.display()
        )));
    }
    if !request.log_path.is_absolute() {
        return Err(ServiceError::InvalidDefinition(format!(
            "LaunchDaemon log path must be absolute: {}",
            request.log_path.display()
        )));
    }
    match request.run_user.as_deref() {
        Some(user) if !user.trim().is_empty() => Ok(()),
        _ => Err(ServiceError::InvalidDefinition(
            "LaunchDaemon install requires run_user so UserName matches the installing user"
                .to_string(),
        )),
    }
}

fn service_target(service_name: &str) -> String {
    format!("system/{service_name}")
}

fn parse_pid(output: &str) -> Option<u32> {
    output.lines().find_map(|line| {
        line.trim()
            .strip_prefix("pid = ")
            .and_then(|value| value.trim().parse::<u32>().ok())
    })
}

fn is_missing_service_output(message: &str) -> bool {
    message.contains("Could not find service")
        || message.contains("No such process")
        || message.contains("not found")
}

fn run_launchctl(args: &[&str], plist_path: Option<&Path>) -> Result<String, ServiceError> {
    let mut command = Command::new("/bin/launchctl");
    command.args(args);
    if let Some(path) = plist_path {
        command.arg(path);
    }

    let output = command.output().map_err(|err| {
        ServiceError::Platform(format!("running launchctl {}: {err}", args.join(" ")))
    })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            format!("exit status {}", output.status)
        } else {
            stderr
        };
        Err(ServiceError::Platform(format!(
            "launchctl {} failed: {detail}",
            args.join(" ")
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LaunchDaemonPlist {
    label: String,
    program_arguments: Vec<String>,
    working_directory: String,
    user_name: String,
    standard_out_path: String,
    standard_error_path: String,
    environment: BTreeMap<String, String>,
}

impl LaunchDaemonPlist {
    fn from_request(request: &ServiceInstallRequest) -> Self {
        let mut environment = request.environment.clone();
        environment.insert(
            EXPECTED_PORT_ENV.to_string(),
            request.expected_port.to_string(),
        );

        let mut program_arguments = Vec::with_capacity(request.arguments.len() + 1);
        program_arguments.push(request.binary_path.display().to_string());
        program_arguments.extend(request.arguments.iter().cloned());

        Self {
            label: request.service_name.as_str().to_string(),
            program_arguments,
            working_directory: request.working_directory.display().to_string(),
            user_name: request.run_user.clone().unwrap_or_default(),
            standard_out_path: request.log_path.display().to_string(),
            standard_error_path: request.log_path.display().to_string(),
            environment,
        }
    }

    fn from_plist_file(path: &Path) -> Result<Self, ServiceError> {
        let output = Command::new("/usr/bin/plutil")
            .args(["-convert", "json", "-o", "-"])
            .arg(path)
            .output()
            .map_err(|err| {
                ServiceError::Platform(format!("running plutil against {}: {err}", path.display()))
            })?;
        if !output.status.success() {
            return Err(ServiceError::InvalidDefinition(format!(
                "plutil could not read LaunchDaemon plist {}: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        let json: Value = serde_json::from_slice(&output.stdout).map_err(|err| {
            ServiceError::InvalidDefinition(format!(
                "decoding LaunchDaemon plist {} as JSON: {err}",
                path.display()
            ))
        })?;
        Self::from_json_value(json)
    }

    fn from_json_value(json: Value) -> Result<Self, ServiceError> {
        let object = json.as_object().ok_or_else(|| {
            ServiceError::InvalidDefinition(
                "LaunchDaemon plist root is not a dictionary".to_string(),
            )
        })?;

        let label = string_field(object, "Label")?;
        let working_directory = string_field(object, "WorkingDirectory")?;
        let user_name = string_field(object, "UserName")?;
        let standard_out_path = string_field(object, "StandardOutPath")?;
        let standard_error_path = string_field(object, "StandardErrorPath")?;
        let program_arguments = string_array_field(object, "ProgramArguments")?;
        let environment = string_map_field(object, "EnvironmentVariables")?;

        Ok(Self {
            label,
            program_arguments,
            working_directory,
            user_name,
            standard_out_path,
            standard_error_path,
            environment,
        })
    }

    fn into_request(mut self) -> Result<ServiceInstallRequest, ServiceError> {
        if self.program_arguments.is_empty() {
            return Err(ServiceError::InvalidDefinition(
                "LaunchDaemon plist is missing ProgramArguments[0]".to_string(),
            ));
        }
        if self.standard_out_path != self.standard_error_path {
            return Err(ServiceError::InvalidDefinition(format!(
                "LaunchDaemon plist uses different stdout/stderr paths ({} vs {}), but MSC 2's shared service model stores one log path",
                self.standard_out_path, self.standard_error_path
            )));
        }

        let binary_path = PathBuf::from(self.program_arguments.remove(0));
        let expected_port = self
            .environment
            .remove(EXPECTED_PORT_ENV)
            .ok_or_else(|| {
                ServiceError::InvalidDefinition(format!(
                    "LaunchDaemon plist is missing {EXPECTED_PORT_ENV} in EnvironmentVariables"
                ))
            })?
            .parse::<u16>()
            .map_err(|err| {
                ServiceError::InvalidDefinition(format!(
                    "LaunchDaemon plist has invalid {EXPECTED_PORT_ENV}: {err}"
                ))
            })?;

        let mut request = ServiceInstallRequest::new(
            self.label,
            binary_path,
            self.working_directory,
            self.standard_out_path,
            expected_port,
        )
        .args(self.program_arguments);
        request.run_user = Some(self.user_name);
        request.environment = self.environment;
        Ok(request)
    }

    fn to_xml(&self) -> String {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
"#,
        );
        push_string_key(&mut xml, "Label", &self.label);
        push_string_array(&mut xml, "ProgramArguments", &self.program_arguments);
        push_string_key(&mut xml, "WorkingDirectory", &self.working_directory);
        push_string_key(&mut xml, "UserName", &self.user_name);
        push_string_map(&mut xml, "EnvironmentVariables", &self.environment);
        push_string_key(&mut xml, "StandardOutPath", &self.standard_out_path);
        push_string_key(&mut xml, "StandardErrorPath", &self.standard_error_path);
        xml.push_str("<key>RunAtLoad</key>\n<false/>\n");
        xml.push_str("<key>KeepAlive</key>\n<false/>\n");
        xml.push_str("</dict>\n</plist>\n");
        xml
    }
}

fn string_field(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, ServiceError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            ServiceError::InvalidDefinition(format!(
                "LaunchDaemon plist is missing string key {key}"
            ))
        })
}

fn string_array_field(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<Vec<String>, ServiceError> {
    let array = object.get(key).and_then(Value::as_array).ok_or_else(|| {
        ServiceError::InvalidDefinition(format!("LaunchDaemon plist is missing array key {key}"))
    })?;
    array
        .iter()
        .map(|value| {
            value.as_str().map(ToOwned::to_owned).ok_or_else(|| {
                ServiceError::InvalidDefinition(format!(
                    "LaunchDaemon plist key {key} contains a non-string value"
                ))
            })
        })
        .collect()
}

fn string_map_field(
    object: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<BTreeMap<String, String>, ServiceError> {
    let map = object.get(key).and_then(Value::as_object).ok_or_else(|| {
        ServiceError::InvalidDefinition(format!(
            "LaunchDaemon plist is missing dictionary key {key}"
        ))
    })?;
    map.iter()
        .map(|(name, value)| {
            value
                .as_str()
                .map(|text| (name.clone(), text.to_string()))
                .ok_or_else(|| {
                    ServiceError::InvalidDefinition(format!(
                        "LaunchDaemon plist key {key} contains a non-string value for {name}"
                    ))
                })
        })
        .collect()
}

fn push_string_key(xml: &mut String, key: &str, value: &str) {
    xml.push_str("<key>");
    xml.push_str(&xml_escape(key));
    xml.push_str("</key>\n<string>");
    xml.push_str(&xml_escape(value));
    xml.push_str("</string>\n");
}

fn push_string_array(xml: &mut String, key: &str, values: &[String]) {
    xml.push_str("<key>");
    xml.push_str(&xml_escape(key));
    xml.push_str("</key>\n<array>\n");
    for value in values {
        xml.push_str("<string>");
        xml.push_str(&xml_escape(value));
        xml.push_str("</string>\n");
    }
    xml.push_str("</array>\n");
}

fn push_string_map(xml: &mut String, key: &str, values: &BTreeMap<String, String>) {
    xml.push_str("<key>");
    xml.push_str(&xml_escape(key));
    xml.push_str("</key>\n<dict>\n");
    for (name, value) in values {
        xml.push_str("<key>");
        xml.push_str(&xml_escape(name));
        xml.push_str("</key>\n<string>");
        xml.push_str(&xml_escape(value));
        xml.push_str("</string>\n");
    }
    xml.push_str("</dict>\n");
}

fn xml_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
