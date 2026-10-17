//! Storage protocol 1 metadata. Cached inventory never grants removal authority.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageItemDto {
    pub id: String,
    pub root_id: String,
    #[serde(default)]
    pub physical_root_id: String,
    pub volume_id: Option<String>,
    pub category: String,
    pub group: String,
    pub domain_id: Option<String>,
    pub server_id: Option<String>,
    pub label: String,
    #[serde(default)]
    pub display_path: Option<String>,
    pub logical_bytes: u64,
    pub allocated_bytes: Option<u64>,
    pub partial: bool,
    pub absent: bool,
    pub shared_references: u64,
    #[serde(default)]
    pub has_hard_links: bool,
    pub protection: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageVolumeDto {
    pub id: String,
    #[serde(default)]
    pub physical_id: String,
    #[serde(default)]
    pub label: String,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub observed_at: u64,
    pub problem: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageIssueDto {
    pub root_id: Option<String>,
    pub reason: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageProgressDto {
    pub roots_completed: usize,
    pub entries: u64,
    pub logical_bytes_visited: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSnapshotDto {
    pub schema_version: u32,
    pub id: String,
    pub host_id: String,
    pub generation: String,
    pub root_identity: String,
    pub started_at: u64,
    pub completed_at: u64,
    pub partial: bool,
    pub cancelled: bool,
    /// The display basis is always logical; allocation is separate and nullable.
    pub logical_bytes: u64,
    pub allocated_bytes: Option<u64>,
    pub items: Vec<StorageItemDto>,
    pub volumes: Vec<StorageVolumeDto>,
    pub issues: Vec<StorageIssueDto>,
    pub progress: StorageProgressDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInventoryDto {
    pub protocol: u32,
    pub host_id: String,
    pub historical: bool,
    pub snapshot: Option<StorageSnapshotDto>,
    pub total_items: usize,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageScanDto {
    pub host_id: String,
    pub operation_id: String,
    pub state: String,
    pub progress: StorageProgressDto,
    pub snapshot_id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInventoryCapabilityDto {
    pub protocol: u32,
    pub scan: bool,
    pub cache_cleanup: bool,
    pub server_data_cleanup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageCandidateDto {
    pub server_id: Option<String>,
    pub consequences: Vec<String>,
    pub id: String,
    pub label: String,
    pub display_path: Option<String>,
    pub logical_bytes: Option<u64>,
    pub removable: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageCandidatesDto {
    pub host_id: String,
    pub category: String,
    pub action: String,
    pub items: Vec<StorageCandidateDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageReviewRequestDto {
    pub host_id: String,
    pub category: String,
    pub action: String,
    pub item_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageReviewDto {
    pub host_id: String,
    pub category: String,
    pub action: String,
    pub preview_token: String,
    pub expires_in_seconds: u64,
    pub confirmation: String,
    pub items: Vec<StorageCandidateDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageReviewedCleanupRequestDto {
    pub host_id: String,
    pub preview_token: String,
    pub confirmation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageOutcomeDto {
    pub id: String,
    pub label: String,
    pub state: String,
    pub removed_logical_bytes: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageReviewedResultDto {
    #[serde(default)]
    pub volumes: Vec<StorageVolumeDto>,
    pub host_id: String,
    pub operation_id: String,
    pub state: String,
    pub items: Vec<StorageOutcomeDto>,
    pub available_bytes: Option<u64>,
    pub observed_at: u64,
}
