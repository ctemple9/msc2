use clap::{Args, Subcommand};
use serde::Serialize;

use super::{CliError, CommonArgs};
#[cfg(not(test))]
use crate::auth::AuthState;

#[derive(Debug, Clone, Subcommand)]
pub enum PairingCommand {
    /// Create a one-use recovery code on this host.
    Create(CreatePairingArgs),
}

#[derive(Debug, Clone, Args)]
pub struct CreatePairingArgs {
    /// The supported client that will redeem the code.
    #[arg(long, default_value = "desktop", value_parser = ["desktop"])]
    pub client_kind: String,

    /// A label recorded with the newly-issued administrator credential.
    #[arg(long, default_value = "recovery-client")]
    pub label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct PairingOutput {
    // This stdout shape is consumed by the Tauri remote-pairing bootstrap.
    // Keep it JSON-only when --json is requested so SSH can parse it safely.
    pairing_code: String,
    agent_host_id: String,
    client_kind: String,
    expires_at: String,
}

pub fn run(common: CommonArgs, command: PairingCommand) -> Result<(), CliError> {
    #[cfg(test)]
    {
        let _ = (common, command);
        Err(CliError::internal(
            "pairing is only available in the agent binary",
        ))
    }
    #[cfg(not(test))]
    {
        match command {
            PairingCommand::Create(args) => {
                let auth = pairing_auth()?;
                let pairing = auth
                    .create_host_local_pairing(args.label)
                    .map_err(|error| CliError::internal(error.to_string()))?;
                let output = PairingOutput {
                    pairing_code: pairing.pairing_code,
                    agent_host_id: pairing.agent_host_id,
                    client_kind: args.client_kind,
                    expires_at: pairing
                        .expires_at
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .to_string(),
                };
                if common.json {
                    println!(
                        "{}",
                        serde_json::to_string(&output)
                            .map_err(|error| CliError::internal(error.to_string()))?
                    );
                } else {
                    println!("pairing code: {}", output.pairing_code);
                    println!("agent host id: {}", output.agent_host_id);
                    println!("expires at (unix seconds): {}", output.expires_at);
                }
                Ok(())
            }
        }
    }
}

#[cfg(not(test))]
fn pairing_auth() -> Result<AuthState, CliError> {
    // Pairing only writes a challenge. Loading/migrating the credential registry
    // here is unnecessary and can also contaminate the SSH JSON response.
    #[cfg(target_os = "macos")]
    if !std::env::var_os("MSC2_MACOS_SECRET_STORE_DIR").is_some_and(|value| !value.is_empty())
        && !std::env::var_os("MSC2_DATA_DIR").is_some_and(|value| !value.is_empty())
    {
        use msc_infrastructure::service::{ServiceManager, ServiceManagerCommand, ServiceName};
        let report = msc_platform_macos::service::MacosLaunchdServiceManager::new()
            .execute(ServiceManagerCommand::Status {
                service_name: ServiceName::new("com.ctemple.msc2.agent"),
            })
            .map_err(|error| CliError::internal(error.to_string()))?;
        if let Some(definition) = report.definition {
            let directory = definition
                .environment
                .get("MSC2_MACOS_SECRET_STORE_DIR")
                .filter(|value| !value.is_empty())
                .map(std::path::PathBuf::from)
                .or_else(|| {
                    definition
                        .environment
                        .get("MSC2_DATA_DIR")
                        .filter(|value| !value.is_empty())
                        .map(|value| std::path::PathBuf::from(value).join("secrets"))
                });
            if let Some(directory) = directory {
                let store =
                    msc_platform_macos::secret_store::MacosSecretStore::at_directory(directory)
                        .map_err(|error| CliError::internal(error.to_string()))?;
                return Ok(AuthState::new(std::sync::Arc::new(store)));
            }
        }
    }
    Ok(AuthState::new(
        crate::auth::production_secret_store()
            .map_err(|error| CliError::internal(error.to_string()))?,
    ))
}
