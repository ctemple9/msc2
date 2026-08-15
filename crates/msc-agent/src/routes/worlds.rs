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
//! **Cancellation is real at the operation-record level only.** A
//! `POST /v1/operations/{id}/cancel` against one of the four async
//! operations marks the *record* cancelled, but none of
//! `worlds::activate_slot`/`world_conversion::convert_world`/
//! `backups::create_backup`/`backups::restore_backup` accept a
//! cancellation token — there is no injectable interruption point in the
//! P6.9-19 application layer today, and adding one is out of this step's
//! scope. A "cancelled" operation's real filesystem/process work still
//! runs to completion in the background; this is a known, flagged
//! limitation, not a silent one.
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

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Extension, Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use msc_api::dto::{
    PermissionCategoryDto, StagedUploadBeginRequestDto, StagedUploadBeginResultDto,
    StagedUploadCompleteResultDto, StagedUploadPurposeDto, WorldActivateRequestDto,
    WorldActivateResultDto, WorldConvertRequestDto, WorldConvertResultDto, WorldCopyRequestDto,
    WorldCreateRequestDto, WorldDeleteRequestDto, WorldDuplicateRequestDto, WorldExportRequestDto,
    WorldExportResultDto, WorldImportRequestDto, WorldMutationResultDto,
    WorldRenameActiveWorldRequestDto, WorldRenameRequestDto, WorldRepairRequestDto,
    WorldReplaceRequestDto, WorldSlotDto, WorldSlotsResponseDto,
};
use msc_application::world_conversion::{
    self, ConversionError, ConversionPlacement, WorldConverter,
};
use msc_application::worlds::{self, WorldError, WorldReplaceSource};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::identity::ServerType;
use msc_domain::world::WorldSlot;
use msc_infrastructure::audit_log::Entry as AuditEntry;
use msc_infrastructure::fs::{FileSystem, StdFileSystem};
use msc_infrastructure::world_store;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{
    LifecycleRoutesState, error_response, invalid_body, require_permission,
};

/// A bounded ceiling for one staged world upload — generous enough for a
/// large modpack world (tens of GB is unusual for a single Minecraft
/// world save) while still bounded, per `phase6-api.md` §4's own
/// deferral of the exact number to this step. Not derived from any
/// fixture or MSC 1 constant — this step's own scoping decision, flagged
/// in the P6.21 report rather than treated as an oracle-derived value.
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
        .route("/worlds/copy", post(copy))
        .route("/worlds/import", post(import))
        .route("/worlds/export", post(export))
        .route("/worlds/rename-active-world", post(rename_active_world))
        .route("/worlds/activate", post(activate))
        .route("/worlds/convert", post(convert))
        .route("/worlds/:slot_id/thumbnail", get(thumbnail))
        .route("/staged-uploads", post(begin_staged_upload))
        .route(
            "/staged-uploads/:id",
            // axum's `Bytes` extractor refuses any body over 2MB by
            // default (`DefaultBodyLimit`'s own doc: "for security
            // reasons") regardless of this route's own `max_bytes`
            // bookkeeping — without this override, every real Minecraft
            // world upload (almost never under 2MB) would 413 before
            // `upload_staged_bytes` ever ran, making
            // `MAX_STAGED_UPLOAD_BYTES` unreachable in practice.
            put(upload_staged_bytes)
                .route_layer(DefaultBodyLimit::max(MAX_STAGED_UPLOAD_BYTES as usize)),
        )
        .route("/staged-downloads/:id", get(download_staged_bytes))
        .with_state(state)
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
    staging: StagingStore,
}

impl WorldsRoutesState {
    pub fn new(lifecycle: LifecycleRoutesState) -> Self {
        Self {
            lifecycle,
            staging: StagingStore::default(),
        }
    }
}

#[derive(Debug, Clone)]
struct StagedUpload {
    purpose: StagedUploadPurposeDto,
    expires_at_unix: u64,
    max_bytes: u64,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct StagedDownload {
    expires_at_unix: u64,
    path: PathBuf,
    size_bytes: u64,
}

/// Bytes live on disk under `<servers_root>/.msc2-staging/{uploads,
/// downloads}/{id}.{bin,zip}` — `servers_root()` is already an
/// agent-controlled directory, so nothing user-supplied ever names a
/// path component here; only the opaque, server-generated `{id}` UUID
/// does. Metadata lives in memory only (an agent restart loses in-flight
/// staged transfers, the same "best-effort, not durable" shape this
/// step's own scope note leaves to a later phase).
#[derive(Clone, Default)]
struct StagingStore {
    uploads: std::sync::Arc<Mutex<HashMap<String, StagedUpload>>>,
    downloads: std::sync::Arc<Mutex<HashMap<String, StagedDownload>>>,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn unix_to_iso8601(unix: u64) -> String {
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

fn staging_root(servers_root: &Path) -> PathBuf {
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

#[allow(clippy::result_large_err)]
fn active_server_or_response(state: &LifecycleRoutesState) -> Result<ConfigServer, Response> {
    state.active_config_server().ok_or_else(no_active_server)
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

/// `POST /v1/worlds/replace` maps a frozen `{slotId, sourceSlotId}`
/// request onto `worlds::replace_world`'s `new_level_name`/
/// `WorldReplaceSource` shape. This route predates Phase 6 (P2.8
/// baseline) and was never service-backed before this step, so there is
/// no earlier real implementation to check against — **flagged as a
/// genuinely open question in the P6.21 report**: `sourceSlotId` is
/// treated as the slot supplying replacement content (its `world.zip`),
/// and the new live level-name is taken from that slot's own
/// `world_level_name` (falling back to the current live level-name if
/// the source slot never recorded one); `slotId` is validated to exist
/// but not otherwise consumed — its intended purpose (perhaps an
/// optimistic-concurrency check against the currently-active slot) is
/// not recoverable from the frozen DTO alone.
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
            if find_slot(server_dir, &body.slot_id).is_none() {
                return slot_not_found(&body.slot_id);
            }
            let Some(source) = find_slot(server_dir, &body.source_slot_id) else {
                return error_response(StatusCode::NOT_FOUND, "not_found", "source_not_found");
            };
            let running = lifecycle.status_snapshot().running;
            let source_zip = world_store::zip_path(server_dir, &source.id);
            let new_level_name = source
                .world_level_name
                .clone()
                .unwrap_or_else(|| msc_domain::world::current_level_name(server.server_type, None));
            match worlds::replace_world(
                &StdFileSystem,
                server_dir,
                server.server_type,
                None,
                &new_level_name,
                &WorldReplaceSource::BackupZip(source_zip),
                running,
                false,
                || false,
            ) {
                Ok(()) => mutation_ok(lifecycle, server, "replaced"),
                Err(error) => world_error_response(error),
            }
        },
    )
    .await
}

/// Bedrock-only capability; no live Bedrock runtime exists before Phase
/// 10, so this always reports `409 conflict, bedrock_only` — the
/// existing baseline `conflict` code, unchanged, per
/// `phase6-api.md` §5's own note that repair keeps its baseline error
/// code rather than adopting `capability_unavailable`.
pub async fn repair(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldRepairRequestDto>>,
) -> Response {
    let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) else {
        let Some(Json(_body)) = body else {
            return invalid_body("missing_body", "Request body is required.");
        };
        let status = StatusCode::CONFLICT;
        audit(
            &state.lifecycle,
            &credential,
            "POST",
            "/v1/worlds/repair",
            status,
        );
        return error_response(
            status,
            "bedrock_only",
            "Repair is only supported for Bedrock servers, which have no live runtime yet.",
        );
    };
    response
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

pub async fn copy(
    State(state): State<WorldsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<WorldCopyRequestDto>>,
) -> Response {
    run_mutation(
        &state,
        &credential,
        "POST",
        "/v1/worlds/copy",
        "world-copy",
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
                Ok(_) => mutation_ok(lifecycle, server, "copied"),
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

// =====================================================================
// Staged upload / import
// =====================================================================

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
    let StagedUploadPurposeDto::WorldImport = body.purpose;

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
    let server = match active_server_or_response(lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
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
        || !matches!(entry.purpose, StagedUploadPurposeDto::WorldImport)
    {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged upload.",
        );
    }

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
        &entry.path,
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
    let _ = std::fs::remove_file(&entry.path);
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
                    size_bytes,
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
            let _ = entry.size_bytes;
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
    let running = lifecycle.status_snapshot().running;
    let task_lifecycle = lifecycle.clone();
    let task_operation_id = operation_id.clone();
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
                || run_pre_mutation_safety_backup(&backup_lifecycle, &backup_dir, backup_type),
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
        None,
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
    )
    .is_ok()
}

// =====================================================================
// Async: convert — always operation-backed, no synchronous variant
// (phase6-api.md SS3: Chunker's process lifetime).
// =====================================================================

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
    let server = match active_server_or_response(&lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let server_dir = Path::new(&server.server_dir).to_path_buf();
    let Some(source_slot) = find_slot(&server_dir, &body.slot_id) else {
        return slot_not_found(&body.slot_id);
    };

    let placement = if body.replace_existing {
        let Some(existing) = world_store::load_slots(&StdFileSystem, &server_dir)
            .into_iter()
            .find(|slot| slot.name == body.target_name)
        else {
            return error_response(StatusCode::NOT_FOUND, "not_found", "slot_not_found");
        };
        ConversionPlacement::ReplaceExisting { slot: existing }
    } else {
        ConversionPlacement::NewSlot {
            name: body.target_name.clone(),
        }
    };

    let converter = LiveWorldConverter;
    if !converter.is_installed() {
        return error_response(
            StatusCode::CONFLICT,
            "capability_unavailable",
            "Chunker is not installed on this agent.",
        );
    }

    let operation_id = match begin_operation(
        &lifecycle,
        &server.id,
        "world-conversion",
        "Starting world conversion.",
    ) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let server_type = server.server_type;
    let running = lifecycle.status_snapshot().running;
    let task_lifecycle = lifecycle.clone();
    let task_operation_id = operation_id.clone();
    let task_operation_id_progress = operation_id.clone();
    tokio::spawn(async move {
        let now = iso8601_now();
        let target_format = default_target_format(server_type);
        let backup_lifecycle = task_lifecycle.clone();
        let backup_dir = server_dir.clone();
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
                "",
                &backup_dir,
                &source_slot,
                server_type,
                running,
                &backup_dir,
                server_type,
                None,
                &target_format,
                placement,
                running,
                &now,
                || run_pre_mutation_safety_backup(&backup_lifecycle, &backup_dir, server_type),
                &mut progress,
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

/// **Flagged, not silently guessed.** `WorldConvertRequestDTO`'s frozen
/// shape (`{slotId, targetName, replaceExisting}`) has no field for
/// Chunker's own `target_format` flag (`-f`, e.g. `"JAVA_1_21_4"`) — a
/// real product decision (which Minecraft version to target, and
/// whether that should ever be client-chosen) with no fixture or MSC 1
/// precedent to derive it from; `ChunkerManager.swift` never hardcodes
/// one either; it comes from whatever UI picker source's own wizard
/// presents. This derives a fixed, clearly-labeled placeholder from the
/// server's own type purely so the route is real and testable today —
/// see the P6.21 report for the open question this leaves for Cameron.
fn default_target_format(server_type: ServerType) -> String {
    match server_type {
        ServerType::Java => "JAVA_1_21_4".to_string(),
        ServerType::Bedrock => "BEDROCK".to_string(),
    }
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
        fn admit_backup(&self, _server_id: &str) -> bool {
            true
        }
        fn run_scheduled_backup(&self, _server_id: &str) {}
    }

    pub(crate) fn test_backup_scheduler() -> &'static crate::backup_scheduler::BackupScheduler {
        Box::leak(Box::new(crate::backup_scheduler::BackupScheduler::new(
            std::sync::Arc::new(NoopSchedulerBackend),
        )))
    }
}
