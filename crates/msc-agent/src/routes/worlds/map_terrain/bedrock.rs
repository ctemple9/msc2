//! Saved Bedrock tiles exported from a consistent BDS copy.
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use msc_domain::app_config_schema::ConfigServer;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{ArtifactQuery, MAX_ARTIFACT, is_direct_child};
use crate::routes::worlds::{WorldsRoutesState, error_response, is_safe_level_folder};
use msc_infrastructure::fs::StdFileSystem;

const SAMPLES_URL: &str = "https://github.com/Mojang/bedrock-samples/releases/download/v1.26.50.4/bedrock-samples-v1.26.50.4-full.zip";
const SAMPLES_SHA256: &str = "c0b6151f5f9a0c31ebe3c909dfc82d97f12473ed2c6e039aa7e22d667042026c";
const MAX_DOWNLOAD: u64 = 180 * 1024 * 1024;
const MAX_EXTRACTED: u64 = 384 * 1024 * 1024;
const MAX_EXPORT_TIME: Duration = Duration::from_secs(600);
const MAX_NEARBY_CATALOG_TIME: Duration = Duration::from_secs(120);
// A converted multi-gigabyte world can take longer than ten minutes just to
// enumerate saved chunks and build its first overview.
const MAX_CATALOG_EXPORT_TIME: Duration = Duration::from_secs(3_600);

#[derive(Clone, Default)]
pub(crate) struct BedrockStore(Arc<Mutex<BedrockState>>, Arc<ExporterLifecycle>);

#[derive(Default)]
struct ExporterLifecycle {
    shutdown: AtomicBool,
    cleanup_failed: AtomicBool,
}

impl ExporterLifecycle {
    fn stop(&self, child: &mut Child) {
        let _ = child.kill();
        if child.wait().is_err() {
            self.cleanup_failed.store(true, Ordering::Release);
        }
    }
}

#[derive(Default)]
struct BedrockState {
    tiles: BTreeMap<String, BedrockTile>,
    regions: BTreeMap<String, BTreeSet<(i32, i32)>>,
    snapshot: Option<BedrockSnapshot>,
    sessions: BTreeMap<String, TileSession>,
}

impl BedrockState {
    fn require_saved_terrain(&mut self, dimension: &str) -> Result<(), String> {
        let tile = self
            .tiles
            .get(dimension)
            .ok_or("The Bedrock tile is unavailable.")?;
        if tile.tiles.is_empty() {
            self.tiles.remove(dimension);
            self.sessions.remove(dimension);
            self.regions.remove(dimension);
            // A map opened before anyone joins may snapshot an ungenerated
            // world. Retry from fresh saved data instead of pinning that copy.
            // Other rendered dimensions still need their consistent snapshot.
            if self.tiles.is_empty() {
                self.snapshot = None;
            }
            return Err(format!(
                "No saved terrain has been generated in {dimension}."
            ));
        }
        Ok(())
    }
}

struct BedrockSnapshot {
    server_id: String,
    world: PathBuf,
    snapshot: Arc<crate::backup_operations::WorldMapSnapshot>,
}

#[derive(Default)]
struct BedrockTile {
    _staging: Option<crate::map_staging::Staging>,
    output: PathBuf,
    tiles: BTreeSet<String>,
    previewed: BTreeSet<String>,
    detailed: BTreeSet<String>,
    last_preview: Option<Instant>,
    camera: Option<(i32, i32)>,
}

struct TileSession {
    child: Child,
    input: ChildStdin,
    replies: Receiver<String>,
    lifecycle: Arc<ExporterLifecycle>,
}

impl TileSession {
    fn start(
        binary: &Path,
        source: &Path,
        pack: &Path,
        output: &Path,
        dimension: &str,
        lifecycle: Arc<ExporterLifecycle>,
    ) -> Result<Self, String> {
        let mut child = Command::new(binary)
            .arg("tile-session")
            .arg(source)
            .arg(pack)
            .arg(output)
            .arg(dimension)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("Could not start the Bedrock tile helper: {error}"))?;
        let Some(input) = child.stdin.take() else {
            lifecycle.stop(&mut child);
            return Err("The Bedrock tile helper has no input pipe.".into());
        };
        let Some(output_pipe) = child.stdout.take() else {
            lifecycle.stop(&mut child);
            return Err("The Bedrock tile helper has no output pipe.".into());
        };
        let (sender, replies) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output_pipe).lines() {
                match line {
                    Ok(line) => {
                        if sender.send(line).is_err() {
                            break;
                        }
                    }
                    _ => break,
                }
            }
        });
        Ok(Self {
            child,
            input,
            replies,
            lifecycle,
        })
    }

    fn render(&mut self, x: i32, z: i32) -> Result<(), String> {
        match self.request(&format!("{x} {z}"), MAX_EXPORT_TIME)? {
            reply if reply == "ok" => Ok(()),
            reply if reply == "error" => {
                Err("The Bedrock tile helper could not render this tile.".into())
            }
            _ => Err("The Bedrock tile helper gave an invalid reply.".into()),
        }
    }

    fn preview(&mut self, x: i32, z: i32) -> Result<(), String> {
        match self.request(&format!("preview {x} {z}"), MAX_EXPORT_TIME)? {
            reply if reply == "ok" => Ok(()),
            reply if reply == "error" => {
                Err("The Bedrock tile helper could not preview this tile.".into())
            }
            reply => Err(format!("Unexpected Bedrock tile helper reply: {reply}")),
        }
    }

    fn discover(&mut self, x: i32, z: i32) -> Result<Vec<(i32, i32)>, String> {
        let reply = self.request(&format!("region {x} {z}"), MAX_EXPORT_TIME)?;
        let anchors = reply
            .strip_prefix("region ")
            .ok_or("The Bedrock region helper could not inspect saved chunks.")?;
        serde_json::from_str(anchors)
            .map_err(|error| format!("The Bedrock region helper gave an invalid reply: {error}"))
    }

    fn request(&mut self, request: &str, timeout: Duration) -> Result<String, String> {
        writeln!(self.input, "{request}")
            .and_then(|()| self.input.flush())
            .map_err(|error| format!("Could not request Bedrock terrain: {error}"))?;
        let started = Instant::now();
        loop {
            if self.lifecycle.shutdown.load(Ordering::Acquire) {
                return Err("The local agent is shutting down.".into());
            }
            if started.elapsed() >= timeout {
                return Err("The Bedrock terrain helper timed out.".into());
            }
            match self.replies.recv_timeout(Duration::from_millis(100)) {
                Ok(reply) => return Ok(reply),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("The Bedrock tile helper stopped unexpectedly.".into());
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
    }
}

impl Drop for TileSession {
    fn drop(&mut self) {
        self.lifecycle.stop(&mut self.child);
    }
}

pub(crate) struct RefreshStats {
    pub reused_tiles: usize,
    pub changed_tiles: usize,
    pub removed_tiles: usize,
}

pub(super) async fn artifact(
    state: &WorldsRoutesState,
    server: &ConfigServer,
    query: &ArtifactQuery,
    content_type: &'static str,
) -> Response {
    if !matches!(
        query.dimension.as_str(),
        "minecraft:overworld" | "minecraft:the_nether" | "minecraft:the_end"
    ) {
        return error_response(
            StatusCode::CONFLICT,
            "dimension_unavailable",
            "This Bedrock dimension is not supported by the saved terrain map.",
        );
    }
    if !matches!(query.path.as_str(), "manifest.json" | "terrain.vtexarr")
        && !matches!(query.path.strip_prefix("tiles/t."), Some(name) if {
            let parts: Vec<_> = name.split('.').collect();
            matches!(parts.as_slice(), [x, z, "vtile"] if x.parse::<i32>().is_ok() && z.parse::<i32>().is_ok())
        })
        && !matches!(query.path.strip_prefix("overview/t."), Some(name) if {
            let parts: Vec<_> = name.split('.').collect();
            matches!(parts.as_slice(), [x, z, "vlr"] if x.parse::<i32>().is_ok() && z.parse::<i32>().is_ok())
        })
    {
        return error_response(
            StatusCode::BAD_REQUEST,
            "invalid_artifact",
            "The requested Bedrock terrain artifact is not supported.",
        );
    }
    let server_dir = PathBuf::from(&server.server_dir);
    let Some(level_name) =
        msc_application::worlds::read_configured_level_name(&StdFileSystem, &server_dir)
            .filter(|name| is_safe_level_folder(name))
    else {
        return error_response(
            StatusCode::CONFLICT,
            "map_unavailable",
            "The active server has no safe configured Bedrock world folder.",
        );
    };
    let worlds = server_dir.join("worlds");
    let world = worlds.join(level_name);
    if !is_direct_child(&server_dir, &worlds)
        || !is_direct_child(&worlds, &world)
        || !world.join("level.dat").is_file()
        || !world.join("db").is_dir()
    {
        return error_response(
            StatusCode::CONFLICT,
            "map_unavailable",
            "The active Bedrock world has no safely accessible saved terrain.",
        );
    }
    let store = state.bedrock_map.clone();
    let lifecycle = state.lifecycle.clone();
    let server_id = server.id.clone();
    let artifact = query.path.clone();
    let dimension = query.dimension.clone();
    let center = query.center_x.zip(query.center_z);
    match tokio::task::spawn_blocking(move || {
        store.read(
            &lifecycle,
            &server_dir,
            &server_id,
            &world,
            &dimension,
            (&artifact, center),
        )
    })
    .await
    {
        Ok(Ok(bytes)) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type),
                (header::CACHE_CONTROL, "private, no-store"),
            ],
            Body::from(bytes),
        )
            .into_response(),
        Ok(Err(error)) => {
            error_response(StatusCode::BAD_GATEWAY, "bedrock_map_unavailable", &error)
        }
        Err(_) => error_response(
            StatusCode::BAD_GATEWAY,
            "bedrock_map_unavailable",
            "The Bedrock terrain exporter stopped unexpectedly.",
        ),
    }
}

impl BedrockStore {
    #[cfg(target_os = "windows")]
    pub(crate) fn set_shutdown(&self, shutdown: bool) {
        self.1.shutdown.store(shutdown, Ordering::Release);
    }

    #[cfg(target_os = "windows")]
    pub(crate) fn release_checked(&self) -> Result<(), String> {
        // The state lock waits for any active exporter, whose polling loop now
        // observes shutdown. Retain data if its process exit was not confirmed.
        self.release();
        if self.1.cleanup_failed.load(Ordering::Acquire) {
            return Err("Bedrock terrain helper exit could not be confirmed.".into());
        }
        Ok(())
    }

    pub(crate) fn release(&self) {
        let mut state = self.0.lock().unwrap();
        state.sessions.clear();
        state.tiles.clear();
        state.regions.clear();
        state.snapshot = None;
    }

    pub(crate) fn use_snapshot(
        &self,
        server_id: String,
        world: PathBuf,
        snapshot: crate::backup_operations::WorldMapSnapshot,
    ) -> Result<RefreshStats, String> {
        let pending = BedrockSnapshot {
            server_id,
            world,
            snapshot: Arc::new(snapshot),
        };
        let mut current = self
            .0
            .lock()
            .map_err(|_| "The Bedrock map cache is unavailable.")?;
        if self.1.shutdown.load(Ordering::Acquire) {
            return Err("The local agent is shutting down.".into());
        }
        current.sessions.clear();
        current.regions.clear();
        let mut next_tiles = BTreeMap::new();
        let mut stats = RefreshStats {
            reused_tiles: 0,
            changed_tiles: 0,
            removed_tiles: 0,
        };
        if let Some(previous) = current
            .snapshot
            .as_ref()
            .filter(|old| old.server_id == pending.server_id && old.world == pending.world)
            && !current.tiles.is_empty()
        {
            let binary = exporter_binary()?;
            let pack = resource_pack()?;
            for (dimension, old) in &current.tiles {
                let estimate = crate::map_staging::estimate_tree(&old.output)
                    .map_err(|error| error.to_string())?;
                let staging = crate::map_staging::Staging::create("bedrock-tiles", estimate)
                    .map_err(|error| error.to_string())?;
                let output = staging.path().to_path_buf();
                let mut next = BedrockTile {
                    _staging: Some(staging),
                    output,
                    tiles: BTreeSet::new(),
                    ..BedrockTile::default()
                };
                if !run_exporter(
                    Command::new(&binary)
                        .arg("catalog")
                        .arg(&pending.snapshot.path)
                        .arg(&pack)
                        .arg(&next.output)
                        .arg(dimension),
                    &self.1,
                    MAX_CATALOG_EXPORT_TIME,
                ) {
                    return Err(format!(
                        "The refreshed {dimension} catalog could not be exported."
                    ));
                }
                let manifest: serde_json::Value = serde_json::from_slice(
                    &fs::read(next.output.join("manifest.json"))
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| format!("The refreshed Bedrock catalog is invalid: {error}"))?;
                let tiles = manifest["tiles"]
                    .as_array()
                    .ok_or("The refreshed Bedrock catalog has no tile list.")?
                    .iter()
                    .map(|tile| {
                        tile["path"]
                            .as_str()
                            .filter(|path| valid_tile_path(path))
                            .map(str::to_owned)
                            .ok_or("The refreshed Bedrock catalog has an invalid tile path.")
                    })
                    .collect::<Result<BTreeSet<_>, _>>()?;
                let diff_path = next.output.join("diff.json");
                if !run_exporter(
                    Command::new(&binary)
                        .arg("diff-catalogs")
                        .arg(&previous.snapshot.path)
                        .arg(&pending.snapshot.path)
                        .arg(dimension)
                        .arg(old.output.join("manifest.json"))
                        .arg(next.output.join("manifest.json"))
                        .arg(&diff_path),
                    &self.1,
                    MAX_EXPORT_TIME,
                ) {
                    return Err(format!(
                        "The refreshed {dimension} chunks could not be compared."
                    ));
                }
                let diff: serde_json::Value = serde_json::from_slice(
                    &fs::read(&diff_path).map_err(|error| error.to_string())?,
                )
                .map_err(|error| format!("The Bedrock tile comparison is invalid: {error}"))?;
                let unchanged = diff["unchangedTiles"]
                    .as_array()
                    .ok_or("The Bedrock tile comparison has no unchanged tile list.")?;
                for path in unchanged {
                    let path = path
                        .as_str()
                        .filter(|path| {
                            valid_tile_path(path)
                                && old.tiles.contains(*path)
                                && tiles.contains(*path)
                        })
                        .ok_or("The Bedrock tile comparison named an invalid tile.")?;
                    let old_file = old.output.join(path);
                    if old_file.is_file() {
                        fs::copy(old_file, next.output.join(path)).map_err(|error| {
                            format!("Could not keep an unchanged Bedrock tile: {error}")
                        })?;
                        if old.detailed.contains(path) {
                            next.detailed.insert(path.to_owned());
                        } else {
                            next.previewed.insert(path.to_owned());
                        }
                        stats.reused_tiles += 1;
                    }
                }
                for name in ["terrain.vtexarr", "texture-index.json"] {
                    fs::copy(old.output.join(name), next.output.join(name))
                        .map_err(|error| format!("Could not keep Bedrock textures: {error}"))?;
                }
                stats.changed_tiles += diff["changedTiles"]
                    .as_u64()
                    .ok_or("The Bedrock tile comparison has no changed count.")?
                    as usize;
                stats.removed_tiles += diff["removedTiles"]
                    .as_u64()
                    .ok_or("The Bedrock tile comparison has no removed count.")?
                    as usize;
                let _ = fs::remove_file(diff_path);
                next.tiles = tiles;
                if !next.previewed.is_empty() {
                    next.last_preview = Some(Instant::now());
                }
                next_tiles.insert(dimension.to_owned(), next);
            }
        }
        current.tiles = next_tiles;
        current.regions.clear();
        current.snapshot = Some(pending);
        Ok(stats)
    }

    fn read(
        &self,
        lifecycle: &crate::routes::lifecycle::LifecycleRoutesState,
        server_dir: &Path,
        server_id: &str,
        world: &Path,
        dimension: &str,
        request: (&str, Option<(i32, i32)>),
    ) -> Result<Vec<u8>, String> {
        let (artifact, center) = request;
        let mut current = self
            .0
            .lock()
            .map_err(|_| "The Bedrock map cache is unavailable.")?;
        if self.1.shutdown.load(Ordering::Acquire) {
            return Err("The local agent is shutting down.".into());
        }
        if current
            .snapshot
            .as_ref()
            .is_some_and(|saved| saved.server_id != server_id || saved.world != world)
        {
            current.sessions.clear();
            current.tiles.clear();
            current.regions.clear();
            current.snapshot = None;
        }
        if !current.tiles.contains_key(dimension) {
            let pack = resource_pack()?;
            let binary = exporter_binary()?;
            let staging = crate::map_staging::Staging::create("bedrock-tiles", 0)
                .map_err(|error| error.to_string())?;
            let output = staging.path().to_path_buf();
            if current.snapshot.is_none() {
                let snapshot_started = Instant::now();
                let copied = if lifecycle.status_snapshot().running {
                    crate::backup_operations::snapshot_bedrock_world(
                        lifecycle.clone(),
                        server_dir,
                        || self.1.shutdown.load(Ordering::Acquire),
                    )
                } else {
                    let generation = lifecycle.bedrock_run_generation();
                    crate::backup_operations::snapshot_stopped_bedrock_world(server_dir, || {
                        self.1.shutdown.load(Ordering::Acquire)
                            || lifecycle.bedrock_runtime_is_busy()
                            || lifecycle.bedrock_run_generation() != generation
                            || lifecycle.active_server_id().as_deref() != Some(server_id)
                    })
                };
                match copied {
                    Ok(snapshot) => {
                        eprintln!(
                            "Bedrock map: saved snapshot ready in {:.1}s ({} bytes)",
                            snapshot_started.elapsed().as_secs_f64(),
                            snapshot.bytes
                        );
                        current.snapshot = Some(BedrockSnapshot {
                            server_id: server_id.to_owned(),
                            world: world.to_owned(),
                            snapshot: Arc::new(snapshot),
                        })
                    }
                    Err(error) => {
                        return Err(error);
                    }
                }
            }
            let source = current
                .snapshot
                .as_ref()
                .map(|saved| saved.snapshot.path.as_path())
                .unwrap_or(world);
            let nearby = dimension == "minecraft:overworld";
            let catalog_started = Instant::now();
            let rendered = Command::new(&binary)
                .arg(if nearby { "catalog-nearby" } else { "catalog" })
                .arg(source)
                .arg(&pack)
                .arg(&output)
                .arg(dimension)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .and_then(|mut child| {
                    let started = Instant::now();
                    loop {
                        match child.try_wait() {
                            Ok(Some(status)) => break Ok(status.success()),
                            Ok(None) => {}
                            Err(error) => {
                                self.1.stop(&mut child);
                                break Err(error);
                            }
                        }
                        if started.elapsed()
                            >= if nearby {
                                MAX_NEARBY_CATALOG_TIME
                            } else {
                                MAX_CATALOG_EXPORT_TIME
                            }
                        {
                            eprintln!(
                                "Bedrock map: {} catalog export timed out after {} minutes",
                                if nearby { "nearby" } else { "full" },
                                if nearby { 2 } else { 60 }
                            );
                            self.1.stop(&mut child);
                            break Ok(false);
                        }
                        if self.1.shutdown.load(Ordering::Acquire) {
                            self.1.stop(&mut child);
                            break Ok(false);
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                });
            if !rendered.is_ok_and(|success| success) {
                return Err(format!(
                    "The saved Bedrock catalog could not be exported at {}. See the agent log; check saved chunks, resources, disk space and storage quota.",
                    output.display()
                ));
            }
            eprintln!(
                "Bedrock map: {} {dimension} catalog ready in {:.1}s",
                if nearby { "nearby" } else { "full" },
                catalog_started.elapsed().as_secs_f64()
            );
            let manifest: serde_json::Value = serde_json::from_slice(
                &fs::read(output.join("manifest.json")).map_err(|error| error.to_string())?,
            )
            .map_err(|error| format!("The Bedrock tile catalog is invalid: {error}"))?;
            let tiles = manifest["tiles"]
                .as_array()
                .ok_or("The Bedrock tile catalog has no tile list.")?
                .iter()
                .filter_map(|tile| tile["path"].as_str().map(str::to_owned))
                .collect();
            current.tiles.insert(
                dimension.to_owned(),
                BedrockTile {
                    output,
                    tiles,
                    _staging: Some(staging),
                    ..BedrockTile::default()
                },
            );
            // The camera discovers saved regions on demand. A simultaneous
            // full-world scan used the same LevelDB tables and CPU for hours on
            // converted worlds, delaying every visible tile.
        }
        current.require_saved_terrain(dimension)?;
        if artifact == "manifest.json"
            && let Some((x, z)) = center
            && (-30_000_000..=30_000_000).contains(&x)
            && (-30_000_000..=30_000_000).contains(&z)
        {
            let region = (x.div_euclid(256), z.div_euclid(256));
            if !current
                .regions
                .get(dimension)
                .is_some_and(|regions| regions.contains(&region))
                && current.tiles.get(dimension).is_some_and(|tile| {
                    fs::read(tile.output.join("manifest.json"))
                        .ok()
                        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                        .is_some_and(|manifest| manifest["catalogPhase"] == "nearby")
                })
            {
                let source = current
                    .snapshot
                    .as_ref()
                    .ok_or("The Bedrock snapshot is unavailable.")?
                    .snapshot
                    .path
                    .clone();
                let output = current
                    .tiles
                    .get(dimension)
                    .ok_or("The Bedrock tile is unavailable.")?
                    .output
                    .clone();
                let stale = current
                    .sessions
                    .get_mut(dimension)
                    .is_some_and(|session| !matches!(session.child.try_wait(), Ok(None)));
                if stale {
                    current.sessions.remove(dimension);
                }
                if !current.sessions.contains_key(dimension) {
                    current.sessions.insert(
                        dimension.to_owned(),
                        TileSession::start(
                            &exporter_binary()?,
                            &source,
                            &resource_pack()?,
                            &output,
                            dimension,
                            Arc::clone(&self.1),
                        )?,
                    );
                }
                let discovered = current
                    .sessions
                    .get_mut(dimension)
                    .ok_or("The Bedrock tile helper is unavailable.")?
                    .discover(region.0, region.1);
                match discovered {
                    Ok(anchors) => {
                        let count = anchors.len();
                        let tile = current
                            .tiles
                            .get_mut(dimension)
                            .ok_or("The Bedrock tile is unavailable.")?;
                        for (x, z) in anchors {
                            tile.tiles.insert(format!("tiles/t.{x}.{z}.vtile"));
                        }
                        current
                            .regions
                            .entry(dimension.to_owned())
                            .or_default()
                            .insert(region);
                        eprintln!("Bedrock map: discovered {count} saved tiles near block {x},{z}");
                    }
                    Err(error) => {
                        current.sessions.remove(dimension);
                        return Err(error);
                    }
                }
            }
        }
        if artifact == "manifest.json" {
            if let Some(position) = center
                && let Some(tile) = current.tiles.get_mut(dimension)
            {
                tile.camera = Some(position);
            }
            let next = current.tiles.get(dimension).and_then(|tile| {
                if tile
                    .last_preview
                    .is_none_or(|when| when.elapsed() < Duration::from_secs(3))
                {
                    return None;
                }
                let (cx, cz) = tile.camera.unwrap_or((0, 0));
                tile.previewed
                    .difference(&tile.detailed)
                    .filter_map(|path| {
                        let (x, z) = tile_origin(path).ok()?;
                        let dx = i64::from(x) * 16 - i64::from(cx);
                        let dz = i64::from(z) * 16 - i64::from(cz);
                        let distance = dx * dx + dz * dz;
                        (distance <= 512 * 512).then_some((distance, path.clone(), x, z))
                    })
                    .min_by_key(|item| item.0)
            });
            if let Some((_, path, x, z)) = next {
                let source = current
                    .snapshot
                    .as_ref()
                    .ok_or("The Bedrock snapshot is unavailable.")?
                    .snapshot
                    .path
                    .clone();
                let output = current
                    .tiles
                    .get(dimension)
                    .ok_or("The Bedrock tile is unavailable.")?
                    .output
                    .clone();
                let stale = current
                    .sessions
                    .get_mut(dimension)
                    .is_some_and(|session| !matches!(session.child.try_wait(), Ok(None)));
                if stale {
                    current.sessions.remove(dimension);
                }
                if !current.sessions.contains_key(dimension) {
                    current.sessions.insert(
                        dimension.to_owned(),
                        TileSession::start(
                            &exporter_binary()?,
                            &source,
                            &resource_pack()?,
                            &output,
                            dimension,
                            Arc::clone(&self.1),
                        )?,
                    );
                }
                let started = Instant::now();
                let result = current
                    .sessions
                    .get_mut(dimension)
                    .ok_or("The Bedrock tile helper is unavailable.")?
                    .render(x, z);
                eprintln!(
                    "[world-map] Bedrock {dimension} detail {x},{z}: {} after {:.2}s",
                    if result.is_ok() { "ready" } else { "failed" },
                    started.elapsed().as_secs_f64()
                );
                match result {
                    Ok(()) => {
                        if let Some(tile) = current.tiles.get_mut(dimension) {
                            tile.detailed.insert(path);
                        }
                    }
                    Err(error) => {
                        eprintln!("Bedrock map: detail refinement failed: {error}");
                        current.sessions.remove(dimension);
                        if let Some(tile) = current.tiles.get_mut(dimension) {
                            tile.last_preview = Some(Instant::now());
                        }
                    }
                }
            }
        }
        let tile = current
            .tiles
            .get(dimension)
            .ok_or("The Bedrock tile is unavailable.")?;
        let output = tile.output.clone();
        let path = output.join(artifact);
        if artifact == "manifest.json" {
            let bytes = fs::read(&path)
                .map_err(|error| format!("Could not read the Bedrock catalog: {error}"))?;
            let mut manifest: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|error| format!("The Bedrock catalog is invalid: {error}"))?;
            manifest["textureLayers"] = serde_json::json!(atlas_layers(&output)?);
            if manifest["catalogPhase"] == "nearby" {
                let entries = manifest["tiles"]
                    .as_array_mut()
                    .ok_or("The Bedrock catalog has no tile list.")?;
                let known: BTreeSet<_> = entries
                    .iter()
                    .filter_map(|entry| entry["path"].as_str().map(str::to_owned))
                    .collect();
                for path in tile.tiles.difference(&known) {
                    let name = path
                        .strip_prefix("tiles/t.")
                        .and_then(|name| name.strip_suffix(".vtile"));
                    if let Some((x, z)) = name.and_then(|name| name.split_once('.'))
                        && let (Ok(x), Ok(z)) = (x.parse::<i32>(), z.parse::<i32>())
                    {
                        entries.push(serde_json::json!({"x": x, "z": z, "path": path, "bytes": 0, "revision": "preview"}));
                    }
                }
            }
            if let Some(entries) = manifest["tiles"].as_array_mut() {
                for entry in entries {
                    let revision = if entry["path"]
                        .as_str()
                        .is_some_and(|path| tile.detailed.contains(path))
                    {
                        "detail"
                    } else {
                        "preview"
                    };
                    entry["revision"] = serde_json::json!(revision);
                }
            }
            return serde_json::to_vec(&manifest)
                .map_err(|error| format!("Could not send the Bedrock catalog: {error}"));
        }
        if artifact.starts_with("tiles/") {
            if !tile.tiles.contains(artifact) {
                return Err("This Bedrock tile has no saved chunks.".into());
            }
            if !path.is_file() {
                let (x, z) = tile_origin(artifact)?;
                let source = current
                    .snapshot
                    .as_ref()
                    .map_or(world, |saved| saved.snapshot.path.as_path())
                    .to_path_buf();
                let binary = exporter_binary()?;
                let pack = resource_pack()?;
                let started = Instant::now();
                eprintln!("[world-map] Previewing Bedrock {dimension} tile {x},{z}");
                let stale = current
                    .sessions
                    .get_mut(dimension)
                    .is_some_and(|session| !matches!(session.child.try_wait(), Ok(None)));
                if stale {
                    current.sessions.remove(dimension);
                }
                if !current.sessions.contains_key(dimension) {
                    let session = TileSession::start(
                        &binary,
                        &source,
                        &pack,
                        &output,
                        dimension,
                        Arc::clone(&self.1),
                    )?;
                    current.sessions.insert(dimension.to_owned(), session);
                }
                let rendered = current
                    .sessions
                    .get_mut(dimension)
                    .ok_or("The Bedrock tile helper is unavailable.")?
                    .preview(x, z);
                eprintln!(
                    "[world-map] Bedrock {dimension} preview {x},{z}: {} after {:.2}s",
                    if rendered.is_ok() { "ready" } else { "failed" },
                    started.elapsed().as_secs_f64()
                );
                if let Err(error) = rendered {
                    current.sessions.remove(dimension);
                    return Err(format!(
                        "The saved Bedrock tile could not be exported at {}: {error} See the agent log for the exporter error, including any disk or quota failure.",
                        output.display(),
                    ));
                }
                if let Some(tile) = current.tiles.get_mut(dimension) {
                    tile.previewed.insert(artifact.to_owned());
                    tile.last_preview = Some(Instant::now());
                    tile.camera = Some((
                        x.saturating_mul(16).saturating_add(32),
                        z.saturating_mul(16).saturating_add(32),
                    ));
                }
            }
        }
        let metadata =
            fs::metadata(&path).map_err(|_| "The Bedrock tile artifact is unavailable.")?;
        if metadata.len() > MAX_ARTIFACT as u64 {
            return Err("The Bedrock tile artifact exceeds the map size limit.".into());
        }
        fs::read(path).map_err(|error| format!("Could not read the Bedrock tile: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(PathBuf);

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn empty_catalog_releases_snapshot_for_retry_but_keeps_rendered_dimensions() {
        let root = std::env::temp_dir().join(format!("msc-map-empty-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let _cleanup = TestDirectory(root.clone());
        let snapshot_dir = root.join("snapshot");
        fs::create_dir(&snapshot_dir).unwrap();
        let empty_output = root.join("empty");
        fs::create_dir(&empty_output).unwrap();
        let mut state = BedrockState {
            snapshot: Some(BedrockSnapshot {
                server_id: "server".into(),
                world: root.join("live-world"),
                snapshot: Arc::new(crate::backup_operations::WorldMapSnapshot {
                    staging: None,
                    path: snapshot_dir.join("world"),
                    bytes: 0,
                    hold_millis: 0,
                }),
            }),
            ..BedrockState::default()
        };
        state.tiles.insert(
            "minecraft:overworld".into(),
            BedrockTile {
                _staging: None,
                output: empty_output.clone(),
                tiles: BTreeSet::new(),
                ..BedrockTile::default()
            },
        );

        assert!(state.require_saved_terrain("minecraft:overworld").is_err());
        assert!(state.tiles.is_empty());
        assert!(state.snapshot.is_none());
        assert!(!empty_output.exists());
        assert!(!snapshot_dir.exists());

        fs::create_dir(&snapshot_dir).unwrap();
        state.snapshot = Some(BedrockSnapshot {
            server_id: "server".into(),
            world: root.join("live-world"),
            snapshot: Arc::new(crate::backup_operations::WorldMapSnapshot {
                staging: None,
                path: snapshot_dir.join("world"),
                bytes: 1,
                hold_millis: 0,
            }),
        });
        state.tiles.insert(
            "minecraft:overworld".into(),
            BedrockTile {
                _staging: None,
                output: root.join("populated"),
                tiles: BTreeSet::from(["tiles/t.0.0.vtile".into()]),
                ..BedrockTile::default()
            },
        );
        state.tiles.insert(
            "minecraft:the_nether".into(),
            BedrockTile {
                _staging: None,
                output: root.join("empty-nether"),
                tiles: BTreeSet::new(),
                ..BedrockTile::default()
            },
        );
        assert!(state.require_saved_terrain("minecraft:the_nether").is_err());
        assert!(state.require_saved_terrain("minecraft:overworld").is_ok());
        assert!(state.snapshot.is_some());
        assert!(snapshot_dir.exists());
        drop(state);
    }
}

fn valid_tile_path(path: &str) -> bool {
    let Some(name) = path
        .strip_prefix("tiles/t.")
        .and_then(|name| name.strip_suffix(".vtile"))
    else {
        return false;
    };
    let Some((x, z)) = name.split_once('.') else {
        return false;
    };
    x.parse::<i32>().is_ok() && z.parse::<i32>().is_ok()
}

fn tile_origin(path: &str) -> Result<(i32, i32), String> {
    let name = path
        .strip_prefix("tiles/t.")
        .and_then(|name| name.strip_suffix(".vtile"))
        .ok_or("Invalid Bedrock tile path")?;
    let (x, z) = name.split_once('.').ok_or("Invalid Bedrock tile path")?;
    let x = x
        .parse::<i32>()
        .map_err(|_| "Invalid Bedrock tile x")?
        .checked_mul(4)
        .ok_or("Bedrock tile x exceeds world bounds")?;
    let z = z
        .parse::<i32>()
        .map_err(|_| "Invalid Bedrock tile z")?
        .checked_mul(4)
        .ok_or("Bedrock tile z exceeds world bounds")?;
    Ok((x, z))
}

fn atlas_layers(output: &Path) -> Result<u32, String> {
    let mut header = [0u8; 20];
    File::open(output.join("terrain.vtexarr"))
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|error| format!("Could not read Bedrock map textures: {error}"))?;
    if &header[..4] != b"VTA1" {
        return Err("The Bedrock map texture atlas is invalid.".into());
    }
    Ok(u32::from_le_bytes(header[16..20].try_into().unwrap()))
}

fn exporter_binary() -> Result<PathBuf, String> {
    let binary = std::env::var_os("MSC2_BEDROCK_MAP_BIN")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe().ok()?.parent().map(|path| {
                path.join(if cfg!(windows) {
                    "bedrock-map.exe"
                } else {
                    "bedrock-map"
                })
            })
        })
        .ok_or("The Bedrock terrain exporter could not be located.")?;
    if !binary.is_file() {
        return Err("The Bedrock terrain exporter is missing beside the MSC agent.".into());
    }
    Ok(binary)
}

fn run_exporter(command: &mut Command, lifecycle: &ExporterLifecycle, timeout: Duration) -> bool {
    run_exporter_cancellable(command, lifecycle, Some(timeout), None)
}

fn run_exporter_cancellable(
    command: &mut Command,
    lifecycle: &ExporterLifecycle,
    timeout: Option<Duration>,
    cancel: Option<&AtomicBool>,
) -> bool {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        // Keep exporter failures in the agent log rather than hiding them
        // behind a generic HTTP error and the viewer's retry loop.
        .stderr(Stdio::inherit())
        .spawn()
        .and_then(|mut child| {
            let started = Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => break Ok(status.success()),
                    Ok(None) => {}
                    Err(error) => {
                        lifecycle.stop(&mut child);
                        break Err(error);
                    }
                }
                if let Some(limit) = timeout
                    && started.elapsed() >= limit
                {
                    eprintln!(
                        "Bedrock map: exporter timed out after {} minutes",
                        limit.as_secs() / 60
                    );
                    lifecycle.stop(&mut child);
                    break Ok(false);
                }
                if lifecycle.shutdown.load(Ordering::Acquire)
                    || cancel.is_some_and(|flag| flag.load(Ordering::Acquire))
                {
                    lifecycle.stop(&mut child);
                    break Ok(false);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        })
        .unwrap_or(false)
}

fn resource_pack() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("MSC2_BEDROCK_RESOURCE_PACK") {
        let path = PathBuf::from(path);
        if path.join("textures/terrain_texture.json").is_file() {
            return Ok(path);
        }
        return Err(
            "MSC2_BEDROCK_RESOURCE_PACK does not contain textures/terrain_texture.json.".into(),
        );
    }
    let base = if let Some(path) = std::env::var_os("MSC2_DATA_DIR") {
        PathBuf::from(path)
    } else {
        #[cfg(target_os = "macos")]
        {
            PathBuf::from(
                std::env::var_os("HOME")
                    .ok_or("HOME is unavailable for Bedrock texture storage.")?,
            )
            .join("Library/Application Support/MSC2")
        }
        #[cfg(target_os = "windows")]
        {
            PathBuf::from(
                std::env::var_os("LOCALAPPDATA")
                    .ok_or("LOCALAPPDATA is unavailable for Bedrock texture storage.")?,
            )
            .join("MSC2")
        }
        #[cfg(target_os = "linux")]
        {
            PathBuf::from(
                std::env::var_os("HOME")
                    .ok_or("HOME is unavailable for Bedrock texture storage.")?,
            )
            .join(".local/share/msc2")
        }
    };
    let root = base.join("bedrock-map-assets");
    let pack = root.join("v1.26.50.4-models");
    if pack.join("textures/terrain_texture.json").is_file() && pack.join("blocks.json").is_file() {
        return Ok(pack);
    }
    fs::create_dir_all(&root)
        .map_err(|error| format!("Could not prepare Bedrock textures: {error}"))?;
    let staging = root.join(format!("download-{}", Uuid::new_v4()));
    fs::create_dir(&staging)
        .map_err(|error| format!("Could not prepare Bedrock textures: {error}"))?;
    let result = download_resource_pack(&staging);
    let installed = result.and_then(|()| {
        fs::rename(staging.join("pack"), &pack)
            .map_err(|error| format!("Could not install verified Bedrock textures: {error}"))
    });
    let _ = fs::remove_dir_all(&staging);
    installed?;
    if pack.join("textures/terrain_texture.json").is_file() && pack.join("blocks.json").is_file() {
        Ok(pack)
    } else {
        Err("The verified Bedrock texture pack could not be installed.".into())
    }
}

fn download_resource_pack(staging: &Path) -> Result<(), String> {
    let client: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .into();
    let response = client
        .get(SAMPLES_URL)
        .call()
        .map_err(|error| format!("Could not download official Bedrock sample textures: {error}"))?;
    let archive_path = staging.join("samples.zip");
    let mut archive_file = File::create(&archive_path).map_err(|error| error.to_string())?;
    let copied = std::io::copy(
        &mut response.into_body().as_reader().take(MAX_DOWNLOAD + 1),
        &mut archive_file,
    )
    .map_err(|error| format!("Bedrock texture download failed: {error}"))?;
    if copied > MAX_DOWNLOAD {
        return Err("Bedrock texture download exceeded its size limit.".into());
    }
    archive_file.flush().map_err(|error| error.to_string())?;
    let mut checksum_file = File::open(&archive_path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut checksum_file, &mut hasher).map_err(|error| error.to_string())?;
    let actual = format!("{:x}", hasher.finalize());
    if actual != SAMPLES_SHA256 {
        return Err("The Bedrock texture download failed its checksum check.".into());
    }
    let mut archive =
        zip::ZipArchive::new(File::open(&archive_path).map_err(|error| error.to_string())?)
            .map_err(|error| format!("Could not open verified Bedrock textures: {error}"))?;
    let pack = staging.join("pack");
    let mut extracted = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let name = entry.name();
        let Some(relative) = name
            .strip_prefix("resource_pack/")
            .or_else(|| name.split_once("/resource_pack/").map(|(_, rest)| rest))
        else {
            continue;
        };
        if !(relative.starts_with("textures/") || relative == "blocks.json")
            || entry.is_dir()
            || entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            continue;
        }
        let relative = Path::new(relative);
        if relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            continue;
        }
        extracted = extracted
            .checked_add(entry.size())
            .ok_or("Bedrock texture archive size overflow")?;
        if extracted > MAX_EXTRACTED {
            return Err("Bedrock textures exceed the extraction size limit.".into());
        }
        let destination = pack.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut output = File::create(destination).map_err(|error| error.to_string())?;
        std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
    }
    if !pack.join("textures/terrain_texture.json").is_file() || !pack.join("blocks.json").is_file()
    {
        return Err("The verified Bedrock archive did not include terrain textures.".into());
    }
    Ok(())
}
