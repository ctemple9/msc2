//! Shared player and allowlist routes for an active Bedrock server.
//!
//! These are deliberately `/v1/players` and `/v1/allowlist`, not a parallel
//! `/v1/bedrock/...` API.  File reads remain useful when the selected backend
//! is unavailable; mutations still use the same permission categories as the
//! frozen contract.

use std::path::Path;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{BedrockRuntimeStateDto, PermissionCategoryDto};
use msc_application::bedrock_players;
use serde::{Deserialize, Serialize};

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{
    LifecycleRoutesState, error_response, invalid_body, require_permission,
};

pub(crate) fn runtime_for(state: &LifecycleRoutesState) -> Option<BedrockRuntimeStateDto> {
    state
        .active_config_server()
        .filter(|server| server.server_type == msc_domain::identity::ServerType::Bedrock)
        .map(|_| state.bedrock_runtime_state())
}

pub(crate) fn require_runtime(state: &LifecycleRoutesState) -> Option<Response> {
    let runtime = runtime_for(state)?;
    (runtime.state != "available").then(|| {
        (
            StatusCode::CONFLICT,
            Json(runtime.capability_unavailable_error()),
        )
            .into_response()
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayerDto {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    uuid: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlayersResponse {
    players: Vec<PlayerDto>,
    count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime: Option<BedrockRuntimeStateDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AllowlistEntryDto {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    xuid: Option<String>,
    ignores_player_limit: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AllowlistResponse {
    server_type: String,
    entries: Vec<AllowlistEntryDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime: Option<BedrockRuntimeStateDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AllowlistMutationRequest {
    action: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AllowlistMutationResult {
    success: bool,
    message: String,
    server_type: String,
    entries: Vec<AllowlistEntryDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime: Option<BedrockRuntimeStateDto>,
}

pub async fn players(State(state): State<LifecycleRoutesState>) -> Response {
    let Some(server) = state.active_config_server() else {
        return Json(PlayersResponse {
            players: Vec::new(),
            count: 0,
            note: Some("no_active_server".to_owned()),
            runtime: runtime_for(&state),
        })
        .into_response();
    };
    if server.server_type != msc_domain::identity::ServerType::Bedrock {
        return Json(PlayersResponse {
            players: Vec::new(),
            count: 0,
            note: Some("not_bedrock".to_owned()),
            runtime: None,
        })
        .into_response();
    }
    let settings = msc_application::bedrock_settings::load(
        &msc_infrastructure::fs::StdFileSystem,
        Path::new(&server.server_dir),
    );
    let cache = bedrock_players::load_name_cache(
        &msc_infrastructure::fs::StdFileSystem,
        Path::new(&server.server_dir),
    );
    match bedrock_players::discover_players(
        Path::new(&server.server_dir),
        &settings.model.level_name,
        &cache,
    ) {
        Ok(players) => Json(PlayersResponse {
            count: players.len(),
            players: players
                .into_iter()
                .map(|player| PlayerDto {
                    name: player.name,
                    uuid: Some(player.xuid),
                })
                .collect(),
            note: None,
            runtime: runtime_for(&state),
        })
        .into_response(),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "player_data_unavailable",
            &error.to_string(),
        ),
    }
}

pub async fn get_allowlist(State(state): State<LifecycleRoutesState>) -> Response {
    let Some(server) = state.active_config_server() else {
        return error_response(
            StatusCode::CONFLICT,
            "conflict",
            "No server is currently active.",
        );
    };
    if server.server_type != msc_domain::identity::ServerType::Bedrock {
        return error_response(
            StatusCode::CONFLICT,
            "not_bedrock",
            "The active server is not Bedrock.",
        );
    }
    let entries = bedrock_players::read_allowlist(
        &msc_infrastructure::fs::StdFileSystem,
        Path::new(&server.server_dir),
    );
    Json(AllowlistResponse {
        server_type: "bedrock".to_owned(),
        entries: to_allowlist_dtos(entries),
        runtime: runtime_for(&state),
    })
    .into_response()
}

pub async fn mutate_allowlist(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<AllowlistMutationRequest>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Players) {
        return response;
    }
    let Some(server) = state.active_config_server() else {
        return error_response(
            StatusCode::CONFLICT,
            "conflict",
            "No server is currently active.",
        );
    };
    if server.server_type != msc_domain::identity::ServerType::Bedrock {
        return error_response(
            StatusCode::CONFLICT,
            "not_bedrock",
            "The active server is not Bedrock.",
        );
    }
    // The file-backed allowlist remains editable while BDS is unavailable;
    // the next start reads the same durable file.  Live-dependent Bedrock
    // operations use `require_runtime` at their route boundary instead.
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let Some(action) = body.action.filter(|value| !value.trim().is_empty()) else {
        return invalid_body("missing_action", "action is required.");
    };
    let Some(name) = body.name.filter(|value| !value.trim().is_empty()) else {
        return invalid_body("missing_name", "name is required.");
    };
    match bedrock_players::mutate_allowlist(
        &msc_infrastructure::fs::StdFileSystem,
        Path::new(&server.server_dir),
        &action,
        &name,
    ) {
        Ok(entries) => Json(AllowlistMutationResult {
            success: true,
            message: action,
            server_type: "bedrock".to_owned(),
            entries: to_allowlist_dtos(entries),
            runtime: runtime_for(&state),
        })
        .into_response(),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        ),
    }
}

fn to_allowlist_dtos(entries: Vec<msc_domain::bedrock::AllowlistEntry>) -> Vec<AllowlistEntryDto> {
    entries
        .into_iter()
        .map(|entry| AllowlistEntryDto {
            name: entry.name,
            xuid: entry.xuid,
            ignores_player_limit: entry.ignores_player_limit,
        })
        .collect()
}
