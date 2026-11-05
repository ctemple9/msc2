//! Additive catalog responses keep provider identity out of legacy ID maps.
use super::{CatalogItemDto, CatalogProjectDetailDto, CatalogVersionDto};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogSearchDto {
    pub provider: String,
    pub results: Vec<CatalogItemDto>,
    pub offset: u32,
    pub total_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogProjectDto {
    pub provider: String,
    pub project: CatalogProjectDetailDto,
    pub distribution_allowed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogVersionsDto {
    pub provider: String,
    pub versions: Vec<ProviderCatalogReleaseDto>,
    pub offset: u32,
    pub total_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogReleaseDto {
    pub version: CatalogVersionDto,
    pub available: bool,
    pub server_pack: bool,
    pub sha1: Option<String>,
    pub manual_download_required: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogAcquireRequestDto {
    pub provider: String,
    pub project_id: String,
    pub version_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogAcquireResultDto {
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentCatalogCapabilityDto {
    pub protocol: u32,
    pub providers: Vec<String>,
    pub pack_acquisition: bool,
    #[serde(default)]
    pub reviewed_installs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentInstallReviewRequestDto {
    pub provider: String,
    pub project_id: u64,
    pub version_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentInstallFileDto {
    pub project_id: String,
    pub version_id: String,
    pub title: String,
    pub file_name: String,
    pub byte_size: u64,
    pub sha1: String,
    pub manual_download_required: bool,
    pub distribution_allowed: Option<bool>,
    pub required_projects: Vec<String>,
    pub incompatible_projects: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentInstallReviewDto {
    pub review_id: String,
    pub server_id: String,
    pub provider: String,
    pub collection_revision: u64,
    pub files: Vec<ContentInstallFileDto>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentInstallApplyDto {
    pub review_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentInstallManualDto {
    pub review_id: String,
    pub version_id: String,
    pub staged_upload_id: Option<String>,
}
