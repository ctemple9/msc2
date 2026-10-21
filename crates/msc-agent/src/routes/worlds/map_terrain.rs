//! Authenticated, bounded access to one private Vantage Java renderer.
pub(super) mod bedrock;
mod dependencies;
mod java_terrain_compat;
pub(super) mod pre_render;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{Extension, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use fs2::FileExt;
use serde::Deserialize;
use uuid::Uuid;

use super::{
    WorldsRoutesState, active_server_or_response, error_response, is_safe_level_folder,
    java_map_dimensions,
};
use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::require_permission;
use msc_api::dto::PermissionCategoryDto;
use msc_domain::identity::ServerType;
use msc_infrastructure::fs::StdFileSystem;

const IDLE: Duration = Duration::from_secs(90);
const MAX_ARTIFACT: usize = 32 * 1024 * 1024;
// Re-bake Bedrock tiles when converted biome names change their geometry and tint data.
const BEDROCK_TILE_CACHE_KEY: &str = "bedrock-anvil-v4-models-v1-biomes-v2";

fn java_bake_capacity(running: bool) -> (u64, usize) {
    let cores = std::thread::available_parallelism().map_or(1, usize::from);
    let memory_mib = if running {
        512
    } else {
        msc_infrastructure::conversion_resources::available_heap_mib()
            .unwrap_or(512)
            .clamp(512, 1024)
    };
    // Each tile can need roughly 200 MiB of working memory. Keep another
    // core free for the desktop or server, and never admit more than four.
    let workers = cores
        .saturating_sub(1)
        .clamp(1, 4)
        .min((memory_mib / 256) as usize);
    (memory_mib, workers)
}

#[derive(Debug)]
struct TerrainError {
    code: &'static str,
    message: String,
}

impl TerrainError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<()> for TerrainError {
    fn from(_: ()) -> Self {
        Self::new(
            "renderer_unavailable",
            "The Java terrain renderer could not supply this artifact. See the agent logs on the server host for details.",
        )
    }
}

#[derive(Clone, Default)]
pub(super) struct RendererStore(Arc<RendererState>);

#[derive(Default)]
struct RendererState {
    shutdown: AtomicBool,
    current: Mutex<Option<Renderer>>,
    snapshot: Mutex<Option<SavedSnapshot>>,
    sweeping: AtomicBool,
    progress: Mutex<Option<PreparationProgress>>,
    pre_render: Mutex<Option<pre_render::Job>>,
    visible_tile_active: AtomicUsize,
    last_visible_tile: Mutex<Option<Instant>>,
}

struct VisibleTileRequest(Arc<RendererState>);

impl Drop for VisibleTileRequest {
    fn drop(&mut self) {
        if let Ok(mut last) = self.0.last_visible_tile.lock() {
            *last = Some(Instant::now());
        }
        self.0.visible_tile_active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Clone, serde::Serialize)]
struct PreparationProgress {
    #[serde(skip)]
    server_id: String,
    dimension: String,
    stage: String,
    completed: Option<usize>,
    total: Option<usize>,
}

struct PreparationReporter<'a>(&'a Mutex<Option<PreparationProgress>>);
impl PreparationReporter<'_> {
    fn update(&self, stage: &str, completed: Option<usize>, total: Option<usize>) {
        if let Ok(mut current) = self.0.lock()
            && let Some(progress) = current.as_mut()
        {
            progress.stage = stage.into();
            progress.completed = completed;
            progress.total = total;
        }
    }
}
impl Drop for PreparationReporter<'_> {
    fn drop(&mut self) {
        if let Ok(mut current) = self.0.lock() {
            *current = None;
        }
    }
}

pub(super) async fn preparation_progress(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let server = match active_server_or_response(&state.lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let progress = state
        .map_renderer
        .0
        .progress
        .lock()
        .ok()
        .and_then(|current| current.clone())
        .filter(|progress| progress.server_id == server.id);
    axum::Json(progress).into_response()
}

struct SavedSnapshot {
    _staging: Option<Arc<crate::map_staging::Staging>>,
    _cache_lock: Option<Arc<std::fs::File>>,
    server_id: String,
    source_world: PathBuf,
    path: PathBuf,
    edition_cache_key: &'static str,
}

struct RendererIdentity<'a> {
    selected_version: Option<&'a str>,
    edition_cache_key: &'a str,
}

struct Renderer {
    _staging: crate::map_staging::Staging,
    _cache_lock: std::fs::File,
    server_id: String,
    world: PathBuf,
    dimension: String,
    port: u16,
    token: String,
    child: Child,
    diagnostics: Arc<Mutex<Vec<u8>>>,
    diagnostic_reader: Option<std::thread::JoinHandle<()>>,
    started_at: Instant,
    first_tile_reported: bool,
    last_use: Instant,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.diagnostic_reader.take() {
            let _ = reader.join();
        }
    }
}

#[derive(Deserialize)]
pub(super) struct ArtifactQuery {
    dimension: String,
    path: String,
    center_x: Option<i32>,
    center_z: Option<i32>,
    moving: Option<bool>,
}

pub(super) async fn artifact(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Query(query): Query<ArtifactQuery>,
) -> Response {
    if state.map_shutdown.load(Ordering::Acquire) {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_stopping",
            "The agent is stopping.",
        );
    }
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Some(content_type) = artifact_type(&query.path) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_artifact",
            "The requested terrain artifact is not supported.",
        );
    };
    let server = match active_server_or_response(&state.lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    if server.server_type == ServerType::Bedrock {
        return bedrock::artifact(&state, &server, &query, content_type).await;
    }
    let server_dir = PathBuf::from(&server.server_dir);
    let Some(level_name) =
        msc_application::worlds::read_java_level_name(&StdFileSystem, &server_dir)
            .filter(|name| is_safe_level_folder(name))
    else {
        return error_response(
            StatusCode::CONFLICT,
            "map_unavailable",
            "The active server has no safe configured world folder.",
        );
    };
    let world = server_dir.join(level_name);
    if !is_direct_child(&server_dir, &world) {
        return error_response(
            StatusCode::CONFLICT,
            "map_unavailable",
            "The active world folder is unavailable.",
        );
    }
    let catalog = match java_map_dimensions(&world, true) {
        Ok(catalog) => catalog,
        Err(_) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "map_unavailable",
                "The world dimensions could not be inspected safely.",
            );
        }
    };
    let Some(dimension) = catalog.iter().find(|item| item.id == query.dimension) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "dimension_unknown",
            "The requested dimension is not in this world.",
        );
    };
    if dimension.state != "ready" {
        return error_response(
            StatusCode::CONFLICT,
            "dimension_empty",
            "This dimension has no saved terrain yet.",
        );
    }
    let store = state.map_renderer.clone();
    let selected_version = crate::routes::versions::minecraft_version_from_selection(
        Some(server.java_flavor),
        server.minecraft_version.clone(),
    );
    let server_id = server.id;
    let dimension_id = query.dimension;
    let artifact = query.path;
    let lifecycle = state.lifecycle.clone();
    match tokio::task::spawn_blocking(move || {
        store.fetch_visible(
            &server_id,
            &world,
            &dimension_id,
            &artifact,
            selected_version.as_deref(),
            &|| lifecycle.status_snapshot().running,
        )
    })
    .await
    {
        Ok(Ok((200, bytes))) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type),
                (header::CACHE_CONTROL, "private, no-store"),
            ],
            Body::from(bytes),
        )
            .into_response(),
        Ok(Ok((404, _))) => error_response(
            StatusCode::NOT_FOUND,
            "artifact_pending",
            "The terrain artifact is not available yet.",
        ),
        Ok(Ok(_)) => error_response(
            StatusCode::BAD_GATEWAY,
            "renderer_unavailable",
            "The terrain renderer could not supply this artifact.",
        ),
        Ok(Err(error)) => error_response(StatusCode::BAD_GATEWAY, error.code, &error.message),
        Err(error) => {
            eprintln!("Java terrain renderer task failed: {error}");
            error_response(
                StatusCode::BAD_GATEWAY,
                "renderer_unavailable",
                "The Java terrain renderer failed. See the agent logs on the server host for details.",
            )
        }
    }
}

pub(super) fn is_direct_child(parent: &Path, child: &Path) -> bool {
    let Ok(parent_meta) = std::fs::symlink_metadata(parent) else {
        return false;
    };
    let Ok(child_meta) = std::fs::symlink_metadata(child) else {
        return false;
    };
    if !parent_meta.is_dir()
        || parent_meta.file_type().is_symlink()
        || !child_meta.is_dir()
        || child_meta.file_type().is_symlink()
    {
        return false;
    }
    matches!((parent.canonicalize(), child.canonicalize()), (Ok(parent), Ok(child)) if child.parent() == Some(parent.as_path()))
}

fn artifact_type(path: &str) -> Option<&'static str> {
    match path {
        "manifest.json" => return Some("application/json"),
        "camera.json" => return Some("application/json"),
        "viewer-world.json" => return Some("application/json"),
        "terrain.vtile" => return Some("application/octet-stream"),
        "terrain.vtexarr" => return Some("application/octet-stream"),
        _ => {}
    }
    if path.len() > 128 {
        return None;
    }
    if let Some(name) = path.strip_prefix("overview/t.") {
        let parts: Vec<_> = name.split('.').collect();
        return matches!(parts.as_slice(), [x, z, "vlr"] if x.parse::<i32>().is_ok() && z.parse::<i32>().is_ok())
            .then_some("application/octet-stream");
    }
    let name = path.strip_prefix("tiles/")?;
    let parts: Vec<_> = name.split('.').collect();
    match parts.as_slice() {
        ["t", x, z, "vtile"] if x.parse::<i32>().is_ok() && z.parse::<i32>().is_ok() => {
            Some("application/octet-stream")
        }
        [level, x, z, "vlr"]
            if level
                .strip_prefix('l')
                .and_then(|n| n.parse::<u8>().ok())
                .is_some_and(|n| (1..=30).contains(&n))
                && x.parse::<i32>().is_ok()
                && z.parse::<i32>().is_ok() =>
        {
            Some("application/octet-stream")
        }
        _ => None,
    }
}

impl RendererStore {
    pub(super) fn fetch_visible(
        &self,
        server_id: &str,
        world: &Path,
        dimension: &str,
        artifact: &str,
        selected_version: Option<&str>,
        server_running: &(impl Fn() -> bool + Sync),
    ) -> Result<(u16, Vec<u8>), TerrainError> {
        let _visible = if artifact.starts_with("tiles/t.") {
            self.0.visible_tile_active.fetch_add(1, Ordering::AcqRel);
            Some(VisibleTileRequest(self.0.clone()))
        } else {
            None
        };
        self.fetch(
            server_id,
            world,
            dimension,
            artifact,
            selected_version,
            server_running,
        )
    }

    fn visible_tile_busy(&self) -> bool {
        if self.0.visible_tile_active.load(Ordering::Acquire) > 0 {
            return true;
        }
        self.0
            .last_visible_tile
            .lock()
            .ok()
            .and_then(|last| *last)
            .is_some_and(|last| last.elapsed() < Duration::from_millis(350))
    }

    #[cfg(target_os = "windows")]
    pub(super) fn set_shutdown(&self, shutdown: bool) {
        self.0.shutdown.store(shutdown, Ordering::Release);
    }
    #[cfg(target_os = "windows")]
    pub(super) fn release_checked(&self) -> Result<(), String> {
        let mut current = self.0.current.lock().unwrap();
        if let Some(renderer) = current.as_mut() {
            if renderer
                .child
                .try_wait()
                .map_err(|error| error.to_string())?
                .is_none()
            {
                renderer.child.kill().map_err(|error| error.to_string())?;
            }
            renderer.child.wait().map_err(|error| error.to_string())?;
        }
        current.take();
        self.0.snapshot.lock().unwrap().take();
        Ok(())
    }

    pub(super) fn release(&self) {
        self.0.current.lock().unwrap().take();
        self.0.snapshot.lock().unwrap().take();
    }

    pub(super) fn use_snapshot(
        &self,
        server_id: String,
        source_world: PathBuf,
        snapshot: crate::backup_operations::WorldMapSnapshot,
    ) -> Result<(), ()> {
        let started = Instant::now();
        let bytes = snapshot.bytes;
        let hold_millis = snapshot.hold_millis;
        let next = SavedSnapshot {
            server_id,
            source_world,
            path: snapshot.path,
            _staging: snapshot.staging.map(Arc::new),
            _cache_lock: None,
            edition_cache_key: "java",
        };
        let mut renderer = self.0.current.lock().map_err(|_| ())?;
        if self.0.shutdown.load(Ordering::Acquire) {
            return Err(());
        }
        let mut saved = self.0.snapshot.lock().map_err(|_| ())?;
        // Stop the renderer before deleting the previous snapshot it reads.
        *renderer = None;
        *saved = Some(next);
        eprintln!(
            "Java map: selected refreshed snapshot in {:.1}ms ({} bytes copied; {}ms save hold)",
            started.elapsed().as_secs_f64() * 1000.0,
            bytes,
            hold_millis
        );
        Ok(())
    }

    pub(super) fn use_prepared_world(
        &self,
        server_id: String,
        source_world: PathBuf,
        prepared_world: PathBuf,
        staging: Arc<crate::map_staging::Staging>,
        cache_lock: Arc<std::fs::File>,
    ) -> Result<(), ()> {
        let mut renderer = self.0.current.lock().map_err(|_| ())?;
        if self.0.shutdown.load(Ordering::Acquire) {
            return Err(());
        }
        let mut saved = self.0.snapshot.lock().map_err(|_| ())?;
        if saved.as_ref().is_some_and(|current| {
            current.server_id == server_id
                && current.source_world == source_world
                && current.path == prepared_world
        }) {
            return Ok(());
        }
        *renderer = None;
        *saved = Some(SavedSnapshot {
            _staging: Some(staging),
            _cache_lock: Some(cache_lock),
            server_id,
            source_world,
            path: prepared_world,
            edition_cache_key: BEDROCK_TILE_CACHE_KEY,
        });
        Ok(())
    }

    pub(super) fn restart_for_cache_settings(&self) -> Result<(), ()> {
        let mut renderer = self.0.current.lock().map_err(|_| ())?;
        if self.0.shutdown.load(Ordering::Acquire) {
            return Err(());
        }
        *renderer = None;
        Ok(())
    }

    pub(super) fn prepared_cache_lock(&self, path: &Path) -> Option<Arc<std::fs::File>> {
        self.0
            .snapshot
            .lock()
            .ok()?
            .as_ref()
            .filter(|saved| saved.path == path)?
            ._cache_lock
            .clone()
    }

    pub(super) fn preparation_update(
        &self,
        server_id: &str,
        dimension: &str,
        stage: &str,
        completed: Option<usize>,
        total: Option<usize>,
    ) {
        if let Ok(mut progress) = self.0.progress.lock() {
            *progress = Some(PreparationProgress {
                server_id: server_id.to_owned(),
                dimension: dimension.to_owned(),
                stage: stage.to_owned(),
                completed,
                total,
            });
        }
    }

    pub(super) fn preparation_finished(&self, server_id: &str) {
        if let Ok(mut progress) = self.0.progress.lock()
            && progress
                .as_ref()
                .is_some_and(|item| item.server_id == server_id)
        {
            *progress = None;
        }
    }

    fn fetch(
        &self,
        server_id: &str,
        world: &Path,
        dimension: &str,
        artifact: &str,
        selected_version: Option<&str>,
        server_running: &(impl Fn() -> bool + Sync),
    ) -> Result<(u16, Vec<u8>), TerrainError> {
        let (port, token, first_tile_pending) = {
            let mut guard = self.0.current.lock().map_err(|_| ())?;
            if self.0.shutdown.load(Ordering::Acquire) {
                return Err(().into());
            }
            let mut saved = self.0.snapshot.lock().map_err(|_| ())?;
            if saved.as_ref().is_some_and(|snapshot| {
                snapshot.server_id != server_id || snapshot.source_world != world
            }) {
                *guard = None;
                *saved = None;
            }
            let render_world = saved
                .as_ref()
                .map_or(world, |snapshot| snapshot.path.as_path());
            let edition_cache_key = saved
                .as_ref()
                .map_or("java", |snapshot| snapshot.edition_cache_key);
            let reuse = guard.as_mut().is_some_and(|renderer| {
                renderer.server_id == server_id
                    && renderer.world == render_world
                    && renderer.dimension == dimension
                    && renderer.last_use.elapsed() < IDLE
                    && renderer.child.try_wait().ok().flatten().is_none()
            });
            if reuse
                && artifact == "manifest.json"
                && let Some(renderer) = guard.as_ref()
            {
                eprintln!(
                    "Java map: warm renderer reused (ready for {:.1}s; last request {:.1}s ago)",
                    renderer.started_at.elapsed().as_secs_f64(),
                    renderer.last_use.elapsed().as_secs_f64()
                );
            }
            if !reuse {
                if let Ok(mut progress) = self.0.progress.lock() {
                    *progress = Some(PreparationProgress {
                        server_id: server_id.into(),
                        dimension: dimension.into(),
                        stage: "Checking terrain renderer".into(),
                        completed: None,
                        total: None,
                    });
                }
                let reporter = PreparationReporter(&self.0.progress);
                let source = if saved.is_some() { "snapshot" } else { "world" };
                eprintln!("Java map: selecting {source} input for renderer startup");
                *guard = Some(Renderer::launch(
                    server_id,
                    render_world,
                    world,
                    dimension,
                    RendererIdentity {
                        selected_version,
                        edition_cache_key,
                    },
                    &reporter,
                    server_running,
                )?);
            }
            let renderer = guard.as_mut().ok_or(())?;
            renderer.last_use = Instant::now();
            let first_tile_pending =
                artifact.starts_with("tiles/t.") && !renderer.first_tile_reported;
            (renderer.port, renderer.token.clone(), first_tile_pending)
        };
        self.ensure_sweeper();
        let url = format!("http://127.0.0.1:{port}/v1/worlds/default/{artifact}");
        let response = http()
            .get(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept-Encoding", "identity")
            .call()
            .map_err(|error| {
                eprintln!(
                    "Java terrain artifact request failed: {}",
                    error.to_string().replace(&token, "[redacted]")
                );
                TerrainError::from(())
            })?;
        let status = response.status().as_u16();
        let bytes = response
            .into_body()
            .with_config()
            .limit(MAX_ARTIFACT as u64)
            .read_to_vec()
            .map_err(|error| {
                eprintln!("Java terrain artifact read failed: {error}");
                TerrainError::from(())
            })?;
        if first_tile_pending
            && let Ok(mut guard) = self.0.current.lock()
            && let Some(renderer) = guard.as_mut()
            && renderer.server_id == server_id
            && renderer.port == port
            && status == 200
            && !renderer.first_tile_reported
        {
            renderer.first_tile_reported = true;
            eprintln!(
                "Java map: first tile response completed {:.1}s after renderer startup ({} bytes; HTTP {})",
                renderer.started_at.elapsed().as_secs_f64(),
                bytes.len(),
                status
            );
        }
        Ok((status, bytes))
    }

    fn ensure_sweeper(&self) {
        if self.0.sweeping.swap(true, Ordering::AcqRel) {
            return;
        }
        let weak: Weak<RendererState> = Arc::downgrade(&self.0);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(15));
                let Some(state) = weak.upgrade() else {
                    break;
                };
                let Ok(mut current) = state.current.lock() else {
                    break;
                };
                if current
                    .as_ref()
                    .is_some_and(|renderer| renderer.last_use.elapsed() >= IDLE)
                {
                    *current = None;
                }
                if current.is_none() {
                    state.snapshot.lock().unwrap().take();
                    state.sweeping.store(false, Ordering::Release);
                    break;
                }
            }
        });
    }
}

impl Renderer {
    fn launch(
        server_id: &str,
        world: &Path,
        source_world: &Path,
        dimension: &str,
        identity: RendererIdentity<'_>,
        progress: &PreparationReporter<'_>,
        server_running: &(impl Fn() -> bool + Sync),
    ) -> Result<Self, TerrainError> {
        let binary = dependencies::binary()?;
        progress.update("Checking and preparing Minecraft textures", None, None);
        let assets = dependencies::assets(world, identity.selected_version)?;
        let (tile_cache, cache_key) = java_terrain_compat::persistent_tiles_for_edition(
            server_id,
            source_world,
            dimension,
            &assets,
            identity.edition_cache_key,
        )
        .map_err(|error| TerrainError::new("map_storage_failed", error.to_string()))?;
        let lock_path = tile_cache.join("lock");
        if std::fs::symlink_metadata(&lock_path).is_ok_and(|meta| !meta.file_type().is_file()) {
            return Err(TerrainError::new(
                "map_storage_failed",
                "The Java tile cache lock is not a regular file.",
            ));
        }
        let cache_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(|error| TerrainError::new("map_storage_failed", error.to_string()))?;
        cache_lock
            .lock_exclusive()
            .map_err(|error| TerrainError::new("map_storage_failed", error.to_string()))?;
        progress.update("Inspecting saved terrain", None, None);
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|_| ())?;
        let port = listener.local_addr().map_err(|_| ())?.port();
        drop(listener);
        let staging = crate::map_staging::Staging::create("java-renderer", 0)
            .map_err(|error| TerrainError::new("map_storage_failed", error.to_string()))?;
        let cache = staging.path().to_path_buf();
        let conversion_started = Instant::now();
        let render_world = match java_terrain_compat::prepare(
            server_id,
            world,
            source_world,
            dimension,
            &cache,
            &|completed, total| {
                progress.update("Converting saved terrain", Some(completed), Some(total))
            },
            server_running,
        ) {
            Ok(path) => path,
            Err(error) => {
                eprintln!("Java terrain compatibility preparation failed: {error}");
                return Err(TerrainError::new(
                    "terrain_preparation_failed",
                    format!(
                        "Saved Java terrain preparation failed at {}: {error}",
                        cache.display()
                    ),
                ));
            }
        };
        eprintln!(
            "Java map: compatible region preparation returned in {:.1}s",
            conversion_started.elapsed().as_secs_f64()
        );
        let used = crate::map_staging::estimate_tree(&tile_cache)
            .map_err(|error| TerrainError::new("map_storage_failed", error.to_string()))?;
        let free = fs2::available_space(&tile_cache)
            .map_err(|error| TerrainError::new("map_storage_failed", error.to_string()))?;
        let growth = free.saturating_sub(msc_infrastructure::conversion_resources::DISK_RESERVE);
        if growth < 64 * 1024 * 1024 {
            return Err(TerrainError::new(
                "map_storage_failed",
                "The Java tile cache needs at least 64 MiB beyond MSC's disk reserve. Free space in Storage, then reopen the map.",
            ));
        }
        let requested_mib = java_terrain_compat::tile_cache_budget_mib(&tile_cache).unwrap_or(4096);
        let cache_limit_mib = ((used.saturating_add(growth)).min(requested_mib * 1024 * 1024)
            / (1024 * 1024))
            .max(64);
        let token = format!(
            "{}{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        );
        let (bake_memory_mib, bake_workers) = java_bake_capacity(server_running());
        eprintln!(
            "Java map: renderer allowing {bake_workers} concurrent bakes with {bake_memory_mib} MiB working budget"
        );
        progress.update("Starting terrain renderer", None, None);
        let mut child = Command::new(binary)
            .arg("server")
            .arg(render_world)
            .args(["--dimension", dimension, "--out"])
            .arg(&tile_cache)
            .args(["--cache-key", &cache_key])
            .arg("--assets")
            .arg(assets)
            .args([
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--memory",
                &bake_memory_mib.to_string(),
                "--cache-limit-mib",
                &cache_limit_mib.to_string(),
                "--threads",
                &bake_workers.to_string(),
                "--prebake",
                "off",
                "--players",
                "off",
                "--max-connections",
                "8",
            ])
            .env("VANTAGE_SERVER_TOKEN", &token)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                eprintln!("Java terrain renderer could not start: {error}");
                TerrainError::new("renderer_start_failed", "The Java terrain renderer could not start. Repair MSC on the server host and check the agent logs for details.")
            })?;
        let diagnostics = Arc::new(Mutex::new(Vec::new()));
        let diagnostic_reader = child.stderr.take().map(|mut stderr| {
            let captured = diagnostics.clone();
            // Keep draining after the diagnostic cap so the renderer cannot
            // block on a full pipe or grow the agent's memory without bound.
            std::thread::spawn(move || {
                let mut buffer = [0u8; 4096];
                while let Ok(count) = stderr.read(&mut buffer) {
                    if count == 0 {
                        break;
                    }
                    if let Ok(mut bytes) = captured.lock() {
                        let remaining = (16 * 1024usize).saturating_sub(bytes.len());
                        bytes.extend_from_slice(&buffer[..count.min(remaining)]);
                    }
                }
            })
        });
        let started_at = Instant::now();
        let mut renderer = Self {
            server_id: server_id.to_string(),
            world: world.to_path_buf(),
            dimension: dimension.to_string(),
            port,
            token,
            _staging: staging,
            _cache_lock: cache_lock,
            child,
            diagnostics,
            diagnostic_reader,
            started_at,
            first_tile_reported: false,
            last_use: Instant::now(),
        };
        let startup_started = Instant::now();
        while startup_started.elapsed() < Duration::from_secs(45) {
            if let Some(status) = renderer.child.try_wait().map_err(|_| ())? {
                if let Some(reader) = renderer.diagnostic_reader.take() {
                    let _ = reader.join();
                }
                renderer.log_failure(&format!("exited with {status}"));
                return Err(TerrainError::new(
                    "renderer_start_failed",
                    "The Java terrain renderer exited during startup. See the agent logs on the server host for details.",
                ));
            }
            let health = format!("http://127.0.0.1:{port}/v1/health");
            if let Ok(response) = http().get(&health).call()
                && response.status().as_u16() == 200
            {
                eprintln!(
                    "Java map: renderer ready in {:.1}s",
                    startup_started.elapsed().as_secs_f64()
                );
                return Ok(renderer);
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        renderer.log_failure("did not become ready within 45 seconds");
        Err(TerrainError::new(
            "renderer_start_timeout",
            "The Java terrain renderer did not become ready within 45 seconds. See the agent logs on the server host for details.",
        ))
    }

    fn log_failure(&self, reason: &str) {
        let diagnostics = self
            .diagnostics
            .lock()
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).replace(&self.token, "[redacted]"));
        eprintln!(
            "Java terrain renderer {reason} at {}: {}",
            self._staging.path().display(),
            diagnostics.unwrap_or_default()
        );
    }
}

fn http() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into()
}
