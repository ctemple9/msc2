//! Resource evidence and scoped diagnostics; a resolved model is not visual acceptance.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 1;
pub const RESOLVER_VERSION: &str = "msc-namespaced-inspection-1";
pub const RENDERER_VERSION: &str = "vantage-0.15.1-baseline";

pub const CAPTURE_FORMAT: &str = "msc-contextual-mesh-1";

/// Issued by the agent from an adopted saved scene, never from client names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureRequest {
    pub format: String,
    pub binding: Binding,
    pub geometry_generation_id: String,
    pub resource_generation_id: String,
    pub input_fingerprint: String,
    pub snapshot_id: String,
    /// Exact saved server configuration, datapacks and per-world data receipts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_data_fingerprint: Option<String>,
    pub dimension: String,
    pub area: Area,
    pub context_area: Area,
    pub minecraft_version: String,
    pub loader: String,
    pub loader_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureFile {
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureMaterial {
    pub texture: String,
    pub mode: String,
    pub alpha_threshold: f32,
    pub cull: bool,
    pub depth_write: bool,
    pub directional_lighting: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapturedBlock {
    pub position: [i32; 3],
    pub id: String,
    #[serde(deserialize_with = "unique_capture_map")]
    pub state: BTreeMap<String, String>,
    /// An empty list explicitly captures intentional absence of geometry.
    pub meshes: Vec<CaptureMesh>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureMesh {
    pub file: String,
    pub material: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureManifest {
    pub request: CaptureRequest,
    /// Computed from the exporter's source snapshot/selected inputs, not echoed
    /// from the request. Internal helpers must check these before and after capture.
    pub observed_snapshot_id: String,
    pub observed_input_fingerprint: String,
    pub captured_at_unix: u64,
    pub game_tick: i64,
    pub saved_frame: bool,
    pub directional_lights: [[f32; 3]; 2],
    pub materials: Vec<CaptureMaterial>,
    pub blocks: Vec<CapturedBlock>,
    #[serde(deserialize_with = "unique_capture_map")]
    pub files: BTreeMap<String, CaptureFile>,
}

fn unique_capture_map<'de, D, T>(deserializer: D) -> Result<BTreeMap<String, T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Unique<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Unique<T> {
        type Value = BTreeMap<String, T>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a bounded map without duplicate keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, T>()? {
                if result.len() >= 4096 || result.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom(
                        "duplicate_or_excessive_capture_keys",
                    ));
                }
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Unique(std::marker::PhantomData))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureMeshData {
    /// Absolute world coordinates; normals/UV/colors are captured vertex data.
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub uv: Vec<f32>,
    pub colors: Vec<f32>,
    pub indices: Vec<u32>,
}

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
    CapturedAppearance,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair: Option<RepairEvidence>,
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
pub struct RepairEvidence {
    pub before_operation_id: String,
    pub source_generation_id: String,
    pub target_generation_id: String,
    pub same_saved_area: bool,
    pub before_counts: BTreeMap<Classification, u64>,
    pub after_counts: BTreeMap<Classification, u64>,
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

/// Safe public requirements omit provider URLs, credentials and host paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequiredClientSource {
    pub identity: String,
    pub file: String,
    pub provider: String,
    pub project_id: Option<String>,
    pub release_id: Option<String>,
    pub file_id: Option<String>,
    pub hashes: BTreeMap<String, String>,
    pub bytes: u64,
    pub code: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderingStatus {
    pub state: String,
    pub operation_id: Option<String>,
    pub generation_id: Option<String>,
    pub resource_generation_id: Option<String>,
    pub snapshot_id: Option<String>,
    pub snapshot_at_unix: Option<u64>,
    pub resources_at_unix: Option<u64>,
    pub stale: bool,
    pub retryable: bool,
    pub reason_code: Option<String>,
    pub required_sources: Vec<RequiredClientSource>,
    pub prerequisites: Vec<String>,
    pub affected_tiles: u64,
    pub unchanged_tiles: u64,
    pub note: String,
}
