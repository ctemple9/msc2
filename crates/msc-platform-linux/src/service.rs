//! `systemd` service management for the Linux headless agent.
//!
//! Phase 4's Linux proof uses a root-owned system unit under
//! `/etc/systemd/system`, but the service itself runs as the installing
//! user via `User=` and `Group=`. This module writes that unit, reloads
//! `systemd`, and reconstructs the shared `ServiceInstallRequest` back out
//! of metadata comments in the installed unit so `status` returns the same
//! cross-platform shape P4.21 defined.

use msc_infrastructure::service::{
    ServiceError, ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceName,
    ServiceState, ServiceStatusReport,
};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const EXPECTED_PORT_ENV: &str = "MSC2_EXPECTED_PORT";
const META_PREFIX: &str = "# MSC2-";

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemSystemctl;

pub trait Systemctl: Send + Sync {
    fn daemon_reload(&self) -> Result<(), ServiceError>;
    fn enable(&self, unit_name: &str) -> Result<(), ServiceError>;
    fn disable(&self, unit_name: &str) -> Result<(), ServiceError>;
    fn start(&self, unit_name: &str) -> Result<(), ServiceError>;
    fn stop(&self, unit_name: &str) -> Result<(), ServiceError>;
    fn show(&self, unit_name: &str) -> Result<String, ServiceError>;
}

impl Systemctl for SystemSystemctl {
    fn daemon_reload(&self) -> Result<(), ServiceError> {
        run_systemctl(&["daemon-reload"]).map(|_| ())
    }

    fn enable(&self, unit_name: &str) -> Result<(), ServiceError> {
        run_systemctl(&["enable", unit_name]).map(|_| ())
    }

    fn disable(&self, unit_name: &str) -> Result<(), ServiceError> {
        run_systemctl(&["disable", unit_name]).map(|_| ())
    }

    fn start(&self, unit_name: &str) -> Result<(), ServiceError> {
        run_systemctl(&["start", unit_name]).map(|_| ())
    }

    fn stop(&self, unit_name: &str) -> Result<(), ServiceError> {
        run_systemctl(&["stop", unit_name]).map(|_| ())
    }

    fn show(&self, unit_name: &str) -> Result<String, ServiceError> {
        run_systemctl(&[
            "show",
            unit_name,
            "--property",
            "ActiveState",
            "--property",
            "MainPID",
        ])
    }
}

#[derive(Debug)]
pub struct LinuxSystemdServiceManager<S = SystemSystemctl> {
    unit_root: PathBuf,
    systemctl: S,
}

impl LinuxSystemdServiceManager<SystemSystemctl> {
    pub fn new() -> Self {
        Self {
            unit_root: PathBuf::from("/etc/systemd/system"),
            systemctl: SystemSystemctl,
        }
    }
}

impl Default for LinuxSystemdServiceManager<SystemSystemctl> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> LinuxSystemdServiceManager<S> {
    pub fn with_systemctl(unit_root: impl Into<PathBuf>, systemctl: S) -> Self {
        Self {
            unit_root: unit_root.into(),
            systemctl,
        }
    }
}

impl<S: Systemctl> ServiceManager for LinuxSystemdServiceManager<S> {
    fn execute(&self, command: ServiceManagerCommand) -> Result<ServiceStatusReport, ServiceError> {
        match command {
            ServiceManagerCommand::Install(request) => self.install(request),
            ServiceManagerCommand::Uninstall { service_name } => self.uninstall(&service_name),
            ServiceManagerCommand::Start { service_name } => self.start(&service_name),
            ServiceManagerCommand::Stop { service_name } => self.stop(&service_name),
            ServiceManagerCommand::Status { service_name } => self.status(&service_name),
        }
    }
}

impl<S: Systemctl> LinuxSystemdServiceManager<S> {
    fn install(&self, request: ServiceInstallRequest) -> Result<ServiceStatusReport, ServiceError> {
        validate_request(&request)?;
        let unit_name = unit_name(request.service_name.as_str());
        let unit_path = self.unit_path(request.service_name.as_str());
        let unit = SystemdUnit::from_request(&request);

        if unit_path.exists() {
            let _ = self.systemctl.stop(&unit_name);
            let _ = self.systemctl.disable(&unit_name);
        }

        if let Some(parent) = unit_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                ServiceError::Platform(format!(
                    "creating systemd unit directory {}: {err}",
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

        fs::write(&unit_path, unit.render()).map_err(|err| {
            ServiceError::Platform(format!(
                "writing systemd unit {}: {err}",
                unit_path.display()
            ))
        })?;
        fs::set_permissions(&unit_path, fs::Permissions::from_mode(0o644)).map_err(|err| {
            ServiceError::Platform(format!(
                "setting systemd unit permissions on {}: {err}",
                unit_path.display()
            ))
        })?;
        self.systemctl.daemon_reload()?;
        self.systemctl.enable(&unit_name)?;
        Ok(ServiceStatusReport::stopped(request))
    }

    fn uninstall(&self, service_name: &ServiceName) -> Result<ServiceStatusReport, ServiceError> {
        let unit_name = unit_name(service_name.as_str());
        let unit_path = self.unit_path(service_name.as_str());
        if !unit_path.exists() {
            return Ok(ServiceStatusReport::not_installed(
                service_name.as_str().to_string(),
            ));
        }

        let _ = self.systemctl.stop(&unit_name);
        let _ = self.systemctl.disable(&unit_name);
        fs::remove_file(&unit_path).map_err(|err| {
            ServiceError::Platform(format!(
                "removing systemd unit {}: {err}",
                unit_path.display()
            ))
        })?;
        self.systemctl.daemon_reload()?;
        Ok(ServiceStatusReport::not_installed(
            service_name.as_str().to_string(),
        ))
    }

    fn start(&self, service_name: &ServiceName) -> Result<ServiceStatusReport, ServiceError> {
        let unit_name = unit_name(service_name.as_str());
        self.require_installed(service_name)?;
        self.systemctl.start(&unit_name)?;
        self.status(service_name)
    }

    fn stop(&self, service_name: &ServiceName) -> Result<ServiceStatusReport, ServiceError> {
        self.require_installed(service_name)?;
        let unit_name = unit_name(service_name.as_str());
        self.systemctl.stop(&unit_name)?;
        self.status(service_name)
    }

    fn status(&self, service_name: &ServiceName) -> Result<ServiceStatusReport, ServiceError> {
        let unit_path = self.unit_path(service_name.as_str());
        if !unit_path.exists() {
            return Ok(ServiceStatusReport::not_installed(
                service_name.as_str().to_string(),
            ));
        }

        let definition = SystemdUnit::from_unit_file(&unit_path)?.into_request()?;
        let output = self.systemctl.show(&unit_name(service_name.as_str()))?;
        let state = parse_state(&output);
        let pid = parse_main_pid(&output);
        Ok(match (state, pid) {
            (Some(ServiceState::Running), Some(pid)) if pid > 0 => {
                ServiceStatusReport::running(definition, pid)
            }
            _ => ServiceStatusReport::stopped(definition),
        })
    }

    fn require_installed(&self, service_name: &ServiceName) -> Result<PathBuf, ServiceError> {
        let unit_path = self.unit_path(service_name.as_str());
        if unit_path.exists() {
            Ok(unit_path)
        } else {
            Err(ServiceError::NotInstalled(service_name.clone()))
        }
    }

    fn unit_path(&self, service_name: &str) -> PathBuf {
        self.unit_root.join(unit_name(service_name))
    }
}

fn validate_request(request: &ServiceInstallRequest) -> Result<(), ServiceError> {
    if request.service_name.as_str().trim().is_empty() {
        return Err(ServiceError::InvalidDefinition(
            "systemd service name cannot be empty".to_string(),
        ));
    }
    if request.service_name.as_str().contains('/') {
        return Err(ServiceError::InvalidDefinition(format!(
            "systemd service name must be a label, not a path: {}",
            request.service_name.as_str()
        )));
    }
    if !request.binary_path.is_absolute() {
        return Err(ServiceError::InvalidDefinition(format!(
            "systemd binary path must be absolute: {}",
            request.binary_path.display()
        )));
    }
    if !request.working_directory.is_absolute() {
        return Err(ServiceError::InvalidDefinition(format!(
            "systemd working directory must be absolute: {}",
            request.working_directory.display()
        )));
    }
    if !request.log_path.is_absolute() {
        return Err(ServiceError::InvalidDefinition(format!(
            "systemd log path must be absolute: {}",
            request.log_path.display()
        )));
    }
    match request.run_user.as_deref() {
        Some(user) if !user.trim().is_empty() => Ok(()),
        _ => Err(ServiceError::InvalidDefinition(
            "systemd install requires run_user so User= and Group= match the installing user"
                .to_string(),
        )),
    }
}

fn unit_name(service_name: &str) -> String {
    format!("{service_name}.service")
}

fn parse_state(output: &str) -> Option<ServiceState> {
    output.lines().find_map(|line| {
        let value = line.strip_prefix("ActiveState=")?;
        Some(match value.trim() {
            "active" => ServiceState::Running,
            _ => ServiceState::Stopped,
        })
    })
}

fn parse_main_pid(output: &str) -> Option<u32> {
    output.lines().find_map(|line| {
        line.strip_prefix("MainPID=")
            .and_then(|value| value.trim().parse::<u32>().ok())
    })
}

fn run_systemctl(args: &[&str]) -> Result<String, ServiceError> {
    let output = Command::new("systemctl")
        .args(args)
        .output()
        .map_err(|err| {
            ServiceError::Platform(format!("running systemctl {}: {err}", args.join(" ")))
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
            "systemctl {} failed: {detail}",
            args.join(" ")
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SystemdUnit {
    service_name: String,
    binary_path: String,
    working_directory: String,
    log_path: String,
    run_user: String,
    expected_port: u16,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
}

impl SystemdUnit {
    fn from_request(request: &ServiceInstallRequest) -> Self {
        Self {
            service_name: request.service_name.as_str().to_string(),
            binary_path: request.binary_path.display().to_string(),
            working_directory: request.working_directory.display().to_string(),
            log_path: request.log_path.display().to_string(),
            run_user: request.run_user.clone().unwrap_or_default(),
            expected_port: request.expected_port,
            arguments: request.arguments.clone(),
            environment: request.environment.clone(),
        }
    }

    fn from_unit_file(path: &Path) -> Result<Self, ServiceError> {
        let text = fs::read_to_string(path).map_err(|err| {
            ServiceError::InvalidDefinition(format!(
                "reading systemd unit {}: {err}",
                path.display()
            ))
        })?;
        Self::from_rendered(&text)
    }

    fn from_rendered(text: &str) -> Result<Self, ServiceError> {
        let metadata = metadata_map(text)?;

        let service_name = decode_metadata_value(required_metadata(&metadata, "ServiceName")?)?;
        let binary_path = decode_metadata_value(required_metadata(&metadata, "BinaryPath")?)?;
        let working_directory =
            decode_metadata_value(required_metadata(&metadata, "WorkingDirectory")?)?;
        let log_path = decode_metadata_value(required_metadata(&metadata, "LogPath")?)?;
        let run_user = decode_metadata_value(required_metadata(&metadata, "RunUser")?)?;
        let expected_port = required_metadata(&metadata, "ExpectedPort")?
            .parse::<u16>()
            .map_err(|err| {
                ServiceError::InvalidDefinition(format!(
                    "invalid systemd unit ExpectedPort metadata: {err}"
                ))
            })?;
        let arguments = decode_metadata_value(required_metadata(&metadata, "Arguments")?)?
            .split('\0')
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        let environment = parse_environment_metadata(&decode_metadata_value(required_metadata(
            &metadata,
            "Environment",
        )?)?)?;

        Ok(Self {
            service_name,
            binary_path,
            working_directory,
            log_path,
            run_user,
            expected_port,
            arguments,
            environment,
        })
    }

    fn into_request(self) -> Result<ServiceInstallRequest, ServiceError> {
        let mut request = ServiceInstallRequest::new(
            self.service_name,
            self.binary_path,
            self.working_directory,
            self.log_path,
            self.expected_port,
        )
        .args(self.arguments);
        request.run_user = Some(self.run_user);
        request.environment = self.environment;
        validate_request(&request)?;
        Ok(request)
    }

    fn render(&self) -> String {
        let mut lines = vec![
            format!(
                "{META_PREFIX}ServiceName={}",
                encode_metadata_value(&self.service_name)
            ),
            format!(
                "{META_PREFIX}BinaryPath={}",
                encode_metadata_value(&self.binary_path)
            ),
            format!(
                "{META_PREFIX}WorkingDirectory={}",
                encode_metadata_value(&self.working_directory)
            ),
            format!(
                "{META_PREFIX}LogPath={}",
                encode_metadata_value(&self.log_path)
            ),
            format!(
                "{META_PREFIX}RunUser={}",
                encode_metadata_value(&self.run_user)
            ),
            format!("{META_PREFIX}ExpectedPort={}", self.expected_port),
            format!(
                "{META_PREFIX}Arguments={}",
                encode_metadata_value(&self.arguments.join("\0"))
            ),
            format!(
                "{META_PREFIX}Environment={}",
                encode_metadata_value(&format_environment_metadata(&self.environment))
            ),
            "[Unit]".to_string(),
            format!("Description=MSC 2 agent ({})", self.service_name),
            "After=network.target".to_string(),
            String::new(),
            "[Service]".to_string(),
            "Type=simple".to_string(),
            format!("User={}", self.run_user),
            format!("Group={}", self.run_user),
            format!("WorkingDirectory={}", self.working_directory),
            format!(
                "ExecStart={}",
                render_exec_start(&self.binary_path, &self.arguments)
            ),
        ];

        for (key, value) in &self.environment {
            lines.push(format!(
                "Environment={}",
                quote_systemd_value(&format!("{key}={value}"))
            ));
        }
        lines.push(format!(
            "Environment={}",
            quote_systemd_value(&format!("{EXPECTED_PORT_ENV}={}", self.expected_port))
        ));
        lines.push(format!("StandardOutput=append:{}", self.log_path));
        lines.push(format!("StandardError=append:{}", self.log_path));
        lines.push("Restart=no".to_string());
        lines.push(String::new());
        lines.push("[Install]".to_string());
        lines.push("WantedBy=multi-user.target".to_string());
        lines.push(String::new());
        lines.join("\n")
    }
}

fn render_exec_start(binary_path: &str, arguments: &[String]) -> String {
    std::iter::once(binary_path)
        .chain(arguments.iter().map(String::as_str))
        .map(quote_systemd_value)
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_systemd_value(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn metadata_map(text: &str) -> Result<BTreeMap<String, String>, ServiceError> {
    let mut values = BTreeMap::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix(META_PREFIX) else {
            continue;
        };
        let (key, value) = rest.split_once('=').ok_or_else(|| {
            ServiceError::InvalidDefinition(format!(
                "systemd unit metadata line is missing '=': {line}"
            ))
        })?;
        values.insert(key.to_string(), value.to_string());
    }
    Ok(values)
}

fn required_metadata<'a>(
    metadata: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, ServiceError> {
    metadata.get(key).map(String::as_str).ok_or_else(|| {
        ServiceError::InvalidDefinition(format!("systemd unit is missing metadata {key}"))
    })
}

fn format_environment_metadata(environment: &BTreeMap<String, String>) -> String {
    environment
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_environment_metadata(text: &str) -> Result<BTreeMap<String, String>, ServiceError> {
    let mut environment = BTreeMap::new();
    if text.is_empty() {
        return Ok(environment);
    }
    for line in text.lines() {
        let (key, value) = line.split_once('=').ok_or_else(|| {
            ServiceError::InvalidDefinition(format!(
                "invalid systemd environment metadata entry: {line}"
            ))
        })?;
        environment.insert(key.to_string(), value.to_string());
    }
    Ok(environment)
}

fn encode_metadata_value(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode_metadata_value(value: &str) -> Result<String, ServiceError> {
    if !value.len().is_multiple_of(2) {
        return Err(ServiceError::InvalidDefinition(
            "systemd metadata hex value has odd length".to_string(),
        ));
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16).map_err(|err| {
                ServiceError::InvalidDefinition(format!(
                    "systemd metadata hex decode failed: {err}"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(bytes).map_err(|err| {
        ServiceError::InvalidDefinition(format!("systemd metadata is not valid UTF-8: {err}"))
    })
}
