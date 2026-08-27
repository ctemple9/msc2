//! `GET /v1/files` and `GET /v1/files/read`: admin-only browse and preview
//! of the active server's directory. The HTTP status-code mapping ports
//! `handleGetFiles`/`handleReadFile`
//! (`RemoteAPIServer+ComponentRoutes.swift:307-337`) on top of
//! `msc_application::server_files`, which carries the oracle citation for
//! the actual browse/read behavior.

use std::path::{Path, PathBuf};

use axum::Json;
use axum::Router;
use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use msc_api::dto::PermissionCategoryDto;
use msc_application::server_files::{self, BrowseOutcome, FileReadOutcome};
use msc_infrastructure::fs::StdFileSystem;
use serde::{Deserialize, Serialize};

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{LifecycleRoutesState, error_response, require_permission};

pub fn router(state: LifecycleRoutesState) -> Router {
    Router::new()
        .route("/files", get(browse_files))
        .route("/files/read", get(read_file))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
pub(crate) struct FilesQuery {
    #[serde(default)]
    path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerFileItemDto {
    id: String,
    name: String,
    path: String,
    is_directory: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_extension: Option<String>,
    is_previewable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerFilesResponseDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    server_name: Option<String>,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_path: Option<String>,
    items: Vec<ServerFileItemDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerFileReadResponseDto {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncated: Option<bool>,
}

async fn browse_files(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Query(query): Query<FilesQuery>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Admin) {
        return response;
    }

    let Some(server) = state.active_config_server() else {
        return error_response(
            StatusCode::CONFLICT,
            "conflict",
            "No server is currently active.",
        );
    };

    let home_dir = agent_home_dir();
    let outcome = server_files::browse_directory(
        &StdFileSystem,
        Path::new(&server.server_dir),
        &home_dir,
        query.path.as_deref(),
    );

    let response = match outcome {
        BrowseOutcome::Listed(listing) => ServerFilesResponseDto {
            server_name: Some(server.display_name),
            path: listing.path,
            parent_path: listing.parent_path,
            items: listing.items.into_iter().map(file_item_dto).collect(),
            note: None,
        },
        BrowseOutcome::InvalidPath => ServerFilesResponseDto {
            server_name: Some(server.display_name),
            path: String::new(),
            parent_path: None,
            items: Vec::new(),
            note: Some("invalid_path".to_owned()),
        },
        BrowseOutcome::DirectoryNotFound => ServerFilesResponseDto {
            server_name: Some(server.display_name),
            path: String::new(),
            parent_path: None,
            items: Vec::new(),
            note: Some("directory_not_found".to_owned()),
        },
    };
    Json(response).into_response()
}

async fn read_file(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Query(query): Query<FilesQuery>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Admin) {
        return response;
    }

    let requested = query.path.unwrap_or_default();
    let trimmed = requested.trim();
    if trimmed.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "invalid_body", "missing_path");
    }

    let Some(server) = state.active_config_server() else {
        return error_response(StatusCode::CONFLICT, "conflict", "no_active_server");
    };

    let home_dir = agent_home_dir();
    let outcome = server_files::read_previewable_file(
        &StdFileSystem,
        Path::new(&server.server_dir),
        &home_dir,
        trimmed,
    );

    match outcome {
        FileReadOutcome::Read {
            name,
            size_bytes,
            content,
            truncated,
        } => Json(ServerFileReadResponseDto {
            success: true,
            message: "ok".to_owned(),
            path: Some(trimmed.to_owned()),
            name: Some(name),
            size_bytes,
            content: Some(content),
            encoding: Some("text".to_owned()),
            truncated: Some(truncated),
        })
        .into_response(),
        // A traversal escape is folded into "not found" rather than given
        // its own status: the oracle's own HTTP layer never gave
        // `invalid_path` a documented code on this route (its status
        // switch falls through to a bare 500 default,
        // `RemoteAPIServer+ComponentRoutes.swift:332`), and the frozen
        // msc2 contract doesn't reserve one either. 404 hides the sandbox
        // boundary instead of confirming something outside it exists --
        // the safer default for a case a real client should never trigger
        // (P12.9, documented in rolling-plan.md).
        FileReadOutcome::InvalidPath | FileReadOutcome::FileNotFound => {
            error_response(StatusCode::NOT_FOUND, "not_found", "file_not_found")
        }
        FileReadOutcome::DirectoryNotFile => {
            error_response(StatusCode::CONFLICT, "conflict", "directory_not_file")
        }
        FileReadOutcome::NotPreviewable { .. } => {
            error_response(StatusCode::CONFLICT, "conflict", "not_previewable")
        }
        FileReadOutcome::ReadFailed { .. } => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "read_failed",
        ),
    }
}

fn file_item_dto(entry: server_files::FileEntry) -> ServerFileItemDto {
    ServerFileItemDto {
        id: entry.path.clone(),
        name: entry.name,
        path: entry.path,
        is_directory: entry.is_directory,
        size_bytes: entry.size_bytes,
        modified_at: entry
            .modified_at
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|duration| super::system_time_to_iso8601(duration.as_secs())),
        file_extension: entry.file_extension,
        is_previewable: entry.is_previewable,
    }
}

/// `safe_path`'s required `home_dir` parameter, used only for its
/// `ForbiddenRoot` check. Duplicated rather than shared, matching
/// `servers.rs`/`templates.rs`/`versions.rs`'s own `agent_home_dir` copies
/// -- no shared HOME resolver exists in this crate yet.
fn agent_home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::CredentialRole;
    use crate::routes::lifecycle::LifecycleRoutesState;
    use crate::routes::operations::OperationsState;
    use crate::ws::console::ConsoleState;
    use msc_application::import::ImportedPaperServer;
    use msc_application::lifecycle::ServerId;
    use msc_domain::properties::ServerPropertiesModel;
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_server_dir(tag: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("msc2-files-route-{tag}-{nonce}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn active_state(tag: &str) -> (LifecycleRoutesState, std::path::PathBuf) {
        let server_dir = temp_server_dir(tag);
        let state = LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );
        let server = ImportedPaperServer {
            id: ServerId::new("paper-1"),
            display_name: "Files Route Paper".to_owned(),
            paper_jar_path: server_dir.join("paper.jar"),
            server_dir: server_dir.clone(),
            eula_accepted: Some(true),
            game_port: 25565,
            max_players: 20,
            world_name: "world".to_owned(),
            properties: ServerPropertiesModel::from_dict(&HashMap::new(), None),
        };
        std::fs::write(&server.paper_jar_path, b"fake jar").unwrap();
        std::fs::write(server_dir.join("server.properties"), b"motd=hi").unwrap();
        state.register_imported_paper(server).unwrap();
        state.select_active_server("paper-1".to_owned()).unwrap();
        (state, server_dir)
    }

    fn admin_credential() -> AuthenticatedCredential {
        AuthenticatedCredential {
            credential_id: "admin".to_owned(),
            label: "console".to_owned(),
            role: CredentialRole::Admin,
            permissions: vec![PermissionCategoryDto::Admin],
        }
    }

    fn players_credential() -> AuthenticatedCredential {
        AuthenticatedCredential {
            credential_id: "named".to_owned(),
            label: "console".to_owned(),
            role: CredentialRole::Named,
            permissions: vec![PermissionCategoryDto::Players],
        }
    }

    #[tokio::test]
    async fn browse_files_requires_admin() {
        let (state, server_dir) = active_state("forbidden");

        let response = browse_files(
            State(state),
            Extension(players_credential()),
            Query(FilesQuery { path: None }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        std::fs::remove_dir_all(server_dir).unwrap();
    }

    #[tokio::test]
    async fn browse_files_conflicts_with_no_active_server() {
        let state = LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );

        let response = browse_files(
            State(state),
            Extension(admin_credential()),
            Query(FilesQuery { path: None }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn browse_files_lists_the_server_root() {
        let (state, server_dir) = active_state("list");

        let response = browse_files(
            State(state),
            Extension(admin_credential()),
            Query(FilesQuery { path: None }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: ServerFilesResponseDto = serde_json::from_slice(&body).unwrap();
        assert_eq!(body.server_name.as_deref(), Some("Files Route Paper"));
        assert_eq!(body.path, "");
        assert!(body.note.is_none());
        let names: Vec<&str> = body.items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"paper.jar"));
        assert!(names.contains(&"server.properties"));
        let properties = body
            .items
            .iter()
            .find(|i| i.name == "server.properties")
            .unwrap();
        assert!(properties.is_previewable);

        std::fs::remove_dir_all(server_dir).unwrap();
    }

    #[tokio::test]
    async fn browse_files_reports_a_traversal_escape_as_a_200_note() {
        let (state, server_dir) = active_state("escape");

        let response = browse_files(
            State(state),
            Extension(admin_credential()),
            Query(FilesQuery {
                path: Some("../../etc".to_owned()),
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: ServerFilesResponseDto = serde_json::from_slice(&body).unwrap();
        assert_eq!(body.note.as_deref(), Some("invalid_path"));

        std::fs::remove_dir_all(server_dir).unwrap();
    }

    #[tokio::test]
    async fn read_file_requires_a_path() {
        let (state, server_dir) = active_state("missing-path");

        let response = read_file(
            State(state),
            Extension(admin_credential()),
            Query(FilesQuery { path: None }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        std::fs::remove_dir_all(server_dir).unwrap();
    }

    #[tokio::test]
    async fn read_file_returns_previewable_contents() {
        let (state, server_dir) = active_state("read");

        let response = read_file(
            State(state),
            Extension(admin_credential()),
            Query(FilesQuery {
                path: Some("server.properties".to_owned()),
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: ServerFileReadResponseDto = serde_json::from_slice(&body).unwrap();
        assert!(body.success);
        assert_eq!(body.content.as_deref(), Some("motd=hi"));

        std::fs::remove_dir_all(server_dir).unwrap();
    }

    #[tokio::test]
    async fn read_file_rejects_a_non_previewable_file_with_409() {
        let (state, server_dir) = active_state("not-previewable");

        let response = read_file(
            State(state),
            Extension(admin_credential()),
            Query(FilesQuery {
                path: Some("paper.jar".to_owned()),
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::CONFLICT);
        std::fs::remove_dir_all(server_dir).unwrap();
    }

    #[tokio::test]
    async fn read_file_reports_a_missing_file_with_404() {
        let (state, server_dir) = active_state("not-found");

        let response = read_file(
            State(state),
            Extension(admin_credential()),
            Query(FilesQuery {
                path: Some("missing.txt".to_owned()),
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        std::fs::remove_dir_all(server_dir).unwrap();
    }
}
