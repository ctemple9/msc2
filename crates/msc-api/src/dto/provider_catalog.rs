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
    #[serde(default)]
    pub source_identification: Vec<String>,
    #[serde(default)]
    pub reviewed_updates: Vec<String>,
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
    #[serde(default)]
    pub replacements: std::collections::BTreeMap<String, String>,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSourceDto {
    pub provider: String,
    pub project_id: String,
    pub version_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentIdentityDto {
    pub path: String,
    pub sha512: Option<String>,
    pub byte_size: u64,
    pub source: Option<ContentSourceDto>,
    pub candidates: Vec<ContentSourceDto>,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentIdentificationDto {
    pub items: Vec<ContentIdentityDto>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentSourceLinkDto {
    pub path: String,
    pub sha512: String,
    pub project_id: Option<u64>,
    pub version_id: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentUpdateDto {
    pub path: String,
    pub provider: String,
    pub project_id: String,
    pub installed_version_id: Option<String>,
    pub available_version_id: Option<String>,
    pub label: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentUpdatesDto {
    pub items: Vec<ContentUpdateDto>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentUpdateReviewRequestDto {
    pub path: String,
    pub sha512: String,
    pub project_id: u64,
    pub version_id: u64,
}
