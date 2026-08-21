//! P8.24: DTOs for the Phase 8 public add-on/component/modpack routes.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentStatusDto {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_build: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_build: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    pub is_up_to_date: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updatable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentsStatusDto {
    pub components: Vec<ComponentStatusDto>,
    pub restart_required_to_apply: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentUpdateRequestDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jar_stem: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_all: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove_source: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentUpdateResultDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_build: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_version: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonItemDto {
    pub jar_stem: String,
    pub display_name: String,
    pub is_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_version: Option<String>,
    pub bucket: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonsResponseDto {
    pub addons: Vec<AddonItemDto>,
    pub is_resolving: bool,
    pub server_supports_addons: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_managed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonUpdateResultDto {
    pub result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jar_stem: Option<String>,
    pub count: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonRemoveRequestDto {
    pub jar_stem: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonRemoveResultDto {
    pub success: bool,
    pub message: String,
    pub jar_stem: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItemDto {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub downloads: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub is_client_only: bool,
    pub project_type: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSearchResponseDto {
    pub supports_addons: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addon_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_version: Option<String>,
    pub results: Vec<CatalogItemDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogInstallRequestDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staged_upload_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogInstallResultDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub installed_dependencies: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientExportItemDto {
    pub id: String,
    pub file_name: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_url: Option<String>,
    pub client_status: String,
    pub status_source: String,
    pub selected_by_default: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientExportResponseDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    pub server_type: String,
    pub export_kind: String,
    pub is_paper_like: bool,
    pub items: Vec<ClientExportItemDto>,
    pub selected_count: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zip_file_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staged_download_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackManualFileDto {
    pub file_id: String,
    pub file_name: String,
    pub project_name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackInspectionRequestDto {
    pub staged_upload_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackInspectionResultDto {
    pub success: bool,
    pub message: String,
    pub format: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecraft_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    pub file_count: i64,
    pub client_only_file_count: i64,
    pub manual_files: Vec<ModpackManualFileDto>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackImportRequestDto {
    pub staged_upload_id: String,
    pub action: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackImportResultDto {
    pub success: bool,
    pub message: String,
    pub operation_id: String,
    pub pending_manual_files: Vec<ModpackManualFileDto>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackManualFileRequestDto {
    pub file_id: String,
    pub staged_upload_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackManualFileResultDto {
    pub success: bool,
    pub message: String,
    pub operation_id: String,
    pub remaining_manual_files: Vec<ModpackManualFileDto>,
    pub all_files_resolved: bool,
}
