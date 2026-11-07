//! Agent-owned review/publication behavior is shared by desktop and terminal
//! clients. Publication reads never revisit today's content or a provider API.
use super::*;
use msc_api::dto::{PublicationReviewRequestDto, PublishContentDto};
use msc_application::{content_publications as snapshots, publication_review as service};
fn server(
    state: &ComponentsRoutesState,
    id: &str,
) -> Result<msc_domain::app_config_schema::ConfigServer, String> {
    let server = state
        .lifecycle
        .app_config_servers()
        .into_iter()
        .find(|server| server.id == id)
        .ok_or("Server not found")?;
    Ok(server)
}
fn public_review(review: service::Review) -> serde_json::Value {
    let content_bytes: u64 = review.files.iter().map(|file| file.file.byte_size).sum();
    let exportable = snapshots::exportable(review.request.pack_export, &review.issues);
    let files: Vec<_> = review
        .files
        .into_iter()
        .map(|file| serde_json::json!({"audience":file.audience,"file":file.file}))
        .collect();
    serde_json::json!({"reviewId":review.id,"serverId":review.server_id,"collectionRevision":review.request.collection_revision,"versionName":review.request.version_name,"audiences":review.request.audiences,"files":files,"candidates":review.candidates,"contentBytes":content_bytes,"complete":review.issues.is_empty(),"exportable":exportable,"issues":review.issues,"exclusions":review.exclusions,"playerSetup":review.player_setup,"guideVersion":msc_application::content_guides::VERSION,"guideStatus":"textFallback"})
}
fn summary(publication: &snapshots::Publication) -> serde_json::Value {
    serde_json::json!({"publicationId":publication.id,"available":true,"publicationRevision":publication.publication_revision,"collectionRevision":publication.collection_revision,"createdAt":publication.created_at,"versionName":publication.version_name,"audiences":publication.audiences,"complete":publication.issues.is_empty(),"exportable":snapshots::exportable(publication.content_fingerprint.starts_with("pack-v1:"), &publication.issues),"issues":publication.issues,"playerFileCount":publication.player_files.len(),"serverFileCount":publication.server_files.len(),"contentBytes":publication.player_files.values().chain(publication.server_files.values()).map(|file|file.byte_size).sum::<u64>(),"guideVersion":publication.guide_version})
}
pub(super) async fn review(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<PublicationReviewRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return invalid_body(
                "invalid_json",
                "Provide a collection revision and publication selections",
            );
        }
    };
    let request: service::Request =
        match serde_json::to_value(body).and_then(serde_json::from_value) {
            Ok(body) => body,
            Err(_) => {
                return invalid_body(
                    "invalid_publication_selection",
                    "Publication selections do not match the content contract",
                );
            }
        };
    let current = match server(&state, &server_id) {
        Ok(server) => server,
        Err(error) => return invalid_body("server_unavailable", &error),
    };
    if current.server_type != ServerType::Java {
        return invalid_body(
            "unsupported_server_type",
            "Content publication creation requires a Java server",
        );
    }
    let operations = state.lifecycle.operations();
    let operation = match operations.begin_lifecycle(
        "content-publication-review",
        Some(server_id.clone()),
        "Reviewing content publication",
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
        "/v1/servers/:server_id/content/publication-reviews",
        response.status(),
    );
    tokio::task::spawn_blocking(move || {
        let identity = crate::auth::StreamAuthentication {
            credential: credential.clone(),
        };
        let cancel = operations.cancellation_check(&operation);
        let cancelled = || cancel() || !auth.stream_retains_permissions(&identity);
        let result = (|| {
            if cancelled() {
                return Err("Publication review cancelled or permission revoked".into());
            }
            let review = service::prepare_with_progress(
                &current,
                &credential.credential_id,
                request,
                &cancelled,
                &|done, total, label| {
                    let _ = operations.progress(&operation, done, total, label);
                },
            )?;
            let fresh = server(&state, &server_id)?;
            if fresh != current || cancelled() {
                return Err("Server or permission changed during review".into());
            }
            if cancelled() {
                return Err("Publication review cancelled or permission revoked".into());
            }
            Ok::<_, String>(review.id)
        })();
        match result {
            Ok(id) => {
                let mut result = BTreeMap::new();
                result.insert("reviewId".into(), id);
                let _ = operations.succeed(
                    &operation,
                    "Publication review ready; inspect eligibility and missing content",
                    result,
                );
            }
            Err(error) => {
                if cancel() {
                    let _ = operations
                        .cancel(&operation, &format!("Content operation cancelled: {error}"));
                } else {
                    let _ = operations.fail(&operation, "publication_review_failed", error);
                }
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
        service::preview(
            &server(&state, &server_id)?,
            &review_id,
            &credential.credential_id,
        )
    })
    .await;
    match result {
        Ok(Ok(review)) => Json(public_review(review)).into_response(),
        Ok(Err(error)) => error_response(
            StatusCode::CONFLICT,
            "publication_review_unavailable",
            &error,
        ),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "publication_review_unavailable",
            "Publication review reader failed",
        ),
    }
}
pub(super) async fn publish(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(server_id): AxumPath<String>,
    body: Result<Json<PublishContentDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return invalid_body(
                "invalid_json",
                "Provide reviewId and explicit incomplete consent when needed",
            );
        }
    };
    let current = match server(&state, &server_id) {
        Ok(server) => server,
        Err(error) => return invalid_body("server_unavailable", &error),
    };
    if current.server_type != ServerType::Java {
        return invalid_body(
            "unsupported_server_type",
            "Content publication creation requires a Java server",
        );
    }
    let operations = state.lifecycle.operations();
    let operation = match operations.begin_lifecycle(
        "content-publish",
        Some(server_id.clone()),
        "Freezing reviewed content publication",
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
        "/v1/servers/:server_id/content/publications",
        response.status(),
    );
    tokio::task::spawn_blocking(move || {
        let identity = crate::auth::StreamAuthentication {
            credential: credential.clone(),
        };
        let cancel = operations.cancellation_check(&operation);
        let cancelled = || cancel() || !auth.stream_retains_permissions(&identity);
        let result = (|| {
            if cancelled() {
                return Err("Publication cancelled or permission revoked".into());
            }
            let review = service::load(&current, &body.review_id, &credential.credential_id)?;
            snapshots::freeze(
                &current,
                review.request.collection_revision,
                review.request.version_name.clone(),
                review.request.audiences.clone(),
                review.player_setup.clone(),
                msc_application::content_guides::VERSION,
                &review.files,
                review.issues.clone(),
                review.exclusions.clone(),
                review.content_fingerprint.clone(),
                review.request.server_paths.clone(),
                review.request.datapacks.clone(),
                body.incomplete_consent,
                &cancelled,
                &mut |collection| {
                    // Re-read every reviewed source after copying, outside the config lock.
                    let fresh = server(&state, &server_id)?;
                    if fresh != current || cancelled() {
                        return Err("Server or permission changed before publication commit".into());
                    }
                    service::load(&fresh, &body.review_id, &credential.credential_id)?;
                    state
                        .lifecycle
                        .try_mutate_config(|config| {
                            if cancel() || !auth.stream_retains_permissions(&identity) {
                                return Err("Publication cancelled or permission revoked");
                            }
                            let stored = config
                                .servers
                                .iter_mut()
                                .find(|server| server.id == server_id)
                                .ok_or("Server removed")?;
                            if stored != &current {
                                return Err("Server changed before publication registry commit");
                            }
                            stored.content_collection = Some(collection);
                            Ok(())
                        })
                        .map_err(|_| "Could not save publication registry".to_string())
                },
            )
        })();
        match result {
            Ok(publication) => {
                let mut result = BTreeMap::new();
                result.insert("publicationId".into(), publication.id);
                let _ = operations.succeed(&operation, "Dated content publication frozen", result);
            }
            Err(error) => {
                if cancel() {
                    let _ = operations
                        .cancel(&operation, &format!("Content operation cancelled: {error}"));
                } else {
                    let _ = operations.fail(&operation, "publication_failed", error);
                }
            }
        }
    });
    response
}
#[derive(Debug, serde::Deserialize)]
pub(super) struct HistoryQuery {
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "history_limit")]
    pub limit: usize,
}
fn history_limit() -> usize {
    50
}
pub(super) async fn history(
    State(state): State<ComponentsRoutesState>,
    Extension(_credential): Extension<AuthenticatedCredential>,
    AxumPath(server_id): AxumPath<String>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    let result=tokio::task::spawn_blocking(move || {
        let server=server(&state,&server_id)?;
        let ids=server.content_collection.as_ref().map(|collection|collection.publication_ids.as_slice()).unwrap_or(&[]);
        let mut items=Vec::new();let limit=query.limit.clamp(1,100);
        for id in ids.iter().rev().skip(query.offset).take(limit) {
            items.push(match snapshots::load(&server,id) {Ok(publication)=>summary(&publication),Err(error)=>serde_json::json!({"publicationId":id,"available":false,"error":error})});
        }
        let comparison=ids.last().map(|id| {
            let publication=snapshots::load(&server,id)?;
            (if publication.content_fingerprint.starts_with("pack-v1:") { service::pack_fingerprint(&server,&publication.reviewed_server_paths,&publication.reviewed_datapacks) } else { service::fingerprint(&server,&publication.reviewed_server_paths,&publication.reviewed_datapacks) }).map(|current|current!=publication.content_fingerprint)
        }).transpose();
        let (unpublished,error)=match comparison {Ok(Some(changed))=>(changed,None),Ok(None)=>(true,None),Err(error)=>(true,Some(error))};
        Ok::<_,String>(serde_json::json!({"items":items,"offset":query.offset,"limit":limit,"total":ids.len(),"unpublishedChanges":unpublished,"comparisonError":error}))
    }).await;
    match result {
        Ok(Ok(history)) => Json(history).into_response(),
        Ok(Err(error)) => error_response(
            StatusCode::CONFLICT,
            "publication_history_unavailable",
            &error,
        ),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "publication_history_unavailable",
            "Publication history reader failed",
        ),
    }
}
pub(super) async fn get(
    State(state): State<ComponentsRoutesState>,
    Extension(_credential): Extension<AuthenticatedCredential>,
    AxumPath((server_id, publication_id)): AxumPath<(String, String)>,
) -> Response {
    let result = tokio::task::spawn_blocking(move || {
        snapshots::load(&server(&state, &server_id)?, &publication_id)
    })
    .await;
    match result {
        Ok(Ok(publication)) => {
            let exportable = snapshots::exportable(
                publication.content_fingerprint.starts_with("pack-v1:"),
                &publication.issues,
            );
            let mut value = serde_json::to_value(publication).expect("publication serialization");
            value["exportable"] = serde_json::json!(exportable);
            Json(value).into_response()
        }
        Ok(Err(error)) => error_response(StatusCode::CONFLICT, "publication_unavailable", &error),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "publication_unavailable",
            "Publication reader failed",
        ),
    }
}
pub(super) async fn datapacks(
    State(state): State<ComponentsRoutesState>,
    Extension(_credential): Extension<AuthenticatedCredential>,
    AxumPath((server_id, slot_id)): AxumPath<(String, String)>,
) -> Response {
    let result=tokio::task::spawn_blocking(move || {
        let files=msc_application::publication_datapacks::inspect(&server(&state,&server_id)?,&slot_id,None,false,&||false)?;
        let items:Vec<_>=files.into_iter().map(|file|serde_json::json!({"slotId":file.slot_id,"path":file.path,"sha512":file.sha512,"sha1":file.sha1,"byteSize":file.byte_size})).collect();
        Ok::<_,String>(serde_json::json!({"items":items}))
    }).await;
    match result {
        Ok(Ok(files)) => Json(files).into_response(),
        Ok(Err(error)) => error_response(
            StatusCode::CONFLICT,
            "publication_datapacks_unavailable",
            &error,
        ),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "publication_datapacks_unavailable",
            "Datapack reader failed",
        ),
    }
}
