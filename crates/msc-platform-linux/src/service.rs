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
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const EXPECTED_PORT_ENV: &str = "MSC2_EXPECTED_PORT";
const META_PREFIX: &str = "# MSC2-";
pub const DESKTOP_AGENT_SERVICE_NAME: &str = "com.ctemple.msc2.agent";

pub trait AuthorizationRunner: Send + Sync {
    fn authorize(&self, helper: &Path, args: &[OsString]) -> Result<Output, ServiceError>;

    fn requires_system_owned_helper(&self) -> bool {
        false
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PkexecRunner;

impl AuthorizationRunner for PkexecRunner {
    fn authorize(&self, helper: &Path, args: &[OsString]) -> Result<Output, ServiceError> {
        Command::new("pkexec")
            .arg(helper)
            .args(args)
            .output()
            .map_err(|error| {
                ServiceError::Platform(format!(
                    "starting the Linux authorization prompt with pkexec: {error}. Install polkit and try again."
                ))
            })
    }

    fn requires_system_owned_helper(&self) -> bool {
        true
    }
}

pub fn install_desktop_service_elevated(
    request: ServiceInstallRequest,
    helper: &Path,
) -> Result<ServiceStatusReport, ServiceError> {
    install_desktop_service_with_runner(request, helper, &PkexecRunner)
}

pub fn install_desktop_service_with_runner<R: AuthorizationRunner>(
    request: ServiceInstallRequest,
    helper: &Path,
    runner: &R,
) -> Result<ServiceStatusReport, ServiceError> {
    let helper = validate_helper_executable(helper, runner.requires_system_owned_helper())?;
    validate_desktop_request(&request, current_uid()?)?;
    let output = runner.authorize(&helper, &helper_install_args(&request))?;
    ensure_authorized_command_succeeded(output)?;
    LinuxSystemdServiceManager::new().execute(ServiceManagerCommand::Status {
        service_name: ServiceName::new(DESKTOP_AGENT_SERVICE_NAME),
    })
}

pub fn uninstall_desktop_service_elevated(
    helper: &Path,
) -> Result<ServiceStatusReport, ServiceError> {
    uninstall_desktop_service_with_runner(helper, &PkexecRunner)
}

pub fn uninstall_desktop_service_with_runner<R: AuthorizationRunner>(
    helper: &Path,
    runner: &R,
) -> Result<ServiceStatusReport, ServiceError> {
    let helper = validate_helper_executable(helper, runner.requires_system_owned_helper())?;
    let args = vec![
        OsString::from("desktop-service-helper"),
        OsString::from("uninstall"),
    ];
    let output = runner.authorize(&helper, &args)?;
    ensure_authorized_command_succeeded(output)?;
    Ok(ServiceStatusReport::not_installed(
        DESKTOP_AGENT_SERVICE_NAME.to_string(),
    ))
}

pub fn helper_install_args(request: &ServiceInstallRequest) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("desktop-service-helper"),
        OsString::from("install"),
        OsString::from("--binary-path"),
        request.binary_path.as_os_str().to_owned(),
        OsString::from("--working-directory"),
        request.working_directory.as_os_str().to_owned(),
        OsString::from("--log-path"),
        request.log_path.as_os_str().to_owned(),
        OsString::from("--run-user"),
        OsString::from(request.run_user.as_deref().unwrap_or_default()),
        OsString::from("--expected-port"),
        OsString::from(request.expected_port.to_string()),
    ];
    for argument in &request.arguments {
        args.extend([OsString::from("--arg"), OsString::from(argument)]);
    }
    for (key, value) in &request.environment {
        args.extend([
            OsString::from("--env"),
            OsString::from(format!("{key}={value}")),
        ]);
    }
    args
}

pub fn run_desktop_service_helper_install(
    request: ServiceInstallRequest,
) -> Result<(), ServiceError> {
    if unsafe { libc::geteuid() } != 0 {
        return Err(ServiceError::Platform(
            "the MSC desktop service helper must run through pkexec".to_string(),
        ));
    }
    let uid = std::env::var("PKEXEC_UID")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| {
            ServiceError::Platform(
                "pkexec did not provide the installing user's identity".to_string(),
            )
        })?;
    validate_desktop_request(&request, uid)?;
    let manager = LinuxSystemdServiceManager::new();
    manager.execute(ServiceManagerCommand::Install(request.clone()))?;
    manager.execute(ServiceManagerCommand::Start {
        service_name: ServiceName::new(DESKTOP_AGENT_SERVICE_NAME),
    })?;
    Ok(())
}

pub fn run_desktop_service_helper_uninstall() -> Result<(), ServiceError> {
    if unsafe { libc::geteuid() } != 0 {
        return Err(ServiceError::Platform(
            "the MSC desktop service helper must run through pkexec".to_string(),
        ));
    }
    let uid = std::env::var("PKEXEC_UID")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| {
            ServiceError::Platform(
                "pkexec did not provide the installing user's identity".to_string(),
            )
        })?;
    run_desktop_service_helper_uninstall_for_uid(uid, &LinuxSystemdServiceManager::new())?;
    Ok(())
}

pub fn run_desktop_service_helper_uninstall_for_uid<S: Systemctl>(
    uid: u32,
    manager: &LinuxSystemdServiceManager<S>,
) -> Result<(), ServiceError> {
    let (user, _) = user_identity(uid)?;
    if let Ok(report) = manager.execute(ServiceManagerCommand::Status {
        service_name: ServiceName::new(DESKTOP_AGENT_SERVICE_NAME),
    }) && let Some(definition) = report.definition
        && definition.run_user.as_deref() != Some(user.as_str())
    {
        return invalid_desktop_request(
            "the installed MSC service belongs to a different user account",
        );
    }
    manager.execute(ServiceManagerCommand::Uninstall {
        service_name: ServiceName::new(DESKTOP_AGENT_SERVICE_NAME),
    })?;
    Ok(())
}

fn validate_helper_executable(
    helper: &Path,
    require_root_owner: bool,
) -> Result<PathBuf, ServiceError> {
    if !helper.is_absolute() {
        return Err(ServiceError::InvalidDefinition(format!(
            "MSC service helper path must be absolute: {}",
            helper.display()
        )));
    }
    let helper = fs::canonicalize(helper).map_err(|error| {
        ServiceError::InvalidDefinition(format!(
            "could not resolve MSC service helper {}: {error}",
            helper.display()
        ))
    })?;
    let metadata = fs::symlink_metadata(&helper).map_err(|error| {
        ServiceError::InvalidDefinition(format!(
            "could not inspect MSC service helper {}: {error}",
            helper.display()
        ))
    })?;
    if !metadata.file_type().is_file()
        || metadata.mode() & 0o111 == 0
        || metadata.mode() & 0o022 != 0
    {
        return Err(ServiceError::InvalidDefinition(format!(
            "MSC service helper must be a non-writable executable file: {}",
            helper.display()
        )));
    }
    if require_root_owner {
        let mut parent = helper.as_path();
        while let Some(path) = parent.parent() {
            let metadata = fs::symlink_metadata(path).map_err(|error| {
                ServiceError::InvalidDefinition(format!(
                    "could not inspect MSC service helper directory {}: {error}",
                    path.display()
                ))
            })?;
            if !metadata.file_type().is_dir() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0
            {
                return Err(ServiceError::InvalidDefinition(format!(
                    "MSC service helper directory must be system-owned and not writable by other users: {}",
                    path.display()
                )));
            }
            parent = path;
        }
        if metadata.uid() != 0 {
            return Err(ServiceError::InvalidDefinition(
                "MSC service helper executable must be system-owned before pkexec can run it"
                    .to_string(),
            ));
        }
    }
    Ok(helper)
}

fn validate_desktop_request(request: &ServiceInstallRequest, uid: u32) -> Result<(), ServiceError> {
    if request.service_name.as_str() != DESKTOP_AGENT_SERVICE_NAME {
        return invalid_desktop_request("service name is not the fixed MSC desktop service name");
    }
    if request.expected_port != 48001 || request.arguments != ["serve", "--bind", "127.0.0.1:48001"]
    {
        return invalid_desktop_request("agent command does not match the local MSC service");
    }
    let (user, home) = user_identity(uid)?;
    if request.run_user.as_deref() != Some(user.as_str()) {
        return invalid_desktop_request("service account does not match the authorizing user");
    }
    let data_dir = home.join(".local/share/msc2");
    let logs_dir = data_dir.join("logs");
    let secrets_dir = data_dir.join("secrets");
    let expected_bootstrap_socket = data_dir.join("local-bootstrap.sock");
    let canonical_data = canonical_owned_directory(&data_dir, uid, "MSC data directory", &home)?;
    canonical_owned_directory(&logs_dir, uid, "MSC log directory", &home)?;
    canonical_owned_directory(&secrets_dir, uid, "MSC secrets directory", &home)?;
    if request.working_directory != canonical_data || request.log_path != logs_dir.join("agent.log")
    {
        return invalid_desktop_request(
            "service data paths must stay inside the installing user's MSC directory",
        );
    }
    let expected_environment = BTreeMap::from([
        ("HOME".to_string(), home.to_string_lossy().into_owned()),
        (
            "MSC2_DATA_DIR".to_string(),
            canonical_data.display().to_string(),
        ),
        (
            "MSC2_MACOS_SECRET_STORE_DIR".to_string(),
            secrets_dir.display().to_string(),
        ),
        (
            "MSC2_LOCAL_BOOTSTRAP_SOCKET".to_string(),
            expected_bootstrap_socket.display().to_string(),
        ),
    ]);
    if request.environment != expected_environment {
        return invalid_desktop_request(
            "service environment contains an unexpected path or setting",
        );
    }
    let binary = fs::canonicalize(&request.binary_path).map_err(|error| {
        ServiceError::InvalidDefinition(format!(
            "could not resolve the staged MSC agent executable {}: {error}",
            request.binary_path.display()
        ))
    })?;
    let builds_dir = canonical_data.join("agent/builds");
    if !binary.starts_with(&builds_dir) {
        return invalid_desktop_request(
            "agent executable must be inside the installing user's staged MSC builds",
        );
    }
    canonical_owned_directory(
        &canonical_data.join("agent"),
        uid,
        "MSC agent directory",
        &home,
    )?;
    canonical_owned_directory(&builds_dir, uid, "MSC agent builds directory", &home)?;
    let Some(build_directory) = binary.parent() else {
        return invalid_desktop_request("staged MSC executable has no build directory");
    };
    canonical_owned_directory(build_directory, uid, "MSC staged build directory", &home)?;
    if binary.file_name() != Some(std::ffi::OsStr::new("msc"))
        || build_directory.file_name().is_none_or(|name| {
            let digest = name.to_string_lossy();
            digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        || request.binary_path != binary
    {
        return invalid_desktop_request(
            "staged MSC executable path is not a content-addressed MSC build",
        );
    }
    validate_user_owned_executable(&binary, uid)?;
    Ok(())
}

fn canonical_owned_directory(
    path: &Path,
    uid: u32,
    label: &str,
    home: &Path,
) -> Result<PathBuf, ServiceError> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        ServiceError::InvalidDefinition(format!(
            "could not resolve {label} {}: {error}",
            path.display()
        ))
    })?;
    if canonical != path {
        return invalid_desktop_request(&format!(
            "{label} must not resolve through a symbolic link"
        ));
    }
    if !canonical.starts_with(home) {
        return invalid_desktop_request(&format!(
            "{label} must stay inside the installing user's home directory"
        ));
    }
    let mut directory = canonical.as_path();
    loop {
        let metadata = fs::symlink_metadata(directory).map_err(|error| {
            ServiceError::InvalidDefinition(format!(
                "could not inspect {label} {}: {error}",
                directory.display()
            ))
        })?;
        if !metadata.file_type().is_dir() || metadata.uid() != uid || metadata.mode() & 0o022 != 0 {
            return invalid_desktop_request(&format!(
                "{label} and its parent directories must be non-writable directories owned by the installing user"
            ));
        }
        if directory == home {
            break;
        }
        let Some(parent) = directory.parent() else {
            return invalid_desktop_request(
                "MSC data path does not descend from the installing user's home",
            );
        };
        if !parent.starts_with(home) {
            return invalid_desktop_request(
                "MSC data path does not descend from the installing user's home",
            );
        }
        directory = parent;
    }
    Ok(canonical)
}

fn validate_user_owned_executable(path: &Path, uid: u32) -> Result<(), ServiceError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        ServiceError::InvalidDefinition(format!(
            "could not inspect staged MSC executable {}: {error}",
            path.display()
        ))
    })?;
    if !metadata.file_type().is_file()
        || metadata.uid() != uid
        || metadata.mode() & 0o111 == 0
        || metadata.mode() & 0o022 != 0
    {
        return invalid_desktop_request(
            "staged MSC executable must be executable, owned by the installing user, and not writable by other users",
        );
    }
    Ok(())
}

fn user_identity(uid: u32) -> Result<(String, PathBuf), ServiceError> {
    let mut entry = std::mem::MaybeUninit::<libc::passwd>::uninit();
    let mut buffer = vec![0_u8; 16 * 1024];
    let mut result = std::ptr::null_mut();
    let status = unsafe {
        libc::getpwuid_r(
            uid,
            entry.as_mut_ptr(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            &mut result,
        )
    };
    if status != 0 || result.is_null() {
        return Err(ServiceError::InvalidDefinition(format!(
            "could not resolve authorizing user id {uid}"
        )));
    }
    let entry = unsafe { entry.assume_init() };
    let name = unsafe { std::ffi::CStr::from_ptr(entry.pw_name) }
        .to_string_lossy()
        .into_owned();
    let home = unsafe { std::ffi::CStr::from_ptr(entry.pw_dir) }
        .to_string_lossy()
        .into_owned();
    Ok((name, PathBuf::from(home)))
}

fn current_uid() -> Result<u32, ServiceError> {
    Ok(unsafe { libc::getuid() })
}

fn ensure_authorized_command_succeeded(output: Output) -> Result<(), ServiceError> {
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let message = match output.status.code() {
        Some(126) | Some(127) => "Elevation was cancelled or denied. Approve the MSC service change in the system prompt and try again.".to_string(),
        _ if detail.is_empty() => format!("Elevated MSC service helper failed with {}", output.status),
        _ => format!("Elevated MSC service helper failed: {detail}"),
    };
    Err(ServiceError::Platform(message))
}

fn invalid_desktop_request<T>(message: &str) -> Result<T, ServiceError> {
    Err(ServiceError::InvalidDefinition(message.to_string()))
}

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
