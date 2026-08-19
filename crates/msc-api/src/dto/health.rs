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

/// P7.24: `StartupProblemDTO` — `GET /v1/health/problems`'s per-problem
/// shape.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupProblemDto {
    pub id: String,
    pub kind: String,
    pub kind_title: String,
    pub icon_system_name: String,
    pub offender_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_jar_stem: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing_dependency: Option<String>,
    pub raw_excerpt: String,
    pub is_repairing: bool,
    pub available_actions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modrinth_url: Option<String>,
    /// Keyed off `kind`, e.g. `diagnostics.crash.forge-dep` —
    /// `helpid-contract.md` §4. `null`, not omitted, when absent: the
    /// schema marks this field `nullable`, not optional.
    #[serde(rename = "helpId")]
    pub help_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthProblemsResponseDto {
    pub server_type: String,
    pub server_running: bool,
    pub is_soft_fail: bool,
    pub problems: Vec<StartupProblemDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthRepairRequestDto {
    pub problem_id: String,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthRepairResultDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<HealthProblemsResponseDto>,
}
