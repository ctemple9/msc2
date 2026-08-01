//! `HealthResponseDTO` / `HealthCardDTO` — `GET /v1/health`'s response
//! shape, carried forward from the MSC 1 baseline (`docs/msc2/api-baseline
//! /openapi.json`) unchanged in wire shape by P2.8's assembly.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCardDto {
    pub id: String,
    pub title: String,
    pub short_label: String,
    pub severity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub icon_system_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_code: Option<String>,
    /// Alongside `detail` — see `helpid-contract.md` §4.
    #[serde(rename = "helpId", default, skip_serializing_if = "Option::is_none")]
    pub help_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponseDto {
    pub server_type: String,
    pub server_name: String,
    pub server_running: bool,
    pub overall_severity: String,
    pub cards: Vec<HealthCardDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
