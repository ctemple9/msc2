use clap::{Args, Subcommand};
use msc_infrastructure::service::{ServiceInstallRequest, ServiceManagerCommand, ServiceName};

use super::{CliError, CommonArgs};

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
    let model = into_model(command)?;
    let rendering = if common.json {
        serde_json::to_string(&describe_command(&model))
            .map_err(|err| CliError::internal(format!("failed to encode JSON output: {err}")))?
    } else {
        describe_command(&model)
    };

    Err(CliError::internal(format!(
        "service management is modeled but not executable yet ({rendering}); platform adapters land in P4.22-P4.24"
    )))
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
