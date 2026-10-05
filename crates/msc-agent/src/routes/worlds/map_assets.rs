//! Worlds-authorized inspection, exact preparation and guarded saved-map adoption.
use super::*;
use axum::extract::Query;
use msc_api::dto::{MapAssetsCapabilitiesDto, MapAssetsCheckRequestDto, MapAssetsCheckStartedDto};
use msc_application::map_assets::{self as service, Context};
use msc_domain::map_assets::{self as domain, Status};
use msc_infrastructure::config_repository::default_app_data_dir;
use msc_infrastructure::map_assets::store::Store;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

#[derive(Clone)]
pub(super) struct AssetsState(Arc<AssetsInner>);
struct AssetsInner {
    workers: Arc<Semaphore>,
    store: Mutex<Option<Store>>,
}
impl Default for AssetsState {
    fn default() -> Self {
        Self(Arc::new(AssetsInner {
            workers: Arc::new(Semaphore::new(2)),
            store: Mutex::new(None),
        }))
    }
}
impl AssetsState {
    pub(super) fn worker(&self) -> Option<tokio::sync::OwnedSemaphorePermit> {
        self.0.workers.clone().try_acquire_owned().ok()
    }
    pub(super) fn store(&self) -> std::io::Result<Store> {
        let mut store = self
            .0
            .store
            .lock()
            .map_err(|_| std::io::Error::other("store_unavailable"))?;
        if let Some(store) = store.as_ref() {
            return Ok(store.clone());
        }
        let opened = Store::open(default_app_data_dir().join("map-assets"))?;
        *store = Some(opened.clone());
        Ok(opened)
    }
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AssetsQuery {
    server_id: String,
}
fn actions() -> Vec<String> {
    [
        "status",
        "report",
        "check",
        "capture_request",
        "prepare",
        "rendering",
        "import",
        "repair",
        "rebuild",
        "selection",
        "restore",
    ]
    .iter()
    .map(|s| format!("worlds.map_assets.{s}.v1"))
    .collect()
}
pub(super) async fn capabilities(
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    Json(MapAssetsCapabilitiesDto {
        schema_version: 1,
        actions: actions(),
        resource_formats: vec![
            "msc-resource-inventory-1".into(),
            "msc-client-resources-1".into(),
        ],
        capture_formats: vec![domain::CAPTURE_FORMAT.into()],
        renderer_adoption: true,
    })
    .into_response()
}
#[allow(clippy::result_large_err)]
fn bound(
    state: &WorldsRoutesState,
    credential: &AuthenticatedCredential,
    server_id: &str,
    slot: &str,
) -> Result<Context, Response> {
    if let Some(response) = require_permission(credential, PermissionCategoryDto::Worlds) {
        return Err(response);
    }
    let server = state
        .lifecycle
        .active_config_server()
        .ok_or_else(no_active_server)?;
    if server.id != server_id {
        return Err(error_response(
            StatusCode::CONFLICT,
            "server_binding_changed",
            "Select the expected active server before inspecting resources.",
        ));
    }
    if server.server_type != ServerType::Java {
        return Err(error_response(
            StatusCode::CONFLICT,
            "map_assets_not_applicable",
            "Java resource inventory does not alter the Bedrock rendering path.",
        ));
    }
    let host = state.lifecycle.map_assets_host_id().map_err(|_| {
        error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "host_identity_unavailable",
            "The agent host identity could not be read.",
        )
    })?;
    service::context(&server, &host, slot).map_err(|e| asset_error(&e))
}
fn asset_error(error: &std::io::Error) -> Response {
    let code = error.to_string();
    let safe = if code.bytes().all(|c| c.is_ascii_lowercase() || c == b'_') {
        code.as_str()
    } else {
        "map_assets_io_failed"
    };
    error_response(
        StatusCode::CONFLICT,
        safe,
        "Map-resource inspection could not complete safely. The prior generation is retained; inspect the issue code before retrying.",
    )
}
#[allow(clippy::result_large_err)]
pub(super) async fn status(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot): AxumPath<String>,
    Query(query): Query<AssetsQuery>,
) -> Response {
    let task = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let context = bound(&task, &credential, &query.server_id, &slot)?;
        let store = task.map_assets.store().map_err(|e| asset_error(&e))?;
        let pointer = store.pointer(&context.binding).map_err(|e| asset_error(&e))?;
        let retained=map_terrain::retained_report(&task,&context);
        let report_available = pointer.is_some() || retained.is_some();
        let renderer_adopted=retained.is_some() || pointer.as_ref().and_then(|p|store.lease(&p.current).ok()).and_then(|l|l.report).is_some_and(|r|r.geometry_generation_id.is_some());
        Ok::<_, Response>(Status {
            schema_version: 1,
            binding: context.binding,
            state: if report_available { "inspected" } else { "unchecked" }.into(),
            input_generation: pointer.as_ref().map(|p| p.current.clone()),
            previous_generation: pointer.and_then(|p| p.previous),
            report_available,
            renderer_adopted,
            actions: actions(),
            note: "Reports describe only their saved snapshot and checked area. Retained renderer reports keep their original binding and saved snapshot; rendering exposes their age and stale state. Use prepare for exact resources and guarded saved-map adoption; unresolved client selection remains an input requirement.".into(),
        })
    }).await;
    match result {
        Ok(Ok(status)) => Json(status).into_response(),
        Ok(Err(response)) => response,
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "map_assets_worker_failed",
            "The inventory worker could not finish.",
        ),
    }
}
#[allow(clippy::result_large_err)]
pub(super) async fn report(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot): AxumPath<String>,
    Query(query): Query<AssetsQuery>,
) -> Response {
    let task = state.clone();
    let result = tokio::task::spawn_blocking(move || {
        let context = bound(&task, &credential, &query.server_id, &slot)?;
        let store = task.map_assets.store().map_err(|e| asset_error(&e))?;
        let pointer = store
            .pointer(&context.binding)
            .map_err(|e| asset_error(&e))?;
        if pointer.is_none()
            && let Some(retained) = map_terrain::retained_report(&task, &context)
        {
            return Ok(retained);
        }
        let pointer = pointer.ok_or_else(|| {
            error_response(
                StatusCode::NOT_FOUND,
                "map_assets_report_unavailable",
                "No report exists for this world binding. Run a scoped check.",
            )
        })?;
        let lease = store.lease(&pointer.current).map_err(|e| asset_error(&e))?;
        let report = lease.report.ok_or_else(|| {
            error_response(
                StatusCode::NOT_FOUND,
                "map_assets_report_unavailable",
                "This generation has no area report.",
            )
        })?;
        if report.binding != context.binding {
            return Err(error_response(
                StatusCode::CONFLICT,
                "binding_changed",
                "The report belongs to a different world binding.",
            ));
        }
        Ok::<_, Response>(report)
    })
    .await;
    match result {
        Ok(Ok(report)) => Json(report).into_response(),
        Ok(Err(response)) => response,
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "map_assets_worker_failed",
            "The report worker could not finish.",
        ),
    }
}
#[allow(clippy::result_large_err)]
pub(super) async fn check(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot): AxumPath<String>,
    payload: Result<Json<MapAssetsCheckRequestDto>, JsonRejection>,
) -> Response {
    let request = match payload {
        Ok(Json(request)) => request,
        Err(_) => {
            return invalid_body(
                "invalid_map_assets_request",
                "Supply serverId, expectedRevision, dimension and integer min/max bounds.",
            );
        }
    };
    let task_state = state.clone();
    let server_id = request.server_id.clone();
    let context = match tokio::task::spawn_blocking(move || {
        bound(&task_state, &credential, &server_id, &slot)
    })
    .await
    {
        Ok(Ok(context)) => context,
        Ok(Err(response)) => return response,
        Err(_) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "map_assets_worker_failed",
                "The binding could not be inspected.",
            );
        }
    };
    if let Err(code) = request.area.validate() {
        return invalid_body(
            code,
            "Use at most 16 chunks and 262,144 blocks in one scoped inspection.",
        );
    }
    if !domain::valid_resource_id(&request.dimension) || !request.dimension.contains(':') {
        return invalid_body(
            "invalid_dimension",
            "Use the original namespaced dimension identifier.",
        );
    }
    if context.binding.revision != request.expected_revision {
        return (StatusCode::CONFLICT,Json(serde_json::json!({"code":"binding_changed","message":"Fetch status and retry with the current binding revision.","binding":context.binding}))).into_response();
    }
    if matches!(
        context.world,
        msc_infrastructure::map_assets::saved_terrain::WorldSource::Directory(_)
    ) && state.lifecycle.status_snapshot().running
        && !map_terrain::has_prepared_scene(&state, &context)
        && map_terrain::inspection_snapshot(&state, &context).is_none()
    {
        return error_response(
            StatusCode::CONFLICT,
            "consistent_snapshot_required",
            "Stop the server before checking its live saved world. Archived slots can be inspected while it runs; ordinary terrain rendering remains available.",
        );
    }
    let permit = match state.map_assets.0.workers.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return error_response(
                StatusCode::CONFLICT,
                "map_assets_worker_limit",
                "Two map-resource scans are already active on this host. Retry after an operation finishes.",
            );
        }
    };
    let operation = match state.lifecycle.operations().begin_lifecycle(
        "world-map-assets-check",
        Some(context.server.id.clone()),
        "Inspecting saved map resources.",
    ) {
        Ok(id) => id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let binding = context.binding.clone();
    let response_id = operation.as_str().to_string();
    let operations = state.lifecycle.operations();
    let cancel = operations.cancellation_check(&operation);
    let lifecycle = state.lifecycle.clone();
    tokio::spawn(async move {
        let work_id = operation.clone();
        let work_operations = operations.clone();
        let check_cancel = cancel.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let store = state.map_assets.store()?;
            let version = crate::routes::versions::minecraft_version_from_selection(
                Some(context.server.java_flavor),
                context.server.minecraft_version.clone(),
            )
            .unwrap_or_default();
            if !version
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
            {
                return Err(std::io::Error::other("invalid_minecraft_version"));
            }
            let vanilla = default_app_data_dir()
                .join("map-dependencies/java-assets")
                .join(&version)
                .join("assets/minecraft");
            let saved_report = map_terrain::check_prepared(
                &state,
                &context,
                &request.dimension,
                request.area,
                work_id.as_str(),
                &check_cancel,
            );
            let (candidate, manifest, report) = if let Some(result) = saved_report {
                result?
            } else {
                let saved = map_terrain::inspection_snapshot(&state, &context);
                let inspected_context = saved.as_ref().map(|s| &s.0).unwrap_or(&context);
                service::inspect(
                    inspected_context,
                    &store,
                    &vanilla,
                    Some(&version),
                    &request.dimension,
                    request.area,
                    work_id.as_str(),
                    &check_cancel,
                    &|current, total, line| {
                        let _ = work_operations.progress(&work_id, current, total, line);
                    },
                )?
            };
            // Selection and slot/source revision must still match when publishing the candidate.
            lifecycle
                .with_expected_active_server(Some(&context.server.id), || {
                    let server = lifecycle
                        .active_config_server()
                        .ok_or_else(|| std::io::Error::other("binding_changed"))?;
                    let current = service::context(
                        &server,
                        &context.binding.agent_host_id,
                        &context.binding.slot_id,
                    )?;
                    if current.binding != context.binding {
                        return Err(std::io::Error::other("binding_changed"));
                    }
                    if let Some(generation) = &report.geometry_generation_id
                        && map_terrain::rendering_status(&state, &context, &report.dimension)
                            .generation_id
                            .as_ref()
                            != Some(generation)
                    {
                        return Err(std::io::Error::other("binding_changed"));
                    }
                    store.publish(&candidate, &manifest, &report, &check_cancel)?;
                    Ok(report)
                })
                .map_err(|_| std::io::Error::other("binding_changed"))?
        })
        .await;
        match result {
            Ok(Ok(report)) => {
                let _ = operations.progress(
                    &operation,
                    4,
                    4,
                    "Scoped resource inspection complete; rendering was not changed.",
                );
                let _ = operations.succeed(
                    &operation,
                    "Scoped resource inspection complete.",
                    BTreeMap::from([
                        ("result".into(), report.outcome),
                        ("resourceGenerationId".into(), report.resource_generation_id),
                        ("snapshotId".into(), report.snapshot_id),
                    ]),
                );
            }
            Ok(Err(error)) if error.to_string() == "cancelled" => {
                let _ = operations.cancel(
                    &operation,
                    "Resource inspection cancelled; previous generation retained.",
                );
            }
            Ok(Err(error)) => {
                let code = error.to_string();
                let code = if code.bytes().all(|c| c.is_ascii_lowercase() || c == b'_') {
                    code.as_str()
                } else {
                    "map_assets_io_failed"
                };
                let _ = operations.fail(
                    &operation,
                    code,
                    "Resource inspection failed safely; previous generation retained.".into(),
                );
            }
            Err(_) => {
                let _ = operations.fail(
                    &operation,
                    "map_assets_worker_failed",
                    "Resource inspection worker failed; previous generation retained.".into(),
                );
            }
        }
    });
    Json(MapAssetsCheckStartedDto {
        result: "check_started".into(),
        operation_id: response_id,
        binding,
    })
    .into_response()
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RenderingQuery {
    server_id: String,
    dimension: String,
}
#[allow(clippy::result_large_err)]
pub(super) async fn rendering(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot): AxumPath<String>,
    Query(query): Query<RenderingQuery>,
) -> Response {
    match tokio::task::spawn_blocking(move || {
        let context = bound(&state, &credential, &query.server_id, &slot)?;
        if !domain::valid_resource_id(&query.dimension) || !query.dimension.contains(':') {
            return Err(invalid_body(
                "invalid_dimension",
                "Supply the original namespaced dimension.",
            ));
        }
        Ok::<_, Response>(map_terrain::rendering_status(
            &state,
            &context,
            &query.dimension,
        ))
    })
    .await
    {
        Ok(Ok(status)) => Json(status).into_response(),
        Ok(Err(response)) => response,
        _ => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "map_assets_worker_failed",
            "Map rendering status is unavailable.",
        ),
    }
}
#[allow(clippy::result_large_err)]
pub(super) async fn prepare(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot): AxumPath<String>,
    payload: Result<Json<msc_api::dto::MapAssetsPrepareRequestDto>, JsonRejection>,
) -> Response {
    let request = match payload {
        Ok(Json(r)) => r,
        Err(_) => {
            return invalid_body(
                "invalid_map_assets_request",
                "Supply serverId, expectedRevision, dimension and optional scoped area.",
            );
        }
    };
    let server = request.server_id.clone();
    let task = state.clone();
    let context = match tokio::task::spawn_blocking(move || {
        bound(&task, &credential, &server, &slot)
    })
    .await
    {
        Ok(Ok(c)) => c,
        Ok(Err(r)) => return r,
        _ => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "map_assets_worker_failed",
                "Map binding is unavailable.",
            );
        }
    };
    if request.area.as_ref().is_some_and(|a| a.validate().is_err()) {
        return invalid_body("invalid_area", "Use at most 16 chunks and 262,144 blocks.");
    }
    if !domain::valid_resource_id(&request.dimension) || !request.dimension.contains(':') {
        return invalid_body(
            "invalid_dimension",
            "Supply the original namespaced dimension.",
        );
    }
    if context.binding.revision != request.expected_revision {
        return error_response(
            StatusCode::CONFLICT,
            "binding_changed",
            "Fetch status and retry using its current revision.",
        );
    }
    if !matches!(
        context.world,
        msc_infrastructure::map_assets::saved_terrain::WorldSource::Directory(_)
    ) {
        return error_response(
            StatusCode::CONFLICT,
            "active_world_required",
            "Select this slot as the active saved world before preparing its renderer.",
        );
    }
    let binding = context.binding.clone();
    match map_terrain::prepare_resources(state, context, request.dimension, request.area, true) {
        Ok(operation_id) => Json(MapAssetsCheckStartedDto {
            result: "prepare_started".into(),
            operation_id,
            binding,
        })
        .into_response(),
        Err((code, message)) => error_response(StatusCode::CONFLICT, code, &message),
    }
}

#[allow(clippy::result_large_err)]
pub(super) async fn capture_request(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot): AxumPath<String>,
    payload: Result<Json<MapAssetsCheckRequestDto>, JsonRejection>,
) -> Response {
    let request = match payload {
        Ok(Json(request)) => request,
        _ => {
            return invalid_body(
                "invalid_capture_request",
                "Supply the current binding, dimension and bounded capture area.",
            );
        }
    };
    match tokio::task::spawn_blocking(move || {
        let _permit = state.map_assets.worker().ok_or_else(|| {
            error_response(
                StatusCode::CONFLICT,
                "map_assets_worker_limit",
                "Wait for the current map operation.",
            )
        })?;
        let context = bound(&state, &credential, &request.server_id, &slot)?;
        if request.expected_revision != context.binding.revision {
            return Err(error_response(
                StatusCode::CONFLICT,
                "binding_changed",
                "Fetch the current world binding.",
            ));
        }
        map_terrain::capture_request(&state, &context, &request.dimension, request.area)
            .map_err(|e| asset_error(&e))
    })
    .await
    {
        Ok(Ok(request)) => Json(request).into_response(),
        Ok(Err(response)) => response,
        _ => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "map_assets_worker_failed",
            "Capture request could not be prepared.",
        ),
    }
}

#[allow(clippy::result_large_err)]
pub(super) async fn import(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot): AxumPath<String>,
    payload: Result<Json<msc_api::dto::MapAssetsImportRequestDto>, JsonRejection>,
) -> Response {
    let request = match payload {
        Ok(Json(r)) => r,
        _ => {
            return invalid_body(
                "invalid_map_assets_request",
                "Supply the bound completed client bundle and checksum.",
            );
        }
    };
    let context = match bound(&state, &credential, &request.server_id, &slot) {
        Ok(c) => c,
        Err(r) => return r,
    };
    if request.expected_revision != context.binding.revision
        || !domain::valid_resource_id(&request.dimension)
        || !request.dimension.contains(':')
        || request.area.as_ref().is_some_and(|a| a.validate().is_err())
    {
        return invalid_body(
            "invalid_map_import_context",
            "Fetch the current binding and supply a valid dimension and bounded area.",
        );
    }
    if !matches!(
        context.world,
        msc_infrastructure::map_assets::saved_terrain::WorldSource::Directory(_)
    ) {
        return error_response(
            StatusCode::CONFLICT,
            "active_world_required",
            "Activate this saved slot before importing map resources.",
        );
    }
    let mut uploads = state.staging.uploads.lock().unwrap();
    let Some(entry) = uploads.get(&request.staged_upload_id) else {
        return invalid_body(
            "upload_unavailable",
            "Begin a matching client resource upload.",
        );
    };
    if entry.purpose != StagedUploadPurposeDto::MapClientAssets
        || !entry.complete
        || now_unix() > entry.expires_at_unix
        || entry.map_binding.as_ref()
            != Some(&(context.binding.clone(), credential.credential_id.clone()))
    {
        return error_response(
            StatusCode::CONFLICT,
            "map_import_binding_mismatch",
            "The completed upload must belong to this credential and exact world binding.",
        );
    }
    let entry = uploads.remove(&request.staged_upload_id).unwrap();
    drop(uploads);
    let binding = context.binding.clone();
    let path = entry.path.clone();
    match map_terrain::import_resources(
        state,
        context,
        request.dimension,
        request.area,
        entry.path,
        request.sha256,
    ) {
        Ok(operation_id) => Json(MapAssetsCheckStartedDto {
            result: "import_started".into(),
            operation_id,
            binding,
        })
        .into_response(),
        Err((code, message)) => {
            let _ = std::fs::remove_file(path);
            error_response(StatusCode::CONFLICT, code, &message)
        }
    }
}

pub(super) async fn client_context(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot): AxumPath<String>,
    Query(query): Query<AssetsQuery>,
) -> Response {
    match bound(&state, &credential, &query.server_id, &slot) {
        Ok(context) => Json(msc_api::dto::MapAssetsClientContextDto {
            minecraft_version: context.server.minecraft_version,
            loader: if matches!(
                context.server.java_flavor.raw_value(),
                "paper" | "purpur" | "spigot" | "pufferfish"
            ) {
                "vanilla".into()
            } else {
                context.server.java_flavor.raw_value().into()
            },
            loader_version: context.server.loader_version,
        })
        .into_response(),
        Err(response) => response,
    }
}

#[allow(clippy::result_large_err)]
pub(super) async fn selection(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot): AxumPath<String>,
    Query(query): Query<AssetsQuery>,
) -> Response {
    let result = tokio::task::spawn_blocking(move || {
        let context = bound(&state, &credential, &query.server_id, &slot)?;
        let store = state.map_assets.store().map_err(|e| asset_error(&e))?;
        let imported = service::imported(&context, &store).map_err(|e| asset_error(&e))?;
        let (selection_revision, manifest) = match imported {
            Some((_, receipt)) => (Some(service::selection_revision(&receipt).map_err(|e| asset_error(&e))?),Some(serde_json::to_value(receipt.manifest).map_err(|_| invalid_body("invalid_resource_selection", "Selection metadata unavailable."))?)),
            None => (None, None),
        };
        let previous_generation = service::previous_resources(&context, &store).map_err(|e| asset_error(&e))?.as_ref().map(service::selection_revision).transpose().map_err(|e| asset_error(&e))?;
        Ok::<_,Response>(msc_api::dto::MapAssetsSelectionDto { binding:context.binding, selection_revision, manifest, previous_generation, note:"Only map resources change. Pack order is low to high priority; visual correctness requires inspection in Minecraft.".into() })
    }).await;
    match result {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(response)) => response,
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "map_assets_worker_failed",
            "Selection worker unavailable.",
        ),
    }
}

fn start_mutation(
    state: WorldsRoutesState,
    context: Context,
    expected: &str,
    dimension: String,
    area: Option<domain::Area>,
    mutation: service::ResourceMutation,
) -> Response {
    if context.binding.revision != expected
        || !domain::valid_resource_id(&dimension)
        || !dimension.contains(':')
        || area.as_ref().is_some_and(|a| a.validate().is_err())
    {
        return error_response(
            StatusCode::CONFLICT,
            "binding_changed",
            "Fetch the current binding and supply a valid dimension/area.",
        );
    }
    let binding = context.binding.clone();
    match map_terrain::mutate_resources(state, context, dimension, area, mutation) {
        Ok(operation_id) => Json(MapAssetsCheckStartedDto {
            result: "repair_started".into(),
            operation_id,
            binding,
        })
        .into_response(),
        Err((code, message)) => error_response(StatusCode::CONFLICT, code, &message),
    }
}
#[allow(clippy::result_large_err)]
pub(super) async fn select(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot): AxumPath<String>,
    payload: Result<Json<msc_api::dto::MapAssetsSelectionRequestDto>, JsonRejection>,
) -> Response {
    let request = match payload {
        Ok(Json(request)) => request,
        _ => {
            return invalid_body(
                "invalid_resource_selection",
                "Supply known ordered pack IDs and the expected selection revision.",
            );
        }
    };
    let task = state.clone();
    let server_id = request.server_id.clone();
    let expected_selection = request.expected_selection_revision.clone();
    let prepared = tokio::task::spawn_blocking(move || {
        let context = bound(&task, &credential, &server_id, &slot)?;
        let store = task.map_assets.store().map_err(|e| asset_error(&e))?;
        let mutation = service::select_resources(
            &context,
            &store,
            &expected_selection,
            request.selected_packs,
            request.mod_order,
        )
        .map_err(|e| asset_error(&e))?;
        Ok::<_, Response>((context, mutation))
    })
    .await;
    match prepared {
        Ok(Ok((context, mutation))) => start_mutation(
            state,
            context,
            &request.expected_revision,
            request.dimension,
            request.area,
            mutation,
        ),
        Ok(Err(response)) => response,
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "map_assets_worker_failed",
            "Selection worker unavailable.",
        ),
    }
}
#[allow(clippy::result_large_err)]
pub(super) async fn restore(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot): AxumPath<String>,
    payload: Result<Json<msc_api::dto::MapAssetsRestoreRequestDto>, JsonRejection>,
) -> Response {
    let request = match payload {
        Ok(Json(request)) => request,
        _ => {
            return invalid_body(
                "invalid_resource_restore",
                "Name the compatible previous resource generation.",
            );
        }
    };
    let task = state.clone();
    let server_id = request.server_id.clone();
    let generation = request.generation.clone();
    let prepared = tokio::task::spawn_blocking(move || {
        let context = bound(&task, &credential, &server_id, &slot)?;
        let store = task.map_assets.store().map_err(|e| asset_error(&e))?;
        let mutation = service::restore_resources(&context, &store, &generation)
            .map_err(|e| asset_error(&e))?;
        Ok::<_, Response>((context, mutation))
    })
    .await;
    match prepared {
        Ok(Ok((context, mutation))) => start_mutation(
            state,
            context,
            &request.expected_revision,
            request.dimension,
            request.area,
            mutation,
        ),
        Ok(Err(response)) => response,
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "map_assets_worker_failed",
            "Restore worker unavailable.",
        ),
    }
}
