//! Explicit, cancellable Java map warming over Vantage's ordinary tile route.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::{Extension, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::PermissionCategoryDto;
use msc_domain::identity::ServerType;
use msc_domain::operation::OperationId;
use msc_infrastructure::fs::StdFileSystem;
use serde::{Deserialize, Serialize};

use super::{RendererStore, java_terrain_compat};
use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::require_permission;
use crate::routes::worlds::{
    WorldsRoutesState, active_server_or_response, audit, error_response, is_safe_level_folder,
    java_map_dimensions,
};

const MIB: u64 = 1024 * 1024;

#[derive(Clone)]
pub(super) struct Job {
    server_id: String,
    dimension: String,
    id: OperationId,
}

#[derive(Deserialize)]
pub(crate) struct DimensionQuery {
    dimension: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartRequest {
    dimension: String,
    cache_budget_mib: u64,
}

#[derive(Deserialize)]
pub(crate) struct CancelRequest {
    dimension: String,
}

#[derive(Deserialize)]
struct Tile {
    x: i32,
    z: i32,
    bytes: u64,
    #[serde(default)]
    baked: bool,
}

#[derive(Deserialize)]
struct Manifest {
    tiles: Vec<Tile>,
    progress: Option<ManifestProgress>,
}

#[derive(Deserialize)]
struct ManifestProgress {
    done: u64,
    total: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Estimate {
    total_tiles: u64,
    cached_tiles: u64,
    cached_bytes: u64,
    estimated_total_bytes: u64,
    suggested_budget_mib: u64,
    max_budget_mib: u64,
    operation_id: Option<String>,
    operation: Option<msc_api::dto::OperationDto>,
}

struct RunRequest {
    server_id: String,
    server_dir: PathBuf,
    world: PathBuf,
    dimension: String,
    version: Option<String>,
    budget_mib: u64,
}

fn selected_world(
    state: &WorldsRoutesState,
    dimension: &str,
) -> Result<(String, PathBuf, PathBuf, Option<String>), Box<Response>> {
    if state.map_shutdown.load(Ordering::Acquire) {
        return Err(Box::new(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_stopping",
            "The agent is stopping.",
        )));
    }
    let server = active_server_or_response(&state.lifecycle).map_err(Box::new)?;
    if server.server_type == ServerType::Bedrock {
        return Err(Box::new(error_response(
            StatusCode::CONFLICT,
            "java_only",
            "Pre-render map currently supports Java worlds. Bedrock parity follows the Java map work.",
        )));
    }
    let server_dir = PathBuf::from(&server.server_dir);
    let name = msc_application::worlds::read_java_level_name(&StdFileSystem, &server_dir)
        .filter(|name| is_safe_level_folder(name))
        .ok_or_else(|| {
            Box::new(error_response(
                StatusCode::CONFLICT,
                "map_unavailable",
                "The active Java world folder is unavailable.",
            ))
        })?;
    let world = server_dir.join(name);
    if !super::is_direct_child(&server_dir, &world) {
        return Err(Box::new(error_response(
            StatusCode::CONFLICT,
            "map_unavailable",
            "The active Java world folder is unavailable.",
        )));
    }
    let dimensions = java_map_dimensions(&world, true).map_err(|_| {
        Box::new(error_response(
            StatusCode::CONFLICT,
            "map_unavailable",
            "Saved dimensions could not be inspected.",
        ))
    })?;
    if !dimensions
        .iter()
        .any(|item| item.id == dimension && item.state == "ready")
    {
        return Err(Box::new(error_response(
            StatusCode::CONFLICT,
            "dimension_empty",
            "The selected dimension has no saved terrain.",
        )));
    }
    let selected_version = crate::routes::versions::minecraft_version_from_selection(
        Some(server.java_flavor),
        server.minecraft_version.clone(),
    );
    Ok((server.id, server_dir, world, selected_version))
}

fn manifest(
    store: &RendererStore,
    state: &WorldsRoutesState,
    server_id: &str,
    world: &Path,
    dimension: &str,
    version: Option<&str>,
    fresh: bool,
) -> Result<Manifest, String> {
    let (status, bytes) = store
        .fetch(
            server_id,
            world,
            dimension,
            if fresh {
                "manifest-fresh.json"
            } else {
                "manifest.json"
            },
            version,
            &|| state.lifecycle.status_snapshot().running,
        )
        .map_err(|error| error.message)?;
    if status != 200 {
        return Err(format!("renderer returned HTTP {status}"));
    }
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn fetch_tile(state: &WorldsRoutesState, task: &RunRequest, tile: &Tile) -> Result<(), String> {
    let path = format!("tiles/t.{}.{}.vtile", tile.x, tile.z);
    let started = Instant::now();
    let response = state.map_renderer.fetch(
        &task.server_id,
        &task.world,
        &task.dimension,
        &path,
        task.version.as_deref(),
        &|| state.lifecycle.status_snapshot().running,
    );
    let elapsed = started.elapsed();
    if elapsed >= Duration::from_secs(5) {
        eprintln!(
            "Java map: pre-render tile ({},{}) request took {:.1}s",
            tile.x,
            tile.z,
            elapsed.as_secs_f64()
        );
    }
    let (status, _) = response.map_err(|error| {
        format!(
            "Tile ({},{}) failed after {:.1}s: {}",
            tile.x,
            tile.z,
            elapsed.as_secs_f64(),
            error.message
        )
    })?;
    if status != 200 {
        return Err(format!(
            "Tile ({},{}) returned HTTP {status} after {:.1}s",
            tile.x,
            tile.z,
            elapsed.as_secs_f64()
        ));
    }
    Ok(())
}

fn estimate(
    state: &WorldsRoutesState,
    server_id: &str,
    world: &Path,
    dimension: &str,
    version: Option<&str>,
) -> Result<Estimate, String> {
    let current = manifest(
        &state.map_renderer,
        state,
        server_id,
        world,
        dimension,
        version,
        false,
    )?;
    let cached_bytes: u64 = current.tiles.iter().map(|tile| tile.bytes).sum();
    let cached_tiles = current.progress.as_ref().map_or(0, |p| p.done);
    let total_tiles = current
        .progress
        .as_ref()
        .map_or(current.tiles.len() as u64, |p| p.total);
    // A first tile is a rough sample, not a promise about varied terrain.
    let sample = if cached_tiles > 0 {
        cached_bytes / cached_tiles.max(1)
    } else {
        2 * MIB
    };
    let estimated_total_bytes = sample.max(256 * 1024).saturating_mul(total_tiles);
    let suggested_budget_mib = (estimated_total_bytes.saturating_mul(5) / 4)
        .div_ceil(MIB)
        .clamp(4096, 65536);
    let (cache, _) = java_terrain_compat::persistent_tiles(
        server_id,
        world,
        dimension,
        &super::dependencies::assets(world, version).map_err(|error| error.message)?,
    )
    .map_err(|error| error.to_string())?;
    let used = crate::map_staging::estimate_tree(&cache).map_err(|error| error.to_string())?;
    let free = fs2::available_space(&cache).map_err(|error| error.to_string())?;
    // Snapshot copying can consume this much space on filesystems without
    // copy-on-write clones; do not advertise that space as tile capacity.
    let snapshot_reserve =
        crate::map_staging::estimate_tree(world).map_err(|error| error.to_string())?;
    let max_budget_mib = ((used.saturating_add(
        free.saturating_sub(msc_infrastructure::conversion_resources::DISK_RESERVE)
            .saturating_sub(snapshot_reserve),
    )) / MIB)
        .min(65536);
    let job = state
        .map_renderer
        .0
        .pre_render
        .lock()
        .map_err(|error| error.to_string())?
        .clone()
        .filter(|job| job.server_id == server_id && job.dimension == dimension);
    let operation = job
        .as_ref()
        .and_then(|job| state.lifecycle.operations().snapshot(job.id.as_str()));
    Ok(Estimate {
        total_tiles,
        cached_tiles,
        cached_bytes,
        estimated_total_bytes,
        suggested_budget_mib,
        max_budget_mib,
        operation_id: job.as_ref().map(|job| job.id.as_str().to_string()),
        operation,
    })
}

pub(crate) async fn status(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Query(query): Query<DimensionQuery>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let (server_id, _, world, version) = match selected_world(&state, &query.dimension) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let dimension = query.dimension;
    let result = tokio::task::spawn_blocking(move || {
        estimate(&state, &server_id, &world, &dimension, version.as_deref())
    })
    .await;
    match result {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(message)) => {
            error_response(StatusCode::BAD_GATEWAY, "pre_render_unavailable", &message)
        }
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "pre_render_unavailable",
            &error.to_string(),
        ),
    }
}

pub(crate) async fn start(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Json(request): Json<StartRequest>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let (server_id, server_dir, world, version) = match selected_world(&state, &request.dimension) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if !(4096..=65536).contains(&request.cache_budget_mib) {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_budget",
            "Choose a cache budget between 4096 and 65536 MiB.",
        );
    }
    let estimate_state = state.clone();
    let estimate_server_id = server_id.clone();
    let estimate_world = world.clone();
    let estimate_dimension = request.dimension.clone();
    let estimate_version = version.clone();
    let current = match tokio::task::spawn_blocking(move || {
        estimate(
            &estimate_state,
            &estimate_server_id,
            &estimate_world,
            &estimate_dimension,
            estimate_version.as_deref(),
        )
    })
    .await
    {
        Ok(Ok(value)) => value,
        Ok(Err(message)) => {
            return error_response(StatusCode::BAD_GATEWAY, "pre_render_unavailable", &message);
        }
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "pre_render_unavailable",
                &error.to_string(),
            );
        }
    };
    if request.cache_budget_mib > current.max_budget_mib
        || request.cache_budget_mib < current.suggested_budget_mib
    {
        return error_response(
            StatusCode::CONFLICT,
            "insufficient_cache",
            "The chosen budget is below the estimated map size or the host's available storage. Free space or choose a suitable budget.",
        );
    }
    let mut active = state.map_renderer.0.pre_render.lock().unwrap();
    if active.as_ref().is_some_and(|job| {
        state
            .lifecycle
            .operations()
            .snapshot(job.id.as_str())
            .is_some_and(|operation| {
                matches!(
                    operation.state,
                    msc_api::dto::OperationStateDto::Running
                        | msc_api::dto::OperationStateDto::Queued
                )
            })
    }) {
        return error_response(
            StatusCode::CONFLICT,
            "pre_render_running",
            "A Java map pre-render is already running on this agent.",
        );
    }
    // A long background bake must not hold the server's mutation lock: the
    // server may start while it runs. The snapshot step checks that boundary.
    let id = match state.lifecycle.operations().begin_lifecycle(
        "world-map-pre-render",
        None,
        "Capturing saved Java terrain for pre-render.",
    ) {
        Ok(id) => id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    *active = Some(Job {
        server_id: server_id.clone(),
        dimension: request.dimension.clone(),
        id: id.clone(),
    });
    drop(active);
    let response_id = id.as_str().to_string();
    let task_state = state.clone();
    let task = RunRequest {
        server_id,
        server_dir,
        world,
        dimension: request.dimension,
        version,
        budget_mib: request.cache_budget_mib,
    };
    tokio::task::spawn_blocking(move || run(task_state, id, task));
    let response = (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "operationId": response_id })),
    )
        .into_response();
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/worlds/map/pre-render",
        response.status(),
    );
    response
}

fn run(state: WorldsRoutesState, id: OperationId, task: RunRequest) {
    let RunRequest {
        server_id,
        server_dir,
        world,
        dimension,
        version,
        budget_mib,
    } = task;
    let operations = state.lifecycle.operations();
    let cancel = operations.cancellation_check(&id);
    let result = (|| -> Result<(u64, u64), String> {
        let snapshot = if state.lifecycle.status_snapshot().running {
            crate::backup_operations::snapshot_java_world(
                state.lifecycle.clone(),
                &server_dir,
                &cancel,
            )?
        } else {
            crate::backup_operations::snapshot_stopped_java_world(&server_dir, || {
                cancel() || state.lifecycle.status_snapshot().running
            })?
        };
        if cancel() {
            return Err("cancelled".into());
        }
        let assets = super::dependencies::assets(&snapshot.path, version.as_deref())
            .map_err(|error| error.message)?;
        let (cache, _) =
            java_terrain_compat::persistent_tiles(&server_id, &world, &dimension, &assets)
                .map_err(|error| error.to_string())?;
        java_terrain_compat::set_tile_cache_budget_mib(&cache, budget_mib)
            .map_err(|error| error.to_string())?;
        state
            .map_renderer
            .use_snapshot(server_id.clone(), world.clone(), snapshot)
            .map_err(|_| "The terrain snapshot could not be selected".to_string())?;
        let initial = manifest(
            &state.map_renderer,
            &state,
            &server_id,
            &world,
            &dimension,
            version.as_deref(),
            true,
        )?;
        let total = initial
            .progress
            .as_ref()
            .map_or(initial.tiles.len() as u64, |p| p.total);
        let mut completed = initial.progress.as_ref().map_or(0, |p| p.done);
        operations
            .progress(&id, completed, total, "Pre-rendering saved Java terrain.")
            .map_err(|error| error.to_string())?;
        let tiles: Vec<_> = initial
            .tiles
            .into_iter()
            .filter(|tile| !tile.baked)
            .collect();
        let (_, bake_capacity) =
            super::java_bake_capacity(state.lifecycle.status_snapshot().running);
        let workers = bake_capacity
            .saturating_sub(1)
            .max(1)
            .min(tiles.len().max(1));
        eprintln!("Java map: pre-render using {workers} background tile worker(s)");
        let next = AtomicUsize::new(0);
        let stop = Arc::new(AtomicBool::new(false));
        let (send, receive) = mpsc::channel::<Result<(), String>>();
        let fetch_task = RunRequest {
            server_id: server_id.clone(),
            server_dir: server_dir.clone(),
            world: world.clone(),
            dimension: dimension.clone(),
            version: version.clone(),
            budget_mib,
        };
        std::thread::scope(|scope| -> Result<(), String> {
            for worker in 0..workers {
                let send = send.clone();
                let state = state.clone();
                let cancel = cancel.clone();
                let stop = stop.clone();
                let tiles = &tiles;
                let next = &next;
                let task = &fetch_task;
                scope.spawn(move || {
                    loop {
                        if stop.load(Ordering::Acquire)
                            || cancel()
                            || next.load(Ordering::Acquire) >= tiles.len()
                        {
                            break;
                        }
                        // Already-started requests finish, but a running server
                        // admits only the first background lane.
                        if worker > 0 && state.lifecycle.status_snapshot().running {
                            std::thread::sleep(Duration::from_millis(100));
                            continue;
                        }
                        let index = next.fetch_add(1, Ordering::AcqRel);
                        let Some(tile) = tiles.get(index) else { break };
                        let result = fetch_tile(&state, task, tile);
                        if result.is_err() {
                            stop.store(true, Ordering::Release);
                        }
                        if send.send(result).is_err() {
                            break;
                        }
                        if state.lifecycle.status_snapshot().running {
                            std::thread::sleep(Duration::from_millis(500));
                        }
                    }
                });
            }
            drop(send);
            let mut received = 0;
            let mut last_report = Instant::now();
            while received < tiles.len() {
                if cancel() {
                    stop.store(true, Ordering::Release);
                    return Err("cancelled".into());
                }
                match receive.recv_timeout(Duration::from_millis(200)) {
                    Ok(Ok(())) => {
                        received += 1;
                        completed += 1;
                    }
                    Ok(Err(error)) => {
                        stop.store(true, Ordering::Release);
                        return Err(error);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        return Err(
                            "The pre-render workers stopped before all tiles completed.".into()
                        );
                    }
                }
                if last_report.elapsed() >= Duration::from_secs(2) {
                    let current = manifest(
                        &state.map_renderer,
                        &state,
                        &server_id,
                        &world,
                        &dimension,
                        version.as_deref(),
                        true,
                    )
                    .inspect_err(|_| {
                        stop.store(true, Ordering::Release);
                    })?;
                    let retained = current.progress.as_ref().map_or(completed, |p| p.done);
                    if retained + 2 < completed {
                        stop.store(true, Ordering::Release);
                        return Err("The detailed-tile cache filled and started evicting pre-rendered tiles. Increase its budget or free storage, then resume.".into());
                    }
                    operations
                        .progress(&id, retained, total, "Pre-rendering saved Java terrain.")
                        .map_err(|error| {
                            stop.store(true, Ordering::Release);
                            error.to_string()
                        })?;
                    last_report = Instant::now();
                }
            }
            Ok(())
        })?;
        let final_manifest = manifest(
            &state.map_renderer,
            &state,
            &server_id,
            &world,
            &dimension,
            version.as_deref(),
            true,
        )?;
        let done = final_manifest
            .progress
            .as_ref()
            .map_or(completed, |p| p.done);
        if done < total {
            return Err(format!(
                "Only {done} of {total} tiles remain cached. Increase the cache budget, then resume."
            ));
        }
        Ok((done, total))
    })();
    match result {
        Ok((done, total)) => {
            let mut details = BTreeMap::new();
            details.insert("cachedTiles".into(), done.to_string());
            details.insert("totalTiles".into(), total.to_string());
            details.insert("cacheBudgetMiB".into(), budget_mib.to_string());
            let _ = operations.succeed(&id, "Saved Java terrain pre-rendered.", details);
        }
        Err(_) if cancel() => {
            let _ = operations.cancel(&id, "Pre-render cancelled; completed tiles remain cached.");
        }
        Err(message) => {
            let _ = operations.fail(&id, "pre_render_failed", message);
        }
    }
}

pub(crate) async fn cancel(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Json(request): Json<CancelRequest>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let (server_id, _, _, _) = match selected_world(&state, &request.dimension) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let job = state.map_renderer.0.pre_render.lock().unwrap().clone();
    let Some(job) =
        job.filter(|job| job.server_id == server_id && job.dimension == request.dimension)
    else {
        return error_response(
            StatusCode::NOT_FOUND,
            "pre_render_missing",
            "No pre-render job is recorded for this world and dimension.",
        );
    };
    match state.lifecycle.operations().request_cancel_authorized(
        &job.id,
        "Stopping Java map pre-render.",
        &credential,
    ) {
        Ok(operation) => {
            let response = (StatusCode::ACCEPTED, Json(operation)).into_response();
            audit(
                &state.lifecycle,
                &credential,
                "POST",
                "/v1/worlds/map/pre-render/cancel",
                response.status(),
            );
            response
        }
        Err(error) => crate::routes::operations::operation_error_response(error),
    }
}
