//! Exports produce an authenticated staged file, never a public invitation URL.
use super::*;
use msc_api::dto::ContentExportRequestDto;
pub(super) async fn export(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<ContentExportRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return invalid_body(
                "invalid_json",
                "Provide publicationId, audience and incomplete consent when needed",
            );
        }
    };
    if !matches!(body.audience.as_str(), "players" | "server") {
        return invalid_body("invalid_audience", "Choose players or server");
    }
    let Some(server) = state
        .lifecycle
        .app_config_servers()
        .into_iter()
        .find(|server| server.id == server_id)
    else {
        return invalid_body("server_unavailable", "Server not found");
    };
    let operations = state.lifecycle.operations();
    let operation = match operations.begin_lifecycle(
        "content-export",
        Some(server_id.clone()),
        "Preparing frozen content ZIP",
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
        "/v1/servers/:server_id/content/exports",
        response.status(),
    );
    tokio::task::spawn_blocking(move || {
        let identity = crate::auth::StreamAuthentication {
            credential: credential.clone(),
        };
        let cancel = operations.cancellation_check(&operation);
        let root = state.lifecycle.servers_root();
        let cancelled = || {
            cancel()
                || !auth.stream_retains_permissions(&identity)
                || state.lifecycle.servers_root() != root
        };
        let id = Uuid::new_v4().to_string();
        let destination = staging_root(&root)
            .join("downloads")
            .join(format!("{id}.zip"));
        let mut last = 0u64;
        let result = msc_application::content_archives::build(
            &server,
            &body.publication_id,
            &body.audience,
            body.allow_incomplete,
            &destination,
            &cancelled,
            &mut |current, total| {
                if current == 0
                    || current == total
                    || current.saturating_sub(last) >= 8 * 1024 * 1024
                {
                    let _ = operations.progress(
                        &operation,
                        current,
                        total,
                        "Writing frozen content ZIP",
                    );
                    last = current;
                }
            },
        );
        let bound = state
            .lifecycle
            .app_config_servers()
            .into_iter()
            .find(|current| current.id == server_id)
            .is_some_and(|current| {
                let old = server.content_collection.as_ref();
                let fresh = current.content_collection.as_ref();
                current.server_dir == server.server_dir
                    && fresh.is_some_and(|collection| {
                        collection.publication_ids.contains(&body.publication_id)
                    })
                    && old
                        .and_then(|collection| collection.extra.get("publicationManifestHashes"))
                        .and_then(|hashes| hashes.get(&body.publication_id))
                        == fresh
                            .and_then(|collection| {
                                collection.extra.get("publicationManifestHashes")
                            })
                            .and_then(|hashes| hashes.get(&body.publication_id))
            });
        match result {
            Ok(archive) if !cancelled() && bound => {
                let expiry = now_unix() + STAGING_TTL_SECONDS;
                state.staging.downloads.lock().unwrap().insert(
                    id.clone(),
                    StagedDownload {
                        required_permission: Some(PermissionCategoryDto::Addons),
                        owner: credential.credential_id,
                        expires_at_unix: expiry,
                        path: destination,
                    },
                );
                let mut result = BTreeMap::new();
                result.insert("stagedDownloadId".into(), id.clone());
                result.insert("downloadPath".into(), format!("/v1/staged-downloads/{id}"));
                result.insert("fileName".into(), archive.file_name);
                result.insert("byteSize".into(), archive.byte_size.to_string());
                result.insert("publicationId".into(), body.publication_id);
                result.insert("audience".into(), body.audience);
                result.insert("complete".into(), archive.complete.to_string());
                result.insert("expiresAt".into(), unix_to_iso8601(expiry));
                let _ = operations.succeed(
                    &operation,
                    "Content ZIP ready for authenticated file transfer",
                    result,
                );
            }
            result => {
                let cleanup = std::fs::remove_file(destination);
                let mut message = match result {
                    Err(error) => error,
                    Ok(_) => {
                        "Export cancelled or source/permission changed before transfer admission"
                            .into()
                    }
                };
                if let Err(error) = cleanup
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    message.push_str(&format!(
                        "; staged archive cleanup could not be confirmed: {error}"
                    ));
                }
                if cancel() {
                    let _ = operations
                        .cancel(&operation, &format!("Content export cancelled: {message}"));
                } else {
                    let _ = operations.fail(&operation, "content_export_failed", message);
                }
            }
        }
    });
    response
}
