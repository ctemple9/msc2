//! Namespace-preserving inspection of selected variants/multipart models and dependencies.
//! This is the foundation resolver's result, not Vantage's fallback-cube result.
use super::inventory::{Inventory, Resource};
use msc_domain::map_assets::{Classification as C, resource_path};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone)]
pub struct Finding {
    pub classification: C,
    pub detail: String,
    pub source_ids: Vec<String>,
}
pub struct Resolver<'a> {
    pub inventory: &'a Inventory,
}
impl Resolver<'_> {
    pub fn inspect(
        &self,
        id: &str,
        state: &BTreeMap<String, String>,
        entity: bool,
    ) -> Vec<Finding> {
        if matches!(
            id,
            "minecraft:air"
                | "minecraft:cave_air"
                | "minecraft:void_air"
                | "minecraft:barrier"
                | "minecraft:light"
                | "minecraft:structure_void"
        ) {
            return vec![finding(
                C::IntentionalEmpty,
                "Known invisible technical block.",
                vec![],
            )];
        }
        let mut findings = Vec::new();
        if entity {
            findings.push(finding(C::MissingContext,"Block-entity appearance needs snapshot-bound client capture; saved entity data alone cannot execute its renderer.",vec![]));
        }
        let Some(path) = resource_path("blockstates", id, "json") else {
            return vec![finding(
                C::InvalidModel,
                "Invalid original resource identifier.",
                vec![],
            )];
        };
        let Some(document) = self.document(&path, &mut findings) else {
            return findings;
        };
        let mut models = Vec::new();
        if let Some(variants) = document.get("variants").and_then(Value::as_object) {
            let mut matches = 0;
            for (key, variant) in variants {
                let valid = key.is_empty()
                    || key.split(',').all(|part| {
                        part.split_once('=')
                            .is_some_and(|(k, v)| state.get(k).is_some_and(|s| s == v))
                    });
                if valid {
                    matches += 1;
                    select(variant, &mut models, &mut findings);
                }
            }
            if matches != 1 {
                findings.push(finding(
                    C::InvalidModel,
                    "Block state does not select exactly one variant.",
                    vec![],
                ));
            }
        } else if let Some(parts) = document.get("multipart").and_then(Value::as_array) {
            for part in parts {
                match part
                    .get("when")
                    .map(|v| condition(v, state, 0))
                    .unwrap_or(Some(true))
                {
                    Some(true) => select(&part["apply"], &mut models, &mut findings),
                    Some(false) => {}
                    None => findings.push(finding(
                        C::InvalidModel,
                        "Unknown or malformed multipart condition.",
                        vec![],
                    )),
                }
            }
        } else {
            findings.push(finding(
                C::InvalidModel,
                "Blockstate has neither recognized variants nor multipart selectors.",
                vec![],
            ));
        }
        if models.len() > 64 {
            findings.push(finding(
                C::InvalidModel,
                "Selected model reference limit (64).",
                vec![],
            ));
            return findings;
        }
        let mut vertices = false;
        let mut explicit_empty = !models.is_empty();
        for model in models {
            let mut chain = BTreeSet::new();
            let (elements, textures, empty) = self.model(&model, 0, &mut chain, &mut findings);
            explicit_empty &= empty;
            vertices |= elements.iter().any(|element| {
                element
                    .get("faces")
                    .and_then(Value::as_object)
                    .is_some_and(|faces| !faces.is_empty())
            });
            for element in elements {
                let Some(faces) = element.get("faces").and_then(Value::as_object) else {
                    findings.push(finding(
                        C::InvalidModel,
                        "Element has no valid faces.",
                        vec![],
                    ));
                    continue;
                };
                for (face, value) in faces {
                    if !["north", "south", "east", "west", "up", "down"].contains(&face.as_str()) {
                        findings.push(finding(C::InvalidModel, "Unknown element face.", vec![]));
                    }
                    let Some(texture) = value.get("texture").and_then(Value::as_str) else {
                        findings.push(finding(
                            C::MissingTexture,
                            "Face texture is absent.",
                            vec![],
                        ));
                        continue;
                    };
                    self.texture(texture, &textures, &mut findings);
                }
            }
        }
        if findings
            .iter()
            .all(|f| f.classification == C::MissingContext)
        {
            if vertices {
                findings.push(finding(C::ModelResolved,"Selected namespaced geometry and texture dependencies resolve; visual acceptance is separate.",self.inventory.resources[&path].sources.clone()));
            } else if explicit_empty {
                findings.push(finding(
                    C::IntentionalEmpty,
                    "Selected model explicitly declares an empty elements array.",
                    self.inventory.resources[&path].sources.clone(),
                ));
            } else {
                findings.push(finding(C::MissingContext,"No positively identified ordinary geometry; code-generated or entity-only appearance may require client capture.",vec![]));
            }
        }
        if !id.starts_with("minecraft:") {
            findings.push(finding(C::UnsupportedRendererNamespace,"The unchanged Vantage baseline strips namespaces; this inspection generation has not been adopted by a namespace-aware renderer.",self.inventory.resources.get(&path).map(|r|r.sources.clone()).unwrap_or_default()));
        }
        findings
    }
    fn document<'a>(&'a self, path: &str, findings: &mut Vec<Finding>) -> Option<&'a Value> {
        let Some(resource) = self.inventory.resources.get(path) else {
            findings.push(finding(
                C::MissingModel,
                &format!("Missing namespaced resource {path}."),
                vec![],
            ));
            return None;
        };
        if resource.conflict {
            findings.push(finding(
                C::SelectionUnknown,
                "Conflicting resource bytes exist; client pack order has not been established.",
                resource.sources.clone(),
            ));
            return None;
        }
        if resource.invalid || resource.json.is_none() {
            findings.push(finding(
                C::InvalidModel,
                "Selected model/blockstate document is malformed.",
                resource.sources.clone(),
            ));
            return None;
        }
        resource.json.as_ref()
    }
    fn model(
        &self,
        id: &str,
        depth: u32,
        chain: &mut BTreeSet<String>,
        findings: &mut Vec<Finding>,
    ) -> (Vec<Value>, BTreeMap<String, String>, bool) {
        if matches!(id, "builtin/entity" | "minecraft:builtin/entity") {
            findings.push(finding(
                C::MissingContext,
                "Built-in entity model requires a client renderer.",
                vec![],
            ));
            return Default::default();
        }
        if matches!(id, "builtin/generated" | "minecraft:builtin/generated") {
            findings.push(finding(
                C::UnsupportedMaterial,
                "Generated item geometry is not an ordinary block model.",
                vec![],
            ));
            return Default::default();
        }
        let Some(path) = resource_path("models", id, "json") else {
            findings.push(finding(C::InvalidModel, "Unsafe model identifier.", vec![]));
            return Default::default();
        };
        if depth >= 32 || !chain.insert(path.clone()) {
            findings.push(finding(
                C::InvalidModel,
                "Model parent depth or cycle limit.",
                vec![],
            ));
            return Default::default();
        }
        let Some(document) = self.document(&path, findings) else {
            return Default::default();
        };
        if let Some(loader) = document.get("loader") {
            findings.push(finding(C::UnsupportedLoader,&format!("Custom model loader {loader} requires matching-client capture or a pinned adapter."),self.inventory.resources[&path].sources.clone()));
            chain.remove(&path);
            return Default::default();
        }
        let (mut elements, mut textures, mut empty) = document
            .get("parent")
            .and_then(Value::as_str)
            .map(|p| self.model(p, depth + 1, chain, findings))
            .unwrap_or_default();
        if let Some(values) = document.get("textures").and_then(Value::as_object) {
            for (k, v) in values {
                if let Some(value) = v.as_str() {
                    textures.insert(k.clone(), value.into());
                } else {
                    findings.push(finding(
                        C::InvalidModel,
                        "Non-string texture binding.",
                        vec![],
                    ));
                }
            }
        }
        if let Some(value) = document.get("elements") {
            if let Some(values) = value.as_array() {
                if values.len() > 4096 {
                    findings.push(finding(
                        C::InvalidModel,
                        "Model element limit (4096).",
                        vec![],
                    ));
                    chain.remove(&path);
                    return Default::default();
                }
                empty = values.is_empty();
                elements = values.clone();
                for element in &elements {
                    for key in ["from", "to"] {
                        if !element.get(key).and_then(Value::as_array).is_some_and(|v| {
                            v.len() == 3
                                && v.iter().all(|n| {
                                    n.as_f64().is_some_and(|n| {
                                        n.is_finite() && (-16.0..=32.0).contains(&n)
                                    })
                                })
                        }) {
                            findings.push(finding(
                                C::InvalidModel,
                                "Element bounds are absent or invalid.",
                                vec![],
                            ));
                        }
                    }
                }
            } else {
                findings.push(finding(
                    C::InvalidModel,
                    "Elements is not an array.",
                    vec![],
                ));
            }
        }
        for key in document.as_object().into_iter().flat_map(|o| o.keys()) {
            if ![
                "parent",
                "textures",
                "elements",
                "ambientocclusion",
                "display",
                "gui_light",
                "render_type",
                "forge_marker",
                "credit",
            ]
            .contains(&key.as_str())
            {
                findings.push(finding(
                    C::UnsupportedMaterial,
                    &format!(
                        "Unrecognized model field {key}; its semantics are not silently ignored."
                    ),
                    vec![],
                ));
            }
        }
        if document.get("render_type").is_some() {
            findings.push(finding(
                C::UnsupportedMaterial,
                "Explicit model render type needs material support verification.",
                vec![],
            ));
        }
        chain.remove(&path);
        (elements, textures, empty)
    }
    fn texture(
        &self,
        reference: &str,
        textures: &BTreeMap<String, String>,
        findings: &mut Vec<Finding>,
    ) {
        let mut value = reference;
        let mut seen = BTreeSet::new();
        for depth in 0..=16 {
            if !value.starts_with('#') {
                break;
            }
            if depth == 16 || !seen.insert(value) {
                findings.push(finding(
                    C::MissingTexture,
                    "Texture variable cycle or depth limit.",
                    vec![],
                ));
                return;
            }
            let Some(next) = textures.get(&value[1..]) else {
                findings.push(finding(
                    C::MissingTexture,
                    "Unbound texture variable.",
                    vec![],
                ));
                return;
            };
            value = next;
        }
        let Some(path) = resource_path("textures", value, "png") else {
            findings.push(finding(
                C::MissingTexture,
                "Invalid texture identifier.",
                vec![],
            ));
            return;
        };
        match self.inventory.resources.get(&path) {
            None => findings.push(finding(
                C::MissingTexture,
                &format!("Missing namespaced texture {path}."),
                vec![],
            )),
            Some(Resource {
                conflict: true,
                sources,
                ..
            }) => findings.push(finding(
                C::SelectionUnknown,
                "Texture overrides require explicit pack order.",
                sources.clone(),
            )),
            Some(Resource {
                invalid: true,
                sources,
                ..
            }) => findings.push(finding(
                C::MissingTexture,
                "Texture is invalid or exceeds decoded image limits.",
                sources.clone(),
            )),
            _ => {}
        }
    }
}
fn finding(classification: C, detail: &str, source_ids: Vec<String>) -> Finding {
    Finding {
        classification,
        detail: detail.into(),
        source_ids,
    }
}
fn select(value: &Value, models: &mut Vec<String>, findings: &mut Vec<Finding>) {
    let value = if let Some(array) = value.as_array() {
        if array.len() != 1 {
            findings.push(finding(
                C::UnsupportedMaterial,
                "Weighted model selection is not verified for the baseline renderer.",
                vec![],
            ));
            return;
        }
        &array[0]
    } else {
        value
    };
    if let Some(model) = value.get("model").and_then(Value::as_str) {
        models.push(model.into());
    } else {
        findings.push(finding(
            C::InvalidModel,
            "Selected variant has no model identifier.",
            vec![],
        ));
    }
}
fn condition(value: &Value, state: &BTreeMap<String, String>, depth: u32) -> Option<bool> {
    if depth > 16 {
        return None;
    }
    let object = value.as_object()?;
    let mut result = true;
    for (k, v) in object {
        let matches = if k == "OR" || k == "AND" {
            let children = v.as_array()?;
            let values = children
                .iter()
                .map(|v| condition(v, state, depth + 1))
                .collect::<Option<Vec<_>>>()?;
            if k == "OR" {
                values.into_iter().any(|v| v)
            } else {
                values.into_iter().all(|v| v)
            }
        } else {
            let values = v.as_str()?;
            state
                .get(k)
                .is_some_and(|s| values.split('|').any(|v| v == s))
        };
        result &= matches;
    }
    Some(result)
}
