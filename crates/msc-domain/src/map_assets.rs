//! Resource evidence and scoped diagnostics; a resolved model is not visual acceptance.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 1;
pub const RESOLVER_VERSION: &str = "msc-namespaced-inspection-1";
pub const RENDERER_VERSION: &str = "vantage-0.15.1-baseline";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    pub agent_host_id: String,
    pub server_id: String,
    pub slot_id: String,
    pub world_incarnation: String,
    pub revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Area {
    pub min: [i32; 3],
    pub max: [i32; 3],
}
impl Area {
    pub fn validate(&self) -> Result<(), &'static str> {
        let mut blocks = 1u64;
        for axis in 0..3 {
            if self.min[axis] > self.max[axis] {
                return Err("invalid_area");
            }
            let width = i64::from(self.max[axis]) - i64::from(self.min[axis]) + 1;
            blocks = blocks.checked_mul(width as u64).ok_or("area_block_limit")?;
        }
        if blocks > 262_144 || self.min[1] < -2048 || self.max[1] > 2047 {
            return Err("area_block_limit");
        }
        let mut chunks = 1u64;
        for axis in [0, 2] {
            if self.min[axis] < -30_000_000 || self.max[axis] >= 30_000_000 {
                return Err("invalid_area");
            }
            chunks *= (i64::from(self.max[axis].div_euclid(16))
                - i64::from(self.min[axis].div_euclid(16))
                + 1) as u64;
        }
        if chunks > 16 {
            return Err("area_chunk_limit");
        }
        Ok(())
    }
    pub fn contains(&self, p: [i32; 3]) -> bool {
        (0..3).all(|axis| p[axis] >= self.min[axis] && p[axis] <= self.max[axis])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclaredMod {
    pub id: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceEvidence {
    pub id: String,
    pub kind: String,
    pub enabled: bool,
    pub sha256: String,
    pub bytes: u64,
    pub evidence: String,
    pub declared_mods: Vec<DeclaredMod>,
    pub namespaces: Vec<String>,
    pub parent_source: Option<String>,
    pub provider: Option<String>,
    pub project_id: Option<String>,
    pub release_id: Option<String>,
    pub file_id: Option<String>,
    pub metadata_issue: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceManifest {
    pub schema_version: u32,
    pub generation_id: String,
    pub minecraft_version: Option<String>,
    pub loader: String,
    pub loader_version: Option<String>,
    pub resolver_version: String,
    pub renderer_version: String,
    pub capture_formats: Vec<String>,
    pub sources: Vec<SourceEvidence>,
    pub selected_pack_order: Option<Vec<String>>,
    pub selection_known: bool,
    pub selection_revision: String,
    pub config_fingerprint: String,
    pub manifest_receipts: BTreeMap<String, String>,
    pub archive_entries: u64,
    pub decompressed_bytes: u64,
    pub resource_documents: u64,
    pub resource_conflicts: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    MissingModel,
    MissingTexture,
    UnsupportedLoader,
    UnsupportedMaterial,
    MissingContext,
    MissingSavedChunk,
    IntentionalEmpty,
    ModelResolved,
    SelectionUnknown,
    UnsupportedRendererNamespace,
    InvalidModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub classification: Classification,
    pub original_id: String,
    pub state: BTreeMap<String, String>,
    pub detail: String,
    pub source_ids: Vec<String>,
    pub block_occurrences: u64,
    pub samples: Vec<[i32; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub schema_version: u32,
    pub binding: Binding,
    pub snapshot_id: String,
    pub snapshot_minecraft_version: Option<String>,
    pub resource_generation_id: String,
    pub geometry_generation_id: Option<String>,
    pub dimension: String,
    pub area: Area,
    pub operation_id: String,
    pub outcome: String,
    pub visual_acceptance: String,
    pub scope: String,
    pub inspected_blocks: u64,
    pub inspected_chunks: u64,
    pub distinct_states: u64,
    /// Inspection does not calculate visibility; absence is never a zero-face claim.
    pub visible_faces: Option<u64>,
    pub counts: BTreeMap<Classification, u64>,
    pub diagnostics: Vec<Diagnostic>,
    pub omitted_issues: u64,
    pub omitted_samples: u64,
    pub sources: Vec<SourceEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub schema_version: u32,
    pub binding: Binding,
    pub state: String,
    pub input_generation: Option<String>,
    pub previous_generation: Option<String>,
    pub report_available: bool,
    pub renderer_adopted: bool,
    pub actions: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckStarted {
    pub result: String,
    pub operation_id: String,
    pub binding: Binding,
}

pub fn valid_resource_id(id: &str) -> bool {
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    id.len() <= 1024
        && !namespace.is_empty()
        && !path.is_empty()
        && namespace
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && path
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-/".contains(&c))
}

pub fn resource_path(kind: &str, id: &str, suffix: &str) -> Option<String> {
    if !valid_resource_id(id) {
        return None;
    }
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    Some(format!("assets/{namespace}/{kind}/{path}.{suffix}"))
}
