//! Exact-selection reviews. Persist before each removal; restart never resumes it.
use super::*;
use axum::extract::{Path, Query};
use msc_api::dto::{
    StorageCandidateDto as Item, StorageCandidatesDto, StorageCleanupRequestDto, StorageOutcomeDto,
    StorageReviewDto, StorageReviewRequestDto, StorageReviewedResultDto,
};
use msc_infrastructure::storage;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone)]
pub(super) enum Owner {
    Data(super::server_data::Data),
    Map(CleanupEntry),
    Transfer(super::super::components::storage_cleanup::Entry),
    Build(super::builds::Build),
}
#[derive(Clone)]
pub(super) struct Candidate {
    pub item: Item,
    pub owner: Owner,
}
pub(super) struct Review {
    credential: String,
    expires: Instant,
    host: String,
    category: String,
    action: String,
    confirmation: String,
    items: Vec<Candidate>,
}
#[derive(Default)]
pub(super) struct Reviews {
    reviews: HashMap<String, Review>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Selection {
    category: String,
    action: String,
}
// Wire fields stay together at the owner-to-display boundary.
#[allow(clippy::too_many_arguments)]
fn candidate(
    host: &str,
    kind: &str,
    id: &str,
    label: String,
    path: Option<String>,
    bytes: Option<u64>,
    removable: bool,
    reason: String,
    owner: Owner,
) -> Candidate {
    Candidate {
        item: Item {
            consequences: vec![],
            id: storage::identity(&[host, kind, id]),
            label,
            display_path: path,
            logical_bytes: bytes,
            removable,
            reason,
        },
        owner,
    }
}
pub(super) async fn discover(
    state: &StorageState,
    host: &str,
    category: &str,
    action: &str,
) -> Result<Vec<Candidate>, String> {
    match (category, action) {
        ("servers", "delete-backups" | "remove-world" | "remove-server") => {
            let lifecycle = state.lifecycle.clone();
            let host = host.to_string();
            let action = action.to_string();
            tokio::task::spawn_blocking(move || {
                super::server_data::candidates(&lifecycle, &host, &action)
            })
            .await
            .map_err(|e| e.to_string())?
        }
        ("cache", "map-cache") => {
            let entries = tokio::task::spawn_blocking(map_staging::preview_cleanup)
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            Ok(entries
                .into_iter()
                .map(|e| {
                    candidate(
                        host,
                        "map",
                        &e.id,
                        e.id.clone(),
                        Some(e.path.clone()),
                        e.removable.then_some(e.size_bytes),
                        e.removable,
                        e.reason.clone(),
                        Owner::Map(e.clone()),
                    )
                })
                .collect())
        }
        ("cache", "old-downloads") => Ok(state
            .components
            .storage_candidates()
            .await
            .into_iter()
            .map(|e| {
                candidate(
                    host,
                    "transfer",
                    &e.id,
                    e.label.clone(),
                    None,
                    e.bytes,
                    e.removable,
                    e.reason.clone(),
                    Owner::Transfer(e.clone()),
                )
            })
            .collect()),
        ("agent", "obsolete-builds") => {
            let lifecycle = state.lifecycle.clone();
            let host_owned = host.to_string();
            let builds = tokio::task::spawn_blocking(move || {
                super::builds::candidates(&lifecycle, &host_owned)
            })
            .await
            .map_err(|e| e.to_string())??;
            Ok(builds
                .into_iter()
                .map(|b| {
                    candidate(
                        host,
                        &b.root_id,
                        &b.name,
                        format!("Agent build {}", &b.name[..12]),
                        Some(b.root.join(&b.name).display().to_string()),
                        b.bytes,
                        b.removable,
                        b.reason.clone(),
                        Owner::Build(b.clone()),
                    )
                })
                .collect())
        }
        _ => Err("This storage category/action is not supported".into()),
    }
}
#[allow(clippy::result_large_err)]
fn host(state: &StorageState, credential: &AuthenticatedCredential) -> Result<String, Response> {
    if let Some(response) = require_permission(credential, PermissionCategoryDto::Admin) {
        return Err(response);
    }
    state
        .lifecycle
        .storage_host_id()
        .map_err(|e| error_response(StatusCode::SERVICE_UNAVAILABLE, "storage_identity", &e))
}
pub(super) async fn candidates(
    State(state): State<StorageState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Query(selection): Query<Selection>,
) -> Response {
    let host = match host(&state, &credential) {
        Ok(h) => h,
        Err(r) => return r,
    };
    if let Some(r) = action_permission(&credential, &selection.action) {
        return r;
    }
    match discover(&state, &host, &selection.category, &selection.action).await {
        Ok(items) => Json(StorageCandidatesDto {
            host_id: host,
            category: selection.category,
            action: selection.action,
            items: items.into_iter().map(|c| c.item).collect(),
        })
        .into_response(),
        Err(e) => error_response(StatusCode::CONFLICT, "storage_candidates", &e),
    }
}
pub(super) async fn review(
    State(state): State<StorageState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Json(body): Json<StorageReviewRequestDto>,
) -> Response {
    let host = match host(&state, &credential) {
        Ok(h) => h,
        Err(r) => return r,
    };
    let ids = body.item_ids.iter().collect::<HashSet<_>>();
    if body.host_id != host
        || ids.is_empty()
        || ids.len() != body.item_ids.len()
        || ids.len() > 64
        || body.item_ids.iter().any(|s| s.len() != 64)
    {
        return error_response(
            StatusCode::BAD_REQUEST,
            "storage_selection",
            "Select up to 64 distinct items on this computer.",
        );
    }
    if let Some(r) = action_permission(&credential, &body.action) {
        return r;
    }
    if matches!(body.action.as_str(), "remove-world" | "remove-server") && ids.len() != 1 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "storage_separate_review",
            "World and server removal each require a separate single-item review.",
        );
    }
    let fresh = match discover(&state, &host, &body.category, &body.action).await {
        Ok(f) => f,
        Err(e) => return error_response(StatusCode::CONFLICT, "storage_review", &e),
    };
    let selected = fresh
        .into_iter()
        .filter(|c| ids.contains(&c.item.id))
        .collect::<Vec<_>>();
    if selected.len() != ids.len() || selected.iter().any(|c| !c.item.removable) {
        return error_response(
            StatusCode::CONFLICT,
            "storage_changed",
            "An item changed, disappeared or is protected. Refresh before reviewing again.",
        );
    }
    let token = uuid::Uuid::new_v4().to_string();
    let confirmation = match body.action.as_str() {
        "remove-server" => format!("DELETE SERVER {}", selected[0].item.label),
        "remove-world" => format!("DELETE WORLD {}", selected[0].item.label),
        "delete-backups" => "DELETE SELECTED BACKUPS".into(),
        _ => "REMOVE SELECTED FILES".into(),
    };
    let response = StorageReviewDto {
        host_id: host.clone(),
        category: body.category.clone(),
        action: body.action.clone(),
        preview_token: token.clone(),
        expires_in_seconds: 300,
        confirmation: confirmation.clone(),
        items: selected.iter().map(|c| c.item.clone()).collect(),
    };
    let mut reviews = state.reviews.lock().unwrap();
    reviews.reviews.retain(|_, p| p.expires > Instant::now());
    if reviews.reviews.len() >= 32 {
        return error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "storage_reviews",
            "Existing reviews must expire before creating another.",
        );
    }
    reviews.reviews.insert(
        token,
        Review {
            credential: credential.credential_id,
            expires: Instant::now() + Duration::from_secs(300),
            host,
            category: body.category,
            action: body.action,
            confirmation,
            items: selected,
        },
    );
    Json(response).into_response()
}
fn result_path(host: &str, id: &str) -> std::path::PathBuf {
    msc_infrastructure::config_repository::default_app_data_dir()
        .join("storage-results")
        .join(format!("{}.json", storage::identity(&[host, id])))
}
fn persist(result: &StorageReviewedResultDto) -> Result<(), String> {
    let path = result_path(&result.host_id, &result.operation_id);
    let parent = path.parent().unwrap();
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    storage::verified_root_id(&result.host_id, parent).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    // Metadata only. Keep at most 64 dated outcomes; never inspect a data tree.
    let mut files = std::fs::read_dir(parent)
        .map_err(|e| e.to_string())?
        .take(129)
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|e| e == "json"))
        .collect::<Vec<_>>();
    files.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
    let remove = files.len().saturating_sub(63);
    for file in files.into_iter().take(remove) {
        if file.path() != path {
            let _ = std::fs::remove_file(file.path());
        }
    }
    msc_infrastructure::atomic_write::atomic_write(
        &msc_infrastructure::fs::StdFileSystem,
        &path,
        &serde_json::to_vec(result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
pub(super) async fn apply(
    State(state): State<StorageState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Json(body): Json<StorageCleanupRequestDto>,
) -> Response {
    let host = match host(&state, &credential) {
        Ok(h) => h,
        Err(r) => return r,
    };
    if body.host_id != host || !state.lifecycle.storage_credential_valid(&credential) {
        return error_response(
            StatusCode::CONFLICT,
            "storage_host_changed",
            "Computer identity or credential changed.",
        );
    }
    let (review, operation) = {
        let mut reviews = state.reviews.lock().unwrap();
        let Some(review) = reviews.reviews.get(&body.preview_token) else {
            return error_response(
                StatusCode::CONFLICT,
                "storage_review_expired",
                "Create a fresh review.",
            );
        };
        if review.host != host
            || review.credential != credential.credential_id
            || review.expires <= Instant::now()
            || review.confirmation != body.confirmation
        {
            return error_response(
                StatusCode::CONFLICT,
                "storage_review_changed",
                "Review expired, credential changed or confirmation did not match.",
            );
        }
        if let Some(r) = action_permission(&credential, &review.action) {
            return r;
        }
        let operation = match state.lifecycle.operations().begin_lifecycle(
            "host-storage-cleanup",
            None,
            "Removing reviewed storage on this computer.",
        ) {
            Ok(o) => o,
            Err(e) => return super::super::operations::operation_error_response(e),
        };
        (
            reviews.reviews.remove(&body.preview_token).unwrap(),
            operation,
        )
    };
    let id = operation.as_str().to_string();
    let mut result = StorageReviewedResultDto {
        host_id: host.clone(),
        operation_id: id.clone(),
        state: "running".into(),
        items: review
            .items
            .iter()
            .map(|c| StorageOutcomeDto {
                id: c.item.id.clone(),
                label: c.item.label.clone(),
                state: "not-attempted".into(),
                removed_logical_bytes: None,
                message: "Not attempted".into(),
            })
            .collect(),
        available_bytes: None,
        observed_at: storage::now(),
    };
    if let Err(e) = persist(&result) {
        let _ = state.lifecycle.finish_operation_failure(
            &operation,
            "storage_result_unavailable",
            e.clone(),
        );
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "storage_result_unavailable",
            &e,
        );
    }
    let worker_host = host.clone();
    tokio::spawn(async move {
        let host = worker_host;
        let cancel = state.lifecycle.operations().cancellation_check(&operation);
        for (index, candidate) in review.items.into_iter().enumerate() {
            if cancel()
                || !state.lifecycle.storage_credential_valid(&credential)
                || action_permission(&credential, &review.action).is_some()
            {
                result.state = "cancelled".into();
                break;
            }
            result.items[index].state = "in-progress".into();
            result.items[index].message =
                "Removal started; outcome is unconfirmed until recorded".into();
            if persist(&result).is_err() {
                result.state = "interrupted".into();
                break;
            }
            let outcome = match candidate.owner {
                Owner::Data(data) => {
                    let host = host.clone();
                    let lifecycle = state.lifecycle.clone();
                    tokio::task::spawn_blocking(move || {
                        super::server_data::remove(&lifecycle, &host, &data)
                    })
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
                }
                Owner::Map(entry) => {
                    let check = cancel.clone();
                    tokio::task::spawn_blocking(move || {
                        map_staging::execute_cleanup(std::slice::from_ref(&entry), &check)
                            .map_err(|e| e.to_string())
                            .and_then(|r| {
                                if r.removed.len() == 1 {
                                    Ok(r.removed_bytes)
                                } else {
                                    Err(r.retained.join("; "))
                                }
                            })
                    })
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
                }
                Owner::Transfer(entry) => state.components.storage_remove(&entry).await,
                Owner::Build(build) => {
                    let host = host.clone();
                    tokio::task::spawn_blocking(move || super::builds::remove(&build, &host))
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                }
            };
            let item = &mut result.items[index];
            match outcome {
                Ok(bytes) => {
                    item.state = "removed".into();
                    item.removed_logical_bytes = Some(bytes);
                    item.message =
                        "Removed logical file bytes; released disk space may differ".into();
                }
                Err(e) => {
                    item.state = if e.contains("unconfirmed") {
                        "unconfirmed"
                    } else {
                        "retained"
                    }
                    .into();
                    item.message = e;
                }
            }
            if persist(&result).is_err() {
                result.state = "interrupted".into();
                break;
            }
        }
        if result.state == "running" {
            result.state = if result.items.iter().all(|i| i.state == "removed") {
                "completed"
            } else {
                "partial"
            }
            .into();
        }
        result.observed_at = storage::now();
        // Capacity is a separate observation, never the claimed amount reclaimed.
        result.available_bytes =
            fs2::available_space(msc_infrastructure::config_repository::default_app_data_dir())
                .ok();
        if persist(&result).is_err() {
            let _ = state.lifecycle.finish_operation_failure(
                &operation,
                "storage_result_unavailable",
                "Result persistence failed; removal outcomes may be unconfirmed".into(),
            );
        } else if result.state == "cancelled" {
            let _ = state.lifecycle.operations().cancel(
                &operation,
                "Cleanup cancelled before later items; earlier removals remain.",
            );
        } else if result.state == "completed" {
            let _ = state.lifecycle.finish_operation_success(
                &operation,
                "Reviewed storage removed.",
                BTreeMap::from([
                    ("category".into(), review.category),
                    ("action".into(), review.action),
                ]),
            );
        } else {
            let _ = state.lifecycle.finish_operation_failure(
                &operation,
                "storage_partial",
                "Some items were retained or have unconfirmed outcomes; inspect the result.".into(),
            );
        }
    });
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"hostId":host,"operationId":id})),
    )
        .into_response()
}
pub(super) async fn result(
    State(state): State<StorageState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Path(id): Path<String>,
) -> Response {
    let host = match host(&state, &credential) {
        Ok(h) => h,
        Err(r) => return r,
    };
    if uuid::Uuid::parse_str(&id).is_err() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "storage_operation",
            "Invalid operation identity.",
        );
    }
    let mut result = match storage::read_small(&result_path(&host, &id), 256 * 1024)
        .ok()
        .and_then(|b| serde_json::from_slice::<StorageReviewedResultDto>(&b).ok())
        .filter(|r| r.host_id == host && r.operation_id == id)
    {
        Some(r) => r,
        None => {
            return error_response(
                StatusCode::NOT_FOUND,
                "storage_result",
                "No retained result exists for this computer and operation.",
            );
        }
    };
    let ongoing = state.lifecycle.operations().snapshot(&id);
    if result.state == "running"
        && ongoing.as_ref().is_none_or(|o| {
            matches!(
                o.state,
                msc_api::dto::OperationStateDto::Succeeded
                    | msc_api::dto::OperationStateDto::Failed
                    | msc_api::dto::OperationStateDto::Cancelled
            )
        })
    {
        result.state = "interrupted".into();
        for i in &mut result.items {
            if i.state == "in-progress" {
                i.state = "unconfirmed".into();
                i.message="Operation interrupted; refresh inventory. Removal will not resume automatically.".into();
            }
        }
    }
    Json(result).into_response()
}

fn action_permission(credential: &AuthenticatedCredential, action: &str) -> Option<Response> {
    match action {
        "remove-server" => require_permission(credential, PermissionCategoryDto::Fleet),
        "remove-world" | "delete-backups" => {
            require_permission(credential, PermissionCategoryDto::Worlds)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn server_data_actions_keep_existing_permission_rules() {
        let mut credential = AuthenticatedCredential {
            credential_id: "controlled".into(),
            label: "Controlled".into(),
            role: crate::auth::CredentialRole::Named,
            permissions: vec![],
        };
        assert!(action_permission(&credential, "delete-backups").is_some());
        assert!(action_permission(&credential, "remove-world").is_some());
        credential.permissions = vec![PermissionCategoryDto::Worlds];
        assert!(action_permission(&credential, "delete-backups").is_none());
        assert!(action_permission(&credential, "remove-world").is_none());
        assert!(action_permission(&credential, "remove-server").is_some());
        credential.permissions = vec![PermissionCategoryDto::Admin];
        assert!(action_permission(&credential, "remove-server").is_none());
    }
}
