//! Phase 4 lifecycle route DTOs from `openapi.json`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerImportRequestDto {
    pub action: Option<String>,
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerImportScanResponseDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_zip: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_players: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eula_accepted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_world_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_flavor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerImportResultDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerDto {
    pub id: String,
    pub name: String,
    pub directory: String,
    pub server_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_flavor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_port: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveServerRequestDto {
    pub server_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandRequestDto {
    pub command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResultDto {
    pub result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_server_id: Option<String>,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleResultDto {
    pub result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_server_id: Option<String>,
}
