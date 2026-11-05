//! Independent player edits commit metadata only; immutable blobs are staged
//! during review and never overwrite active server files.
use super::*;
use msc_api::dto::{ContentChangesReviewDto, ContentInstallApplyDto, PlayerFileAdmissionDto};
use msc_application::content_management as service;
fn server(
    state: &ComponentsRoutesState,
    id: &str,
) -> Result<msc_domain::app_config_schema::ConfigServer, String> {
    state
        .lifecycle
        .app_config_servers()
        .into_iter()
        .find(|server| server.id == id)
        .ok_or_else(|| "Server not found".into())
}
pub(super) fn public_review(review: service::Review) -> serde_json::Value {
    serde_json::json!({"reviewId": review.id, "collectionRevision": review.source_collection.as_ref().map_or(0, |collection| collection.revision), "collection":review.result, "changes":review.changes, "warnings":review.warnings})
}
pub(super) async fn get(
    State(state): State<ComponentsRoutesState>,
    Extension(_credential): Extension<AuthenticatedCredential>,
    AxumPath(server_id): AxumPath<String>,
) -> Response {
    match tokio::task::spawn_blocking(move || service::snapshot(&server(&state, &server_id)?)).await
    {
        Ok(Ok(snapshot)) => Json(snapshot).into_response(),
        Ok(Err(error)) => error_response(StatusCode::CONFLICT, "content_unavailable", &error),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "content_unavailable",
            "Content reader failed",
        ),
    }
}
pub(super) async fn admit(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<PlayerFileAdmissionDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return invalid_body(
                "invalid_json",
                "Provide a player content path, exact size and SHA512",
            );
        }
    };
    if let Err(error) = service::content_path(&body.path) {
        return invalid_body("invalid_content_path", &error);
    }
    if body.byte_size == 0
        || body.byte_size > 512 * 1024 * 1024
        || body.sha512.len() != 128
        || !body.sha512.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return invalid_body(
            "invalid_file_identity",
            "Provide a nonempty file up to 512 MiB and its exact SHA512",
        );
    }
    if let Err(error) = server(&state, &server_id) {
        return invalid_body("server_unavailable", &error);
    }
    let id = Uuid::new_v4().to_string();
    let expiry = now_unix() + STAGING_TTL_SECONDS;
    state.staging.uploads.lock().unwrap().insert(
        id.clone(),
        StagedUpload {
            provider_owner: Some(credential.credential_id.clone()),
            pack_origin: None,
            native_guard: None,
            purpose: StagedUploadPurposeDto::AddonLocalFile,
            file_name: Path::new(&body.path)
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned),
            operation_id: Some(format!(
                "player:{server_id}:{}",
                body.sha512.to_ascii_lowercase()
            )),
            file_id: Some(body.path),
            expires_at_unix: expiry,
            max_bytes: body.byte_size,
            expected_bytes: Some(body.byte_size),
            received_bytes: 0,
            complete: false,
            path: staging_root(&state.lifecycle.servers_root())
                .join("uploads")
                .join(format!("{id}.bin")),
        },
    );
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/servers/:server_id/content/player-files/admit",
        StatusCode::OK,
    );
    Json(StagedUploadBeginResultDto {
        staged_upload_id: id.clone(),
        upload_path: format!("/v1/staged-uploads/{id}"),
        expires_at: unix_to_iso8601(expiry),
        max_bytes: body.byte_size as i64,
        max_chunk_bytes: Some(STAGED_UPLOAD_CHUNK_BYTES as i64),
    })
    .into_response()
}
pub(super) async fn review(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<ContentChangesReviewDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return invalid_body(
                "invalid_json",
                "Provide content changes and a collection revision",
            );
        }
    };
    let changes: Vec<service::Change> =
        match serde_json::to_value(body.changes).and_then(serde_json::from_value) {
            Ok(changes) => changes,
            Err(_) => {
                return invalid_body(
                    "invalid_content_changes",
                    "Choose a valid audience and content action",
                );
            }
        };
    let current = match server(&state, &server_id) {
        Ok(server) => server,
        Err(error) => return invalid_body("server_unavailable", &error),
    };
    let operations = state.lifecycle.operations();
    let operation = match operations.begin_lifecycle(
        "addon-player-review",
        Some(server_id.clone()),
        "Reviewing player content",
    ) {
        Ok(operation) => operation,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let mut uploads = BTreeMap::new();
    let validation = (|| {
        let store = state.staging.uploads.lock().unwrap();
        for change in &changes {
            if let Some(id) = &change.staged_upload_id {
                let entry = store.get(id).ok_or("Player upload is unavailable")?;
                let sha512 = change
                    .sha512
                    .as_ref()
                    .ok_or("Uploaded files need an exact SHA512")?;
                if !entry.complete
                    || entry.expires_at_unix < now_unix()
                    || entry.provider_owner.as_deref() != Some(credential.credential_id.as_str())
                    || !matches!(entry.purpose, StagedUploadPurposeDto::AddonLocalFile)
                    || entry.file_id.as_deref() != Some(change.path.as_str())
                    || entry.operation_id.as_deref()
                        != Some(format!("player:{server_id}:{sha512}").as_str())
                {
                    return Err(
                        "Player upload does not match this credential, server or content path",
                    );
                }
                uploads.insert(id.clone(), entry.path.clone());
            }
        }
        Ok(())
    })();
    if let Err(error) = validation {
        let _ = operations.fail(&operation, "player_review_failed", error.into());
        return invalid_body("invalid_player_upload", error);
    }
    let audit_state = state.lifecycle.clone();
    let audit_credential = credential.clone();
    let identity = crate::auth::StreamAuthentication {
        credential: credential.clone(),
    };
    let result = tokio::task::spawn_blocking(move || {
        if !auth.stream_retains_permissions(&identity) {
            return Err("Content permission revoked".into());
        }
        let review = service::review(
            &current,
            &credential.credential_id,
            body.collection_revision,
            changes,
            &uploads,
        )?;
        if !auth.stream_retains_permissions(&identity) || server(&state, &server_id)? != current {
            return Err("Server or permission changed during review".into());
        }
        Ok::<_, String>(review)
    })
    .await;
    let response = match result {
        Ok(Ok(review)) => {
            let _ = operations.succeed(&operation, "Player content review ready", BTreeMap::new());
            Json(public_review(review)).into_response()
        }
        other => {
            let error = match other {
                Ok(Err(error)) => error,
                _ => "Content review worker failed".into(),
            };
            let _ = operations.fail(&operation, "player_review_failed", error.clone());
            error_response(StatusCode::CONFLICT, "player_review_failed", &error)
        }
    };
    audit(
        &audit_state,
        &audit_credential,
        "POST",
        "/v1/servers/:server_id/content/review",
        response.status(),
    );
    response
}
pub(super) async fn apply(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<ContentInstallApplyDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Provide a reviewId"),
    };
    let operations = state.lifecycle.operations();
    let operation = match operations.begin_lifecycle(
        "addon-player-apply",
        Some(server_id.clone()),
        "Applying reviewed player content",
    ) {
        Ok(operation) => operation,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let response = (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"operationId":operation.as_str()})),
    )
        .into_response();
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/servers/:server_id/content/apply",
        response.status(),
    );
    tokio::task::spawn_blocking(move || {
        let identity = crate::auth::StreamAuthentication {
            credential: credential.clone(),
        };
        let cancel = operations.cancellation_check(&operation);
        let result = (|| {
            let current = server(&state, &server_id)?;
            let review =
                service::load_review(&current, &body.review_id, &credential.credential_id)?;
            state
                .lifecycle
                .try_mutate_config(|config| {
                    if cancel() || !auth.stream_retains_permissions(&identity) {
                        return Err("Content apply cancelled or permission revoked");
                    }
                    let stored = config
                        .servers
                        .iter_mut()
                        .find(|server| server.id == server_id)
                        .ok_or("Server removed")?;
                    if stored != &current {
                        return Err("Server changed while applying player content");
                    }
                    stored.content_collection = Some(review.result);
                    Ok(())
                })
                .map_err(|_| {
                    "Could not commit player content; refresh and review again".to_string()
                })?;
            Ok::<_, String>(())
        })();
        match result {
            Ok(()) => {
                let _ = operations.succeed(&operation, "Player content saved", BTreeMap::new());
            }
            Err(error) => {
                let _ = operations.fail(&operation, "player_apply_failed", error);
            }
        }
    });
    response
}

pub(super) async fn get_review(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath((server_id, review_id)): AxumPath<(String, String)>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let result = tokio::task::spawn_blocking(move || {
        service::load_review(
            &server(&state, &server_id)?,
            &review_id,
            &credential.credential_id,
        )
    })
    .await;
    match result {
        Ok(Ok(review)) => Json(public_review(review)).into_response(),
        Ok(Err(error)) => {
            error_response(StatusCode::CONFLICT, "content_review_unavailable", &error)
        }
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "content_review_unavailable",
            "Content review reader failed",
        ),
    }
}
