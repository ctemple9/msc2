//! P7.23: `ServerCreateRequestDTO`/`ServerCreateResultDTO`,
//! `ServerDeleteRequestDTO`/`ServerDeleteResultDTO`,
//! `ServerRenameRequestDTO`/`ServerRenameResultDTO`,
//! `ServerDirectoryRequestDTO`/`ServerDirectoryResultDTO`,
//! `ServerEULARequestDTO`/`ServerEULAResultDTO` — `openapi.json`'s frozen
//! shapes for `POST /v1/servers/create|delete|rename|directory|eula`.

use serde::{Deserialize, Serialize};

use super::BedrockRuntimeStateDto;
use super::worlds::{WorldGameplayDto, WorldGenerationDto, WorldIdentityDto};

/// The first world profile supplied alongside a fresh server create. It
/// intentionally omits readback-only safety and field metadata; the agent
/// owns those values after it has created the slot.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCreateWorldSettingsDto {
    #[serde(default)]
    pub identity: WorldIdentityDto,
    #[serde(default)]
    pub generation: WorldGenerationDto,
    #[serde(default)]
    pub gameplay: WorldGameplayDto,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCreateRequestDto {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_flavor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_players: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_cross_play: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_play_bedrock_port: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_playit: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_voice_chat: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_xbox_broadcast: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gamemode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minecraft_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accept_eula: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bedrock_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docker_image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staged_modpack_upload_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_settings: Option<ServerCreateWorldSettingsDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCreateResultDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<BedrockRuntimeStateDto>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerDeleteRequestDto {
    pub server_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerDeleteResultDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerRenameRequestDto {
    pub server_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerRenameResultDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerDirectoryRequestDto {
    pub server_id: String,
    pub directory: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerDirectoryResultDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
}

/// `serverId` is optional in the wire schema, but the real route treats
/// an absent/empty one as `missing_server_id` — see `openapi.json`'s own
/// `x-notes` on this DTO.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerEulaRequestDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerEulaResultDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted: Option<bool>,
}
