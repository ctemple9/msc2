//! `SettingsResponseDTO`/`SettingsUpdateResultDTO` and friends —
//! `GET`/`POST /v1/settings`'s typed `server.properties` schema, per
//! `openapi.json`. Wire shapes only; the section/field builder and the
//! validate-and-persist logic live in `msc-agent`'s settings route (this
//! crate carries no dependency on `msc-domain`, matching every other DTO
//! module here).
//!
//! `helpId` replaces the MSC 1 baseline's free-text `SettingFieldDTO.help`
//! field one-for-one (`docs/msc2/api-contract/helpid-contract.md` §4):
//! present only on the handful of fields MSC 1 actually gave inline help
//! text, resolved later via the still-homeless `GET /v1/help/{helpId}`
//! route this phase does not build.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::BedrockRuntimeStateDto;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingOptionDto {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingFieldDto {
    pub key: String,
    pub label: String,
    pub r#type: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_int: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_int: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<SettingOptionDto>>,
    #[serde(rename = "helpId", default, skip_serializing_if = "Option::is_none")]
    pub help_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsSectionDto {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub fields: Vec<SettingFieldDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsResponseDto {
    pub server_type: String,
    pub server_name: String,
    pub server_running: bool,
    pub editable: bool,
    pub sections: Vec<SettingsSectionDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<BedrockRuntimeStateDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsUpdateRequestDto {
    pub changes: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingRejectionDto {
    pub key: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdateResultDto {
    pub success: bool,
    pub message: String,
    pub restart_required: bool,
    pub applied_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejected: Option<Vec<SettingRejectionDto>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sections: Option<Vec<SettingsSectionDto>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<BedrockRuntimeStateDto>,
}
