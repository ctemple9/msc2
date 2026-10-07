//! Preview-bound cleanup of rebuildable, abandoned map staging on this host.
use super::lifecycle::{LifecycleRoutesState, error_response, require_permission};
use crate::auth::AuthenticatedCredential;
use crate::map_staging::{self, CleanupEntry};
use axum::{
    Json, Router,
    extract::{Extension, State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use msc_api::dto::PermissionCategoryDto;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
struct StorageState {
    lifecycle: LifecycleRoutesState,
    previews: Arc<Mutex<HashMap<String, Preview>>>,
}
struct Preview {
    credential_id: String,
    expires: Instant,
    entries: Vec<CleanupEntry>,
}

pub fn router(lifecycle: LifecycleRoutesState) -> Router {
    Router::new()
        .route("/host/storage/preview", post(preview))
        .route("/host/storage/cleanup", post(cleanup))
        .with_state(StorageState {
            lifecycle,
            previews: Arc::new(Mutex::new(HashMap::new())),
        })
}

async fn preview(
    State(state): State<StorageState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Admin) {
        return response;
    }
    let entries = match tokio::task::spawn_blocking(map_staging::preview_cleanup).await {
        Ok(Ok(entries)) => entries,
        result => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage_scan_failed",
                &format!("Could not inspect rebuildable map data: {result:?}"),
            );
        }
    };
    let token = uuid::Uuid::new_v4().to_string();
    let reclaimable_bytes = entries
        .iter()
        .filter(|item| item.removable)
        .map(|item| item.size_bytes)
        .sum::<u64>();
    let mut previews = state.previews.lock().unwrap();
    previews.retain(|_, preview| preview.expires > Instant::now());
    if previews.len() >= 64 {
        return error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "too_many_previews",
            "Try again after existing cleanup previews expire.",
        );
    }
    previews.insert(
        token.clone(),
        Preview {
            credential_id: credential.credential_id,
            expires: Instant::now() + Duration::from_secs(300),
            entries: entries.clone(),
        },
    );
    Json(serde_json::json!({ "previewToken": token, "root": map_staging::cleanup_root(), "entries": entries, "reclaimableBytes": reclaimable_bytes, "expiresInSeconds": 300 })).into_response()
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CleanupRequest {
    preview_token: String,
}

async fn cleanup(
    State(state): State<StorageState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<CleanupRequest>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Admin) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return super::lifecycle::invalid_body(
                "invalid_json",
                "A cleanup previewToken is required.",
            );
        }
    };
    let preview = {
        let mut previews = state.previews.lock().unwrap();
        match previews.get(&body.preview_token) {
            Some(preview)
                if preview.credential_id == credential.credential_id
                    && preview.expires > Instant::now() =>
            {
                previews.remove(&body.preview_token).unwrap()
            }
            _ => {
                return error_response(
                    StatusCode::CONFLICT,
                    "cleanup_preview_expired",
                    "Scan again before confirming cleanup.",
                );
            }
        }
    };
    if !preview.entries.iter().any(|entry| entry.removable) {
        return Json(serde_json::json!({ "removedBytes": 0, "removed": [], "retained": [] }))
            .into_response();
    }
    let operation = match state.lifecycle.operations().begin_lifecycle(
        "host-storage-cleanup",
        None,
        "Cleaning abandoned map staging.",
    ) {
        Ok(operation) => operation,
        Err(error) => return super::operations::operation_error_response(error),
    };
    let lifecycle = state.lifecycle.clone();
    let should_cancel = lifecycle.operations().cancellation_check(&operation);
    let worker_cancel = should_cancel.clone();
    let result = tokio::task::spawn_blocking(move || {
        map_staging::execute_cleanup(&preview.entries, &worker_cancel)
    })
    .await;
    match result {
        Ok(Ok(result)) => {
            if should_cancel() {
                let _ = lifecycle.operations().cancel(
                    &operation,
                    "Storage cleanup cancelled at a directory boundary.",
                );
            } else {
                let _ = lifecycle.finish_operation_success(
                    &operation,
                    "Storage cleanup completed.",
                    std::collections::BTreeMap::from([(
                        "removedBytes".to_string(),
                        result.removed_bytes.to_string(),
                    )]),
                );
            }
            let _ = lifecycle
                .audit_log()
                .log(&msc_infrastructure::audit_log::Entry {
                    timestamp: std::time::SystemTime::now(),
                    client_ip: "authenticated-management".to_string(),
                    token_label: credential.label,
                    method: "POST".to_string(),
                    path: "/v1/host/storage/cleanup".to_string(),
                    status_code: 200,
                });
            Json(result).into_response()
        }
        result => {
            let message =
                format!("Cleanup failed; scan again to inspect remaining files: {result:?}");
            let _ = lifecycle.finish_operation_failure(
                &operation,
                "storage_cleanup_failed",
                message.clone(),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage_cleanup_failed",
                &message,
            )
        }
    }
}
