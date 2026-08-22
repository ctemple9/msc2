//! `/v1/config/geyser` — Geyser owns most of its YAML, so MSC exposes only
//! the Bedrock listener values it can patch without rewriting that file.

use std::path::Path;

use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use msc_api::dto::{
    GeyserConfigResponseDto, GeyserConfigUpdateRequestDto, GeyserConfigUpdateResultDto,
    PermissionCategoryDto,
};
use msc_application::geyser;
use msc_domain::identity::ServerType;
use msc_infrastructure::fs::{FileSystem, StdFileSystem};

use crate::{
    auth::AuthenticatedCredential,
    routes::lifecycle::{LifecycleRoutesState, error_response, invalid_body, require_permission},
};

pub async fn get_config(
    State(state): State<LifecycleRoutesState>,
) -> Json<GeyserConfigResponseDto> {
    let config = state.app_config_snapshot();
    let active = config
        .active_server_id
        .as_ref()
        .and_then(|id| config.servers.iter().find(|server| &server.id == id));
    let Some(server) = active else {
        return Json(empty_response("no_active_server"));
    };
    if server.server_type != ServerType::Java {
        return Json(empty_response("Geyser is available only for Java servers."));
    }
    let fs = StdFileSystem;
    let installation = geyser::installation(&fs, Path::new(&server.server_dir));
    let path = geyser::config_path(Path::new(&server.server_dir));
    let config = geyser::read_config(&fs, Path::new(&server.server_dir));
    Json(GeyserConfigResponseDto {
        server_name: server.display_name.clone(),
        server_type: "java".into(),
        is_geyser_installed: installation.geyser_installed,
        address: config.as_ref().map(|config| config.address.clone()),
        port: config.and_then(|config| config.port.map(i64::from)),
        config_file_exists: fs.stat(&path).map(|meta| meta.is_file).unwrap_or(false),
        note: if installation.geyser_installed {
            None
        } else {
            Some("geyser_not_installed".into())
        },
    })
}

pub async fn update_config(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<GeyserConfigUpdateRequestDto>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Settings) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let config = state.app_config_snapshot();
    let active = config
        .active_server_id
        .as_ref()
        .and_then(|id| config.servers.iter().find(|server| &server.id == id));
    let Some(server) = active else {
        return error_response(
            StatusCode::CONFLICT,
            "no_active_server",
            "No active server.",
        );
    };
    if server.server_type != ServerType::Java {
        return error_response(
            StatusCode::CONFLICT,
            "not_supported",
            "Geyser is available only for Java servers.",
        );
    }
    let fs = StdFileSystem;
    if !geyser::installation(&fs, Path::new(&server.server_dir)).geyser_installed {
        return error_response(
            StatusCode::CONFLICT,
            "not_installed",
            "Geyser is not installed for this server.",
        );
    }
    match geyser::update_config(
        &fs,
        Path::new(&server.server_dir),
        body.address.as_deref(),
        body.port,
    ) {
        Ok(config) => Json(GeyserConfigUpdateResultDto {
            success: true,
            message: "saved".into(),
            address: Some(config.address),
            port: config.port.map(i64::from),
        })
        .into_response(),
        Err(message) => error_response(StatusCode::BAD_REQUEST, "invalid_config", &message),
    }
}

fn empty_response(note: &str) -> GeyserConfigResponseDto {
    GeyserConfigResponseDto {
        server_name: String::new(),
        server_type: "java".into(),
        is_geyser_installed: false,
        address: None,
        port: None,
        config_file_exists: false,
        note: Some(note.into()),
    }
}
