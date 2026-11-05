//! Explicit provider source actions keep legacy Modrinth link maps unchanged.
use super::*;
use msc_api::dto::{ContentIdentificationDto, ContentSourceLinkDto, ContentUpdatesDto};
use msc_application::content_sources as service;

struct CancelRead(std::sync::Arc<std::sync::atomic::AtomicBool>);
impl Drop for CancelRead {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::Release);
    }
}

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

// Convert between independent application and wire models using their shared
// documented field contract. A mismatch fails rather than returning partial data.
fn wire<T: serde::de::DeserializeOwned>(value: impl serde::Serialize) -> Result<T, String> {
    serde_json::from_value(serde_json::to_value(value).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

pub(super) async fn identify(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
) -> Response {
    let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let _guard = CancelRead(cancelled.clone());
    let identity = crate::auth::StreamAuthentication { credential };
    let result = tokio::task::spawn_blocking(move || {
        let server = server(&state, &server_id)?;
        let secrets =
            production_secret_store().map_err(|_| "Provider secret storage unavailable")?;
        wire::<ContentIdentificationDto>(service::identify(
            &server,
            &HttpTransport::new(),
            secrets.as_ref(),
            &|| {
                cancelled.load(std::sync::atomic::Ordering::Acquire)
                    || !auth.stream_retains_permissions(&identity)
            },
        )?)
    })
    .await;
    match result {
        Ok(Ok(result)) => Json(result).into_response(),
        Ok(Err(error)) => error_response(StatusCode::CONFLICT, "identification_failed", &error),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "identification_failed",
            "Identification worker failed",
        ),
    }
}

pub(super) async fn updates(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
) -> Response {
    let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let _guard = CancelRead(cancelled.clone());
    let identity = crate::auth::StreamAuthentication { credential };
    let result = tokio::task::spawn_blocking(move || {
        let server = server(&state, &server_id)?;
        let secrets =
            production_secret_store().map_err(|_| "Provider secret storage unavailable")?;
        wire::<ContentUpdatesDto>(service::updates(
            &server,
            &HttpTransport::new(),
            secrets.as_ref(),
            &|| {
                cancelled.load(std::sync::atomic::Ordering::Acquire)
                    || !auth.stream_retains_permissions(&identity)
            },
        )?)
    })
    .await;
    match result {
        Ok(Ok(result)) => Json(result).into_response(),
        Ok(Err(error)) => error_response(StatusCode::CONFLICT, "updates_unavailable", &error),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "updates_unavailable",
            "Update lookup worker failed",
        ),
    }
}

pub(super) async fn link(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<ContentSourceLinkDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return invalid_body(
                "invalid_json",
                "Provide an installed file and reviewed checksum",
            );
        }
    };
    let operations = state.lifecycle.operations();
    let operation = match operations.begin_lifecycle(
        "addon-source-link",
        Some(server_id.clone()),
        "Updating provider source",
    ) {
        Ok(operation) => operation,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let audit_state = state.lifecycle.clone();
    let identity = crate::auth::StreamAuthentication {
        credential: credential.clone(),
    };
    let result = tokio::task::spawn_blocking(move || {
        let before = server(&state, &server_id)?;
        let secrets =
            production_secret_store().map_err(|_| "Provider secret storage unavailable")?;
        let collection = service::link_source(
            &before,
            service::SourceSelection {
                path: &body.path,
                expected_sha512: &body.sha512,
                project_id: body.project_id,
                version_id: body.version_id,
            },
            &HttpTransport::new(),
            secrets.as_ref(),
        )?;
        let local = provider::fingerprint::identify(
            &msc_application::content_install::managed_path(&before.server_dir, &body.path)?,
        )?;
        if local.sha512 != body.sha512 {
            return Err("File changed during source lookup".into());
        }
        if !auth.stream_retains_permissions(&identity) {
            return Err("Source permission was revoked during lookup".into());
        }
        state
            .lifecycle
            .try_mutate_config(|config| {
                let current = config
                    .servers
                    .iter_mut()
                    .find(|server| server.id == server_id)
                    .ok_or("Server removed")?;
                if current != &before {
                    return Err("Server metadata changed during source lookup");
                }
                current.content_collection = Some(collection);
                Ok(())
            })
            .map_err(|_| "Could not save source metadata".to_string())
    })
    .await;
    let response = match result {
        Ok(Ok(())) => {
            let _ = operations.succeed(&operation, "Provider source saved", BTreeMap::new());
            Json(serde_json::json!({"success": true})).into_response()
        }
        Ok(Err(error)) => {
            let _ = operations.fail(&operation, "source_link_failed", error.clone());
            error_response(StatusCode::CONFLICT, "source_link_failed", &error)
        }
        Err(_) => {
            let _ = operations.fail(
                &operation,
                "source_link_failed",
                "Source worker failed".into(),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "source_link_failed",
                "Source worker failed",
            )
        }
    };
    audit(
        &audit_state,
        &credential,
        "POST",
        "/v1/servers/:server_id/content/source",
        response.status(),
    );
    response
}

pub(super) async fn review_update(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<msc_api::dto::ContentUpdateReviewRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return invalid_body(
                "invalid_json",
                "Provide the installed file and exact replacement release",
            );
        }
    };
    let current = match super::content_install::selected(&state, &server_id) {
        Ok(server) => server,
        Err(message) => return error_response(StatusCode::CONFLICT, "server_changed", message),
    };
    let operations = state.lifecycle.operations();
    let operation = match operations.begin_lifecycle(
        "addon-update-review",
        Some(server_id.clone()),
        "Reviewing content update",
    ) {
        Ok(operation) => operation,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let cancel = operations.cancellation_check(&operation);
    let identity = crate::auth::StreamAuthentication {
        credential: credential.clone(),
    };
    let audit_state = state.lifecycle.clone();
    let owner = credential.credential_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        msc_application::content_install::recover_activation(&current)?;
        let secrets =
            production_secret_store().map_err(|_| "Provider secret storage unavailable")?;
        let record = service::review_update(
            &current,
            &owner,
            service::UpdateSelection {
                path: &body.path,
                sha512: &body.sha512,
                project_id: body.project_id,
                version_id: body.version_id,
            },
            &HttpTransport::new(),
            secrets.as_ref(),
            &|| cancel() || !auth.stream_retains_permissions(&identity),
        )?;
        let fresh = server(&state, &server_id)?;
        if fresh != current || state.lifecycle.status_snapshot().running {
            return Err("Server changed during update review".into());
        }
        Ok::<_, String>(record)
    })
    .await;
    let response = match result {
        Ok(Ok(record)) => {
            let _ = operations.succeed(
                &operation,
                "Update review ready",
                BTreeMap::from([("reviewId".into(), record.id.clone())]),
            );
            Json(super::content_install::public_review(record)).into_response()
        }
        Ok(Err(error)) => {
            let _ = operations.fail(&operation, "update_review_failed", error.clone());
            error_response(StatusCode::CONFLICT, "update_review_failed", &error)
        }
        Err(_) => {
            let _ = operations.fail(
                &operation,
                "update_review_failed",
                "Update review worker failed".into(),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "update_review_failed",
                "Update review worker failed",
            )
        }
    };
    audit(
        &audit_state,
        &credential,
        "POST",
        "/v1/servers/:server_id/content/update-review",
        response.status(),
    );
    response
}
