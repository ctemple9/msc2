//! Authenticated, bounded access to one private Vantage Java renderer.
pub(super) mod bedrock;
mod java_terrain_compat;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
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

#[derive(Clone, Default)]
pub(super) struct RendererStore(Arc<RendererState>);

#[derive(Default)]
struct RendererState {
    current: Mutex<Option<Renderer>>,
    snapshot: Mutex<Option<SavedSnapshot>>,
    sweeping: AtomicBool,
}

struct SavedSnapshot {
    server_id: String,
    source_world: PathBuf,
    path: PathBuf,
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
    child: Child,
    last_use: Instant,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.cache);
    }
}

#[derive(Deserialize)]
pub(super) struct ArtifactQuery {
    dimension: String,
    path: String,
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
    let server_id = server.id;
    let dimension_id = query.dimension;
    let artifact = query.path;
    match tokio::task::spawn_blocking(move || {
        store.fetch(&server_id, &world, &dimension_id, &artifact)
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
        _ => error_response(
            StatusCode::BAD_GATEWAY,
            "renderer_unavailable",
            "The terrain renderer is unavailable; check that Java client assets are installed on the host.",
        ),
    }
}

fn is_direct_child(parent: &Path, child: &Path) -> bool {
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
        };
        let mut renderer = self.0.current.lock().map_err(|_| ())?;
        let mut saved = self.0.snapshot.lock().map_err(|_| ())?;
        // Stop the renderer before deleting the previous snapshot it reads.
        *renderer = None;
        *saved = Some(next);
        Ok(())
    }

    fn fetch(
        &self,
        server_id: &str,
        world: &Path,
        dimension: &str,
        artifact: &str,
    ) -> Result<(u16, Vec<u8>), ()> {
        let (port, token) = {
            let mut guard = self.0.current.lock().map_err(|_| ())?;
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
            let reuse = guard.as_mut().is_some_and(|renderer| {
                renderer.server_id == server_id
                    && renderer.world == render_world
                    && renderer.dimension == dimension
                    && renderer.last_use.elapsed() < IDLE
                    && renderer.child.try_wait().ok().flatten().is_none()
            });
            if !reuse {
                *guard = Some(Renderer::launch(server_id, render_world, dimension)?);
            }
            let renderer = guard.as_mut().ok_or(())?;
            renderer.last_use = Instant::now();
            (renderer.port, renderer.token.clone())
        };
        self.ensure_sweeper();
        let url = format!("http://127.0.0.1:{port}/v1/worlds/default/{artifact}");
        let response = http()
            .get(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept-Encoding", "identity")
            .call()
            .map_err(|_| ())?;
        let status = response.status().as_u16();
        let bytes = response
            .into_body()
            .with_config()
            .limit(MAX_ARTIFACT as u64)
            .read_to_vec()
            .map_err(|_| ())?;
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
                    state.sweeping.store(false, Ordering::Release);
                    break;
                }
            }
        });
    }
}

impl Renderer {
    fn launch(server_id: &str, world: &Path, dimension: &str) -> Result<Self, ()> {
        let binary = std::env::var_os("MSC2_VANTAGE_BIN")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::current_exe().ok()?.parent().map(|parent| {
                    parent.join(if cfg!(windows) {
                        "vantage.exe"
                    } else {
                        "vantage"
                    })
                })
            })
            .ok_or(())?;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|_| ())?;
        let port = listener.local_addr().map_err(|_| ())?.port();
        drop(listener);
        let cache = std::env::temp_dir().join(format!("msc-map-renderer-{}", Uuid::new_v4()));
        std::fs::create_dir(&cache).map_err(|_| ())?;
        let render_world = match java_terrain_compat::prepare(world, dimension, &cache) {
            Ok(path) => path,
            Err(error) => {
                eprintln!("Java terrain compatibility preparation failed: {error}");
                let _ = std::fs::remove_dir_all(&cache);
                return Err(());
            }
        };
        let token = format!(
            "{}{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        );
        let child = Command::new(binary)
            .arg("server")
            .arg(render_world)
            .args(["--dimension", dimension, "--out"])
            .arg(cache.join("render"))
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
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| {
                let _ = std::fs::remove_dir_all(&cache);
            })?;
        let mut renderer = Self {
            server_id: server_id.to_string(),
            world: world.to_path_buf(),
            dimension: dimension.to_string(),
            port,
            token,
            cache,
            child,
            last_use: Instant::now(),
        };
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(45) {
            if renderer.child.try_wait().map_err(|_| ())?.is_some() {
                return Err(());
            }
            let health = format!("http://127.0.0.1:{port}/v1/health");
            if let Ok(response) = http().get(&health).call()
                && response.status().as_u16() == 200
            {
                return Ok(renderer);
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        Err(())
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
