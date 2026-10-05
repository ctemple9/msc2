//! Additive read-only map-resource contract, shared with the host-local CLI.
pub use msc_domain::map_assets::{
    Area as MapAssetsAreaDto, Binding as MapAssetsBindingDto,
    CaptureManifest as MapCaptureManifestDto, CaptureRequest as MapCaptureRequestDto,
    CheckStarted as MapAssetsCheckStartedDto, Report as MapAssetsReportDto,
    Status as MapAssetsStatusDto,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapAssetsCheckRequestDto {
    pub server_id: String,
    pub expected_revision: String,
    pub dimension: String,
    pub area: MapAssetsAreaDto,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAssetsCapabilitiesDto {
    pub schema_version: u32,
    pub actions: Vec<String>,
    pub resource_formats: Vec<String>,
    pub capture_formats: Vec<String>,
    pub renderer_adoption: bool,
}

pub use msc_domain::map_assets::{
    RenderingStatus as MapRenderingStatusDto, RequiredClientSource as MapRequiredClientSourceDto,
};
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapAssetsPrepareRequestDto {
    pub server_id: String,
    pub expected_revision: String,
    pub dimension: String,
    pub area: Option<MapAssetsAreaDto>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapAssetsImportRequestDto {
    pub server_id: String,
    pub expected_revision: String,
    pub dimension: String,
    pub area: Option<MapAssetsAreaDto>,
    pub staged_upload_id: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAssetsClientContextDto {
    pub minecraft_version: Option<String>,
    pub loader: String,
    pub loader_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAssetsSelectionDto {
    pub binding: MapAssetsBindingDto,
    pub selection_revision: Option<String>,
    pub manifest: Option<serde_json::Value>,
    pub previous_generation: Option<String>,
    pub note: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapAssetsSelectionRequestDto {
    pub server_id: String,
    pub expected_revision: String,
    pub expected_selection_revision: String,
    pub dimension: String,
    pub area: Option<MapAssetsAreaDto>,
    pub selected_packs: Vec<String>,
    pub mod_order: Option<Vec<String>>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapAssetsRestoreRequestDto {
    pub server_id: String,
    pub expected_revision: String,
    pub dimension: String,
    pub area: Option<MapAssetsAreaDto>,
    pub generation: String,
}
