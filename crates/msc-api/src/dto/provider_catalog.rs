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
    pub incompatible_install_review: bool,
    #[serde(default)]
    pub creation_pack_setup: bool,
    #[serde(default)]
    pub java_datapacks: Vec<String>,
    #[serde(default)]
    pub source_identification: Vec<String>,
    #[serde(default)]
    pub reviewed_updates: Vec<String>,
    #[serde(default)]
    pub retained_pack_recovery: bool,
    #[serde(default)]
    pub player_content_management: bool,
    #[serde(default)]
    pub player_content_recovery: bool,
    #[serde(default)]
    pub reviewed_pack_updates: bool,
    #[serde(default)]
    pub reviewed_pack_replacement: bool,
    #[serde(default)]
    pub content_publications: bool,
    #[serde(default)]
    pub content_zip_exports: bool,
    #[serde(default)]
    pub content_mrpack_exports: bool,
    #[serde(default)]
    pub content_host_saves: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentInstallReviewRequestDto {
    #[serde(default)]
    pub allow_incompatible: bool,
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
    #[serde(default)]
    pub allow_incompatible: bool,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetainedPackMissingDto {
    pub path: String,
    pub reason: String,
    pub byte_size: Option<u64>,
    pub manual_recovery_available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetainedPackStatusDto {
    pub source_server_pack: Option<bool>,
    pub player_recovery_required: bool,
    pub pack_id: String,
    pub name: String,
    pub version: String,
    pub original_sha512: String,
    pub resolved_file_count: usize,
    pub files_complete: bool,
    pub missing: Vec<RetainedPackMissingDto>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetainedPackManualDto {
    pub path: String,
    pub staged_upload_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentChangeDto {
    pub action: String,
    pub path: String,
    pub sha512: Option<String>,
    pub audience: Option<String>,
    pub optional: Option<bool>,
    pub enabled: Option<bool>,
    pub redistribution_evidence: Option<String>,
    pub staged_upload_id: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentChangesReviewDto {
    pub collection_revision: u64,
    pub changes: Vec<ContentChangeDto>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerFileAdmissionDto {
    pub path: String,
    pub byte_size: u64,
    pub sha512: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentRecoveryReviewDto {
    pub retained_pack_id: Option<String>,
    pub collection_revision: u64,
    pub staged_upload_id: Option<String>,
    #[serde(default)]
    pub selected_paths: Vec<String>,
    #[serde(default)]
    pub restore_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackUpdateReviewDto {
    #[serde(default)]
    pub confirm_world_compatibility: bool,
    #[serde(default)]
    pub preserve_additions: std::collections::BTreeMap<String, String>,
    pub collection_revision: u64,
    pub staged_upload_id: Option<String>,
    pub previous_review_id: Option<String>,
    #[serde(default)]
    pub restore_server: Vec<String>,
    #[serde(default)]
    pub restore_player: Vec<String>,
    #[serde(default)]
    pub audiences: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub confirm_unverified_origin: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationApprovalDto {
    pub audience: String,
    pub path: String,
    pub sha512: String,
    pub redistribution_evidence: String,
    #[serde(default)]
    pub reviewed_configuration: bool,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationExclusionDto {
    pub audience: String,
    pub path: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationDatapackDto {
    pub slot_id: String,
    pub path: String,
    pub sha512: String,
    pub redistribution_evidence: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationReviewRequestDto {
    #[serde(default = "publication_audiences")]
    pub audiences: Vec<String>,
    pub collection_revision: u64,
    pub version_name: Option<String>,
    pub player_setup: Option<PublicationPlayerSetupDto>,
    #[serde(default)]
    pub approvals: Vec<PublicationApprovalDto>,
    #[serde(default)]
    pub excluded: Vec<PublicationExclusionDto>,
    #[serde(default)]
    pub server_paths: Vec<String>,
    #[serde(default)]
    pub datapacks: Vec<PublicationDatapackDto>,
}
fn publication_audiences() -> Vec<String> {
    vec!["players".into(), "server".into()]
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishContentDto {
    pub review_id: String,
    #[serde(default)]
    pub incomplete_consent: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentExportRequestDto {
    pub host_destination: Option<ContentHostDestinationDto>,
    #[serde(default)]
    pub include_mrpack: bool,
    pub publication_id: String,
    pub audience: String,
    #[serde(default)]
    pub allow_incomplete: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicationPlayerSetupDto {
    pub loader: String,
    pub loader_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentHostDestinationDto {
    pub relative_path: String,
    pub replace_sha512: Option<String>,
}
