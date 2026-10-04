//! Operation-independent inspection service. It neither downloads nor adopts renderer assets.
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::map_assets::{
    self, Area, Binding, Classification as C, Diagnostic, Report, ResourceManifest,
};
use msc_domain::world::WorldSlot;
use msc_infrastructure::map_assets::{
    self as io_assets,
    inventory::Inventory,
    resolver::Resolver,
    saved_terrain::WorldSource,
    store::{Candidate, Store},
    *,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::Path;
type StateKey = (String, BTreeMap<String, String>, bool);

pub struct Context {
    pub binding: Binding,
    pub server: ConfigServer,
    pub world: WorldSource,
    pub revision_inputs: BTreeMap<String, String>,
}

pub fn context(server: &ConfigServer, host: &str, slot_id: &str) -> io::Result<Context> {
    if uuid::Uuid::parse_str(slot_id).is_err() {
        return Err(error("invalid_slot_id"));
    }
    let root = Path::new(&server.server_dir);
    safe_path(root)?;
    let slot_path = root.join("world_slots").join(slot_id).join("slot.json");
    let raw = read(&slot_path, MAX_JSON)?;
    let value: Value = serde_json::from_slice(&raw).map_err(|_| error("invalid_slot_metadata"))?;
    let slot = WorldSlot::decode(&value).map_err(|_| error("invalid_slot_metadata"))?;
    if slot.id != slot_id {
        return Err(error("slot_identity_mismatch"));
    }
    let marker = root.join("world_slots/active_slot_id.txt");
    let active = if marker.exists() {
        String::from_utf8(read(&marker, 4096)?)
            .map_err(|_| error("invalid_active_slot"))?
            .trim()
            .to_string()
    } else {
        let mut slots = Vec::new();
        for path in io_assets::inventory::source_paths(&root.join("world_slots"))? {
            if !path.is_dir() {
                continue;
            }
            let metadata = path.join("slot.json");
            if !metadata.exists() {
                continue;
            }
            if let Ok(value) = serde_json::from_slice::<Value>(&read(&metadata, MAX_JSON)?)
                && let Ok(slot) = WorldSlot::decode(&value)
            {
                slots.push(slot);
            }
        }
        map_assets_active(&slots).unwrap_or_default()
    };
    let world = if active == slot_id {
        let properties = read(&root.join("server.properties"), MAX_JSON)?;
        let text = String::from_utf8(properties).map_err(|_| error("invalid_server_properties"))?;
        let name = text
            .lines()
            .filter_map(|line| line.trim().split_once('='))
            .find(|(key, _)| key.trim() == "level-name")
            .map(|(_, name)| name.trim().to_string())
            .unwrap_or_else(|| "world".into());
        if !safe_member(&name) || name.contains('/') {
            return Err(error("unsafe_world_folder"));
        }
        let path = root.join(name);
        safe_path(&path)?;
        WorldSource::Directory(path)
    } else {
        let path = root.join("world_slots").join(slot_id).join("world.zip");
        safe_path(&path)?;
        WorldSource::Archive(path)
    };
    let source = match &world {
        WorldSource::Directory(path) => path.join("level.dat"),
        WorldSource::Archive(path) => path.clone(),
    };
    safe_path(&source)?;
    let mut inputs = BTreeMap::from([
        ("slotMetadata".into(), hash(&raw)),
        (
            "sourceIdentity".into(),
            hash(stamp(&std::fs::symlink_metadata(&source)?).as_bytes()),
        ),
    ]);
    if marker.exists() {
        inputs.insert("activeSlot".into(), hash(&read(&marker, 4096)?));
    }
    let incarnation = hash_json(&(slot.id, slot.created_at, &inputs))?;
    inputs.insert("mods".into(), tree_revision(&root.join("mods"))?);
    inputs.insert(
        "approvedPacks".into(),
        tree_revision(&root.join("resource-packs"))?,
    );
    inputs.insert(
        "configuration".into(),
        hash_json(&store::fingerprint_configs(&root.join("config"), &|| {
            false
        })?)?,
    );
    for name in [
        "modrinth.index.json",
        "manifest.json",
        "minecraftinstance.json",
    ] {
        let path = root.join(name);
        if path.exists() {
            inputs.insert(name.into(), hash(&read(&path, MAX_JSON)?));
        }
    }
    if let WorldSource::Directory(world) = &world {
        inputs.insert(
            "worldServerConfiguration".into(),
            hash_json(&store::fingerprint_configs(
                &world.join("serverconfig"),
                &|| false,
            )?)?,
        );
    }
    let revision = hash_json(&(
        host,
        &server.id,
        slot_id,
        &incarnation,
        &inputs,
        server.minecraft_version.clone(),
        server.java_flavor.raw_value(),
        server.loader_version.clone(),
    ))?;
    Ok(Context {
        binding: Binding {
            agent_host_id: host.into(),
            server_id: server.id.clone(),
            slot_id: slot_id.into(),
            world_incarnation: incarnation,
            revision,
        },
        server: server.clone(),
        world,
        revision_inputs: inputs,
    })
}
fn tree_revision(root: &Path) -> io::Result<String> {
    let mut entries = BTreeMap::new();
    for path in inventory::source_paths(root)? {
        safe_path(&path)?;
        let metadata = std::fs::symlink_metadata(&path)?;
        if !metadata.is_file() {
            return Err(error("non_regular_input"));
        }
        entries.insert(
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            stamp(&metadata),
        );
    }
    hash_json(&entries)
}

fn map_assets_active(slots: &[WorldSlot]) -> Option<String> {
    msc_domain::world::resolve_active_slot_id(slots, None)
}

#[allow(clippy::too_many_arguments)]
pub fn inspect(
    context: &Context,
    store: &Store,
    vanilla: &Path,
    game_version: Option<&str>,
    dimension: &str,
    area: Area,
    operation_id: &str,
    cancel: &dyn Fn() -> bool,
    progress: &dyn Fn(u64, u64, &str),
) -> io::Result<(Candidate, ResourceManifest, Report)> {
    area.validate().map_err(error)?;
    poll(cancel)?;
    let root = Path::new(&context.server.server_dir);
    let mut candidate = store.begin()?;
    let mut inventory = Inventory::default();
    progress(0, 4, "Inspecting exact installed resource inputs.");
    let mod_paths = inventory::source_paths(&root.join("mods"))?;
    let mut source_stamps = BTreeMap::new();
    for path in mod_paths {
        if path.is_file() {
            source_stamps.insert(path.clone(), stamp(&std::fs::symlink_metadata(&path)?));
        }
    }
    inventory.scan_mods(&root.join("mods"), &mut candidate, cancel)?;
    // These files are already admitted by MSC's resource-pack upload store. Their
    // inventory presence does not imply that a matching client has selected them.
    for path in inventory::source_paths(&root.join("resource-packs"))? {
        if path.extension().is_some_and(|s| s == "zip") {
            inventory.scan_archive(&path, "approved_server_pack", &mut candidate, cancel)?;
        }
    }
    inventory.scan_assets(vanilla, cancel)?;
    let mut receipts = BTreeMap::new();
    for name in [
        "modrinth.index.json",
        "manifest.json",
        "minecraftinstance.json",
    ] {
        let path = root.join(name);
        if path.exists() {
            receipts.insert(name.into(), hash(&read(&path, MAX_JSON)?));
        }
    }
    let config = store::fingerprint_configs(&root.join("config"), cancel)?;
    let server_config = match &context.world {
        WorldSource::Directory(path) => {
            store::fingerprint_configs(&path.join("serverconfig"), cancel)?
        }
        _ => BTreeMap::new(),
    };
    let config_fingerprint = hash_json(&(&config, &server_config))?;
    progress(1, 4, "Inspecting saved chunks in the requested area.");
    let world = match &context.world {
        WorldSource::Directory(path) => WorldSource::Directory(path.clone()),
        WorldSource::Archive(path) => WorldSource::Archive(candidate.copy(path, cancel)?.0),
    };
    let snapshot_minecraft_version = world.recorded_game_version()?;
    let terrain = world.inspect(dimension, area, cancel)?;
    progress(2, 4, "Resolving namespaced model and texture dependencies.");
    let mut grouped: BTreeMap<StateKey, Vec<[i32; 3]>> = BTreeMap::new();
    for block in &terrain.blocks {
        grouped
            .entry((block.id.clone(), block.state.clone(), block.entity))
            .or_default()
            .push(block.position);
    }
    let distinct_states = grouped
        .keys()
        .map(|(id, state, _)| (id, state))
        .collect::<BTreeSet<_>>()
        .len() as u64;
    let resolver = Resolver {
        inventory: &inventory,
    };
    let mut diagnostics = Vec::new();
    let mut counts = BTreeMap::new();
    let mut omitted_issues = 0;
    let mut omitted_samples = 0;
    for ((id, state, entity), positions) in grouped {
        poll(cancel)?;
        let mut seen = BTreeSet::new();
        let mut classified = BTreeSet::new();
        for finding in resolver.inspect(&id, &state, entity) {
            if !seen.insert((finding.classification, finding.detail.clone())) {
                continue;
            }
            if classified.insert(finding.classification) {
                *counts.entry(finding.classification).or_insert(0) += positions.len() as u64;
            }
            if diagnostics.len() < 1000 {
                omitted_samples += positions.len().saturating_sub(5) as u64;
                diagnostics.push(Diagnostic {
                    classification: finding.classification,
                    original_id: id.clone(),
                    state: state.clone(),
                    detail: finding.detail,
                    source_ids: finding.source_ids,
                    block_occurrences: positions.len() as u64,
                    samples: positions.iter().take(5).copied().collect(),
                });
            } else {
                omitted_issues += 1;
                omitted_samples += positions.len() as u64;
            }
        }
    }
    if !terrain.missing.is_empty() {
        counts.insert(C::MissingSavedChunk, terrain.missing.len() as u64);
        if diagnostics.len() < 1000 {
            diagnostics.push(Diagnostic {classification:C::MissingSavedChunk,original_id:"saved_chunk".into(),state:BTreeMap::new(),detail:"No fully saved chunk exists at these chunk origins; counts are missing chunks, not air blocks.".into(),source_ids:vec![],block_occurrences:0,samples:terrain.missing.iter().take(5).copied().collect()});
        } else {
            omitted_issues += 1;
        }
        omitted_samples += terrain.missing.len().saturating_sub(5) as u64;
    }
    if snapshot_minecraft_version
        .as_deref()
        .zip(game_version)
        .is_some_and(|(snapshot, resources)| snapshot != resources)
    {
        counts.insert(C::MissingContext, terrain.blocks.len() as u64);
        if diagnostics.len() < 1000 {
            diagnostics.push(Diagnostic { classification: C::MissingContext, original_id: "saved_snapshot".into(), state: BTreeMap::new(), detail: "The saved world's recorded Minecraft version differs from the installed resource selection. Supply resources matching this snapshot before adopting geometry.".into(), source_ids: vec![], block_occurrences: terrain.blocks.len() as u64, samples: vec![] });
        } else {
            omitted_issues += 1;
        }
    }
    candidate.input_stamps.extend(inventory.file_stamps.clone());
    candidate.verify_inputs(cancel)?;
    let selection_revision = hash_json(&(&inventory.sources, &receipts, &config_fingerprint))?;
    let mut manifest = ResourceManifest {
        schema_version: map_assets::SCHEMA_VERSION,
        generation_id: String::new(),
        minecraft_version: game_version
            .filter(|v| !v.is_empty() && !v.eq_ignore_ascii_case("latest"))
            .map(str::to_string),
        loader: context.server.java_flavor.raw_value().into(),
        loader_version: context.server.loader_version.clone(),
        resolver_version: map_assets::RESOLVER_VERSION.into(),
        renderer_version: map_assets::RENDERER_VERSION.into(),
        capture_formats: vec![],
        sources: inventory.sources,
        selected_pack_order: None,
        selection_known: false,
        selection_revision,
        config_fingerprint,
        manifest_receipts: receipts,
        archive_entries: inventory.entries,
        decompressed_bytes: inventory.decompressed,
        resource_documents: inventory.documents,
        resource_conflicts: inventory.conflicts,
    };
    // Sort evidence for identity only. It is deliberately not a guessed resource priority order.
    manifest.sources.sort_by(|a, b| a.id.cmp(&b.id));
    manifest.generation_id = hash_json(&manifest)?;
    let mut report=Report {schema_version:map_assets::SCHEMA_VERSION,binding:context.binding.clone(),snapshot_id:terrain.snapshot_id,snapshot_minecraft_version,resource_generation_id:manifest.generation_id.clone(),geometry_generation_id:None,dimension:dimension.into(),area,operation_id:operation_id.into(),outcome:"checked".into(),visual_acceptance:"pending".into(),scope:"saved blocks in requested bounds only; inventory resolution, not adopted renderer output; client selection unknown".into(),inspected_blocks:terrain.blocks.len()as u64,inspected_chunks:terrain.chunks,distinct_states,visible_faces:None,counts,diagnostics,omitted_issues,omitted_samples,sources:manifest.sources.clone()};
    if report
        .counts
        .keys()
        .any(|class| !matches!(class, C::ModelResolved | C::IntentionalEmpty))
    {
        report.outcome = "needs_input".into();
    }
    progress(
        3,
        4,
        "Rechecking input and world binding before publication.",
    );
    if store::fingerprint_configs(&root.join("config"), cancel)? != config {
        return Err(error("input_changed"));
    }
    if let WorldSource::Directory(path) = &context.world
        && store::fingerprint_configs(&path.join("serverconfig"), cancel)? != server_config
    {
        return Err(error("input_changed"));
    }
    let after = inventory::source_paths(&root.join("mods"))?;
    let mut after_stamps = BTreeMap::new();
    for path in after {
        if path.is_file() {
            after_stamps.insert(path.clone(), stamp(&std::fs::symlink_metadata(&path)?));
        }
    }
    if after_stamps != source_stamps {
        return Err(error("input_changed"));
    }
    if self::context(
        &context.server,
        &context.binding.agent_host_id,
        &context.binding.slot_id,
    )?
    .binding
        != context.binding
    {
        return Err(error("binding_changed"));
    }
    poll(cancel)?;
    Ok((candidate, manifest, report))
}
