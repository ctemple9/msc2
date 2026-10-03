use clap::{Args, Subcommand, ValueEnum};
use msc_infrastructure::service::{
    ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceName, ServiceState,
    ServiceStatusReport,
};
use serde::Serialize;

use super::{CliError, CommonArgs};

const AGENT_SERVICE_NAME: &str = "com.ctemple.msc2.agent";

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AgentTarget {
    Agent,
}

#[derive(Debug, Clone, Copy)]
pub enum AgentAction {
    Start,
    Stop,
    Status,
    Enable,
    Disable,
}

pub fn run_agent(
    common: CommonArgs,
    _target: AgentTarget,
    action: AgentAction,
) -> Result<(), CliError> {
    let manager = service_manager()?;
    let service_name = ServiceName::new(AGENT_SERVICE_NAME);
    let current = manager
        .execute(ServiceManagerCommand::Status {
            service_name: service_name.clone(),
        })
        .map_err(service_error)?;
    if current.state == ServiceState::NotInstalled && !matches!(action, AgentAction::Status) {
        return Err(CliError::usage(
            "the local MSC agent service is not installed",
        ));
    }

    let report = match action {
        AgentAction::Status => current,
        AgentAction::Enable | AgentAction::Disable => {
            set_local_agent_boot_enabled(matches!(action, AgentAction::Enable))
                .map_err(service_error)?;
            current
        }
        AgentAction::Start if current.state == ServiceState::Running => current,
        AgentAction::Stop if current.state == ServiceState::Stopped => current,
        #[cfg(target_os = "macos")]
        AgentAction::Start => {
            msc_platform_macos::service::start_local_agent().map_err(service_error)?
        }
        #[cfg(target_os = "macos")]
        AgentAction::Stop => {
            msc_platform_macos::service::stop_local_agent().map_err(service_error)?
        }
        #[cfg(not(target_os = "macos"))]
        AgentAction::Start => manager
            .execute(ServiceManagerCommand::Start { service_name })
            .map_err(service_error)?,
        #[cfg(not(target_os = "macos"))]
        AgentAction::Stop => manager
            .execute(ServiceManagerCommand::Stop { service_name })
            .map_err(service_error)?,
    };
    let boot_enabled = if report.state == ServiceState::NotInstalled {
        None
    } else {
        Some(local_agent_boot_enabled().map_err(service_error)?)
    };
    let action_name = match action {
        AgentAction::Start => "start",
        AgentAction::Stop => "stop",
        AgentAction::Status => "status",
        AgentAction::Enable => "enable",
        AgentAction::Disable => "disable",
    };
    if common.json {
        let output = AgentServiceOutput {
            command: action_name,
            installed: report.state != ServiceState::NotInstalled,
            state: state_name(report.state),
            boot_enabled,
            pid: report.pid,
        };
        println!(
            "{}",
            serde_json::to_string(&output).map_err(|error| {
                CliError::internal(format!("failed to encode JSON output: {error}"))
            })?
        );
    } else {
        let state = match report.state {
            ServiceState::NotInstalled => "not installed",
            ServiceState::Stopped => "installed, stopped",
            ServiceState::Running => "installed, running",
        };
        let boot = match boot_enabled {
            Some(true) => ", starts at boot",
            Some(false) => ", disabled at boot",
            None => "",
        };
        println!("agent service: {state}{boot}");
    }
    Ok(())
}

fn service_error(error: msc_infrastructure::service::ServiceError) -> CliError {
    CliError::internal(format!("local agent service: {error}"))
}

fn state_name(state: ServiceState) -> &'static str {
    match state {
        ServiceState::NotInstalled => "notInstalled",
        ServiceState::Stopped => "stopped",
        ServiceState::Running => "running",
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentServiceOutput {
    command: &'static str,
    installed: bool,
    state: &'static str,
    boot_enabled: Option<bool>,
    pid: Option<u32>,
}

fn local_agent_boot_enabled() -> Result<bool, msc_infrastructure::service::ServiceError> {
    #[cfg(target_os = "linux")]
    {
        return msc_platform_linux::service::local_agent_boot_enabled();
    }
    #[cfg(target_os = "macos")]
    {
        return msc_platform_macos::service::local_agent_boot_enabled();
    }
    #[cfg(target_os = "windows")]
    {
        return msc_platform_windows::service::local_agent_boot_enabled();
    }
    #[allow(unreachable_code)]
    Err(msc_infrastructure::service::ServiceError::Unsupported(
        "unsupported platform".into(),
    ))
}

fn set_local_agent_boot_enabled(
    enabled: bool,
) -> Result<(), msc_infrastructure::service::ServiceError> {
    #[cfg(target_os = "linux")]
    {
        return msc_platform_linux::service::set_local_agent_boot_enabled(enabled);
    }
    #[cfg(target_os = "macos")]
    {
        return msc_platform_macos::service::set_local_agent_boot_enabled(enabled);
    }
    #[cfg(target_os = "windows")]
    {
        return msc_platform_windows::service::set_local_agent_boot_enabled(enabled);
    }
    #[allow(unreachable_code)]
    Err(msc_infrastructure::service::ServiceError::Unsupported(
        "unsupported platform".into(),
    ))
}

#[derive(Debug, Clone, Subcommand)]
pub enum ServiceCommand {
    /// Register the agent as a background service.
    Install(ServiceInstallArgs),
    /// Remove the background service registration.
    Uninstall(ServiceTargetArgs),
    /// Start the installed background service.
    Start(ServiceTargetArgs),
    /// Stop the installed background service.
    Stop(ServiceTargetArgs),
    /// Show whether the background service is installed or running.
    Status(ServiceTargetArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ServiceInstallArgs {
    /// Stable local service name, for example `msc-agent`.
    #[arg(long)]
    pub service_name: String,

    /// Path to the `msc` binary the service should launch.
    #[arg(long)]
    pub binary_path: String,

    /// Directory the service should treat as its current working directory.
    #[arg(long)]
    pub working_directory: String,

    /// Where the platform adapter should send the service's own stdout/stderr.
    #[arg(long)]
    pub log_path: String,

    /// User account the service should run as, if the platform supports it.
    #[arg(long)]
    pub run_user: Option<String>,

    /// TCP port the service should expose the loopback management API on.
    #[arg(long)]
    pub expected_port: u16,

    /// Extra argument passed to the agent binary. Repeat for multiple values.
    #[arg(long = "arg")]
    pub arguments: Vec<String>,

    /// Extra environment entry in `KEY=VALUE` form. Repeat for multiple values.
    #[arg(long = "env", value_name = "KEY=VALUE")]
    pub environment: Vec<String>,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceTargetArgs {
    /// Stable local service name, for example `msc-agent`.
    #[arg(long)]
    pub service_name: String,
}

pub async fn run(common: CommonArgs, command: ServiceCommand) -> Result<(), CliError> {
    let manager = service_manager()?;
    run_with_manager(common, command, manager.as_ref()).await
}

pub(crate) async fn run_with_manager(
    common: CommonArgs,
    command: ServiceCommand,
    manager: &dyn ServiceManager,
) -> Result<(), CliError> {
    let model = into_model(command)?;
    let command = describe_command(&model);
    let report = manager
        .execute(model)
        .map_err(|error| CliError::internal(format!("service management failed: {error}")))?;

    if common.json {
        let output = ServiceCommandOutput::from_report(command, &report);
        let rendering = serde_json::to_string(&output)
            .map_err(|err| CliError::internal(format!("failed to encode JSON output: {err}")))?;
        println!("{rendering}");
    } else {
        println!("{command}: {}", describe_report(&report));
    }

    Ok(())
}

fn service_manager() -> Result<Box<dyn ServiceManager>, CliError> {
    #[cfg(target_os = "macos")]
    {
        return Ok(Box::new(
            msc_platform_macos::service::MacosLaunchdServiceManager::new(),
        ));
    }
    #[cfg(target_os = "windows")]
    {
        return Ok(Box::new(
            msc_platform_windows::service::WindowsServiceManager::new(),
        ));
    }
    #[cfg(target_os = "linux")]
    {
        return Ok(Box::new(
            msc_platform_linux::service::LinuxSystemdServiceManager::new(),
        ));
    }
    #[allow(unreachable_code)]
    Err(CliError::internal(
        "this platform has no MSC service manager",
    ))
}

/// Returns the data directory configured for the installed local agent.
/// Unix CLI authorization sockets live beside that agent's state, which may
/// be outside the CLI process's default directory.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(crate) fn installed_agent_data_dir() -> Result<Option<std::path::PathBuf>, CliError> {
    let report = service_manager()?
        .execute(ServiceManagerCommand::Status {
            service_name: ServiceName::new(AGENT_SERVICE_NAME),
        })
        .map_err(|error| CliError::internal(format!("service management failed: {error}")))?;
    Ok(report
        .definition
        .and_then(|definition| definition.environment.get("MSC2_DATA_DIR").cloned())
        .map(std::path::PathBuf::from))
}

fn into_model(command: ServiceCommand) -> Result<ServiceManagerCommand, CliError> {
    match command {
        ServiceCommand::Install(args) => Ok(ServiceManagerCommand::Install(args.into_request()?)),
        ServiceCommand::Uninstall(args) => Ok(ServiceManagerCommand::Uninstall {
            service_name: ServiceName::new(args.service_name),
        }),
        ServiceCommand::Start(args) => Ok(ServiceManagerCommand::Start {
            service_name: ServiceName::new(args.service_name),
        }),
        ServiceCommand::Stop(args) => Ok(ServiceManagerCommand::Stop {
            service_name: ServiceName::new(args.service_name),
        }),
        ServiceCommand::Status(args) => Ok(ServiceManagerCommand::Status {
            service_name: ServiceName::new(args.service_name),
        }),
    }
}

impl ServiceInstallArgs {
    fn into_request(self) -> Result<ServiceInstallRequest, CliError> {
        let mut request = ServiceInstallRequest::new(
            self.service_name,
            self.binary_path,
            self.working_directory,
            self.log_path,
            self.expected_port,
        )
        .args(self.arguments);
        if let Some(run_user) = self.run_user {
            request = request.run_user(run_user);
        }
        for entry in self.environment {
            let (key, value) = parse_env_entry(&entry)?;
            request = request.env(key, value);
        }
        Ok(request)
    }
}

fn parse_env_entry(entry: &str) -> Result<(String, String), CliError> {
    let (key, value) = entry.split_once('=').ok_or_else(|| {
        CliError::usage(format!(
            "invalid --env {entry:?}; expected KEY=VALUE so the service model stays explicit"
        ))
    })?;
    if key.trim().is_empty() {
        return Err(CliError::usage(format!(
            "invalid --env {entry:?}; the key cannot be empty"
        )));
    }
    Ok((key.to_string(), value.to_string()))
}

fn describe_command(command: &ServiceManagerCommand) -> String {
    match command {
        ServiceManagerCommand::Install(request) => format!(
            "install {} -> {} (cwd {}, port {})",
            request.service_name.as_str(),
            request.binary_path.display(),
            request.working_directory.display(),
            request.expected_port
        ),
        ServiceManagerCommand::Uninstall { service_name } => {
            format!("uninstall {}", service_name.as_str())
        }
        ServiceManagerCommand::Start { service_name } => format!("start {}", service_name.as_str()),
        ServiceManagerCommand::Stop { service_name } => format!("stop {}", service_name.as_str()),
        ServiceManagerCommand::Status { service_name } => {
            format!("status {}", service_name.as_str())
        }
    }
}

fn describe_report(report: &ServiceStatusReport) -> String {
    match report.state {
        ServiceState::NotInstalled => format!("{} is not installed", report.service_name.as_str()),
        ServiceState::Stopped => {
            format!("{} is installed and stopped", report.service_name.as_str())
        }
        ServiceState::Running => format!(
            "{} is running{}",
            report.service_name.as_str(),
            report
                .pid
                .map(|pid| format!(" (pid {pid})"))
                .unwrap_or_default()
        ),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceCommandOutput {
    command: String,
    service_name: String,
    state: &'static str,
    pid: Option<u32>,
}

impl ServiceCommandOutput {
    fn from_report(command: String, report: &ServiceStatusReport) -> Self {
        Self {
            command,
            service_name: report.service_name.as_str().to_string(),
            state: match report.state {
                ServiceState::NotInstalled => "notInstalled",
                ServiceState::Stopped => "stopped",
                ServiceState::Running => "running",
            },
            pid: report.pid,
        }
    }
}
