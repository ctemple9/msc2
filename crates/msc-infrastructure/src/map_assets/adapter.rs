//! Bijective recognized-field adapter for Vantage's flat resolver; game inputs are never edited.
use super::{inventory::Inventory, resolver::Resolver, *};
use msc_domain::map_assets::{Classification as C, valid_resource_id};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const PREFIX: &str = "__msc_namespace_v1";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaletteResult {
    pub original_id: String,
    pub state: BTreeMap<String, String>,
    pub renderer_id: String,
    pub model_resolved: bool,
    pub classifications: Vec<C>,
    pub dependencies: BTreeSet<String>,
    pub fingerprint: String,
}
pub struct Adapter<'a> {
    pub inventory: &'a Inventory,
}
fn hex(raw: &str) -> String {
    raw.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
}
/// No hash collision is possible: namespace and complete path bytes are encoded separately.
pub fn reference(id: &str) -> io::Result<String> {
    if !valid_resource_id(id) {
        return Err(error("invalid_resource_id"));
    }
    let (ns, path) = id.split_once(':').unwrap_or(("minecraft", id));
    if ns == "minecraft" {
        if path == PREFIX || path.starts_with(&format!("{PREFIX}/")) {
            return Err(error("reserved_adapter_collision"));
        }
        return Ok(path.into());
    }
    Ok(format!("{PREFIX}/{}/{}", hex(ns), hex(path)))
}
pub fn block_id(id: &str) -> io::Result<String> {
    let mapped = reference(id)?;
    if id.split_once(':').is_some_and(|(ns, _)| ns != "minecraft") {
        // Vantage 0.15.1 recognizes `glass` as non-occluding. Conservatively
        // keep all mod geometry non-occluding until mod opacity is evidenced.
        Ok(format!("minecraft:{mapped}_glass"))
    } else {
        Ok(format!("minecraft:{mapped}"))
    }
}
fn rewrite(reference: &mut Value) -> io::Result<()> {
    if let Some(id) = reference.as_str()
        && !id.starts_with('#')
    {
        *reference = Value::String(self::reference(id)?);
    }
    Ok(())
}
fn variants(value: &mut Value) -> io::Result<()> {
    if let Some(array) = value.as_array_mut() {
        for value in array {
            variants(value)?;
        }
    } else if let Some(model) = value.get_mut("model") {
        rewrite(model)?;
    }
    Ok(())
}
/// Unknown fields (including loader payloads and arbitrary strings) stay byte-semantically intact.
pub fn document(path: &str, mut value: Value) -> io::Result<Value> {
    if path.contains("/blockstates/") {
        if let Some(values) = value.get_mut("variants").and_then(Value::as_object_mut) {
            for v in values.values_mut() {
                variants(v)?;
            }
        }
        if let Some(parts) = value.get_mut("multipart").and_then(Value::as_array_mut) {
            for part in parts {
                if let Some(v) = part.get_mut("apply") {
                    variants(v)?;
                }
            }
        }
    } else if path.contains("/models/") {
        if let Some(parent) = value.get_mut("parent")
            && !parent.as_str().is_some_and(|s| {
                matches!(
                    s,
                    "builtin/entity"
                        | "builtin/generated"
                        | "minecraft:builtin/entity"
                        | "minecraft:builtin/generated"
                )
            })
        {
            rewrite(parent)?;
        }
        if let Some(textures) = value.get_mut("textures").and_then(Value::as_object_mut) {
            for value in textures.values_mut() {
                if value.is_string() {
                    rewrite(value)?;
                } else if let Some(sprite) = value.get_mut("sprite") {
                    rewrite(sprite)?;
                }
            }
        }
        if let Some(elements) = value.get_mut("elements").and_then(Value::as_array_mut) {
            for element in elements {
                if let Some(faces) = element.get_mut("faces").and_then(Value::as_object_mut) {
                    for face in faces.values_mut() {
                        if let Some(texture) = face.get_mut("texture") {
                            rewrite(texture)?;
                        }
                    }
                }
            }
        }
    }
    Ok(value)
}
impl Adapter<'_> {
    pub fn materialize(&self, root: &Path, cancel: &dyn Fn() -> bool) -> io::Result<()> {
        for (name, r) in &self.inventory.resources {
            poll(cancel)?;
            if name.starts_with("data/minecraft/worldgen/biome/") && !r.conflict && !r.invalid {
                let object = r
                    .object
                    .as_ref()
                    .ok_or_else(|| error("resource_bytes_unavailable"))?;
                if file_hash(object, MAX_ENTRY, cancel)? != r.sha256 {
                    return Err(error("resource_object_checksum"));
                }
                let target = root
                    .parent()
                    .and_then(Path::parent)
                    .ok_or_else(|| error("unsafe_resource_path"))?
                    .join(name);
                fs::create_dir_all(
                    target
                        .parent()
                        .ok_or_else(|| error("unsafe_resource_path"))?,
                )?;
                fs::copy(object, target)?;
                continue;
            }
            let parts = name.splitn(4, '/').collect::<Vec<_>>();
            if parts.len() != 4
                || parts[0] != "assets"
                || !["blockstates", "models", "textures", "colormaps", "lang"].contains(&parts[2])
            {
                continue;
            }
            // Ambiguous/unusable resources are attributed by palette inspection;
            // they never select arbitrary bytes or abort unrelated models.
            if r.conflict || r.invalid {
                continue;
            }
            let suffix = if matches!(parts[2], "blockstates" | "models" | "lang") {
                ".json"
            } else if parts[3].ends_with(".png.mcmeta") {
                ".png.mcmeta"
            } else if parts[3].ends_with(".png") {
                ".png"
            } else {
                continue;
            };
            let id = format!(
                "{}:{}",
                parts[1],
                parts[3]
                    .strip_suffix(suffix)
                    .ok_or_else(|| error("unsafe_resource_path"))?
            );
            let mut mapped = reference(&id)?;
            if parts[2] == "blockstates" && parts[1] != "minecraft" {
                mapped.push_str("_glass");
            }
            let target = root.join(parts[2]).join(format!("{mapped}{suffix}"));
            fs::create_dir_all(
                target
                    .parent()
                    .ok_or_else(|| error("unsafe_resource_path"))?,
            )?;
            let object = r
                .object
                .as_ref()
                .ok_or_else(|| error("resource_bytes_unavailable"))?;
            if file_hash(object, MAX_ENTRY, cancel)? != r.sha256 {
                return Err(error("resource_object_checksum"));
            }
            if matches!(parts[2], "models" | "blockstates") {
                let value = document(
                    name,
                    r.json
                        .clone()
                        .ok_or_else(|| error("invalid_selected_resource"))?,
                )?;
                fs::write(
                    target,
                    serde_json::to_vec(&value).map_err(|_| error("serialization_failed"))?,
                )?;
            } else {
                fs::copy(object, target)?;
            }
        }
        Ok(())
    }
    pub fn palette(
        &self,
        id: &str,
        state: &BTreeMap<String, String>,
        entity: bool,
    ) -> io::Result<PaletteResult> {
        let findings = Resolver {
            inventory: self.inventory,
        }
        .inspect(id, state, entity);
        let classifications = findings
            .iter()
            .map(|f| f.classification)
            .filter(|c| *c != C::UnsupportedRendererNamespace)
            .collect::<Vec<_>>();

        let supported = classifications
            .iter()
            .all(|c| matches!(c, C::ModelResolved | C::IntentionalEmpty));
        let model_resolved = supported && classifications.contains(&C::ModelResolved);
        let missing_baseline = classifications.iter().any(|c| {
            matches!(
                c,
                C::MissingModel
                    | C::MissingTexture
                    | C::InvalidModel
                    | C::SelectionUnknown
                    | C::UnsupportedLoader
                    | C::UnsupportedMaterial
            )
        });
        let renderer_id = if !supported && (!id.starts_with("minecraft:") || missing_baseline) {
            format!(
                "minecraft:{PREFIX}/fallback/{}_glass",
                hash_json(&(id, state))?
            )
        } else {
            block_id(id)?
        };
        let mut dependencies = BTreeSet::new();
        if let Some(path) = msc_domain::map_assets::resource_path("blockstates", id, "json") {
            self.dependencies(&path, 0, &mut dependencies);
        }
        let hashes = dependencies
            .iter()
            .map(|p| {
                (
                    p,
                    self.inventory
                        .resources
                        .get(p)
                        .map(|r| (&r.sha256, r.conflict, r.invalid)),
                )
            })
            .collect::<Vec<_>>();
        let fingerprint = hash_json(&(id, state, entity, hashes))?;
        Ok(PaletteResult {
            original_id: id.into(),
            state: state.clone(),
            renderer_id,
            model_resolved,
            classifications,
            dependencies,
            fingerprint,
        })
    }
    fn dependencies(&self, path: &str, depth: u32, seen: &mut BTreeSet<String>) {
        if depth >= 32 || !seen.insert(path.into()) {
            return;
        }
        let Some(value) = self
            .inventory
            .resources
            .get(path)
            .and_then(|r| r.json.as_ref())
        else {
            return;
        };
        if path.contains("/blockstates/") {
            let mut values = Vec::new();
            if let Some(v) = value.get("variants").and_then(Value::as_object) {
                values.extend(v.values());
            }
            if let Some(parts) = value.get("multipart").and_then(Value::as_array) {
                values.extend(parts.iter().filter_map(|p| p.get("apply")));
            }
            for value in values {
                let values = value
                    .as_array()
                    .map(|a| a.iter().collect::<Vec<_>>())
                    .unwrap_or_else(|| vec![value]);
                for v in values {
                    if let Some(id) = v["model"].as_str()
                        && let Some(p) = msc_domain::map_assets::resource_path("models", id, "json")
                    {
                        self.dependencies(&p, depth + 1, seen);
                    }
                }
            }
        } else if path.contains("/models/") {
            if let Some(parent) = value["parent"].as_str()
                && let Some(p) = msc_domain::map_assets::resource_path("models", parent, "json")
            {
                self.dependencies(&p, depth + 1, seen);
            }
            if let Some(textures) = value["textures"].as_object() {
                for texture in textures.values() {
                    if let Some(id) = texture.as_str().or_else(|| texture["sprite"].as_str())
                        && !id.starts_with('#')
                        && let Some(p) =
                            msc_domain::map_assets::resource_path("textures", id, "png")
                    {
                        seen.insert(format!("{p}.mcmeta"));
                        seen.insert(p);
                    }
                }
            }
            if let Some(elements) = value["elements"].as_array() {
                for element in elements {
                    if let Some(faces) = element["faces"].as_object() {
                        for face in faces.values() {
                            if let Some(id) = face["texture"].as_str()
                                && !id.starts_with('#')
                                && let Some(p) =
                                    msc_domain::map_assets::resource_path("textures", id, "png")
                            {
                                seen.insert(format!("{p}.mcmeta"));
                                seen.insert(p);
                            }
                        }
                    }
                }
            }
        }
    }
}
