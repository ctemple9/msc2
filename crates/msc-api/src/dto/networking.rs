use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayitStatusDto {
    pub server_name: String,
    pub server_type: String,
    pub playit_enabled: bool,
    pub is_running: bool,
    pub has_secret_key: bool,
    pub java_address: Option<String>,
    pub bedrock_address: Option<String>,
    pub voice_address: Option<String>,
    pub voice_chat_enabled: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayitActionResultDto {
    pub result: String,
    pub message: Option<String>,
    pub operation_id: Option<String>,
}

/// Credentials submitted to the agent for the native Playit account flow.
/// This type intentionally derives `Deserialize` only: email and password
/// must never be serialized into an operation result, status response, or log.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayitSetupRequestDto {
    pub email: String,
    pub password: String,
}

/// Admission response for the native Playit setup operation. The operation
/// status contains progress and any structured failure; this response carries
/// only its opaque identifier and never account or agent data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayitSetupAcceptedDto {
    pub result: String,
    pub operation_id: String,
    pub message: Option<String>,
}

/// Result of clearing host-local Playit state. Repeating the request is a
/// successful no-op once the key and derived state are already absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayitResetResultDto {
    pub result: String,
    pub message: Option<String>,
    pub operation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePackItemDto {
    pub id: String,
    pub name: String,
    pub file_name: String,
    pub file_size_display: String,
    pub pack_kind: String,
    pub is_active: bool,
    pub type_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePacksResponseDto {
    pub server_type: String,
    pub is_java: bool,
    pub packs: Vec<ResourcePackItemDto>,
    pub geyser_packs: Vec<ResourcePackItemDto>,
    pub is_geyser_available: bool,
    pub active_pack_url: Option<String>,
    pub require_pack: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePackActivateRequestDto {
    pub pack_id: Option<String>,
    pub require: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePackSetUrlRequestDto {
    pub url: String,
    pub sha1: Option<String>,
    pub require: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePackToggleRequestDto {
    pub pack_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePackRemoveRequestDto {
    pub pack_id: String,
    pub pack_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePackMutationResultDto {
    pub success: bool,
    pub message: String,
    pub updated: Option<ResourcePacksResponseDto>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuckDnsStatusResponseDto {
    pub hostname: Option<String>,
    pub is_configured: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuckDnsUpdateRequestDto {
    pub hostname: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuckDnsUpdateResultDto {
    pub success: bool,
    pub hostname: Option<String>,
    pub message: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityPortDiagnosticDto {
    pub outcome: String,
    pub detail: Option<String>,
    pub help_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityPortDiagnosticsDto {
    pub local: ConnectivityPortDiagnosticDto,
    pub public: ConnectivityPortDiagnosticDto,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityResponseDto {
    pub server_type: String,
    pub server_name: String,
    pub server_running: bool,
    pub status: String,
    pub severity: String,
    pub headline: String,
    pub detail: Option<String>,
    pub join_address: Option<String>,
    pub method: String,
    pub join_address_source: String,
    pub local_listening: Option<bool>,
    pub externally_reachable: Option<bool>,
    pub port_diagnostics: ConnectivityPortDiagnosticsDto,
    pub note: Option<String>,
    pub help_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeyserConfigResponseDto {
    pub server_name: String,
    pub server_type: String,
    pub is_geyser_installed: bool,
    pub address: Option<String>,
    pub port: Option<i64>,
    pub config_file_exists: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeyserConfigUpdateRequestDto {
    pub address: Option<String>,
    pub port: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeyserConfigUpdateResultDto {
    pub success: bool,
    pub message: String,
    pub address: Option<String>,
    pub port: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastStatusDto {
    pub xbox_broadcast_running: bool,
    pub bedrock_broadcast_running: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gamertag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastAutoStartDto {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastAuthPromptDto {
    pub is_present: bool,
    pub code: Option<String>,
    #[serde(rename = "linkURL")]
    pub link_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastCredentialsDto {
    pub email: String,
    pub password: String,
    pub gamertag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastJarStatusDto {
    pub installed: bool,
    pub downloading: bool,
    pub filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastJarDownloadResultDto {
    pub success: bool,
    pub message: String,
    pub filename: Option<String>,
    pub operation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastSimpleResultDto {
    pub result: String,
    pub operation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationEventDto {
    pub id: String,
    pub server_id: String,
    pub occurred_at_iso8601: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub help_id: Option<String>,
}
