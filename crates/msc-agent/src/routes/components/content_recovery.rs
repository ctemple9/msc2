//! Long recovery prepares a review; apply uses the independent collection
//! transaction and never calls the running-server modpack importer.
use super::*;
use msc_api::dto::ContentRecoveryReviewDto;
use msc_application::content_recovery as service;
pub(super) async fn review(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<ContentRecoveryReviewDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return invalid_body(
                "invalid_json",
                "Provide a revision and original pack or selected server files",
            );
        }
    };
    let Some(server) = state
        .lifecycle
        .app_config_servers()
        .into_iter()
        .find(|server| server.id == server_id)
    else {
        return invalid_body("server_unavailable", "Server not found");
    };
    if body.staged_upload_id.is_some() && body.retained_pack_id.is_some() {
        return invalid_body(
            "ambiguous_original",
            "Choose an uploaded archive or a retained original",
        );
    }
    let mut lease = None;
    let upload = if let Some(id) = &body.staged_upload_id {
        if !state.staging.native.target_matches(id, &server_id) {
            return invalid_body(
                "target_mismatch",
                "Original pack was selected for another server",
            );
        }
        let Some(entry) = state.staging.uploads.lock().unwrap().get(id).cloned() else {
            return invalid_body("upload_unavailable", "Original pack upload is unavailable");
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
                "Original pack upload is expired, incomplete or belongs to another credential",
            );
        }
        lease = state.staging.native.inspection_lease(id, &credential);
        Some(entry)
    } else {
        None
    };
    let operations = state.lifecycle.operations();
    let operation = match operations.begin_lifecycle(
        "addon-player-recovery-review",
        Some(server_id.clone()),
        "Preparing player content recovery",
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
        "/v1/servers/:server_id/content/recovery-review",
        response.status(),
    );
    tokio::task::spawn_blocking(move || {
        let _lease = lease;
        let identity = crate::auth::StreamAuthentication {
            credential: credential.clone(),
        };
        let cancel = operations.cancellation_check(&operation);
        let cancelled = || cancel() || !auth.stream_retains_permissions(&identity);
        let mut staged_dir = None;
        let result = (|| {
            if cancelled() {
                return Err("Recovery cancelled or permission revoked".into());
            }
            let secrets = production_secret_store()
                .map_err(|_| "Provider secret storage unavailable".to_string())?;
            let transport = HttpTransport::new();
            let retained_pack = body
                .retained_pack_id
                .as_ref()
                .map(|id| msc_application::pack_retention::load(&server, id))
                .transpose()?;
            let inspection = if let Some(pack) = &retained_pack {
                let inspection = msc_application::pack_retention::inspect_original(
                    Path::new(&server.server_dir),
                    pack,
                    &transport,
                    secrets.as_ref(),
                    &cancelled,
                )?;
                staged_dir = Some(inspection.staged_dir.clone());
                Some(inspection)
            } else if let Some(entry) = upload.as_ref() {
                let mut inspection = modpacks::inspect_staged_archive(
                    &StdFileSystem,
                    &transport,
                    secrets.as_ref(),
                    &entry.path,
                    &staging_root(&state.lifecycle.servers_root()).join("recovery"),
                    operation.as_str(),
                )
                .map_err(|error| error.to_string())?;
                inspection.origin = entry.pack_origin.clone();
                staged_dir = Some(inspection.staged_dir.clone());
                Some(inspection)
            } else {
                None
            };
            let recovered = service::review(
                &server,
                &credential.credential_id,
                body.collection_revision,
                &body.selected_paths,
                &body.restore_paths,
                inspection.as_ref(),
                retained_pack,
                &transport,
                secrets.as_ref(),
                &cancelled,
            )?;
            let fresh = state
                .lifecycle
                .app_config_servers()
                .into_iter()
                .find(|current| current.id == server_id)
                .ok_or("Server removed")?;
            if fresh != server || cancelled() {
                return Err("Server or permission changed during recovery review".into());
            }
            // This also rechecks server and retained bytes after the long download.
            msc_application::content_management::load_review(
                &fresh,
                &recovered.review.id,
                &credential.credential_id,
            )?;
            Ok::<_, String>(recovered)
        })();
        if let Some(directory) = staged_dir {
            let _ = std::fs::remove_dir_all(directory);
        }
        match result {
            Ok(recovered) => {
                let mut result = BTreeMap::new();
                result.insert("reviewId".into(), recovered.review.id);
                if let Some(pack_id) = recovered.pack_id {
                    result.insert("packId".into(), pack_id);
                }
                let _ = operations.succeed(
                    &operation,
                    "Player recovery review ready; server files unchanged",
                    result,
                );
            }
            Err(error) => {
                let _ = operations.fail(&operation, "player_recovery_review_failed", error);
            }
        }
    });
    response
}
