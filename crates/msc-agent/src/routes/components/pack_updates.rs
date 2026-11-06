//! Pack review and activation keep server selection, operation locking and
//! permission revalidation at the boundary used by desktop and terminal clients.
use super::*;
use msc_api::dto::{ContentInstallApplyDto, PackUpdateReviewDto};
use msc_application::{pack_activation, pack_retention, pack_review as service};
fn public_review(review: service::Review) -> serde_json::Value {
    let missing_files = review.retained_pack.unresolved.iter().map(|(path, reason)| {
        let expected = review.retained_pack.expected_files.get(path);
        serde_json::json!({"path":path,"reason":reason,"byteSize":expected.map(|file| file.byte_size),"manualRecoveryAvailable":expected.is_some_and(|file| file.byte_size > 0 && file.byte_size <= 512*1024*1024 && (file.sha1.is_some() || file.sha512.is_some()))})
    }).collect::<Vec<_>>();
    serde_json::json!({"missingFiles":missing_files,"replacement":review.replacement,"reviewId":review.id,"serverId":review.activation.server_id,"collectionRevision":review.activation.collection_revision,"packId":review.retained_pack.id,"name":review.retained_pack.name,"version":review.retained_pack.version,"serverChanges":review.server_changes,"playerChanges":review.player_changes,"missing":review.missing,"warnings":review.warnings,"collection":review.activation.collection})
}
pub(super) async fn get(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath((server_id, review_id)): AxumPath<(String, String)>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let result = tokio::task::spawn_blocking(move || {
        let server = state
            .lifecycle
            .app_config_servers()
            .into_iter()
            .find(|server| server.id == server_id)
            .ok_or("Server not found")?;
        service::load(&server, &review_id, &credential.credential_id)
    })
    .await;
    match result {
        Ok(Ok(review)) => Json(public_review(review)).into_response(),
        Ok(Err(error)) => error_response(StatusCode::CONFLICT, "pack_review_unavailable", &error),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "pack_review_unavailable",
            "Pack review reader failed",
        ),
    }
}
pub(super) async fn review(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<PackUpdateReviewDto>, JsonRejection>,
) -> Response {
    review_mode(
        State(state),
        Extension(credential),
        Extension(auth),
        AxumPath(server_id),
        body,
        false,
    )
    .await
}

pub(super) async fn replacement_review(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<PackUpdateReviewDto>, JsonRejection>,
) -> Response {
    review_mode(
        State(state),
        Extension(credential),
        Extension(auth),
        AxumPath(server_id),
        body,
        true,
    )
    .await
}

async fn review_mode(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<PackUpdateReviewDto>, JsonRejection>,
    replacement: bool,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return invalid_body(
                "invalid_json",
                "Provide an exact pack archive, revision and restoration choices",
            );
        }
    };
    let server = match content_install::selected(&state, &server_id) {
        Ok(server) => server,
        Err(error) => return invalid_body("server_unavailable", error),
    };
    if body.staged_upload_id.is_some() == body.previous_review_id.is_some() {
        return invalid_body(
            "ambiguous_pack",
            "Choose exactly one uploaded archive or previous review",
        );
    }
    let audiences: BTreeMap<String, msc_domain::content_collection::ContentAudience> =
        match serde_json::to_value(&body.audiences).and_then(serde_json::from_value) {
            Ok(value) => value,
            Err(_) => {
                return invalid_body(
                    "invalid_audience",
                    "Choose server, players or both for each classification",
                );
            }
        };
    let mut lease = None;
    let upload = if let Some(id) = &body.staged_upload_id {
        if !state.staging.native.target_matches(id, &server_id) {
            return invalid_body("target_mismatch", "Pack upload belongs to another server");
        }
        let Some(entry) = state.staging.uploads.lock().unwrap().get(id).cloned() else {
            return invalid_body("upload_unavailable", "Pack archive upload is unavailable");
        };
        if !entry.complete
            || entry.expires_at_unix < now_unix()
            || !matches!(entry.purpose, StagedUploadPurposeDto::ModpackArchive)
            || entry
                .provider_owner
                .as_ref()
                .is_some_and(|owner| owner != &credential.credential_id)
        {
            return invalid_body(
                "upload_unavailable",
                "Pack archive is incomplete, expired or belongs to another credential",
            );
        }
        lease = state.staging.native.inspection_lease(id, &credential);
        Some(entry)
    } else {
        None
    };
    let operations = state.lifecycle.operations();
    let operation = match operations.begin_lifecycle(
        if replacement {
            "addon-pack-replacement-review"
        } else {
            "addon-pack-update-review"
        },
        Some(server_id.clone()),
        if replacement {
            "Preparing base pack replacement review"
        } else {
            "Preparing pack update review"
        },
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
        if replacement {
            "/v1/servers/:server_id/content/pack-replacement-review"
        } else {
            "/v1/servers/:server_id/content/pack-update-review"
        },
        response.status(),
    );
    tokio::task::spawn_blocking(move || {
        let _lease = lease;
        let identity = crate::auth::StreamAuthentication {
            credential: credential.clone(),
        };
        let cancel = operations.cancellation_check(&operation);
        let cancelled = || {
            cancel()
                || !auth.stream_retains_permissions(&identity)
                || content_install::selected(&state, &server_id).is_err()
        };
        let mut staging = None;
        let result = (|| {
            if cancelled() {
                return Err("Pack review cancelled or server/permission changed".into());
            }
            msc_application::content_install::recover_activation(&server)?;
            let secrets = production_secret_store()
                .map_err(|_| "Provider secret storage unavailable".to_string())?;
            let transport = HttpTransport::new();
            let retained = if let Some(id) = &body.previous_review_id {
                let previous = service::load_reference(&server, id, &credential.credential_id)?;
                if previous.replacement != replacement {
                    return Err("Previous review belongs to a different pack action".into());
                }
                Some(service::retained_for_review(&server, &previous)?)
            } else {
                None
            };
            let inspection = if let Some(pack) = &retained {
                pack_retention::inspect_original(
                    Path::new(&server.server_dir),
                    pack,
                    &transport,
                    secrets.as_ref(),
                    &cancelled,
                )?
            } else {
                let entry = upload.as_ref().ok_or("Pack upload disappeared")?;
                let mut inspection = modpacks::inspect_staged_archive(
                    &StdFileSystem,
                    &transport,
                    secrets.as_ref(),
                    &entry.path,
                    &staging_root(&state.lifecycle.servers_root()).join("pack-updates"),
                    operation.as_str(),
                )
                .map_err(|error| error.to_string())?;
                inspection.origin = entry.pack_origin.clone();
                inspection
            };
            staging = Some(inspection.staged_dir.clone());
            let review = if replacement {
                service::prepare_replacement(
                    &server,
                    &credential.credential_id,
                    body.collection_revision,
                    &inspection,
                    retained,
                    &body.restore_server,
                    &body.restore_player,
                    &audiences,
                    body.confirm_unverified_origin,
                    &service::ReplacementChoices {
                        confirm_world_compatibility: body.confirm_world_compatibility,
                        preserve_additions: body.preserve_additions.clone(),
                    },
                    &transport,
                    secrets.as_ref(),
                    &cancelled,
                )?
            } else {
                service::prepare(
                    &server,
                    &credential.credential_id,
                    body.collection_revision,
                    &inspection,
                    retained,
                    &body.restore_server,
                    &body.restore_player,
                    &audiences,
                    body.confirm_unverified_origin,
                    &transport,
                    secrets.as_ref(),
                    &cancelled,
                )?
            };
            let fresh = content_install::selected(&state, &server_id).map_err(str::to_string)?;
            if fresh != server || cancelled() {
                return Err("Server changed during pack review".into());
            }
            service::load(&fresh, &review.id, &credential.credential_id)?;
            Ok::<_, String>(review)
        })();
        if let Some(directory) = staging {
            let _ = std::fs::remove_dir_all(directory);
        }
        match result {
            Ok(review) => {
                let mut result = BTreeMap::new();
                result.insert("reviewId".into(), review.id);
                let _ = operations.succeed(
                    &operation,
                    "Pack review ready; inspect conflicts and missing content before applying",
                    result,
                );
            }
            Err(error) => {
                let _ = operations.fail(&operation, "pack_review_failed", error);
            }
        }
    });
    response
}
pub(super) async fn apply(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<ContentInstallApplyDto>, JsonRejection>,
) -> Response {
    apply_mode(
        State(state),
        Extension(credential),
        Extension(auth),
        AxumPath(server_id),
        body,
        false,
    )
    .await
}

pub(super) async fn replacement_apply(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<ContentInstallApplyDto>, JsonRejection>,
) -> Response {
    apply_mode(
        State(state),
        Extension(credential),
        Extension(auth),
        AxumPath(server_id),
        body,
        true,
    )
    .await
}

async fn apply_mode(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<ContentInstallApplyDto>, JsonRejection>,
    replacement: bool,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Provide a pack reviewId"),
    };
    if let Err(error) = content_install::selected(&state, &server_id) {
        return invalid_body("server_unavailable", error);
    }
    let operations = state.lifecycle.operations();
    let operation = match operations.begin_lifecycle(
        if replacement {
            "addon-pack-replacement"
        } else {
            "addon-pack-update"
        },
        Some(server_id.clone()),
        if replacement {
            "Applying reviewed base pack replacement"
        } else {
            "Applying reviewed pack update"
        },
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
        if replacement {
            "/v1/servers/:server_id/content/pack-replacement"
        } else {
            "/v1/servers/:server_id/content/pack-update"
        },
        response.status(),
    );
    tokio::task::spawn_blocking(move || {
        let identity = crate::auth::StreamAuthentication {
            credential: credential.clone(),
        };
        let cancel = operations.cancellation_check(&operation);
        let cancelled = || {
            cancel()
                || !auth.stream_retains_permissions(&identity)
                || content_install::selected(&state, &server_id).is_err()
        };
        let result = (|| {
            let server = content_install::selected(&state, &server_id).map_err(str::to_string)?;
            pack_activation::recover(&server)?;
            msc_application::content_install::recover_activation(&server)?;
            let review = service::load(&server, &body.review_id, &credential.credential_id)?;
            if review.replacement != replacement {
                return Err("Pack review belongs to a different action".into());
            }
            if !review.missing.is_empty() {
                return Err("Resolve missing content/classification and prepare a fresh pack review before activation".into());
            }
            if cancelled() {
                return Err("Pack update cancelled or permission revoked".into());
            }
            pack_activation::prepare(&server, &review.activation, &cancelled)?;
            service::load(&server, &body.review_id, &credential.credential_id)?;
            pack_activation::activate(&server, &review.activation, &cancelled, &mut |collection| {
                state
                    .lifecycle
                    .try_mutate_config(|config| {
                        if cancel()
                            || !auth.stream_retains_permissions(&identity)
                            || config.active_server_id.as_deref() != Some(server_id.as_str())
                        {
                            return Err("Pack update cancelled or server changed");
                        }
                        let current = config
                            .servers
                            .iter_mut()
                            .find(|current| current.id == server_id)
                            .ok_or("Server removed")?;
                        if current != &server {
                            return Err("Server changed during pack activation");
                        }
                        current.content_collection = Some(collection);
                        current.pack_managed = true;
                        current.pack_name = Some(review.retained_pack.name.clone());
                        current.pack_version = Some(review.retained_pack.version.clone());
                        current.modpack_identity =
                            Some(msc_domain::modpack::ModpackIdentity::source(
                                if review.retained_pack.format == "mrpack" {
                                    "modrinth"
                                } else {
                                    "curseforge"
                                },
                                Some(&review.retained_pack.name),
                                Some(&review.retained_pack.version),
                            ));
                        Ok(())
                    })
                    .map_err(|_| "Could not save updated pack metadata".to_string())
            })
        })();
        match result {
            Ok(()) => {
                let _ = operations.succeed(
                    &operation,
                    if replacement {
                        "Base pack replacement applied; worlds unchanged"
                    } else {
                        "Pack update applied"
                    },
                    BTreeMap::new(),
                );
            }
            Err(error) => {
                let _ = operations.fail(&operation, "pack_update_failed", error);
            }
        }
    });
    response
}

fn manual_binding(
    review_id: &str,
    expected: &pack_retention::ExpectedPackFile,
) -> Result<String, String> {
    Ok(format!(
        "pack-review:{review_id}:{}",
        sha512_hex(&serde_json::to_vec(expected).map_err(|error| error.to_string())?)
    ))
}
pub(super) async fn manual_admit(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath((server_id, review_id)): AxumPath<(String, String)>,
    body: Result<Json<msc_api::dto::RetainedPackManualDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Choose a missing reviewed pack file"),
    };
    let loaded = tokio::task::spawn_blocking(move || {
        let server = content_install::selected(&state, &server_id).map_err(str::to_string)?;
        let review = service::load(&server, &review_id, &credential.credential_id)?;
        let pack = service::retained_for_review(&server, &review)?;
        if !pack.unresolved.contains_key(&body.path) {
            return Err("Pack file is not awaiting recovery".into());
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
            return Err("File has no supported exact identity".into());
        }
        let binding = manual_binding(&review_id, &expected)?;
        Ok::<_, String>((state, credential, expected, binding))
    })
    .await;
    let (state, credential, expected, binding) = match loaded {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => return invalid_body("pack_manual_unavailable", &error),
        Err(_) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "pack_manual_unavailable",
                "Pack reader failed",
            );
        }
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
                .map(str::to_string),
            operation_id: Some(binding),
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
        "/v1/servers/:server_id/content/pack-reviews/:review_id/manual/admit",
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
pub(super) async fn manual_complete(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath((server_id, review_id)): AxumPath<(String, String)>,
    body: Result<Json<msc_api::dto::RetainedPackManualDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return invalid_body(
                "invalid_json",
                "Provide the reviewed file and staged upload",
            );
        }
    };
    let Some(id) = body.staged_upload_id else {
        return invalid_body("missing_upload", "stagedUploadId is required");
    };
    let Some(entry) = state.staging.uploads.lock().unwrap().get(&id).cloned() else {
        return invalid_body("upload_unavailable", "Manual upload is unavailable");
    };
    if !entry.complete
        || entry.expires_at_unix < now_unix()
        || entry.provider_owner.as_deref() != Some(credential.credential_id.as_str())
        || entry.file_id.as_deref() != Some(body.path.as_str())
        || !matches!(entry.purpose, StagedUploadPurposeDto::ModpackUnresolvedFile)
    {
        return invalid_body(
            "invalid_upload",
            "Manual upload does not match this owner and file",
        );
    }
    let server = match content_install::selected(&state, &server_id) {
        Ok(server) => server,
        Err(error) => return invalid_body("server_unavailable", error),
    };
    let operations = state.lifecycle.operations();
    let operation = match operations.begin_lifecycle(
        "addon-pack-manual",
        Some(server_id.clone()),
        "Recovering staged pack content",
    ) {
        Ok(operation) => operation,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    // Redeem while authenticated task-local upload ownership is still in scope.
    if state.staging.uploads.lock().unwrap().remove(&id).is_none() {
        let _ = operations.fail(
            &operation,
            "pack_manual_failed",
            "Manual upload was consumed".into(),
        );
        return invalid_body("upload_unavailable", "Manual upload was consumed");
    }
    let response = (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"operationId":operation.as_str()})),
    )
        .into_response();
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/servers/:server_id/content/pack-reviews/:review_id/manual/complete",
        response.status(),
    );
    tokio::task::spawn_blocking(move || {
        let identity = crate::auth::StreamAuthentication {
            credential: credential.clone(),
        };
        let cancel = operations.cancellation_check(&operation);
        let result = (|| {
            if cancel() || !auth.stream_retains_permissions(&identity) {
                return Err("Manual recovery cancelled or permission revoked".into());
            }
            let fresh = content_install::selected(&state, &server_id).map_err(str::to_string)?;
            if fresh != server {
                return Err("Server changed after manual recovery admission".into());
            }
            let review = service::load(&fresh, &review_id, &credential.credential_id)?;
            let mut pack = service::retained_for_review(&fresh, &review)?;
            let expected = pack
                .expected_files
                .get(&body.path)
                .ok_or("Reviewed file disappeared")?;
            if entry.operation_id.as_deref() != Some(manual_binding(&review_id, expected)?.as_str())
            {
                return Err("Manual upload belongs to another pack review".into());
            }
            pack_retention::accept_manual_file_cancellable(
                Path::new(&fresh.server_dir),
                &mut pack,
                &body.path,
                &entry.path,
                &|| {
                    cancel()
                        || !auth.stream_retains_permissions(&identity)
                        || content_install::selected(&state, &server_id).is_err()
                },
            )?;
            Ok::<_, String>(())
        })();
        let _ = std::fs::remove_file(entry.path);
        match result {
            Ok(()) => {
                let mut result = BTreeMap::new();
                result.insert("previousReviewId".into(), review_id);
                let _ = operations.succeed(
                    &operation,
                    "Staged pack file recovered; prepare a fresh review before activation",
                    result,
                );
            }
            Err(error) => {
                let _ = operations.fail(&operation, "pack_manual_failed", error);
            }
        }
    });
    response
}
