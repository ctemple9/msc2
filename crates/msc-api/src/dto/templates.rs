//! P7.23: `TemplateItemDTO`/`TemplatesResponseDTO`/
//! `TemplateMutationRequestDTO`/`TemplateMutationResultDTO` — `openapi.json`'s
//! frozen shapes for `GET`/`POST /v1/templates`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateItemDto {
    pub id: String,
    pub kind: String,
    pub filename: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplatesResponseDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    pub server_running: bool,
    pub paper_templates: Vec<TemplateItemDto>,
    pub plugin_templates: Vec<TemplateItemDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateMutationRequestDto {
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_cross_play: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_play_bedrock_port: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_playit: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gamemode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accept_eula: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_plugins: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateMutationResultDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_server_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_server_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exported_count: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub templates: Option<TemplatesResponseDto>,
}
