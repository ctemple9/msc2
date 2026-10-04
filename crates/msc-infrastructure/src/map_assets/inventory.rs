use super::{store::Candidate, *};
use msc_domain::map_assets::{DeclaredMod, SourceEvidence};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Seek};
use std::path::PathBuf;
use zip::ZipArchive;

#[derive(Default)]
pub struct Resource {
    pub sha256: String,
    pub sources: Vec<String>,
    pub json: Option<Value>,
    pub invalid: bool,
    pub conflict: bool,
    pub bytes: u64,
}
#[derive(Default)]
pub struct Inventory {
    pub sources: Vec<SourceEvidence>,
    pub resources: BTreeMap<String, Resource>,
    pub entries: u64,
    pub decompressed: u64,
    pub documents: u64,
    pub conflicts: u64,
    pub file_stamps: BTreeMap<PathBuf, String>,
    document_bytes: u64,
    nested: u64,
}
impl Inventory {
    pub fn scan_mods(
        &mut self,
        directory: &Path,
        candidate: &mut Candidate,
        cancel: &dyn Fn() -> bool,
    ) -> io::Result<()> {
        if !directory.exists() {
            return Ok(());
        }
        safe_path(directory)?;
        let mut paths = fs::read_dir(directory)?
            .take(10_001)
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()?;
        if paths.len() > 10_000 {
            return Err(error("inventory_source_limit"));
        }
        paths.sort();
        for path in paths {
            poll(cancel)?;
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| error("invalid_mod_name"))?;
            let name = name.to_ascii_lowercase();
            let enabled = name.ends_with(".jar");
            if !enabled && !name.ends_with(".jar.disabled") {
                continue;
            }
            let (stored, sha, bytes) = candidate.copy(&path, cancel)?;
            self.archive(open(&stored)?, sha, bytes, enabled, None, 0, cancel)?;
        }
        Ok(())
    }
    pub fn scan_archive(
        &mut self,
        path: &Path,
        kind: &str,
        candidate: &mut Candidate,
        cancel: &dyn Fn() -> bool,
    ) -> io::Result<()> {
        let (stored, sha, bytes) = candidate.copy(path, cancel)?;
        let root_source = self.sources.len();
        self.archive(open(&stored)?, sha, bytes, true, None, 0, cancel)?;
        if let Some(source) = self.sources.get_mut(root_source) {
            source.kind = kind.into();
            source.metadata_issue = None;
        }
        Ok(())
    }
    pub fn scan_assets(&mut self, root: &Path, cancel: &dyn Fn() -> bool) -> io::Result<()> {
        if !root.exists() {
            return Ok(());
        }
        safe_path(root)?;
        let source_id = hash(b"existing-minecraft-client-cache");
        let starting_bytes = self.decompressed;
        let mut stack = vec![root.to_path_buf()];
        let mut receipt = BTreeMap::new();
        while let Some(directory) = stack.pop() {
            self.file_stamps
                .insert(directory.clone(), stamp(&fs::symlink_metadata(&directory)?));
            for e in fs::read_dir(directory)? {
                poll(cancel)?;
                let e = e?;
                self.entries += 1;
                if self.entries > 2_000_000 {
                    return Err(error("archive_entry_limit"));
                }
                let m = fs::symlink_metadata(e.path())?;
                if m.is_dir() {
                    safe_path(&e.path())?;
                    stack.push(e.path());
                    continue;
                }
                if !m.is_file() {
                    return Err(error("linked_input"));
                }
                let relative = e
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| error("asset_path"))?
                    .to_string_lossy()
                    .replace('\\', "/");
                let name = format!("assets/minecraft/{relative}");
                if !safe_member(&name) {
                    return Err(error("unsafe_archive_path"));
                }
                let limit = if name.ends_with(".json") {
                    MAX_JSON
                } else {
                    MAX_ENTRY
                };
                let before = stamp(&m);
                let raw = read(&e.path(), limit)?;
                if before != stamp(&fs::symlink_metadata(e.path())?) {
                    return Err(error("input_changed"));
                }
                self.file_stamps.insert(e.path(), before);
                self.decompressed += raw.len() as u64;
                if self.decompressed > MAX_SCAN {
                    return Err(error("decompressed_scan_limit"));
                }
                receipt.insert(name.clone(), hash(&raw));
                self.add_resource(&name, &source_id, &raw)?;
            }
        }
        if !receipt.is_empty() {
            self.sources.push(SourceEvidence {
                id: source_id,
                kind: "existing_client_cache".into(),
                enabled: true,
                sha256: hash_json(&receipt)?,
                bytes: self.decompressed - starting_bytes,
                evidence: "local_hashed".into(),
                declared_mods: vec![],
                namespaces: vec!["minecraft".into()],
                parent_source: None,
                provider: None,
                project_id: None,
                release_id: None,
                file_id: None,
                metadata_issue: None,
            });
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    fn archive<R: Read + Seek>(
        &mut self,
        input: R,
        sha: String,
        bytes: u64,
        enabled: bool,
        parent: Option<String>,
        depth: u32,
        cancel: &dyn Fn() -> bool,
    ) -> io::Result<()> {
        if self.sources.len() >= 10_000 {
            return Err(error("inventory_source_limit"));
        }
        let id = hash_json(&(&sha, enabled, &parent))?;
        let mut archive = ZipArchive::new(input).map_err(|_| error("invalid_archive"))?;
        if archive.len() > 200_000 {
            return Err(error("archive_entry_limit"));
        }
        let mut names = BTreeSet::new();
        let mut namespaces = BTreeSet::new();
        let mut metadata = BTreeMap::new();
        let mut nested_names = Vec::new();
        for index in 0..archive.len() {
            poll(cancel)?;
            self.entries += 1;
            if self.entries > 2_000_000 {
                return Err(error("archive_entry_limit"));
            }
            let mut entry = archive
                .by_index(index)
                .map_err(|_| error("invalid_archive_entry"))?;
            let name = entry.name().to_string();
            if !safe_member(&name) || !names.insert(name.trim_end_matches('/').to_ascii_lowercase())
            {
                return Err(error("unsafe_or_duplicate_archive_path"));
            }
            if let Some(mode) = entry.unix_mode() {
                let kind = mode & 0o170000;
                if ![0, 0o100000, 0o040000].contains(&kind) {
                    return Err(error("archive_link_or_special_file"));
                }
            }
            if entry.is_dir() {
                continue;
            }
            if entry.size() > MAX_ENTRY {
                return Err(error("decompressed_entry_limit"));
            }
            self.decompressed = self
                .decompressed
                .checked_add(entry.size())
                .ok_or_else(|| error("decompressed_scan_limit"))?;
            if self.decompressed > MAX_SCAN {
                return Err(error("decompressed_scan_limit"));
            }
            let wanted = name.starts_with("assets/")
                || matches!(
                    name.as_str(),
                    "fabric.mod.json"
                        | "META-INF/neoforge.mods.toml"
                        | "META-INF/mods.toml"
                        | "META-INF/jarjar/metadata.json"
                );
            if wanted && name.ends_with(".json") && entry.size() > MAX_JSON {
                return Err(error("json_document_limit"));
            }
            if matches!(
                name.as_str(),
                "META-INF/neoforge.mods.toml" | "META-INF/mods.toml"
            ) && entry.size() > MAX_JSON
            {
                return Err(error("json_document_limit"));
            }
            let mut raw = Vec::new();
            if wanted {
                entry.by_ref().take(MAX_ENTRY + 1).read_to_end(&mut raw)?;
                if raw.len() as u64 != entry.size() {
                    return Err(error("archive_entry_size_mismatch"));
                }
            } else {
                let copied = io::copy(&mut entry.by_ref().take(MAX_ENTRY + 1), &mut io::sink())?;
                if copied != entry.size() {
                    return Err(error("archive_entry_size_mismatch"));
                }
            }
            if name.starts_with("assets/") {
                if let Some(namespace) = name.split('/').nth(1)
                    && msc_domain::map_assets::valid_resource_id(&format!("{namespace}:x"))
                {
                    namespaces.insert(namespace.into());
                }
                if enabled {
                    self.add_resource(&name, &id, &raw)?;
                }
            } else if wanted {
                metadata.insert(name, raw);
            }
        }
        let (mods, issue) = declared_mods(&metadata);
        if enabled {
            if let Some(raw) = metadata.get("fabric.mod.json") {
                let value: Value =
                    serde_json::from_slice(raw).map_err(|_| error("malformed_nested_metadata"))?;
                if let Some(jars) = value["jars"].as_array() {
                    for jar in jars {
                        nested_names.push(
                            jar["file"]
                                .as_str()
                                .ok_or_else(|| error("malformed_nested_metadata"))?
                                .to_string(),
                        );
                    }
                }
            }
            if let Some(raw) = metadata.get("META-INF/jarjar/metadata.json") {
                let value: Value =
                    serde_json::from_slice(raw).map_err(|_| error("malformed_nested_metadata"))?;
                if let Some(jars) = value["jars"].as_array() {
                    for jar in jars {
                        nested_names.push(
                            jar["path"]
                                .as_str()
                                .ok_or_else(|| error("malformed_nested_metadata"))?
                                .to_string(),
                        );
                    }
                }
            }
        }
        self.sources.push(SourceEvidence {
            id: id.clone(),
            kind: "mod_jar".into(),
            enabled,
            sha256: sha,
            bytes,
            evidence: "local_hashed".into(),
            declared_mods: mods,
            namespaces: namespaces.into_iter().collect(),
            parent_source: parent,
            provider: None,
            project_id: None,
            release_id: None,
            file_id: None,
            metadata_issue: issue,
        });
        let mut declared = BTreeSet::new();
        for name in nested_names {
            poll(cancel)?;
            self.nested += 1;
            if depth >= 4 || self.nested > 1024 {
                return Err(error("nested_jar_limit"));
            }
            if !safe_member(&name) || !name.ends_with(".jar") || !declared.insert(name.clone()) {
                return Err(error("invalid_nested_jar_declaration"));
            }
            let mut entry = archive
                .by_name(&name)
                .map_err(|_| error("declared_nested_jar_missing"))?;
            if entry.size() > MAX_ENTRY {
                return Err(error("decompressed_entry_limit"));
            }
            let mut raw = Vec::new();
            entry.by_ref().take(MAX_ENTRY + 1).read_to_end(&mut raw)?;
            if raw.len() as u64 != entry.size() {
                return Err(error("archive_entry_size_mismatch"));
            }
            let size = raw.len() as u64;
            let sha = hash(&raw);
            self.archive(
                Cursor::new(raw),
                sha,
                size,
                true,
                Some(id.clone()),
                depth + 1,
                cancel,
            )?;
        }
        Ok(())
    }
    fn add_resource(&mut self, name: &str, id: &str, raw: &[u8]) -> io::Result<()> {
        if !(name.ends_with(".json")
            && (name.contains("/models/") || name.contains("/blockstates/")))
            && !name.contains("/textures/")
        {
            return Ok(());
        }
        let sha = hash(raw);
        if let Some(existing) = self.resources.get_mut(name) {
            if !existing.sources.contains(&id.to_string()) {
                existing.sources.push(id.into());
            }
            if existing.sha256 != sha && !existing.conflict {
                existing.conflict = true;
                self.conflicts += 1;
            }
            return Ok(());
        }
        let mut resource = Resource {
            sha256: sha,
            sources: vec![id.into()],
            bytes: raw.len() as u64,
            ..Default::default()
        };
        if name.ends_with(".json") {
            if raw.len() as u64 > MAX_JSON {
                return Err(error("json_document_limit"));
            }
            self.document_bytes += raw.len() as u64;
            if self.document_bytes > MAX_DOCUMENTS {
                return Err(error("resource_document_budget"));
            }
            self.documents += 1;
            match serde_json::from_slice(raw) {
                Ok(value) => resource.json = Some(value),
                Err(_) => resource.invalid = true,
            }
        } else if name.ends_with(".png") {
            resource.invalid = !valid_png(raw);
        }
        self.resources.insert(name.into(), resource);
        Ok(())
    }
}
fn declared_mods(metadata: &BTreeMap<String, Vec<u8>>) -> (Vec<DeclaredMod>, Option<String>) {
    if let Some(raw) = metadata.get("fabric.mod.json") {
        if let Ok(value) = serde_json::from_slice::<Value>(raw)
            && let Some(id) = value["id"].as_str()
        {
            return (
                vec![DeclaredMod {
                    id: id.into(),
                    version: value["version"].as_str().map(str::to_string),
                }],
                None,
            );
        }
        return (vec![], Some("malformed_mod_metadata".into()));
    }
    for name in ["META-INF/neoforge.mods.toml", "META-INF/mods.toml"] {
        if let Some(raw) = metadata.get(name) {
            let value = std::str::from_utf8(raw)
                .ok()
                .and_then(|s| toml::from_str::<toml::Value>(s).ok());
            if let Some(value) = value
                && let Some(mods) = value.get("mods").and_then(toml::Value::as_array)
            {
                let mods: Vec<_> = mods
                    .iter()
                    .filter_map(|m| {
                        m.get("modId")
                            .and_then(toml::Value::as_str)
                            .map(|id| DeclaredMod {
                                id: id.into(),
                                version: m
                                    .get("version")
                                    .and_then(toml::Value::as_str)
                                    .filter(|v| !v.contains("${"))
                                    .map(str::to_string),
                            })
                    })
                    .collect();
                if !mods.is_empty() {
                    return (mods, None);
                }
            }
            return (vec![], Some("malformed_mod_metadata".into()));
        }
    }
    (vec![], Some("unknown_mod_identity".into()))
}
fn valid_png(raw: &[u8]) -> bool {
    let mut decoder = png::Decoder::new(Cursor::new(raw));
    decoder.set_limits(png::Limits {
        bytes: 256 * 1024 * 1024,
    });
    let Ok(mut reader) = decoder.read_info() else {
        return false;
    };
    let info = reader.info();
    if info.width == 0
        || info.height == 0
        || info.width > 8192
        || info.height > 8192
        || u64::from(info.width) * u64::from(info.height) * 4 > MAX_ENTRY
    {
        return false;
    }
    let Some(size) = reader
        .output_buffer_size()
        .filter(|size| *size as u64 <= MAX_ENTRY)
    else {
        return false;
    };
    reader.next_frame(&mut vec![0; size]).is_ok()
}

pub fn source_paths(root: &Path) -> io::Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(vec![]);
    }
    safe_path(root)?;
    let paths = fs::read_dir(root)?
        .take(10_001)
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()?;
    if paths.len() > 10_000 {
        return Err(error("inventory_source_limit"));
    }
    Ok(paths)
}
