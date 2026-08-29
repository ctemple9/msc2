//! `POST /v1/command`.

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{CommandRequestDto, CommandResultDto, ErrorDto, PermissionCategoryDto};
use msc_application::commands::validate_api_command;
use msc_application::world_safety::{self, SafetyConfirmation};

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{
    LifecycleRoutesState, invalid_body, lifecycle_error_response, require_permission,
};

pub async fn command(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<serde_json::Value>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::ServerControl) {
        return response;
    }

    let Json(raw_body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let confirmation = raw_body
        .get("confirmation")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let body = match serde_json::from_value::<CommandRequestDto>(raw_body) {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_body", "Request body must be a command object."),
    };
    let command = match validate_api_command(body.command.as_deref()) {
        Ok(command) => command,
        Err(error) => return invalid_body(error.code(), &error.to_string()),
    };

    if state.active_bedrock_server().is_some() {
        // Bedrock accepts client-style slash commands but exposes the canonical
        // slash-free command in both the runtime payload and response DTO.
        let command = command.strip_prefix('/').unwrap_or(&command).to_string();
        if let Some(required) = world_safety::confirmation_for_command(
            msc_domain::identity::ServerType::Bedrock,
            &command,
        ) && !world_safety::is_confirmed(required, confirmation.as_deref())
        {
            return confirmation_required_response(required);
        }
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

    if let Some(required) =
        world_safety::confirmation_for_command(msc_domain::identity::ServerType::Java, &command)
        && !world_safety::is_confirmed(required, confirmation.as_deref())
    {
        return confirmation_required_response(required);
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

fn confirmation_required_response(required: SafetyConfirmation) -> Response {
    (
        StatusCode::CONFLICT,
        Json(ErrorDto {
            code: "confirmation_required".to_string(),
            message: required.message().to_string(),
            help_id: None,
            details: Some(required.details()),
        }),
    )
        .into_response()
}
