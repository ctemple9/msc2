//! `GET /v1/servers` and `POST /v1/servers/import`.

use std::path::PathBuf;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::response::{IntoResponse, Response};
use msc_api::dto::{
    PermissionCategoryDto, ServerDto, ServerImportRequestDto, ServerImportResultDto,
    ServerImportScanResponseDto,
};
use msc_application::import::{
    PaperImportError, PaperImportRequest, StdPaperImportFileSystem, import_existing_paper_server,
};

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{
    LifecycleRoutesState, error_response, invalid_body, require_permission,
};

pub async fn list(State(state): State<LifecycleRoutesState>) -> Json<Vec<ServerDto>> {
    Json(
        state
            .servers()
            .into_iter()
            .map(|server| ServerDto {
                id: server.id,
                name: server.name,
                directory: server.directory,
                server_type: server.server_type,
                java_flavor: server.java_flavor,
                game_port: server.game_port,
                host_address: None,
            })
            .collect(),
    )
}

pub async fn import(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<ServerImportRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Fleet) {
        return response;
    }

    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let Some(action) = body.action.filter(|value| !value.trim().is_empty()) else {
        return invalid_body("missing_action", "action is required.");
    };
    let Some(source_path) = body.source_path.filter(|value| !value.trim().is_empty()) else {
        return invalid_body("missing_source_path", "sourcePath is required.");
    };

    if action == "scan" {
        return Json(ServerImportScanResponseDto {
            success: true,
            message: "Paper server directory scan completed.".to_string(),
            source_path: Some(source_path),
            is_zip: Some(false),
            server_type: Some("java".to_string()),
            port: None,
            max_players: None,
            eula_accepted: None,
            default_world_name: None,
            java_flavor: Some("paper".to_string()),
        })
        .into_response();
    }

    if !matches!(action.as_str(), "importExisting" | "importPaper") {
        return invalid_body(
            "invalid_action",
            "Only Paper directory import is available in Phase 4.",
        );
    }

    let display_name = body
        .display_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_display_name(&source_path));
    let request = PaperImportRequest::new(display_name, PathBuf::from(&source_path));
    let fs = StdPaperImportFileSystem;
    let mut registry = RouteRegistry { state: &state };

    match import_existing_paper_server(&fs, &mut registry, &request) {
        Ok(server) => Json(ServerImportResultDto {
            success: true,
            message: "Imported Paper server.".to_string(),
            server_id: Some(server.id.as_str().to_string()),
            server_name: Some(server.display_name),
            imported: Some(1),
            skipped: Some(0),
            replaced: Some(false),
        })
        .into_response(),
        Err(error) => import_error_response(error),
    }
}

struct RouteRegistry<'state> {
    state: &'state LifecycleRoutesState,
}

impl msc_application::import::PaperServerRegistry for RouteRegistry<'_> {
    fn register(
        &mut self,
        server: msc_application::import::ImportedPaperServer,
    ) -> Result<(), PaperImportError> {
        self.state.register_imported_paper(server);
        Ok(())
    }
}

fn import_error_response(error: PaperImportError) -> Response {
    match error {
        PaperImportError::EmptyDisplayName => {
            invalid_body("invalid_body", "displayName cannot be empty.")
        }
        PaperImportError::NoJavaServerJar { .. } => error_response(
            axum::http::StatusCode::CONFLICT,
            "conflict",
            &error.to_string(),
        ),
        PaperImportError::ReadDirectory { .. } | PaperImportError::Registry(_) => error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        ),
    }
}

fn default_display_name(source_path: &str) -> String {
    PathBuf::from(source_path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Imported Paper Server")
        .to_string()
}
