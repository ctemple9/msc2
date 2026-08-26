//! `GET`/`POST /v1/session-log` routes for the active Java server.

use std::path::Path;

use axum::Json;
use axum::Router;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use msc_api::dto::PermissionCategoryDto;
use msc_application::session_log::{self, SessionEvent, SessionEventType};
use msc_infrastructure::fs::StdFileSystem;
use serde::{Deserialize, Serialize};

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{LifecycleRoutesState, error_response, require_permission};

pub fn router(state: LifecycleRoutesState) -> Router {
    Router::new()
        .route("/session-log", get(get_session_log))
        .route("/session-log/clear", post(clear_session_log))
        .with_state(state)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionEventDto {
    id: String,
    player_name: String,
    event_type: String,
    timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionLogResponseDto {
    active_server_id: Option<String>,
    events: Vec<SessionEventDto>,
}

async fn get_session_log(State(state): State<LifecycleRoutesState>) -> Json<SessionLogResponseDto> {
    let Some(server) = state.active_config_server() else {
        return Json(empty_response(None));
    };

    Json(SessionLogResponseDto {
        active_server_id: Some(server.id),
        events: session_event_dtos(session_log::load_events(
            &StdFileSystem,
            Path::new(&server.server_dir),
        )),
    })
}

async fn clear_session_log(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
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

    if let Err(error) = session_log::clear_events(&StdFileSystem, Path::new(&server.server_dir)) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        );
    }

    Json(empty_response(Some(server.id))).into_response()
}

fn empty_response(active_server_id: Option<String>) -> SessionLogResponseDto {
    SessionLogResponseDto {
        active_server_id,
        events: Vec::new(),
    }
}

fn session_event_dtos(events: Vec<SessionEvent>) -> Vec<SessionEventDto> {
    events
        .into_iter()
        .map(|event| SessionEventDto {
            id: event.id.to_string(),
            player_name: event.player_name,
            event_type: event_type_string(event.event_type).to_owned(),
            timestamp: event.timestamp,
        })
        .collect()
}

fn event_type_string(event_type: SessionEventType) -> &'static str {
    match event_type {
        SessionEventType::Joined => "joined",
        SessionEventType::Left => "left",
    }
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
        let dir = std::env::temp_dir().join(format!("msc2-session-log-route-{tag}-{nonce}"));
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
            display_name: "Session Log Route Paper".to_owned(),
            paper_jar_path: server_dir.join("paper.jar"),
            server_dir: server_dir.clone(),
            eula_accepted: Some(true),
            game_port: 25565,
            max_players: 20,
            world_name: "world".to_owned(),
            properties: ServerPropertiesModel::from_dict(&HashMap::new(), None),
        };
        std::fs::write(&server.paper_jar_path, b"fake jar").unwrap();
        state.register_imported_paper(server).unwrap();
        state.select_active_server("paper-1".to_owned()).unwrap();
        (state, server_dir)
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
    async fn get_session_log_returns_empty_when_no_server_is_active() {
        let state = LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );

        let Json(response) = get_session_log(State(state)).await;

        assert_eq!(response.active_server_id, None);
        assert!(response.events.is_empty());
    }

    #[tokio::test]
    async fn get_session_log_maps_persisted_events_to_the_contract() {
        let (state, server_dir) = active_state("get");
        session_log::append_event(
            &StdFileSystem,
            &server_dir,
            "Alex",
            SessionEventType::Joined,
            "2026-08-26T12:34:56Z".to_owned(),
        )
        .unwrap();
        session_log::append_event(
            &StdFileSystem,
            &server_dir,
            "Alex",
            SessionEventType::Left,
            "2026-08-26T13:34:56Z".to_owned(),
        )
        .unwrap();

        let Json(response) = get_session_log(State(state)).await;

        assert_eq!(response.active_server_id.as_deref(), Some("paper-1"));
        assert_eq!(response.events.len(), 2);
        assert_eq!(response.events[0].player_name, "Alex");
        assert_eq!(response.events[0].event_type, "joined");
        assert_eq!(response.events[1].event_type, "left");
        assert_eq!(response.events[0].timestamp, "2026-08-26T12:34:56Z");

        std::fs::remove_dir_all(server_dir).unwrap();
    }

    #[tokio::test]
    async fn clear_session_log_requires_an_active_server() {
        let state = LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );

        let response = clear_session_log(State(state), Extension(players_credential())).await;

        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["code"], "conflict");
    }

    #[tokio::test]
    async fn clear_session_log_removes_events_and_returns_empty_state() {
        let (state, server_dir) = active_state("clear");
        session_log::append_event(
            &StdFileSystem,
            &server_dir,
            "Alex",
            SessionEventType::Joined,
            "2026-08-26T12:34:56Z".to_owned(),
        )
        .unwrap();

        let response = clear_session_log(State(state), Extension(players_credential())).await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: SessionLogResponseDto = serde_json::from_slice(&body).unwrap();
        assert_eq!(body.active_server_id.as_deref(), Some("paper-1"));
        assert!(body.events.is_empty());
        assert!(session_log::load_events(&StdFileSystem, &server_dir).is_empty());

        std::fs::remove_dir_all(server_dir).unwrap();
    }
}
