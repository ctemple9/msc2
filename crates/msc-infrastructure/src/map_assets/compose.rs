//! Resource precedence is explicit: vanilla, mod roots, ordered selected packs, client overrides.
use super::{
    inventory::{Inventory, Resource},
    *,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Winner {
    pub sha256: String,
    pub sources: Vec<String>,
    pub overridden_sources: Vec<String>,
}
#[derive(Default)]
pub struct Stack {
    pub inventory: Inventory,
    pub winners: BTreeMap<String, Winner>,
    pub prerequisites: Vec<String>,
    total_entries: u64,
    total_bytes: u64,
    total_documents: u64,
}
impl Stack {
    /// Same-priority mods are a group, never an alphabetically invented load order.
    pub fn push(&mut self, layer: Inventory, ordered: bool, format: Option<u32>) -> io::Result<()> {
        self.total_entries += layer.entries;
        self.total_bytes += layer.decompressed;
        self.total_documents += layer.documents;
        if self.total_entries > 2_000_000
            || self.total_bytes > MAX_SCAN
            || self.total_documents > 100_000
        {
            return Err(error("resource_stack_budget"));
        }
        let metadata = layer
            .resources
            .get("pack.mcmeta")
            .and_then(|r| r.json.clone());
        if metadata.as_ref().is_some_and(|_| {
            layer
                .resources
                .get("pack.mcmeta")
                .is_some_and(|r| r.invalid)
        }) {
            return Err(error("invalid_pack_metadata"));
        }
        if let Some(meta) = &metadata
            && let Some(filters) = meta
                .pointer("/filter/block")
                .and_then(serde_json::Value::as_array)
        {
            if filters.len() > 128 {
                return Err(error("pack_filter_limit"));
            }
            for filter in filters {
                let ns = filter["namespace"].as_str().unwrap_or(".*");
                let path = filter["path"].as_str().unwrap_or(".*");
                if ns.len() + path.len() > 4096 {
                    return Err(error("pack_filter_limit"));
                }
                let ns = Regex::new(&format!("^(?:{ns})$"))
                    .map_err(|_| error("unsupported_pack_filter"))?;
                let path = Regex::new(&format!("^(?:{path})$"))
                    .map_err(|_| error("unsupported_pack_filter"))?;
                self.inventory.resources.retain(|name, _| {
                    let mut parts = name.splitn(3, '/');
                    parts.next();
                    !parts
                        .next()
                        .zip(parts.next())
                        .is_some_and(|(n, p)| ns.is_match(n) && path.is_match(p))
                });
                self.winners
                    .retain(|name, _| self.inventory.resources.contains_key(name));
            }
        }
        let mut overlays = Vec::new();
        if let Some(meta) = &metadata
            && let Some(entries) = meta
                .pointer("/overlays/entries")
                .and_then(serde_json::Value::as_array)
        {
            if entries.len() > 64 {
                return Err(error("pack_overlay_limit"));
            }
            let Some(format) = format else {
                self.prerequisites
                    .push("pack_format_required_for_overlays".into());
                return Ok(());
            };
            for entry in entries {
                let directory = entry["directory"]
                    .as_str()
                    .ok_or_else(|| error("invalid_pack_overlay"))?;
                if !safe_member(directory) {
                    return Err(error("invalid_pack_overlay"));
                }
                let f = &entry["formats"];
                let supported = if let Some(n) = f.as_u64() {
                    n == format as u64
                } else if let Some(a) = f.as_array() {
                    a.iter().any(|n| n.as_u64() == Some(format as u64))
                } else {
                    f["min_inclusive"]
                        .as_u64()
                        .zip(f["max_inclusive"].as_u64())
                        .is_some_and(|(min, max)| min <= format as u64 && max >= format as u64)
                };
                if supported {
                    overlays.push(format!("{directory}/"));
                }
            }
        }
        let mut resources = layer.resources;
        let names = resources
            .keys()
            .filter(|name| name.starts_with("assets/"))
            .cloned()
            .collect::<Vec<_>>();
        for name in names {
            if let Some(resource) = resources.remove(&name) {
                self.insert(name, resource, ordered);
            }
        }
        for prefix in overlays {
            let names = resources
                .keys()
                .filter(|name| name.starts_with(&format!("{prefix}assets/")))
                .cloned()
                .collect::<Vec<_>>();
            for name in names {
                if let Some(resource) = resources.remove(&name) {
                    self.insert(name[prefix.len()..].into(), resource, true);
                }
            }
        }
        self.inventory.sources.extend(layer.sources);
        self.inventory.file_stamps.extend(layer.file_stamps);
        self.inventory.entries = self.total_entries;
        self.inventory.decompressed = self.total_bytes;
        self.inventory.documents = self.total_documents;
        self.inventory.conflicts = self
            .inventory
            .resources
            .values()
            .filter(|r| r.conflict)
            .count() as u64;
        Ok(())
    }
    fn insert(&mut self, name: String, mut resource: Resource, ordered: bool) {
        let mut overridden = Vec::new();
        if let Some(old) = self.inventory.resources.get(&name) {
            if !ordered && old.sha256 != resource.sha256 {
                resource.conflict = true;
                resource.sources.extend(old.sources.iter().cloned());
            } else {
                overridden.extend(old.sources.clone());
            }
            if let Some(w) = self.winners.get(&name) {
                overridden.extend(w.overridden_sources.clone());
            }
        }
        self.winners.insert(
            name.clone(),
            Winner {
                sha256: resource.sha256.clone(),
                sources: resource.sources.clone(),
                overridden_sources: overridden,
            },
        );
        self.inventory.resources.insert(name, resource);
    }
    pub fn materialize(&self, root: &Path, cancel: &dyn Fn() -> bool) -> io::Result<()> {
        for (name, resource) in &self.inventory.resources {
            poll(cancel)?;
            if resource.conflict {
                return Err(error("client_resource_order_required"));
            }
            if resource.invalid {
                return Err(error("invalid_selected_resource"));
            }
            let object = resource
                .object
                .as_ref()
                .ok_or_else(|| error("resource_bytes_unavailable"))?;
            if !safe_member(name) || file_hash(object, MAX_ENTRY, cancel)? != resource.sha256 {
                return Err(error("resource_object_checksum"));
            }
            let target = root.join(name);
            fs::create_dir_all(
                target
                    .parent()
                    .ok_or_else(|| error("unsafe_resource_path"))?,
            )?;
            fs::copy(object, target)?;
        }
        Ok(())
    }
}
