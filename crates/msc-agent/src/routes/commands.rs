//! `POST /v1/command`.

use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{
    CommandRequestDto, CommandResultDto, ErrorDto, PermissionCategoryDto, RelativeTimeRequestDto,
    RelativeTimeResultDto,
};
use msc_application::commands::validate_api_command;
use msc_application::world_safety::{self, SafetyConfirmation};

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{
    LifecycleRoutesState, error_response, invalid_body, lifecycle_error_response,
    lifecycle_route_error_response, require_permission,
};

const RELATIVE_TIME_QUERY: &str = "time query gametime";
const RELATIVE_TIME_QUERY_TIMEOUT: Duration = Duration::from_secs(2);

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

/// `POST /v1/time/relative` — resolve a named time of day against the active
/// Minecraft day, then send the runtime's absolute command. The query and
/// set are kept together here so a client cannot accidentally reintroduce the
/// old day-zero behavior by calculating from a cached or assumed value.
pub async fn relative_time(
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
    let body = match serde_json::from_value::<RelativeTimeRequestDto>(raw_body) {
        Ok(body) => body,
        Err(_) => {
            return invalid_body("invalid_body", "Request body must be a time preset object.");
        }
    };
    let preset_name = match body.preset.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => value,
        _ => return invalid_body("missing_preset", "A relative time preset is required."),
    };
    let Some(preset) = msc_domain::time::RelativeTimePreset::from_raw_value(preset_name) else {
        return invalid_body(
            "invalid_preset",
            "Relative time preset must be dawn, dusk, or night.",
        );
    };

    let Some(server) = state.active_config_server() else {
        return lifecycle_error_response(
            msc_application::lifecycle::LifecycleError::NoActiveServer,
        );
    };
    if !state.status_snapshot().running {
        return error_response(
            StatusCode::CONFLICT,
            "server_not_running",
            "Relative Minecraft time requires a running server.",
        );
    }

    let server_type = server.server_type;
    if server_type == msc_domain::identity::ServerType::Bedrock
        && state.bedrock_runtime_state().state != "available"
    {
        return error_response(
            StatusCode::CONFLICT,
            "capability_unavailable",
            "The selected Bedrock runtime cannot perform relative Minecraft time.",
        );
    }

    let _query_guard = state.time_query_lock().lock().await;
    let before = state.time_observation();
    let send_query: Result<(), Response> = match server_type {
        msc_domain::identity::ServerType::Java => state
            .send_controller_command(RELATIVE_TIME_QUERY)
            .map(|_| ())
            .map_err(lifecycle_error_response),
        msc_domain::identity::ServerType::Bedrock => state
            .send_bedrock_controller_command(RELATIVE_TIME_QUERY)
            .map(|_| ())
            .map_err(lifecycle_route_error_response),
    };
    if let Err(response) = send_query {
        return response;
    }

    let deadline = Instant::now() + RELATIVE_TIME_QUERY_TIMEOUT;
    let current_absolute_ticks = loop {
        state.drain_time_query_events();
        let observation = state.time_observation();
        if observation.generation != before.generation
            && let Some(ticks) = observation.absolute_ticks
        {
            break ticks;
        }
        if Instant::now() >= deadline {
            return error_response(
                StatusCode::CONFLICT,
                "capability_unavailable",
                "The selected runtime did not answer its Minecraft time query.",
            );
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    let target = msc_domain::time::RelativeTimeTarget::for_day(preset, current_absolute_ticks);
    let command = target.absolute_command(server_type);
    let send_absolute: Result<(), Response> = match server_type {
        msc_domain::identity::ServerType::Java => state
            .send_command(&command)
            .map(|_| ())
            .map_err(lifecycle_error_response),
        msc_domain::identity::ServerType::Bedrock => state
            .send_bedrock_command(&command)
            .map(|_| ())
            .map_err(lifecycle_route_error_response),
    };
    if let Err(response) = send_absolute {
        return response;
    }

    Json(RelativeTimeResultDto {
        result: "sent".to_string(),
        active_server_id: Some(server.id),
        preset: preset.raw_value().to_string(),
        current_absolute_ticks: target.current_absolute_ticks,
        current_day: target.current_day,
        current_daytime_ticks: target.current_daytime_ticks,
        target_day: target.target_day,
        target_daytime_ticks: target.target_daytime_ticks,
        query_command: RELATIVE_TIME_QUERY.to_string(),
        command,
        runtime: (server_type == msc_domain::identity::ServerType::Bedrock)
            .then(|| state.bedrock_runtime_state()),
    })
    .into_response()
}
