//! Phase 6 backup and staged-transfer route DTOs, frozen in
//! `docs/msc2/api-contract/openapi.json` (P6.8) and ported here verbatim
//! (`docs/msc2/worlds/phase6-api.md`).

use serde::{Deserialize, Serialize};

use super::BedrockRuntimeStateDto;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupItemDto {
    pub id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modification_date: Option<String>,
    pub is_automatic: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot_name: Option<String>,
    pub trigger_reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupsResponseDto {
    #[serde(default)]
    pub backups: Vec<BackupItemDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<BedrockRuntimeStateDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupConfigResponseDto {
    pub server_name: String,
    pub auto_backup_enabled: bool,
    pub auto_backup_interval_minutes: i64,
    pub auto_backup_max_count: i64,
    #[serde(default)]
    pub interval_options: Vec<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<BedrockRuntimeStateDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupConfigUpdateRequestDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_backup_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_backup_interval_minutes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_backup_max_count: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupConfigUpdateResultDto {
    pub success: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<BackupConfigResponseDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<BedrockRuntimeStateDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupNowResultDto {
    pub result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<BedrockRuntimeStateDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestoreRequestDto {
    pub backup_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestoreResultDto {
    pub result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<BedrockRuntimeStateDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupDeleteRequestDto {
    pub backup_id: String,
}

/// `purpose` is a closed enum, per `phase6-api.md` §4: a staging slot can
/// only be redeemed by the route it was created for. P6.34 adds
/// `ActiveWorldReplace` (`active-world-replace`) alongside the original
/// `WorldImport` value for the new direct-live-world-replacement route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StagedUploadPurposeDto {
    WorldImport,
    ActiveWorldReplace,
    ModpackArchive,
    AddonLocalFile,
    CurseforgeManualFile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedUploadBeginRequestDto {
    pub purpose: StagedUploadPurposeDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedUploadBeginResultDto {
    pub staged_upload_id: String,
    /// `PUT /v1/staged-uploads/{id}` — bounded to this token, not an
    /// arbitrary remote path.
    pub upload_path: String,
    pub expires_at: String,
    pub max_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedUploadCompleteResultDto {
    pub staged_upload_id: String,
    pub received_bytes: i64,
    pub sha256: String,
}
