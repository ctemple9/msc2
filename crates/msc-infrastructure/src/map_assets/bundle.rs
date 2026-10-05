//! Portable resource-only imports. No launcher secrets, executable classes or saves cross hosts.
use super::{inventory::Inventory, store::Store, *};
use msc_domain::map_assets::{Binding, SourceEvidence};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Seek, Write};
use std::path::PathBuf;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

pub const MAX_BUNDLE: u64 = 8 * 1024 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Layer {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub sources: Vec<SourceEvidence>,
    pub resources: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub minecraft_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub selection_known: bool,
    /// Low to high priority. Mod layers remain unordered unless explicitly selected.
    pub layers: Vec<Layer>,
    pub selected_packs: Vec<String>,
    pub mod_order: Option<Vec<String>>,
    pub evidence: String,
    #[serde(default)]
    pub required_sources: Vec<acquire::RequiredSource>,
    #[serde(default)]
    pub curseforge_files: Vec<(i64, i64)>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub binding: Binding,
    pub input_revision: String,
    pub bundle_sha256: String,
    pub manifest: Manifest,
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn resource(name: &str) -> bool {
    safe_member(name)
        && (name.starts_with("assets/")
            || name == "pack.mcmeta"
            || name.contains("/assets/")
            || name.starts_with("data/minecraft/worldgen/biome/"))
        && ![".class", ".jar", ".exe", ".dll", ".so"]
            .iter()
            .any(|suffix| name.ends_with(suffix))
}
impl Manifest {
    pub fn validate(&self) -> io::Result<()> {
        if self.schema_version != 1
            || self.evidence != "local_hashed"
            || self.minecraft_version.is_empty()
            || self.minecraft_version.len() > 128
            || self.loader.len() > 32
            || self.layers.len() > 10_000
            || self.required_sources.len() > 10_000
            || self.curseforge_files.len() > 10_000
            || self
                .curseforge_files
                .iter()
                .any(|(p, f)| *p <= 0 || *f <= 0)
            || !self.curseforge_files.is_empty() && !self.required_sources.is_empty()
        {
            return Err(error("invalid_client_resource_manifest"));
        }
        let mut ids = BTreeSet::new();
        let mut count = 0;
        for layer in &self.layers {
            if !digest(&layer.id)
                || !ids.insert(layer.id.clone())
                || !matches!(layer.kind.as_str(), "mod" | "pack" | "override")
                || layer.sources.len() > 1024
                || layer.label.len() > 256
                || layer.label.chars().any(char::is_control)
            {
                return Err(error("invalid_client_resource_layer"));
            }
            let mut names = BTreeSet::new();
            for (name, sha) in &layer.resources {
                count += 1;
                if count > 2_000_000
                    || !resource(name)
                    || !digest(sha)
                    || !names.insert(name.to_lowercase())
                {
                    return Err(error("unsafe_or_colliding_client_resource"));
                }
            }
            if layer
                .sources
                .iter()
                .any(|s| s.evidence != "local_hashed" || s.provider.is_some() || s.bytes > MAX_SCAN)
            {
                return Err(error("unverified_client_provenance"));
            }
        }
        let check_order = |order: &[String], kind: &str| {
            let mut seen = BTreeSet::new();
            order.iter().all(|id| {
                seen.insert(id) && self.layers.iter().any(|l| &l.id == id && l.kind == kind)
            })
        };
        if !check_order(&self.selected_packs, "pack")
            || self.mod_order.as_ref().is_some_and(|o| {
                !check_order(o, "mod")
                    || o.len() != self.layers.iter().filter(|l| l.kind == "mod").count()
            })
        {
            return Err(error("invalid_client_resource_order"));
        }
        for source in &self.required_sources {
            if !safe_member(&source.path)
                || !source.path.starts_with("mods/")
                || source.hashes.is_empty()
                || source.bytes == 0
                || source.bytes > MAX_ARCHIVE
                || source.urls.len() > 8
                || source.urls.iter().any(|u| !acquire::allowed_download(u))
            {
                return Err(error("invalid_exact_client_source"));
            }
        }
        Ok(())
    }
}

/// Prism's selected instance or a directly selected official game directory.
/// Only unambiguous installed version metadata is accepted.
pub fn export_instance(
    root: &Path,
    scratch: &Path,
    destination: &Path,
    cancel: &dyn Fn() -> bool,
) -> io::Result<Manifest> {
    safe_path(root)?;
    let root = if root.join(".minecraft").is_dir() {
        root.join(".minecraft")
    } else if root.join("minecraft").is_dir() {
        root.join("minecraft")
    } else {
        root.to_path_buf()
    };
    safe_path(&root)?;
    let mut game = None;
    let mut loader = "vanilla".to_string();
    let mut loader_version = None;
    let prism = root.parent().unwrap_or(&root).join("mmc-pack.json");
    if prism.exists() {
        let value: serde_json::Value = serde_json::from_slice(&read(&prism, MAX_JSON)?)
            .map_err(|_| error("invalid_client_version_metadata"))?;
        for component in value["components"]
            .as_array()
            .ok_or_else(|| error("invalid_client_version_metadata"))?
        {
            let uid = component["uid"].as_str().unwrap_or("");
            let version = component["version"].as_str().map(str::to_string);
            match uid {
                "net.minecraft" => game = version,
                "net.fabricmc.fabric-loader" => {
                    loader = "fabric".into();
                    loader_version = version;
                }
                "net.minecraftforge" => {
                    loader = "forge".into();
                    loader_version = version;
                }
                "net.neoforged" => {
                    loader = "neoforge".into();
                    loader_version = version;
                }
                _ => {}
            }
        }
    } else if root.join("modrinth.index.json").exists() {
        let value: serde_json::Value =
            serde_json::from_slice(&read(&root.join("modrinth.index.json"), MAX_JSON)?)
                .map_err(|_| error("invalid_client_version_metadata"))?;
        game = value["dependencies"]["minecraft"]
            .as_str()
            .map(str::to_string);
        for (key, name) in [
            ("fabric-loader", "fabric"),
            ("forge", "forge"),
            ("neoforge", "neoforge"),
        ] {
            if let Some(v) = value["dependencies"][key].as_str() {
                loader = name.into();
                loader_version = Some(v.into());
            }
        }
    } else if root.join("manifest.json").exists() {
        let raw = read(&root.join("manifest.json"), MAX_JSON)?;
        let manifest = msc_domain::modpack_manifest::parse_curseforge_metadata(
            std::str::from_utf8(&raw).map_err(|_| error("invalid_client_version_metadata"))?,
        )
        .map_err(|_| error("invalid_client_version_metadata"))?;
        game = Some(manifest.minecraft_version);
        loader = manifest
            .loader_flavor
            .map(|l| l.label().to_lowercase())
            .unwrap_or_else(|| "vanilla".into());
        loader_version = manifest.loader_version;
    } else {
        // launcher_profiles is inspected only for version IDs. No account data is retained.
        let profiles = root.join("launcher_profiles.json");
        if profiles.exists() {
            let value: serde_json::Value = serde_json::from_slice(&read(&profiles, MAX_JSON)?)
                .map_err(|_| error("invalid_client_version_metadata"))?;
            let versions = value["profiles"]
                .as_object()
                .ok_or_else(|| error("client_instance_version_required"))?
                .values()
                .filter_map(|p| p["lastVersionId"].as_str())
                .collect::<BTreeSet<_>>();
            if versions.len() != 1 {
                return Err(error("select_unambiguous_client_instance"));
            }
            let id = versions.into_iter().next().unwrap();
            if !safe_member(id) || id.contains('/') {
                return Err(error("unsafe_client_version"));
            }
            let value: serde_json::Value = serde_json::from_slice(&read(
                &root.join("versions").join(id).join(format!("{id}.json")),
                MAX_JSON,
            )?)
            .map_err(|_| error("invalid_client_version_metadata"))?;
            game = Some(value["inheritsFrom"].as_str().unwrap_or(id).into());
            if value["inheritsFrom"].is_string() {
                return Err(error("select_launcher_instance_with_loader_metadata"));
            }
        }
    }
    let mut manifest = Manifest {
        schema_version: 1,
        minecraft_version: game.ok_or_else(|| error("client_instance_version_required"))?,
        loader,
        loader_version,
        selection_known: false,
        layers: vec![],
        selected_packs: vec![],
        mod_order: None,
        evidence: "local_hashed".into(),
        required_sources: vec![],
        curseforge_files: vec![],
    };
    let store = Store::open(scratch.join("scan"))?;
    let mut candidate = store.begin()?;
    let mut objects = BTreeMap::new();
    let mut add = |path: &Path, kind: &str| -> io::Result<String> {
        poll(cancel)?;
        let mut inventory = Inventory::default();
        inventory.capture_into(
            candidate
                .directory()
                .join(format!("layer-{}", manifest.layers.len())),
        )?;
        if path.is_dir() {
            inventory.scan_tree(path, "", "client_resources", cancel)?;
        } else {
            inventory.scan_archive(path, "client_resources", &mut candidate, cancel)?;
        }
        if inventory
            .resources
            .values()
            .any(|r| r.conflict || r.invalid)
        {
            return Err(error("conflicting_or_invalid_client_resources"));
        }
        let mut resources = BTreeMap::new();
        for (name, r) in inventory.resources {
            if !resource(&name) {
                return Err(error("unsafe_client_resource"));
            }
            objects.insert(
                r.sha256.clone(),
                r.object
                    .ok_or_else(|| error("resource_bytes_unavailable"))?,
            );
            resources.insert(name, r.sha256);
        }
        candidate.input_stamps.extend(inventory.file_stamps);
        let id = hash_json(&(kind, &inventory.sources, &resources))?;
        if manifest.layers.iter().any(|l| l.id == id) {
            return Ok(id);
        }
        manifest.layers.push(Layer {
            id: id.clone(),
            kind: kind.into(),
            label: path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Selected resources")
                .to_string(),
            sources: inventory.sources,
            resources,
        });
        Ok(id)
    };
    for path in inventory::source_paths(&root.join("mods"))? {
        if path.extension().is_some_and(|s| s == "jar") {
            add(&path, "mod")?;
        }
    }
    let mut packs = BTreeMap::new();
    for path in inventory::source_paths(&root.join("resourcepacks"))? {
        if path.is_dir() || path.extension().is_some_and(|s| s == "zip") {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| error("invalid_client_pack_name"))?
                .to_string();
            packs.insert(format!("file/{name}"), add(&path, "pack")?);
        }
    }
    let options = root.join("options.txt");
    if options.exists() {
        let text = String::from_utf8(read(&options, MAX_JSON)?)
            .map_err(|_| error("invalid_client_pack_selection"))?;
        if let Some(raw) = text.lines().find_map(|l| l.strip_prefix("resourcePacks:")) {
            let selected: Vec<String> =
                serde_json::from_str(raw).map_err(|_| error("invalid_client_pack_selection"))?;
            manifest.selection_known = true;
            for name in selected {
                if matches!(name.as_str(), "vanilla" | "mod_resources" | "fabric") {
                    continue;
                }
                manifest.selected_packs.push(
                    packs
                        .get(&name)
                        .ok_or_else(|| error("selected_client_pack_unavailable"))?
                        .clone(),
                );
            }
        }
    }
    candidate.verify_inputs(cancel)?;
    manifest.validate()?;
    let mut writer = ZipWriter::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?,
    );
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("manifest.json", options)?;
    writer.write_all(&serde_json::to_vec(&manifest).map_err(|_| error("serialization_failed"))?)?;
    let mut total = 0;
    for (sha, path) in objects {
        poll(cancel)?;
        let bytes = read(&path, MAX_ENTRY)?;
        total += bytes.len() as u64;
        if total > MAX_BUNDLE - MAX_JSON || hash(&bytes) != sha {
            return Err(error("client_bundle_budget_or_checksum"));
        }
        writer.start_file(format!("objects/{sha}"), options)?;
        writer.write_all(&bytes)?;
    }
    writer.finish()?.sync_all()?;
    Ok(manifest)
}

/// Extract only checksum-named resource objects. Reinspect through Inventory before selection.
pub fn unpack(bundle: &Path, target: &Path, cancel: &dyn Fn() -> bool) -> io::Result<Manifest> {
    let mut file = open(bundle)?;
    if file.metadata()?.len() > MAX_BUNDLE {
        return Err(error("client_bundle_byte_limit"));
    }
    let expanded = expanded_size(bundle)?;
    fs::create_dir_all(target)?;
    if fs2::available_space(target)? < expanded + 64 * 1024 * 1024 {
        return Err(error("client_bundle_free_space"));
    }
    file.rewind()?;
    let mut archive = ZipArchive::new(file).map_err(|_| error("invalid_client_bundle"))?;
    if archive.len() > 2_000_001 {
        return Err(error("client_bundle_entry_limit"));
    }
    let manifest: Manifest = {
        let mut entry = archive
            .by_name("manifest.json")
            .map_err(|_| error("client_bundle_manifest_required"))?;
        if entry.size() > MAX_JSON {
            return Err(error("client_bundle_manifest_limit"));
        }
        let mut bytes = Vec::new();
        entry.by_ref().take(MAX_JSON + 1).read_to_end(&mut bytes)?;
        serde_json::from_slice(&bytes).map_err(|_| error("invalid_client_resource_manifest"))?
    };
    manifest.validate()?;
    let required = manifest
        .layers
        .iter()
        .flat_map(|l| l.resources.values().cloned())
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut total = 0;
    fs::create_dir_all(target.join("objects"))?;
    safe_path(target)?;
    for i in 0..archive.len() {
        poll(cancel)?;
        let mut entry = archive
            .by_index(i)
            .map_err(|_| error("invalid_client_bundle"))?;
        let name = entry.name().to_string();
        if !seen.insert(name.clone())
            || entry.is_dir()
            || entry
                .unix_mode()
                .is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o100000)
        {
            return Err(error("unsafe_client_bundle_entry"));
        }
        if name == "manifest.json" {
            continue;
        }
        let sha = name
            .strip_prefix("objects/")
            .filter(|s| digest(s))
            .ok_or_else(|| error("unexpected_client_bundle_entry"))?;
        if !required.contains(sha) || entry.size() > MAX_ENTRY {
            return Err(error("unexpected_client_bundle_object"));
        }
        total += entry.size();
        if total > MAX_SCAN {
            return Err(error("client_bundle_decompressed_limit"));
        }
        let mut bytes = Vec::new();
        entry.by_ref().take(MAX_ENTRY + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != entry.size() || hash(&bytes) != sha {
            return Err(error("client_bundle_checksum"));
        }
        fs::write(target.join("objects").join(sha), bytes)?;
    }
    if required
        .iter()
        .any(|sha| !seen.contains(&format!("objects/{sha}")))
    {
        return Err(error("client_bundle_object_missing"));
    }
    // Inventory validates JSON/image bounds, resource references and case semantics on every host.
    for layer in &manifest.layers {
        let root = target.join("layers").join(&layer.id);
        fs::create_dir_all(&root)?;
        for (name, sha) in &layer.resources {
            poll(cancel)?;
            let destination = root.join(name);
            fs::create_dir_all(
                destination
                    .parent()
                    .ok_or_else(|| error("unsafe_client_resource"))?,
            )?;
            fs::copy(target.join("objects").join(sha), destination)?;
        }
        let mut inventory = Inventory::default();
        inventory.scan_tree(&root, "", "imported_client_resources", cancel)?;
        if inventory
            .resources
            .values()
            .any(|r| r.invalid || r.conflict)
        {
            return Err(error("invalid_imported_client_resource"));
        }
    }
    if !manifest.curseforge_files.is_empty() {
        let id = format!(
            "{}-{}",
            manifest.loader,
            manifest.loader_version.as_deref().unwrap_or("")
        );
        let value = serde_json::json!({"manifestType":"minecraftModpack", "minecraft":{"version":manifest.minecraft_version,"modLoaders":[{"id":id,"primary":true}]},"files":manifest.curseforge_files.iter().map(|(p,f)| serde_json::json!({"projectID":p,"fileID":f,"required":true})).collect::<Vec<_>>()});
        fs::write(
            target.join("manifest.json"),
            serde_json::to_vec(&value).map_err(|_| error("serialization_failed"))?,
        )?;
    }
    Ok(manifest)
}

pub fn receipt_root(store: &Store, binding: &Binding) -> io::Result<PathBuf> {
    Ok(store.directory().join("imports").join(hash_json(&(
        &binding.agent_host_id,
        &binding.server_id,
        &binding.slot_id,
    ))?))
}

/// Pack archives are unpacked to private scratch using a strict resource allowlist.
/// Missing referenced client downloads remain exact preparation inputs, never guessed versions.
pub fn export_archive(
    source: &Path,
    scratch: &Path,
    destination: &Path,
    cancel: &dyn Fn() -> bool,
) -> io::Result<Manifest> {
    let mut archive =
        ZipArchive::new(open(source)?).map_err(|_| error("invalid_client_pack_archive"))?;
    if archive.len() > 200_000 || fs::metadata(source)?.len() > MAX_ARCHIVE {
        return Err(error("client_pack_archive_limit"));
    }
    let root = scratch.join("pack-instance");
    fs::create_dir_all(&root)?;
    let mut names = BTreeSet::new();
    let mut priorities = BTreeMap::new();
    let mut total = 0;
    for i in 0..archive.len() {
        poll(cancel)?;
        let mut entry = archive
            .by_index(i)
            .map_err(|_| error("invalid_client_pack_archive"))?;
        let name = entry.name().to_string();
        if !safe_member(&name)
            || !names.insert(name.trim_end_matches('/').to_lowercase())
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err(error("unsafe_client_pack_archive"));
        }
        if entry.is_dir() {
            continue;
        }
        total += entry.size();
        if total > MAX_SCAN || entry.size() > MAX_ENTRY {
            return Err(error("client_pack_decompressed_limit"));
        }
        let selected = name
            .strip_prefix("client-overrides/")
            .or_else(|| name.strip_prefix("overrides/"))
            .unwrap_or(&name);
        if !(name == "modrinth.index.json"
            || name == "manifest.json"
            || selected == "options.txt"
            || selected.starts_with("mods/") && selected.ends_with(".jar")
            || selected.starts_with("resourcepacks/") && selected.ends_with(".zip"))
        {
            continue;
        }
        let limit = if matches!(
            selected,
            "options.txt" | "manifest.json" | "modrinth.index.json"
        ) {
            MAX_JSON
        } else {
            MAX_ARCHIVE
        };
        if entry.size() > limit {
            return Err(error("client_pack_entry_limit"));
        }
        let path = root.join(selected);
        fs::create_dir_all(
            path.parent()
                .ok_or_else(|| error("unsafe_client_pack_archive"))?,
        )?;
        let mut bytes = Vec::new();
        entry.by_ref().take(limit + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != entry.size() {
            return Err(error("client_pack_entry_size"));
        }
        let priority = if name.starts_with("client-overrides/") {
            2
        } else if name.starts_with("overrides/") {
            1
        } else {
            0
        };
        let key = selected.to_lowercase();
        if priorities
            .get(&key)
            .is_none_or(|(old, _): &(u8, String)| *old < priority)
        {
            if let Some((_, old_name)) = priorities.get(&key)
                && old_name != selected
            {
                return Err(error("case_colliding_client_override"));
            }
            fs::write(path, bytes)?;
            priorities.insert(key, (priority, selected.to_string()));
        }
    }
    if root.join("manifest.json").exists() && !root.join("modrinth.index.json").exists() {
        let mut manifest = export_instance(&root, scratch, destination, cancel)?;
        let raw = read(&root.join("manifest.json"), MAX_JSON)?;
        let parsed = msc_domain::modpack_manifest::parse_curseforge_metadata(
            std::str::from_utf8(&raw).map_err(|_| error("invalid_client_pack_manifest"))?,
        )
        .map_err(|_| error("invalid_client_pack_manifest"))?;
        manifest.curseforge_files = parsed
            .files
            .into_iter()
            .filter(|f| f.required)
            .map(|f| (f.project_id, f.file_id))
            .collect();
        rewrite_manifest(destination, &manifest, scratch)?;
        return Ok(manifest);
    }
    let mut manifest = export_instance(&root, scratch, destination, cancel)?;
    let value: serde_json::Value =
        serde_json::from_slice(&read(&root.join("modrinth.index.json"), MAX_JSON)?)
            .map_err(|_| error("invalid_client_pack_manifest"))?;
    for file in value["files"]
        .as_array()
        .ok_or_else(|| error("invalid_client_pack_manifest"))?
    {
        let path = file["path"]
            .as_str()
            .ok_or_else(|| error("invalid_client_pack_manifest"))?;
        if !safe_member(path) {
            return Err(error("unsafe_client_pack_archive"));
        }
        if !path.starts_with("mods/")
            || !path.ends_with(".jar")
            || matches!(
                file["env"]["client"].as_str(),
                Some("unsupported" | "optional")
            )
        {
            continue;
        }
        let hashes = serde_json::from_value(file["hashes"].clone())
            .map_err(|_| error("missing_client_published_hash"))?;
        let bytes = file["fileSize"]
            .as_u64()
            .ok_or_else(|| error("missing_client_published_size"))?;
        let urls: Vec<String> = serde_json::from_value(file["downloads"].clone())
            .map_err(|_| error("invalid_client_download"))?;
        if urls.iter().any(|u| !acquire::allowed_download(u)) {
            return Err(error("unapproved_client_download_host"));
        }
        manifest.required_sources.push(acquire::RequiredSource {
            identity: hash_json(&(path, &hashes, bytes))?,
            path: path.into(),
            provider: "modrinth_manifest".into(),
            project_id: None,
            release_id: None,
            file_id: None,
            hashes,
            bytes,
            urls,
            reason: None,
        });
    }
    // Rebuild only the manifest entry; resource objects were already filtered and hashed.
    rewrite_manifest(destination, &manifest, scratch)?;
    Ok(manifest)
}
fn rewrite_manifest(bundle: &Path, manifest: &Manifest, scratch: &Path) -> io::Result<()> {
    manifest.validate()?;
    let mut source = ZipArchive::new(open(bundle)?).map_err(|_| error("invalid_client_bundle"))?;
    let temporary = scratch.join("rewritten-bundle.zip");
    let mut target = ZipWriter::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?,
    );
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    target.start_file("manifest.json", options)?;
    target.write_all(&serde_json::to_vec(manifest).map_err(|_| error("serialization_failed"))?)?;
    for i in 0..source.len() {
        let mut entry = source
            .by_index(i)
            .map_err(|_| error("invalid_client_bundle"))?;
        if entry.name() == "manifest.json" {
            continue;
        }
        target.start_file(entry.name(), options)?;
        io::copy(&mut entry, &mut target)?;
    }
    target.finish()?.sync_all()?;
    fs::remove_file(bundle)?;
    fs::rename(temporary, bundle)
}

pub fn expanded_size(bundle: &Path) -> io::Result<u64> {
    let file = open(bundle)?;
    if file.metadata()?.len() > MAX_BUNDLE {
        return Err(error("client_bundle_byte_limit"));
    }
    let mut archive = ZipArchive::new(file).map_err(|_| error("invalid_client_bundle"))?;
    if archive.len() > 2_000_001 {
        return Err(error("client_bundle_entry_limit"));
    }
    let mut total = 0u64;
    let manifest: Manifest = {
        let mut entry = archive
            .by_name("manifest.json")
            .map_err(|_| error("client_bundle_manifest_required"))?;
        if entry.size() > MAX_JSON {
            return Err(error("client_bundle_manifest_limit"));
        }
        let mut raw = Vec::new();
        entry.by_ref().take(MAX_JSON + 1).read_to_end(&mut raw)?;
        serde_json::from_slice(&raw).map_err(|_| error("invalid_client_resource_manifest"))?
    };
    manifest.validate()?;
    let mut sizes = BTreeMap::new();
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|_| error("invalid_client_bundle"))?;
        total = total
            .checked_add(entry.size())
            .ok_or_else(|| error("client_bundle_decompressed_limit"))?;
        if entry.size() > MAX_ENTRY || total > MAX_SCAN {
            return Err(error("client_bundle_decompressed_limit"));
        }
        sizes.insert(entry.name().to_string(), entry.size());
    }
    for layer in &manifest.layers {
        for sha in layer.resources.values() {
            total = total
                .checked_add(
                    *sizes
                        .get(&format!("objects/{sha}"))
                        .ok_or_else(|| error("client_bundle_object_missing"))?,
                )
                .ok_or_else(|| error("client_bundle_decompressed_limit"))?;
            if total > MAX_SCAN {
                return Err(error("client_bundle_decompressed_limit"));
            }
        }
    }
    Ok(total + MAX_JSON)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClientContext {
    pub minecraft_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
}
/// An explicitly chosen loose file carries user-supplied game context, never publisher provenance.
pub fn export_file(
    source: &Path,
    scratch: &Path,
    destination: &Path,
    context: &ClientContext,
    cancel: &dyn Fn() -> bool,
) -> io::Result<Manifest> {
    let root = scratch.join("single-instance");
    fs::create_dir_all(root.join("mods"))?;
    fs::create_dir_all(root.join("resourcepacks"))?;
    let mut dependencies = serde_json::Map::from_iter([(
        "minecraft".into(),
        serde_json::Value::String(context.minecraft_version.clone()),
    )]);
    if context.loader != "vanilla" {
        let key = if context.loader == "fabric" {
            "fabric-loader"
        } else {
            &context.loader
        };
        dependencies.insert(
            key.into(),
            serde_json::Value::String(
                context
                    .loader_version
                    .clone()
                    .ok_or_else(|| error("matching_loader_version_required"))?,
            ),
        );
    }
    fs::write(
        root.join("modrinth.index.json"),
        serde_json::to_vec(&serde_json::json!({"dependencies": dependencies}))
            .map_err(|_| error("serialization_failed"))?,
    )?;
    let raw = read(source, MAX_ARCHIVE)?;
    let folder = if source.extension().is_some_and(|s| s == "jar") {
        "mods"
    } else {
        "resourcepacks"
    };
    let name = if folder == "mods" {
        "selected.jar"
    } else {
        "selected.zip"
    };
    fs::write(root.join(folder).join(name), raw)?;
    let mut manifest = export_instance(&root, scratch, destination, cancel)?;
    // A single file cannot establish the complete client's enabled-pack selection.
    manifest.selection_known = false;
    rewrite_manifest(destination, &manifest, scratch)?;
    Ok(manifest)
}
