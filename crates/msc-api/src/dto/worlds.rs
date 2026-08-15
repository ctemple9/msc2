//! Phase 6 world-slot route DTOs, frozen in `docs/msc2/api-contract/openapi.json`
//! (P6.8) and ported here verbatim (`docs/msc2/worlds/phase6-api.md`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldSlotDto {
    pub id: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zip_size_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_seed: Option<String>,
    pub has_thumbnail: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldSlotsResponseDto {
    #[serde(default)]
    pub slots: Vec<WorldSlotDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_slot_id: Option<String>,
    pub server_running: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_repairing: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldCreateRequestDto {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldRenameRequestDto {
    pub slot_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldReplaceRequestDto {
    pub slot_id: String,
    pub source_slot_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldRepairRequestDto {
    pub slot_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldActivateRequestDto {
    pub slot_id: String,
}

/// `x-notes` on `POST /v1/worlds/activate`: read directly from
/// `RemoteAPIServer+HTTP.swift`, not the `WorldMutationResultDTO` shape
/// every other `/worlds/*` mutation uses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldActivateResultDto {
    pub result: String,
    /// Operation id for progress polling (`GET /v1/operations/{id}`) or
    /// `/v1/operations/{id}/stream` and cancellation; optional so older
    /// clients can ignore it, matching `SimpleResultDto`'s P4 precedent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldMutationResultDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<WorldSlotsResponseDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldDeleteRequestDto {
    pub slot_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldDuplicateRequestDto {
    pub slot_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldCopyRequestDto {
    pub slot_id: String,
    pub source_slot_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldImportRequestDto {
    pub name: String,
    pub staged_upload_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldExportRequestDto {
    pub slot_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldExportResultDto {
    pub staged_download_id: String,
    pub expires_at: String,
    pub size_bytes: i64,
}

/// Direct rename of the active/live world's on-disk folders
/// (`AppViewModel+WorldManagement.swift::renameWorld`) — distinct from
/// [`WorldRenameRequestDto`], which renames a slot's metadata only and
/// touches no files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldRenameActiveWorldRequestDto {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldConvertRequestDto {
    pub slot_id: String,
    pub target_name: String,
    pub replace_existing: bool,
}

/// Always operation-backed (`type: world-conversion`,
/// `operation-model.md` §2) — Chunker's process lifetime makes this the
/// one Phase 6 world mutation with no synchronous variant, so unlike
/// every other result DTO in this module, `operation_id` is required.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldConvertResultDto {
    pub result: String,
    pub operation_id: String,
}
