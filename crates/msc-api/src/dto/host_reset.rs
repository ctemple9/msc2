use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostResetRequestDto {
    pub mode: String,
    pub confirmation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostResetAcceptedDto {
    pub operation_id: String,
    pub host_id: String,
    pub mode: String,
    pub agent_state: String,
    pub message: String,
}
