//! `GET /v1/status` — a single hard-coded fake server, standing in for
//! real process/Docker detection (later-phase substrate work). Runs
//! behind `auth::require_bearer_token`.

use axum::Json;
use msc_api::dto::RemoteApiStatus;

pub async fn status() -> Json<RemoteApiStatus> {
    Json(RemoteApiStatus {
        running: true,
        active_server_id: Some("demo-survival".to_string()),
        pid: Some(51234),
        server_type: Some("paper".to_string()),
        docker_container_running: None,
        docker_container_status: None,
    })
}
