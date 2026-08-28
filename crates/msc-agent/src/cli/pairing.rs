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
    /// The client that will redeem the code.
    #[arg(long, default_value = "desktop", value_parser = ["desktop", "browser"])]
    pub client_kind: String,

    /// A label recorded with the newly-issued administrator credential.
    #[arg(long, default_value = "recovery-client")]
    pub label: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct PairingOutput {
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
                let auth = AuthState::default_persistent_service_store();
                let pairing = auth
                    .create_host_local_pairing(&args.client_kind, args.label)
                    .map_err(CliError::internal)?;
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
