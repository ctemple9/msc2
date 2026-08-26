//! `RemoteAPIStatus` — `GET /v1/status`'s response shape, carried forward
//! from the MSC 1 baseline (`docs/msc2/api-baseline/openapi.json`)
//! unchanged in wire shape by P2.8's assembly.

use serde::{Deserialize, Serialize};

use super::BedrockRuntimeStateDto;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteApiStatus {
    pub running: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_server_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docker_container_running: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docker_container_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<BedrockRuntimeStateDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceSnapshotDto {
    pub ts: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tps_1m: Option<PerformanceMetricNumberDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tps_5m: Option<PerformanceMetricNumberDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tps_15m: Option<PerformanceMetricNumberDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub players_online: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_percent: Option<PerformanceMetricNumberDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ram_used_mb: Option<PerformanceMetricNumberDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ram_max_mb: Option<PerformanceMetricNumberDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_size_mb: Option<PerformanceMetricNumberDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<BedrockRuntimeStateDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceMetricNumberDto {
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help_id: Option<String>,
}
