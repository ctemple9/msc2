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
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
use axum::body::Bytes;
use axum::extract::{Extension, Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use msc_api::dto::{
    PermissionCategoryDto, StagedUploadPurposeDto, WorldActivateRequestDto, WorldActivateResultDto,
    WorldConvertFormatsResponseDto, WorldConvertRequestDto, WorldConvertResultDto,
    WorldCreateRequestDto, WorldDeleteRequestDto, WorldDuplicateRequestDto, WorldExportRequestDto,
    WorldExportResultDto, WorldImportRequestDto, WorldMutationResultDto,
    WorldRenameActiveWorldRequestDto, WorldRenameRequestDto, WorldRepairRequestDto,
    WorldRepairResultDto, WorldReplaceActiveRequestDto, WorldReplaceActiveResultDto,
    WorldReplaceRequestDto, WorldSlotDto, WorldSlotsResponseDto, WorldThumbnailUploadRequestDto,
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
use msc_application::worlds::{self, WorldError, WorldReplaceSource};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::identity::ServerType;
use msc_domain::world::WorldSlot;
use msc_infrastructure::audit_log::Entry as AuditEntry;
use msc_infrastructure::fs::{FileSystem, StdFileSystem};
use msc_infrastructure::world_store;
#[cfg(test)]
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::auth::AuthenticatedCredential;
use crate::routes::bedrock::require_runtime;
use crate::routes::lifecycle::{
    LifecycleRoutesState, ReconciliationStatus, error_response, invalid_body,
    reconciliation_degraded_response, require_permission,
};

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
const STAGING_TTL_SECONDS: u64 = 30 * 60;

pub fn router(state: WorldsRoutesState) -> Router {
    Router::new()
        .route("/worlds", get(list))
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
        .route("/worlds/convert/formats", get(convert_formats))
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
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    if let Err(response) = active_server_or_response(&lifecycle) {
        return response;
    }

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

    let response = Json(WorldConvertFormatsResponseDto {
        formats: converter.supported_formats(&resolved_java_path),
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

// =====================================================================
// Shared route state: `LifecycleRoutesState` plus the staged-upload/
// download store — a new, self-contained type kept in this file (world
// import/export's own concern) rather than a new crate module, to stay
// inside this step's own file list.
// =====================================================================

#[derive(Clone)]
pub struct WorldsRoutesState {
    pub lifecycle: LifecycleRoutesState,
    pub(crate) staging: StagingStore,
}

impl WorldsRoutesState {
    #[cfg(test)]
    pub fn new(lifecycle: LifecycleRoutesState) -> Self {
        Self {
            lifecycle,
            staging: StagingStore::default(),
        }
    }

    pub fn with_staging(lifecycle: LifecycleRoutesState, staging: StagingStore) -> Self {
        Self { lifecycle, staging }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StagedUpload {
    pub(crate) purpose: StagedUploadPurposeDto,
    pub(crate) expires_at_unix: u64,
    pub(crate) max_bytes: u64,
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
            let now = iso8601_now();
            match worlds::create_slot_from_current_world(
                &StdFileSystem,
                Path::new(&server.server_dir),
                server.server_type,
                None,
                name,
                body.seed.as_deref(),
                &now,
            ) {
                Ok(_) => mutation_ok(lifecycle, server, "created"),
                Err(error) => world_error_response(error),
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

/// `POST /v1/worlds/replace` — **corrected post-review (Cameron)**: this
/// is `WorldSlotManager.copySlotIntoExisting(source, into: dest, ...)`,
/// a saved-slot-to-saved-slot copy, not `AppViewModel+WorldManagement
/// .swift::replaceWorld`'s live-world operation the original P6.21 pass
/// guessed at (that guess is what the "flagged as a genuinely open
/// question" comment previously here recorded — Cameron's answer:
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
            headers.insert(header::CONTENT_TYPE, "image/png".parse().unwrap());
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
        | StagedUploadPurposeDto::CurseforgeManualFile => {
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
            expires_at_unix,
            max_bytes: MAX_STAGED_UPLOAD_BYTES,
            path,
        },
    );

    let response = Json(StagedUploadBeginResultDto {
        staged_upload_id: id.clone(),
        upload_path: format!("/v1/staged-uploads/{id}"),
        expires_at: unix_to_iso8601(expires_at_unix),
        max_bytes: MAX_STAGED_UPLOAD_BYTES as i64,
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

pub async fn import(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldImportRequestDto>>,
) -> Response {
    let lifecycle = &state.lifecycle;
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Some(Json(body)) = body else {
        return invalid_body("invalid_json", "Request body must be valid JSON.");
    };
    if body.name.trim().is_empty() {
        return invalid_body("name_required", "name must not be blank.");
    }
    let staged_upload_id = body.staged_upload_id.trim();
    let backup_id = body.backup_id.as_deref().map(str::trim);
    let has_staged_upload = !staged_upload_id.is_empty();
    let has_backup = backup_id.is_some_and(|id| !id.is_empty());
    if has_staged_upload == has_backup {
        return invalid_body(
            "invalid_body",
            "exactly one of stagedUploadId or backupId must be provided.",
        );
    }
    let server = match active_server_or_response(lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let (source_path, staged_path) = if has_staged_upload {
        let entry = {
            state
                .staging
                .uploads
                .lock()
                .unwrap()
                .remove(staged_upload_id)
        };
        let Some(entry) = entry else {
            return error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                "Unknown or already-redeemed staged upload.",
            );
        };
        if now_unix() > entry.expires_at_unix
            || !matches!(entry.purpose, StagedUploadPurposeDto::WorldImport)
        {
            return error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                "Unknown or already-redeemed staged upload.",
            );
        }
        (entry.path.clone(), Some(entry.path))
    } else {
        let backup_id = backup_id.expect("exactly one source was validated above");
        let server_dir = Path::new(&server.server_dir);
        let Some(entry) = backups::list_backups(&StdFileSystem, server_dir)
            .into_iter()
            .find(|entry| entry.filename == backup_id)
        else {
            return error_response(StatusCode::NOT_FOUND, "backup_not_found", "No such backup.");
        };
        (entry.zip_path, None)
    };

    let operation_id =
        match begin_operation(lifecycle, &server.id, "world-import", "Importing world.") {
            Ok(id) => id,
            Err(response) => return response,
        };
    let now = iso8601_now();
    let response = match worlds::import_zip_as_new_slot(
        &StdFileSystem,
        Path::new(&server.server_dir),
        server.server_type,
        None,
        &source_path,
        body.name.trim(),
        &now,
    ) {
        Ok(_) => {
            let _ = lifecycle
                .operations()
                .succeed(&operation_id, "Imported.", BTreeMap::new());
            mutation_ok(lifecycle, &server, "imported")
        }
        Err(error) => {
            let _ = lifecycle
                .operations()
                .fail(&operation_id, "world_error", error.to_string());
            world_error_response(error)
        }
    };
    if let Some(staged_path) = staged_path {
        let _ = std::fs::remove_file(staged_path);
    }
    audit(
        lifecycle,
        &credential,
        "POST",
        "/v1/worlds/import",
        response.status(),
    );
    response
}

pub async fn export(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldExportRequestDto>>,
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
    let Some(slot) = find_slot(server_dir, &body.slot_id) else {
        return slot_not_found(&body.slot_id);
    };

    let id = Uuid::new_v4().to_string();
    let servers_root = lifecycle.servers_root();
    let downloads_dir = staging_root(&servers_root).join("downloads");
    if std::fs::create_dir_all(&downloads_dir).is_err() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Could not prepare staging directory.",
        );
    }
    let destination = downloads_dir.join(format!("{id}.zip"));

    let response = match worlds::export_slot_zip(&StdFileSystem, server_dir, &slot, &destination) {
        Ok(()) => {
            let size_bytes = std::fs::metadata(&destination)
                .map(|m| m.len())
                .unwrap_or(0);
            let expires_at_unix = now_unix() + STAGING_TTL_SECONDS;
            state.staging.downloads.lock().unwrap().insert(
                id.clone(),
                StagedDownload {
                    expires_at_unix,
                    path: destination,
                },
            );
            Json(WorldExportResultDto {
                staged_download_id: id,
                expires_at: unix_to_iso8601(expires_at_unix),
                size_bytes: size_bytes as i64,
            })
            .into_response()
        }
        Err(error) => world_error_response(error),
    };
    audit(
        lifecycle,
        &credential,
        "POST",
        "/v1/worlds/export",
        response.status(),
    );
    response
}

#[cfg(test)]
pub async fn download_staged_bytes(
    State(state): State<WorldsRoutesState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let entry = { state.staging.downloads.lock().unwrap().remove(&id) };
    let Some(entry) = entry else {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged download.",
        );
    };
    if now_unix() > entry.expires_at_unix {
        let _ = std::fs::remove_file(&entry.path);
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged download.",
        );
    }
    match std::fs::read(&entry.path) {
        Ok(bytes) => {
            let _ = std::fs::remove_file(&entry.path);
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "application/zip".parse().unwrap());
            (StatusCode::OK, headers, bytes).into_response()
        }
        Err(_) => error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged download.",
        ),
    }
}

// =====================================================================
// Async: activate
// =====================================================================

pub async fn activate(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldActivateRequestDto>>,
) -> Response {
    let lifecycle = state.lifecycle.clone();
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Some(Json(body)) = body else {
        return invalid_body("invalid_json", "Request body must be valid JSON.");
    };
    let server = match active_server_or_response(&lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let server_dir = Path::new(&server.server_dir).to_path_buf();
    let Some(slot) = find_slot(&server_dir, &body.slot_id) else {
        return error_response(
            StatusCode::CONFLICT,
            "conflict",
            "server_running_or_slot_not_found",
        );
    };
    let operation_id = match begin_operation(
        &lifecycle,
        &server.id,
        "world-activate",
        "Activating world slot.",
    ) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let server_type = server.server_type;
    let raw_level_name =
        crate::backup_operations::configured_java_level_name(server_type, &server_dir);
    let running = lifecycle.status_snapshot().running;
    let task_lifecycle = lifecycle.clone();
    let task_operation_id = operation_id.clone();
    let should_cancel = lifecycle.operations().cancellation_check(&operation_id);
    let backup_should_cancel = should_cancel.clone();
    tokio::spawn(async move {
        let now = iso8601_now();
        let backup_lifecycle = task_lifecycle.clone();
        let backup_dir = server_dir.clone();
        let backup_type = server_type;
        let result = tokio::task::spawn_blocking(move || {
            worlds::activate_slot(
                &StdFileSystem,
                &backup_dir,
                backup_type,
                &slot,
                running,
                &now,
                || {
                    run_pre_mutation_safety_backup(
                        &backup_lifecycle,
                        &backup_dir,
                        backup_type,
                        raw_level_name.as_deref(),
                        &backup_should_cancel,
                    )
                },
                should_cancel,
            )
        })
        .await;
        match result {
            Ok(Ok(_)) => {
                let mut result = BTreeMap::new();
                result.insert("result".to_string(), "activated".to_string());
                let _ = task_lifecycle.operations().succeed(
                    &task_operation_id,
                    "Activation complete.",
                    result,
                );
            }
            Ok(Err(worlds::ActivationError::Cancelled)) => {
                let _ = task_lifecycle
                    .operations()
                    .cancel(&task_operation_id, "Activation cancelled.");
            }
            Ok(Err(error)) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "world_error",
                    error.to_string(),
                );
            }
            Err(_) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "internal_error",
                    "Activation task panicked.".to_string(),
                );
            }
        }
    });

    let response = Json(WorldActivateResultDto {
        result: "activation_started".to_string(),
        operation_id: Some(operation_id.as_str().to_string()),
    })
    .into_response();
    audit(
        &lifecycle,
        &credential,
        "POST",
        "/v1/worlds/activate",
        response.status(),
    );
    response
}

/// The mandatory pre-activation/pre-replace/pre-conversion safety backup
/// every P6.13/16/19 caller needs as a `impl FnOnce() -> bool` — a real
/// manual, tokened `backups::create_backup` call over the same server
/// directory, matching every other "already handled, report success"
/// caller in this codebase.
fn run_pre_mutation_safety_backup(
    lifecycle: &LifecycleRoutesState,
    server_dir: &Path,
    server_type: ServerType,
    raw_level_name: Option<&str>,
    should_cancel: impl Fn() -> bool,
) -> bool {
    let _ = lifecycle;
    let now = iso8601_now();
    let association = msc_domain::world::BackupAssociation {
        slot_id: None,
        slot_name: None,
        world_seed: None,
    };
    msc_application::backups::create_backup(
        &StdFileSystem,
        server_dir,
        server_type,
        raw_level_name,
        &association,
        None,
        None,
        false,
        true,
        Some("pre-mutation"),
        None,
        &now,
        None,
        || false,
        should_cancel,
    )
    .is_ok()
}

// =====================================================================
// Async: replace-active-world (P6.34) — exposes P6.33's transactional
// `worlds::replace_world` through the agent, separately named from
// `POST /v1/worlds/replace` (`replace` above, a saved-slot-to-saved-slot
// copy — `phase6-api.md` SS9/SS10 records why they're distinct
// operations). Follows `routes/backups.rs::restore`'s shape — a
// mandatory safety backup plus a transactional live-world swap, guard-
// ordered the same way (cheap up-front checks, then a journaled
// operation, then the real work on a spawned blocking task) — rather
// than `activate`'s, since restore is the closer existing analog.
// =====================================================================

pub async fn replace_active(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldReplaceActiveRequestDto>>,
) -> Response {
    let lifecycle = state.lifecycle.clone();
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let Some(Json(body)) = body else {
        return invalid_body("invalid_json", "Request body must be valid JSON.");
    };
    if body.new_level_name.trim().is_empty() {
        return invalid_body("name_required", "newLevelName must not be blank.");
    }
    let server = match active_server_or_response(&lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let server_dir = Path::new(&server.server_dir).to_path_buf();

    // Cheap, up-front checks that need no journaled operation to observe
    // them — mirroring `restore`'s own "running-server, then missing-
    // source" ordering. `replace_world` re-checks running-server itself
    // (`false` is passed below because this guard already refused
    // otherwise), the same "outer route pre-checks, inner service
    // re-checks" belt-and-braces `restore` already has.
    if lifecycle.status_snapshot().running {
        return error_response(StatusCode::CONFLICT, "server_running", "Server is running.");
    }

    // Redeem the staged upload up front (if any) — the same "missing,
    // expired, or wrong-purpose staged id is a plain 404" contract
    // `import` already established. Never an arbitrary server-local
    // path: the only sources this route can ever build are `Fresh` (no
    // upload given) or `BackupZip` (an uploaded, bounded, server-
    // generated staging path) — `WorldReplaceSource::ExistingFolder`
    // is unreachable from this route.
    let world_source = if let Some(staged_upload_id) = &body.staged_upload_id {
        let entry = {
            state
                .staging
                .uploads
                .lock()
                .unwrap()
                .remove(staged_upload_id)
        };
        let Some(entry) = entry else {
            return error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                "Unknown or already-redeemed staged upload.",
            );
        };
        if now_unix() > entry.expires_at_unix
            || !matches!(entry.purpose, StagedUploadPurposeDto::ActiveWorldReplace)
        {
            return error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                "Unknown or already-redeemed staged upload.",
            );
        }
        WorldReplaceSource::BackupZip(entry.path)
    } else {
        WorldReplaceSource::Fresh
    };

    // Resolve the mandatory safety backup's association before spawning
    // — the same "load slots/active id up front, hand the association to
    // the background task" shape `restore` already uses.
    let slots = world_store::load_slots(&StdFileSystem, &server_dir);
    let marker = world_store::load_explicit_active_slot_id(&StdFileSystem, &server_dir);
    let active_id = msc_domain::world::resolve_active_slot_id(&slots, marker.as_deref());
    let association =
        msc_domain::world::effective_backup_association(&slots, active_id.as_deref(), None, None);

    let operation_id = match begin_operation(
        &lifecycle,
        &server.id,
        "world-replace-active",
        "Replacing active world.",
    ) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let server_type = server.server_type;
    let raw_level_name =
        crate::backup_operations::configured_java_level_name(server_type, &server_dir);
    let server_id = server.id.clone();
    let server_name = server.display_name.clone();
    let new_level_name = body.new_level_name.trim().to_string();
    let task_lifecycle = lifecycle.clone();
    let task_operation_id = operation_id.clone();
    let should_cancel = lifecycle.operations().cancellation_check(&operation_id);
    tokio::spawn(async move {
        let now = iso8601_now();
        let result = tokio::task::spawn_blocking(move || {
            worlds::replace_world(
                &StdFileSystem,
                &server_dir,
                server_type,
                raw_level_name.as_deref(),
                &new_level_name,
                &world_source,
                false,
                &association,
                Some(&server_id),
                Some(&server_name),
                &now,
                should_cancel,
            )
        })
        .await;
        match result {
            Ok(Ok(_)) => {
                let mut result = BTreeMap::new();
                result.insert("result".to_string(), "replaced".to_string());
                let _ = task_lifecycle.operations().succeed(
                    &task_operation_id,
                    "Replacement complete.",
                    result,
                );
            }
            Ok(Err(WorldError::Cancelled)) => {
                let _ = task_lifecycle
                    .operations()
                    .cancel(&task_operation_id, "Replacement cancelled.");
            }
            Ok(Err(error)) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "world_error",
                    error.to_string(),
                );
            }
            Err(_) => {
                let _ = task_lifecycle.operations().fail(
                    &task_operation_id,
                    "internal_error",
                    "Replacement task panicked.".to_string(),
                );
            }
        }
    });

    let response = Json(WorldReplaceActiveResultDto {
        result: "replace_started".to_string(),
        operation_id: Some(operation_id.as_str().to_string()),
    })
    .into_response();
    audit(
        &lifecycle,
        &credential,
        "POST",
        "/v1/worlds/replace-active-world",
        response.status(),
    );
    response
}

// =====================================================================
// Async: convert — always operation-backed, no synchronous variant
// (phase6-api.md SS3: Chunker's process lifetime).
// =====================================================================

/// **Corrected post-review (Cameron).** MSC 1 conversion always names a
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
    let should_cancel = lifecycle.operations().cancellation_check(&operation_id);
    let backup_should_cancel = should_cancel.clone();
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
            world_conversion::convert_world(
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
            )
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
        std::env::var_os("MSC2_CHUNKER_JAR_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs_app_support_dir().join("chunker-cli.jar"))
    }
}

fn dirs_app_support_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        #[cfg(target_os = "macos")]
        {
            return home.join("Library/Application Support/MSC2");
        }
        #[cfg(not(target_os = "macos"))]
        {
            return home.join(".msc2");
        }
    }
    std::env::temp_dir().join("msc2")
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
        std::fs::write(dir.join("world/level.dat"), b"fake").unwrap();
        dir
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
        assert_eq!(
            updated_destination.zip_size_bytes, source.zip_size_bytes,
            "destination's content now matches the source's"
        );
        assert_eq!(
            untouched_source.zip_size_bytes, source.zip_size_bytes,
            "the source slot itself is left untouched"
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
            })),
        )
        .await;
        let created: WorldMutationResultDto = json_body(created).await;
        let slot_id = created.updated.unwrap().slots[0].id.clone();

        let response = activate(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldActivateRequestDto {
                slot_id: slot_id.clone(),
            })),
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
                operation_id: None,
                file_id: None,
            })),
        )
        .await;
        assert_eq!(begin.status(), StatusCode::OK);
        let begun: StagedUploadBeginResultDto = json_body(begin).await;

        // Build a tiny real zip to upload.
        let zip_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            {
                let mut writer = zip::ZipWriter::new(&mut buf);
                writer
                    .start_file("world/level.dat", zip::write::SimpleFileOptions::default())
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
        assert_eq!(imported.updated.unwrap().slots.len(), 1);

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
                operation_id: None,
                file_id: None,
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
                operation_id: None,
                file_id: None,
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
                operation_id: None,
                file_id: None,
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
        let (lifecycle, _dir) = state_with_active_server("staged-export");
        let state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        let created = create(
            State(state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldCreateRequestDto {
                name: "Survival".to_string(),
                seed: None,
            })),
        )
        .await;
        let created: WorldMutationResultDto = json_body(created).await;
        let slot_id = created.updated.unwrap().slots[0].id.clone();

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
                operation_id: None,
                file_id: None,
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
                operation_id: None,
                file_id: None,
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
        let worlds_state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();

        // A real world archive must exist before a backup can be taken.
        let created = create(
            State(worlds_state.clone()),
            Extension(credential.clone()),
            Some(Json(WorldCreateRequestDto {
                name: "Survival".to_string(),
                seed: None,
            })),
        )
        .await;
        assert_eq!(created.status(), StatusCode::OK);

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
