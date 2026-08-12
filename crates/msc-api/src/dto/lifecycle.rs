//! Phase 4/5 lifecycle route DTOs from `openapi.json`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// `ServerImportRequestDTO`'s frozen shape (`docs/msc2/api-contract/openapi.json`).
/// `action` preserves the real values `scan|importExisting|importTransfer`;
/// `importKind` preserves `folder|zip|transfer|auto` (P5.17).
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_world_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_players: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accept_eula: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_playit: Option<bool>,
    /// `merge` (default, including absent/unrecognized) or `replaceAll` —
    /// see `phase5-scope.md`'s "Transfer behavior".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer_mode: Option<String>,
    /// Required, non-blank, when `transfer_mode == "replaceAll"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
    /// Keyed by the *source* server's id, as recorded in the transfer
    /// manifest.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub java_port_overrides: HashMap<String, i64>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub bedrock_port_overrides: HashMap<String, i64>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub worlds: Vec<ServerImportWorldDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_world_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_flavor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_mc_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_loader_version: Option<String>,
}

/// `ServerImportWorldDTO`'s frozen shape — `id` mirrors MSC 1's own
/// `DetectedWorld: Identifiable`, whose `id` is just its `name`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerImportWorldDto {
    pub id: String,
    pub name: String,
    pub size_bytes: i64,
    pub dimensions_label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerImportResultDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
}
