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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_token: Option<String>,
    pub host_id: String,
    pub mode: String,
    pub agent_state: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostResetStatusDto {
    pub state: String,
    pub mode: Option<String>,
    pub message: String,
    pub can_cancel: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostResetCompletionDto {
    pub state: String,
}
