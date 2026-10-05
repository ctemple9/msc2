//! Explicit private client preparation. Never launches code or edits a source instance.
use super::*;
use msc_domain::map_assets::{CaptureRequest, ResourceManifest};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::PathBuf;
use zip::ZipArchive;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Prepare {
    pub instance: PathBuf,
    pub context: PathBuf,
    pub game_jar: PathBuf,
    pub helpers: PathBuf,
    pub java: PathBuf,
    pub destination: PathBuf,
    /// Explicit allocation overrides the selected instance's memory settings.
    #[serde(default)]
    pub max_memory_mib: Option<u32>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prepared {
    pub root: PathBuf,
    pub game: PathBuf,
    pub instance_id: String,
    pub request: CaptureRequest,
    pub helper_sha256: String,
    pub files: u64,
    pub bytes: u64,
}
fn write_new(path: &Path, raw: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        safe_path(parent)?;
    }
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(raw)?;
    file.sync_all()
}
struct Candidate(PathBuf, bool);
impl Drop for Candidate {
    fn drop(&mut self) {
        if !self.1 {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
fn copy_tree(
    source: &Path,
    target: &Path,
    receipts: &mut BTreeMap<String, String>,
    game: &Path,
    total: &mut u64,
    cancel: &dyn Fn() -> bool,
) -> io::Result<()> {
    if !source.exists() {
        return Ok(());
    }
    safe_path(source)?;
    let mut entries = 0u64;
    let mut stack = vec![(source.to_path_buf(), target.to_path_buf(), 0u32)];
    while let Some((source, target, depth)) = stack.pop() {
        poll(cancel)?;
        if depth > 32 {
            return Err(error("capture_input_depth_limit"));
        }
        for entry in fs::read_dir(&source)? {
            poll(cancel)?;
            let entry = entry?;
            entries += 1;
            if entries > 10000 {
                return Err(error("capture_input_entry_limit"));
            }
            safe_path(&entry.path())?;
            let metadata = entry.metadata()?;
            let name = entry
                .file_name()
                .to_str()
                .ok_or_else(|| error("invalid_capture_input_name"))?
                .to_string();
            if !safe_member(&name) {
                return Err(error("unsafe_capture_input"));
            }
            let destination = target.join(&name);
            if metadata.is_dir() {
                stack.push((entry.path(), destination, depth + 1));
            } else if metadata.is_file() {
                let raw = read(&entry.path(), MAX_ENTRY)?;
                *total += raw.len() as u64;
                if *total > 2 * 1024 * 1024 * 1024 || receipts.len() >= 10000 {
                    return Err(error("capture_input_budget"));
                }
                write_new(&destination, &raw)?;
                if file_hash(&entry.path(), MAX_ENTRY, cancel)? != hash(&raw) {
                    return Err(error("input_changed"));
                }
                let relative = destination
                    .strip_prefix(game)
                    .map_err(|_| error("unsafe_capture_input"))?
                    .to_string_lossy()
                    .replace('\\', "/");
                receipts.insert(relative, hash(&raw));
            } else {
                return Err(error("linked_input"));
            }
        }
    }
    Ok(())
}
// Bind the complete copied tree, including newly added or removed inputs.
fn tree_receipts(root: &Path, cancel: &dyn Fn() -> bool) -> io::Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    if !root.exists() {
        return Ok(result);
    }
    let mut stack = vec![(root.to_path_buf(), 0u32)];
    let mut bytes = 0u64;
    let mut entries = 0u64;
    while let Some((directory, depth)) = stack.pop() {
        if depth > 32 {
            return Err(error("capture_input_depth_limit"));
        }
        safe_path(&directory)?;
        for entry in fs::read_dir(directory)? {
            poll(cancel)?;
            let entry = entry?;
            entries += 1;
            if entries > 10000 {
                return Err(error("capture_input_entry_limit"));
            }
            safe_path(&entry.path())?;
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|_| error("unsafe_capture_input"))?
                .to_str()
                .ok_or_else(|| error("invalid_capture_input_name"))?
                .replace('\\', "/");
            if !safe_member(&relative) {
                return Err(error("unsafe_capture_input"));
            }
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                stack.push((entry.path(), depth + 1));
            } else if metadata.is_file() {
                bytes = bytes
                    .checked_add(metadata.len())
                    .ok_or_else(|| error("capture_input_budget"))?;
                if bytes > 2 * 1024 * 1024 * 1024 || result.len() >= 10000 {
                    return Err(error("capture_input_budget"));
                }
                result.insert(relative, file_hash(&entry.path(), MAX_ENTRY, cancel)?);
            } else {
                return Err(error("linked_input"));
            }
        }
    }
    Ok(result)
}
fn memory_config(path: &Path) -> io::Result<BTreeMap<String, String>> {
    let raw = match read(path, 64 * 1024) {
        Ok(raw) => raw,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(e),
    };
    let text = std::str::from_utf8(&raw).map_err(|_| error("invalid_capture_memory_settings"))?;
    let mut general = true;
    let mut values = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') && line.ends_with(']') {
            general = line == "[General]";
            continue;
        }
        if !general || line.starts_with(['#', ';']) {
            continue;
        }
        if let Some((key, value)) = line.split_once('=')
            && ["OverrideMemory", "MaxMemAlloc"].contains(&key.trim())
            && values
                .insert(key.trim().to_string(), value.trim().to_string())
                .is_some()
        {
            return Err(error("invalid_capture_memory_settings"));
        }
    }
    Ok(values)
}
fn allocation(value: &str) -> io::Result<u32> {
    value
        .parse::<u32>()
        .ok()
        .filter(|n| (512..=65536).contains(n))
        .ok_or_else(|| error("capture_memory_mib_out_of_range"))
}
fn selected_memory(instance: &Path, requested: Option<u32>) -> io::Result<u32> {
    if let Some(value) = requested {
        return allocation(&value.to_string());
    }
    let settings = memory_config(&instance.join("instance.cfg"))?;
    let override_memory = match settings
        .get("OverrideMemory")
        .map(|v| v.to_ascii_lowercase())
        .as_deref()
    {
        None | Some("false") => false,
        Some("true") => true,
        _ => return Err(error("invalid_capture_memory_settings")),
    };
    if override_memory {
        return settings
            .get("MaxMemAlloc")
            .map(|value| allocation(value))
            .unwrap_or(Ok(3072));
    }
    if let Some(parent) = instance
        .parent()
        .filter(|p| p.file_name().is_some_and(|n| n == "instances"))
        && let Some(launcher) = parent.parent()
    {
        let global = memory_config(&launcher.join("prismlauncher.cfg"))?;
        return global
            .get("MaxMemAlloc")
            .map(|value| allocation(value))
            .unwrap_or(Ok(3072));
    }
    Ok(3072)
}

// Redirect to private files so a probe cannot retain pipe readers indefinitely.
fn verify_java(java: &Path, major: u64, work: &Path, cancel: &dyn Fn() -> bool) -> io::Result<()> {
    const MAX_PROBE: u64 = 64 * 1024;
    let stdout_path = work.join("java-version.stdout");
    let stderr_path = work.join("java-version.stderr");
    let stdout = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&stdout_path)?;
    let stderr = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&stderr_path)?;
    poll(cancel)?;
    let mut child = std::process::Command::new(java)
        .arg("-version")
        .stdin(std::process::Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .spawn()?;
    let started = std::time::Instant::now();
    let result = (|| {
        loop {
            poll(cancel)?;
            if started.elapsed() > std::time::Duration::from_secs(5) {
                return Err(error("capture_java_probe_timeout"));
            }
            if fs::metadata(&stdout_path)?.len() > MAX_PROBE
                || fs::metadata(&stderr_path)?.len() > MAX_PROBE
            {
                return Err(error("capture_java_probe_output_limit"));
            }
            if let Some(status) = child.try_wait()? {
                if fs::metadata(&stdout_path)?.len() > MAX_PROBE {
                    return Err(error("capture_java_probe_output_limit"));
                }
                if !status.success()
                    || !String::from_utf8_lossy(&read(&stderr_path, MAX_PROBE)?)
                        .contains(&format!("\"{major}."))
                {
                    return Err(error("capture_java_version_mismatch"));
                }
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let _ = fs::remove_file(stdout_path);
    let _ = fs::remove_file(stderr_path);
    result
}

/// A prepared directory is published only after actual game/mod/pack/config
/// bytes and the saved chunk receipt match the adopted host context.
pub fn prepare(input: &Prepare, cancel: &dyn Fn() -> bool) -> io::Result<Prepared> {
    if input.destination.exists() {
        return Err(error("capture_destination_exists"));
    }
    let source = input.instance.canonicalize()?;
    let parent = input
        .destination
        .parent()
        .ok_or_else(|| error("invalid_capture_destination"))?
        .canonicalize()?;
    if parent.starts_with(&source) {
        return Err(error("capture_destination_inside_source"));
    }
    safe_path(&input.instance)?;
    safe_path(&input.helpers)?;
    let max_memory_mib = selected_memory(&source, input.max_memory_mib)?;
    let metadata_raw = read(&input.instance.join("mmc-pack.json"), MAX_JSON)?;
    let metadata: serde_json::Value = serde_json::from_slice(&metadata_raw)
        .map_err(|_| error("matching_prism_instance_required"))?;
    let source_game = if input.instance.join(".minecraft").is_dir() {
        input.instance.join(".minecraft")
    } else {
        input.instance.join("minecraft")
    };
    safe_path(&source_game)?;
    // Atomic creation establishes exclusive cleanup ownership of this directory.
    fs::create_dir(&input.destination)?;
    let mut candidate = Candidate(input.destination.clone(), false);
    safe_path(&input.destination)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&input.destination, fs::Permissions::from_mode(0o700))?;
    }
    let instance_id = "msc-capture".to_string();
    let instance = input.destination.join("instances").join(&instance_id);
    let game = instance.join(".minecraft");
    let work = game.join(".msc-map-capture");
    fs::create_dir_all(&work)?;
    let archive_hash = file_hash(&input.context, 256 * 1024 * 1024, cancel)?;
    let mut archive =
        ZipArchive::new(open(&input.context)?).map_err(|_| error("invalid_capture_context"))?;
    if archive.len() > 10001 {
        return Err(error("capture_context_entry_limit"));
    }
    let mut names = BTreeSet::new();
    let mut unpacked = 0u64;
    for i in 0..archive.len() {
        poll(cancel)?;
        let mut entry = archive
            .by_index(i)
            .map_err(|_| error("invalid_capture_context"))?;
        let name = entry.name().to_string();
        if !safe_member(&name)
            || entry.is_dir()
            || !names.insert(name.to_ascii_lowercase())
            || entry
                .unix_mode()
                .is_some_and(|m| ![0, 0o100000].contains(&(m & 0o170000)))
            || entry.size() > MAX_ENTRY
        {
            return Err(error("unsafe_capture_context"));
        }
        if ![
            "request.json",
            "resources.json",
            "client.json",
            "winners.json",
            "files.json",
        ]
        .contains(&name.as_str())
            && !name.starts_with("world/")
        {
            return Err(error("unexpected_capture_context_file"));
        }
        unpacked += entry.size();
        if unpacked > 256 * 1024 * 1024 {
            return Err(error("capture_context_byte_limit"));
        }
        let mut raw = Vec::new();
        entry.by_ref().take(MAX_ENTRY + 1).read_to_end(&mut raw)?;
        if raw.len() as u64 != entry.size() {
            return Err(error("capture_context_byte_limit"));
        }
        write_new(&work.join("received").join(name), &raw)?;
    }
    if file_hash(&input.context, 256 * 1024 * 1024, cancel)? != archive_hash {
        return Err(error("input_changed"));
    }
    let received = work.join("received");
    let expected_files: BTreeMap<String, String> =
        serde_json::from_slice(&read(&received.join("files.json"), MAX_JSON)?)
            .map_err(|_| error("invalid_capture_context"))?;
    if expected_files.len() + 1 != names.len() {
        return Err(error("capture_context_file_set_mismatch"));
    }
    for (name, expected) in &expected_files {
        if !safe_member(name) || file_hash(&received.join(name), MAX_ENTRY, cancel)? != *expected {
            return Err(error("capture_context_checksum_mismatch"));
        }
    }
    let request: CaptureRequest =
        serde_json::from_slice(&read(&received.join("request.json"), MAX_JSON)?)
            .map_err(|_| error("invalid_capture_request"))?;
    let manifest: ResourceManifest =
        serde_json::from_slice(&read(&received.join("resources.json"), MAX_JSON)?)
            .map_err(|_| error("invalid_capture_resources"))?;
    if request.format != msc_domain::map_assets::CAPTURE_FORMAT
        || !manifest.selection_known
        || request.input_fingerprint != hash_json(&manifest)?
        || request.resource_generation_id != manifest.generation_id
        || request.loader != manifest.loader
        || Some(&request.minecraft_version) != manifest.minecraft_version.as_ref()
        || Some(&request.loader_version) != manifest.loader_version.as_ref()
    {
        return Err(error("capture_resource_context_mismatch"));
    }
    let approved_client: bundle::Manifest =
        serde_json::from_slice(&read(&received.join("client.json"), MAX_JSON)?)
            .map_err(|_| error("invalid_capture_client_manifest"))?;
    approved_client.validate()?;
    if manifest.manifest_receipts.get("clientManifest") != Some(&hash_json(&approved_client)?)
        || !approved_client.selection_known
        || Some(&approved_client.minecraft_version) != manifest.minecraft_version.as_ref()
        || approved_client.loader != manifest.loader
        || approved_client.loader_version != manifest.loader_version
        || Some(&approved_client.selected_packs) != manifest.selected_pack_order.as_ref()
        || approved_client.client_config_fingerprint.as_ref()
            != manifest.manifest_receipts.get("clientConfiguration")
    {
        return Err(error("capture_client_manifest_context_mismatch"));
    }
    if supplemental::context_area(request.area)? != request.context_area {
        return Err(error("capture_context_area_mismatch"));
    }
    if request.context_data_fingerprint.as_ref()
        != Some(&supplemental::context_data_fingerprint(
            &received.join("world"),
            cancel,
        )?)
    {
        return Err(error("capture_saved_data_mismatch"));
    }
    let terrain = saved_terrain::WorldSource::Directory(received.join("world")).inspect(
        &request.dimension,
        request.context_area,
        cancel,
    )?;
    if !terrain.missing.is_empty() || terrain.snapshot_id != request.snapshot_id {
        return Err(error("capture_snapshot_mismatch"));
    }
    let components = metadata["components"]
        .as_array()
        .ok_or_else(|| error("invalid_client_components"))?;
    for (uid, expected) in [
        ("net.minecraft", &request.minecraft_version),
        (
            match request.loader.as_str() {
                "forge" => "net.minecraftforge",
                "neoforge" => "net.neoforged",
                "fabric" => "net.fabricmc.fabric-loader",
                _ => return Err(error("unsupported_capture_loader")),
            },
            &request.loader_version,
        ),
    ] {
        let matching = components
            .iter()
            .filter(|c| c["uid"].as_str() == Some(uid))
            .collect::<Vec<_>>();
        if matching.len() != 1 || matching[0]["version"].as_str() != Some(expected) {
            return Err(error("capture_client_version_mismatch"));
        }
    }
    let loader_uid = match request.loader.as_str() {
        "forge" => "net.minecraftforge",
        "neoforge" => "net.neoforged",
        "fabric" => "net.fabricmc.fabric-loader",
        _ => return Err(error("unsupported_capture_loader")),
    };
    if components.iter().any(|c| {
        let Some(uid) = c["uid"].as_str() else {
            return true;
        };
        !["net.minecraft", "org.lwjgl3", "org.lwjgl", loader_uid].contains(&uid)
            && !(request.loader == "fabric" && uid == "net.fabricmc.intermediary")
    }) {
        return Err(error("capture_custom_client_component_refused"));
    }
    if components
        .iter()
        .any(|c| c["cachedVersion"].is_object() || c["patches"].is_array())
    {
        return Err(error("capture_custom_client_patch_refused"));
    }
    let helpers: serde_json::Value =
        serde_json::from_slice(&read(&input.helpers.join("helpers.json"), MAX_JSON)?)
            .map_err(|_| error("invalid_capture_helper_manifest"))?;
    let helper = helpers["helpers"]
        .as_array()
        .and_then(|v| {
            v.iter().find(|h| {
                h["minecraft"].as_str() == Some(&request.minecraft_version)
                    && h["loaderFamily"].as_str() == Some(&request.loader)
                    && h["loader"].as_str() == Some(&request.loader_version)
            })
        })
        .ok_or_else(|| error("matching_capture_helper_unavailable"))?;
    let java_major = helper["java"]
        .as_u64()
        .ok_or_else(|| error("invalid_capture_helper_manifest"))?;
    safe_path(&input.java)?;
    verify_java(&input.java, java_major, &work, cancel)?;
    let helper_name = helper["file"]
        .as_str()
        .filter(|n| safe_member(n) && !n.contains('/') && n.ends_with(".jar"))
        .ok_or_else(|| error("unsafe_capture_helper"))?;
    let helper_raw = read(&input.helpers.join(helper_name), MAX_JSON)?;
    let helper_hash = hash(&helper_raw);
    if Some(helper_hash.as_str()) != helper["sha256"].as_str()
        || helper["bytes"].as_u64() != Some(helper_raw.len() as u64)
    {
        return Err(error("capture_helper_checksum_mismatch"));
    }
    let mut source_trees = BTreeMap::new();
    for folder in ["mods", "resourcepacks", "config"] {
        source_trees.insert(folder, tree_receipts(&source_game.join(folder), cancel)?);
    }
    // Reuse the resource inspector's actual loader, mod closure and pack order.
    let selection = bundle::export_instance(
        &input.instance,
        &work.join("inspection"),
        &work.join("inspection.zip"),
        cancel,
    )?;
    if !selection.selection_known
        || selection.selected_packs != manifest.selected_pack_order.clone().unwrap_or_default()
    {
        return Err(error("capture_client_pack_order_mismatch"));
    }
    let configs = store::fingerprint_configs(&source_game.join("config"), cancel)?;
    let approved_config = manifest
        .manifest_receipts
        .get("clientConfiguration")
        .ok_or_else(|| error("capture_client_configuration_receipt_required"))?;
    if hash_json(&configs)? != *approved_config {
        return Err(error("capture_client_configuration_mismatch"));
    }
    let active = selection
        .layers
        .iter()
        .filter(|l| l.kind == "mod" || selection.selected_packs.contains(&l.id))
        .flat_map(|l| l.sources.iter())
        .filter(|s| s.enabled)
        .map(|s| s.sha256.clone())
        .collect::<BTreeSet<_>>();
    // Host-only mods are not client dependencies. Bind the exact client closure
    // explicitly approved through the imported portable resource manifest.
    let required = approved_client
        .layers
        .iter()
        .filter(|layer| layer.kind == "mod" || approved_client.selected_packs.contains(&layer.id))
        .flat_map(|layer| layer.sources.iter())
        .filter(|source| source.enabled)
        .map(|source| source.sha256.clone())
        .collect::<BTreeSet<_>>();
    if !active.is_subset(&required) || !required.is_subset(&active) {
        return Err(error("capture_client_source_closure_mismatch"));
    }
    // Vanilla trees are checked from the actual version-matched client JAR.
    let mut jar =
        ZipArchive::new(open(&input.game_jar)?).map_err(|_| error("invalid_minecraft_client"))?;
    let mut assets = BTreeMap::new();
    let mut biomes = BTreeMap::new();
    let mut expanded = 0u64;
    if jar.len() > 100000 {
        return Err(error("capture_game_entry_limit"));
    }
    for i in 0..jar.len() {
        poll(cancel)?;
        let mut entry = jar
            .by_index(i)
            .map_err(|_| error("invalid_minecraft_client"))?;
        let name = entry.name().to_string();
        if entry.is_dir()
            || (!name.starts_with("assets/minecraft/")
                && !name.starts_with("data/minecraft/worldgen/biome/"))
        {
            continue;
        }
        if !safe_member(&name) || entry.size() > MAX_ENTRY {
            return Err(error("unsafe_minecraft_client_resource"));
        }
        expanded += entry.size();
        if expanded > 512 * 1024 * 1024 {
            return Err(error("capture_game_byte_limit"));
        }
        let mut raw = Vec::new();
        entry.by_ref().take(MAX_ENTRY + 1).read_to_end(&mut raw)?;
        if raw.len() as u64 != entry.size() {
            return Err(error("invalid_minecraft_client_resource"));
        }
        let map = if name.starts_with("assets/") {
            &mut assets
        } else {
            &mut biomes
        };
        if map.insert(name, hash(&raw)).is_some() {
            return Err(error("duplicate_minecraft_client_resource"));
        }
    }
    for source in manifest.sources.iter().filter(|s| s.enabled) {
        let observed = match source.kind.as_str() {
            "existing_client_cache" => Some(hash_json(&assets)?),
            "existing_client_biomes" => Some(hash_json(&biomes)?),
            _ => None,
        };
        if observed.as_ref().is_some_and(|h| h != &source.sha256) {
            return Err(error("capture_vanilla_resource_mismatch"));
        }
    }
    let mut receipts = BTreeMap::new();
    let mut copied = 0u64;
    for folder in ["mods", "resourcepacks", "config"] {
        copy_tree(
            &source_game.join(folder),
            &game.join(folder),
            &mut receipts,
            &game,
            &mut copied,
            cancel,
        )?;
    }
    for (folder, expected) in &source_trees {
        if tree_receipts(&source_game.join(folder), cancel)? != *expected
            || tree_receipts(&game.join(folder), cancel)? != *expected
        {
            return Err(error("capture_client_sources_changed"));
        }
    }
    let options = read(&source_game.join("options.txt"), MAX_JSON)?;
    write_new(&game.join("options.txt"), &options)?;
    write_new(&instance.join("mmc-pack.json"), &metadata_raw)?;
    // Inspect the executable archives actually cloned, not only observations of
    // source files before and after copying: a source can change and change back.
    let cloned = bundle::export_instance(
        &instance,
        &work.join("clone-inspection"),
        &work.join("clone-inspection.zip"),
        cancel,
    )?;
    let cloned_active = cloned
        .layers
        .iter()
        .filter(|l| l.kind == "mod" || cloned.selected_packs.contains(&l.id))
        .flat_map(|l| l.sources.iter())
        .filter(|source| source.enabled)
        .map(|source| source.sha256.clone())
        .collect::<BTreeSet<_>>();
    if !cloned.selection_known
        || cloned.selected_packs != selection.selected_packs
        || cloned_active != required
        || (approved_client.mod_order.is_some() && cloned.mod_order != approved_client.mod_order)
        || cloned.client_config_fingerprint.as_ref() != Some(approved_config)
    {
        return Err(error("capture_cloned_source_closure_mismatch"));
    }
    fs::remove_dir_all(work.join("clone-inspection"))?;
    fs::remove_file(work.join("clone-inspection.zip"))?;
    for (source, target) in [
        (received.join("world"), work.join("snapshot")),
        (received.join("world"), game.join("saves/msc-capture")),
    ] {
        copy_tree(&source, &target, &mut receipts, &game, &mut copied, cancel)?;
    }
    if request.context_data_fingerprint.as_ref()
        != Some(&supplemental::context_data_fingerprint(
            &work.join("snapshot"),
            cancel,
        )?)
    {
        return Err(error("capture_saved_data_changed"));
    }
    receipts.retain(|name, _| !name.starts_with("saves/"));
    if hash_json(&store::fingerprint_configs(&game.join("config"), cancel)?)? != *approved_config {
        return Err(error("capture_client_configuration_changed"));
    }
    for name in [
        "request.json",
        "resources.json",
        "client.json",
        "winners.json",
    ] {
        let raw = read(&received.join(name), MAX_JSON)?;
        write_new(&work.join(name), &raw)?;
        receipts.insert(format!(".msc-map-capture/{name}"), hash(&raw));
    }
    write_new(&game.join("mods/msc-map-capture.jar"), &helper_raw)?;
    receipts.insert("mods/msc-map-capture.jar".into(), helper_hash.clone());
    // Keep the actual vanilla binary for pre/post checks; Prism loads the same
    // selected component. Preparation never copies accounts, sessions or saves.
    let raw = read(&input.game_jar, MAX_ENTRY)?;
    write_new(&work.join("client.jar"), &raw)?;
    receipts.insert(".msc-map-capture/client.jar".into(), hash(&raw));

    let java = input
        .java
        .to_str()
        .ok_or_else(|| error("invalid_java_path"))?
        .replace('\\', "/");
    if java.contains(['\n', '\r']) {
        return Err(error("invalid_java_path"));
    }
    write_new(
        &input.destination.join("prismlauncher.cfg"),
        b"[General]\nCloseAfterLaunch=false\nQuitAfterGameStop=true\n",
    )?;
    write_new(&instance.join("instance.cfg"),format!("[General]\nname=MSC Saved Map Capture\nOverrideJavaLocation=true\nJavaPath={java}\nOverrideMemory=true\nMinMemAlloc=512\nMaxMemAlloc={max_memory_mib}\n").as_bytes())?;
    write_new(&work.join("prepared.json"),&serde_json::to_vec(&serde_json::json!({"files":receipts,"contextSha256":archive_hash,"sourceVerified":true,"selectedPacks":String::from_utf8_lossy(&options).lines().find_map(|line|line.strip_prefix("resourcePacks:")).and_then(|raw|serde_json::from_str::<Vec<String>>(raw).ok()).ok_or_else(||error("client_pack_selection_required"))?})).map_err(|_|error("serialization_failed"))?)?;
    let _ = fs::remove_dir_all(work.join("inspection"));
    let _ = fs::remove_dir_all(received);
    let _ = fs::remove_file(work.join("inspection.zip"));
    for (folder, expected) in &source_trees {
        if tree_receipts(&source_game.join(folder), cancel)? != *expected {
            return Err(error("capture_client_sources_changed"));
        }
    }
    if read(&input.instance.join("mmc-pack.json"), MAX_JSON)? != metadata_raw {
        return Err(error("capture_client_components_changed"));
    }
    if read(&source_game.join("options.txt"), MAX_JSON)? != options {
        return Err(error("capture_client_pack_selection_changed"));
    }
    poll(cancel)?;
    candidate.1 = true;
    Ok(Prepared {
        root: input.destination.clone(),
        game,
        instance_id,
        request,
        helper_sha256: helper_hash,
        files: receipts.len() as u64,
        bytes: copied,
    })
}

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/map_capture_payload.rs"));
}
/// Restore release-owned files from this binary. No network, GPU or Java starts.
pub fn bundled_helpers(cache: &Path) -> io::Result<PathBuf> {
    if embedded::HELPERS.is_empty() {
        let root = std::env::current_exe()?
            .parent()
            .ok_or_else(|| error("capture_helpers_unavailable"))?
            .join("map-capture/0.2.0");
        if root.join("helpers.json").is_file() {
            return Ok(root);
        }
        return Err(error("capture_helpers_unavailable"));
    }
    fs::create_dir_all(cache)?;
    safe_path(cache)?;
    let identity = hash(
        embedded::HELPERS
            .iter()
            .find(|(n, _)| *n == "helpers.json")
            .ok_or_else(|| error("capture_helpers_unavailable"))?
            .1,
    );
    let destination = cache.join(identity);
    if destination.exists() {
        for (name, raw) in embedded::HELPERS {
            if read(&destination.join(name), MAX_JSON)? != *raw {
                return Err(error("capture_helper_checksum_mismatch"));
            }
        }
        return Ok(destination);
    }
    let candidate = cache.join(format!("candidate-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&candidate)?;
    let mut guard = Candidate(candidate.clone(), false);
    for (name, raw) in embedded::HELPERS {
        write_new(&candidate.join(name), raw)?;
    }
    if fs::rename(&candidate, &destination).is_err() {
        if !destination.is_dir() {
            return Err(error("capture_helper_install_failed"));
        }
        for (name, raw) in embedded::HELPERS {
            if read(&destination.join(name), MAX_JSON)? != *raw {
                return Err(error("capture_helper_checksum_mismatch"));
            }
        }
    } else {
        guard.1 = true;
    }
    Ok(destination)
}

/// Export checked resources through private scratch; failed output is never published.
pub fn export_resources(
    instance: &Path,
    destination: &Path,
    cancel: &dyn Fn() -> bool,
) -> io::Result<bundle::Manifest> {
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    safe_path(parent)?;
    let source = instance.canonicalize()?;
    if parent.canonicalize()?.starts_with(source) {
        return Err(error("capture_destination_inside_source"));
    }
    if destination.exists() {
        return Err(error("capture_destination_exists"));
    }
    let temporary = std::env::temp_dir()
        .canonicalize()?
        .join(format!("msc-map-resources-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&temporary)?;
    let _guard = Candidate(temporary.clone(), false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o700))?;
    }
    let archive = temporary.join("resources.zip");
    let manifest =
        bundle::export_instance(instance, &temporary.join("inspection"), &archive, cancel)?;
    let mut input = open(&archive)?;
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)?;
    let published = (|| {
        let mut buffer = [0u8; 65536];
        loop {
            poll(cancel)?;
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            output.write_all(&buffer[..count])?;
        }
        output.sync_all()?;
        poll(cancel)
    })();
    drop(output);
    if let Err(e) = published {
        let _ = fs::remove_file(destination);
        return Err(e);
    }
    Ok(manifest)
}
