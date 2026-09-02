//! In-memory host/server selection shared by CLI commands and the TUI.
//!
//! Selecting a server still means selecting the agent's active server through
//! the existing HTTP route. This module does not cache credentials or invent a
//! local management state store.

use msc_api::dto::{ActiveServerRequestDto, ServerDto, SimpleResultDto};

use super::transport::SharedClient;
use crate::cli::CliError;

pub(crate) async fn ensure_active_server(
    client: &SharedClient,
    selector: Option<&str>,
) -> Result<ServerDto, CliError> {
    let selector = selector
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CliError::usage("server selector cannot be empty"))?;
    let server = resolve_server(client, selector).await?;
    let body = ActiveServerRequestDto {
        server_id: Some(server.id.clone()),
    };
    let _: SimpleResultDto = client.post_json("/v1/active-server", &body).await?;
    Ok(server)
}

pub(crate) async fn resolve_server(
    client: &SharedClient,
    selector: &str,
) -> Result<ServerDto, CliError> {
    let servers: Vec<ServerDto> = client.get_json("/v1/servers").await?;
    if servers.is_empty() {
        return Err(CliError::usage("the agent reports no imported servers"));
    }

    if let Some(server) = servers.iter().find(|server| server.id == selector) {
        return Ok(server.clone());
    }

    let exact_name_matches: Vec<&ServerDto> = servers
        .iter()
        .filter(|server| server.name == selector)
        .collect();
    if exact_name_matches.len() == 1 {
        return Ok(exact_name_matches[0].clone());
    }
    if exact_name_matches.len() > 1 {
        return Err(CliError::usage(format!(
            "multiple servers are named {selector:?}; use the server id instead"
        )));
    }

    let folded = selector.to_ascii_lowercase();
    let folded_matches: Vec<&ServerDto> = servers
        .iter()
        .filter(|server| server.name.to_ascii_lowercase() == folded)
        .collect();
    if folded_matches.len() == 1 {
        return Ok(folded_matches[0].clone());
    }
    if folded_matches.len() > 1 {
        return Err(CliError::usage(format!(
            "multiple servers match {selector:?}; use the server id instead"
        )));
    }

    Err(CliError::usage(format!(
        "no imported server matched {selector:?}"
    )))
}
