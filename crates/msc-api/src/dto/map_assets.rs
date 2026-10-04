//! Additive read-only map-resource contract, shared with the host-local CLI.
pub use msc_domain::map_assets::{
    Area as MapAssetsAreaDto, Binding as MapAssetsBindingDto,
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
