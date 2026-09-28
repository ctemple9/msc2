//! Import, export, activation, and replacement route orchestration.

use super::*;

pub async fn import(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldImportRequestDto>>,
) -> Response {
    let lifecycle = &state.lifecycle;
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Some(Json(body)) = body else {
        return invalid_body("invalid_json", "Request body must be valid JSON.");
    };
    if body.name.trim().is_empty() {
        return invalid_body("name_required", "name must not be blank.");
    }
    let staged_upload_id = body.staged_upload_id.trim();
    let backup_id = body.backup_id.as_deref().map(str::trim);
    let has_staged_upload = !staged_upload_id.is_empty();
    let has_backup = backup_id.is_some_and(|id| !id.is_empty());
    if has_staged_upload == has_backup {
        return invalid_body(
            "invalid_body",
            "exactly one of stagedUploadId or backupId must be provided.",
        );
    }
    let server = match active_server_or_response(lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let (source_path, staged_path) = if has_staged_upload {
        let entry = {
            state
                .staging
                .uploads
                .lock()
                .unwrap()
                .remove(staged_upload_id)
        };
        let Some(entry) = entry else {
            return error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                "Unknown or already-redeemed staged upload.",
            );
        };
        if now_unix() > entry.expires_at_unix
            || !entry.complete
            || !matches!(entry.purpose, StagedUploadPurposeDto::WorldImport)
        {
            return error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                "Unknown or already-redeemed staged upload.",
            );
        }
        (entry.path.clone(), Some(entry.path))
    } else {
        let backup_id = backup_id.expect("exactly one source was validated above");
        let server_dir = Path::new(&server.server_dir);
        let Some(entry) = backups::list_backups(&StdFileSystem, server_dir)
            .into_iter()
            .find(|entry| entry.filename == backup_id)
        else {
            return error_response(StatusCode::NOT_FOUND, "backup_not_found", "No such backup.");
        };
        (entry.zip_path, None)
    };

    if let Err(error) =
        msc_infrastructure::archive::validate_world_archive(&source_path, server.server_type)
    {
        if let Some(staged_path) = staged_path {
            let _ = std::fs::remove_file(staged_path);
        }
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_world_archive",
            &error.to_string(),
        );
    }

    let operation_id =
        match begin_operation(lifecycle, &server.id, "world-import", "Importing world.") {
            Ok(id) => id,
            Err(response) => return response,
        };
    let now = iso8601_now();
    let response = match worlds::import_zip_as_new_slot(
        &StdFileSystem,
        Path::new(&server.server_dir),
        server.server_type,
        None,
        &source_path,
        body.name.trim(),
        &now,
    ) {
        Ok(_) => {
            let _ = lifecycle
                .operations()
                .succeed(&operation_id, "Imported.", BTreeMap::new());
            mutation_ok(lifecycle, &server, "imported")
        }
        Err(error) => {
            let _ = lifecycle
                .operations()
                .fail(&operation_id, "world_error", error.to_string());
            world_error_response(error)
        }
    };
    if let Some(staged_path) = staged_path {
        let _ = std::fs::remove_file(staged_path);
    }
    audit(
        lifecycle,
        &credential,
        "POST",
        "/v1/worlds/import",
        response.status(),
    );
    response
}

pub async fn export(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldExportRequestDto>>,
) -> Response {
    let lifecycle = &state.lifecycle;
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Some(Json(body)) = body else {
        return invalid_body("invalid_json", "Request body must be valid JSON.");
    };
    let server = match active_server_or_response(lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let server_dir = Path::new(&server.server_dir);
    let Some(slot) = find_slot(server_dir, &body.slot_id) else {
        return slot_not_found(&body.slot_id);
    };

    let id = Uuid::new_v4().to_string();
    let servers_root = lifecycle.servers_root();
    let downloads_dir = staging_root(&servers_root).join("downloads");
    if std::fs::create_dir_all(&downloads_dir).is_err() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Could not prepare staging directory.",
        );
    }
    let destination = downloads_dir.join(format!("{id}.zip"));

    let response = match worlds::export_slot_zip(&StdFileSystem, server_dir, &slot, &destination) {
        Ok(()) => {
            let size_bytes = std::fs::metadata(&destination)
                .map(|m| m.len())
                .unwrap_or(0);
            let expires_at_unix = now_unix() + STAGING_TTL_SECONDS;
            state.staging.downloads.lock().unwrap().insert(
                id.clone(),
                StagedDownload {
                    expires_at_unix,
                    path: destination,
                },
            );
            Json(WorldExportResultDto {
                staged_download_id: id,
                expires_at: unix_to_iso8601(expires_at_unix),
                size_bytes: size_bytes as i64,
            })
            .into_response()
        }
        Err(error) => world_error_response(error),
    };
    audit(
        lifecycle,
        &credential,
        "POST",
        "/v1/worlds/export",
        response.status(),
    );
    response
}

#[cfg(test)]
pub async fn download_staged_bytes(
    State(state): State<WorldsRoutesState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let entry = { state.staging.downloads.lock().unwrap().remove(&id) };
    let Some(entry) = entry else {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged download.",
        );
    };
    if now_unix() > entry.expires_at_unix {
        let _ = std::fs::remove_file(&entry.path);
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged download.",
        );
    }
    match std::fs::read(&entry.path) {
        Ok(bytes) => {
            let _ = std::fs::remove_file(&entry.path);
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "application/zip".parse().unwrap());
            (StatusCode::OK, headers, bytes).into_response()
        }
        Err(_) => error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged download.",
        ),
    }
}

// =====================================================================
// Async: activate
// =====================================================================

pub async fn activate(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<serde_json::Value>>,
) -> Response {
    let lifecycle = state.lifecycle.clone();
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Some(Json(raw_body)) = body else {
        return invalid_body("invalid_json", "Request body must be valid JSON.");
    };
    let confirmation = raw_body
        .get("confirmation")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let body = match serde_json::from_value::<WorldActivateRequestDto>(raw_body) {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_body", "Request body must be a world object."),
    };
    let server = match active_server_or_response(&lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    if lifecycle.status_snapshot().running {
        return error_response(StatusCode::CONFLICT, "server_running", "Server is running.");
    }
    let server_dir = Path::new(&server.server_dir).to_path_buf();
    let Some(slot) = find_slot(&server_dir, &body.slot_id) else {
        return error_response(
            StatusCode::CONFLICT,
            "conflict",
            "server_running_or_slot_not_found",
        );
    };
    let profile = world_store::load_profile(&StdFileSystem, &server_dir, &slot);
    if let Some(required) =
        world_safety::confirmation_for_world_profile(server.server_type, &profile)
        && !world_safety::is_confirmed(required, confirmation.as_deref())
    {
        return confirmation_required_response(required);
    }
    let operation_id = match begin_operation(
        &lifecycle,
        &server.id,
        "world-activate",
        "Activating world slot.",
    ) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let server_type = server.server_type;
    let raw_level_name =
        crate::backup_operations::configured_java_level_name(server_type, &server_dir);
    let running = lifecycle.status_snapshot().running;
    let task_lifecycle = lifecycle.clone();
    let task_operation_id = operation_id.clone();
    let should_cancel = lifecycle.operations().cancellation_check(&operation_id);
    let backup_should_cancel = should_cancel.clone();
    tokio::spawn(async move {
        let now = iso8601_now();
        let backup_lifecycle = task_lifecycle.clone();
        let backup_dir = server_dir.clone();
        let backup_type = server_type;
        let result = tokio::task::spawn_blocking(move || {
            worlds::activate_slot(
                &StdFileSystem,
                &backup_dir,
                backup_type,
                &slot,
                running,
                &now,
                || {
                    run_pre_mutation_safety_backup(
                        &backup_lifecycle,
                        &backup_dir,
                        backup_type,
                        raw_level_name.as_deref(),
                        &backup_should_cancel,
                    )
                },
                should_cancel,
            )
        })
        .await;
        match result {
            Ok(Ok(_)) => {
                let mut result = BTreeMap::new();
                result.insert("result".to_string(), "activated".to_string());
                let _ = task_lifecycle.operations().succeed(
                    &task_operation_id,
                    "Activation complete.",
                    result,
                );
            }
            Ok(Err(worlds::ActivationError::Cancelled)) => {
                let _ = task_lifecycle
                    .operations()
                    .cancel(&task_operation_id, "Activation cancelled.");
            }
            Ok(Err(error)) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "world_error",
                    error.to_string(),
                );
            }
            Err(_) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "internal_error",
                    "Activation task panicked.".to_string(),
                );
            }
        }
    });

    let response = Json(WorldActivateResultDto {
        result: "activation_started".to_string(),
        operation_id: Some(operation_id.as_str().to_string()),
    })
    .into_response();
    audit(
        &lifecycle,
        &credential,
        "POST",
        "/v1/worlds/activate",
        response.status(),
    );
    response
}

/// The mandatory pre-activation/pre-replace/pre-conversion safety backup
/// every P6.13/16/19 caller needs as a `impl FnOnce() -> bool` — a real
/// manual, tokened `backups::create_backup` call over the same server
/// directory, matching every other "already handled, report success"
/// caller in this codebase.
pub(super) fn run_pre_mutation_safety_backup(
    lifecycle: &LifecycleRoutesState,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    should_cancel: impl Fn() -> bool,
) -> bool {
    let _ = lifecycle;
    let now = iso8601_now();
    let association = msc_domain::world::BackupAssociation {
        slot_id: None,
        slot_name: None,
        world_seed: None,
    };
    msc_application::backups::create_backup(
        &StdFileSystem,
        server_dir,
        server_type,
        raw_level_name,
        &association,
        None,
        None,
        false,
        true,
        Some("pre-mutation"),
        None,
        &now,
        None,
        || false,
        should_cancel,
    )
    .is_ok()
}

// =====================================================================
// Async: replace-active-world (P6.34) — exposes P6.33's transactional
// `worlds::replace_world` through the agent, separately named from
// `POST /v1/worlds/replace` (`replace` above, a saved-slot-to-saved-slot
// copy — `phase6-api.md` SS9/SS10 records why they're distinct
// operations). Follows `routes/backups.rs::restore`'s shape — a
// mandatory safety backup plus a transactional live-world swap, guard-
// ordered the same way (cheap up-front checks, then a journaled
// operation, then the real work on a spawned blocking task) — rather
// than `activate`'s, since restore is the closer existing analog.
// =====================================================================

pub async fn replace_active(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldReplaceActiveRequestDto>>,
) -> Response {
    let lifecycle = state.lifecycle.clone();
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Some(Json(body)) = body else {
        return invalid_body("invalid_json", "Request body must be valid JSON.");
    };
    if body.new_level_name.trim().is_empty() {
        return invalid_body("name_required", "newLevelName must not be blank.");
    }
    let server = match active_server_or_response(&lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let server_dir = Path::new(&server.server_dir).to_path_buf();

    // Cheap, up-front checks that need no journaled operation to observe
    // them — mirroring `restore`'s own "running-server, then missing-
    // source" ordering. `replace_world` re-checks running-server itself
    // (`false` is passed below because this guard already refused
    // otherwise), the same "outer route pre-checks, inner service
    // re-checks" belt-and-braces `restore` already has.
    if lifecycle.status_snapshot().running {
        return error_response(StatusCode::CONFLICT, "server_running", "Server is running.");
    }

    // Redeem the staged upload up front (if any) — the same "missing,
    // expired, or wrong-purpose staged id is a plain 404" contract
    // `import` already established. Never an arbitrary server-local
    // path: the only sources this route can ever build are `Fresh` (no
    // upload given) or `BackupZip` (an uploaded, bounded, server-
    // generated staging path) — `WorldReplaceSource::ExistingFolder`
    // is unreachable from this route.
    let world_source = if let Some(staged_upload_id) = &body.staged_upload_id {
        let entry = {
            state
                .staging
                .uploads
                .lock()
                .unwrap()
                .remove(staged_upload_id)
        };
        let Some(entry) = entry else {
            return error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                "Unknown or already-redeemed staged upload.",
            );
        };
        if now_unix() > entry.expires_at_unix
            || !entry.complete
            || !matches!(entry.purpose, StagedUploadPurposeDto::ActiveWorldReplace)
        {
            return error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                "Unknown or already-redeemed staged upload.",
            );
        }
        WorldReplaceSource::BackupZip(entry.path)
    } else {
        WorldReplaceSource::Fresh
    };

    // Resolve the mandatory safety backup's association before spawning
    // — the same "load slots/active id up front, hand the association to
    // the background task" shape `restore` already uses.
    let slots = world_store::load_slots(&StdFileSystem, &server_dir);
    let marker = world_store::load_explicit_active_slot_id(&StdFileSystem, &server_dir);
    let active_id = msc_domain::world::resolve_active_slot_id(&slots, marker.as_deref());
    let association =
        msc_domain::world::effective_backup_association(&slots, active_id.as_deref(), None, None);

    let operation_id = match begin_operation(
        &lifecycle,
        &server.id,
        "world-replace-active",
        "Replacing active world.",
    ) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let server_type = server.server_type;
    let raw_level_name =
        crate::backup_operations::configured_java_level_name(server_type, &server_dir);
    let server_id = server.id.clone();
    let server_name = server.display_name.clone();
    let new_level_name = body.new_level_name.trim().to_string();
    let task_lifecycle = lifecycle.clone();
    let task_operation_id = operation_id.clone();
    let should_cancel = lifecycle.operations().cancellation_check(&operation_id);
    tokio::spawn(async move {
        let now = iso8601_now();
        let result = tokio::task::spawn_blocking(move || {
            worlds::replace_world(
                &StdFileSystem,
                &server_dir,
                server_type,
                raw_level_name.as_deref(),
                &new_level_name,
                &world_source,
                false,
                &association,
                Some(&server_id),
                Some(&server_name),
                &now,
                should_cancel,
            )
        })
        .await;
        match result {
            Ok(Ok(_)) => {
                let mut result = BTreeMap::new();
                result.insert("result".to_string(), "replaced".to_string());
                let _ = task_lifecycle.operations().succeed(
                    &task_operation_id,
                    "Replacement complete.",
                    result,
                );
            }
            Ok(Err(WorldError::Cancelled)) => {
                let _ = task_lifecycle
                    .operations()
                    .cancel(&task_operation_id, "Replacement cancelled.");
            }
            Ok(Err(error)) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "world_error",
                    error.to_string(),
                );
            }
            Err(_) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "internal_error",
                    "Replacement task panicked.".to_string(),
                );
            }
        }
    });

    let response = Json(WorldReplaceActiveResultDto {
        result: "replace_started".to_string(),
        operation_id: Some(operation_id.as_str().to_string()),
    })
    .into_response();
    audit(
        &lifecycle,
        &credential,
        "POST",
        "/v1/worlds/replace-active-world",
        response.status(),
    );
    response
}
