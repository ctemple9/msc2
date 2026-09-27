//! Exact-release gamerule picker contract; defaults are not current world values.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameruleDefinitionDto {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(rename = "type")]
    pub value_type: String,
    pub default_value: String,
    pub choices: Vec<String>,
    pub experimental: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameruleCatalogDto {
    pub server_type: String,
    pub minecraft_version: Option<String>,
    pub available: bool,
    /// Completeness covers built-in rules, not rules registered by mods.
    pub complete: bool,
    pub source: Option<String>,
    pub note: Option<String>,
    pub rules: Vec<GameruleDefinitionDto>,
}
