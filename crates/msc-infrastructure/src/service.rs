//! Cross-platform service-management contract for the agent.
//!
//! P4.21 defines the shared model the platform crates will implement in the
//! next steps. The CLI can already describe the install/start/stop/status
//! requests without knowing whether the current host uses launchd, systemd, or
//! the Windows service manager.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceName(String);

impl ServiceName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInstallRequest {
    pub service_name: ServiceName,
    pub binary_path: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: BTreeMap<String, String>,
    pub log_path: PathBuf,
    pub run_user: Option<String>,
    pub expected_port: u16,
}

impl ServiceInstallRequest {
    pub fn new(
        service_name: impl Into<String>,
        binary_path: impl Into<PathBuf>,
        working_directory: impl Into<PathBuf>,
        log_path: impl Into<PathBuf>,
        expected_port: u16,
    ) -> Self {
        Self {
            service_name: ServiceName::new(service_name),
            binary_path: binary_path.into(),
            arguments: Vec::new(),
            working_directory: working_directory.into(),
            environment: BTreeMap::new(),
            log_path: log_path.into(),
            run_user: None,
            expected_port,
        }
    }

    pub fn arg(mut self, argument: impl Into<String>) -> Self {
        self.arguments.push(argument.into());
        self
    }

    pub fn args(mut self, arguments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.arguments.extend(arguments.into_iter().map(Into::into));
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }

    pub fn run_user(mut self, run_user: impl Into<String>) -> Self {
        self.run_user = Some(run_user.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceManagerCommand {
    Install(ServiceInstallRequest),
    Uninstall { service_name: ServiceName },
    Start { service_name: ServiceName },
    Stop { service_name: ServiceName },
    Status { service_name: ServiceName },
}

impl ServiceManagerCommand {
    pub fn service_name(&self) -> &ServiceName {
        match self {
            Self::Install(request) => &request.service_name,
            Self::Uninstall { service_name }
            | Self::Start { service_name }
            | Self::Stop { service_name }
            | Self::Status { service_name } => service_name,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    NotInstalled,
    Stopped,
    Running,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceStatusReport {
    pub service_name: ServiceName,
    pub state: ServiceState,
    pub pid: Option<u32>,
    pub definition: Option<ServiceInstallRequest>,
}

impl ServiceStatusReport {
    pub fn not_installed(service_name: impl Into<String>) -> Self {
        Self {
            service_name: ServiceName::new(service_name),
            state: ServiceState::NotInstalled,
            pid: None,
            definition: None,
        }
    }

    pub fn stopped(definition: ServiceInstallRequest) -> Self {
        Self {
            service_name: definition.service_name.clone(),
            state: ServiceState::Stopped,
            pid: None,
            definition: Some(definition),
        }
    }

    pub fn running(definition: ServiceInstallRequest, pid: u32) -> Self {
        Self {
            service_name: definition.service_name.clone(),
            state: ServiceState::Running,
            pid: Some(pid),
            definition: Some(definition),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    Unsupported(String),
    NotInstalled(ServiceName),
    InvalidDefinition(String),
    Platform(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(message)
            | Self::InvalidDefinition(message)
            | Self::Platform(message) => f.write_str(message),
            Self::NotInstalled(service_name) => {
                write!(f, "service is not installed: {}", service_name.as_str())
            }
        }
    }
}

impl std::error::Error for ServiceError {}

pub trait ServiceManager: Send + Sync {
    fn execute(&self, command: ServiceManagerCommand) -> Result<ServiceStatusReport, ServiceError>;
}

#[derive(Debug, Default)]
pub struct FakeServiceManager {
    state: Mutex<FakeServiceState>,
}

#[derive(Debug, Default)]
struct FakeServiceState {
    commands: Vec<ServiceManagerCommand>,
    installed: BTreeMap<ServiceName, ServiceInstallRequest>,
    running: BTreeMap<ServiceName, u32>,
    next_pid: u32,
    fail_next: Option<ServiceError>,
}

impl FakeServiceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commands(&self) -> Vec<ServiceManagerCommand> {
        self.state.lock().unwrap().commands.clone()
    }

    pub fn fail_next(&self, error: ServiceError) {
        self.state.lock().unwrap().fail_next = Some(error);
    }
}

impl ServiceManager for FakeServiceManager {
    fn execute(&self, command: ServiceManagerCommand) -> Result<ServiceStatusReport, ServiceError> {
        let mut state = self.state.lock().unwrap();
        state.commands.push(command.clone());

        if let Some(error) = state.fail_next.take() {
            return Err(error);
        }

        match command {
            ServiceManagerCommand::Install(request) => {
                let service_name = request.service_name.clone();
                state.running.remove(&service_name);
                state.installed.insert(service_name, request.clone());
                Ok(ServiceStatusReport::stopped(request))
            }
            ServiceManagerCommand::Uninstall { service_name } => {
                state.installed.remove(&service_name);
                state.running.remove(&service_name);
                Ok(ServiceStatusReport::not_installed(
                    service_name.as_str().to_string(),
                ))
            }
            ServiceManagerCommand::Start { service_name } => {
                let definition = state
                    .installed
                    .get(&service_name)
                    .cloned()
                    .ok_or_else(|| ServiceError::NotInstalled(service_name.clone()))?;
                let pid = if let Some(pid) = state.running.get(&service_name) {
                    *pid
                } else {
                    let pid = if state.next_pid == 0 {
                        1000
                    } else {
                        state.next_pid
                    };
                    state.next_pid = pid + 1;
                    state.running.insert(service_name, pid);
                    pid
                };
                Ok(ServiceStatusReport::running(definition, pid))
            }
            ServiceManagerCommand::Stop { service_name } => {
                let definition = state
                    .installed
                    .get(&service_name)
                    .cloned()
                    .ok_or_else(|| ServiceError::NotInstalled(service_name.clone()))?;
                state.running.remove(&service_name);
                Ok(ServiceStatusReport::stopped(definition))
            }
            ServiceManagerCommand::Status { service_name } => {
                let Some(definition) = state.installed.get(&service_name).cloned() else {
                    return Ok(ServiceStatusReport::not_installed(
                        service_name.as_str().to_string(),
                    ));
                };
                if let Some(pid) = state.running.get(&service_name) {
                    Ok(ServiceStatusReport::running(definition, *pid))
                } else {
                    Ok(ServiceStatusReport::stopped(definition))
                }
            }
        }
    }
}
