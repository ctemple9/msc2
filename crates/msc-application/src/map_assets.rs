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
        ".msc-map-source/current",
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
        ".msc-map-source/current",
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

/// A candidate owns its scratch and cannot switch the production renderer.
pub struct PreparedResources {
    pub candidate: Candidate,
    pub stack: io_assets::compose::Stack,
    pub manifest: ResourceManifest,
    pub missing: Vec<io_assets::acquire::MissingSource>,
    pub prerequisites: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_resources(
    context: &Context,
    store: &Store,
    vanilla: &Path,
    game: &str,
    transport: &dyn msc_infrastructure::addon_provider::AddonTransport,
    secrets: &dyn msc_infrastructure::secret_store::SecretStore,
    download_cache: &Path,
    offline: bool,
    cancel: &dyn Fn() -> bool,
    progress: &dyn Fn(u64, u64, &str),
) -> io::Result<PreparedResources> {
    let mut candidate = store.begin()?;
    let mut stack = io_assets::compose::Stack::default();
    let mut layer_index = 0;
    let scratch = candidate.directory().to_path_buf();
    let mut layer = || -> io::Result<Inventory> {
        let mut inventory = Inventory::default();
        inventory.capture_into(scratch.join(format!("layer-{layer_index}")))?;
        layer_index += 1;
        Ok(inventory)
    };
    progress(0, 4, "Reading version-matched vanilla resources.");
    let mut base = layer()?;
    base.scan_assets(vanilla, cancel)?;
    let format_path = vanilla
        .parent()
        .and_then(Path::parent)
        .map(|p| p.join("pack.mcmeta"));
    let pack_format = format_path
        .filter(|p| p.exists())
        .and_then(|p| read(&p, MAX_JSON).ok())
        .and_then(|raw| serde_json::from_slice::<Value>(&raw).ok())
        .and_then(|v| v["pack"]["pack_format"].as_u64())
        .and_then(|n| u32::try_from(n).ok());
    stack.push(base, true, pack_format)?;
    let root = Path::new(&context.server.server_dir);
    let receipt = root.join(".msc-map-source/current");
    let source = if receipt.exists() {
        let id = String::from_utf8(read(&receipt, 128)?)
            .map_err(|_| error("invalid_map_source_receipt"))?;
        if id.len() != 64 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(error("invalid_map_source_receipt"));
        }
        root.join(".msc-map-source").join(id)
    } else {
        root.to_path_buf()
    };
    let mut prerequisites = Vec::new();
    let has_source =
        source.join("modrinth.index.json").exists() || source.join("manifest.json").exists();
    if context.server.modpack_identity.is_some() && !has_source {
        prerequisites.push("exact_client_manifest_required".into());
    }
    progress(
        1,
        4,
        "Collecting exact client files; verified downloads are reusable.",
    );
    let requests = io_assets::acquire::requests(
        &source,
        transport,
        secrets,
        game,
        context.server.java_flavor.raw_value(),
        context.server.loader_version.as_deref(),
        cancel,
    )?;
    let mut missing = Vec::new();
    let mut client_mods = layer()?;
    let mut manifest_paths = BTreeSet::new();
    for request in &requests {
        poll(cancel)?;
        manifest_paths.insert(root.join(&request.path));
        match io_assets::acquire::acquire(request, root, download_cache, transport, offline, cancel)
        {
            Ok(path) if request.path.starts_with("mods/") => {
                let start = client_mods.sources.len();
                client_mods.scan_archive(&path, "exact_client_mod", &mut candidate, cancel)?;
                if let Some(source) = client_mods.sources.get_mut(start) {
                    source.provider = Some(request.provider.clone());
                    source.project_id = request.project_id.clone();
                    source.release_id = request.release_id.clone();
                    source.file_id = request.file_id.clone();
                    source.evidence = "published_hash_verified".into();
                }
            }
            Ok(_) => {
                prerequisites.push(format!(
                    "client_pack_selection_required:{}",
                    request.identity
                ));
            }
            Err(issue) if issue.code == "cancelled" => return Err(error("cancelled")),
            Err(issue) => missing.push(issue),
        }
    }
    // Installed loose JARs remain read-only. Provider hash identification enriches
    // evidence; a different client file is never guessed from a filename.
    for path in inventory::source_paths(&root.join("mods"))? {
        if !path.extension().is_some_and(|e| e == "jar") || manifest_paths.contains(&path) {
            continue;
        }
        let start = client_mods.sources.len();
        client_mods.scan_archive(&path, "installed_mod", &mut candidate, cancel)?;
        if !offline {
            let raw = read(&path, MAX_ARCHIVE)?;
            if let Ok(Some(version)) =
                msc_infrastructure::addon_provider::modrinth_version_from_hash(
                    transport,
                    &msc_infrastructure::download_staging::sha512_hex(&raw),
                )
                && version.game_versions.iter().any(|v| v == game)
                && version
                    .loaders
                    .iter()
                    .any(|l| l == context.server.java_flavor.raw_value())
                && let Some(file) = version.files.iter().find(|file| {
                    file.hashes.get("sha512").is_some_and(|h| {
                        h.eq_ignore_ascii_case(&msc_infrastructure::download_staging::sha512_hex(
                            &raw,
                        ))
                    })
                })
                && file.size == raw.len() as u64
                && let Some(evidence) = client_mods.sources.get_mut(start)
            {
                evidence.provider = Some("modrinth".into());
                evidence.project_id = Some(version.project_id);
                evidence.release_id = Some(version.id);
                evidence.evidence = "exact_provider_hash_relationship".into();
            }
        }
    }
    stack.push(client_mods, true, pack_format)?;
    // Nested resources and top-level mod conflicts still require actual client
    // order; the scan order is not presented as Minecraft's priority order.
    for folder in ["overrides", "client-overrides"] {
        let mut override_mods = layer()?;
        override_mods.scan_mods(&source.join(folder).join("mods"), &mut candidate, cancel)?;
        stack.push(override_mods, false, pack_format)?;
    }
    progress(2, 4, "Composing selected resources and their provenance.");
    // Server packs enter only through approved stored files and a matching configured
    // checksum. Merely having an uploaded ZIP never enables it for the map.
    let properties = root.join("server.properties");
    if properties.exists() {
        let bytes = read(&properties, MAX_JSON)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| error("invalid_server_properties"))?;
        let expected = text
            .lines()
            .filter_map(|s| s.split_once('='))
            .find(|(key, _)| key.trim() == "resource-pack-sha1")
            .map(|(_, v)| v.trim());
        if let Some(expected) = expected.filter(|s| s.len() == 40) {
            let mut matched = false;
            for path in inventory::source_paths(&root.join("resource-packs"))? {
                if path.extension().is_some_and(|e| e == "zip")
                    && msc_infrastructure::download_staging::sha1_hex(&read(&path, MAX_ARCHIVE)?)
                        .eq_ignore_ascii_case(expected)
                {
                    let mut pack = layer()?;
                    pack.scan_archive(&path, "configured_server_pack", &mut candidate, cancel)?;
                    stack.push(pack, true, pack_format)?;
                    matched = true;
                    break;
                }
            }
            if !matched {
                prerequisites.push(format!("approved_server_pack_import_required:{expected}"));
            }
        } else if text.lines().any(|s| {
            s.strip_prefix("resource-pack=")
                .is_some_and(|s| !s.trim().is_empty())
        }) {
            prerequisites.push("approved_server_pack_identity_required".into());
        }
    }
    for folder in ["overrides", "client-overrides"] {
        let mut tree = layer()?;
        tree.scan_tree(&source.join(folder), "", folder, cancel)?;
        stack.push(tree, true, pack_format)?;
        if !inventory::source_paths(&source.join(folder).join("resourcepacks"))?.is_empty() {
            prerequisites.push("client_pack_selection_required".into());
        }
    }
    prerequisites.extend(stack.prerequisites.clone());
    if stack.inventory.conflicts > 0 {
        prerequisites.push("client_mod_resource_order_required".into());
    }
    prerequisites.sort();
    prerequisites.dedup();
    candidate
        .input_stamps
        .extend(stack.inventory.file_stamps.clone());
    candidate.verify_inputs(cancel)?;
    let config = store::fingerprint_configs(&root.join("config"), cancel)?;
    let manifest_receipts = if has_source {
        BTreeMap::from([("clientSource".into(), hash_json(&context.revision_inputs)?)])
    } else {
        BTreeMap::new()
    };
    let mut manifest = ResourceManifest {
        schema_version: 1,
        generation_id: String::new(),
        minecraft_version: Some(game.into()),
        loader: context.server.java_flavor.raw_value().into(),
        loader_version: context.server.loader_version.clone(),
        resolver_version: map_assets::RESOLVER_VERSION.into(),
        renderer_version: "msc-private-resource-adapter-1".into(),
        capture_formats: vec![],
        sources: stack.inventory.sources.clone(),
        selected_pack_order: None,
        selection_known: prerequisites.is_empty(),
        selection_revision: hash_json(&stack.winners)?,
        config_fingerprint: hash_json(&config)?,
        manifest_receipts,
        archive_entries: stack.inventory.entries,
        decompressed_bytes: stack.inventory.decompressed,
        resource_documents: stack.inventory.documents,
        resource_conflicts: stack.inventory.conflicts,
    };
    manifest.generation_id = hash_json(&(&manifest, &missing, &prerequisites))?;
    fs_write_receipt(
        candidate.directory(),
        &manifest,
        &stack.winners,
        &missing,
        &prerequisites,
    )?;
    progress(
        3,
        4,
        "Candidate complete; production rendering has not changed.",
    );
    Ok(PreparedResources {
        candidate,
        stack,
        manifest,
        missing,
        prerequisites,
    })
}
fn fs_write_receipt(
    root: &Path,
    manifest: &ResourceManifest,
    winners: &impl serde::Serialize,
    missing: &impl serde::Serialize,
    prerequisites: &impl serde::Serialize,
) -> io::Result<()> {
    std::fs::write(root.join("resources.json"),serde_json::to_vec(&serde_json::json!({"manifest":manifest,"winners":winners,"missing":missing,"prerequisites":prerequisites})).map_err(|_|error("serialization_failed"))?)
}
