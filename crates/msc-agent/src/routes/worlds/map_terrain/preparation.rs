//! Mod-resource work is operation-backed, deduplicated and never holds the renderer mutex.
use super::*;
use msc_application::map_assets::{self as service, Context};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::map_assets::{Area, RenderingStatus, Report, RequiredClientSource};
use msc_domain::operation::OperationId;
use msc_infrastructure::map_assets::{
    self as assets,
    adoption::{Coordinator, Ticket},
    saved_terrain::WorldSource,
};
use std::collections::BTreeMap;

#[derive(Clone, Default)]
pub(super) struct PreparedStore(Arc<PreparedInner>);
#[derive(Default)]
struct PreparedInner {
    coordinator: Mutex<Coordinator<Scene>>,
    generations: Mutex<BTreeMap<String, Arc<Scene>>>,
    details: Mutex<BTreeMap<String, RenderingStatus>>,
}
struct Scene {
    renderer: Arc<Renderer>,
    key: String,
    binding: msc_domain::map_assets::Binding,
    report: Report,
    status: RenderingStatus,
    tile_fingerprints: BTreeMap<String, String>,
    snapshot_epoch: u64,
    atlas_digest: String,
    atlas_layers: u32,
    inventory: assets::inventory::Inventory,
    resource_manifest: msc_domain::map_assets::ResourceManifest,
    missing_sources: bool,
    input_prerequisites: bool,
}
pub(super) fn required(state: &WorldsRoutesState, server: &ConfigServer) -> bool {
    if let Ok(context) = active_context(state, server)
        && let Ok(store) = state.map_assets.store()
        && let Ok(root) = assets::bundle::receipt_root(&store, &context.binding)
        && root.join("current.json").is_file()
    {
        return true;
    }
    let root = Path::new(&server.server_dir);
    if root.join(".msc-map-source/current").is_file() {
        return true;
    }
    if let Ok(paths) = assets::inventory::source_paths(&root.join("mods"))
        && paths
            .iter()
            .any(|p| p.extension().is_some_and(|e| e == "jar"))
    {
        return true;
    }
    assets::read(&root.join("server.properties"), assets::MAX_JSON)
        .ok()
        .is_some_and(|raw| {
            String::from_utf8_lossy(&raw).lines().any(|line| {
                line.strip_prefix("resource-pack=")
                    .is_some_and(|url| !url.trim().is_empty())
            })
        })
}
fn active_context(state: &WorldsRoutesState, server: &ConfigServer) -> std::io::Result<Context> {
    let root = Path::new(&server.server_dir);
    let marker = root.join("world_slots/active_slot_id.txt");
    let slot = if marker.exists() {
        String::from_utf8(assets::read(&marker, 4096)?)
            .map_err(|_| assets::error("invalid_active_slot"))?
            .trim()
            .to_string()
    } else {
        let mut slots = Vec::new();
        for path in assets::inventory::source_paths(&root.join("world_slots"))? {
            let path = path.join("slot.json");
            if !path.exists() {
                continue;
            }
            let value: serde_json::Value =
                serde_json::from_slice(&assets::read(&path, assets::MAX_JSON)?)
                    .map_err(|_| assets::error("invalid_slot_metadata"))?;
            slots.push(
                msc_domain::world::WorldSlot::decode(&value)
                    .map_err(|_| assets::error("invalid_slot_metadata"))?,
            );
        }
        msc_domain::world::resolve_active_slot_id(&slots, None)
            .ok_or_else(|| assets::error("active_slot_required"))?
    };
    let host = state
        .lifecycle
        .map_assets_host_id()
        .map_err(|_| assets::error("host_identity_unavailable"))?;
    service::context(server, &host, &slot)
}
fn key(context: &Context, dimension: &str) -> std::io::Result<String> {
    assets::hash_json(&(
        &context.binding.agent_host_id,
        &context.binding.server_id,
        &context.binding.slot_id,
        &context.server.server_dir,
        match &context.world {
            WorldSource::Directory(path) | WorldSource::Archive(path) => path,
        },
        dimension,
    ))
}
fn revision(context: &Context, epoch: u64) -> std::io::Result<String> {
    // A saved scene does not silently move on each autosave. Explicit snapshot
    // refresh changes epoch; source age is reported separately.
    let mut inputs = context.revision_inputs.clone();
    inputs.remove("sourceIdentity");
    let root = msc_infrastructure::config_repository::default_app_data_dir()
        .join("map-assets/imports")
        .join(assets::hash_json(&(
            &context.binding.agent_host_id,
            &context.binding.server_id,
            &context.binding.slot_id,
        ))?);
    if root.join("current.json").exists() {
        inputs.insert(
            "importedClientResources".into(),
            assets::hash(&assets::read(&root.join("current.json"), assets::MAX_JSON)?),
        );
    }
    assets::hash_json(&(
        &inputs,
        &context.server.minecraft_version,
        &context.server.loader_version,
        context.server.java_flavor.raw_value(),
        epoch,
    ))
}
fn unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn empty_status(state: &str) -> RenderingStatus {
    RenderingStatus {
        state: state.into(),
        operation_id: None,
        generation_id: None,
        resource_generation_id: None,
        snapshot_id: None,
        snapshot_at_unix: None,
        resources_at_unix: None,
        stale: false,
        retryable: false,
        reason_code: None,
        required_sources: vec![],
        prerequisites: vec![],
        affected_tiles: 0,
        unchanged_tiles: 0,
        note: "Preparation has not adopted a map generation.".into(),
    }
}
impl PreparedStore {
    pub fn invalidate_snapshots(&self, epoch: &AtomicU64) {
        if let Ok(mut coordinator) = self.0.coordinator.lock() {
            epoch.fetch_add(1, Ordering::AcqRel);
            let keys = coordinator.entries.keys().cloned().collect::<Vec<_>>();
            for key in keys {
                coordinator.cancel(&key);
            }
        }
    }

    pub fn active(&self) -> bool {
        self.0
            .coordinator
            .lock()
            .map(|c| !c.entries.is_empty())
            .unwrap_or(false)
    }
    pub fn report(&self, context: &Context) -> Option<Report> {
        self.report_for(context, None)
    }
    fn report_for(&self, context: &Context, dimension: Option<&str>) -> Option<Report> {
        let stored = state_report(context);
        let coordinator = self.0.coordinator.lock().ok()?;
        let scenes = coordinator
            .entries
            .values()
            .filter_map(|e| e.current.as_ref())
            .filter(|scene| {
                dimension.is_none_or(|d| scene.report.dimension == d)
                    && scene.key == key(context, &scene.report.dimension).unwrap_or_default()
            })
            .collect::<Vec<_>>();
        if let Some(report) = stored
            && scenes
                .iter()
                .any(|scene| report.geometry_generation_id == scene.report.geometry_generation_id)
        {
            return Some(report);
        }
        scenes
            .into_iter()
            .max_by_key(|scene| scene.status.resources_at_unix)
            .map(|scene| scene.report.clone())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn check_saved(
        &self,
        state: &WorldsRoutesState,
        context: &Context,
        dimension: &str,
        area: Area,
        operation: &str,
        cancel: &dyn Fn() -> bool,
    ) -> Option<
        std::io::Result<(
            assets::store::Candidate,
            msc_domain::map_assets::ResourceManifest,
            Report,
        )>,
    > {
        let key = match key(context, dimension) {
            Ok(key) => key,
            Err(error) => return Some(Err(error)),
        };
        let scene = self.current(&key, None)?;
        Some((|| {
            let epoch = state.map_renderer.0.snapshot_epoch.load(Ordering::Acquire);
            let input = revision(context, epoch)?;
            if !self
                .0
                .coordinator
                .lock()
                .map_err(|_| assets::error("renderer_unavailable"))?
                .entries
                .get(&key)
                .is_some_and(|entry| entry.revision == input && entry.outcome != "preparing")
            {
                return Err(assets::error("stale_map_scene"));
            }
            validate_scope(&scene.renderer, area, &scene.atlas_digest, cancel)?;
            let snapshot = scene
                .renderer
                ._snapshot
                .as_ref()
                .ok_or_else(|| assets::error("consistent_snapshot_required"))?;
            let mut report = service::report_resources(
                context,
                &scene.inventory,
                &scene.resource_manifest,
                scene.missing_sources,
                scene.input_prerequisites,
                &WorldSource::Directory(snapshot.path.clone()),
                dimension,
                area,
                operation,
                cancel,
            )?;
            report.geometry_generation_id = Some(scene.renderer.generation.clone());
            if report.outcome == "ready" {
                report.outcome = "checked".into();
            }
            report.scope = "saved blocks in requested bounds; adopted resources and matching terrain artifacts; visual acceptance pending".into();
            let candidate = state.map_assets.store()?.begin()?;
            Ok((candidate, scene.resource_manifest.clone(), report))
        })())
    }
    pub fn has_server(&self, id: &str) -> bool {
        self.0
            .generations
            .lock()
            .map(|g| g.values().any(|s| s.renderer.server_id == id))
            .unwrap_or(false)
    }

    pub fn sweep(&self) {
        let mut retired = Vec::new();
        if let Ok(mut generations) = self.0.generations.lock() {
            let keys = generations
                .iter()
                .filter(|(_, s)| s.renderer.idle())
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for id in keys {
                if let Some(scene) = generations.remove(&id) {
                    retired.push(scene);
                }
            }
        }
        let retained = if let Ok(mut coordinator) = self.0.coordinator.lock() {
            coordinator.entries.retain(|_, entry| {
                entry.outcome == "preparing"
                    || entry.current.as_ref().is_some_and(|s| !s.renderer.idle())
            });
            coordinator
                .entries
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>()
        } else {
            std::collections::BTreeSet::new()
        };
        if let Ok(mut details) = self.0.details.lock() {
            details.retain(|key, _| retained.contains(key));
        }
        // Killing helpers/removing caches occurs after the coordination locks.
        drop(retired);
    }
    pub fn status(&self, context: &Context, dimension: &str, epoch: u64) -> RenderingStatus {
        let Ok(key) = key(context, dimension) else {
            return empty_status("failed");
        };
        let mut status = self
            .0
            .details
            .lock()
            .ok()
            .and_then(|d| d.get(&key).cloned())
            .unwrap_or_else(|| empty_status("unchecked"));
        let current = self
            .0
            .coordinator
            .lock()
            .ok()
            .and_then(|c| c.entries.get(&key).and_then(|e| e.current.clone()));
        if let Some(scene) = current {
            status.generation_id = Some(scene.renderer.generation.clone());
            status.resource_generation_id = scene.status.resource_generation_id.clone();
            status.snapshot_id = scene.status.snapshot_id.clone();
            status.snapshot_at_unix = scene.status.snapshot_at_unix;
            status.resources_at_unix = scene.status.resources_at_unix;
            status.stale = scene.binding != context.binding || scene.snapshot_epoch != epoch;
            if status.stale {
                status.note="The retained scene uses its recorded saved snapshot and resource time. Refresh terrain or complete resource preparation to replace it.".into();
            }
        }
        status
    }
    fn renderer_failed(&self, key: &str, generation: &str) {
        if let Ok(coordinator) = self.0.coordinator.lock()
            && coordinator
                .entries
                .get(key)
                .filter(|entry| entry.outcome != "preparing")
                .and_then(|e| e.current.as_ref())
                .is_some_and(|s| s.renderer.generation == generation)
            && let Ok(mut details) = self.0.details.lock()
            && let Some(status) = details.get_mut(key)
        {
            status.state = "failed".into();
            status.reason_code = Some("renderer_unavailable".into());
            status.retryable = true;
            status.note="The helper stopped serving this retained generation; the displayed scene is retained. Use Repair map assets to prepare a replacement.".into();
        }
    }
    fn current(&self, key: &str, generation: Option<&str>) -> Option<Arc<Scene>> {
        if let Some(generation) = generation {
            return self
                .0
                .generations
                .lock()
                .ok()?
                .get(generation)
                .filter(|s| s.key == key)
                .cloned();
        }
        self.0
            .coordinator
            .lock()
            .ok()?
            .entries
            .get(key)?
            .current
            .clone()
    }
    pub fn start(
        &self,
        state: WorldsRoutesState,
        context: Context,
        dimension: String,
        area: Option<Area>,
        force: bool,
    ) -> Result<String, TerrainError> {
        self.start_job(state, context, dimension, area, force, None, None)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn start_job(
        &self,
        state: WorldsRoutesState,
        context: Context,
        dimension: String,
        area: Option<Area>,
        force: bool,
        source: Option<(PathBuf, String)>,
        mutation: Option<service::ResourceMutation>,
    ) -> Result<String, TerrainError> {
        let before = self.report_for(&context, Some(&dimension));
        let area = area.or_else(|| before.as_ref().map(|r| r.area));
        let key = key(&context, &dimension).map_err(|_| ())?;
        let epoch = state.map_renderer.0.snapshot_epoch.load(Ordering::Acquire);
        let input = revision(&context, epoch).map_err(|_| ())?;
        let mut coordinator = self.0.coordinator.lock().map_err(|_| ())?;
        if let Some(entry) = coordinator.entries.get(&key)
            && ((!force && entry.revision == input)
                || (entry.revision == input && entry.outcome == "preparing"))
        {
            if source.is_some() || mutation.is_some() {
                return Err(TerrainError::new(
                    "map_operation_busy",
                    "Wait for the current map operation before importing resources.",
                ));
            }
            return Ok(entry.operation_id.clone().unwrap_or_default());
        }
        if coordinator.entries.len() >= 8 && !coordinator.entries.contains_key(&key) {
            return Err(TerrainError::new(
                "map_binding_limit",
                "Close unused map views before preparing another binding.",
            ));
        }
        if self.0.generations.lock().map_err(|_| ())?.len() >= 4 {
            return Err(TerrainError::new(
                "map_reader_limit",
                "Close unused map views before preparing another generation.",
            ));
        }
        let permit = state.map_assets.worker().ok_or_else(|| {
            TerrainError::new(
                "map_assets_worker_limit",
                "Two resource operations are already active; retry after they finish.",
            )
        })?;
        let operations = state.lifecycle.operations();
        let operation = operations
            .begin_lifecycle(
                "world-map-assets-prepare",
                Some(context.server.id.clone()),
                "Preparing exact saved-map resources.",
            )
            .map_err(|_| {
                TerrainError::new(
                    "map_operation_busy",
                    "Another server operation is active; reopen the map after it finishes.",
                )
            })?;
        let operation_text = operation.as_str().to_string();
        let ticket = coordinator
            .begin(&key, &input, &operation_text, force)
            .ok_or(())?;
        drop(coordinator);
        let mut details = self.status(&context, &dimension, epoch);
        details.state = "preparing".into();
        details.operation_id = Some(operation_text.clone());
        details.reason_code = None;
        details.retryable = false;
        self.0.details.lock().map_err(|_| ())?.insert(key, details);
        state.map_renderer.ensure_sweeper();
        let store = self.clone();
        tokio::spawn(async move {
            let failed_ticket = ticket.clone();
            let operation_for_work = operation.clone();
            let work_state = state.clone();
            let worker_store = store.clone();
            let result = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                let result = worker_store.prepare(
                    &work_state,
                    context,
                    &dimension,
                    area,
                    &ticket,
                    &operation_for_work,
                    epoch,
                    source.as_ref(),
                    mutation.as_ref(),
                    before,
                );
                if let Some((path, _)) = source {
                    let _ = std::fs::remove_file(path);
                }
                result
            })
            .await;
            match result {
                Ok(Ok(status)) => {
                    let _ = operations.succeed(
                        &operation,
                        "Saved map candidate adopted; visual acceptance remains pending.",
                        BTreeMap::from([
                            ("result".into(), status.state),
                            (
                                "generationId".into(),
                                status.generation_id.unwrap_or_default(),
                            ),
                        ]),
                    );
                }
                Ok(Err(error)) => {
                    let code = error.to_string();
                    let safe = if code.bytes().all(|c| c.is_ascii_lowercase() || c == b'_') {
                        code.as_str()
                    } else {
                        "map_preparation_failed"
                    };
                    if safe == "cancelled" {
                        let _ = operations.cancel(
                            &operation,
                            "Map preparation cancelled; prior generation retained.",
                        );
                    } else {
                        let _ = operations.fail(
                            &operation,
                            safe,
                            "Map preparation failed; prior generation retained.".into(),
                        );
                    }
                }
                Err(_) => {
                    let same_job =
                        store.0.coordinator.lock().is_ok_and(|mut coordinator| {
                            coordinator.abort(&failed_ticket, "failed")
                        });
                    if same_job
                        && let Ok(mut details) = store.0.details.lock()
                        && let Some(status) = details.get_mut(&failed_ticket.key)
                    {
                        status.state = "failed".into();
                        status.retryable = true;
                        status.reason_code = Some("map_assets_worker_failed".into());
                    }
                    let _ = operations.fail(
                        &operation,
                        "map_assets_worker_failed",
                        "Map preparation worker failed; prior generation retained.".into(),
                    );
                }
            }
        });
        Ok(operation_text)
    }
    #[allow(clippy::too_many_arguments)]
    fn prepare(
        &self,
        state: &WorldsRoutesState,
        mut context: Context,
        dimension: &str,
        area: Option<Area>,
        ticket: &Ticket,
        operation: &OperationId,
        epoch: u64,
        source: Option<&(PathBuf, String)>,
        mutation: Option<&service::ResourceMutation>,
        before: Option<Report>,
    ) -> std::io::Result<RenderingStatus> {
        let operations = state.lifecycle.operations();
        let operation_cancel = operations.cancellation_check(operation);
        let server_id = context.server.id.clone();
        let server_root = context.server.server_dir.clone();
        let cancellation_ticket = ticket.clone();
        let cancel = || {
            cancellation_ticket.cancelled()
                || !state
                    .lifecycle
                    .active_config_server()
                    .is_some_and(|server| {
                        server.id == server_id && server.server_dir == server_root
                    })
                || operation_cancel()
                || state.map_renderer.0.snapshot_epoch.load(Ordering::Acquire) != epoch
        };
        let progress = |n, total, line: &str| {
            let _ = operations.progress(operation, n, total, line);
        };
        let mut import_ticket = ticket.clone();
        let import_result = (|| {
            if let Some((path, sha)) = source {
                progress(0, 8, "Validating the matching client resource bundle.");
                service::import_bundle(&context, &state.map_assets.store()?, path, sha, &cancel)?;
            }
            if let Some(mutation) = mutation {
                service::apply_resource_mutation(
                    &context,
                    &state.map_assets.store()?,
                    mutation,
                    &cancel,
                )?;
            }
            if source.is_some() || mutation.is_some() {
                let input = revision(&context, epoch)?;
                if !self
                    .0
                    .coordinator
                    .lock()
                    .map_err(|_| assets::error("renderer_unavailable"))?
                    .rebase(&mut import_ticket, input)
                {
                    return Err(assets::error("cancelled"));
                }
            }
            Ok::<_, std::io::Error>(())
        })();
        let ticket = &import_ticket;
        let result = (|| {
            import_result?;
            assets::poll(&cancel)?;
            progress(0, 8, "Capturing a consistent private saved snapshot.");
            let world = match &context.world {
                WorldSource::Directory(path) => path.clone(),
                _ => return Err(assets::error("active_world_required")),
            };
            let saved = state
                .map_renderer
                .0
                .snapshot
                .lock()
                .map_err(|_| assets::error("renderer_unavailable"))?
                .as_ref()
                .filter(|s| s.server_id == context.server.id && s.source_world == world)
                .cloned()
                .or_else(|| {
                    self.current(&ticket.key, None)
                        .filter(|s| s.snapshot_epoch == epoch)
                        .and_then(|s| s.renderer._snapshot.clone())
                        .filter(|s| s.server_id == context.server.id && s.source_world == world)
                });
            let snapshot = if let Some(saved) = saved {
                saved
            } else if state.lifecycle.status_snapshot().running {
                let captured = crate::backup_operations::snapshot_java_world(
                    state.lifecycle.clone(),
                    Path::new(&context.server.server_dir),
                    cancel,
                )
                .map_err(|_| assets::error("consistent_snapshot_failed"))?;
                Arc::new(SavedSnapshot {
                    server_id: context.server.id.clone(),
                    source_world: world.clone(),
                    path: captured.path,
                    captured_at: unix(),
                })
            } else {
                stopped_snapshot(&context.server.id, &world, &cancel)?
            };
            // A flush may update level.dat. Resource and slot inputs must remain
            // the same; bind the operation to the post-flush live metadata.
            let server = state
                .lifecycle
                .active_config_server()
                .ok_or_else(|| assets::error("binding_changed"))?;
            let current = service::context(
                &server,
                &context.binding.agent_host_id,
                &context.binding.slot_id,
            )?;
            if revision(&current, epoch)? != ticket.revision {
                return Err(assets::error("binding_changed"));
            }
            context = current;
            progress(1, 8, "Preparing version-matched vanilla assets.");
            let selected = crate::routes::versions::minecraft_version_from_selection(
                Some(context.server.java_flavor),
                context.server.minecraft_version.clone(),
            );
            let vanilla = std::fs::canonicalize(
                dependencies::assets(&snapshot.path, selected.as_deref())
                    .map_err(|_| assets::error("client_assets_unavailable"))?,
            )?;
            let game = WorldSource::Directory(snapshot.path.clone())
                .recorded_game_version()?
                .or(selected)
                .ok_or_else(|| assets::error("minecraft_version_required"))?;
            let content = state.map_assets.store()?;
            let secrets = crate::auth::production_secret_store()
                .map_err(|_| assets::error("provider_credentials_unavailable"))?;
            let transport = msc_infrastructure::addon_provider::HttpTransport::map_resources();
            let prepared = service::prepare_resources(
                &context,
                &content,
                &vanilla,
                &game,
                &transport,
                secrets.as_ref(),
                &msc_infrastructure::config_repository::default_app_data_dir()
                    .join("map-assets-downloads"),
                false,
                &cancel,
                &|n, _, line| progress(n + 2, 8, line),
            )?;
            progress(
                5,
                8,
                "Adapting private namespaces and saved block palettes.",
            );
            let cache = std::env::temp_dir().join(format!("msc-map-renderer-{}", Uuid::new_v4()));
            std::fs::create_dir(&cache)?;
            let mut cleanup = Scratch(Some(cache.clone()));
            let terrain = java_terrain_compat::prepare_adapted(
                &snapshot.path,
                dimension,
                &cache,
                &prepared.stack.inventory,
                &cancel,
            )?;
            let binary = std::fs::canonicalize(
                dependencies::binary().map_err(|_| assets::error("renderer_missing"))?,
            )?;
            let helper_digest = assets::file_hash(&binary, 128 * 1024 * 1024, &cancel)?;
            let generation = assets::hash_json(&(
                1,
                dimension,
                &context.binding,
                epoch,
                &terrain.snapshot_id,
                &prepared.manifest.generation_id,
                &helper_digest,
            ))?;
            assets::poll(&cancel)?;
            progress(
                6,
                8,
                "Validating a saved tile and its matching texture atlas.",
            );
            let mut renderer = Renderer::launch_at(
                &context.server.id,
                &snapshot.path,
                dimension,
                &terrain.path,
                &terrain.renderer_dimension,
                &cache.join("assets/minecraft"),
                cache.clone(),
                binary,
                &cancel,
            )
            .map_err(|_| assets::error("renderer_candidate_failed"))?;
            cleanup.0 = None;
            renderer.generation = generation.clone();
            renderer._snapshot = Some(snapshot.clone());
            let (status, raw) = renderer
                .read("manifest.json")
                .map_err(|_| assets::error("renderer_manifest_failed"))?;
            if status != 200 {
                return Err(assets::error("renderer_manifest_failed"));
            }
            let manifest: serde_json::Value = serde_json::from_slice(&raw)
                .map_err(|_| assets::error("invalid_renderer_manifest"))?;
            let width = manifest["tileChunks"]
                .as_i64()
                .filter(|n| (1..=32).contains(n))
                .ok_or_else(|| assets::error("invalid_renderer_manifest"))?
                as i32;
            let tiles = manifest["tiles"]
                .as_array()
                .ok_or_else(|| assets::error("invalid_renderer_manifest"))?;
            if tiles.is_empty() {
                return Err(assets::error("missing_saved_chunks"));
            }
            if tiles.len() > 100_000 {
                return Err(assets::error("render_tile_limit"));
            }
            let area = area.unwrap_or_else(|| default_area(&terrain));
            let requested = tiles
                .iter()
                .filter(|tile| {
                    tile["path"]
                        .as_str()
                        .is_some_and(|p| p.ends_with(".vtile") && super::artifact_type(p).is_some())
                })
                .filter(|tile| {
                    let x = tile["x"].as_i64();
                    let z = tile["z"].as_i64();
                    x.is_some_and(|x| {
                        i64::from(area.min[0].div_euclid(16 * width)) <= x
                            && x <= i64::from(area.max[0].div_euclid(16 * width))
                    }) && z.is_some_and(|z| {
                        i64::from(area.min[2].div_euclid(16 * width)) <= z
                            && z <= i64::from(area.max[2].div_euclid(16 * width))
                    })
                })
                .collect::<Vec<_>>();
            let tile = requested
                .first()
                .ok_or_else(|| assets::error("missing_saved_chunks"))?;
            let tile_path = tile["path"]
                .as_str()
                .ok_or_else(|| assets::error("invalid_renderer_manifest"))?;
            let (status, tile_bytes) = renderer
                .read(tile_path)
                .map_err(|_| assets::error("renderer_tile_failed"))?;
            if status != 200 || tile_bytes.is_empty() {
                return Err(assets::error("renderer_tile_failed"));
            }
            for tile in requested.iter().skip(1) {
                assets::poll(&cancel)?;
                let path = tile["path"]
                    .as_str()
                    .ok_or_else(|| assets::error("invalid_renderer_manifest"))?;
                let (status, bytes) = renderer
                    .read(path)
                    .map_err(|_| assets::error("renderer_tile_failed"))?;
                if status != 200 {
                    return Err(assets::error("renderer_tile_failed"));
                }
                validate_artifact(&bytes, false)?;
            }
            let (status, texture_bytes) = renderer
                .read("terrain.vtexarr")
                .map_err(|_| assets::error("renderer_atlas_failed"))?;
            if status != 200 || texture_bytes.is_empty() {
                return Err(assets::error("renderer_atlas_failed"));
            }
            validate_artifact(&tile_bytes, false)?;
            validate_artifact(&texture_bytes, true)?;
            let atlas = inflate_artifact(&texture_bytes)?;
            let atlas_digest = assets::hash(&atlas);
            let atlas_layers = u32::from_le_bytes(
                atlas[16..20]
                    .try_into()
                    .map_err(|_| assets::error("invalid_renderer_atlas"))?,
            );
            let mut report = service::prepared_report(
                &context,
                &prepared,
                &WorldSource::Directory(snapshot.path.clone()),
                dimension,
                area,
                operation.as_str(),
                &cancel,
            )?;
            report.geometry_generation_id = Some(generation.clone());
            let mut fingerprints = BTreeMap::new();
            for tile in tiles {
                let x = tile["x"]
                    .as_i64()
                    .and_then(|n| i32::try_from(n).ok())
                    .ok_or_else(|| assets::error("invalid_renderer_manifest"))?;
                let z = tile["z"]
                    .as_i64()
                    .and_then(|n| i32::try_from(n).ok())
                    .ok_or_else(|| assets::error("invalid_renderer_manifest"))?;
                if x.unsigned_abs() > 1_875_000 || z.unsigned_abs() > 1_875_000 {
                    return Err(assets::error("invalid_renderer_manifest"));
                }
                if let Some(path) = tile["path"].as_str() {
                    fingerprints.insert(
                        path.into(),
                        assets::hash_json(&(
                            terrain.tile_fingerprint(x, z, width)?,
                            &atlas_digest,
                            &helper_digest,
                            "msc-private-resource-adapter-1",
                        ))?,
                    );
                }
            }
            let prior = self.current(&ticket.key, None);
            let unchanged = prior.as_ref().map_or(0, |old| {
                fingerprints
                    .iter()
                    .filter(|(p, h)| old.tile_fingerprints.get(*p) == Some(*h))
                    .count() as u64
            });
            service::classify_repair(before.as_ref(), &mut report, true);
            let mut status = empty_status(&report.outcome);
            status.operation_id = Some(operation.as_str().into());
            status.generation_id = Some(generation.clone());
            status.resource_generation_id = Some(prepared.manifest.generation_id.clone());
            status.snapshot_id = Some(terrain.snapshot_id.clone());
            status.snapshot_at_unix = Some(snapshot.captured_at);
            status.resources_at_unix = Some(unix());
            status.affected_tiles = fingerprints.len() as u64 - unchanged;
            status.unchanged_tiles = unchanged;
            status.required_sources = prepared
                .missing
                .iter()
                .map(|m| RequiredClientSource {
                    identity: m.source.identity.clone(),
                    file: m.source.path.clone(),
                    provider: m.source.provider.clone(),
                    project_id: m.source.project_id.clone(),
                    release_id: m.source.release_id.clone(),
                    file_id: m.source.file_id.clone(),
                    hashes: m.source.hashes.clone(),
                    bytes: m.source.bytes,
                    code: m.code.clone(),
                })
                .collect();
            status.prerequisites = prepared.prerequisites.clone();
            status.note="Saved map candidate adopted after a tile/atlas check. Unresolved blocks are marked fallbacks; scoped diagnostics and visual acceptance are separate.".into();
            let scene = Arc::new(Scene {
                renderer: Arc::new(renderer),
                key: ticket.key.clone(),
                binding: context.binding.clone(),
                report: report.clone(),
                status: status.clone(),
                tile_fingerprints: fingerprints,
                snapshot_epoch: epoch,
                atlas_digest,
                atlas_layers,
                resource_manifest: prepared.manifest.clone(),
                missing_sources: !prepared.missing.is_empty(),
                input_prerequisites: !prepared.prerequisites.is_empty(),
                inventory: prepared.stack.inventory,
            });
            progress(
                7,
                8,
                "Rechecking server, slot, inputs and snapshot before adoption.",
            );
            state
                .lifecycle
                .with_expected_active_server(Some(&context.server.id), || {
                    let server = state
                        .lifecycle
                        .active_config_server()
                        .ok_or_else(|| assets::error("binding_changed"))?;
                    let current = service::context(
                        &server,
                        &context.binding.agent_host_id,
                        &context.binding.slot_id,
                    )?;
                    if current.binding != context.binding
                        || revision(&current, epoch)? != ticket.revision
                    {
                        return Err(assets::error("binding_changed"));
                    }
                    prepared.candidate.verify_inputs(&cancel)?;
                    assets::poll(&cancel)?;
                    content.publish_guarded(
                        &prepared.candidate,
                        &prepared.manifest,
                        &report,
                        &cancel,
                        |temp, path| {
                            let mut coordinator = self
                                .0
                                .coordinator
                                .lock()
                                .map_err(|_| assets::error("renderer_unavailable"))?;
                            assets::poll(&cancel)?;
                            if !coordinator.matches(ticket) {
                                return Err(assets::error("cancelled"));
                            }
                            let mut generations = self
                                .0
                                .generations
                                .lock()
                                .map_err(|_| assets::error("renderer_unavailable"))?;
                            let mut details = self
                                .0
                                .details
                                .lock()
                                .map_err(|_| assets::error("renderer_unavailable"))?;
                            if let Ok(mut last_use) = scene.renderer.last_use.lock() {
                                *last_use = Instant::now();
                            }
                            if !scene.renderer.alive() {
                                return Err(assets::error("renderer_candidate_failed"));
                            }
                            std::fs::rename(temp, path)?;
                            generations.insert(generation, scene.clone());
                            coordinator.finish(ticket, Some(scene), &status.state);
                            details.insert(ticket.key.clone(), status.clone());
                            Ok(())
                        },
                    )?;

                    Ok(())
                })
                .map_err(|_| assets::error("binding_changed"))??;
            progress(8, 8, "Saved map candidate adopted.");
            Ok(status)
        })();
        if let Err(error) = &result {
            let code = error.to_string();
            let cancelled = cancel() || code == "cancelled";
            if let Ok(mut coordinator) = self.0.coordinator.lock()
                && coordinator.abort(ticket, if cancelled { "cancelled" } else { "failed" })
                && let Ok(mut details) = self.0.details.lock()
            {
                let mut status = details
                    .get(&ticket.key)
                    .cloned()
                    .unwrap_or_else(|| empty_status("failed"));
                status.state = if cancelled { "cancelled" } else { "failed" }.into();
                status.reason_code = Some(
                    if code.bytes().all(|c| c.is_ascii_lowercase() || c == b'_') {
                        code
                    } else {
                        "map_preparation_failed".into()
                    },
                );
                status.retryable = !cancelled
                    && !status.reason_code.as_ref().is_some_and(|code| {
                        code.starts_with("matching_")
                            || code.ends_with("_required")
                            || code.starts_with("invalid_client_resource")
                            || code == "client_bundle_checksum"
                            || code == "compatible_previous_resources_unavailable"
                    });
                status.note="Prior generation retained; preparation will not repeat on tile requests. Retry explicitly after addressing this failure.".into();
                details.insert(ticket.key.clone(), status);
            }
        }

        if result.is_err() && cancel() {
            Err(assets::error("cancelled"))
        } else {
            result
        }
    }
}
struct Scratch(Option<PathBuf>);
impl Drop for Scratch {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}
fn stopped_snapshot(
    server: &str,
    world: &Path,
    cancel: &dyn Fn() -> bool,
) -> std::io::Result<Arc<SavedSnapshot>> {
    let root = std::env::temp_dir().join(format!("msc-map-snapshot-{}", Uuid::new_v4()));
    std::fs::create_dir(&root)?;
    let mut cleanup = Scratch(Some(root.clone()));
    let target = root.join("world");
    std::fs::create_dir(&target)?;
    let mut stack = vec![(world.to_path_buf(), target.clone(), 0)];
    let mut stamps = BTreeMap::new();
    let mut total = 0u64;
    let mut count = 0;
    while let Some((source, dest, depth)) = stack.pop() {
        if depth > 64 {
            return Err(assets::error("snapshot_depth_limit"));
        }
        stamps.insert(
            source.clone(),
            assets::stamp(&std::fs::symlink_metadata(&source)?),
        );
        assets::safe_path(&source)?;
        for entry in std::fs::read_dir(source)? {
            assets::poll(cancel)?;
            count += 1;
            if count > 100_000 {
                return Err(assets::error("snapshot_entry_limit"));
            }
            let entry = entry?;
            let meta = std::fs::symlink_metadata(entry.path())?;
            let target = dest.join(entry.file_name());
            if meta.is_dir() {
                std::fs::create_dir(&target)?;
                stack.push((entry.path(), target, depth + 1));
            } else if meta.is_file() {
                total += meta.len();
                if total > 2 * 1024 * 1024 * 1024 {
                    return Err(assets::error("snapshot_byte_limit"));
                }
                let before = assets::stamp(&meta);
                use std::io::Write;
                let mut input = assets::open(&entry.path())?;
                let mut output = std::fs::File::create(target)?;
                let mut copied = 0u64;
                let mut buffer = [0u8; 64 * 1024];
                loop {
                    assets::poll(cancel)?;
                    let n = input.read(&mut buffer)?;
                    if n == 0 {
                        break;
                    }
                    copied += n as u64;
                    if copied > meta.len() {
                        return Err(assets::error("snapshot_input_changed"));
                    }
                    output.write_all(&buffer[..n])?;
                }
                if copied != meta.len() {
                    return Err(assets::error("snapshot_input_changed"));
                }
                stamps.insert(entry.path(), before.clone());
                if assets::stamp(&std::fs::symlink_metadata(entry.path())?) != before {
                    return Err(assets::error("snapshot_input_changed"));
                }
            } else {
                return Err(assets::error("linked_snapshot_input"));
            }
        }
    }
    for (path, before) in stamps {
        assets::poll(cancel)?;
        if assets::stamp(&std::fs::symlink_metadata(path)?) != before {
            return Err(assets::error("snapshot_input_changed"));
        }
    }
    cleanup.0 = None;
    Ok(Arc::new(SavedSnapshot {
        server_id: server.into(),
        source_world: world.into(),
        path: target,
        captured_at: unix(),
    }))
}
fn default_area(terrain: &java_terrain_compat::AdaptedWorld) -> Area {
    let (x, z) = terrain
        .chunks
        .keys()
        .next()
        .and_then(|key| key.split_once(','))
        .and_then(|(x, z)| x.parse::<i32>().ok().zip(z.parse::<i32>().ok()))
        .unwrap_or((0, 0));
    Area {
        min: [x * 16, -64, z * 16],
        max: [x * 16 + 15, 319, z * 16 + 15],
    }
}
/// Reject corrupt/truncated/grossly oversized candidate output before publication.
fn inflate_artifact(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    const MAX_DECODED: u64 = 256 * 1024 * 1024;
    let mut raw = Vec::new();
    if bytes.starts_with(&[0x1f, 0x8b]) {
        flate2::read::GzDecoder::new(bytes)
            .take(MAX_DECODED + 1)
            .read_to_end(&mut raw)?;
    } else {
        raw.extend_from_slice(bytes);
    }
    if raw.len() as u64 > MAX_DECODED {
        return Err(assets::error("renderer_artifact_budget"));
    }
    Ok(raw)
}
fn validate_artifact(bytes: &[u8], texture: bool) -> std::io::Result<()> {
    let body = inflate_artifact(bytes)?;
    if body.len() < 20
        || if texture {
            !body.starts_with(b"VTA1")
        } else {
            !matches!(
                &body[..4],
                b"VTL1"
                    | b"VTL2"
                    | b"VTL3"
                    | b"VTL4"
                    | b"VTL5"
                    | b"VTL6"
                    | b"VTL7"
                    | b"VTL8"
                    | b"VTL9"
                    | b"VTLA"
            )
        }
    {
        return Err(assets::error("invalid_renderer_artifact"));
    }
    if texture {
        let n = |offset| {
            u32::from_le_bytes(body[offset..offset + 4].try_into().expect("bounded header"))
                as usize
        };
        let (version, w, h, layers) = (n(4), n(8), n(12), n(16));
        if !(1..=2).contains(&version)
            || w == 0
            || h == 0
            || w > 4096
            || h > 4096
            || layers == 0
            || layers > 65536
        {
            return Err(assets::error("invalid_renderer_atlas"));
        }
        let end = w
            .checked_mul(h)
            .and_then(|n| n.checked_mul(layers))
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_add(20))
            .filter(|n| *n <= body.len())
            .ok_or_else(|| assets::error("invalid_renderer_atlas"))?;
        if version == 1 {
            if end != body.len() {
                return Err(assets::error("invalid_renderer_atlas"));
            }
        } else {
            let count = u32::from_le_bytes(
                body.get(end..end + 4)
                    .ok_or_else(|| assets::error("invalid_renderer_atlas"))?
                    .try_into()
                    .expect("bounded animation header"),
            ) as usize;
            if count > 65536 || end + 4 + count * 12 != body.len() {
                return Err(assets::error("invalid_renderer_atlas"));
            }
            for entry in body[end + 4..].chunks_exact(12) {
                let base =
                    u32::from_le_bytes(entry[..4].try_into().expect("bounded animation")) as usize;
                let frames =
                    u16::from_le_bytes(entry[4..6].try_into().expect("bounded animation")) as usize;
                if frames == 0 || base + frames > layers {
                    return Err(assets::error("invalid_renderer_atlas"));
                }
            }
        }
    }
    Ok(())
}

pub(super) async fn artifact(
    state: WorldsRoutesState,
    server: ConfigServer,
    _world: PathBuf,
    query: ArtifactQuery,
    content_type: &'static str,
) -> Response {
    let task = state.clone();
    let task_server = server.clone();
    let context_result =
        tokio::task::spawn_blocking(move || active_context(&task, &task_server)).await;
    let context = match context_result {
        Ok(Ok(context)) => context,
        _ => {
            return error_response(
                StatusCode::CONFLICT,
                "map_binding_unavailable",
                "The active saved-world binding could not be read safely.",
            );
        }
    };
    let key = match key(&context, &query.dimension) {
        Ok(key) => key,
        Err(_) => {
            return error_response(
                StatusCode::CONFLICT,
                "map_binding_unavailable",
                "The saved-world binding is unavailable.",
            );
        }
    };
    let store = state.map_renderer.0.prepared.clone();
    if query.path == "manifest.json"
        && query.generation.is_none()
        && let Err(error) = store.start(
            state.clone(),
            context.clone(),
            query.dimension.clone(),
            None,
            false,
        )
        && store.current(&key, None).is_none()
    {
        return error_response(StatusCode::CONFLICT, error.code, &error.message);
    }
    let epoch = state.map_renderer.0.snapshot_epoch.load(Ordering::Acquire);
    let status = store.status(&context, &query.dimension, epoch);
    let Some(scene) = store.current(&key, query.generation.as_deref()) else {
        return (StatusCode::CONFLICT,axum::Json(serde_json::json!({"code":if query.generation.is_some(){"map_generation_retired"}else{"map_preparing"},"message":"Map preparation has not published a usable candidate yet.","details":{"rendering":status,"binding":context.binding}}))).into_response();
    };
    let artifact = query.path.clone();
    let generation = scene.renderer.generation.clone();
    let request_generation = generation.clone();
    match tokio::task::spawn_blocking(move || {
        let (code, mut bytes) = scene.renderer.read(&artifact)?;
        if artifact == "manifest.json" && code == 200 {
            let mut manifest: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|_| TerrainError::from(()))?;
            manifest["mscGenerationId"] = generation.into();
            manifest["mscAtlasDigest"] = scene.atlas_digest.clone().into();
            manifest["mscAtlasLayers"] = scene.atlas_layers.into();
            manifest["mscBinding"] =
                serde_json::to_value(&scene.binding).map_err(|_| TerrainError::from(()))?;
            manifest["mscRendering"] =
                serde_json::to_value(&status).map_err(|_| TerrainError::from(()))?;
            if let Some(d) = manifest.get_mut("dimension") {
                d["id"] = scene.report.dimension.clone().into();
                if !scene.report.dimension.starts_with("minecraft:") {
                    d["kind"] = "custom".into();
                }
            }
            if let Some(tiles) = manifest["tiles"].as_array_mut() {
                for tile in tiles {
                    if let Some(path) = tile["path"].as_str().map(str::to_owned)
                        && let Some(revision) = scene.tile_fingerprints.get(&path)
                    {
                        tile["revision"] = revision.clone().into();
                    }
                }
            }
            bytes = serde_json::to_vec(&manifest).map_err(|_| TerrainError::from(()))?;
        }
        Ok::<_, TerrainError>((code, bytes))
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
            "The saved tile is not available; other tiles remain visible.",
        ),
        Ok(Err(error)) => {
            store.renderer_failed(&key, &request_generation);
            let status = store.status(&context, &query.dimension, epoch);
            (StatusCode::BAD_GATEWAY,axum::Json(serde_json::json!({"code":error.code,"message":error.message,"details":{"rendering":status,"binding":context.binding}}))).into_response()
        }
        _ => error_response(
            StatusCode::BAD_GATEWAY,
            "renderer_unavailable",
            "The candidate artifact could not be read; the scene is retained.",
        ),
    }
}

fn state_report(context: &Context) -> Option<Report> {
    let root = msc_infrastructure::config_repository::default_app_data_dir().join("map-assets");
    let key = assets::hash_json(&(
        &context.binding.agent_host_id,
        &context.binding.server_id,
        &context.binding.slot_id,
    ))
    .ok()?;
    let pointer: assets::store::Pointer = serde_json::from_slice(
        &assets::read(
            &root.join("bindings").join(format!("{key}.json")),
            assets::MAX_JSON,
        )
        .ok()?,
    )
    .ok()?;
    if pointer.binding != context.binding
        || pointer.current.len() != 64
        || !pointer.current.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(
        &assets::read(
            &root
                .join("generations")
                .join(format!("{}.json", pointer.current)),
            32 * 1024 * 1024,
        )
        .ok()?,
    )
    .ok()?;
    let manifest: msc_domain::map_assets::ResourceManifest =
        serde_json::from_value(value["manifest"].clone()).ok()?;
    let report: Report = serde_json::from_value(value["report"].clone()).ok()?;
    if assets::hash_json(&(&manifest, &report)).ok()? != pointer.current
        || report.binding != context.binding
        || manifest.generation_id != report.resource_generation_id
    {
        return None;
    }
    Some(report)
}

fn validate_scope(
    renderer: &Renderer,
    area: Area,
    atlas_digest: &str,
    cancel: &dyn Fn() -> bool,
) -> std::io::Result<()> {
    let (status, raw) = renderer
        .read("manifest.json")
        .map_err(|_| assets::error("renderer_manifest_failed"))?;
    if status != 200 {
        return Err(assets::error("renderer_manifest_failed"));
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&raw).map_err(|_| assets::error("invalid_renderer_manifest"))?;
    let width = manifest["tileChunks"]
        .as_i64()
        .filter(|n| (1..=32).contains(n))
        .ok_or_else(|| assets::error("invalid_renderer_manifest"))?;
    let tiles = manifest["tiles"]
        .as_array()
        .ok_or_else(|| assets::error("invalid_renderer_manifest"))?;
    if tiles.len() > 100_000 {
        return Err(assets::error("render_tile_limit"));
    }
    for tile in tiles {
        let x = tile["x"]
            .as_i64()
            .ok_or_else(|| assets::error("invalid_renderer_manifest"))?;
        let z = tile["z"]
            .as_i64()
            .ok_or_else(|| assets::error("invalid_renderer_manifest"))?;
        if x < i64::from(area.min[0]).div_euclid(16 * width)
            || x > i64::from(area.max[0]).div_euclid(16 * width)
            || z < i64::from(area.min[2]).div_euclid(16 * width)
            || z > i64::from(area.max[2]).div_euclid(16 * width)
        {
            continue;
        }
        let path = tile["path"]
            .as_str()
            .filter(|p| p.ends_with(".vtile") && super::artifact_type(p).is_some())
            .ok_or_else(|| assets::error("invalid_renderer_manifest"))?;
        assets::poll(cancel)?;
        let (status, bytes) = renderer
            .read(path)
            .map_err(|_| assets::error("renderer_tile_failed"))?;
        if status != 200 {
            return Err(assets::error("renderer_tile_failed"));
        }
        validate_artifact(&bytes, false)?;
    }
    let (status, bytes) = renderer
        .read("terrain.vtexarr")
        .map_err(|_| assets::error("renderer_atlas_failed"))?;
    if status != 200 {
        return Err(assets::error("renderer_atlas_failed"));
    }
    validate_artifact(&bytes, true)?;
    if assets::hash(&inflate_artifact(&bytes)?) != atlas_digest {
        return Err(assets::error("renderer_atlas_changed"));
    }
    Ok(())
}
