//! P7.24: `VersionsResponseDTO`/`VersionEntryDTO`,
//! `VersionChangeRequestDTO`/`VersionChangeResultDTO`,
//! `JavaRuntimesResponseDTO`/`JavaRuntimeDTO`,
//! `JavaConfigResponseDTO`/`JavaConfigSetRequestDTO`,
//! `JavaRuntimeInstallRequestDTO`/`JavaRuntimeInstallResultDTO`,
//! `RAMConfigResponseDTO`/`RAMConfigUpdateRequestDTO`/
//! `RAMConfigUpdateResultDTO` — `openapi.json`'s frozen shapes for
//! `GET /v1/versions`, `GET /v1/versions/create`,
//! `POST /v1/components/version`, `GET /v1/java-runtimes`,
//! `GET`/`POST /v1/config/java-runtime`, `GET`/`POST /v1/config/ram`,
//! and `POST /v1/java-runtimes/install`.

use serde::{Deserialize, Serialize};

use super::BedrockRuntimeStateDto;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionEntryDto {
    pub id: String,
    pub display_label: String,
    pub mc_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_label: Option<String>,
    pub is_stable: bool,
    pub is_latest: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionsResponseDto {
    pub supports_versions: bool,
    pub flavor_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<String>,
    pub is_bedrock: bool,
    pub versions: Vec<VersionEntryDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<BedrockRuntimeStateDto>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionChangeRequestDto {
    pub version_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionChangeResultDto {
    pub success: bool,
    pub message: String,
    pub requires_restart: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<BedrockRuntimeStateDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRuntimeDto {
    pub name: String,
    pub executable_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub major_version: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRuntimesResponseDto {
    pub runtimes: Vec<JavaRuntimeDto>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaConfigResponseDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_flags: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaConfigSetRequestDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_flags: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServersRootResponseDto {
    pub path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServersRootSetRequestDto {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostSetupStateDto {
    pub complete: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRuntimeInstallRequestDto {
    pub major: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRuntimeInstallResultDto {
    pub success: bool,
    pub message: String,
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RamConfigResponseDto {
    pub server_name: String,
    pub server_type: String,
    pub min_ram_gb: f64,
    pub max_ram_gb: f64,
    pub physical_ram_gb: i64,
    pub recommended_max_gb: i64,
    pub server_running: bool,
    pub has_active_server: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RamConfigUpdateRequestDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_ram_gb: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_ram_gb: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RamConfigUpdateResultDto {
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_ram_gb: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_ram_gb: Option<f64>,
    pub restart_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
