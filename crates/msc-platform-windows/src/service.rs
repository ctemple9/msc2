//! Windows Service management for the headless agent.
//!
//! Phase 4's Windows proof registers the agent with the Service Control
//! Manager under the installing user's account, not `LocalSystem`
//! (`docs/msc2/substrate/service-identity.md` §2). Windows does not expose
//! rich service metadata back out through one query call, so this module
//! stores the shared `ServiceInstallRequest` beside the installed service and
//! reconstructs it for `status`.

use msc_infrastructure::service::{
    ServiceError, ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceName,
    ServiceState, ServiceStatusReport,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PASSWORD_ENV: &str = "MSC2_WINDOWS_SERVICE_PASSWORD";

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemSc;

pub trait Sc: Send + Sync {
    fn create(
        &self,
        service_name: &str,
        bin_path: &str,
        run_user: &str,
        password: Option<&str>,
    ) -> Result<(), ServiceError>;
    fn delete(&self, service_name: &str) -> Result<(), ServiceError>;
    fn start(&self, service_name: &str) -> Result<(), ServiceError>;
    fn stop(&self, service_name: &str) -> Result<(), ServiceError>;
    fn query(&self, service_name: &str) -> Result<String, ServiceError>;
}

impl Sc for SystemSc {
    fn create(
        &self,
        service_name: &str,
        bin_path: &str,
        run_user: &str,
        password: Option<&str>,
    ) -> Result<(), ServiceError> {
        let mut args = vec![
            "create".to_string(),
            service_name.to_string(),
            "start=".to_string(),
            "auto".to_string(),
            "type=".to_string(),
            "own".to_string(),
            "binPath=".to_string(),
            bin_path.to_string(),
            "obj=".to_string(),
            run_user.to_string(),
        ];
        if let Some(password) = password {
            args.push("password=".to_string());
            args.push(password.to_string());
        }
        run_sc(&args).map(|_| ())
    }

    fn delete(&self, service_name: &str) -> Result<(), ServiceError> {
        run_sc(&["delete".to_string(), service_name.to_string()]).map(|_| ())
    }

    fn start(&self, service_name: &str) -> Result<(), ServiceError> {
        run_sc(&["start".to_string(), service_name.to_string()]).map(|_| ())
    }

    fn stop(&self, service_name: &str) -> Result<(), ServiceError> {
        run_sc(&["stop".to_string(), service_name.to_string()]).map(|_| ())
    }

    fn query(&self, service_name: &str) -> Result<String, ServiceError> {
        run_sc(&["queryex".to_string(), service_name.to_string()])
    }
}

#[derive(Debug)]
pub struct WindowsServiceManager<S = SystemSc> {
    metadata_root: PathBuf,
    sc: S,
}

impl WindowsServiceManager<SystemSc> {
    pub fn new() -> Self {
        Self {
            metadata_root: PathBuf::from(r"C:\ProgramData\MSC2\Services"),
            sc: SystemSc,
        }
    }
}

impl Default for WindowsServiceManager<SystemSc> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> WindowsServiceManager<S> {
    pub fn with_sc(metadata_root: impl Into<PathBuf>, sc: S) -> Self {
        Self {
            metadata_root: metadata_root.into(),
            sc,
        }
    }
}

impl<S: Sc> ServiceManager for WindowsServiceManager<S> {
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

impl<S: Sc> WindowsServiceManager<S> {
    fn install(&self, request: ServiceInstallRequest) -> Result<ServiceStatusReport, ServiceError> {
        validate_request(&request)?;
        let metadata_path = self.metadata_path(request.service_name.as_str());

        if let Some(parent) = metadata_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                ServiceError::Platform(format!(
                    "creating Windows service metadata directory {}: {err}",
                    parent.display()
                ))
            })?;
        }

        if let Some(parent) = request.log_path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                ServiceError::Platform(format!(
                    "creating Windows service log directory {}: {err}",
                    parent.display()
                ))
            })?;
        }

        let _ = self.sc.stop(request.service_name.as_str());
        let _ = self.sc.delete(request.service_name.as_str());

        let password = std::env::var(PASSWORD_ENV).ok();
        let bin_path = render_bin_path(&request);
        self.sc.create(
            request.service_name.as_str(),
            &bin_path,
            request.run_user.as_deref().unwrap_or_default(),
            password.as_deref(),
        )?;
        fs::write(&metadata_path, render_metadata(&request)).map_err(|err| {
            ServiceError::Platform(format!(
                "writing Windows service metadata {}: {err}",
                metadata_path.display()
            ))
        })?;
        Ok(ServiceStatusReport::stopped(request))
    }

    fn uninstall(&self, service_name: &ServiceName) -> Result<ServiceStatusReport, ServiceError> {
        let metadata_path = self.metadata_path(service_name.as_str());
        if !metadata_path.exists() {
            return Ok(ServiceStatusReport::not_installed(
                service_name.as_str().to_string(),
            ));
        }

        let _ = self.sc.stop(service_name.as_str());
        let _ = self.sc.delete(service_name.as_str());
        fs::remove_file(&metadata_path).map_err(|err| {
            ServiceError::Platform(format!(
                "removing Windows service metadata {}: {err}",
                metadata_path.display()
            ))
        })?;
        Ok(ServiceStatusReport::not_installed(
            service_name.as_str().to_string(),
        ))
    }

    fn start(&self, service_name: &ServiceName) -> Result<ServiceStatusReport, ServiceError> {
        self.require_installed(service_name)?;
        self.sc.start(service_name.as_str())?;
        self.status(service_name)
    }

    fn stop(&self, service_name: &ServiceName) -> Result<ServiceStatusReport, ServiceError> {
        let definition = self.read_request(service_name)?;
        self.sc.stop(service_name.as_str())?;
        Ok(ServiceStatusReport::stopped(definition))
    }

    fn status(&self, service_name: &ServiceName) -> Result<ServiceStatusReport, ServiceError> {
        let metadata_path = self.metadata_path(service_name.as_str());
        if !metadata_path.exists() {
            return Ok(ServiceStatusReport::not_installed(
                service_name.as_str().to_string(),
            ));
        }

        let definition = self.read_request(service_name)?;
        let output = self.sc.query(service_name.as_str())?;
        let state = parse_state(&output);
        let pid = parse_pid(&output);
        Ok(match (state, pid) {
            (Some(ServiceState::Running), Some(pid)) if pid > 0 => {
                ServiceStatusReport::running(definition, pid)
            }
            _ => ServiceStatusReport::stopped(definition),
        })
    }

    fn require_installed(&self, service_name: &ServiceName) -> Result<PathBuf, ServiceError> {
        let metadata_path = self.metadata_path(service_name.as_str());
        if metadata_path.exists() {
            Ok(metadata_path)
        } else {
            Err(ServiceError::NotInstalled(service_name.clone()))
        }
    }

    fn metadata_path(&self, service_name: &str) -> PathBuf {
        self.metadata_root.join(format!("{service_name}.metadata"))
    }

    fn read_request(
        &self,
        service_name: &ServiceName,
    ) -> Result<ServiceInstallRequest, ServiceError> {
        let metadata_path = self.require_installed(service_name)?;
        let contents = fs::read_to_string(&metadata_path).map_err(|err| {
            ServiceError::Platform(format!(
                "reading Windows service metadata {}: {err}",
                metadata_path.display()
            ))
        })?;
        parse_metadata(&contents)
    }
}

fn validate_request(request: &ServiceInstallRequest) -> Result<(), ServiceError> {
    if request.service_name.as_str().trim().is_empty() {
        return Err(ServiceError::InvalidDefinition(
            "Windows service name cannot be empty".to_string(),
        ));
    }
    if request.service_name.as_str().contains(['\\', '/']) {
        return Err(ServiceError::InvalidDefinition(format!(
            "Windows service name must be a label, not a path: {}",
            request.service_name.as_str()
        )));
    }
    if request
        .run_user
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Err(ServiceError::InvalidDefinition(
            "Windows service install requires run_user so Log On As matches the installing user"
                .to_string(),
        ));
    }
    if !is_windows_absolute_path(&request.binary_path) {
        return Err(ServiceError::InvalidDefinition(format!(
            "Windows service binary path must be absolute: {}",
            request.binary_path.display()
        )));
    }
    if !is_windows_absolute_path(&request.working_directory) {
        return Err(ServiceError::InvalidDefinition(format!(
            "Windows service working directory must be absolute: {}",
            request.working_directory.display()
        )));
    }
    if !is_windows_absolute_path(&request.log_path) {
        return Err(ServiceError::InvalidDefinition(format!(
            "Windows service log path must be absolute: {}",
            request.log_path.display()
        )));
    }
    Ok(())
}

fn is_windows_absolute_path(path: &Path) -> bool {
    let text = path.as_os_str().to_string_lossy();
    if text.starts_with(r"\\") {
        return true;
    }

    let bytes = text.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn render_bin_path(request: &ServiceInstallRequest) -> String {
    std::iter::once(
        request
            .binary_path
            .as_os_str()
            .to_string_lossy()
            .to_string(),
    )
    .chain(request.arguments.iter().cloned())
    .map(|arg| quote_windows_argument(&arg))
    .collect::<Vec<_>>()
    .join(" ")
}

fn render_metadata(request: &ServiceInstallRequest) -> String {
    let arguments = request.arguments.join("\n");
    let environment = request
        .environment
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");
    let run_user = request.run_user.as_deref().unwrap_or_default();

    [
        ("service_name", request.service_name.as_str().to_string()),
        ("binary_path", request.binary_path.display().to_string()),
        (
            "working_directory",
            request.working_directory.display().to_string(),
        ),
        ("log_path", request.log_path.display().to_string()),
        ("expected_port", request.expected_port.to_string()),
        ("run_user", run_user.to_string()),
        ("arguments_hex", encode_hex(arguments.as_bytes())),
        ("environment_hex", encode_hex(environment.as_bytes())),
    ]
    .into_iter()
    .map(|(key, value)| format!("{key}={value}"))
    .collect::<Vec<_>>()
    .join("\n")
        + "\n"
}

fn parse_metadata(contents: &str) -> Result<ServiceInstallRequest, ServiceError> {
    let mut map = BTreeMap::new();
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let (key, value) = line.split_once('=').ok_or_else(|| {
            ServiceError::InvalidDefinition(format!(
                "Windows service metadata line is missing '=': {line}"
            ))
        })?;
        map.insert(key.to_string(), value.to_string());
    }

    let service_name = required_field(&map, "service_name")?;
    let binary_path = PathBuf::from(required_field(&map, "binary_path")?);
    let working_directory = PathBuf::from(required_field(&map, "working_directory")?);
    let log_path = PathBuf::from(required_field(&map, "log_path")?);
    let expected_port = required_field(&map, "expected_port")?
        .parse::<u16>()
        .map_err(|err| {
            ServiceError::InvalidDefinition(format!(
                "invalid Windows service expected_port metadata: {err}"
            ))
        })?;
    let run_user = required_field(&map, "run_user")?;
    let arguments = decode_hex(&required_field(&map, "arguments_hex")?)?;
    let environment = decode_hex(&required_field(&map, "environment_hex")?)?;

    let mut request = ServiceInstallRequest::new(
        service_name,
        binary_path,
        working_directory,
        log_path,
        expected_port,
    );
    for argument in arguments.lines() {
        if !argument.is_empty() {
            request = request.arg(argument.to_string());
        }
    }
    if !run_user.trim().is_empty() {
        request = request.run_user(run_user);
    }
    for entry in environment.lines() {
        if entry.is_empty() {
            continue;
        }
        let (key, value) = entry.split_once('=').ok_or_else(|| {
            ServiceError::InvalidDefinition(format!(
                "invalid Windows service environment metadata entry: {entry}"
            ))
        })?;
        request = request.env(key.to_string(), value.to_string());
    }
    Ok(request)
}

fn required_field(map: &BTreeMap<String, String>, key: &str) -> Result<String, ServiceError> {
    map.get(key).cloned().ok_or_else(|| {
        ServiceError::InvalidDefinition(format!("Windows service metadata is missing {key}"))
    })
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex(encoded: &str) -> Result<String, ServiceError> {
    if !encoded.len().is_multiple_of(2) {
        return Err(ServiceError::InvalidDefinition(
            "Windows service metadata hex value has odd length".to_string(),
        ));
    }

    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    let raw = encoded.as_bytes();
    for chunk in raw.as_chunks::<2>().0 {
        let text = std::str::from_utf8(chunk).map_err(|err| {
            ServiceError::InvalidDefinition(format!(
                "Windows service metadata hex is not valid UTF-8: {err}"
            ))
        })?;
        let byte = u8::from_str_radix(text, 16).map_err(|err| {
            ServiceError::InvalidDefinition(format!(
                "Windows service metadata hex decode failed: {err}"
            ))
        })?;
        bytes.push(byte);
    }
    String::from_utf8(bytes).map_err(|err| {
        ServiceError::InvalidDefinition(format!(
            "Windows service metadata is not valid UTF-8: {err}"
        ))
    })
}

fn parse_state(output: &str) -> Option<ServiceState> {
    output.lines().find_map(|line| {
        if !line.contains("STATE") {
            return None;
        }
        let upper = line.to_ascii_uppercase();
        if upper.contains("RUNNING") {
            Some(ServiceState::Running)
        } else if upper.contains("STOPPED") {
            Some(ServiceState::Stopped)
        } else {
            None
        }
    })
}

fn parse_pid(output: &str) -> Option<u32> {
    output.lines().find_map(|line| {
        let (_, value) = line.split_once(':')?;
        if !line.to_ascii_uppercase().contains("PID") {
            return None;
        }
        value.trim().parse::<u32>().ok()
    })
}

fn quote_windows_argument(argument: &str) -> String {
    if argument.is_empty() {
        return "\"\"".to_string();
    }
    if !argument.contains([' ', '\t', '"']) {
        return argument.to_string();
    }

    let mut rendered = String::from("\"");
    let mut backslashes = 0;
    for ch in argument.chars() {
        match ch {
            '\\' => backslashes += 1,
            '"' => {
                rendered.push_str(&"\\".repeat(backslashes * 2 + 1));
                rendered.push('"');
                backslashes = 0;
            }
            _ => {
                if backslashes > 0 {
                    rendered.push_str(&"\\".repeat(backslashes));
                    backslashes = 0;
                }
                rendered.push(ch);
            }
        }
    }
    if backslashes > 0 {
        rendered.push_str(&"\\".repeat(backslashes * 2));
    }
    rendered.push('"');
    rendered
}

fn run_sc(args: &[String]) -> Result<String, ServiceError> {
    let output = Command::new("sc.exe")
        .args(args)
        .output()
        .map_err(|err| ServiceError::Platform(format!("running sc.exe: {err}")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(ServiceError::Platform(format!(
            "sc.exe {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}
