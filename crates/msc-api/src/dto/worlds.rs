//! Phase 6 world-slot route DTOs, frozen in `docs/msc2/api-contract/openapi.json`
//! (P6.8) and ported here verbatim (`docs/msc2/worlds/phase6-api.md`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A slot-local profile is intentionally a nested object rather than another
/// group of world-looking fields on `SettingsResponseDto`. The profile is
/// attached to a slot when persistence/readback lands in P12.24; this step
/// freezes its wire shape without inventing values for legacy slots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldProfileDto {
    pub schema_version: u32,
    pub identity: WorldIdentityDto,
    pub generation: WorldGenerationDto,
    pub gameplay: WorldGameplayDto,
    pub safety: WorldSafetyDto,
    pub field_metadata: BTreeMap<String, WorldProfileFieldMetadataDto>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorldIdentityDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorldGenerationDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flat_preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structures: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub biome_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator_options: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bonus_chest: Option<bool>,
    #[serde(default)]
    pub data_packs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorldGameplayDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_game_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardcore: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commands: Option<bool>,
    #[serde(default)]
    pub gamerules: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cheats: Option<bool>,
    #[serde(default)]
    pub experiments: BTreeMap<String, bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_map: Option<bool>,
    #[serde(default)]
    pub supported_toggles: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorldSafetyDto {
    /// Known values are `safe`, `achievement_disabled`, `unknown`, and
    /// `unsupported`; this remains a string so a newer agent can add a
    /// safety state without making an older client reject the whole profile.
    pub state: String,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldProfileFieldMetadataDto {
    /// Capability key the client should use when deciding whether the field
    /// is meaningful for this edition/version/flavor.
    pub capability: String,
    pub lifecycle: String,
    pub value_state: String,
    #[serde(rename = "helpId", default, skip_serializing_if = "Option::is_none")]
    pub help_id: Option<String>,
}

/// Future detailed slot shape: the existing slot summary plus the profile
/// that travels with it. The current `/v1/worlds` route remains the legacy
/// summary until P12.24 can populate profiles from metadata safely.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldSlotWithProfileDto {
    pub slot: WorldSlotDto,
    pub profile: WorldProfileDto,
}

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

/// `WorldSlotManager.copySlotIntoExisting` — a saved-slot-to-saved-slot
/// copy, not a live-world operation: `slot_id` is the existing
/// *destination* slot being overwritten, `source_slot_id` is the slot
/// whose saved content replaces it. Corrected post-review (Cameron):
/// this route does not touch the active/live world and needs no new
/// level name.
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
pub struct WorldRepairResultDto {
    pub result: String,
    /// Operation id for progress polling (`GET /v1/operations/{id}`) or
    /// `/v1/operations/{id}/stream` and cancellation, matching the
    /// operation-backed world activation response.
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
pub struct WorldImportRequestDto {
    pub name: String,
    #[serde(default)]
    pub staged_upload_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldThumbnailUploadRequestDto {
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

/// `AppViewModel+WorldManagement.swift::replaceWorld` — direct live-world
/// replacement. Separately named from [`WorldReplaceRequestDto`]
/// (`WorldSlotManager.copySlotIntoExisting`, a saved-slot-to-saved-slot
/// copy that never touches the live world) per Cameron's post-P6.21-review
/// correction (`phase6-api.md` SS9/SS10). Accepts only a bounded staged
/// upload — redeemed once, `purpose: "active-world-replace"` — plus the
/// new level name; never an arbitrary server-local path. Omitting
/// `staged_upload_id` replaces with a fresh (empty) world, matching
/// `msc_application::worlds::WorldReplaceSource::Fresh`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldReplaceActiveRequestDto {
    pub new_level_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staged_upload_id: Option<String>,
}

/// Always operation-backed — the mandatory pre-replace safety backup and
/// (for an uploaded source) zip staging are real filesystem work, the
/// same class as `activate`/`backups/restore`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldReplaceActiveResultDto {
    pub result: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
}

/// Corrected post-review (Cameron): MSC 1 conversion always names a
/// separate, opposite-edition *target* server (`targetServerId`) — the
/// source slot lives on the currently-active server, but the converted
/// world is placed on a different, explicitly-named one. `targetFormat`
/// is the exact Chunker format string the client chose from
/// `WorldConverter::supported_formats` (MSC 1 defaults its own picker to
/// the newest compatible format but always lets the user override it —
/// never hardcoded here). Exactly one of `target_name` (place into a
/// fresh slot) or `target_slot_id` (overwrite an existing slot on the
/// target server, looked up by id, not display name) must be present —
/// validated at the route layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldConvertRequestDto {
    pub source_slot_id: String,
    pub target_server_id: String,
    pub target_format: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_slot_id: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldConvertFormatsResponseDto {
    pub formats: Vec<String>,
}
