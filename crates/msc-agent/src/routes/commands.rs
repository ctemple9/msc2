//! `POST /v1/command`.

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::response::{IntoResponse, Response};
use msc_api::dto::{CommandRequestDto, CommandResultDto, PermissionCategoryDto};
use msc_application::commands::validate_api_command;

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{
    LifecycleRoutesState, invalid_body, lifecycle_error_response, require_permission,
};

pub async fn command(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<CommandRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::ServerControl) {
        return response;
    }

    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let command = match validate_api_command(body.command.as_deref()) {
        Ok(command) => command,
        Err(error) => return invalid_body(error.code(), &error.to_string()),
    };

    if state.active_bedrock_server().is_some() {
        return match state.send_bedrock_command(&command) {
            Ok(active_server_id) => Json(CommandResultDto {
                result: "sent".to_string(),
                active_server_id,
                command,
                runtime: Some(state.bedrock_runtime_state()),
            })
            .into_response(),
            Err(error) => crate::routes::lifecycle::lifecycle_route_error_response(error),
        };
    }

    match state.send_command(&command) {
        Ok(active_server_id) => Json(CommandResultDto {
            result: "sent".to_string(),
            active_server_id,
            command,
            runtime: None,
        })
        .into_response(),
        Err(error) => lifecycle_error_response(error),
    }
}
