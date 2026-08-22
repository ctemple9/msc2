use serde::{Deserialize, Serialize};
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
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityPortDiagnosticDto {
    pub outcome: String,
    pub detail: Option<String>,
    pub help_id: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityPortDiagnosticsDto {
    pub local: ConnectivityPortDiagnosticDto,
    pub public: ConnectivityPortDiagnosticDto,
}
#[derive(Debug, Clone, Serialize)]
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
