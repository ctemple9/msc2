//! Retained player-file recovery does not install bytes into a running server.
use super::*;
use msc_api::dto::{RetainedPackManualDto, RetainedPackMissingDto, RetainedPackStatusDto};
use msc_application::pack_retention as service;

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
fn active_pack(
    server: &msc_domain::app_config_schema::ConfigServer,
    pack_id: &str,
) -> Result<(), String> {
    if server
        .content_collection
        .as_ref()
        .and_then(|collection| collection.extra.get("basePackId"))
        .and_then(serde_json::Value::as_str)
        != Some(pack_id)
    {
        return Err("Only the current base pack can receive player-file recovery".into());
    }
    Ok(())
}
fn binding(pack_id: &str, expected: &service::ExpectedPackFile) -> Result<String, String> {
    let encoded = serde_json::to_vec(expected).map_err(|error| error.to_string())?;
    Ok(format!("{pack_id}:{}", sha512_hex(&encoded)))
}
fn status(pack: service::RetainedPack) -> RetainedPackStatusDto {
    RetainedPackStatusDto {
        source_server_pack: pack.origin.as_ref().and_then(|origin| origin.server_pack),
        player_recovery_required: pack
            .origin
            .as_ref()
            .is_some_and(|origin| origin.server_pack == Some(true)),
        pack_id: pack.id,
        name: pack.name,
        version: pack.version,
        original_sha512: pack.original.sha512,
        resolved_file_count: pack.resolved_files.len(),
        files_complete: pack.unresolved.is_empty(),
        missing: pack
            .unresolved
            .into_iter()
            .map(|(path, reason)| {
                let expected = pack.expected_files.get(&path);
                RetainedPackMissingDto {
                    path,
                    reason,
                    byte_size: expected.map(|file| file.byte_size),
                    manual_recovery_available: expected.is_some_and(|file| {
                        file.byte_size > 0
                            && file.byte_size <= 512 * 1024 * 1024
                            && (file.sha1.is_some() || file.sha512.is_some())
                    }),
                }
            })
            .collect(),
    }
}

pub(super) async fn get_status(
    State(state): State<ComponentsRoutesState>,
    Extension(_credential): Extension<AuthenticatedCredential>,
    AxumPath((server_id, pack_id)): AxumPath<(String, String)>,
) -> Response {
    let result =
        tokio::task::spawn_blocking(move || service::load(&server(&state, &server_id)?, &pack_id))
            .await;
    match result {
        Ok(Ok(pack)) => Json(status(pack)).into_response(),
        Ok(Err(error)) => error_response(StatusCode::CONFLICT, "retained_pack_unavailable", &error),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "retained_pack_unavailable",
            "Retained pack reader failed",
        ),
    }
}

pub(super) async fn admit(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath((server_id, pack_id)): AxumPath<(String, String)>,
    body: Result<Json<RetainedPackManualDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Provide a retained manifest path"),
    };
    let loaded = tokio::task::spawn_blocking(move || {
        let server = server(&state, &server_id)?;
        active_pack(&server, &pack_id)?;
        let pack = service::load(&server, &pack_id)?;
        if !pack.unresolved.contains_key(&body.path) {
            return Err("Retained file is not awaiting recovery".into());
        }
        let expected = pack
            .expected_files
            .get(&body.path)
            .ok_or("Resolve provider metadata before manual recovery")?
            .clone();
        if expected.byte_size == 0
            || expected.byte_size > 512 * 1024 * 1024
            || (expected.sha1.is_none() && expected.sha512.is_none())
        {
            return Err("This retained file has no supported exact identity".into());
        }
        Ok::<_, String>((state, pack_id, expected))
    })
    .await;
    let (state, pack_id, expected) = match loaded {
        Ok(Ok(loaded)) => loaded,
        Ok(Err(error)) => return invalid_body("manual_recovery_unavailable", &error),
        Err(_) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "manual_recovery_unavailable",
                "Retained pack reader failed",
            );
        }
    };
    let token = match binding(&pack_id, &expected) {
        Ok(token) => token,
        Err(error) => return invalid_body("invalid_manifest", &error),
    };
    let id = Uuid::new_v4().to_string();
    let expiry = now_unix() + STAGING_TTL_SECONDS;
    state.staging.uploads.lock().unwrap().insert(
        id.clone(),
        StagedUpload {
            provider_owner: Some(credential.credential_id.clone()),
            pack_origin: None,
            native_guard: None,
            purpose: StagedUploadPurposeDto::ModpackUnresolvedFile,
            file_name: Path::new(&expected.path)
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned),
            operation_id: Some(token),
            file_id: Some(expected.path),
            expires_at_unix: expiry,
            max_bytes: expected.byte_size,
            expected_bytes: Some(expected.byte_size),
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
        "/v1/servers/:server_id/content/retained-packs/:pack_id/manual/admit",
        StatusCode::OK,
    );
    Json(StagedUploadBeginResultDto {
        staged_upload_id: id.clone(),
        upload_path: format!("/v1/staged-uploads/{id}"),
        expires_at: unix_to_iso8601(expiry),
        max_bytes: expected.byte_size as i64,
        max_chunk_bytes: Some(STAGED_UPLOAD_CHUNK_BYTES as i64),
    })
    .into_response()
}

pub(super) async fn complete(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath((server_id, pack_id)): AxumPath<(String, String)>,
    body: Result<Json<RetainedPackManualDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Provide the retained file upload"),
    };
    let Some(id) = body.staged_upload_id else {
        return invalid_body("missing_upload", "stagedUploadId is required");
    };
    let Some(entry) = state.staging.uploads.lock().unwrap().get(&id).cloned() else {
        return invalid_body(
            "unknown_upload",
            "Upload is unavailable or belongs to another credential",
        );
    };
    if !entry.complete
        || entry.expires_at_unix < now_unix()
        || entry.provider_owner.as_deref() != Some(credential.credential_id.as_str())
        || entry.file_id.as_deref() != Some(body.path.as_str())
        || !matches!(entry.purpose, StagedUploadPurposeDto::ModpackUnresolvedFile)
    {
        return invalid_body("invalid_upload", "Upload does not match this retained file");
    }
    let operations = state.lifecycle.operations();
    let operation = match operations.begin_lifecycle(
        "addon-retained-recovery",
        Some(server_id.clone()),
        "Recovering retained player content",
    ) {
        Ok(operation) => operation,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let audit_state = state.lifecycle.clone();
    let identity = crate::auth::StreamAuthentication {
        credential: credential.clone(),
    };
    let result: Result<RetainedPackStatusDto, String> = (async {
        // Redemption remains within the authenticated request scope.
        let server = server(&state, &server_id)?;
        active_pack(&server, &pack_id)?;
        let pack = service::load(&server, &pack_id)?;
        let expected = pack
            .expected_files
            .get(&body.path)
            .ok_or("Retained manifest file disappeared")?;
        if entry.operation_id.as_deref() != Some(binding(&pack_id, expected)?.as_str()) {
            return Err("Retained manifest changed after upload admission".into());
        }
        if state.staging.uploads.lock().unwrap().remove(&id).is_none() {
            return Err("Upload already consumed".into());
        }
        tokio::task::spawn_blocking(move || {
            let result = (|| {
                let mut pack = pack;
                if !auth.stream_retains_permissions(&identity) {
                    return Err("Recovery permission was revoked".into());
                }
                service::accept_manual_file(
                    Path::new(&server.server_dir),
                    &mut pack,
                    &body.path,
                    &entry.path,
                )?;
                let collection = service::collection(
                    Path::new(&server.server_dir),
                    &pack,
                    server.content_collection.as_ref(),
                )?;
                state
                    .lifecycle
                    .try_mutate_config(|config| {
                        if !auth.stream_retains_permissions(&identity) {
                            return Err("Recovery permission was revoked");
                        }
                        let current = config
                            .servers
                            .iter_mut()
                            .find(|current| current.id == server_id)
                            .ok_or("Server removed")?;
                        if current != &server {
                            return Err("Server changed during recovery");
                        }
                        current.content_collection = Some(collection);
                        Ok(())
                    })
                    .map_err(|_| "Could not save recovered player collection".to_string())?;
                Ok(status(pack))
            })();
            let _ = std::fs::remove_file(entry.path);
            result
        })
        .await
        .map_err(|_| "Recovery worker failed".to_string())?
    })
    .await;
    let response = match result {
        Ok(result) => {
            let _ = operations.succeed(&operation, "Retained file recovered", BTreeMap::new());
            Json(result).into_response()
        }
        Err(error) => {
            let _ = operations.fail(&operation, "retained_recovery_failed", error.clone());
            error_response(StatusCode::CONFLICT, "retained_recovery_failed", &error)
        }
    };
    audit(
        &audit_state,
        &credential,
        "POST",
        "/v1/servers/:server_id/content/retained-packs/:pack_id/manual/complete",
        response.status(),
    );
    response
}

pub(super) async fn retry(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath((server_id, pack_id)): AxumPath<(String, String)>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let current = match server(&state, &server_id) {
        Ok(server) => server,
        Err(error) => return invalid_body("server_unavailable", &error),
    };
    if let Err(error) = active_pack(&current, &pack_id) {
        return invalid_body("retained_pack_unavailable", &error);
    }
    let operations = state.lifecycle.operations();
    let operation = match operations.begin_lifecycle(
        "addon-retained-retry",
        Some(server_id.clone()),
        "Recovering retained pack content",
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
        "/v1/servers/:server_id/content/retained-packs/:pack_id/retry",
        response.status(),
    );
    tokio::task::spawn_blocking(move || {
        let identity = crate::auth::StreamAuthentication { credential };
        let cancellation = operations.cancellation_check(&operation);
        let cancelled = || cancellation() || !auth.stream_retains_permissions(&identity);
        let result = (|| {
            let mut pack = service::load(&current, &pack_id)?;
            let secrets = production_secret_store()
                .map_err(|_| "Provider secret storage unavailable".to_string())?;
            service::retry_resolution(
                Path::new(&current.server_dir),
                &mut pack,
                &HttpTransport::new(),
                secrets.as_ref(),
                &cancelled,
            )?;
            if cancelled() {
                return Err("Pack recovery cancelled or permission revoked".into());
            }
            let collection = service::collection(
                Path::new(&current.server_dir),
                &pack,
                current.content_collection.as_ref(),
            )?;
            state
                .lifecycle
                .try_mutate_config(|config| {
                    if cancelled() {
                        return Err("Recovery cancelled");
                    }
                    let stored = config
                        .servers
                        .iter_mut()
                        .find(|server| server.id == server_id)
                        .ok_or("Server removed")?;
                    if stored != &current {
                        return Err("Server changed during recovery");
                    }
                    stored.content_collection = Some(collection);
                    Ok(())
                })
                .map_err(|_| "Could not save recovered player collection".to_string())?;
            Ok::<_, String>(pack)
        })();
        match result {
            Ok(pack) => {
                let mut result = BTreeMap::new();
                result.insert("packId".into(), pack.id);
                result.insert(
                    "filesComplete".into(),
                    pack.unresolved.is_empty().to_string(),
                );
                let _ = operations.succeed(
                    &operation,
                    "Retained pack recovery finished; inspect missing files before publishing",
                    result,
                );
            }
            Err(error) => {
                let _ = operations.fail(&operation, "retained_recovery_failed", error);
            }
        }
    });
    response
}
