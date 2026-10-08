//! Phase 6 world-slot routes (P6.21) — backed by the real
//! `msc_application::worlds`/`msc_application::world_conversion` services
//! over `StdFileSystem` and the active server's real directory, per
//! `docs/msc2/worlds/phase6-api.md`.
//!
//! **Operation journaling, per-server exclusivity, and cancellation.**
//! Every mutation route below — synchronous or async — begins a
//! journaled operation (`state.lifecycle.operations().begin_lifecycle`)
//! targeting the active server's id before doing any real work, exactly
//! the way `LifecycleRoutesState::start_active_server` already does.
//! `msc_infrastructure::operation_journal::OperationJournal::admit`
//! refuses a second non-terminal entry sharing that target, so this one
//! journal call gives every route below per-server exclusivity for free
//! (`worlds.rs`'s own P6.13 section doc names this exact mechanism as
//! "left for the route layer (P6.21) to wire") — a concurrent mutation
//! against the same active server gets `409 conflict`, not a silently
//! interleaved write. The four genuinely async operations
//! (`activate`/`convert`, plus `backups::now`/`backups::restore` in
//! `routes/backups.rs`) run their real work on a spawned `tokio` task
//! (mirroring `spawn_process_pump`'s existing shape) and `succeed`/`fail`
//! the operation from inside it; every synchronous CRUD route begins,
//! does the work, and `succeed`s/`fail`s all within the same
//! request/response cycle, so it's already terminal by the time a client
//! could poll or cancel it.
//!
//! **Cancellation is cooperative and truthful.** A
//! `POST /v1/operations/{id}/cancel` signals the operation's worker and
//! returns `202` while cleanup is pending. World transactions poll at
//! boundaries before touching the live world, and backup creation polls
//! between bounded archive chunks; only the worker records `cancelled`
//! after its cleanup has finished, so per-server exclusivity remains held
//! for the entire mutation lifetime.
//!
//! **Audit attribution is scoped to this module and `routes/backups.rs`
//! only.** `msc_infrastructure::audit_log::AuditLog` is wired here (one
//! entry per mutation: method, path, credential label, response status)
//! but nowhere else in this agent yet — `routes/lifecycle.rs`,
//! `routes/settings.rs`, `routes/servers.rs` remain unaudited, a
//! pre-existing gap this step doesn't attempt to close.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
use axum::body::Bytes;
use axum::extract::Query;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use msc_api::dto::{
    BedrockBehaviorPackInstallRequestDto, BedrockBehaviorPackInstallResultDto,
    BedrockBehaviorPackSearchResponseDto, ErrorDto, JavaDatapackInstallRequestDto,
    JavaDatapackInstallResultDto, PermissionCategoryDto, StagedUploadPurposeDto,
    WorldActivateRequestDto, WorldActivateResultDto, WorldChunkerDownloadResultDto,
    WorldConvertFormatsResponseDto, WorldConvertRequestDto, WorldConvertResultDto,
    WorldCreateRequestDto, WorldDeleteRequestDto, WorldDuplicateRequestDto, WorldExportRequestDto,
    WorldExportResultDto, WorldGameplayDto, WorldGenerationDto, WorldIdentityDto,
    WorldImportRequestDto, WorldMapDimensionDto, WorldMapDimensionsResponseDto,
    WorldMutationResultDto, WorldPackDependencyDto, WorldPackRecordDto, WorldPackSourceDto,
    WorldProfileDto, WorldProfileFieldMetadataDto, WorldRenameActiveWorldRequestDto,
    WorldRenameRequestDto, WorldRepairRequestDto, WorldRepairResultDto,
    WorldReplaceActiveRequestDto, WorldReplaceActiveResultDto, WorldReplaceRequestDto,
    WorldSafetyDto, WorldSlotDto, WorldSlotWithProfileDto, WorldSlotsResponseDto,
    WorldThumbnailUploadRequestDto,
};
#[cfg(test)]
use msc_api::dto::{
    StagedUploadBeginRequestDto, StagedUploadBeginResultDto, StagedUploadCompleteResultDto,
};
use msc_application::backups;
use msc_application::world_conversion::{
    self, ConversionError, ConversionPlacement, WorldConverter,
};
use msc_application::world_repair::{RepairServerControl, WorldRepairError, repair_world};
use msc_application::world_safety::{self, SafetyConfirmation};
use msc_application::worlds::{self, WorldError, WorldReplaceSource};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::identity::ServerType;
use msc_domain::world::WorldSlot;
use msc_domain::world_profile::{WorldPackRecord, WorldProfile, WorldProfileField};
use msc_infrastructure::audit_log::Entry as AuditEntry;
use msc_infrastructure::fs::{FileSystem, StdFileSystem};
use msc_infrastructure::jar_provider::HttpTransport;
use msc_infrastructure::world_store;
use serde::Serialize;
#[cfg(test)]
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::auth::AuthenticatedCredential;
use crate::routes::bedrock::require_runtime;
use crate::routes::lifecycle::{
    LifecycleRoutesState, ReconciliationStatus, error_response, invalid_body,
    reconciliation_degraded_response, require_permission,
};

mod map_terrain;

/// A bounded ceiling for one staged world upload — generous enough for a
/// large modpack world (tens of GB is unusual for a single Minecraft
/// world save) while still bounded, per `phase6-api.md` §4's own
/// deferral of the exact number to this step. Not derived from any
/// fixture or MSC 1 constant — this step's own scoping decision, flagged
/// in the P6.21 report rather than treated as an oracle-derived value.
#[cfg(test)]
const MAX_STAGED_UPLOAD_BYTES: u64 = 10 * 1024 * 1024 * 1024;
/// How long a staged upload/download token stays redeemable before it
/// expires — another of this step's own scoping decisions (§4 leaves the
/// exact window to "P6.21 wiring").
pub(crate) const STAGING_TTL_SECONDS: u64 = 30 * 60;

pub fn router(state: WorldsRoutesState) -> Router {
    Router::new()
        .route("/worlds", get(list))
        .route("/worlds/map/dimensions", get(map_dimensions))
        .route("/worlds/map/terrain", get(map_terrain::artifact))
        .route(
            "/worlds/map/progress",
            get(map_terrain::preparation_progress),
        )
        .route("/worlds/map/refresh", post(refresh_map))
        .route("/worlds/map/players", get(map_players))
        .route("/worlds/map-proof/snapshot", post(snapshot_map_proof))
        .route("/worlds/map-proof/players", get(map_proof_players))
        .route("/catalog/gamerules", get(gamerule_catalog))
        .route("/worlds/create", post(create))
        .route("/worlds/rename", post(rename))
        .route("/worlds/replace", post(replace))
        .route("/worlds/repair", post(repair))
        .route("/worlds/update", post(update))
        .route("/worlds/delete", post(delete))
        .route("/worlds/duplicate", post(duplicate))
        .route("/worlds/import", post(import))
        .route("/worlds/export", post(export))
        .route("/worlds/rename-active-world", post(rename_active_world))
        .route("/worlds/replace-active-world", post(replace_active))
        .route("/worlds/activate", post(activate))
        .route(
            "/worlds/:slot_id/profile",
            get(get_profile).post(update_profile),
        )
        .route(
            "/worlds/:slot_id/datapacks/install",
            post(install_java_datapack),
        )
        .route("/catalog/behaviorpacks", get(search_bedrock_behavior_packs))
        .route(
            "/catalog/behaviorpacks/:project_id",
            get(get_bedrock_behavior_pack_detail),
        )
        .route(
            "/worlds/:slot_id/behaviorpacks/install",
            post(install_bedrock_behavior_pack),
        )
        .route("/worlds/convert/formats", get(convert_formats))
        .route(
            "/worlds/convert/chunker",
            get(check_chunker_update).post(download_chunker),
        )
        .route("/worlds/convert", post(convert))
        .route(
            "/worlds/:slot_id/thumbnail",
            get(thumbnail).post(set_thumbnail),
        )
        .with_state(state)
}

pub async fn convert_formats(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    let lifecycle = state.lifecycle.clone();
    if state.map_shutdown.load(Ordering::Acquire) {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_stopping",
            "The agent is stopping.",
        );
    }
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let converter = LiveWorldConverter;
    let java_available = converter.resolve_java_path("").is_some();
    let installed = converter.is_installed();
    let formats = if installed && java_available {
        converter
            .resolve_java_path("")
            .map(|path| converter.supported_formats(&path))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let response = Json(WorldConvertFormatsResponseDto {
        formats,
        installed,
        downloading: state.chunker_download_in_progress.load(Ordering::Acquire),
        java_available,
        version: msc_infrastructure::chunker::installed_version(),
    })
    .into_response();
    audit(
        &lifecycle,
        &credential,
        "GET",
        "/v1/worlds/convert/formats",
        response.status(),
    );
    response
}

pub async fn check_chunker_update(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let result = tokio::task::spawn_blocking(|| {
        msc_infrastructure::chunker::latest_version(&HttpTransport::new())
    })
    .await;
    let response = match result {
        Ok(Ok(version)) => Json(serde_json::json!({ "latestVersion": version })).into_response(),
        Ok(Err(error)) => error_response(
            StatusCode::BAD_GATEWAY,
            "chunker_check_failed",
            &error.to_string(),
        ),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "chunker_check_failed",
            &error.to_string(),
        ),
    };
    audit(
        &state.lifecycle,
        &credential,
        "GET",
        "/v1/worlds/convert/chunker",
        response.status(),
    );
    response
}

pub async fn download_chunker(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    if state
        .chunker_download_in_progress
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return error_response(
            StatusCode::CONFLICT,
            "download_in_progress",
            "A Chunker download is already in progress.",
        );
    }
    let operation_id = match begin_operation(
        &state.lifecycle,
        "chunker",
        "chunker-download",
        "Downloading Chunker.",
    ) {
        Ok(id) => id,
        Err(response) => {
            state
                .chunker_download_in_progress
                .store(false, Ordering::Release);
            return response;
        }
    };
    let worker_state = state.clone();
    let worker_operation_id = operation_id.clone();
    let task_operation_id = operation_id.clone();
    tokio::spawn(async move {
        let task_state = worker_state.clone();
        let result = tokio::task::spawn_blocking(move || {
            let transport = HttpTransport::new();
            let mut progress = |line: &str| {
                let _ =
                    task_state
                        .lifecycle
                        .operations()
                        .progress(&worker_operation_id, 0, 0, line);
            };
            msc_infrastructure::chunker::download_latest(&transport, &StdFileSystem, &mut progress)
        })
        .await;
        worker_state
            .chunker_download_in_progress
            .store(false, Ordering::Release);
        match result {
            Ok(Ok(acquired)) => {
                let mut details = BTreeMap::new();
                details.insert("version".to_string(), acquired.metadata.version);
                let _ = worker_state.lifecycle.operations().succeed(
                    &task_operation_id,
                    "Chunker is ready.",
                    details,
                );
            }
            Ok(Err(error)) => {
                let _ = worker_state.lifecycle.operations().fail(
                    &task_operation_id,
                    "chunker_download_failed",
                    error.to_string(),
                );
            }
            Err(error) => {
                let _ = worker_state.lifecycle.operations().fail(
                    &task_operation_id,
                    "background_worker_failed",
                    error.to_string(),
                );
            }
        }
    });
    let response = Json(WorldChunkerDownloadResultDto {
        result: "chunker_download_started".to_string(),
        operation_id: operation_id.as_str().to_string(),
    })
    .into_response();
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/worlds/convert/chunker",
        response.status(),
    );
    response
}

// =====================================================================
// Shared route state: `LifecycleRoutesState` plus the staged-upload/
// download store — a new, self-contained type kept in this file (world
// import/export's own concern) rather than a new crate module, to stay
// inside this step's own file list.
// =====================================================================

#[derive(Clone)]
pub struct WorldsRoutesState {
    map_shutdown: std::sync::Arc<AtomicBool>,
    pub lifecycle: LifecycleRoutesState,
    pub(crate) staging: StagingStore,
    pub(crate) chunker_download_in_progress: std::sync::Arc<AtomicBool>,
    map_renderer: map_terrain::RendererStore,
    bedrock_map: map_terrain::bedrock::BedrockStore,
}

impl WorldsRoutesState {
    pub(crate) fn begin_map_shutdown(&self) {
        self.map_shutdown.store(true, Ordering::Release);
        #[cfg(target_os = "windows")]
        {
            self.bedrock_map.set_shutdown(true);
            self.map_renderer.set_shutdown(true);
        }
    }
    #[cfg(target_os = "windows")]
    pub(crate) fn cancel_map_shutdown(&self) {
        self.map_shutdown.store(false, Ordering::Release);
        self.bedrock_map.set_shutdown(false);
        self.map_renderer.set_shutdown(false);
    }
    #[cfg(target_os = "windows")]
    pub(crate) fn release_map_caches_checked(&self) -> Result<(), String> {
        self.map_renderer.release_checked()?;
        self.bedrock_map.release_checked()
    }
    pub(crate) fn release_map_caches(&self) {
        self.map_renderer.release();
        self.bedrock_map.release();
    }

    #[cfg(test)]
    pub fn new(lifecycle: LifecycleRoutesState) -> Self {
        Self {
            map_shutdown: std::sync::Arc::new(AtomicBool::new(false)),
            lifecycle,
            staging: StagingStore::default(),
            chunker_download_in_progress: std::sync::Arc::new(AtomicBool::new(false)),
            map_renderer: map_terrain::RendererStore::default(),
            bedrock_map: map_terrain::bedrock::BedrockStore::default(),
        }
    }

    pub fn with_staging(lifecycle: LifecycleRoutesState, staging: StagingStore) -> Self {
        Self {
            map_shutdown: std::sync::Arc::new(AtomicBool::new(false)),
            lifecycle,
            staging,
            chunker_download_in_progress: std::sync::Arc::new(AtomicBool::new(false)),
            map_renderer: map_terrain::RendererStore::default(),
            bedrock_map: map_terrain::bedrock::BedrockStore::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StagedUpload {
    pub(crate) purpose: StagedUploadPurposeDto,
    pub(crate) file_name: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) file_id: Option<String>,
    pub(crate) expires_at_unix: u64,
    pub(crate) max_bytes: u64,
    pub(crate) expected_bytes: Option<u64>,
    pub(crate) received_bytes: u64,
    pub(crate) complete: bool,
    pub(crate) path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct StagedDownload {
    pub(crate) expires_at_unix: u64,
    pub(crate) path: PathBuf,
}

/// Bytes live on disk under `<servers_root>/.msc2-staging/{uploads,
/// downloads}/{id}.{bin,zip}` — `servers_root()` is already an
/// agent-controlled directory, so nothing user-supplied ever names a
/// path component here; only the opaque, server-generated `{id}` UUID
/// does. Metadata lives in memory only (an agent restart loses in-flight
/// staged transfers, the same "best-effort, not durable" shape this
/// step's own scope note leaves to a later phase).
#[derive(Clone, Default)]
pub(crate) struct StagingStore {
    pub(crate) uploads: std::sync::Arc<Mutex<HashMap<String, StagedUpload>>>,
    pub(crate) downloads: std::sync::Arc<Mutex<HashMap<String, StagedDownload>>>,
}

pub(crate) fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub(crate) fn unix_to_iso8601(unix: u64) -> String {
    let days = (unix / 86_400) as i64;
    let secs_of_day = unix % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days` (public domain) — this crate's
/// third private copy of the same algorithm (`routes/lifecycle.rs`,
/// `msc-infrastructure::audit_log` are the other two), each kept local
/// to its one call site per this codebase's own established precedent
/// rather than made `pub` across a crate boundary for it.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

pub(crate) fn staging_root(servers_root: &Path) -> PathBuf {
    servers_root.join(".msc2-staging")
}

fn audit(
    state: &LifecycleRoutesState,
    credential: &AuthenticatedCredential,
    method: &str,
    path: &str,
    status: StatusCode,
) {
    let _ = state.audit_log().log(&AuditEntry {
        timestamp: SystemTime::now(),
        client_ip: String::new(),
        token_label: credential.label.clone(),
        method: method.to_string(),
        path: path.to_string(),
        status_code: status.as_u16(),
    });
}

// =====================================================================
// Shared helpers
// =====================================================================

fn no_active_server() -> Response {
    error_response(
        StatusCode::CONFLICT,
        "conflict",
        "No server is currently active.",
    )
}

fn slot_not_found(slot_id: &str) -> Response {
    error_response(
        StatusCode::NOT_FOUND,
        "not_found",
        &format!("No world slot named '{slot_id}' exists."),
    )
}

/// Resolves the active server for a mutation route, refusing (per
/// P6.29/P6.38) a server that has not reached [`ReconciliationStatus::Ready`]
/// reconciliation before any mutation logic runs. Read-only routes
/// (`list`, `thumbnail`, staged-download) deliberately call
/// `active_config_server` directly instead of this function — a damaged
/// server still needs to be inspectable, per the gate review's "keep the
/// agent available for diagnosis" requirement.
#[allow(clippy::result_large_err)]
fn active_server_or_response(state: &LifecycleRoutesState) -> Result<ConfigServer, Response> {
    let server = state.active_config_server().ok_or_else(no_active_server)?;
    match state.reconciliation_status(&server.id) {
        ReconciliationStatus::Ready => {}
        ReconciliationStatus::Reconciling => {
            return Err(reconciliation_degraded_response(
                "world reconciliation is still in progress",
            ));
        }
        ReconciliationStatus::Degraded { reason } => {
            return Err(reconciliation_degraded_response(&reason));
        }
    }
    Ok(server)
}

fn find_slot(server_dir: &Path, slot_id: &str) -> Option<WorldSlot> {
    world_store::load_slots(&StdFileSystem, server_dir)
        .into_iter()
        .find(|slot| slot.id == slot_id)
}

fn resolved_active_slot_id(server_dir: &Path) -> Option<String> {
    let slots = world_store::load_slots(&StdFileSystem, server_dir);
    let marker = world_store::load_explicit_active_slot_id(&StdFileSystem, server_dir);
    msc_domain::world::resolve_active_slot_id(&slots, marker.as_deref())
}

fn slots_response(server: &ConfigServer, running: bool) -> WorldSlotsResponseDto {
    let server_dir = Path::new(&server.server_dir);
    let slots = world_store::load_slots(&StdFileSystem, server_dir);
    let active_id = resolved_active_slot_id(server_dir);
    WorldSlotsResponseDto {
        slots: slots
            .iter()
            .map(|slot| to_slot_dto(slot, active_id.as_deref() == Some(slot.id.as_str())))
            .collect(),
        active_slot_id: active_id,
        server_running: running,
        is_repairing: Some(false),
    }
}

fn to_slot_dto(slot: &WorldSlot, is_active: bool) -> WorldSlotDto {
    WorldSlotDto {
        id: slot.id.clone(),
        name: slot.name.clone(),
        is_active,
        created_at: slot.created_at.clone(),
        zip_size_bytes: slot.zip_size_bytes,
        world_seed: slot.world_seed.clone(),
        has_thumbnail: slot.thumbnail_file_name.is_some(),
    }
}

fn profile_to_dto(
    profile: &WorldProfile,
    server_type: ServerType,
    world_zip_path: &Path,
) -> WorldProfileDto {
    let field_metadata = WorldProfileField::ALL
        .into_iter()
        .map(|field| {
            let value_state = if !field.applies_to(server_type) {
                "unsupported".to_string()
            } else if field == WorldProfileField::SafetyState {
                match profile.safety.state {
                    msc_domain::world_profile::WorldSafetyState::Safe => "detected",
                    msc_domain::world_profile::WorldSafetyState::AchievementDisabled => {
                        "achievement_disabled"
                    }
                    msc_domain::world_profile::WorldSafetyState::Unknown => "unknown",
                    msc_domain::world_profile::WorldSafetyState::Unsupported => "unsupported",
                }
                .to_string()
            } else {
                "detected".to_string()
            };
            (
                field.key().to_string(),
                WorldProfileFieldMetadataDto {
                    capability: field.capability().to_string(),
                    lifecycle: if server_type == ServerType::Bedrock
                        && matches!(
                            field,
                            WorldProfileField::GameplayDifficulty
                                | WorldProfileField::GameplayDefaultGameMode
                                | WorldProfileField::GameplayGamerules
                                | WorldProfileField::GameplayCoordinates
                        ) {
                        "restart_required".to_string()
                    } else {
                        field.apply_policy().raw_value().to_string()
                    },
                    value_state,
                    help_id: field.help_id().map(str::to_string),
                },
            )
        })
        .collect();
    WorldProfileDto {
        schema_version: profile.schema_version,
        identity: WorldIdentityDto {
            name: profile.identity.name.clone(),
            level_name: profile.identity.level_name.clone(),
            seed: profile.identity.seed.clone(),
        },
        generation: WorldGenerationDto {
            world_type: profile.generation.world_type.clone(),
            flat_preset: profile.generation.flat_preset.clone(),
            structures: profile.generation.structures,
            biome_source: profile.generation.biome_source.clone(),
            generator_options: profile.generation.generator_options.clone(),
            bonus_chest: profile.generation.bonus_chest,
            data_packs: profile.generation.data_packs.clone(),
        },
        gameplay: WorldGameplayDto {
            difficulty: profile.gameplay.difficulty.clone(),
            default_game_mode: profile.gameplay.default_game_mode.clone(),
            hardcore: profile.gameplay.hardcore,
            commands: profile.gameplay.commands,
            gamerules: profile.gameplay.gamerules.clone(),
            cheats: profile.gameplay.cheats,
            experiments: profile.gameplay.experiments.clone(),
            coordinates: profile.gameplay.coordinates,
            starting_map: profile.gameplay.starting_map,
            supported_toggles: profile.gameplay.supported_toggles.clone(),
        },
        safety: WorldSafetyDto {
            state: profile.safety.state.raw_value().to_string(),
            reasons: profile.safety.reasons.clone(),
        },
        packs: profile
            .packs
            .iter()
            .map(|pack| WorldPackRecordDto {
                id: pack.id.clone(),
                edition: pack.edition.clone(),
                kind: pack.kind.clone(),
                name: pack.name.clone(),
                source: WorldPackSourceDto {
                    provider: pack.source.provider.clone(),
                    project_id: pack.source.project_id.clone(),
                    version_id: pack.source.version_id.clone(),
                    version: pack.source.version.clone(),
                    url: pack.source.url.clone(),
                },
                files: pack.files.clone(),
                size_bytes: msc_application::addons::world_pack_size_bytes(
                    world_zip_path,
                    &pack.files,
                ),
                checksum: pack.checksum.clone(),
                compatibility: pack.compatibility.clone(),
                minecraft_versions: pack.minecraft_versions.clone(),
                enabled: pack.enabled,
                dependencies: pack
                    .dependencies
                    .iter()
                    .map(|dependency| WorldPackDependencyDto {
                        id: dependency.id.clone(),
                        kind: dependency.kind.clone(),
                        required: dependency.required,
                    })
                    .collect(),
            })
            .collect(),
        field_metadata,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorldProfileChangeDto {
    key: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorldProfileUpdateResultDto {
    success: bool,
    message: String,
    status: String,
    slot: WorldSlotWithProfileDto,
    changes: Vec<WorldProfileChangeDto>,
}

fn profile_key(key: &str) -> Option<&'static str> {
    match key {
        "identity.name" => Some("identity.name"),
        "identity.level-name" | "identity.levelName" => Some("identity.level-name"),
        "identity.seed" => Some("identity.seed"),
        "generation.world-type" | "generation.worldType" => Some("generation.world-type"),
        "generation.flat-preset" | "generation.flatPreset" => Some("generation.flat-preset"),
        "generation.structures" => Some("generation.structures"),
        "generation.biome-source" | "generation.biomeSource" => Some("generation.biome-source"),
        "generation.generator-options" | "generation.generatorOptions" => {
            Some("generation.generator-options")
        }
        "generation.bonus-chest" | "generation.bonusChest" => Some("generation.bonus-chest"),
        "generation.data-packs" | "generation.dataPacks" => Some("generation.data-packs"),
        "gameplay.difficulty" => Some("gameplay.difficulty"),
        "gameplay.default-game-mode" | "gameplay.defaultGameMode" => {
            Some("gameplay.default-game-mode")
        }
        "gameplay.hardcore" => Some("gameplay.hardcore"),
        "gameplay.commands" => Some("gameplay.commands"),
        "gameplay.gamerules" => Some("gameplay.gamerules"),
        "gameplay.cheats" => Some("gameplay.cheats"),
        "gameplay.experiments" => Some("gameplay.experiments"),
        "gameplay.coordinates" => Some("gameplay.coordinates"),
        "gameplay.starting-map" | "gameplay.startingMap" => Some("gameplay.starting-map"),
        "gameplay.supported-toggles" | "gameplay.supportedToggles" => {
            Some("gameplay.supported-toggles")
        }
        "packs" => Some("packs"),
        _ => None,
    }
}

fn collect_profile_changes(
    value: &serde_json::Value,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let Some(object) = value.as_object() else {
        return Err("request must be an object".to_string());
    };
    let source = object.get("changes").or_else(|| object.get("profile"));
    let source = source.unwrap_or(value);
    let Some(source) = source.as_object() else {
        return Err("changes must be an object".to_string());
    };
    let mut changes = BTreeMap::new();
    for (key, value) in source {
        if object.get("profile").is_some()
            && matches!(key.as_str(), "schemaVersion" | "safety" | "fieldMetadata")
        {
            continue;
        }
        if let Some(section) = ["identity", "generation", "gameplay"]
            .into_iter()
            .find(|section| *section == key)
        {
            let Some(fields) = value.as_object() else {
                return Err(format!("{key} must be an object"));
            };
            for (field, value) in fields {
                let dotted = format!("{section}.{field}");
                let normalized = profile_key(&dotted)
                    .ok_or_else(|| format!("unknown world profile field: {dotted}"))?;
                changes.insert(normalized.to_string(), value.clone());
            }
        } else {
            let normalized =
                profile_key(key).ok_or_else(|| format!("unknown world profile field: {key}"))?;
            changes.insert(normalized.to_string(), value.clone());
        }
    }
    if changes.is_empty() {
        return Err("changes must include at least one key".to_string());
    }
    Ok(changes)
}

fn optional_profile_string(value: &serde_json::Value) -> Result<Option<String>, String> {
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_str()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(Some)
        .ok_or_else(|| "value must be a non-empty string or null".to_string())
}

fn optional_profile_bool(value: &serde_json::Value) -> Result<Option<bool>, String> {
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_bool()
        .map(Some)
        .ok_or_else(|| "value must be a boolean or null".to_string())
}

fn profile_map_strings(value: &serde_json::Value) -> Result<BTreeMap<String, String>, String> {
    value
        .as_object()
        .ok_or_else(|| "value must be an object".to_string())?
        .iter()
        .map(|(key, value)| {
            value
                .as_str()
                .map(|value| (key.clone(), value.to_string()))
                .ok_or_else(|| format!("{key} must be a string"))
        })
        .collect()
}

fn profile_map_bools(value: &serde_json::Value) -> Result<BTreeMap<String, bool>, String> {
    value
        .as_object()
        .ok_or_else(|| "value must be an object".to_string())?
        .iter()
        .map(|(key, value)| {
            value
                .as_bool()
                .map(|value| (key.clone(), value))
                .ok_or_else(|| format!("{key} must be a boolean"))
        })
        .collect()
}

fn apply_profile_change(
    profile: &mut WorldProfile,
    server_type: ServerType,
    key: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    if key == "packs" {
        profile.packs = decode_world_packs(value, server_type)?;
        return Ok(());
    }
    let field = WorldProfileField::ALL
        .into_iter()
        .find(|field| field.key() == key)
        .ok_or_else(|| format!("unknown world profile field: {key}"))?;
    if !field.applies_to(server_type) {
        return Err(format!("{key} is unsupported for this server type"));
    }
    match field {
        WorldProfileField::IdentityName => profile.identity.name = optional_profile_string(value)?,
        WorldProfileField::IdentityLevelName => {
            let name = optional_profile_string(value)?;
            if let Some(name) = &name
                && (name == "." || name == ".." || name.contains(['/', '\\', ':', '\n', '\r']))
            {
                return Err("Minecraft folder name must be a single folder name".into());
            }
            profile.identity.level_name = name;
        }
        WorldProfileField::IdentitySeed => profile.identity.seed = optional_profile_string(value)?,
        WorldProfileField::GenerationWorldType => {
            let kind = optional_profile_string(value)?;
            if server_type == ServerType::Bedrock
                && kind.as_deref().is_some_and(|value| {
                    ![
                        "default",
                        "normal",
                        "minecraft:normal",
                        "flat",
                        "minecraft:flat",
                    ]
                    .contains(&value)
                })
            {
                return Err("BDS supports Default and Flat world generation".into());
            }
            profile.generation.world_type = kind;
        }
        WorldProfileField::GenerationFlatPreset => {
            profile.generation.flat_preset = optional_profile_string(value)?
        }
        WorldProfileField::GenerationStructures => {
            profile.generation.structures = optional_profile_bool(value)?
        }
        WorldProfileField::GenerationBiomeSource => {
            profile.generation.biome_source = optional_profile_string(value)?
        }
        WorldProfileField::GenerationGeneratorOptions => {
            profile.generation.generator_options = optional_profile_string(value)?
        }
        WorldProfileField::GenerationBonusChest => {
            profile.generation.bonus_chest = optional_profile_bool(value)?
        }
        WorldProfileField::GenerationDataPacks => {
            if value.is_null() {
                profile.generation.data_packs.clear();
            } else {
                profile.generation.data_packs = value
                    .as_array()
                    .ok_or_else(|| "value must be an array".to_string())?
                    .iter()
                    .map(|value| {
                        value
                            .as_str()
                            .map(str::to_string)
                            .ok_or_else(|| "data pack names must be strings".to_string())
                    })
                    .collect::<Result<_, _>>()?;
            }
        }
        WorldProfileField::GameplayDifficulty => {
            let value = optional_profile_string(value)?;
            if value
                .as_deref()
                .is_some_and(|value| !["peaceful", "easy", "normal", "hard"].contains(&value))
            {
                return Err("difficulty is not a recognized value".to_string());
            }
            profile.gameplay.difficulty = value;
        }
        WorldProfileField::GameplayDefaultGameMode => {
            let value = optional_profile_string(value)?;
            if value.as_deref().is_some_and(|value| {
                !["survival", "creative", "adventure", "spectator"].contains(&value)
            }) {
                return Err("default game mode is not a recognized value".to_string());
            }
            if server_type == ServerType::Bedrock && value.as_deref() == Some("spectator") {
                return Err("BDS does not support Spectator as the default game mode".into());
            }
            profile.gameplay.default_game_mode = value;
        }
        WorldProfileField::GameplayHardcore => {
            profile.gameplay.hardcore = optional_profile_bool(value)?
        }
        WorldProfileField::GameplayCommands => {
            profile.gameplay.commands = optional_profile_bool(value)?
        }
        WorldProfileField::GameplayGamerules => {
            let rules = profile_map_strings(value)?;
            if server_type == ServerType::Bedrock {
                for (name, value) in &rules {
                    if name.is_empty()
                        || !name.chars().all(|ch| ch.is_ascii_alphanumeric())
                        || !["true", "false"].contains(&value.as_str())
                            && value.parse::<i32>().is_err()
                            && !(name.eq_ignore_ascii_case("playerwaypoints")
                                && ["everyone", "off"].contains(&value.as_str()))
                    {
                        return Err(format!(
                            "Gamerule {name} must have a name made of letters/digits and a true, false, or integer value"
                        ));
                    }
                }
            }
            profile.gameplay.gamerules = rules;
        }
        WorldProfileField::GameplayCheats => {
            profile.gameplay.cheats = optional_profile_bool(value)?
        }
        WorldProfileField::GameplayExperiments => {
            profile.gameplay.experiments = profile_map_bools(value)?
        }
        WorldProfileField::GameplayCoordinates => {
            profile.gameplay.coordinates = optional_profile_bool(value)?
        }
        WorldProfileField::GameplayStartingMap => {
            profile.gameplay.starting_map = optional_profile_bool(value)?
        }
        WorldProfileField::GameplaySupportedToggles => {
            profile.gameplay.supported_toggles = profile_map_bools(value)?
        }
        WorldProfileField::SafetyState => {
            return Err("safety.state is read-only".to_string());
        }
    }
    Ok(())
}

fn decode_world_packs(
    value: &serde_json::Value,
    server_type: ServerType,
) -> Result<Vec<WorldPackRecord>, String> {
    if value.is_null() {
        return Ok(Vec::new());
    }
    let packs: Vec<WorldPackRecord> = serde_json::from_value(value.clone())
        .map_err(|_| "packs must be an array of valid world-pack records".to_string())?;
    let expected_kind = match server_type {
        ServerType::Java => "java_datapack",
        ServerType::Bedrock => "bedrock_behavior_pack",
    };
    let mut ids = std::collections::BTreeSet::new();
    for pack in &packs {
        if pack.id.trim().is_empty() || pack.name.trim().is_empty() {
            return Err("world-pack id and name must not be empty".to_string());
        }
        if pack.edition != server_type.raw_value()
            || !(pack.kind == expected_kind
                || server_type == ServerType::Bedrock && pack.kind == "bedrock_resource_pack")
        {
            return Err(format!(
                "{} records are not supported for this server edition",
                pack.kind
            ));
        }
        if !ids.insert(&pack.id) {
            return Err(format!("duplicate world-pack id: {}", pack.id));
        }
        for path in &pack.files {
            let path = std::path::Path::new(path);
            if path.as_os_str().is_empty()
                || path.is_absolute()
                || path.components().any(|part| {
                    matches!(
                        part,
                        std::path::Component::ParentDir | std::path::Component::RootDir
                    )
                })
                || path.to_string_lossy().contains(['\\', ':'])
            {
                return Err("world-pack file paths must stay inside the selected world".to_string());
            }
        }
    }
    Ok(packs)
}

pub async fn get_profile(
    State(state): State<WorldsRoutesState>,
    AxumPath(slot_id): AxumPath<String>,
) -> Response {
    let Some(server) = state.lifecycle.active_config_server() else {
        return no_active_server();
    };
    let server_dir = Path::new(&server.server_dir);
    let Some(slot) = find_slot(server_dir, &slot_id) else {
        return error_response(StatusCode::NOT_FOUND, "not_found", "World slot not found.");
    };
    let profile = world_store::load_profile(&StdFileSystem, server_dir, &slot);
    Json(WorldSlotWithProfileDto {
        slot: to_slot_dto(
            &slot,
            resolved_active_slot_id(server_dir).as_deref() == Some(slot.id.as_str()),
        ),
        profile: profile_to_dto(
            &profile,
            server.server_type,
            &worlds::world_pack_archive_path(server_dir, &slot.id),
        ),
    })
    .into_response()
}

pub async fn install_java_datapack(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot_id): AxumPath<String>,
    body: Result<Json<JavaDatapackInstallRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let server = match active_server_or_response(&state.lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    if server.server_type != ServerType::Java {
        return error_response(
            StatusCode::CONFLICT,
            "unsupported_server_type",
            "Java datapacks can only be installed in Java worlds.",
        );
    }
    let Some(minecraft_version) = server.minecraft_version.as_deref() else {
        return error_response(
            StatusCode::CONFLICT,
            "unknown_minecraft_version",
            "The server's Minecraft version is not available.",
        );
    };
    let server_dir = Path::new(&server.server_dir);
    let Some(slot) = find_slot(server_dir, &slot_id) else {
        return error_response(StatusCode::NOT_FOUND, "not_found", "World slot not found.");
    };
    if resolved_active_slot_id(server_dir).as_deref() == Some(slot.id.as_str())
        && state.lifecycle.status_snapshot().running
    {
        return error_response(
            StatusCode::CONFLICT,
            "server_running",
            "Stop the server before changing a datapack in its active world.",
        );
    }

    let local = body.staged_upload_id.is_some();
    let (archive, catalog, local_name) = if let Some(id) = &body.staged_upload_id {
        if !body.project_id.is_empty() || !body.version_id.is_empty() {
            return invalid_body(
                "invalid_pack_source",
                "Choose a catalog release or an uploaded file, not both.",
            );
        }
        let entry = state.staging.uploads.lock().unwrap().remove(id);
        let Some(entry) = entry else {
            return invalid_body("invalid_upload", "The uploaded datapack is unavailable.");
        };
        if now_unix() > entry.expires_at_unix
            || !entry.complete
            || entry.purpose != StagedUploadPurposeDto::AddonLocalFile
        {
            let _ = std::fs::remove_file(&entry.path);
            return invalid_body(
                "invalid_upload",
                "The uploaded datapack is expired, incomplete, or has the wrong purpose.",
            );
        }
        let archive = std::fs::read(&entry.path);
        let _ = std::fs::remove_file(&entry.path);
        let archive = match archive {
            Ok(bytes) => bytes,
            Err(error) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "upload_read_failed",
                    &error.to_string(),
                );
            }
        };
        (
            archive,
            None,
            entry
                .file_name
                .unwrap_or_else(|| "Imported datapack".into()),
        )
    } else {
        if body.project_id.trim().is_empty() || body.version_id.trim().is_empty() {
            return invalid_body(
                "invalid_pack_source",
                "Choose a catalog release or upload a datapack ZIP.",
            );
        }
        let transport = msc_infrastructure::addon_provider::HttpTransport::new();
        let version = match msc_infrastructure::addon_provider::modrinth_version(
            &transport,
            &body.version_id,
        ) {
            Ok(version) if version.project_id == body.project_id => version,
            Ok(_) => {
                return invalid_body(
                    "version_project_mismatch",
                    "The selected version does not belong to that Modrinth project.",
                );
            }
            Err(error) => {
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    "provider_error",
                    &error.to_string(),
                );
            }
        };
        if !version.loaders.iter().any(|loader| loader == "datapack") {
            return invalid_body(
                "invalid_datapack",
                "Choose a datapack release. Fabric, NeoForge and other mod builds cannot be installed as datapacks.",
            );
        }
        if !version
            .game_versions
            .iter()
            .any(|value| value == minecraft_version)
        {
            return error_response(
                StatusCode::CONFLICT,
                "incompatible_version",
                "This datapack version does not list the server's Minecraft version.",
            );
        }
        let Some(file) = msc_domain::addon_provider::modrinth_primary_file(&version.files) else {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "missing_archive",
                "The selected Modrinth version has no datapack archive.",
            );
        };
        let archive =
            match msc_infrastructure::addon_provider::download_datapack(&transport, &file.url) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return error_response(
                        StatusCode::BAD_GATEWAY,
                        "download_failed",
                        &error.to_string(),
                    );
                }
            };
        let project = match msc_infrastructure::addon_provider::modrinth_project(
            &transport,
            &body.project_id,
        ) {
            Ok(project) => project,
            Err(error) => {
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    "provider_error",
                    &error.to_string(),
                );
            }
        };
        (archive, Some((version, project)), String::new())
    };
    let operation_id = match begin_operation(
        &state.lifecycle,
        &server.id,
        "world-datapack-install",
        "Installing Java datapack.",
    ) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let active = resolved_active_slot_id(server_dir).as_deref() == Some(slot.id.as_str());
    let original_profile = world_store::load_profile(&StdFileSystem, server_dir, &slot);
    let level_name = worlds::read_configured_level_name(&StdFileSystem, server_dir)
        .unwrap_or_else(|| slot.name.clone());
    let slot = if active && server_dir.join(&level_name).join("level.dat").is_file() {
        match worlds::update_active_slot_from_current_world(
            &StdFileSystem,
            server_dir,
            ServerType::Java,
            Some(&level_name),
            &slot,
        )
        .and_then(|updated| {
            world_store::save_profile(&StdFileSystem, server_dir, &updated, &original_profile)?;
            Ok(updated)
        }) {
            Ok(updated) => updated,
            Err(error) => {
                let _ = state.lifecycle.operations().fail(
                    &operation_id,
                    "world_snapshot_failed",
                    error.to_string(),
                );
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "world_snapshot_failed",
                    &error.to_string(),
                );
            }
        }
    } else {
        slot
    };
    let pack_archive =
        match worlds::prepare_world_pack_archive(server_dir, server.server_type, &slot) {
            Ok(path) => path,
            Err(error) => {
                let _ = state.lifecycle.operations().fail(
                    &operation_id,
                    "world_write_failed",
                    error.to_string(),
                );
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "world_write_failed",
                    &error.to_string(),
                );
            }
        };
    let installation = if let Some((version, project)) = &catalog {
        msc_application::addons::install_java_datapack(
            &pack_archive,
            &archive,
            version,
            &body.project_id,
            &project.title,
            minecraft_version,
        )
    } else {
        msc_application::addons::install_local_java_datapack(&pack_archive, &archive, &local_name)
    };
    let (name, checksum, installed_paths, backup_path) = match installation {
        Ok(result) => result,
        Err(msc_application::addons::JavaDatapackError::IncompatibleVersion) => {
            let _ = state.lifecycle.operations().fail(
                &operation_id,
                "incompatible_version",
                "This datapack version does not list the server's Minecraft version.".to_string(),
            );
            return error_response(
                StatusCode::CONFLICT,
                "incompatible_version",
                "This datapack version does not list the server's Minecraft version.",
            );
        }
        Err(error) => {
            let _ = state.lifecycle.operations().fail(
                &operation_id,
                "invalid_datapack",
                error.to_string(),
            );
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_datapack",
                &error.to_string(),
            );
        }
    };
    let mut profile = world_store::load_profile(&StdFileSystem, server_dir, &slot);
    let (id, source, minecraft_versions) = if let Some((version, project)) = catalog {
        (
            format!("{}:{}", body.project_id, version.id),
            msc_domain::world_profile::WorldPackSource {
                provider: Some("modrinth".into()),
                project_id: Some(body.project_id),
                version_id: Some(version.id.clone()),
                version: Some(version.version_number),
                url: Some(format!(
                    "https://modrinth.com/datapack/{}/version/{}",
                    project.slug, version.id
                )),
            },
            version.game_versions,
        )
    } else {
        (
            format!("local:{checksum}"),
            msc_domain::world_profile::WorldPackSource {
                provider: Some("local-file".into()),
                project_id: None,
                version_id: None,
                version: None,
                url: None,
            },
            Vec::new(),
        )
    };
    let pack = WorldPackRecord {
        id,
        edition: "java".into(),
        kind: "java_datapack".into(),
        name,
        source,
        files: installed_paths,
        checksum: Some(format!("sha512:{checksum}")),
        compatibility: Some(if local { "unknown" } else { "compatible" }.into()),
        minecraft_versions,
        enabled: true,
        dependencies: Vec::new(),
    };
    profile.packs.retain(|existing| existing.id != pack.id);
    profile.packs.push(pack.clone());
    if let Err(error) = world_store::save_profile(&StdFileSystem, server_dir, &slot, &profile) {
        let _ = std::fs::copy(&backup_path, &pack_archive);
        let _ = state.lifecycle.operations().fail(
            &operation_id,
            "profile_write_failed",
            error.to_string(),
        );
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "profile_write_failed",
            &error.to_string(),
        );
    }
    if active {
        let raw_level_name = worlds::read_configured_level_name(&StdFileSystem, server_dir);
        if let Err(error) = worlds::activate_slot(
            &StdFileSystem,
            server_dir,
            ServerType::Java,
            &slot,
            false,
            &iso8601_now(),
            || {
                run_pre_mutation_safety_backup(
                    &state.lifecycle,
                    server_dir,
                    ServerType::Java,
                    raw_level_name.as_deref(),
                    || false,
                )
            },
            || false,
        ) {
            let _ = state.lifecycle.operations().fail(
                &operation_id,
                "pack_activation_failed",
                error.to_string(),
            );
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "pack_activation_failed",
                &error.to_string(),
            );
        }
    }
    let mut result = BTreeMap::new();
    result.insert("packId".to_string(), pack.id.clone());
    let _ = state
        .lifecycle
        .operations()
        .succeed(&operation_id, "Java datapack installed.", result);
    let response = Json(JavaDatapackInstallResultDto {
        result: "installed".to_string(),
        pack: WorldPackRecordDto {
            id: pack.id,
            edition: pack.edition,
            kind: pack.kind,
            name: pack.name,
            source: WorldPackSourceDto {
                provider: pack.source.provider,
                project_id: pack.source.project_id,
                version_id: pack.source.version_id,
                version: pack.source.version,
                url: pack.source.url,
            },
            size_bytes: msc_application::addons::world_pack_size_bytes(&pack_archive, &pack.files),
            files: pack.files,
            checksum: pack.checksum,
            compatibility: pack.compatibility,
            minecraft_versions: pack.minecraft_versions,
            enabled: pack.enabled,
            dependencies: Vec::new(),
        },
    })
    .into_response();
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/worlds/{slotId}/datapacks/install",
        response.status(),
    );
    response
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BedrockBehaviorPackSearchQuery {
    q: Option<String>,
    game_version: Option<String>,
    offset: Option<u32>,
    kind: Option<String>,
}

pub async fn search_bedrock_behavior_packs(
    Extension(_credential): Extension<AuthenticatedCredential>,
    Query(query): Query<BedrockBehaviorPackSearchQuery>,
) -> Response {
    let secrets = match crate::auth::production_secret_store() {
        Ok(secrets) => secrets,
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "secret_store_unavailable",
                &error.to_string(),
            );
        }
    };
    let kind = query.kind.as_deref().unwrap_or("behavior");
    if !["all", "resource", "behavior"].contains(&kind) {
        return invalid_body(
            "invalid_pack_kind",
            "Pack kind must be all, resource, or behavior.",
        );
    }
    let transport = msc_infrastructure::addon_provider::HttpTransport::new();
    let results = match msc_infrastructure::addon_provider::curseforge_search_bedrock_packs(
        &transport,
        secrets.as_ref(),
        query.q.as_deref().unwrap_or_default(),
        query.game_version.as_deref(),
        20,
        query.offset.unwrap_or(0),
        kind,
    ) {
        Ok(results) => results,
        Err(error) => {
            let (status, code) = match error {
                msc_domain::addon_provider::AddonProviderError::MissingApiKey => {
                    (StatusCode::CONFLICT, "missing_curseforge_api_key")
                }
                msc_domain::addon_provider::AddonProviderError::Unauthorized => {
                    (StatusCode::UNAUTHORIZED, "curseforge_unauthorized")
                }
                _ => (StatusCode::BAD_GATEWAY, "provider_error"),
            };
            return error_response(status, code, &error.to_string());
        }
    };
    Json(BedrockBehaviorPackSearchResponseDto {
        results: results
            .into_iter()
            .map(|hit| {
                let file = query
                    .game_version
                    .as_ref()
                    .and_then(|version| {
                        hit.latest_files_indexes
                            .iter()
                            .find(|file| file.game_version == *version)
                    })
                    .or_else(|| hit.latest_files_indexes.first());
                msc_api::dto::BedrockBehaviorPackCatalogItemDto {
                    project_id: hit.id.to_string(),
                    slug: hit.slug,
                    title: hit.name,
                    description: hit.summary,
                    downloads: hit.download_count,
                    icon_url: hit.logo.map(|logo| logo.url),
                    file_id: file.map(|file| file.file_id).unwrap_or_default(),
                    file_name: file
                        .map(|file| file.filename.clone())
                        .unwrap_or_else(|| "Open to choose a file".into()),
                    minecraft_version: file
                        .map(|file| file.game_version.clone())
                        .unwrap_or_default(),
                }
            })
            .collect(),
        game_version: query.game_version,
    })
    .into_response()
}

pub async fn get_bedrock_behavior_pack_detail(
    Extension(_credential): Extension<AuthenticatedCredential>,
    AxumPath(project_id): AxumPath<String>,
) -> Response {
    let project_id = match project_id.parse::<i64>() {
        Ok(id) if id > 0 => id,
        _ => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "invalid_project_id",
                "The CurseForge project ID must be a positive integer.",
            );
        }
    };
    let secrets = match crate::auth::production_secret_store() {
        Ok(secrets) => secrets,
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "secret_store_unavailable",
                &error.to_string(),
            );
        }
    };
    let transport = msc_infrastructure::addon_provider::HttpTransport::new();
    let (project, description, files) =
        match msc_infrastructure::addon_provider::curseforge_bedrock_project_detail(
            &transport,
            secrets.as_ref(),
            project_id,
        ) {
            Ok(detail) => detail,
            Err(error) => {
                let (status, code) = match error {
                    msc_domain::addon_provider::AddonProviderError::MissingApiKey => {
                        (StatusCode::CONFLICT, "missing_curseforge_api_key")
                    }
                    msc_domain::addon_provider::AddonProviderError::Unauthorized => {
                        (StatusCode::UNAUTHORIZED, "curseforge_unauthorized")
                    }
                    _ => (StatusCode::BAD_GATEWAY, "provider_error"),
                };
                return error_response(status, code, &error.to_string());
            }
        };

    Json(msc_api::dto::BedrockBehaviorPackDetailDto {
        project_id: project.id.to_string(),
        slug: project.slug,
        title: project.name,
        author: project.authors.first().map(|author| author.name.clone()),
        description,
        downloads: project.download_count,
        icon_url: project.logo.map(|logo| logo.url),
        source_url: project.links.website_url,
        gallery: project
            .screenshots
            .into_iter()
            .map(|image| msc_api::dto::BedrockBehaviorPackImageDto {
                title: image.title,
                url: image.url,
            })
            .collect(),
        files: files
            .into_iter()
            .map(|file| msc_api::dto::BedrockBehaviorPackFileDto {
                id: file.id,
                display_name: file.display_name.unwrap_or_else(|| file.file_name.clone()),
                file_name: file.file_name,
                release_type: file.release_type,
                downloads: file.download_count,
                file_date: file.file_date,
                game_versions: file.game_versions,
            })
            .collect(),
    })
    .into_response()
}

pub async fn install_bedrock_behavior_pack(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot_id): AxumPath<String>,
    body: Result<Json<serde_json::Value>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let server = match active_server_or_response(&state.lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    if server.server_type != ServerType::Bedrock {
        return error_response(
            StatusCode::CONFLICT,
            "unsupported_server_type",
            "Bedrock packs can only be installed in Bedrock worlds.",
        );
    }
    let runtime = crate::routes::bedrock::runtime_for(&state.lifecycle);
    let Some(bedrock_version) =
        crate::routes::versions::installed_bedrock_version(&server, runtime.as_ref())
    else {
        return error_response(
            StatusCode::CONFLICT,
            "unknown_bedrock_version",
            "The installed Bedrock server version could not be verified. Repair the server files before installing packs.",
        );
    };
    let server_dir = Path::new(&server.server_dir);
    let Some(slot) = find_slot(server_dir, &slot_id) else {
        return error_response(StatusCode::NOT_FOUND, "not_found", "World slot not found.");
    };
    if resolved_active_slot_id(server_dir).as_deref() == Some(slot.id.as_str())
        && state.lifecycle.status_snapshot().running
    {
        return error_response(
            StatusCode::CONFLICT,
            "server_running",
            "Stop the server before changing packs in its active world.",
        );
    }
    let (archive, source_project_id, source_file_id, source_url) = if let Some(id) = body
        .get("stagedUploadId")
        .and_then(serde_json::Value::as_str)
    {
        if body.get("projectId").is_some() || body.get("fileId").is_some() {
            return invalid_body(
                "invalid_pack_source",
                "Choose a catalog file or an uploaded file, not both.",
            );
        }
        let entry = state.staging.uploads.lock().unwrap().remove(id);
        let Some(entry) = entry else {
            return invalid_body("invalid_upload", "The uploaded pack is unavailable.");
        };
        if now_unix() > entry.expires_at_unix
            || !entry.complete
            || entry.purpose != StagedUploadPurposeDto::AddonLocalFile
        {
            let _ = std::fs::remove_file(&entry.path);
            return invalid_body(
                "invalid_upload",
                "The uploaded pack is expired, incomplete, or has the wrong purpose.",
            );
        }
        let archive = std::fs::read(&entry.path);
        let _ = std::fs::remove_file(&entry.path);
        let archive = match archive {
            Ok(bytes) => bytes,
            Err(error) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "upload_read_failed",
                    &error.to_string(),
                );
            }
        };
        (
            archive,
            entry.file_name.unwrap_or_else(|| "Imported pack".into()),
            String::new(),
            "local-file".into(),
        )
    } else {
        let body: BedrockBehaviorPackInstallRequestDto = match serde_json::from_value(body) {
            Ok(body) => body,
            Err(_) => {
                return invalid_body(
                    "invalid_pack_source",
                    "Choose a catalog file or upload a pack.",
                );
            }
        };
        let project_id = match body.project_id.parse::<i64>() {
            Ok(id) if id > 0 => id,
            _ => {
                return invalid_body(
                    "invalid_project_id",
                    "CurseForge project ID must be a positive number.",
                );
            }
        };
        let secrets = match crate::auth::production_secret_store() {
            Ok(secrets) => secrets,
            Err(error) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "secret_store_unavailable",
                    &error.to_string(),
                );
            }
        };
        let transport = msc_infrastructure::addon_provider::HttpTransport::new();
        let file = match msc_infrastructure::addon_provider::curseforge_files(
            &transport,
            secrets.as_ref(),
            &[body.file_id],
        ) {
            Ok(files) => match files
                .into_iter()
                .find(|file| file.id == body.file_id && file.mod_id == project_id)
            {
                Some(file) => file,
                None => {
                    return error_response(
                        StatusCode::NOT_FOUND,
                        "file_not_found",
                        "That file does not belong to the selected CurseForge project.",
                    );
                }
            },
            Err(error) => {
                let (status, code) = match error {
                    msc_domain::addon_provider::AddonProviderError::MissingApiKey => {
                        (StatusCode::CONFLICT, "missing_curseforge_api_key")
                    }
                    msc_domain::addon_provider::AddonProviderError::Unauthorized => {
                        (StatusCode::UNAUTHORIZED, "curseforge_unauthorized")
                    }
                    _ => (StatusCode::BAD_GATEWAY, "provider_error"),
                };
                return error_response(status, code, &error.to_string());
            }
        };
        let download_url = match file.download_url.clone() {
            Some(url) => url,
            None => match msc_infrastructure::addon_provider::curseforge_bedrock_file_download_url(
                &transport,
                secrets.as_ref(),
                project_id,
                body.file_id,
            ) {
                Ok(Some(url)) => url,
                Ok(None) => {
                    return error_response(
                        StatusCode::CONFLICT,
                        "manual_download_required",
                        "CurseForge does not permit this file to be downloaded through the API.",
                    );
                }
                Err(error) => {
                    return error_response(
                        StatusCode::BAD_GATEWAY,
                        "provider_error",
                        &error.to_string(),
                    );
                }
            },
        };
        let archive = match msc_infrastructure::addon_provider::download_curseforge_bedrock_addon(
            &transport,
            &download_url,
        ) {
            Ok(bytes) => bytes,
            Err(error) => {
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    "download_failed",
                    &error.to_string(),
                );
            }
        };
        let project = match msc_infrastructure::addon_provider::curseforge_mods(
            &transport,
            secrets.as_ref(),
            &[project_id],
        ) {
            Ok(projects) => projects
                .into_iter()
                .find(|project| project.id == project_id),
            Err(error) => {
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    "provider_error",
                    &error.to_string(),
                );
            }
        };
        (
            archive,
            body.project_id,
            body.file_id.to_string(),
            project
                .as_ref()
                .and_then(|project| project.website_url())
                .unwrap_or("https://www.curseforge.com/minecraft-bedrock")
                .to_owned(),
        )
    };
    let operation_id = match begin_operation(
        &state.lifecycle,
        &server.id,
        "world-behavior-pack-install",
        "Installing Bedrock world pack.",
    ) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let active = resolved_active_slot_id(server_dir).as_deref() == Some(slot.id.as_str());
    let original_profile = world_store::load_profile(&StdFileSystem, server_dir, &slot);
    // Refresh an active world's archive before modifying it, preserving gameplay
    // since its last save. Restore its pack/preferences metadata after detection.
    let slot = if active
        && server_dir
            .join("worlds")
            .join(
                worlds::read_configured_level_name(&StdFileSystem, server_dir)
                    .unwrap_or_else(|| slot.name.clone()),
            )
            .join("db")
            .is_dir()
    {
        match worlds::update_active_slot_from_current_world(
            &StdFileSystem,
            server_dir,
            ServerType::Bedrock,
            None,
            &slot,
        )
        .and_then(|updated| {
            world_store::save_profile(&StdFileSystem, server_dir, &updated, &original_profile)?;
            Ok(updated)
        }) {
            Ok(updated) => updated,
            Err(error) => {
                let _ = state.lifecycle.operations().fail(
                    &operation_id,
                    "world_snapshot_failed",
                    error.to_string(),
                );
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "world_snapshot_failed",
                    &error.to_string(),
                );
            }
        }
    } else {
        slot
    };
    let pack_archive =
        match worlds::prepare_world_pack_archive(server_dir, server.server_type, &slot) {
            Ok(path) => path,
            Err(error) => {
                let _ = state.lifecycle.operations().fail(
                    &operation_id,
                    "world_write_failed",
                    error.to_string(),
                );
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "world_write_failed",
                    &error.to_string(),
                );
            }
        };
    let (packs, backup_path) = match msc_application::addons::install_bedrock_behavior_pack(
        &pack_archive,
        &archive,
        &source_project_id,
        &source_file_id,
        &source_url,
        &bedrock_version,
    ) {
        Ok(installed) => installed,
        Err(error) => {
            let (status, code) = match &error {
                msc_application::addons::BedrockBehaviorPackError::IncompatibleVersion => {
                    (StatusCode::CONFLICT, "incompatible_version")
                }
                msc_application::addons::BedrockBehaviorPackError::Invalid(_) => {
                    (StatusCode::UNPROCESSABLE_ENTITY, "invalid_behavior_pack")
                }
                msc_application::addons::BedrockBehaviorPackError::Io(_) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, "world_write_failed")
                }
            };
            let _ = state
                .lifecycle
                .operations()
                .fail(&operation_id, code, error.to_string());
            return error_response(status, code, &error.to_string());
        }
    };
    let original_profile = world_store::load_profile(&StdFileSystem, server_dir, &slot);
    let mut profile = original_profile.clone();
    for pack in &packs {
        profile.packs.retain(|existing| existing.id != pack.id);
        profile.packs.push(pack.clone());
    }
    if let Err(error) = world_store::save_profile(&StdFileSystem, server_dir, &slot, &profile) {
        let world_path = &pack_archive;
        let _ = std::fs::remove_file(world_path);
        let _ = std::fs::copy(&backup_path, world_path);
        let _ = state.lifecycle.operations().fail(
            &operation_id,
            "profile_write_failed",
            error.to_string(),
        );
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "profile_write_failed",
            &error.to_string(),
        );
    }
    if active {
        let raw_level_name = worlds::read_configured_level_name(&StdFileSystem, server_dir);
        if let Err(error) = worlds::activate_slot(
            &StdFileSystem,
            server_dir,
            ServerType::Bedrock,
            &slot,
            false,
            &iso8601_now(),
            || {
                run_pre_mutation_safety_backup(
                    &state.lifecycle,
                    server_dir,
                    ServerType::Bedrock,
                    raw_level_name.as_deref(),
                    || false,
                )
            },
            || false,
        ) {
            let _ = state.lifecycle.operations().fail(
                &operation_id,
                "pack_activation_failed",
                error.to_string(),
            );
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "pack_activation_failed",
                &error.to_string(),
            );
        }
    }
    let result = packs
        .iter()
        .map(|pack| ("packId".to_string(), pack.id.clone()))
        .collect();
    let _ = state.lifecycle.operations().succeed(
        &operation_id,
        "Bedrock world pack installed.",
        result,
    );
    let response = Json(BedrockBehaviorPackInstallResultDto {
        operation_id: operation_id.as_str().to_string(),
        packs: packs
            .into_iter()
            .map(|pack| WorldPackRecordDto {
                id: pack.id,
                edition: pack.edition,
                kind: pack.kind,
                name: pack.name,
                source: WorldPackSourceDto {
                    provider: pack.source.provider,
                    project_id: pack.source.project_id,
                    version_id: pack.source.version_id,
                    version: pack.source.version,
                    url: pack.source.url,
                },
                size_bytes: msc_application::addons::world_pack_size_bytes(
                    &pack_archive,
                    &pack.files,
                ),
                files: pack.files,
                checksum: pack.checksum,
                compatibility: pack.compatibility,
                minecraft_versions: pack.minecraft_versions,
                enabled: pack.enabled,
                dependencies: pack
                    .dependencies
                    .into_iter()
                    .map(|dependency| WorldPackDependencyDto {
                        id: dependency.id,
                        kind: dependency.kind,
                        required: dependency.required,
                    })
                    .collect(),
            })
            .collect(),
    })
    .into_response();
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/worlds/{slotId}/behaviorpacks/install",
        response.status(),
    );
    response
}

pub async fn update_profile(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot_id): AxumPath<String>,
    body: Result<Json<serde_json::Value>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let changes = match collect_profile_changes(&body) {
        Ok(changes) => changes,
        Err(message) => return invalid_body("invalid_body", &message),
    };
    let server = match active_server_or_response(&state.lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let server_dir = Path::new(&server.server_dir);
    let Some(slot) = find_slot(server_dir, &slot_id) else {
        return error_response(StatusCode::NOT_FOUND, "not_found", "World slot not found.");
    };
    let original_profile = world_store::load_profile(&StdFileSystem, server_dir, &slot);
    let mut profile = original_profile.clone();
    for (key, value) in &changes {
        if let Err(message) = apply_profile_change(&mut profile, server.server_type, key, value) {
            return invalid_body("invalid_body", &message);
        }
    }
    if server.server_type == ServerType::Java
        && let Err(error) = msc_application::java_world_settings::validate(&profile)
    {
        return invalid_body("invalid_body", &error.to_string());
    }
    if let Err(message) = msc_infrastructure::gamerule_catalog::validate_values(
        server.server_type.raw_value(),
        gamerule_server_version(&server, &state.lifecycle).as_deref(),
        &profile.gameplay.gamerules,
    ) {
        return invalid_body("invalid_gamerule", &message);
    }
    let confirmation = body
        .get("confirmation")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    if let Some(required) =
        world_safety::confirmation_for_profile_changes(server.server_type, &changes)
        && !world_safety::is_confirmed(required, confirmation.as_deref())
    {
        return confirmation_required_response(required);
    }

    let active = resolved_active_slot_id(server_dir).as_deref() == Some(slot.id.as_str());
    let running = state.lifecycle.status_snapshot().running;
    if active
        && server.server_type == ServerType::Java
        && profile.identity.level_name != original_profile.identity.level_name
    {
        if running {
            return error_response(
                StatusCode::CONFLICT,
                "server_running",
                "Stop the server before changing the active world's folder name.",
            );
        }
        if !run_pre_mutation_safety_backup(
            &state.lifecycle,
            server_dir,
            ServerType::Java,
            original_profile.identity.level_name.as_deref(),
            || false,
        ) {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "backup_failed",
                "The safety backup failed; the world folder was not changed.",
            );
        }
    }
    let changes_packs =
        changes.iter().any(|(key, _)| key == "packs") && profile.packs != original_profile.packs;
    let mut pack_backup = None;
    let mut pack_mutation: Option<(WorldPackRecord, Option<bool>)> = None;
    if changes_packs {
        if changes.len() != 1 {
            return invalid_body(
                "one_profile_change_required",
                "Change packs separately from other world settings.",
            );
        }
        if active && running {
            return error_response(
                StatusCode::CONFLICT,
                "server_running",
                "Stop the server before changing packs in its active world.",
            );
        }
        let added: Vec<_> = profile
            .packs
            .iter()
            .filter(|pack| !original_profile.packs.iter().any(|old| old.id == pack.id))
            .collect();
        if !added.is_empty() {
            return invalid_body(
                "pack_install_required",
                "Install new packs through Browse Packs.",
            );
        }
        for old in &original_profile.packs {
            if let Some(candidate) = profile.packs.iter().find(|pack| pack.id == old.id) {
                let mut metadata_only_enabled = candidate.clone();
                metadata_only_enabled.enabled = old.enabled;
                if metadata_only_enabled != *old {
                    return invalid_body(
                        "pack_metadata_read_only",
                        "Pack details can only be changed through pack install and management actions.",
                    );
                }
            }
        }
        let removed: Vec<_> = original_profile
            .packs
            .iter()
            .filter(|old| !profile.packs.iter().any(|pack| pack.id == old.id))
            .collect();
        let toggled: Vec<_> = original_profile
            .packs
            .iter()
            .filter(|old| {
                profile
                    .packs
                    .iter()
                    .find(|pack| pack.id == old.id)
                    .is_some_and(|pack| pack.enabled != old.enabled)
            })
            .collect();
        if removed.len() + toggled.len() != 1 {
            return invalid_body(
                "one_pack_change_required",
                "Change one installed pack at a time.",
            );
        }
        let pack = removed
            .first()
            .copied()
            .or_else(|| toggled.first().copied())
            .unwrap();
        let next_enabled = profile
            .packs
            .iter()
            .find(|candidate| candidate.id == pack.id)
            .map(|candidate| candidate.enabled);
        if server.server_type != ServerType::Bedrock
            && (pack.edition != "java" || pack.kind != "java_datapack" || next_enabled.is_some())
        {
            return invalid_body(
                "unsupported_pack_change",
                "Java datapacks can only be deleted here.",
            );
        }
        let dependent = original_profile.packs.iter().any(|candidate| {
            candidate.enabled
                && candidate.id != pack.id
                && candidate.dependencies.iter().any(|dependency| {
                    dependency.id.eq_ignore_ascii_case(&pack.id) && dependency.required
                })
        });
        if dependent && next_enabled != Some(true) {
            return error_response(
                StatusCode::CONFLICT,
                "required_pack_dependency",
                "Another enabled pack requires this pack. Remove or disable that pack first.",
            );
        }
        if next_enabled == Some(true)
            && pack.dependencies.iter().any(|dependency| {
                dependency.required
                    && !profile.packs.iter().any(|candidate| {
                        candidate.id.eq_ignore_ascii_case(&dependency.id) && candidate.enabled
                    })
            })
        {
            return error_response(
                StatusCode::CONFLICT,
                "required_pack_dependency",
                "Enable this pack's required packs first.",
            );
        }
        pack_mutation = Some((pack.clone(), next_enabled));
    }
    let mut updated_slot = slot.clone();
    if let Some(name) = profile.identity.name.as_deref() {
        updated_slot.name = name.to_string();
    }
    if let Some(level_name) = profile.identity.level_name.as_deref() {
        updated_slot.world_level_name = Some(level_name.to_string());
    }
    let level_name = worlds::read_configured_level_name(&StdFileSystem, server_dir)
        .unwrap_or_else(|| slot.name.clone());
    let generated = if server.server_type == ServerType::Java {
        server_dir.join(&level_name).join("level.dat").is_file()
    } else {
        server_dir
            .join("worlds")
            .join(&level_name)
            .join("db")
            .is_dir()
    };
    let updated_slot = if changes_packs && active && generated {
        match worlds::update_active_slot_from_current_world(
            &StdFileSystem,
            server_dir,
            server.server_type,
            original_profile.identity.level_name.as_deref(),
            &updated_slot,
        )
        .and_then(|updated| {
            world_store::save_profile(&StdFileSystem, server_dir, &updated, &original_profile)?;
            Ok(updated)
        }) {
            Ok(updated) => updated,
            Err(error) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "world_snapshot_failed",
                    &error.to_string(),
                );
            }
        }
    } else {
        updated_slot
    };
    if changes_packs {
        let (pack, next_enabled) = pack_mutation.as_ref().expect("validated above");
        match msc_application::addons::mutate_world_pack(
            &worlds::world_pack_archive_path(server_dir, &updated_slot.id),
            pack,
            *next_enabled,
        ) {
            Ok(backup) => pack_backup = Some(backup),
            Err(error) => {
                return error_response(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "pack_change_failed",
                    &error.to_string(),
                );
            }
        }
    }
    if let Err(error) =
        world_store::save_profile(&StdFileSystem, server_dir, &updated_slot, &profile)
    {
        if let Some(backup) = &pack_backup {
            let _ = std::fs::copy(
                backup,
                worlds::world_pack_archive_path(server_dir, &updated_slot.id),
            );
        }
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        );
    }

    if changes_packs && active {
        let raw_level_name = worlds::read_configured_level_name(&StdFileSystem, server_dir);
        if let Err(error) = worlds::activate_slot(
            &StdFileSystem,
            server_dir,
            server.server_type,
            &updated_slot,
            false,
            &iso8601_now(),
            || {
                run_pre_mutation_safety_backup(
                    &state.lifecycle,
                    server_dir,
                    server.server_type,
                    raw_level_name.as_deref(),
                    || false,
                )
            },
            || false,
        ) {
            if let Some(backup) = &pack_backup {
                let _ = std::fs::copy(
                    backup,
                    worlds::world_pack_archive_path(server_dir, &updated_slot.id),
                );
            }
            let _ = world_store::save_profile(
                &StdFileSystem,
                server_dir,
                &updated_slot,
                &original_profile,
            );
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "pack_activation_failed",
                &error.to_string(),
            );
        }
    }

    let mut report = if active && !changes_packs {
        match worlds::apply_world_profile(
            &StdFileSystem,
            server_dir,
            server.server_type,
            &profile,
            worlds::WorldProfileApplyContext::Activation,
            running,
        ) {
            Ok(report) => report,
            Err(error) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    &error.to_string(),
                );
            }
        }
    } else {
        worlds::WorldProfileApplicationReport::default()
    };
    if active && running && server.server_type == ServerType::Bedrock {
        for (key, command, value) in [
            (
                "gameplay.difficulty",
                "difficulty",
                profile.gameplay.difficulty.as_deref(),
            ),
            (
                "gameplay.default-game-mode",
                "defaultgamemode",
                profile.gameplay.default_game_mode.as_deref(),
            ),
        ] {
            if !changes.iter().any(|(changed, _)| changed == key) {
                continue;
            }
            let Some(value) = value else { continue };
            let applied = match state
                .lifecycle
                .send_bedrock_command(&format!("{command} {value}"))
            {
                Ok(_) if key == "gameplay.default-game-mode" => {
                    // BDS defaultgamemode only affects new players unless
                    // force-gamemode is enabled. Keep the slot default
                    // independent, then move current players explicitly.
                    state
                        .lifecycle
                        .send_bedrock_command(&format!("gamemode {value} @a"))
                }
                Ok(_) => Ok(None),
                Err(error) => Err(error),
            };
            let status = match applied {
                Ok(_) => (worlds::WorldProfileApplyStatus::Live, None),
                Err(error) => (
                    worlds::WorldProfileApplyStatus::PendingRestart,
                    Some(format!("runtime_command_failed: {error:?}")),
                ),
            };
            if let Some(change) = report.changes.iter_mut().find(|change| change.key == key) {
                change.status = status.0;
                change.reason = status.1;
            }
        }
    }
    if active && running && server.server_type == ServerType::Java {
        match state.lifecycle.apply_active_java_world_gameplay() {
            Ok(()) => {
                for change in &mut report.changes {
                    if matches!(
                        change.key.as_str(),
                        "gameplay.gamerules" | "gameplay.difficulty" | "gameplay.default-game-mode"
                    ) {
                        change.status = worlds::WorldProfileApplyStatus::Live;
                        change.reason = None;
                    }
                }
            }
            Err(error) => {
                for change in &mut report.changes {
                    if matches!(
                        change.key.as_str(),
                        "gameplay.gamerules" | "gameplay.difficulty" | "gameplay.default-game-mode"
                    ) {
                        change.status = worlds::WorldProfileApplyStatus::PendingRestart;
                        change.reason = Some(format!("runtime_command_failed: {error}"));
                    }
                }
            }
        }
    }
    let mut response_changes = Vec::new();
    for (key, _) in changes {
        if !active && key != "packs" {
            response_changes.push(WorldProfileChangeDto {
                key,
                status: "pending_activation".to_string(),
                reason: Some("slot_not_active".to_string()),
            });
            continue;
        }
        if key == "packs" && changes_packs {
            response_changes.push(WorldProfileChangeDto {
                key,
                status: "applied".to_string(),
                reason: None,
            });
        } else if let Some(change) = report.changes.iter_mut().find(|change| change.key == key) {
            response_changes.push(WorldProfileChangeDto {
                key,
                status: change.status.raw_value().to_string(),
                reason: change.reason.clone(),
            });
        } else {
            response_changes.push(WorldProfileChangeDto {
                key,
                status: "live".into(),
                reason: None,
            });
        }
    }
    let status = if response_changes
        .iter()
        .any(|change| change.status == "blocked")
    {
        "blocked"
    } else if response_changes
        .iter()
        .any(|change| change.status == "pending_restart")
    {
        "pending_restart"
    } else if response_changes
        .iter()
        .any(|change| change.status == "pending_activation")
    {
        "pending_activation"
    } else {
        "live"
    };
    if let Some(backup) = pack_backup.take() {
        let _ = std::fs::remove_file(backup);
    }
    let saved_profile = world_store::load_profile(&StdFileSystem, server_dir, &updated_slot);
    Json(WorldProfileUpdateResultDto {
        success: true,
        message: "saved".to_string(),
        status: status.to_string(),
        slot: WorldSlotWithProfileDto {
            slot: to_slot_dto(&updated_slot, active),
            profile: profile_to_dto(
                &saved_profile,
                server.server_type,
                &worlds::world_pack_archive_path(server_dir, &updated_slot.id),
            ),
        },
        changes: response_changes,
    })
    .into_response()
}

fn confirmation_required_response(required: SafetyConfirmation) -> Response {
    (
        StatusCode::CONFLICT,
        Json(ErrorDto {
            code: "confirmation_required".to_string(),
            message: required.message().to_string(),
            help_id: None,
            details: Some(required.details()),
        }),
    )
        .into_response()
}

fn mutation_ok(state: &LifecycleRoutesState, server: &ConfigServer, message: &str) -> Response {
    let running = state.status_snapshot().running;
    Json(WorldMutationResultDto {
        success: true,
        message: message.to_string(),
        updated: Some(slots_response(server, running)),
    })
    .into_response()
}

fn world_error_response(error: WorldError) -> Response {
    match error {
        WorldError::EmptyName => invalid_body("name_required", "name must not be blank."),
        WorldError::ActiveSlotDeleteRefused => error_response(
            StatusCode::CONFLICT,
            "active_slot_refused",
            "The active world slot cannot be deleted.",
        ),
        WorldError::ServerRunning => {
            error_response(StatusCode::CONFLICT, "server_running", "Server is running.")
        }
        WorldError::NoSourceZip | WorldError::NoWorldFolders => error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "No source world data was found.",
        ),
        WorldError::InvalidWorldSource => {
            invalid_body("invalid_body", "Replacement world source is invalid.")
        }
        WorldError::TargetFolderExists(name) => error_response(
            StatusCode::CONFLICT,
            "conflict",
            &format!("A folder named {name} already exists."),
        ),
        WorldError::BackupFailed => error_response(
            StatusCode::CONFLICT,
            "conflict",
            "Pre-operation safety backup failed.",
        ),
        WorldError::NoArchiveOrFreshMetadata => error_response(
            StatusCode::CONFLICT,
            "conflict",
            "Slot has no saved world archive.",
        ),
        // P6.34: `replace_world` (P6.33) is the one caller of these three
        // — reached only from `replace_active` below, and only via its
        // background task's own `Ok(Err(error)) => fail(...)` arm (which
        // uses `error.to_string()` directly, the same convention
        // `activate`/`convert`/`restore` already use for their async
        // failures), never through this synchronous responder. Covered
        // here purely so this match stays exhaustive over `WorldError`.
        WorldError::SafetyBackupFailed(_) => error_response(
            StatusCode::CONFLICT,
            "conflict",
            "Pre-replace safety backup failed.",
        ),
        WorldError::Manifest => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Replace transaction manifest is missing or unreadable.",
        ),
        WorldError::Cancelled => {
            error_response(StatusCode::CONFLICT, "conflict", "Operation was cancelled.")
        }
        WorldError::Io(_) | WorldError::Archive(_) | WorldError::AtomicWrite(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        ),
    }
}

/// One journaled operation per mutation — see the module doc's "operation
/// journaling, per-server exclusivity, and cancellation" section. Returns
/// `Err(response)` (already `409 conflict` via `operation_error_response`)
/// if another non-terminal operation already targets this server.
#[allow(clippy::result_large_err)]
fn begin_operation(
    state: &LifecycleRoutesState,
    server_id: &str,
    operation_type: &str,
    status_line: &str,
) -> Result<msc_domain::operation::OperationId, Response> {
    state
        .operations()
        .begin_lifecycle(operation_type, Some(server_id.to_string()), status_line)
        .map_err(crate::routes::operations::operation_error_response)
}

// =====================================================================
// Synchronous slot CRUD routes
// =====================================================================

pub async fn list(State(state): State<WorldsRoutesState>) -> Response {
    let lifecycle = &state.lifecycle;
    let Some(server) = lifecycle.active_config_server() else {
        return Json(WorldSlotsResponseDto {
            slots: Vec::new(),
            active_slot_id: None,
            server_running: false,
            is_repairing: Some(false),
        })
        .into_response();
    };
    let running = lifecycle.status_snapshot().running;
    Json(slots_response(&server, running)).into_response()
}

/// Temporary Phase 18 proof endpoint. The private path is returned only to
/// a Worlds-authorized operator so the standalone exporter can inspect it.
pub async fn snapshot_map_proof(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    let lifecycle = state.lifecycle.clone();
    if state.map_shutdown.load(Ordering::Acquire) {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_stopping",
            "The agent is stopping.",
        );
    }
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    if let Some(response) = require_runtime(&lifecycle) {
        return response;
    }
    let server = match active_server_or_response(&lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    if !lifecycle.status_snapshot().running {
        return error_response(
            StatusCode::CONFLICT,
            "conflict",
            "An MSC-managed server must be running for this snapshot proof.",
        );
    }
    let server_type = server.server_type;
    let kind = if server_type == ServerType::Bedrock {
        "BDS"
    } else {
        "Java"
    };
    let operation_id = match lifecycle.operations().begin_lifecycle(
        "world-map-proof-snapshot",
        Some(server.id),
        &format!("Capturing a consistent {kind} map snapshot."),
    ) {
        Ok(id) => id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let should_cancel = lifecycle.operations().cancellation_check(&operation_id);
    let task_lifecycle = lifecycle.clone();
    let task_operation_id = operation_id.clone();
    tokio::spawn(async move {
        let snapshot_lifecycle = task_lifecycle.clone();
        let server_dir = PathBuf::from(server.server_dir);
        let result = tokio::task::spawn_blocking(move || {
            if server_type == ServerType::Bedrock {
                crate::backup_operations::snapshot_bedrock_world(
                    snapshot_lifecycle,
                    &server_dir,
                    should_cancel,
                )
            } else {
                crate::backup_operations::snapshot_java_world(
                    snapshot_lifecycle,
                    &server_dir,
                    should_cancel,
                )
            }
        })
        .await;
        match result {
            Ok(Ok(snapshot)) => {
                let mut details = BTreeMap::new();
                details.insert("worldPath".to_string(), snapshot.path.display().to_string());
                details.insert("bytesCopied".to_string(), snapshot.bytes.to_string());
                details.insert("holdMillis".to_string(), snapshot.hold_millis.to_string());
                let _ = task_lifecycle.operations().succeed(
                    &task_operation_id,
                    &format!("{kind} map snapshot ready for offline export."),
                    details,
                );
            }
            Ok(Err(error)) => {
                let _ =
                    task_lifecycle
                        .operations()
                        .fail(&task_operation_id, "snapshot_failed", error);
            }
            Err(_) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "snapshot_failed",
                    format!("{kind} map snapshot task panicked; save restoration was attempted."),
                );
            }
        }
    });
    let response = Json(serde_json::json!({
        "result": "snapshot_started",
        "operationId": operation_id.as_str(),
    }))
    .into_response();
    audit(
        &lifecycle,
        &credential,
        "POST",
        "/v1/worlds/map-proof/snapshot",
        response.status(),
    );
    response
}

/// Capture a consistent save and make it the source for subsequent map artifacts.
pub async fn refresh_map(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    let lifecycle = state.lifecycle.clone();
    if state.map_shutdown.load(Ordering::Acquire) {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_stopping",
            "The agent is stopping.",
        );
    }
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    if let Some(response) = require_runtime(&lifecycle) {
        return response;
    }
    let server = match active_server_or_response(&lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    if !lifecycle.status_snapshot().running {
        return error_response(
            StatusCode::CONFLICT,
            "server_stopped",
            "Start the active server before refreshing its terrain.",
        );
    }
    if server.server_type == ServerType::Bedrock {
        let server_dir = PathBuf::from(&server.server_dir);
        let Some(level_name) =
            msc_application::worlds::read_configured_level_name(&StdFileSystem, &server_dir)
                .filter(|name| is_safe_level_folder(name))
        else {
            return error_response(
                StatusCode::CONFLICT,
                "map_unavailable",
                "The active server has no safe configured Bedrock world folder.",
            );
        };
        let worlds = server_dir.join("worlds");
        let source_world = worlds.join(level_name);
        if !map_terrain::is_direct_child(&server_dir, &worlds)
            || !map_terrain::is_direct_child(&worlds, &source_world)
            || !source_world.join("level.dat").is_file()
            || !source_world.join("db").is_dir()
        {
            return error_response(
                StatusCode::CONFLICT,
                "map_unavailable",
                "The active Bedrock world has no safely accessible saved terrain.",
            );
        }
        let operation_id = match lifecycle.operations().begin_lifecycle(
            "world-map-refresh",
            Some(server.id.clone()),
            "Capturing current Bedrock terrain.",
        ) {
            Ok(id) => id,
            Err(error) => return crate::routes::operations::operation_error_response(error),
        };
        let should_cancel = lifecycle.operations().cancellation_check(&operation_id);
        let task_lifecycle = lifecycle.clone();
        let task_operation_id = operation_id.clone();
        let store = state.bedrock_map.clone();
        tokio::spawn(async move {
            let snapshot_lifecycle = task_lifecycle.clone();
            let result = tokio::task::spawn_blocking(move || {
                crate::backup_operations::snapshot_bedrock_world(
                    snapshot_lifecycle,
                    &server_dir,
                    should_cancel,
                )
            })
            .await;
            match result {
                Ok(Ok(snapshot)) => {
                    let mut details = BTreeMap::new();
                    details.insert("bytesCopied".to_string(), snapshot.bytes.to_string());
                    details.insert("holdMillis".to_string(), snapshot.hold_millis.to_string());
                    match tokio::task::spawn_blocking(move || {
                        store.use_snapshot(server.id, source_world, snapshot)
                    })
                    .await
                    {
                        Ok(Ok(stats)) => {
                            details
                                .insert("tilesReused".to_string(), stats.reused_tiles.to_string());
                            details.insert(
                                "tilesChanged".to_string(),
                                stats.changed_tiles.to_string(),
                            );
                            details.insert(
                                "tilesRemoved".to_string(),
                                stats.removed_tiles.to_string(),
                            );
                            let _ = task_lifecycle.operations().succeed(
                                &task_operation_id,
                                "Current Bedrock terrain is ready.",
                                details,
                            );
                        }
                        Ok(Err(error)) => {
                            let _ = task_lifecycle.operations().fail(
                                &task_operation_id,
                                "renderer_unavailable",
                                error,
                            );
                        }
                        Err(_) => {
                            let _ = task_lifecycle.operations().fail(
                                &task_operation_id,
                                "renderer_unavailable",
                                "The Bedrock tile comparison stopped unexpectedly.".to_string(),
                            );
                        }
                    }
                }
                Ok(Err(error)) => {
                    let _ = task_lifecycle.operations().fail(
                        &task_operation_id,
                        "snapshot_failed",
                        error,
                    );
                }
                Err(_) => {
                    let _ = task_lifecycle.operations().fail(
                        &task_operation_id,
                        "snapshot_failed",
                        "Bedrock map snapshot task panicked; save restoration was attempted."
                            .to_string(),
                    );
                }
            }
        });
        let response = Json(
            serde_json::json!({"result": "refresh_started", "operationId": operation_id.as_str()}),
        )
        .into_response();
        audit(
            &lifecycle,
            &credential,
            "POST",
            "/v1/worlds/map/refresh",
            response.status(),
        );
        return response;
    }
    let server_dir = PathBuf::from(&server.server_dir);
    let Some(level_name) =
        msc_application::worlds::read_java_level_name(&StdFileSystem, &server_dir)
            .filter(|name| is_safe_level_folder(name))
    else {
        return error_response(
            StatusCode::CONFLICT,
            "map_unavailable",
            "The active server has no safe configured world folder.",
        );
    };
    let source_world = server_dir.join(level_name);
    let operation_id = match lifecycle.operations().begin_lifecycle(
        "world-map-refresh",
        Some(server.id.clone()),
        "Capturing current Java terrain.",
    ) {
        Ok(id) => id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let should_cancel = lifecycle.operations().cancellation_check(&operation_id);
    let task_lifecycle = lifecycle.clone();
    let task_operation_id = operation_id.clone();
    let renderer = state.map_renderer.clone();
    tokio::spawn(async move {
        let snapshot_lifecycle = task_lifecycle.clone();
        let result = tokio::task::spawn_blocking(move || {
            crate::backup_operations::snapshot_java_world(
                snapshot_lifecycle,
                &server_dir,
                should_cancel,
            )
        })
        .await;
        match result {
            Ok(Ok(snapshot)) => {
                let mut details = BTreeMap::new();
                details.insert("bytesCopied".to_string(), snapshot.bytes.to_string());
                details.insert("holdMillis".to_string(), snapshot.hold_millis.to_string());
                if renderer
                    .use_snapshot(server.id, source_world, snapshot)
                    .is_ok()
                {
                    let _ = task_lifecycle.operations().succeed(
                        &task_operation_id,
                        "Current Java terrain is ready.",
                        details,
                    );
                } else {
                    let _ = task_lifecycle.operations().fail(
                        &task_operation_id,
                        "renderer_unavailable",
                        "The saved terrain snapshot could not be selected.".to_string(),
                    );
                }
            }
            Ok(Err(error)) => {
                let _ =
                    task_lifecycle
                        .operations()
                        .fail(&task_operation_id, "snapshot_failed", error);
            }
            Err(_) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "snapshot_failed",
                    "Java map snapshot task panicked; save restoration was attempted.".to_string(),
                );
            }
        }
    });
    let response = Json(serde_json::json!({
        "result": "refresh_started",
        "operationId": operation_id.as_str(),
    }))
    .into_response();
    audit(
        &lifecycle,
        &credential,
        "POST",
        "/v1/worlds/map/refresh",
        response.status(),
    );
    response
}

pub async fn map_proof_players(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    Json(state.lifecycle.map_players()).into_response()
}

pub async fn map_players(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    Json(state.lifecycle.map_players()).into_response()
}

/// Returns dimension names and save-directory availability for the selected
/// world. The response deliberately contains no host paths and does not read
/// chunk data; terrain snapshots remain a separate, consistency-gated flow.
pub async fn map_dimensions(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let lifecycle = &state.lifecycle;
    let server = match active_server_or_response(lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let server_dir = PathBuf::from(&server.server_dir);
    let level_name = if server.server_type == ServerType::Bedrock {
        msc_application::worlds::read_configured_level_name(&StdFileSystem, &server_dir)
    } else {
        msc_application::worlds::read_java_level_name(&StdFileSystem, &server_dir)
    };
    let Some(level_name) = level_name.filter(|name| is_safe_level_folder(name)) else {
        return error_response(
            StatusCode::CONFLICT,
            "map_unavailable",
            "The active server has no safe configured world folder.",
        );
    };
    let world_dir = if server.server_type == ServerType::Bedrock {
        server_dir.join("worlds").join(level_name)
    } else {
        server_dir.join(level_name)
    };
    let world_metadata = match std::fs::symlink_metadata(&world_dir) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Some(metadata),
        Ok(_) => {
            return error_response(
                StatusCode::CONFLICT,
                "map_unavailable",
                "The active world folder is not a regular directory.",
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "map_unavailable",
                "The active world folder could not be inspected.",
            );
        }
    };
    let dimensions = if server.server_type == ServerType::Bedrock {
        bedrock_map_dimensions(&world_dir)
    } else {
        match java_map_dimensions(world_dir.as_path(), world_metadata.is_some()) {
            Ok(dimensions) => dimensions,
            Err(_) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "map_unavailable",
                    "The active world's dimension folders could not be inspected safely.",
                );
            }
        }
    };
    let response = Json(WorldMapDimensionsResponseDto {
        server_id: server.id,
        server_type: if server.server_type == ServerType::Bedrock {
            "bedrock".to_string()
        } else {
            "java".to_string()
        },
        server_running: lifecycle.status_snapshot().running,
        dimensions,
    })
    .into_response();
    audit(
        lifecycle,
        &credential,
        "GET",
        "/v1/worlds/map/dimensions",
        response.status(),
    );
    response
}

fn is_safe_level_folder(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains(':')
        && !name.chars().any(char::is_control)
}

fn dimension_state(region_file_count: u32) -> String {
    if region_file_count == 0 {
        "no_saved_terrain".to_string()
    } else {
        "ready".to_string()
    }
}

fn count_region_files(region_dir: &Path) -> Result<u32, std::io::Error> {
    match std::fs::symlink_metadata(region_dir) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return Ok(0),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    let mut count = 0u32;
    let mut entries_seen = 0usize;
    for entry in std::fs::read_dir(region_dir)? {
        entries_seen += 1;
        if entries_seen > 131_072 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "region directory scan exceeded its entry limit",
            ));
        }
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "mca")
        {
            count = count.saturating_add(1);
            if count > 65_536 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "region file count exceeded its limit",
                ));
            }
        }
    }
    Ok(count)
}

fn count_region_files_beneath(
    world_dir: &Path,
    relative_region_dir: &Path,
) -> Result<u32, std::io::Error> {
    let mut current = world_dir.to_path_buf();
    for segment in relative_region_dir.components() {
        let std::path::Component::Normal(segment) = segment else {
            return Ok(0);
        };
        current.push(segment);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => return Ok(0),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(error),
        }
    }
    count_region_files(&current)
}

fn java_map_dimensions(
    world_dir: &Path,
    world_exists: bool,
) -> Result<Vec<WorldMapDimensionDto>, std::io::Error> {
    let candidates = [
        (
            "minecraft:overworld",
            "Overworld",
            ["region", "dimensions/minecraft/overworld/region"],
        ),
        (
            "minecraft:the_nether",
            "The Nether",
            ["DIM-1/region", "dimensions/minecraft/the_nether/region"],
        ),
        (
            "minecraft:the_end",
            "The End",
            ["DIM1/region", "dimensions/minecraft/the_end/region"],
        ),
    ];
    let mut dimensions = BTreeMap::new();
    for (id, display_name, paths) in candidates {
        let count = paths
            .iter()
            .map(|path| count_region_files_beneath(world_dir, Path::new(path)))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .max()
            .unwrap_or_default();
        dimensions.insert(
            id.to_string(),
            WorldMapDimensionDto {
                id: id.to_string(),
                display_name: display_name.to_string(),
                state: dimension_state(count),
                region_file_count: count,
                reason: (!world_exists)
                    .then(|| "World terrain has not been created yet.".to_string()),
            },
        );
    }

    if world_exists {
        let dimensions_root = world_dir.join("dimensions");
        match std::fs::symlink_metadata(&dimensions_root) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                let mut budget = 0usize;
                for namespace_entry in std::fs::read_dir(&dimensions_root)? {
                    budget += 1;
                    if budget > 4096 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "dimension folder scan exceeded its entry limit",
                        ));
                    }
                    let namespace_entry = namespace_entry?;
                    let namespace_type = namespace_entry.file_type()?;
                    let namespace = namespace_entry.file_name().to_string_lossy().into_owned();
                    if !namespace_type.is_dir() || !valid_dimension_segment(&namespace) {
                        continue;
                    }
                    collect_java_dimensions(
                        &namespace,
                        Path::new(""),
                        &namespace_entry.path(),
                        0,
                        &mut budget,
                        &mut dimensions,
                    )?;
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(dimensions.into_values().collect())
}

fn valid_dimension_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && segment
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn collect_java_dimensions(
    namespace: &str,
    relative: &Path,
    current: &Path,
    depth: usize,
    budget: &mut usize,
    dimensions: &mut BTreeMap<String, WorldMapDimensionDto>,
) -> Result<(), std::io::Error> {
    if depth > 8 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "dimension folder nesting exceeded its limit",
        ));
    }
    let mut child_dimensions = Vec::new();
    let mut has_payload = false;
    for entry in std::fs::read_dir(current)? {
        *budget += 1;
        if *budget > 4096 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "dimension folder scan exceeded its entry limit",
            ));
        }
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        let segment = entry.file_name().to_string_lossy().into_owned();
        if !valid_dimension_segment(&segment) {
            continue;
        }
        if matches!(segment.as_str(), "region" | "entities" | "poi" | "data") {
            has_payload = true;
        } else {
            child_dimensions.push((segment, entry.path()));
        }
    }

    if !relative.as_os_str().is_empty() && (has_payload || child_dimensions.is_empty()) {
        let id_path = relative.to_string_lossy().replace('\\', "/");
        let id = format!("{namespace}:{id_path}");
        let count = count_region_files(&current.join("region"))?;
        dimensions
            .entry(id.clone())
            .or_insert_with(|| WorldMapDimensionDto {
                id: id.clone(),
                display_name: id.clone(),
                state: dimension_state(count),
                region_file_count: count,
                reason: None,
            });
    }

    for (segment, path) in child_dimensions {
        let mut next = relative.to_path_buf();
        next.push(segment);
        collect_java_dimensions(namespace, &next, &path, depth + 1, budget, dimensions)?;
    }
    Ok(())
}

fn bedrock_map_dimensions(world: &Path) -> Vec<WorldMapDimensionDto> {
    let ready = world.join("level.dat").is_file() && world.join("db").is_dir();
    [
        ("minecraft:overworld", "Overworld"),
        ("minecraft:the_nether", "The Nether"),
        ("minecraft:the_end", "The End"),
    ]
    .into_iter()
    .map(|(id, display_name)| WorldMapDimensionDto {
        id: id.to_string(),
        display_name: display_name.to_string(),
        state: if ready { "ready" } else { "no_saved_terrain" }.to_string(),
        region_file_count: 0,
        reason: Some(if ready {
            "Saved tiles load on demand when this dimension contains generated chunks.".to_string()
        } else {
            "This Bedrock world has no saved level.dat and LevelDB terrain yet.".to_string()
        }),
    })
    .collect()
}

pub async fn create(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldCreateRequestDto>>,
) -> Response {
    run_mutation(
        &state,
        &credential,
        "POST",
        "/v1/worlds/create",
        "world-create",
        body,
        |lifecycle, server, body| {
            let name = body.name.trim();
            if name.is_empty() {
                return invalid_body("name_required", "name must not be blank.");
            }
            if lifecycle.status_snapshot().running {
                return error_response(
                    StatusCode::CONFLICT,
                    "server_running",
                    "Stop the server before creating and activating a new world.",
                );
            }
            let server_dir = Path::new(&server.server_dir);
            let now = iso8601_now();
            let mut slot = msc_domain::world::build_fresh_slot(
                Uuid::new_v4().to_string().to_uppercase(),
                name,
                body.seed.as_deref(),
                server.server_type,
                now.clone(),
            );
            let mut profile = worlds::fresh_world_profile(&slot, Some("normal"), Some("survival"));
            for (key, value) in &body.changes {
                if key == "packs" {
                    return invalid_body("invalid_body", "Install packs after creating the world.");
                }
                if let Err(message) =
                    apply_profile_change(&mut profile, server.server_type, key, value)
                {
                    return invalid_body("invalid_body", &message);
                }
            }
            if server.server_type == ServerType::Java
                && let Err(error) = msc_application::java_world_settings::validate(&profile)
            {
                return invalid_body("invalid_body", &error.to_string());
            }
            if let Err(message) = msc_infrastructure::gamerule_catalog::validate_values(
                server.server_type.raw_value(),
                gamerule_server_version(server, &state.lifecycle).as_deref(),
                &profile.gameplay.gamerules,
            ) {
                return invalid_body("invalid_gamerule", &message);
            }
            if let Some(required) =
                world_safety::confirmation_for_world_profile(server.server_type, &profile)
                && !world_safety::is_confirmed(required, body.confirmation.as_deref())
            {
                return confirmation_required_response(required);
            }
            profile.identity.name = Some(name.to_string());
            let level_name = profile.identity.level_name.clone().unwrap_or_else(|| {
                slot.world_level_name
                    .clone()
                    .expect("fresh slots have a folder name")
            });
            profile.identity.level_name = Some(level_name.clone());
            slot.world_level_name = Some(level_name);
            if server.server_type == ServerType::Bedrock && profile.identity.seed.is_none() {
                profile.identity.seed = Some((Uuid::new_v4().as_u128() as i64).to_string());
            }
            slot.world_seed = profile.identity.seed.clone();
            if let Err(error) =
                world_store::save_profile(&StdFileSystem, server_dir, &slot, &profile)
            {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "world_create_failed",
                    &error.to_string(),
                );
            }
            let configured_level_name =
                worlds::read_configured_level_name(&StdFileSystem, server_dir);
            match worlds::activate_slot(
                &StdFileSystem,
                server_dir,
                server.server_type,
                &slot,
                false,
                &now,
                || {
                    run_pre_mutation_safety_backup(
                        lifecycle,
                        server_dir,
                        server.server_type,
                        configured_level_name.as_deref(),
                        || false,
                    )
                },
                || false,
            ) {
                Ok(_) => mutation_ok(lifecycle, server, "created_and_activated"),
                Err(error) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "world_activation_failed",
                    &format!("The new world could not be activated: {error}"),
                ),
            }
        },
    )
    .await
}

pub async fn rename(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldRenameRequestDto>>,
) -> Response {
    run_mutation(
        &state,
        &credential,
        "POST",
        "/v1/worlds/rename",
        "world-rename",
        body,
        |lifecycle, server, body| {
            let Some(slot) = find_slot(Path::new(&server.server_dir), &body.slot_id) else {
                return slot_not_found(&body.slot_id);
            };
            match worlds::rename_slot(
                &StdFileSystem,
                Path::new(&server.server_dir),
                &slot,
                &body.name,
            ) {
                Ok(_) => mutation_ok(lifecycle, server, "renamed"),
                Err(error) => world_error_response(error),
            }
        },
    )
    .await
}

/// `POST /v1/worlds/replace` — **corrected post-review (the owner)**: this
/// is `WorldSlotManager.copySlotIntoExisting(source, into: dest, ...)`,
/// a saved-slot-to-saved-slot copy, not `AppViewModel+WorldManagement
/// .swift::replaceWorld`'s live-world operation the original P6.21 pass
/// guessed at (that guess is what the "flagged as a genuinely open
/// question" comment previously here recorded — the owner's answer:
/// "slotId is the existing destination slot, and sourceSlotId is the
/// slot whose saved contents replace it. This is not a concurrency
/// check and does not operate on the live world."). No new level name
/// is needed, and `/v1/worlds/copy` — a newly-proposed route with no
/// MSC 1 counterpart — duplicated this exact behavior, so it has been
/// removed from the contract rather than kept alongside it.
pub async fn replace(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldReplaceRequestDto>>,
) -> Response {
    run_mutation(
        &state,
        &credential,
        "POST",
        "/v1/worlds/replace",
        "world-replace",
        body,
        |lifecycle, server, body| {
            let server_dir = Path::new(&server.server_dir);
            let Some(source) = find_slot(server_dir, &body.source_slot_id) else {
                return error_response(StatusCode::NOT_FOUND, "not_found", "source_not_found");
            };
            let Some(destination) = find_slot(server_dir, &body.slot_id) else {
                return slot_not_found(&body.slot_id);
            };
            let now = iso8601_now();
            match worlds::copy_slot_into_existing(
                &StdFileSystem,
                server_dir,
                &source,
                &destination,
                &now,
            ) {
                Ok(_) => mutation_ok(lifecycle, server, "replaced"),
                Err(error) => world_error_response(error),
            }
        },
    )
    .await
}

/// Bedrock-only capability.  The repair itself starts BDS briefly to
/// regenerate `level.dat`, so an imported server remains inspectable while
/// an unavailable runtime is reported through the contract-wide capability
/// error rather than the old Phase-6 placeholder.
pub async fn repair(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldRepairRequestDto>>,
) -> Response {
    let lifecycle = state.lifecycle.clone();
    if state.map_shutdown.load(Ordering::Acquire) {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_stopping",
            "The agent is stopping.",
        );
    }
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Some(Json(body)) = body else {
        return invalid_body("missing_body", "Request body is required.");
    };
    let Some(server) = state.lifecycle.active_config_server() else {
        return no_active_server();
    };
    if server.server_type != ServerType::Bedrock {
        return error_response(
            StatusCode::CONFLICT,
            "bedrock_only",
            "Repair is only supported for Bedrock servers.",
        );
    }
    if let Some(response) = require_runtime(&state.lifecycle) {
        return response;
    }
    if state.lifecycle.status_snapshot().running {
        return error_response(
            StatusCode::CONFLICT,
            "server_running",
            "Stop the server before repairing a Bedrock world.",
        );
    }
    if resolved_active_slot_id(Path::new(&server.server_dir)).as_deref()
        != Some(body.slot_id.as_str())
    {
        return error_response(
            StatusCode::CONFLICT,
            "not_active_slot",
            "Only the active Bedrock world can be repaired.",
        );
    }
    let operation_id = match lifecycle.operations().begin_lifecycle(
        "world-repair",
        None,
        "Repairing Bedrock world.",
    ) {
        Ok(id) => id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let server_dir = Path::new(&server.server_dir).to_path_buf();
    let server_type = server.server_type;
    let raw_level_name =
        crate::backup_operations::configured_java_level_name(server_type, &server_dir);
    let task_lifecycle = lifecycle.clone();
    let task_operation_id = operation_id.clone();
    tokio::spawn(async move {
        let control = AgentRepairServerControl::new(task_lifecycle.clone());
        let backup_lifecycle = task_lifecycle.clone();
        let backup_dir = server_dir.clone();
        let progress_lifecycle = task_lifecycle.clone();
        let progress_operation_id = task_operation_id.clone();
        let result = tokio::task::spawn_blocking(move || {
            repair_world(
                &StdFileSystem,
                &control,
                &server_dir,
                || {
                    run_pre_mutation_safety_backup(
                        &backup_lifecycle,
                        &backup_dir,
                        server_type,
                        raw_level_name.as_deref(),
                        || false,
                    )
                },
                |status_line| {
                    let _ = progress_lifecycle.operations().progress(
                        &progress_operation_id,
                        0,
                        1,
                        status_line,
                    );
                },
            )
        })
        .await;
        match result {
            Ok(Ok(())) => {
                let mut result = BTreeMap::new();
                result.insert("result".to_string(), "repaired".to_string());
                let _ = task_lifecycle.operations().succeed(
                    &task_operation_id,
                    "Bedrock world repair complete.",
                    result,
                );
            }
            Ok(Err(error)) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    world_repair_error_code(&error),
                    error.to_string(),
                );
            }
            Err(_) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "internal_error",
                    "World repair task panicked.".to_string(),
                );
            }
        }
    });

    let response = Json(WorldRepairResultDto {
        result: "repair_started".to_string(),
        operation_id: Some(operation_id.as_str().to_string()),
    })
    .into_response();
    audit(
        &lifecycle,
        &credential,
        "POST",
        "/v1/worlds/repair",
        response.status(),
    );
    response
}

struct AgentRepairServerControl {
    lifecycle: LifecycleRoutesState,
    start_operation_id: Mutex<Option<String>>,
    start_failed: Mutex<bool>,
}

impl AgentRepairServerControl {
    fn new(lifecycle: LifecycleRoutesState) -> Self {
        Self {
            lifecycle,
            start_operation_id: Mutex::new(None),
            start_failed: Mutex::new(false),
        }
    }
}

impl RepairServerControl for AgentRepairServerControl {
    fn start(&self) {
        match self.lifecycle.start_active_server() {
            Ok(result) => {
                *self.start_operation_id.lock().unwrap() = result.operation_id;
            }
            Err(_) => {
                *self.start_failed.lock().unwrap() = true;
            }
        }
    }

    fn is_ready(&self) -> bool {
        if *self.start_failed.lock().unwrap() {
            return false;
        }
        let _ = self.lifecycle.status_snapshot();
        let Some(operation_id) = self.start_operation_id.lock().unwrap().clone() else {
            return false;
        };
        self.lifecycle
            .operations()
            .snapshot(&operation_id)
            .is_some_and(|operation| operation.state == msc_api::dto::OperationStateDto::Succeeded)
    }

    fn stop(&self) {
        let _ = self.lifecycle.stop_active_bedrock_server();
    }

    fn is_running(&self) -> bool {
        self.lifecycle.status_snapshot().running
    }
}

fn world_repair_error_code(error: &WorldRepairError) -> &'static str {
    match error {
        WorldRepairError::BackupFailed => "backup_failed",
        WorldRepairError::NoLevelName
        | WorldRepairError::StartTimedOut
        | WorldRepairError::Io(_)
        | WorldRepairError::RestoreFailed(_) => "world_repair_failed",
    }
}

pub async fn update(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let lifecycle = &state.lifecycle;
    let server = match active_server_or_response(lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let server_id = server.id.clone();
    let operation_id = match begin_operation(
        lifecycle,
        &server_id,
        "world-update",
        "Saving active world.",
    ) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let server_dir = Path::new(&server.server_dir);
    let active_id = resolved_active_slot_id(server_dir);
    let Some(slot) = active_id.and_then(|id| find_slot(server_dir, &id)) else {
        let _ = lifecycle.operations().fail(
            &operation_id,
            "not_found",
            "No active world slot.".to_string(),
        );
        return error_response(StatusCode::NOT_FOUND, "not_found", "No active world slot.");
    };

    let response = match worlds::update_active_slot_from_current_world(
        &StdFileSystem,
        server_dir,
        server.server_type,
        None,
        &slot,
    ) {
        Ok(_) => {
            let _ = lifecycle.operations().succeed(
                &operation_id,
                "Active world saved.",
                BTreeMap::new(),
            );
            mutation_ok(lifecycle, &server, "updated")
        }
        Err(error) => {
            let _ = lifecycle
                .operations()
                .fail(&operation_id, "world_error", error.to_string());
            world_error_response(error)
        }
    };
    audit(
        lifecycle,
        &credential,
        "POST",
        "/v1/worlds/update",
        StatusCode::from_u16(response.status().as_u16()).unwrap(),
    );
    response
}

pub async fn delete(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldDeleteRequestDto>>,
) -> Response {
    run_mutation(
        &state,
        &credential,
        "POST",
        "/v1/worlds/delete",
        "world-delete",
        body,
        |lifecycle, server, body| {
            let server_dir = Path::new(&server.server_dir);
            let Some(slot) = find_slot(server_dir, &body.slot_id) else {
                return slot_not_found(&body.slot_id);
            };
            let active_id = resolved_active_slot_id(server_dir);
            match worlds::delete_slot(&StdFileSystem, server_dir, &slot, active_id.as_deref()) {
                Ok(()) => mutation_ok(lifecycle, server, "deleted"),
                Err(error) => world_error_response(error),
            }
        },
    )
    .await
}

pub async fn duplicate(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldDuplicateRequestDto>>,
) -> Response {
    run_mutation(
        &state,
        &credential,
        "POST",
        "/v1/worlds/duplicate",
        "world-duplicate",
        body,
        |lifecycle, server, body| {
            let server_dir = Path::new(&server.server_dir);
            let Some(source) = find_slot(server_dir, &body.slot_id) else {
                return slot_not_found(&body.slot_id);
            };
            let now = iso8601_now();
            let new_name = format!("{} copy", source.name);
            match worlds::duplicate_slot(&StdFileSystem, server_dir, &source, &new_name, &now) {
                Ok(_) => mutation_ok(lifecycle, server, "duplicated"),
                Err(error) => world_error_response(error),
            }
        },
    )
    .await
}

pub async fn rename_active_world(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldRenameActiveWorldRequestDto>>,
) -> Response {
    run_mutation(
        &state,
        &credential,
        "POST",
        "/v1/worlds/rename-active-world",
        "world-rename-active",
        body,
        |lifecycle, server, body| {
            let server_dir = Path::new(&server.server_dir);
            let running = lifecycle.status_snapshot().running;
            match worlds::rename_world(
                &StdFileSystem,
                server_dir,
                server.server_type,
                None,
                &body.name,
                running,
                false,
                || false,
            ) {
                Ok(()) => mutation_ok(lifecycle, server, "renamed"),
                Err(error) => world_error_response(error),
            }
        },
    )
    .await
}

/// Shared shape for every synchronous CRUD mutation: permission check,
/// body parse, active-server resolution, one journaled operation
/// (exclusivity for free), the caller's own logic, then
/// succeed/fail the operation and audit-log the outcome.
async fn run_mutation<B, F>(
    state: &WorldsRoutesState,
    credential: &AuthenticatedCredential,
    method: &str,
    path: &str,
    operation_type: &str,
    body: Option<Json<B>>,
    logic: F,
) -> Response
where
    F: FnOnce(&LifecycleRoutesState, &ConfigServer, &B) -> Response,
{
    let lifecycle = &state.lifecycle;
    if let Some(response) = require_permission(credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Some(Json(body)) = body else {
        return invalid_body("invalid_json", "Request body must be valid JSON.");
    };
    let server = match active_server_or_response(lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let operation_id = match begin_operation(lifecycle, &server.id, operation_type, "Working.") {
        Ok(id) => id,
        Err(response) => return response,
    };

    let response = logic(lifecycle, &server, &body);
    if response.status().is_success() {
        let _ = lifecycle
            .operations()
            .succeed(&operation_id, "Done.", BTreeMap::new());
    } else {
        let _ = lifecycle.operations().fail(
            &operation_id,
            "mutation_failed",
            format!("HTTP {}", response.status()),
        );
    }
    audit(lifecycle, credential, method, path, response.status());
    response
}

// =====================================================================
// Thumbnail
// =====================================================================

pub async fn thumbnail(
    State(state): State<WorldsRoutesState>,
    AxumPath(slot_id): AxumPath<String>,
) -> Response {
    let lifecycle = &state.lifecycle;
    let Some(server) = lifecycle.active_config_server() else {
        return slot_not_found(&slot_id);
    };
    let server_dir = Path::new(&server.server_dir);
    let Some(slot) = find_slot(server_dir, &slot_id) else {
        return slot_not_found(&slot_id);
    };
    let Some(file_name) = &slot.thumbnail_file_name else {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "This slot has no thumbnail.",
        );
    };
    let path = world_store::thumbnail_path(server_dir, &slot.id, file_name);
    match StdFileSystem.read(&path) {
        Ok(bytes) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "image/jpeg".parse().unwrap());
            (StatusCode::OK, headers, bytes).into_response()
        }
        Err(_) => error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "This slot has no thumbnail.",
        ),
    }
}

pub async fn set_thumbnail(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(slot_id): AxumPath<String>,
    body: Option<Json<WorldThumbnailUploadRequestDto>>,
) -> Response {
    let lifecycle = &state.lifecycle;
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Some(Json(body)) = body else {
        return invalid_body("invalid_json", "Request body must be valid JSON.");
    };
    let server = match active_server_or_response(lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let server_dir = Path::new(&server.server_dir);
    let Some(slot) = find_slot(server_dir, &slot_id) else {
        return slot_not_found(&slot_id);
    };
    let entry = {
        state
            .staging
            .uploads
            .lock()
            .unwrap()
            .remove(&body.staged_upload_id)
    };
    let Some(entry) = entry else {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged upload.",
        );
    };
    if now_unix() > entry.expires_at_unix
        || !entry.complete
        || !matches!(entry.purpose, StagedUploadPurposeDto::WorldThumbnail)
    {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged upload.",
        );
    }

    let operation_id = match begin_operation(
        lifecycle,
        &server.id,
        "world-thumbnail",
        "Setting world thumbnail.",
    ) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let response = match std::fs::read(&entry.path)
        .ok()
        .and_then(|bytes| encode_thumbnail(&bytes).ok())
    {
        Some(encoded) => {
            match worlds::set_slot_thumbnail(&StdFileSystem, server_dir, &slot, &encoded) {
                Ok(_) => {
                    let _ = lifecycle.operations().succeed(
                        &operation_id,
                        "Thumbnail set.",
                        BTreeMap::new(),
                    );
                    mutation_ok(lifecycle, &server, "thumbnail_set")
                }
                Err(error) => {
                    let _ = lifecycle.operations().fail(
                        &operation_id,
                        "world_error",
                        error.to_string(),
                    );
                    world_error_response(error)
                }
            }
        }
        None => {
            let message = "Thumbnail image could not be decoded.";
            let _ = lifecycle
                .operations()
                .fail(&operation_id, "invalid_body", message.to_string());
            invalid_body("invalid_body", message)
        }
    };
    let _ = std::fs::remove_file(&entry.path);
    audit(
        lifecycle,
        &credential,
        "POST",
        "/v1/worlds/:slot_id/thumbnail",
        response.status(),
    );
    response
}

fn encode_thumbnail(bytes: &[u8]) -> Result<Vec<u8>, image::ImageError> {
    use image::ImageReader;
    use image::codecs::jpeg::JpegEncoder;
    use std::io::Cursor;

    let image = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .decode()?;
    let thumbnail = image.thumbnail(800, 450);
    let mut encoded = Cursor::new(Vec::new());
    let encoder = JpegEncoder::new_with_quality(&mut encoded, 82);
    thumbnail.write_with_encoder(encoder)?;
    Ok(encoded.into_inner())
}

// =====================================================================
// Staged upload / import
// =====================================================================

#[cfg(test)]
pub async fn begin_staged_upload(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<StagedUploadBeginRequestDto>>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Some(Json(body)) = body else {
        return invalid_body("invalid_json", "Request body must be valid JSON.");
    };
    // Both closed-enum purposes begin the same way — bytes land in the
    // same uploads store, tagged with whichever `purpose` the caller
    // named. Redemption (`import`/`replace_active`) is what actually
    // enforces "a staging slot can only be redeemed by the route it was
    // created for" (`phase6-api.md` §4), by checking the stored purpose
    // matches.
    match body.purpose {
        StagedUploadPurposeDto::WorldImport
        | StagedUploadPurposeDto::ActiveWorldReplace
        | StagedUploadPurposeDto::WorldThumbnail => {}
        StagedUploadPurposeDto::ModpackArchive
        | StagedUploadPurposeDto::AddonLocalFile
        | StagedUploadPurposeDto::CurseforgeManualFile
        | StagedUploadPurposeDto::ModpackUnresolvedFile => {
            return invalid_body(
                "invalid_purpose",
                "This staged upload route only accepts world import purposes.",
            );
        }
    }

    let id = Uuid::new_v4().to_string();
    let servers_root = state.lifecycle.servers_root();
    let uploads_dir = staging_root(&servers_root).join("uploads");
    if std::fs::create_dir_all(&uploads_dir).is_err() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Could not prepare staging directory.",
        );
    }
    let path = uploads_dir.join(format!("{id}.bin"));
    let expires_at_unix = now_unix() + STAGING_TTL_SECONDS;
    state.staging.uploads.lock().unwrap().insert(
        id.clone(),
        StagedUpload {
            purpose: body.purpose,
            file_name: body.file_name.clone(),
            operation_id: body.operation_id.clone(),
            file_id: body.file_id.clone(),
            expires_at_unix,
            max_bytes: MAX_STAGED_UPLOAD_BYTES,
            expected_bytes: body
                .expected_bytes
                .filter(|size| *size > 0)
                .map(|size| size as u64),
            received_bytes: 0,
            complete: false,
            path,
        },
    );

    let response = Json(StagedUploadBeginResultDto {
        staged_upload_id: id.clone(),
        upload_path: format!("/v1/staged-uploads/{id}"),
        expires_at: unix_to_iso8601(expires_at_unix),
        max_bytes: MAX_STAGED_UPLOAD_BYTES as i64,
        max_chunk_bytes: None,
    })
    .into_response();
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/staged-uploads",
        response.status(),
    );
    response
}

#[cfg(test)]
pub async fn upload_staged_bytes(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(id): AxumPath<String>,
    body: Bytes,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let entry = { state.staging.uploads.lock().unwrap().get(&id).cloned() };
    let Some(entry) = entry else {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged upload.",
        );
    };
    if now_unix() > entry.expires_at_unix {
        state.staging.uploads.lock().unwrap().remove(&id);
        return error_response(
            StatusCode::CONFLICT,
            "staged_upload_expired",
            "This staged upload has expired.",
        );
    }
    if body.len() as u64 > entry.max_bytes {
        return error_response(
            StatusCode::CONFLICT,
            "max_bytes_exceeded",
            "Upload exceeds the staged upload's byte ceiling.",
        );
    }
    if let Some(parent) = entry.path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&entry.path, &body).is_err() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Could not write staged upload.",
        );
    }
    if let Some(upload) = state.staging.uploads.lock().unwrap().get_mut(&id) {
        upload.received_bytes = body.len() as u64;
        upload.complete = true;
    }

    let mut hasher = Sha256::new();
    hasher.update(&body);
    let sha256 = format!("{:x}", hasher.finalize());

    let response = Json(StagedUploadCompleteResultDto {
        staged_upload_id: id.clone(),
        received_bytes: body.len() as i64,
        sha256,
    })
    .into_response();
    audit(
        &state.lifecycle,
        &credential,
        "PUT",
        "/v1/staged-uploads/:id",
        response.status(),
    );
    response
}

#[path = "worlds/import_activation.rs"]
mod import_activation;
use import_activation::run_pre_mutation_safety_backup;
pub use import_activation::*;

// =====================================================================
// Async: convert — always operation-backed, no synchronous variant
// (phase6-api.md SS3: Chunker's process lifetime).
// =====================================================================

/// **Corrected post-review (the owner).** MSC 1 conversion always names a
/// separate, opposite-edition *target* server
/// (`AppViewModel+WorldConversion.swift::performWorldConversion`'s own
/// `sourceServer`/`targetServer` parameters,
/// `WorldConversionWizardView`'s `selectedTargetServer` picker filtered
/// to `s.id != sourceServer.id && (sourceServer.isBedrock ? s.isJava :
/// s.isBedrock)`) — the original P6.21 pass wrongly passed the active
/// server as both source and target. `sourceSlotId` still resolves
/// against the active server (this whole API's implicit-active-server
/// convention); `targetServerId` is now a required, separately-looked-up
/// `ConfigServer`. `targetFormat` is client-chosen and validated against
/// `WorldConverter::supported_formats` — never hardcoded (MSC 1's own
/// wizard defaults its picker to the newest compatible format but always
/// lets the user override it; this route has no picker of its own to
/// default, so an invalid/unsupported value is simply rejected).
pub async fn convert(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldConvertRequestDto>>,
) -> Response {
    let lifecycle = state.lifecycle.clone();
    if state.map_shutdown.load(Ordering::Acquire) {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "agent_stopping",
            "The agent is stopping.",
        );
    }
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Some(Json(body)) = body else {
        return invalid_body("invalid_json", "Request body must be valid JSON.");
    };
    match (&body.target_name, &body.target_slot_id) {
        (Some(_), None) | (None, Some(_)) => {}
        _ => {
            return invalid_body(
                "invalid_body",
                "Exactly one of targetName or targetSlotId must be provided.",
            );
        }
    }

    let source_server = match active_server_or_response(&lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let source_server_dir = Path::new(&source_server.server_dir).to_path_buf();
    let Some(source_slot) = find_slot(&source_server_dir, &body.source_slot_id) else {
        return slot_not_found(&body.source_slot_id);
    };

    let Some(target_server) = lifecycle
        .app_config_servers()
        .into_iter()
        .find(|server| server.id == body.target_server_id)
    else {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "target_server_not_found",
        );
    };
    match lifecycle.reconciliation_status(&target_server.id) {
        ReconciliationStatus::Ready => {}
        ReconciliationStatus::Reconciling => {
            return reconciliation_degraded_response(
                "the conversion target's world reconciliation is still in progress",
            );
        }
        ReconciliationStatus::Degraded { reason } => {
            return reconciliation_degraded_response(&format!(
                "conversion target reconciliation failed: {reason}"
            ));
        }
    }
    let target_server_dir = Path::new(&target_server.server_dir).to_path_buf();
    let target_raw_level_name = crate::backup_operations::configured_java_level_name(
        target_server.server_type,
        &target_server_dir,
    );

    let placement = if let Some(target_slot_id) = &body.target_slot_id {
        let Some(existing) = find_slot(&target_server_dir, target_slot_id) else {
            return slot_not_found(target_slot_id);
        };
        ConversionPlacement::ReplaceExisting { slot: existing }
    } else {
        ConversionPlacement::NewSlot {
            name: body
                .target_name
                .clone()
                .expect("target_name is Some, checked above"),
        }
    };

    let converter = LiveWorldConverter;
    let Some(resolved_java_path) = converter.resolve_java_path("") else {
        return error_response(
            StatusCode::CONFLICT,
            "capability_unavailable",
            "No Java runtime could be resolved for Chunker.",
        );
    };
    if !converter.is_installed() {
        return error_response(
            StatusCode::CONFLICT,
            "capability_unavailable",
            "Chunker is not installed on this agent.",
        );
    }
    let supported_formats = converter.supported_formats(&resolved_java_path);
    if !supported_formats.contains(&body.target_format) {
        return invalid_body(
            "unsupported_target_format",
            &format!(
                "'{}' is not a format the installed Chunker jar supports. Supported: {}.",
                body.target_format,
                supported_formats.join(", ")
            ),
        );
    }

    // This agent only ever runs one server process at a time (the
    // "active" one) — a non-active target server has no process of its
    // own and can never be "running" here, unlike the source, which is
    // always the active server and so always reflects the live status
    // snapshot.
    let running = lifecycle.status_snapshot().running;
    let is_target_running = target_server.id == source_server.id && running;

    // A server can be reconciled before its first start. In that case Paper
    // may have since generated the live world while the active slot still
    // has no saved archive. Snapshot that stopped active world through the
    // existing atomic world writer so conversion does not require a hidden
    // manual "Save current world" prerequisite. Non-active archive-less
    // slots remain a hard not-found error inside the conversion operation.
    let source_slot = if !running
        && !matches!(
            std::fs::metadata(msc_infrastructure::world_store::zip_path(
                &source_server_dir,
                &source_slot.id,
            )),
            Ok(metadata) if metadata.is_file()
        )
        && resolved_active_slot_id(&source_server_dir).as_deref() == Some(source_slot.id.as_str())
    {
        match worlds::update_active_slot_from_current_world(
            &StdFileSystem,
            &source_server_dir,
            source_server.server_type,
            None,
            &source_slot,
        ) {
            Ok(updated) => updated,
            Err(WorldError::NoWorldFolders) => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    "source_world_not_ready",
                    "Start the source server once so it can generate a world, then stop it before converting.",
                );
            }
            Err(error) => return world_error_response(error),
        }
    } else {
        source_slot
    };

    // Journaled against the *target* server, not the source: conversion
    // writes a new/replaced slot into the target, while the source is
    // only ever read (its zip is extracted, never mutated) — exclusivity
    // needs to protect whichever server this operation actually mutates.
    let operation_id = match begin_operation(
        &lifecycle,
        &target_server.id,
        "world-conversion",
        "Starting world conversion.",
    ) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let source_server_type = source_server.server_type;
    let target_server_type = target_server.server_type;
    let target_format = body.target_format.clone();
    let task_lifecycle = lifecycle.clone();
    let task_operation_id = operation_id.clone();
    let task_operation_id_progress = operation_id.clone();
    let profile_lifecycle = task_lifecycle.clone();
    let should_cancel = lifecycle.operations().cancellation_check(&operation_id);
    let backup_should_cancel = should_cancel.clone();
    let source_profile_dir = source_server_dir.clone();
    let target_profile_dir = target_server_dir.clone();
    let source_profile_slot = source_slot.clone();
    tokio::spawn(async move {
        let now = iso8601_now();
        let backup_lifecycle = task_lifecycle.clone();
        let backup_dir = target_server_dir.clone();
        let backup_type = target_server_type;
        let progress_lifecycle = task_lifecycle.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut progress = |line: &str| {
                let _ = progress_lifecycle.operations().progress(
                    &task_operation_id_progress,
                    0,
                    0,
                    line,
                );
            };
            let converted = world_conversion::convert_world(
                &StdFileSystem,
                &converter,
                &resolved_java_path,
                &source_server_dir,
                &source_slot,
                source_server_type,
                running,
                &target_server_dir,
                target_server_type,
                target_raw_level_name.as_deref(),
                &target_format,
                placement,
                is_target_running,
                &now,
                || {
                    run_pre_mutation_safety_backup(
                        &backup_lifecycle,
                        &backup_dir,
                        backup_type,
                        target_raw_level_name.as_deref(),
                        &backup_should_cancel,
                    )
                },
                &mut progress,
                should_cancel,
            );
            if let Ok(target_slot) = &converted {
                // Conversion changes edition and therefore creates or
                // replaces a slot on another server. Copy the source's raw
                // profile after placement so explicitly saved values and
                // newer unknown fields travel with the world too.
                if let Err(error) = msc_infrastructure::world_store::copy_profile(
                    &StdFileSystem,
                    &source_profile_dir,
                    &source_profile_slot,
                    &target_profile_dir,
                    target_slot,
                ) {
                    let _ = profile_lifecycle.operations().progress(
                        &task_operation_id_progress,
                        0,
                        0,
                        &format!("Warning: could not preserve converted world profile: {error}"),
                    );
                }
            }
            converted
        })
        .await;
        match result {
            Ok(Ok(_)) => {
                let mut result = BTreeMap::new();
                result.insert("result".to_string(), "converted".to_string());
                let _ = task_lifecycle.operations().succeed(
                    &task_operation_id,
                    "Conversion complete.",
                    result,
                );
            }
            Ok(Err(ConversionError::Cancelled)) => {
                let _ = task_lifecycle
                    .operations()
                    .cancel(&task_operation_id, "Conversion cancelled.");
            }
            Ok(Err(error)) => {
                let code = if matches!(
                    error,
                    ConversionError::ChunkerNotInstalled | ConversionError::JavaNotFound
                ) {
                    "capability_unavailable"
                } else {
                    "conversion_error"
                };
                let _ =
                    task_lifecycle
                        .operations()
                        .fail(&task_operation_id, code, error.to_string());
            }
            Err(_) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "internal_error",
                    "Conversion task panicked.".to_string(),
                );
            }
        }
    });

    let response = Json(WorldConvertResultDto {
        result: "conversion_started".to_string(),
        operation_id: operation_id.as_str().to_string(),
    })
    .into_response();
    audit(
        &lifecycle,
        &credential,
        "POST",
        "/v1/worlds/convert",
        response.status(),
    );
    response
}

// =====================================================================
// Production `WorldConverter` — real java-path resolution and a real
// Chunker jar-path check; deliberately does **not** implement the
// GitHub-release auto-download flow (a separate, larger feature with no
// route/fixture in this contract calling for it) — see the P6.21 report.
// =====================================================================

#[derive(Default)]
pub struct LiveWorldConverter;

impl LiveWorldConverter {
    fn chunker_jar_path() -> PathBuf {
        msc_infrastructure::chunker::jar_path()
    }
}

impl WorldConverter for LiveWorldConverter {
    fn is_installed(&self) -> bool {
        matches!(std::fs::metadata(Self::chunker_jar_path()), Ok(meta) if meta.is_file())
    }

    fn resolve_java_path(&self, configured_java_path: &str) -> Option<String> {
        if !configured_java_path.trim().is_empty()
            && let Ok(path) =
                msc_infrastructure::java_runtime_detection::normalized_java_executable_path(
                    &StdFileSystem,
                    configured_java_path,
                )
        {
            return Some(path);
        }
        for candidate in [
            "/usr/bin/java",
            "/usr/local/bin/java",
            "/opt/homebrew/bin/java",
        ] {
            if std::fs::metadata(candidate).is_ok() {
                return Some(candidate.to_string());
            }
        }
        std::process::Command::new("which")
            .arg("java")
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn convert(
        &self,
        input_dir: &Path,
        output_dir: &Path,
        target_format: &str,
        java_path: &str,
        progress: &mut dyn FnMut(&str),
    ) -> Result<(), String> {
        use std::io::{BufRead, BufReader};
        use std::process::{Command, Stdio};

        let jar_path = Self::chunker_jar_path();
        let mut child = Command::new(java_path)
            .arg("-jar")
            .arg(&jar_path)
            .arg("-i")
            .arg(input_dir)
            .arg("-f")
            .arg(target_format)
            .arg("-o")
            .arg(output_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("failed to start Chunker: {error}"))?;

        if let Some(stdout) = child.stdout.take() {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    progress(trimmed);
                }
            }
        }
        if let Some(stderr) = child.stderr.take() {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    progress(trimmed);
                }
            }
        }

        let status = child
            .wait()
            .map_err(|error| format!("Chunker process error: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "Chunker exited with code {}",
                status.code().unwrap_or(-1)
            ))
        }
    }

    fn supported_formats(&self, resolved_java_path: &str) -> Vec<String> {
        use std::process::Command;

        let jar_path = Self::chunker_jar_path();
        let Ok(output) = Command::new(resolved_java_path)
            .arg("-jar")
            .arg(&jar_path)
            .arg("-f")
            .arg("?")
            .output()
        else {
            return Vec::new();
        };
        let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
        combined.push(' ');
        combined.push_str(&String::from_utf8_lossy(&output.stderr));

        let mut results = Vec::new();
        for token in combined.split(|c: char| !c.is_alphanumeric() && c != '_') {
            if is_chunker_format_token(token) && !results.contains(&token.to_string()) {
                results.push(token.to_string());
            }
        }
        results
    }
}

/// `ChunkerManager.supportedFormats`'s own regex, `(?:JAVA|BEDROCK)_R?
/// \d+(?:_\d+)*`, reproduced as a manual token check rather than pulling
/// in the `regex` crate for one call site (`msc-agent` has no other
/// regex need) — this crate's own established "write the ~15-line
/// algorithm instead" precedent (`civil_from_days`, this file's own
/// three private copies). `PREVIEW`/`SETTINGS` are excluded per source's
/// own comment ("not conversion targets"); neither could match this
/// shape anyway (both lack a leading digit after the prefix), kept here
/// only for parity with source's explicit exclusion list.
fn is_chunker_format_token(token: &str) -> bool {
    const EXCLUDED: [&str; 2] = ["PREVIEW", "SETTINGS"];
    if EXCLUDED.contains(&token) {
        return false;
    }
    let Some(rest) = token
        .strip_prefix("JAVA_")
        .or_else(|| token.strip_prefix("BEDROCK_"))
    else {
        return false;
    };
    let rest = rest.strip_prefix('R').unwrap_or(rest);
    !rest.is_empty()
        && rest
            .split('_')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

fn iso8601_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    unix_to_iso8601(duration.as_secs())
}

// =====================================================================
// P6.21 tests. Named `world_backup_routes_*` so the plan's Verify
// command (`cargo nextest run -p msc-agent world_backup_routes`, a name
// substring filter — `dto_conformance.rs`'s own doc comment notes
// nextest filters match test name, not file/binary name) selects these
// alongside `routes/backups.rs`'s own tests below. Live here rather than
// in `tests/world_backup_routes.rs` for the same reason
// `tests/backup_scheduler.rs` gives: this crate has no `lib.rs`, so an
// external test file can't reach `crate::routes::worlds` at all — only a
// black-box, spawned-process test could, and that's a much heavier tool
// than these route-logic checks need (`routes/settings.rs` already set
// this exact "tests live inline" precedent).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::CredentialRole;
    use crate::routes::backups::{self, BackupsRoutesState};
    use crate::routes::operations::OperationsState;
    use crate::ws::console::ConsoleState;
    use msc_application::import::ImportedPaperServer;
    use msc_application::lifecycle::ServerId;
    use msc_domain::properties::ServerPropertiesModel;
    use std::collections::HashMap;

    fn imported_server(server_dir: PathBuf) -> ImportedPaperServer {
        ImportedPaperServer {
            id: ServerId::new("paper-1"),
            display_name: "Worlds Route Paper".to_string(),
            paper_jar_path: server_dir.join("paper.jar"),
            server_dir,
            eula_accepted: Some(true),
            game_port: 25565,
            max_players: 20,
            world_name: "world".to_string(),
            properties: ServerPropertiesModel::from_dict(&HashMap::new(), None),
        }
    }

    fn temp_server_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "msc2-worlds-route-{tag}-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("world")).unwrap();
        // Gzip'd Java root compound: world profile application now reads
        // and updates this file when a slot becomes active.
        std::fs::write(
            dir.join("world/level.dat"),
            [
                31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 227, 98, 96, 96, 0, 0, 120, 63, 249, 78, 4, 0,
                0, 0,
            ],
        )
        .unwrap();
        dir
    }

    fn seed_slot_archive(server_dir: &Path, slot_id: &str, level_name: &str) {
        let path = world_store::zip_path(server_dir, slot_id);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(
            format!("{level_name}/level.dat"),
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        std::io::Write::write_all(
            &mut zip,
            &[
                31, 139, 8, 0, 0, 0, 0, 0, 2, 255, 227, 98, 96, 96, 0, 0, 120, 63, 249, 78, 4, 0,
                0, 0,
            ],
        )
        .unwrap();
        zip.finish().unwrap();
    }

    fn state_with_active_server(tag: &str) -> (LifecycleRoutesState, PathBuf) {
        let state = LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );
        let server_dir = temp_server_dir(tag);
        let server = imported_server(server_dir.clone());
        std::fs::write(&server.paper_jar_path, b"fake jar").unwrap();
        state.register_imported_paper(server).unwrap();
        state.select_active_server("paper-1".to_string()).unwrap();
        (state, server_dir)
    }

    fn worlds_credential() -> AuthenticatedCredential {
        AuthenticatedCredential {
            credential_id: "named".to_string(),
            label: "console".to_string(),
            role: CredentialRole::Named,
            permissions: vec![PermissionCategoryDto::Worlds],
        }
    }

    fn other_credential() -> AuthenticatedCredential {
        AuthenticatedCredential {
            credential_id: "named".to_string(),
            label: "console".to_string(),
            role: CredentialRole::Named,
            permissions: vec![PermissionCategoryDto::ServerControl],
        }
    }

    async fn json_body<T: serde::de::DeserializeOwned>(response: Response) -> T {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn world_backup_routes_slot_crud_happy_path() {
        let (lifecycle, server_dir) = state_with_active_server("crud");
        let state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        // create
        let response = create(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldCreateRequestDto {
                name: "Survival".to_string(),
                seed: Some("42".to_string()),
                ..Default::default()
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let created: WorldMutationResultDto = json_body(response).await;
        assert!(created.success);
        let slot_id = created
            .updated
            .as_ref()
            .unwrap()
            .slots
            .first()
            .unwrap()
            .id
            .clone();
        seed_slot_archive(&server_dir, &slot_id, "Survival");

        // list
        let response = list(State(state.clone())).await;
        let listed: WorldSlotsResponseDto = json_body(response).await;
        assert_eq!(listed.slots.len(), 1);

        // rename
        let response = rename(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldRenameRequestDto {
                slot_id: slot_id.clone(),
                name: "Renamed".to_string(),
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let renamed: WorldMutationResultDto = json_body(response).await;
        assert_eq!(
            renamed.updated.unwrap().slots[0].name,
            "Renamed".to_string()
        );

        // duplicate
        let response = duplicate(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldDuplicateRequestDto {
                slot_id: slot_id.clone(),
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let duplicated: WorldMutationResultDto = json_body(response).await;
        assert_eq!(duplicated.updated.unwrap().slots.len(), 2);

        // delete whichever of the two slots is *not* currently resolved
        // active -- `resolve_active_slot_id`'s "newest-created" fallback
        // (`msc_domain::world`) breaks ties on `created_at` by directory
        // listing order when both slots land in the same second (this
        // test's `create`/`duplicate` calls are fast enough that they
        // usually do), so "the duplicate" is not reliably "the inactive
        // one" -- ask the resolver directly instead of assuming.
        let active_id = resolved_active_slot_id(&server_dir);
        let slots = world_store::load_slots(&StdFileSystem, &server_dir);
        let dup_id = slots
            .iter()
            .find(|s| Some(s.id.as_str()) != active_id.as_deref())
            .unwrap()
            .id
            .clone();
        let response = delete(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldDeleteRequestDto { slot_id: dup_id })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let deleted: WorldMutationResultDto = json_body(response).await;
        assert_eq!(deleted.updated.unwrap().slots.len(), 1);
    }

    /// **Corrected post-review**: `/v1/worlds/replace` is
    /// `copySlotIntoExisting`, not a live-world operation — proves the
    /// destination slot's content actually changes (its `zip_size_bytes`
    /// now matches the source's) while the source slot itself is left
    /// untouched, and that `slotId`/`sourceSlotId` are both consumed
    /// (unlike the pre-correction reading, which left `slotId`
    /// unconsumed).
    #[tokio::test]
    async fn world_backup_routes_replace_copies_saved_slot_content_into_destination() {
        let (lifecycle, _dir) = state_with_active_server("replace");
        let state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        let create_slot = |name: &'static str| {
            let state = state.clone();
            let credential = credential.clone();
            async move {
                let response = create(
                    State(state),
                    Extension(credential),
                    Some(Json(WorldCreateRequestDto {
                        name: name.to_string(),
                        seed: None,
                        ..Default::default()
                    })),
                )
                .await;
                assert_eq!(response.status(), StatusCode::OK);
                let created: WorldMutationResultDto = json_body(response).await;
                created
                    .updated
                    .unwrap()
                    .slots
                    .into_iter()
                    .find(|s| s.name == name)
                    .unwrap()
            }
        };

        let source = create_slot("Source").await;
        let destination = create_slot("Destination").await;
        assert_ne!(source.id, destination.id);
        seed_slot_archive(_dir.as_path(), &source.id, "Source");
        seed_slot_archive(_dir.as_path(), &destination.id, "Destination");
        let source_archive_path = world_store::zip_path(&_dir, &source.id);
        let source_archive_before = std::fs::read(&source_archive_path).unwrap();

        let response = replace(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldReplaceRequestDto {
                slot_id: destination.id.clone(),
                source_slot_id: source.id.clone(),
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let replaced: WorldMutationResultDto = json_body(response).await;
        assert!(replaced.success);

        let slots = replaced.updated.unwrap().slots;
        assert_eq!(slots.len(), 2, "no slot is created or removed by replace");
        let updated_destination = slots.iter().find(|s| s.id == destination.id).unwrap();
        let untouched_source = slots.iter().find(|s| s.id == source.id).unwrap();
        let source_archive_size = source_archive_before.len() as i64;
        assert_eq!(
            updated_destination.zip_size_bytes,
            Some(source_archive_size),
            "destination's content now matches the source's"
        );
        assert_eq!(
            untouched_source.zip_size_bytes,
            Some(source_archive_size),
            "the source slot itself is left untouched"
        );
        assert_eq!(
            std::fs::read(source_archive_path).unwrap(),
            source_archive_before,
            "the source archive bytes remain unchanged"
        );

        // A missing destination/source slot is still a 404 either way.
        let response = replace(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldReplaceRequestDto {
                slot_id: "does-not-exist".to_string(),
                source_slot_id: source.id.clone(),
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    /// **Corrected post-review**: `WorldConvertRequestDto` now requires
    /// exactly one of `target_name`/`target_slot_id` (both or neither is
    /// `400 invalid_body`) — the frozen contract's original
    /// `replaceExisting: bool` couldn't express "which slot" once
    /// `targetSlotId` replaced a display-name lookup.
    #[tokio::test]
    async fn world_backup_routes_convert_requires_exactly_one_of_target_name_or_target_slot_id() {
        let (lifecycle, _dir) = state_with_active_server("convert-xor");
        let state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        let base = WorldConvertRequestDto {
            source_slot_id: "slot-1".to_string(),
            target_server_id: "paper-1".to_string(),
            target_format: "JAVA_1_21_4".to_string(),
            target_name: None,
            target_slot_id: None,
        };

        // Neither provided.
        let response = convert(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(base.clone())),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Both provided.
        let mut both = base.clone();
        both.target_name = Some("New Name".to_string());
        both.target_slot_id = Some("slot-2".to_string());
        let response = convert(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(both)),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// **Corrected post-review**: conversion now resolves `sourceSlotId`
    /// against the active server and `targetServerId` against a
    /// *separately looked-up* server — the pre-correction route always
    /// used the same active server for both, so `target_server_not_found`
    /// could never actually fire and a real cross-server conversion was
    /// impossible. This can't exercise a full successful conversion
    /// without a real installed Chunker jar (`LiveWorldConverter` isn't
    /// fake-injectable at the route layer), so it only proves the
    /// plumbing reaches the converter-capability guard with the *correct*
    /// two distinct servers resolved, and that an unknown target server
    /// id is rejected before ever reaching that guard.
    #[tokio::test]
    async fn world_backup_routes_convert_resolves_separate_source_and_target_servers() {
        let (lifecycle, server_dir) = state_with_active_server("convert-cross");
        let state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        let response = create(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldCreateRequestDto {
                name: "Source".to_string(),
                seed: None,
                ..Default::default()
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let created: WorldMutationResultDto = json_body(response).await;
        let source_slot_id = created.updated.unwrap().slots.first().unwrap().id.clone();

        let target_dir = temp_server_dir("convert-cross-target");
        let target_server = ImportedPaperServer {
            id: ServerId::new("paper-2"),
            display_name: "Convert Target Paper".to_string(),
            paper_jar_path: target_dir.join("paper.jar"),
            server_dir: target_dir,
            eula_accepted: Some(true),
            game_port: 25566,
            max_players: 20,
            world_name: "world".to_string(),
            properties: ServerPropertiesModel::from_dict(&HashMap::new(), None),
        };
        std::fs::write(&target_server.paper_jar_path, b"fake jar").unwrap();
        lifecycle.register_imported_paper(target_server).unwrap();

        // An unknown target server is rejected before touching the
        // converter at all.
        let response = convert(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldConvertRequestDto {
                source_slot_id: source_slot_id.clone(),
                target_server_id: "does-not-exist".to_string(),
                target_format: "JAVA_1_21_4".to_string(),
                target_name: Some("Converted".to_string()),
                target_slot_id: None,
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // A real, distinct target server resolves fine and reaches the
        // converter-capability guard (409 capability_unavailable, since
        // no real Chunker jar is installed in this test environment) --
        // not target_server_not_found, proving source/target were
        // correctly distinguished.
        let response = convert(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldConvertRequestDto {
                source_slot_id: source_slot_id.clone(),
                target_server_id: "paper-2".to_string(),
                target_format: "JAVA_1_21_4".to_string(),
                target_name: Some("Converted".to_string()),
                target_slot_id: None,
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body: msc_api::dto::ErrorDto = json_body(response).await;
        assert_eq!(body.code, "capability_unavailable");

        // A distinct target whose imported world archive cannot be
        // reconciled is refused before Chunker capability checks. The
        // source is healthy; authority must follow the server conversion
        // will mutate, not merely the currently active source.
        let degraded_dir = temp_server_dir("convert-degraded-target");
        std::fs::remove_dir_all(degraded_dir.join("world")).unwrap();
        std::fs::create_dir_all(degraded_dir.join("world_slots/slot-corrupt")).unwrap();
        std::fs::write(
            degraded_dir.join("world_slots/slot-corrupt/slot.json"),
            r#"{"id":"slot-corrupt","name":"Broken","created_at":"2026-01-01T00:00:00Z"}"#,
        )
        .unwrap();
        std::fs::write(
            degraded_dir.join("world_slots/slot-corrupt/world.zip"),
            b"not a zip",
        )
        .unwrap();
        std::fs::write(
            degraded_dir.join("world_slots/active_slot_id.txt"),
            "slot-corrupt",
        )
        .unwrap();
        let degraded_target = ImportedPaperServer {
            id: ServerId::new("paper-degraded"),
            display_name: "Degraded Convert Target".to_string(),
            paper_jar_path: degraded_dir.join("paper.jar"),
            server_dir: degraded_dir,
            eula_accepted: Some(true),
            game_port: 25567,
            max_players: 20,
            world_name: "world".to_string(),
            properties: ServerPropertiesModel::from_dict(&HashMap::new(), None),
        };
        std::fs::write(&degraded_target.paper_jar_path, b"fake jar").unwrap();
        lifecycle.register_imported_paper(degraded_target).unwrap();
        lifecycle.set_reconciliation_status(
            "paper-degraded",
            ReconciliationStatus::Degraded {
                reason: "corrupt imported archive".to_string(),
            },
        );

        let response = convert(
            State(state),
            Extension(credential),
            Some(Json(WorldConvertRequestDto {
                source_slot_id,
                target_server_id: "paper-degraded".to_string(),
                target_format: "JAVA_1_21_4".to_string(),
                target_name: Some("Must Not Convert".to_string()),
                target_slot_id: None,
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body: msc_api::dto::ErrorDto = json_body(response).await;
        assert_eq!(body.code, "world_reconciliation_degraded");

        let _ = server_dir;
    }

    #[tokio::test]
    async fn world_backup_routes_activate_is_async_and_pollable() {
        let (lifecycle, _dir) = state_with_active_server("activate");
        let state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        let created = create(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldCreateRequestDto {
                name: "Survival".to_string(),
                seed: None,
                ..Default::default()
            })),
        )
        .await;
        let created: WorldMutationResultDto = json_body(created).await;
        let slot_id = created.updated.unwrap().slots[0].id.clone();
        let response = activate(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(serde_json::json!({ "slotId": slot_id.clone() }))),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let result: WorldActivateResultDto = json_body(response).await;
        assert_eq!(result.result, "activation_started");
        let operation_id = result.operation_id.expect("activation is operation-backed");

        // Poll until the background task finishes (real filesystem work,
        // no fake clock to advance).
        let mut snapshot = None;
        for _ in 0..200 {
            if let Some(record) = lifecycle.operations().snapshot(&operation_id)
                && (record.state == msc_api::dto::OperationStateDto::Succeeded
                    || record.state == msc_api::dto::OperationStateDto::Failed)
            {
                snapshot = Some(record);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let snapshot = snapshot.expect("activation operation reached a terminal state");
        assert_eq!(snapshot.state, msc_api::dto::OperationStateDto::Succeeded);
    }

    #[tokio::test]
    async fn world_backup_routes_staged_upload_import_round_trip() {
        let (lifecycle, _dir) = state_with_active_server("staged-import");
        let state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        let begin = begin_staged_upload(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(StagedUploadBeginRequestDto {
                purpose: StagedUploadPurposeDto::WorldImport,
                content_type: None,
                file_name: None,
                operation_id: None,
                file_id: None,
                expected_bytes: None,
            })),
        )
        .await;
        assert_eq!(begin.status(), StatusCode::OK);
        let begun: StagedUploadBeginResultDto = json_body(begin).await;

        // Loose external world files must reach normalization through the real
        // route. Activation's strict layout rejects this source before import;
        // using it here catches a route precheck bypassing the normalizer.
        let zip_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            {
                let mut writer = zip::ZipWriter::new(&mut buf);
                writer
                    .start_file("level.dat", zip::write::SimpleFileOptions::default())
                    .unwrap();
                use std::io::Write;
                writer.write_all(b"fake level dat").unwrap();
                writer.finish().unwrap();
            }
            buf.into_inner()
        };

        let upload = upload_staged_bytes(
            State(state.clone()),
            Extension(credential.clone()),
            AxumPath(begun.staged_upload_id.clone()),
            Bytes::from(zip_bytes),
        )
        .await;
        assert_eq!(upload.status(), StatusCode::OK);
        let uploaded: StagedUploadCompleteResultDto = json_body(upload).await;
        assert!(uploaded.received_bytes > 0);
        assert!(!uploaded.sha256.is_empty());

        let imported = import(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldImportRequestDto {
                name: "Imported World".to_string(),
                staged_upload_id: begun.staged_upload_id.clone(),
                backup_id: None,
            })),
        )
        .await;
        assert_eq!(imported.status(), StatusCode::OK);
        let imported: WorldMutationResultDto = json_body(imported).await;
        assert!(imported.success);
        let slots = imported.updated.unwrap().slots;
        assert_eq!(slots.len(), 1);
        let names = msc_infrastructure::archive::list_entry_names(&world_store::zip_path(
            &_dir,
            &slots[0].id,
        ))
        .unwrap();
        assert!(names.contains(&"world/level.dat".to_string()));

        // Re-uploading (or re-importing) the same, already-redeemed id is
        // a plain 404.
        let second = upload_staged_bytes(
            State(state.clone()),
            Extension(credential.clone()),
            AxumPath(begun.staged_upload_id.clone()),
            Bytes::from_static(b"x"),
        )
        .await;
        assert_eq!(second.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn world_backup_routes_staged_thumbnail_upload_round_trip() {
        let (lifecycle, server_dir) = state_with_active_server("staged-thumbnail");
        let state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        let created = create(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldCreateRequestDto {
                name: "Thumbnail World".to_string(),
                seed: None,
                ..Default::default()
            })),
        )
        .await;
        let created: WorldMutationResultDto = json_body(created).await;
        let slot_id = created.updated.unwrap().slots[0].id.clone();

        let begin = begin_staged_upload(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(StagedUploadBeginRequestDto {
                purpose: StagedUploadPurposeDto::WorldThumbnail,
                content_type: Some("image/png".to_string()),
                file_name: None,
                operation_id: None,
                file_id: None,
                expected_bytes: None,
            })),
        )
        .await;
        assert_eq!(begin.status(), StatusCode::OK);
        let begun: StagedUploadBeginResultDto = json_body(begin).await;

        let source = image::RgbaImage::from_pixel(1200, 900, image::Rgba([32, 96, 144, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(source)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let upload = upload_staged_bytes(
            State(state.clone()),
            Extension(credential.clone()),
            AxumPath(begun.staged_upload_id.clone()),
            Bytes::from(png.into_inner()),
        )
        .await;
        assert_eq!(upload.status(), StatusCode::OK);

        let response = set_thumbnail(
            State(state.clone()),
            Extension(credential.clone()),
            AxumPath(slot_id.clone()),
            Some(Json(WorldThumbnailUploadRequestDto {
                staged_upload_id: begun.staged_upload_id,
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let result: WorldMutationResultDto = json_body(response).await;
        assert!(result.success);
        assert!(
            result
                .updated
                .unwrap()
                .slots
                .into_iter()
                .find(|slot| slot.id == slot_id)
                .unwrap()
                .has_thumbnail
        );

        let thumbnail_response = thumbnail(State(state.clone()), AxumPath(slot_id.clone())).await;
        assert_eq!(thumbnail_response.status(), StatusCode::OK);
        assert_eq!(
            thumbnail_response
                .headers()
                .get(header::CONTENT_TYPE)
                .unwrap(),
            "image/jpeg"
        );

        let thumbnail_path = server_dir.join(format!("world_slots/{slot_id}/thumbnail.jpg"));
        let thumbnail = image::load_from_memory(&std::fs::read(thumbnail_path).unwrap()).unwrap();
        assert_eq!((thumbnail.width(), thumbnail.height()), (600, 450));
    }

    #[tokio::test]
    async fn world_backup_routes_staged_thumbnail_upload_rejects_invalid_image_and_wrong_purpose() {
        let (lifecycle, server_dir) = state_with_active_server("staged-thumbnail-invalid");
        let state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        let created = create(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldCreateRequestDto {
                name: "Thumbnail World".to_string(),
                seed: None,
                ..Default::default()
            })),
        )
        .await;
        let created: WorldMutationResultDto = json_body(created).await;
        let slot_id = created.updated.unwrap().slots[0].id.clone();

        let begin = begin_staged_upload(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(StagedUploadBeginRequestDto {
                purpose: StagedUploadPurposeDto::WorldThumbnail,
                content_type: None,
                file_name: None,
                operation_id: None,
                file_id: None,
                expected_bytes: None,
            })),
        )
        .await;
        let begun: StagedUploadBeginResultDto = json_body(begin).await;
        let upload = upload_staged_bytes(
            State(state.clone()),
            Extension(credential.clone()),
            AxumPath(begun.staged_upload_id.clone()),
            Bytes::from_static(b"not an image"),
        )
        .await;
        assert_eq!(upload.status(), StatusCode::OK);

        let response = set_thumbnail(
            State(state.clone()),
            Extension(credential.clone()),
            AxumPath(slot_id.clone()),
            Some(Json(WorldThumbnailUploadRequestDto {
                staged_upload_id: begun.staged_upload_id.clone(),
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(
            !server_dir
                .join(format!("world_slots/{slot_id}/thumbnail.jpg"))
                .exists()
        );

        let second = set_thumbnail(
            State(state.clone()),
            Extension(credential.clone()),
            AxumPath(slot_id.clone()),
            Some(Json(WorldThumbnailUploadRequestDto {
                staged_upload_id: begun.staged_upload_id,
            })),
        )
        .await;
        assert_eq!(second.status(), StatusCode::NOT_FOUND);

        let wrong_purpose = begin_staged_upload(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(StagedUploadBeginRequestDto {
                purpose: StagedUploadPurposeDto::WorldImport,
                content_type: None,
                file_name: None,
                operation_id: None,
                file_id: None,
                expected_bytes: None,
            })),
        )
        .await;
        let wrong_purpose: StagedUploadBeginResultDto = json_body(wrong_purpose).await;
        let response = set_thumbnail(
            State(state),
            Extension(credential),
            AxumPath(slot_id),
            Some(Json(WorldThumbnailUploadRequestDto {
                staged_upload_id: wrong_purpose.staged_upload_id,
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn world_backup_routes_staged_export_download_round_trip_and_single_redemption() {
        let (lifecycle, server_dir) = state_with_active_server("staged-export");
        let state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        let created = create(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldCreateRequestDto {
                name: "Survival".to_string(),
                seed: None,
                ..Default::default()
            })),
        )
        .await;
        let created: WorldMutationResultDto = json_body(created).await;
        let slot_id = created.updated.unwrap().slots[0].id.clone();
        seed_slot_archive(&server_dir, &slot_id, "Survival");

        let exported = export(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldExportRequestDto {
                slot_id: slot_id.clone(),
            })),
        )
        .await;
        assert_eq!(exported.status(), StatusCode::OK);
        let exported: WorldExportResultDto = json_body(exported).await;
        assert!(exported.size_bytes > 0);

        let first = download_staged_bytes(
            State(state.clone()),
            AxumPath(exported.staged_download_id.clone()),
        )
        .await;
        assert_eq!(first.status(), StatusCode::OK);

        let second = download_staged_bytes(
            State(state.clone()),
            AxumPath(exported.staged_download_id.clone()),
        )
        .await;
        assert_eq!(second.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn world_backup_routes_replace_active_fresh_round_trip_creates_safety_backup() {
        let (lifecycle, server_dir) = state_with_active_server("replace-fresh");
        let state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        assert!(server_dir.join("world").exists());

        let response = replace_active(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldReplaceActiveRequestDto {
                new_level_name: "brand-new".to_string(),
                staged_upload_id: None,
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let result: WorldReplaceActiveResultDto = json_body(response).await;
        assert_eq!(result.result, "replace_started");
        let operation_id = result.operation_id.expect("replace is operation-backed");

        let snapshot = poll_operation_to_terminal(&lifecycle, &operation_id).await;
        assert_eq!(
            snapshot.state,
            msc_api::dto::OperationStateDto::Succeeded,
            "{snapshot:?}"
        );

        // The old "world" folder is gone -- a fresh world generates on
        // next start -- but the mandatory, untokened pre-replace safety
        // backup protects it first.
        assert!(!server_dir.join("world").exists());
        let backups = msc_application::backups::list_backups(&StdFileSystem, &server_dir);
        assert!(
            backups.iter().any(|b| b.trigger_reason == "pre-replace"),
            "expected a pre-replace safety backup, got {backups:?}"
        );
    }

    #[tokio::test]
    async fn world_backup_routes_replace_active_staged_upload_round_trip() {
        let (lifecycle, server_dir) = state_with_active_server("replace-staged");
        let state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        let begin = begin_staged_upload(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(StagedUploadBeginRequestDto {
                purpose: StagedUploadPurposeDto::ActiveWorldReplace,
                content_type: None,
                file_name: None,
                operation_id: None,
                file_id: None,
                expected_bytes: None,
            })),
        )
        .await;
        assert_eq!(begin.status(), StatusCode::OK);
        let begun: StagedUploadBeginResultDto = json_body(begin).await;

        // A real, tiny zip whose one top-level entry is named after the
        // new level name -- `apply_world_identity` only ever writes
        // `level-name` into `server.properties`; it doesn't rename
        // anything on disk, so a caller-supplied `newLevelName` must
        // already match the uploaded source's own folder name for the
        // two to agree post-replace (the same contract P6.33's
        // `replace_world` already established for `BackupZip`/
        // `ExistingFolder` sources).
        let zip_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            {
                let mut writer = zip::ZipWriter::new(&mut buf);
                writer
                    .start_file(
                        "restored-world/level.dat",
                        zip::write::SimpleFileOptions::default(),
                    )
                    .unwrap();
                use std::io::Write;
                writer.write_all(b"uploaded level dat").unwrap();
                writer.finish().unwrap();
            }
            buf.into_inner()
        };
        let upload = upload_staged_bytes(
            State(state.clone()),
            Extension(credential.clone()),
            AxumPath(begun.staged_upload_id.clone()),
            Bytes::from(zip_bytes),
        )
        .await;
        assert_eq!(upload.status(), StatusCode::OK);

        let response = replace_active(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldReplaceActiveRequestDto {
                new_level_name: "restored-world".to_string(),
                staged_upload_id: Some(begun.staged_upload_id.clone()),
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let result: WorldReplaceActiveResultDto = json_body(response).await;
        let operation_id = result.operation_id.expect("replace is operation-backed");

        let snapshot = poll_operation_to_terminal(&lifecycle, &operation_id).await;
        assert_eq!(
            snapshot.state,
            msc_api::dto::OperationStateDto::Succeeded,
            "{snapshot:?}"
        );

        let installed = server_dir.join("restored-world/level.dat");
        assert_eq!(std::fs::read(&installed).unwrap(), b"uploaded level dat");

        // An already-redeemed staged upload cannot be reused.
        let second = replace_active(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldReplaceActiveRequestDto {
                new_level_name: "again".to_string(),
                staged_upload_id: Some(begun.staged_upload_id.clone()),
            })),
        )
        .await;
        assert_eq!(second.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn world_backup_routes_replace_active_rejects_wrong_purpose_staged_upload() {
        let (lifecycle, _dir) = state_with_active_server("replace-wrong-purpose");
        let state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        let begin = begin_staged_upload(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(StagedUploadBeginRequestDto {
                purpose: StagedUploadPurposeDto::WorldImport,
                content_type: None,
                file_name: None,
                operation_id: None,
                file_id: None,
                expected_bytes: None,
            })),
        )
        .await;
        let begun: StagedUploadBeginResultDto = json_body(begin).await;
        let upload = upload_staged_bytes(
            State(state.clone()),
            Extension(credential.clone()),
            AxumPath(begun.staged_upload_id.clone()),
            Bytes::from_static(b"irrelevant bytes for this purpose-only guard"),
        )
        .await;
        assert_eq!(upload.status(), StatusCode::OK);

        // `replace_active` never redeems a staged upload begun for a
        // different purpose, even though `upload_staged_bytes` itself
        // accepted the bytes -- "a staging slot can only be redeemed by
        // the route it was created for" (phase6-api.md SS4).
        let response = replace_active(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldReplaceActiveRequestDto {
                new_level_name: "whatever".to_string(),
                staged_upload_id: Some(begun.staged_upload_id.clone()),
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn world_backup_routes_replace_active_requires_worlds_permission() {
        let (lifecycle, _dir) = state_with_active_server("replace-perm");
        let state = WorldsRoutesState::new(lifecycle);
        let response = replace_active(
            State(state),
            Extension(other_credential()),
            Some(Json(WorldReplaceActiveRequestDto {
                new_level_name: "whatever".to_string(),
                staged_upload_id: None,
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    /// Shared poll loop for `world-replace-active`'s async operation —
    /// real filesystem work, no fake clock to advance, mirroring
    /// `world_backup_routes_activate_is_async_and_pollable`'s own
    /// inline loop.
    async fn poll_operation_to_terminal(
        lifecycle: &LifecycleRoutesState,
        operation_id: &str,
    ) -> msc_api::dto::OperationDto {
        for _ in 0..200 {
            if let Some(record) = lifecycle.operations().snapshot(operation_id)
                && (record.state == msc_api::dto::OperationStateDto::Succeeded
                    || record.state == msc_api::dto::OperationStateDto::Failed)
            {
                return record;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("operation {operation_id} never reached a terminal state");
    }

    #[tokio::test]
    async fn world_backup_routes_mutation_requires_worlds_permission() {
        let (lifecycle, _dir) = state_with_active_server("perm");
        let state = WorldsRoutesState::new(lifecycle);
        let response = create(
            State(state),
            Extension(other_credential()),
            Some(Json(WorldCreateRequestDto {
                name: "Survival".to_string(),
                seed: None,
                ..Default::default()
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn world_backup_routes_per_server_exclusivity_rejects_concurrent_mutation() {
        let (lifecycle, _dir) = state_with_active_server("exclusivity");
        let credential = worlds_credential();
        let server = lifecycle.active_config_server().unwrap();

        let first = begin_operation(&lifecycle, &server.id, "world-create", "Working.");
        assert!(first.is_ok());
        let second = begin_operation(&lifecycle, &server.id, "world-create", "Working.");
        assert!(
            second.is_err(),
            "a second mutation against the same active server must be refused, not queued"
        );
        let response = second.err().unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let _ = credential;
    }

    #[tokio::test]
    async fn world_backup_routes_backup_now_and_delete_and_sole_verified_refusal() {
        let (lifecycle, _dir) = state_with_active_server("backup-delete");
        let credential = worlds_credential();

        let scheduler = test_backup_scheduler();
        let backups_state = BackupsRoutesState {
            lifecycle: lifecycle.clone(),
            scheduler,
        };

        let response =
            backups::now(State(backups_state.clone()), Extension(credential.clone())).await;
        assert_eq!(response.status(), StatusCode::OK);
        let started: msc_api::dto::BackupNowResultDto = json_body(response).await;
        let operation_id = started.operation_id.unwrap();

        let mut snapshot = None;
        for _ in 0..200 {
            if let Some(record) = lifecycle.operations().snapshot(&operation_id)
                && (record.state == msc_api::dto::OperationStateDto::Succeeded
                    || record.state == msc_api::dto::OperationStateDto::Failed)
            {
                snapshot = Some(record);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let snapshot = snapshot.expect("backup-now operation reached a terminal state");
        assert_eq!(
            snapshot.state,
            msc_api::dto::OperationStateDto::Succeeded,
            "{:?}",
            snapshot.error
        );

        let list_response = backups::list(State(backups_state.clone())).await;
        let listed: msc_api::dto::BackupsResponseDto = json_body(list_response).await;
        assert_eq!(listed.backups.len(), 1);
        let backup_id = listed.backups[0].id.clone();

        // The sole verified backup can't be deleted.
        let refused = backups::delete(
            State(backups_state.clone()),
            Extension(credential.clone()),
            Some(Json(msc_api::dto::BackupDeleteRequestDto {
                backup_id: backup_id.clone(),
            })),
        )
        .await;
        assert_eq!(refused.status(), StatusCode::CONFLICT);

        // A second backup makes deletion legal again.
        let response =
            backups::now(State(backups_state.clone()), Extension(credential.clone())).await;
        let started: msc_api::dto::BackupNowResultDto = json_body(response).await;
        let operation_id = started.operation_id.unwrap();
        for _ in 0..200 {
            if let Some(record) = lifecycle.operations().snapshot(&operation_id)
                && record.state == msc_api::dto::OperationStateDto::Succeeded
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        // Guard against a colliding filename (same-second timestamp):
        // if the scheduler only produced one distinct file, skip the
        // now-legal-delete assertion rather than flake.
        let list_response = backups::list(State(backups_state.clone())).await;
        let listed: msc_api::dto::BackupsResponseDto = json_body(list_response).await;
        if listed.backups.len() >= 2 {
            let allowed = backups::delete(
                State(backups_state.clone()),
                Extension(credential.clone()),
                Some(Json(msc_api::dto::BackupDeleteRequestDto { backup_id })),
            )
            .await;
            assert_eq!(allowed.status(), StatusCode::OK);
        }
    }

    struct NoopSchedulerBackend;
    impl crate::backup_scheduler::SchedulerBackend for NoopSchedulerBackend {
        fn is_running(&self, _server_id: &str) -> bool {
            false
        }
        fn online_player_count(&self, _server_id: &str) -> usize {
            0
        }
        fn run_scheduled_backup(&self, _server_id: &str) {}
    }

    pub(crate) fn test_backup_scheduler() -> &'static crate::backup_scheduler::BackupScheduler {
        Box::leak(Box::new(crate::backup_scheduler::BackupScheduler::new(
            std::sync::Arc::new(NoopSchedulerBackend),
        )))
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameruleCatalogQuery {
    server_type: String,
    minecraft_version: Option<String>,
    java_flavor: Option<String>,
    #[serde(default)]
    active_server: bool,
}

pub async fn gamerule_catalog(
    State(state): State<WorldsRoutesState>,
    Query(query): Query<GameruleCatalogQuery>,
) -> Response {
    if !["java", "bedrock"].contains(&query.server_type.as_str()) {
        return invalid_body("invalid_server_type", "serverType must be java or bedrock.");
    }
    let active = query
        .active_server
        .then(|| state.lifecycle.active_config_server())
        .flatten();
    let installed_version = active
        .as_ref()
        .filter(|server| server.server_type.raw_value() == query.server_type)
        .and_then(|server| gamerule_server_version(server, &state.lifecycle));
    let version = crate::routes::versions::minecraft_version_from_selection(
        query
            .java_flavor
            .as_deref()
            .and_then(msc_domain::identity::JavaServerFlavor::from_raw_value),
        installed_version.or(query.minecraft_version),
    )
    .filter(|version| !version.eq_ignore_ascii_case("latest"));
    let catalog =
        msc_infrastructure::gamerule_catalog::catalog(&query.server_type, version.as_deref());
    Json(msc_api::dto::GameruleCatalogDto {
        server_type: query.server_type.clone(),
        minecraft_version: version,
        available: catalog.is_some(),
        complete: catalog.is_some_and(|catalog| catalog.complete),
        source: catalog.map(|catalog| catalog.source.clone()),
        note: Some(if catalog.is_some() && query.server_type == "java" {
            "Verified built-in rules. Server mods may add rules; use Edit as Text for those.".into()
        } else if catalog.is_some() {
            "Verified built-in rules for this Bedrock release.".into()
        } else {
            "Rule picker unavailable for this version. You can enter rules manually or keep Minecraft's defaults.".into()
        }),
        rules: catalog.map(|catalog| catalog.rules.iter().map(|rule| msc_api::dto::GameruleDefinitionDto {
            id: rule.id.clone(), label: rule.label.clone(), description: rule.description.clone(),
            value_type: rule.value_type.clone(), default_value: rule.default_value.clone(),
            choices: rule.choices.clone(), experimental: rule.experimental, minimum: rule.minimum, maximum: rule.maximum,
        }).collect()).unwrap_or_default(),
    }).into_response()
}

// Bedrock records its release separately from Java, and LATEST is only a selection policy.
fn gamerule_server_version(
    server: &ConfigServer,
    lifecycle: &LifecycleRoutesState,
) -> Option<String> {
    if server.server_type == ServerType::Bedrock {
        let runtime = crate::routes::bedrock::runtime_for(lifecycle);
        crate::routes::versions::installed_bedrock_version(server, runtime.as_ref())
    } else {
        server.minecraft_version.clone()
    }
}
