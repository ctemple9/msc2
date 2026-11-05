//! Reviewed installs bind an explicit server, credential and filesystem baseline.
use super::*;
use msc_api::dto::{
    ContentInstallApplyDto, ContentInstallFileDto, ContentInstallManualDto,
    ContentInstallReviewDto, ContentInstallReviewRequestDto,
};
use msc_application::content_install as service;
use msc_domain::app_config_schema::ConfigServer;

pub(super) fn selected(
    state: &ComponentsRoutesState,
    server_id: &str,
) -> Result<ConfigServer, &'static str> {
    let server = state
        .lifecycle
        .active_config_server()
        .filter(|server| server.id == server_id)
        .ok_or("Select the reviewed server before continuing")?;
    if state.lifecycle.status_snapshot().running {
        return Err("Stop the server before changing content");
    }
    Ok(server)
}

pub(super) fn public_review(record: service::StoredInstallReview) -> ContentInstallReviewDto {
    ContentInstallReviewDto {
        review_id: record.id,
        server_id: record.server_id,
        provider: record.review.provider,
        collection_revision: record.collection_revision,
        warnings: record.review.warnings,
        replacements: record.replacements,
        files: record
            .review
            .files
            .into_iter()
            .map(|file| ContentInstallFileDto {
                project_id: file.project_id,
                version_id: file.version_id,
                title: file.title,
                file_name: file.file_name,
                byte_size: file.byte_size,
                sha1: file.sha1,
                manual_download_required: file.download_url.is_none(),
                distribution_allowed: file.distribution_allowed,
                required_projects: file.required_projects,
                incompatible_projects: file.incompatible_projects,
            })
            .collect(),
    }
}

pub(super) async fn review(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<ContentInstallReviewRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Provide an exact provider release"),
    };
    if body.provider != "curseforge" || body.project_id == 0 || body.version_id == 0 {
        return invalid_body(
            "invalid_source",
            "Choose an exact CurseForge release; existing Modrinth installs retain their endpoint",
        );
    }
    let server = match selected(&state, &server_id) {
        Ok(server) => server,
        Err(message) => return error_response(StatusCode::CONFLICT, "server_changed", message),
    };
    let operations = state.lifecycle.operations();
    let review_operation = match operations.begin_lifecycle(
        "addon-install-review",
        Some(server_id.clone()),
        "Reviewing required content",
    ) {
        Ok(operation) => operation,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let review_cancel = operations.cancellation_check(&review_operation);
    let identity = crate::auth::StreamAuthentication {
        credential: credential.clone(),
    };
    let result = tokio::task::spawn_blocking(move || {
        service::recover_activation(&server)?;
        let baseline = service::content_baseline(&server)?;
        let secrets = production_secret_store().map_err(|_| "Provider secret storage unavailable".to_string())?;
        let version = server.minecraft_version.clone().ok_or("Select an exact Minecraft version")?;
        let pins = msc_application::content_sources::verified_pins(&server)?;
        let mut review = service::review_curseforge_with_pins(&HttpTransport::new(), secrets.as_ref(), body.project_id,
            body.version_id, service::ReviewContext { flavor: server.java_flavor, game_version: &version, installed: &pins },
            &|| review_cancel() || !auth.stream_retains_permissions(&identity)).map_err(|error| error.to_string())?;
        if let Some(collection) = &server.content_collection {
            for file in &review.files {
                for installed in collection.current_files.values().filter_map(|entry| entry.source.as_ref()).filter(|source| source.provider == "curseforge") {
                    if file.incompatible_projects.contains(&installed.project_id) {
                        return Err(format!("{} conflicts with installed project {}", file.title, installed.project_id));
                    }
                    if installed.project_id == file.project_id && installed.version_id != file.version_id {
                        return Err(format!("{} is already installed at another version; use update review", file.title));
                    }
                }
            }
        }
        if review.files.iter().any(|file| baseline.contains_key(&file.file_name)) {
            review.warnings.push("Existing filenames will only be accepted if their bytes match the selected release; other files are preserved".into());
        }
        let fresh = selected(&state, &server_id).map_err(|_| "Server selection or state changed".to_string())?;
        if fresh != server || service::content_baseline(&fresh)? != baseline { return Err("Server changed during review".into()); }
        let record = service::StoredInstallReview {
            id: Uuid::new_v4().to_string(), owner: credential.credential_id,
            server_id, server_dir: server.server_dir, minecraft_version: version,
            flavor: server.java_flavor.raw_value().into(), created_unix: now_unix(),
            collection_revision: server.content_collection.as_ref().map_or(0, |collection| collection.revision),
            baseline, review, replacements: BTreeMap::new(),
        };
        service::store_review(&record)?;
        Ok::<_, String>(record)
    }).await;
    match result {
        Ok(Ok(record)) => {
            let _ = operations.succeed(
                &review_operation,
                "Content review ready",
                BTreeMap::from([("reviewId".into(), record.id.clone())]),
            );
            Json(public_review(record)).into_response()
        }
        Ok(Err(error)) => {
            let _ = operations.fail(&review_operation, "review_failed", error.clone());
            error_response(StatusCode::CONFLICT, "review_failed", &error)
        }
        Err(_) => {
            let _ = operations.fail(
                &review_operation,
                "review_failed",
                "Content review worker failed".into(),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "review_failed",
                "Content review worker failed",
            )
        }
    }
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
    let server = match selected(&state, &server_id) {
        Ok(server) => server,
        Err(message) => return error_response(StatusCode::CONFLICT, "server_changed", message),
    };
    let operation = match state.lifecycle.operations().begin_lifecycle(
        "addon-reviewed-install",
        Some(server_id.clone()),
        "Staging reviewed content",
    ) {
        Ok(operation) => operation,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let operation_id = operation.as_str().to_owned();
    let cancel = state.lifecycle.operations().cancellation_check(&operation);
    let identity = crate::auth::StreamAuthentication {
        credential: credential.clone(),
    };
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/servers/:server_id/content/install",
        StatusCode::ACCEPTED,
    );
    tokio::task::spawn_blocking(move || {
        let cancelled = || {
            cancel()
                || !auth.stream_retains_permissions(&identity)
                || state.lifecycle.active_server_id().as_deref() != Some(server_id.as_str())
                || state.lifecycle.status_snapshot().running
        };
        let result = (|| {
            service::recover_activation(&server)?;
            let record = service::load_review(&server, &body.review_id, &credential.credential_id)?;
            let secrets = production_secret_store()
                .map_err(|_| "Provider secret storage unavailable".to_string())?;
            let missing = service::stage_files(&record, secrets.as_ref(), &cancelled)?;
            if !missing.is_empty() {
                return Err(format!(
                    "Manual files required before installation: {}",
                    missing.join(", ")
                ));
            }
            if cancelled() {
                return Err("Content installation cancelled or server state changed".into());
            }
            let fresh = selected(&state, &server_id).map_err(|_| "Server changed".to_string())?;
            service::load_review(&fresh, &body.review_id, &credential.credential_id)?;
            service::activate(&fresh, &record, &cancelled, &mut |collection| {
                state
                    .lifecycle
                    .try_mutate_config(|config| {
                        if config.active_server_id.as_deref() != Some(server_id.as_str()) {
                            return Err("Server selection changed");
                        }
                        let server = config
                            .servers
                            .iter_mut()
                            .find(|server| server.id == server_id)
                            .ok_or("Server removed")?;
                        if server != &fresh {
                            return Err("Server configuration changed during activation");
                        }
                        server.content_collection = Some(collection);
                        Ok(())
                    })
                    .map_err(|_| "Could not persist reviewed content metadata".to_string())
            })
        })();
        match result {
            Ok(()) => {
                let _ = state.lifecycle.finish_operation_success(
                    &operation,
                    "Installed reviewed content",
                    BTreeMap::new(),
                );
            }
            Err(error) if cancel() => {
                let _ = state
                    .lifecycle
                    .operations()
                    .cancel(&operation, &format!("Cancelled; {error}"));
            }
            Err(error) => {
                let _ = state.lifecycle.finish_operation_failure(
                    &operation,
                    "reviewed_install_failed",
                    error,
                );
            }
        }
    });
    (
        StatusCode::ACCEPTED,
        Json(msc_api::dto::CatalogAcquireResultDto { operation_id }),
    )
        .into_response()
}

pub(super) async fn manual_admission(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<ContentInstallManualDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Provide reviewId and versionId"),
    };
    let server = match selected(&state, &server_id) {
        Ok(server) => server,
        Err(message) => return error_response(StatusCode::CONFLICT, "server_changed", message),
    };
    let record = match service::load_review(&server, &body.review_id, &credential.credential_id) {
        Ok(record) => record,
        Err(error) => return error_response(StatusCode::CONFLICT, "review_changed", &error),
    };
    let Some(file) = record
        .review
        .files
        .iter()
        .find(|file| file.version_id == body.version_id)
    else {
        return invalid_body("unknown_file", "File does not belong to this review");
    };
    let id = Uuid::new_v4().to_string();
    let expiry = now_unix() + STAGING_TTL_SECONDS;
    state.staging.uploads.lock().unwrap().insert(
        id.clone(),
        StagedUpload {
            provider_owner: Some(credential.credential_id),
            native_guard: None,
            purpose: StagedUploadPurposeDto::CurseforgeManualFile,
            file_name: Some(file.file_name.clone()),
            operation_id: Some(body.review_id),
            file_id: Some(body.version_id),
            expires_at_unix: expiry,
            max_bytes: file.byte_size,
            expected_bytes: Some(file.byte_size),
            received_bytes: 0,
            complete: false,
            path: staging_root(&state.lifecycle.servers_root())
                .join("uploads")
                .join(format!("{id}.bin")),
        },
    );
    Json(StagedUploadBeginResultDto {
        staged_upload_id: id.clone(),
        upload_path: format!("/v1/staged-uploads/{id}"),
        expires_at: unix_to_iso8601(expiry),
        max_bytes: file.byte_size as i64,
        max_chunk_bytes: Some(STAGED_UPLOAD_CHUNK_BYTES as i64),
    })
    .into_response()
}

pub(super) async fn manual_complete(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<ContentInstallManualDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Provide the reviewed upload identity"),
    };
    let server = match selected(&state, &server_id) {
        Ok(server) => server,
        Err(message) => return error_response(StatusCode::CONFLICT, "server_changed", message),
    };
    let Some(id) = body.staged_upload_id else {
        return invalid_body("missing_upload", "stagedUploadId is required");
    };
    let entry = state.staging.uploads.lock().unwrap().get(&id).cloned();
    let Some(entry) = entry.filter(|entry| {
        entry.complete
            && entry.expires_at_unix >= now_unix()
            && entry.provider_owner.as_deref() == Some(credential.credential_id.as_str())
            && entry.operation_id.as_deref() == Some(body.review_id.as_str())
            && entry.file_id.as_deref() == Some(body.version_id.as_str())
            && matches!(entry.purpose, StagedUploadPurposeDto::CurseforgeManualFile)
    }) else {
        return invalid_body(
            "invalid_upload",
            "Upload does not belong to this reviewed file",
        );
    };
    if state.staging.uploads.lock().unwrap().remove(&id).is_none() {
        return error_response(
            StatusCode::CONFLICT,
            "upload_consumed",
            "Upload was already consumed",
        );
    }
    let result = tokio::task::spawn_blocking(move || {
        let result = service::load_review(&server, &body.review_id, &credential.credential_id)
            .and_then(|record| {
                service::accept_manual(
                    &record,
                    &body.version_id,
                    &entry.path,
                    entry.file_name.as_deref().unwrap_or(""),
                )
            });
        let _ = std::fs::remove_file(entry.path);
        result
    })
    .await;
    match result {
        Ok(Ok(())) => Json(serde_json::json!({"success": true})).into_response(),
        Ok(Err(error)) => invalid_body("manual_file_rejected", &error),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "manual_file_failed",
            "Manual file worker failed",
        ),
    }
}
