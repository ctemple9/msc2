//! `RemoteAPIStatus` — `GET /v1/status`'s response shape, carried forward
//! from the MSC 1 baseline (`docs/msc2/api-baseline/openapi.json`)
//! unchanged in wire shape by P2.8's assembly.

use serde::{Deserialize, Serialize};

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
}
