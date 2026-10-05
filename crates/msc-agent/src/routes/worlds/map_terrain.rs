//! Authenticated, bounded access to one private Vantage Java renderer.
pub(super) mod bedrock;
mod dependencies;
mod java_terrain_compat;
mod preparation;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{Extension, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
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
    current: Mutex<Option<Arc<Renderer>>>,
    leases: Mutex<std::collections::BTreeMap<String, Arc<Renderer>>>,
    prepared: preparation::PreparedStore,
    snapshot: Mutex<Option<Arc<SavedSnapshot>>>,
    sweeping: AtomicBool,
    snapshot_epoch: AtomicU64,
    baseline_gate: Mutex<()>,
}

struct SavedSnapshot {
    server_id: String,
    source_world: PathBuf,
    path: PathBuf,
    captured_at: u64,
}

impl Drop for SavedSnapshot {
    fn drop(&mut self) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }
}

struct Renderer {
    server_id: String,
    world: PathBuf,
    dimension: String,
    port: u16,
    token: String,
    cache: PathBuf,
    child: Mutex<Child>,
    diagnostics: Arc<Mutex<Vec<u8>>>,
    diagnostic_reader: Option<std::thread::JoinHandle<()>>,
    last_use: Mutex<Instant>,
    generation: String,
    _snapshot: Option<Arc<SavedSnapshot>>,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        if let Ok(child) = self.child.get_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(reader) = self.diagnostic_reader.take() {
            let _ = reader.join();
        }
        let _ = std::fs::remove_dir_all(&self.cache);
    }
}

#[derive(Deserialize)]
pub(super) struct ArtifactQuery {
    dimension: String,
    path: String,
    generation: Option<String>,
    #[serde(rename = "serverId")]
    server_id: Option<String>,
}

pub(super) async fn artifact(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Query(query): Query<ArtifactQuery>,
) -> Response {
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
    if query.server_id.as_deref().is_some_and(|id| id != server.id) {
        return error_response(
            StatusCode::CONFLICT,
            "server_binding_changed",
            "The selected server changed; the prior scene was retained.",
        );
    }
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
    if preparation::required(&state, &server)
        || state.map_renderer.0.prepared.has_server(&server.id)
    {
        return preparation::artifact(state, server, world, query, content_type).await;
    }
    let selected_version = crate::routes::versions::minecraft_version_from_selection(
        Some(server.java_flavor),
        server.minecraft_version.clone(),
    );
    let server_id = server.id;
    let dimension_id = query.dimension;
    let artifact = query.path;
    let generation = query.generation;
    match tokio::task::spawn_blocking(move || {
        store.fetch(
            &server_id,
            &world,
            &dimension_id,
            &artifact,
            selected_version.as_deref(),
            generation.as_deref(),
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
    pub(super) fn use_snapshot(
        &self,
        server_id: String,
        source_world: PathBuf,
        snapshot: crate::backup_operations::WorldMapSnapshot,
    ) -> Result<(), ()> {
        let next = SavedSnapshot {
            server_id,
            source_world,
            path: snapshot.path,
            captured_at: super::now_unix(),
        };
        let mut renderer = self.0.current.lock().map_err(|_| ())?;
        let mut saved = self.0.snapshot.lock().map_err(|_| ())?;
        // Readers own their snapshot lease until their generation retires.
        let retired = renderer.take();
        *saved = Some(Arc::new(next));
        self.0.prepared.invalidate_snapshots(&self.0.snapshot_epoch);
        drop(saved);
        drop(renderer);
        drop(retired);
        Ok(())
    }

    fn fetch(
        &self,
        server_id: &str,
        world: &Path,
        dimension: &str,
        artifact: &str,
        selected_version: Option<&str>,
        generation: Option<&str>,
    ) -> Result<(u16, Vec<u8>), TerrainError> {
        if let Some(generation) = generation {
            let renderer = self
                .0
                .leases
                .lock()
                .map_err(|_| ())?
                .get(generation)
                .cloned()
                .filter(|r| {
                    r.server_id == server_id
                        && r.dimension == dimension
                        && r._snapshot
                            .as_ref()
                            .map_or(r.world.as_path(), |s| s.source_world.as_path())
                            == world
                })
                .ok_or_else(|| {
                    TerrainError::new(
                        "map_generation_retired",
                        "This map generation has retired. Reopen the saved map.",
                    )
                })?;
            return renderer.read(artifact);
        }
        let _launch_gate = self.0.baseline_gate.lock().map_err(|_| ())?;
        let epoch = self.0.snapshot_epoch.load(Ordering::Acquire);
        let snapshot = self
            .0
            .snapshot
            .lock()
            .map_err(|_| ())?
            .as_ref()
            .filter(|s| s.server_id == server_id && s.source_world == world)
            .cloned();
        let render_world = snapshot.as_ref().map_or(world, |s| s.path.as_path());
        let reusable = self
            .0
            .current
            .lock()
            .map_err(|_| ())?
            .as_ref()
            .filter(|r| {
                r.server_id == server_id
                    && r.world == render_world
                    && r.dimension == dimension
                    && r.alive()
            })
            .cloned();
        let renderer = if let Some(renderer) = reusable {
            renderer
        } else {
            // Dependencies, compatibility and helper health all run outside the global mutex.
            let mut next = Renderer::launch(server_id, render_world, dimension, selected_version)?;
            next._snapshot = snapshot.clone();
            if self.0.snapshot_epoch.load(Ordering::Acquire) != epoch {
                return Err(TerrainError::new(
                    "snapshot_changed",
                    "The saved snapshot changed during preparation; reopen the map.",
                ));
            }
            let next = Arc::new(next);
            let mut leases = self.0.leases.lock().map_err(|_| ())?;
            leases.retain(|_, r| !r.idle());
            if leases.len() >= 4 {
                return Err(TerrainError::new(
                    "map_reader_limit",
                    "Close unused maps before preparing another generation.",
                ));
            }
            leases.insert(next.generation.clone(), next.clone());
            *self.0.current.lock().map_err(|_| ())? = Some(next.clone());
            next
        };
        self.ensure_sweeper();
        renderer.read(artifact)
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
                if current.as_ref().is_some_and(|renderer| renderer.idle()) {
                    *current = None;
                }
                if let Ok(mut leases) = state.leases.lock() {
                    leases.retain(|_, r| !r.idle());
                }
                state.prepared.sweep();
                if current.is_none() && !state.prepared.active() {
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
        dimension: &str,
        selected_version: Option<&str>,
    ) -> Result<Self, TerrainError> {
        let binary = dependencies::binary()?;
        let assets = dependencies::assets(world, selected_version)?;
        let cache = std::env::temp_dir().join(format!("msc-map-renderer-{}", Uuid::new_v4()));
        std::fs::create_dir(&cache).map_err(|_| ())?;
        let render_world = match java_terrain_compat::prepare(world, dimension, &cache) {
            Ok(path) => path,
            Err(error) => {
                eprintln!("Java terrain compatibility preparation failed: {error}");
                let _ = std::fs::remove_dir_all(&cache);
                return Err(TerrainError::new(
                    "terrain_preparation_failed",
                    "The saved Java terrain could not be prepared for rendering. See the agent logs on the server host for details.",
                ));
            }
        };
        Self::launch_at(
            server_id,
            world,
            dimension,
            &render_world,
            dimension,
            &assets,
            cache,
            binary,
            &|| false,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn launch_at(
        server_id: &str,
        world: &Path,
        dimension: &str,
        render_world: &Path,
        renderer_dimension: &str,
        assets: &Path,
        cache: PathBuf,
        binary: PathBuf,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Self, TerrainError> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|_| ())?;
        let port = listener.local_addr().map_err(|_| ())?.port();
        drop(listener);
        let token = format!(
            "{}{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        );
        let mut child = Command::new(binary)
            .arg("server")
            .arg(render_world)
            .args(["--dimension", renderer_dimension, "--out"])
            .arg(cache.join("render"))
            .arg("--assets")
            .arg(assets)
            .args([
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--memory",
                "512",
                "--threads",
                "2",
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
                let _ = std::fs::remove_dir_all(&cache);
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
        let mut renderer = Self {
            server_id: server_id.to_string(),
            world: world.to_path_buf(),
            dimension: dimension.to_string(),
            port,
            token,
            cache,
            child: Mutex::new(child),
            diagnostics,
            diagnostic_reader,
            last_use: Mutex::new(Instant::now()),
            generation: Uuid::new_v4().simple().to_string(),
            _snapshot: None,
        };
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(45) {
            if cancel() {
                return Err(TerrainError::new(
                    "cancelled",
                    "Map preparation was cancelled; prior scene retained.",
                ));
            }
            if let Some(status) = renderer
                .child
                .lock()
                .map_err(|_| ())?
                .try_wait()
                .map_err(|_| ())?
            {
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

    fn idle(&self) -> bool {
        self.last_use
            .lock()
            .map(|t| t.elapsed() >= IDLE)
            .unwrap_or(true)
    }
    fn alive(&self) -> bool {
        !self.idle()
            && self
                .child
                .lock()
                .ok()
                .is_some_and(|mut child| child.try_wait().ok().flatten().is_none())
    }
    fn read(&self, artifact: &str) -> Result<(u16, Vec<u8>), TerrainError> {
        if let Ok(mut t) = self.last_use.lock() {
            *t = Instant::now();
        }
        let url = format!(
            "http://127.0.0.1:{}/v1/worlds/default/{artifact}",
            self.port
        );
        let response = http()
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept-Encoding", "identity")
            .call()
            .map_err(|_| TerrainError::from(()))?;
        let status = response.status().as_u16();
        let mut bytes = response
            .into_body()
            .with_config()
            .limit(MAX_ARTIFACT as u64)
            .read_to_vec()
            .map_err(|_| TerrainError::from(()))?;
        if artifact == "manifest.json" && status == 200 {
            let mut manifest: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| TerrainError::from(()))?;
            manifest["mscGenerationId"] = self.generation.clone().into();
            bytes = serde_json::to_vec(&manifest).map_err(|_| TerrainError::from(()))?;
        }
        Ok((status, bytes))
    }

    fn log_failure(&self, reason: &str) {
        let diagnostics = self
            .diagnostics
            .lock()
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).replace(&self.token, "[redacted]"));
        eprintln!(
            "Java terrain renderer {reason}: {}",
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

pub(super) fn rendering_status(
    self_state: &WorldsRoutesState,
    context: &msc_application::map_assets::Context,
    dimension: &str,
) -> msc_domain::map_assets::RenderingStatus {
    self_state.map_renderer.0.prepared.status(
        context,
        dimension,
        self_state
            .map_renderer
            .0
            .snapshot_epoch
            .load(Ordering::Acquire),
    )
}
pub(super) fn prepare_resources(
    state: WorldsRoutesState,
    context: msc_application::map_assets::Context,
    dimension: String,
    area: Option<msc_domain::map_assets::Area>,
    force: bool,
) -> Result<String, (&'static str, String)> {
    state
        .map_renderer
        .0
        .prepared
        .clone()
        .start(state, context, dimension, area, force)
        .map_err(|e| (e.code, e.message))
}

pub(super) fn retained_report(
    state: &WorldsRoutesState,
    context: &msc_application::map_assets::Context,
) -> Option<msc_domain::map_assets::Report> {
    state.map_renderer.0.prepared.report(context)
}

pub(super) fn import_resources(
    state: WorldsRoutesState,
    context: msc_application::map_assets::Context,
    dimension: String,
    area: Option<msc_domain::map_assets::Area>,
    path: PathBuf,
    sha: String,
) -> Result<String, (&'static str, String)> {
    state
        .map_renderer
        .0
        .prepared
        .clone()
        .start_job(
            state,
            context,
            dimension,
            area,
            true,
            Some((path, sha)),
            None,
        )
        .map_err(|e| (e.code, e.message))
}

pub(super) fn mutate_resources(
    state: WorldsRoutesState,
    context: msc_application::map_assets::Context,
    dimension: String,
    area: Option<msc_domain::map_assets::Area>,
    mutation: msc_application::map_assets::ResourceMutation,
) -> Result<String, (&'static str, String)> {
    state
        .map_renderer
        .0
        .prepared
        .clone()
        .start_job(state, context, dimension, area, true, None, Some(mutation))
        .map_err(|e| (e.code, e.message))
}
pub(super) fn check_prepared(
    state: &WorldsRoutesState,
    context: &msc_application::map_assets::Context,
    dimension: &str,
    area: msc_domain::map_assets::Area,
    operation: &str,
    cancel: &dyn Fn() -> bool,
) -> Option<
    std::io::Result<(
        msc_infrastructure::map_assets::store::Candidate,
        msc_domain::map_assets::ResourceManifest,
        msc_domain::map_assets::Report,
    )>,
> {
    state
        .map_renderer
        .0
        .prepared
        .check_saved(state, context, dimension, area, operation, cancel)
}
pub(super) fn has_prepared_scene(
    state: &WorldsRoutesState,
    context: &msc_application::map_assets::Context,
) -> bool {
    state.map_renderer.0.prepared.report(context).is_some()
}

pub(super) fn inspection_snapshot(
    state: &WorldsRoutesState,
    context: &msc_application::map_assets::Context,
) -> Option<(msc_application::map_assets::Context, Arc<dyn Send + Sync>)> {
    let saved = state.map_renderer.0.snapshot.lock().ok()?.clone()?;
    if saved.server_id != context.server.id
        || !matches!(&context.world, msc_infrastructure::map_assets::saved_terrain::WorldSource::Directory(path) if *path == saved.source_world)
    {
        return None;
    }
    let mut inspection = context.clone();
    inspection.world =
        msc_infrastructure::map_assets::saved_terrain::WorldSource::Directory(saved.path.clone());
    Some((inspection, saved))
}
