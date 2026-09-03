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
    /// Controls whether `/v1/addons` performs provider-backed update checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_addon_updates: Option<bool>,
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
    #[serde(rename = "iconURL", default, skip_serializing_if = "Option::is_none")]
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
    pub check_addon_updates: Option<bool>,
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
    #[serde(rename = "iconURL", default, skip_serializing_if = "Option::is_none")]
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
pub struct CatalogGalleryImageDto {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub featured: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogProjectDetailDto {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
    #[serde(rename = "iconURL", default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub downloads: i64,
    pub followers: i64,
    pub server_side: String,
    pub gallery: Vec<CatalogGalleryImageDto>,
    #[serde(rename = "sourceURL", default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(rename = "issuesURL", default, skip_serializing_if = "Option::is_none")]
    pub issues_url: Option<String>,
    #[serde(rename = "wikiURL", default, skip_serializing_if = "Option::is_none")]
    pub wiki_url: Option<String>,
    #[serde(
        rename = "discordURL",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub discord_url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogVersionDependencyDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    pub dependency_type: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogVersionFileDto {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogVersionDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub version_type: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_published: Option<String>,
    pub dependencies: Vec<CatalogVersionDependencyDto>,
    pub files: Vec<CatalogVersionFileDto>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogVersionsResponseDto {
    pub versions: Vec<CatalogVersionDto>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
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
    #[serde(rename = "iconURL", default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(
        rename = "projectURL",
        default,
        skip_serializing_if = "Option::is_none"
    )]
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
pub struct ModpackFileDto {
    pub path: String,
    pub client_only: bool,
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
    pub override_file_count: i64,
    pub files: Vec<ModpackFileDto>,
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

#[cfg(test)]
mod tests {
    //! `#[serde(rename_all = "camelCase")]` turns `icon_url` into `iconUrl`,
    //! not `iconURL` -- it has no concept of an acronym staying uppercase.
    //! The frozen contract (docs/msc2/api-contract/openapi.json) spells
    //! these fields `iconURL`/`projectURL`, matching MSC 1's Swift naming.
    //! P12.7a found this the hard way (a real agent silently sending
    //! `iconUrl`, which the generated `iconURL`-typed TypeScript never
    //! reads) after fixing `GET /v1/catalog/search` to actually populate
    //! the field at all. These guard the explicit `#[serde(rename = ...)]`
    //! overrides against a future refactor dropping them silently.
    use super::*;

    #[test]
    fn addon_item_icon_url_serializes_as_icon_url_acronym() {
        let dto = AddonItemDto {
            icon_url: Some("https://example.test/icon.png".to_string()),
            ..Default::default()
        };
        let value = serde_json::to_value(&dto).unwrap();
        assert_eq!(value["iconURL"], "https://example.test/icon.png");
        assert!(value.get("iconUrl").is_none());
    }

    #[test]
    fn catalog_item_icon_url_serializes_as_icon_url_acronym() {
        let dto = CatalogItemDto {
            icon_url: Some("https://example.test/icon.png".to_string()),
            ..Default::default()
        };
        let value = serde_json::to_value(&dto).unwrap();
        assert_eq!(value["iconURL"], "https://example.test/icon.png");
        assert!(value.get("iconUrl").is_none());
    }

    #[test]
    fn client_export_item_icon_and_project_url_serialize_with_acronym() {
        let dto = ClientExportItemDto {
            icon_url: Some("https://example.test/icon.png".to_string()),
            project_url: Some("https://example.test/project".to_string()),
            ..Default::default()
        };
        let value = serde_json::to_value(&dto).unwrap();
        assert_eq!(value["iconURL"], "https://example.test/icon.png");
        assert_eq!(value["projectURL"], "https://example.test/project");
        assert!(value.get("iconUrl").is_none());
        assert!(value.get("projectUrl").is_none());
    }

    #[test]
    fn catalog_project_detail_urls_serialize_with_acronyms() {
        let dto = CatalogProjectDetailDto {
            icon_url: Some("https://example.test/icon.png".to_string()),
            source_url: Some("https://example.test/source".to_string()),
            issues_url: Some("https://example.test/issues".to_string()),
            wiki_url: Some("https://example.test/wiki".to_string()),
            discord_url: Some("https://example.test/discord".to_string()),
            ..Default::default()
        };
        let value = serde_json::to_value(&dto).unwrap();
        assert_eq!(value["iconURL"], "https://example.test/icon.png");
        assert_eq!(value["sourceURL"], "https://example.test/source");
        assert_eq!(value["issuesURL"], "https://example.test/issues");
        assert_eq!(value["wikiURL"], "https://example.test/wiki");
        assert_eq!(value["discordURL"], "https://example.test/discord");
        assert!(value.get("iconUrl").is_none());
        assert!(value.get("sourceUrl").is_none());
        assert!(value.get("issuesUrl").is_none());
        assert!(value.get("wikiUrl").is_none());
        assert!(value.get("discordUrl").is_none());
    }
}
