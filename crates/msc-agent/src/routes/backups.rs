//! Phase 6 backup routes (P6.21) — backed by the real
//! `msc_application::backups` service over `StdFileSystem` and the
//! active server's real directory, per `docs/msc2/worlds/phase6-api.md`.
//! See `routes/worlds.rs`'s module doc for the shared operation-
//! journaling/exclusivity/cancellation/audit design this module reuses.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use msc_api::dto::{
    BackupConfigResponseDto, BackupConfigUpdateRequestDto, BackupConfigUpdateResultDto,
    BackupDeleteRequestDto, BackupItemDto, BackupNowResultDto, BackupRestoreRequestDto,
    BackupRestoreResultDto, BackupsResponseDto, PermissionCategoryDto, SimpleResultDto,
};
use msc_application::backups::{self, RestoreError};
use msc_domain::backup as domain_backup;
use msc_domain::identity::ServerType;
use msc_infrastructure::audit_log::Entry as AuditEntry;
use msc_infrastructure::backup_store::BackupEntry;
use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::world_store;

use crate::auth::AuthenticatedCredential;
use crate::backup_scheduler::BackupScheduler;
use crate::routes::lifecycle::{
    LifecycleRoutesState, ReconciliationStatus, error_response, invalid_body,
    reconciliation_degraded_response, require_permission,
};
use msc_domain::app_config_schema::ConfigServer;

/// The default interval choices `GET /v1/backups/config` offers a
/// client — mirrors `AppViewModel+Backups.swift`'s own picker options.
/// No fixture pins this exact list; a reasonable, documented default per
/// this step's own scope (the same kind of "P6.21 wiring decision"
/// `phase6-api.md` §4 already anticipates for the staged-transfer
/// ceiling).
const INTERVAL_OPTIONS_MINUTES: [i64; 5] = [15, 30, 60, 120, 240];

#[derive(Clone)]
pub struct BackupsRoutesState {
    pub lifecycle: LifecycleRoutesState,
    pub scheduler: &'static BackupScheduler,
}

pub fn router(state: BackupsRoutesState) -> Router {
    Router::new()
        .route("/backups", get(list))
        .route("/backups/config", get(get_config).post(update_config))
        .route("/backups/now", post(now))
        .route("/backups/restore", post(restore))
        .route("/backups/delete", post(delete))
        .with_state(state)
}

fn no_active_server() -> Response {
    error_response(
        StatusCode::CONFLICT,
        "conflict",
        "No server is currently active.",
    )
}

/// Resolves the active server for a backup mutation route (`now`,
/// `restore`, `delete`), refusing (per P6.29, mirroring `routes/
/// worlds.rs`'s own gate) a server left `Degraded` by startup
/// reconciliation before any mutation runs. `list`/`get_config`/
/// `update_config` deliberately keep calling `active_config_server`
/// directly — reading and editing backup *settings* isn't a world/backup
/// mutation, and stays available for diagnosis on a damaged server.
#[allow(clippy::result_large_err)]
fn active_server_or_response(state: &LifecycleRoutesState) -> Result<ConfigServer, Response> {
    let server = state.active_config_server().ok_or_else(no_active_server)?;
    if let ReconciliationStatus::Degraded { reason } = state.reconciliation_status(&server.id) {
        return Err(reconciliation_degraded_response(&reason));
    }
    Ok(server)
}

fn iso8601_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = duration.as_secs() as i64;
    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

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

fn to_item_dto(entry: &BackupEntry) -> BackupItemDto {
    let modification_date = entry.modified.map(|time| {
        let secs = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let days = (secs / 86_400) as i64;
        let secs_of_day = secs % 86_400;
        let (year, month, day) = civil_from_days(days);
        let hour = secs_of_day / 3_600;
        let minute = (secs_of_day % 3_600) / 60;
        let second = secs_of_day % 60;
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
    });
    BackupItemDto {
        id: entry.filename.clone(),
        display_name: entry.display_name.clone(),
        file_size: entry.file_size.map(|size| size as i64),
        modification_date,
        is_automatic: domain_backup::is_automatic_trigger(&entry.trigger_reason),
        slot_id: entry.slot_id.clone(),
        slot_name: entry.slot_name.clone(),
        trigger_reason: entry.trigger_reason.clone(),
    }
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

pub async fn list(State(state): State<BackupsRoutesState>) -> Response {
    let Some(server) = state.lifecycle.active_config_server() else {
        return Json(BackupsResponseDto {
            backups: Vec::new(),
        })
        .into_response();
    };
    let entries = backups::list_backups(&StdFileSystem, Path::new(&server.server_dir));
    Json(BackupsResponseDto {
        backups: entries.iter().map(to_item_dto).collect(),
    })
    .into_response()
}

pub async fn get_config(State(state): State<BackupsRoutesState>) -> Response {
    let Some(server) = state.lifecycle.active_config_server() else {
        return Json(BackupConfigResponseDto {
            server_name: String::new(),
            auto_backup_enabled: false,
            auto_backup_interval_minutes: 30,
            auto_backup_max_count: 10,
            interval_options: INTERVAL_OPTIONS_MINUTES.to_vec(),
            note: Some("no_active_server".to_string()),
        })
        .into_response();
    };
    Json(BackupConfigResponseDto {
        server_name: server.display_name,
        auto_backup_enabled: server.auto_backup_enabled,
        auto_backup_interval_minutes: server.auto_backup_interval_minutes,
        auto_backup_max_count: server.auto_backup_max_count,
        interval_options: INTERVAL_OPTIONS_MINUTES.to_vec(),
        note: None,
    })
    .into_response()
}

pub async fn update_config(
    State(state): State<BackupsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<BackupConfigUpdateRequestDto>>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Settings) {
        return response;
    }
    let Some(Json(body)) = body else {
        return invalid_body("invalid_json", "Request body must be valid JSON.");
    };
    if body.auto_backup_enabled.is_none()
        && body.auto_backup_interval_minutes.is_none()
        && body.auto_backup_max_count.is_none()
    {
        return invalid_body("no_changes", "At least one field must be present.");
    }
    let Some(server) = state.lifecycle.active_config_server() else {
        return no_active_server();
    };

    let response = match state.lifecycle.update_backup_config(
        &server.id,
        body.auto_backup_enabled,
        body.auto_backup_interval_minutes,
        body.auto_backup_max_count,
    ) {
        Ok(updated) => {
            // Live-reconfigure the scheduler with the latest server list
            // so a changed interval/enabled flag takes effect immediately
            // — `backup_scheduler.rs`'s own doc names this exact call as
            // P6.21's route-wiring job.
            state
                .scheduler
                .reconfigure(&state.lifecycle.app_config_servers());
            Json(BackupConfigUpdateResultDto {
                success: true,
                message: "saved".to_string(),
                config: Some(BackupConfigResponseDto {
                    server_name: updated.display_name,
                    auto_backup_enabled: updated.auto_backup_enabled,
                    auto_backup_interval_minutes: updated.auto_backup_interval_minutes,
                    auto_backup_max_count: updated.auto_backup_max_count,
                    interval_options: INTERVAL_OPTIONS_MINUTES.to_vec(),
                    note: None,
                }),
            })
            .into_response()
        }
        Err(error) => error_response(StatusCode::CONFLICT, "conflict", &error.to_string()),
    };
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/backups/config",
        response.status(),
    );
    response
}

pub async fn now(
    State(state): State<BackupsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    let lifecycle = state.lifecycle.clone();
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Worlds) {
        return response;
    }
    let server = match active_server_or_response(&lifecycle) {
        Ok(server) => server,
        Err(response) => return response,
    };
    let running = lifecycle.status_snapshot().running;
    // P6.31: routed through the one authoritative backup operation this
    // agent now shares with the scheduler's own fired tick — see
    // `backup_operations`'s module doc for why the manual route no
    // longer builds its own `LiveBackupConsole`/`create_backup` call
    // here.
    let operation_id =
        match crate::backup_operations::start_backup(&lifecycle, server, running, false, None) {
            Ok(id) => id,
            Err(error) => return crate::routes::operations::operation_error_response(error),
        };

    let response = Json(BackupNowResultDto {
        result: "backup_started".to_string(),
        operation_id: Some(operation_id.as_str().to_string()),
    })
    .into_response();
    audit(
        &lifecycle,
        &credential,
        "POST",
        "/v1/backups/now",
        response.status(),
    );
    response
}

pub async fn restore(
    State(state): State<BackupsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<BackupRestoreRequestDto>>,
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

    let entries = backups::list_backups(&StdFileSystem, &server_dir);
    let Some(entry) = entries
        .into_iter()
        .find(|entry| entry.filename == body.backup_id)
    else {
        return error_response(StatusCode::NOT_FOUND, "backup_not_found", "No such backup.");
    };

    // Guard-ordered exactly as `fixtures/backup-restore` pins: Bedrock,
    // then running-server, then cross-slot, then missing-source — the
    // first three are checked up front (cheap, no journaled operation
    // needed to observe them); missing-source is re-checked inside
    // `restore_backup` itself and surfaces from there.
    if server.server_type == ServerType::Bedrock {
        return error_response(
            StatusCode::CONFLICT,
            "capability_unavailable",
            "Live-world restore is currently supported for Java servers only.",
        );
    }
    let running = lifecycle.status_snapshot().running;
    if running {
        return error_response(StatusCode::CONFLICT, "conflict", "Server is running.");
    }
    let slots = world_store::load_slots(&StdFileSystem, &server_dir);
    let marker = world_store::load_explicit_active_slot_id(&StdFileSystem, &server_dir);
    let active_id = msc_domain::world::resolve_active_slot_id(&slots, marker.as_deref());
    if let (Some(backup_slot), Some(active_slot)) = (entry.slot_id.as_deref(), active_id.as_deref())
        && backup_slot != active_slot
    {
        return error_response(
            StatusCode::CONFLICT,
            "conflict",
            "Backup belongs to a different slot than the active one.",
        );
    }

    let operation_id = match lifecycle.operations().begin_lifecycle(
        "backup-restore",
        Some(server.id.clone()),
        "Restoring backup.",
    ) {
        Ok(id) => id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };

    let server_type = server.server_type;
    let server_id = server.id.clone();
    let server_name = server.display_name.clone();
    let task_lifecycle = lifecycle.clone();
    let task_operation_id = operation_id.clone();
    let should_cancel = lifecycle.operations().cancellation_check(&operation_id);
    tokio::spawn(async move {
        let now = iso8601_now();
        let association = msc_domain::world::effective_backup_association(
            &slots,
            active_id.as_deref(),
            None,
            None,
        );
        let zip_path = entry.zip_path.clone();
        let backup_slot_id = entry.slot_id.clone();
        let result = tokio::task::spawn_blocking(move || {
            backups::restore_backup(
                &StdFileSystem,
                &server_dir,
                server_type,
                None,
                &zip_path,
                backup_slot_id.as_deref(),
                active_id.as_deref(),
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
                result.insert("result".to_string(), "restored".to_string());
                let _ = task_lifecycle.operations().succeed(
                    &task_operation_id,
                    "Restore complete.",
                    result,
                );
            }
            Ok(Err(RestoreError::Cancelled)) => {
                let _ = task_lifecycle
                    .operations()
                    .cancel(&task_operation_id, "Restore cancelled.");
            }
            Ok(Err(error)) => {
                let code = match error {
                    RestoreError::BedrockNotSupported => "capability_unavailable",
                    _ => "restore_error",
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
                    "Restore task panicked.".to_string(),
                );
            }
        }
    });

    let response = Json(BackupRestoreResultDto {
        result: "restore_started".to_string(),
        operation_id: Some(operation_id.as_str().to_string()),
    })
    .into_response();
    audit(
        &lifecycle,
        &credential,
        "POST",
        "/v1/backups/restore",
        response.status(),
    );
    response
}

pub async fn delete(
    State(state): State<BackupsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Option<Json<BackupDeleteRequestDto>>,
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
    let server_dir = Path::new(&server.server_dir);
    let entries = backups::list_backups(&StdFileSystem, server_dir);
    let Some(entry) = entries
        .iter()
        .find(|entry| entry.filename == body.backup_id)
    else {
        return error_response(StatusCode::NOT_FOUND, "backup_not_found", "No such backup.");
    };

    // Phase 6's own retention-floor correction (`phase6-api.md`'s
    // `deleteBackup` note): refuse to drop the sole remaining verified
    // backup, mirroring `backup_store::prune_managed_backups`'s own
    // floor for automatic pruning but applied here to a manual,
    // single-backup delete too.
    if entry.verified {
        let verified_count = entries.iter().filter(|e| e.verified).count();
        if verified_count <= 1 {
            return error_response(
                StatusCode::CONFLICT,
                "sole_verified_backup",
                "Cannot delete the last remaining verified backup.",
            );
        }
    }

    let response = match backups::delete_backup(&StdFileSystem, &entry.zip_path) {
        Ok(()) => Json(SimpleResultDto {
            result: "deleted".to_string(),
            active_server_id: None,
            operation_id: None,
        })
        .into_response(),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        ),
    };
    audit(
        &lifecycle,
        &credential,
        "POST",
        "/v1/backups/delete",
        response.status(),
    );
    response
}

// =====================================================================
// P6.21 tests -- see `routes/worlds.rs`'s own test module doc for why
// these live inline (`world_backup_routes_*`-prefixed, matching the
// plan's Verify command's nextest name filter) rather than in
// `tests/world_backup_routes.rs`.
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::CredentialRole;
    use crate::backup_scheduler::{BackupScheduler, SchedulerBackend};
    use crate::routes::operations::OperationsState;
    use crate::routes::worlds::{self, WorldsRoutesState};
    use crate::ws::console::ConsoleState;
    use axum::extract::{Extension, State};
    use msc_api::dto::{PermissionCategoryDto, WorldCreateRequestDto, WorldMutationResultDto};
    use msc_domain::app_config_schema::ConfigServer;
    use std::path::PathBuf;
    use uuid::Uuid;

    struct NoopSchedulerBackend;
    impl SchedulerBackend for NoopSchedulerBackend {
        fn is_running(&self, _server_id: &str) -> bool {
            false
        }
        fn online_player_count(&self, _server_id: &str) -> usize {
            0
        }
        fn run_scheduled_backup(&self, _server_id: &str) {}
    }

    fn test_backup_scheduler() -> &'static BackupScheduler {
        Box::leak(Box::new(BackupScheduler::new(std::sync::Arc::new(
            NoopSchedulerBackend,
        ))))
    }

    fn temp_server_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "msc2-backups-route-{tag}-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("world")).unwrap();
        std::fs::write(dir.join("world/level.dat"), b"fake").unwrap();
        dir
    }

    fn java_server(id: &str, dir: &Path) -> ConfigServer {
        ConfigServer::new(
            id,
            "Backups Route Server",
            dir.to_string_lossy().to_string(),
            "",
            2.0,
            4.0,
        )
    }

    fn worlds_credential() -> AuthenticatedCredential {
        AuthenticatedCredential {
            credential_id: "named".to_string(),
            label: "console".to_string(),
            role: CredentialRole::Named,
            permissions: vec![PermissionCategoryDto::Worlds],
        }
    }

    async fn json_body<T: serde::de::DeserializeOwned>(response: Response) -> T {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn wait_for_terminal(
        lifecycle: &LifecycleRoutesState,
        operation_id: &str,
    ) -> msc_api::dto::OperationDto {
        for _ in 0..200 {
            if let Some(record) = lifecycle.operations().snapshot(operation_id)
                && matches!(
                    record.state,
                    msc_api::dto::OperationStateDto::Succeeded
                        | msc_api::dto::OperationStateDto::Failed
                )
            {
                return record;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("operation {operation_id} did not reach a terminal state");
    }

    #[tokio::test]
    async fn world_backup_routes_restore_guard_order_and_capability_unavailable() {
        let lifecycle = LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );
        let server_dir = temp_server_dir("restore-guards");
        let server = java_server("java-1", &server_dir);
        lifecycle.merge_config_servers(vec![server]).unwrap();
        lifecycle
            .select_active_server("java-1".to_string())
            .unwrap();

        let worlds_state = WorldsRoutesState::new(lifecycle.clone());
        let credential = worlds_credential();
        let created = worlds::create(
            State(worlds_state),
            Extension(credential.clone()),
            Some(Json(WorldCreateRequestDto {
                name: "Survival".to_string(),
                seed: None,
            })),
        )
        .await;
        let _created: WorldMutationResultDto = json_body(created).await;

        let scheduler = test_backup_scheduler();
        let backups_state = BackupsRoutesState {
            lifecycle: lifecycle.clone(),
            scheduler,
        };
        let started = now(State(backups_state.clone()), Extension(credential.clone())).await;
        let started: BackupNowResultDto = json_body(started).await;
        let record = wait_for_terminal(&lifecycle, &started.operation_id.unwrap()).await;
        assert_eq!(record.state, msc_api::dto::OperationStateDto::Succeeded);

        let listed = list(State(backups_state.clone())).await;
        let listed: BackupsResponseDto = json_body(listed).await;
        assert_eq!(listed.backups.len(), 1);
        let backup_id = listed.backups[0].id.clone();

        // Missing backup -> 404 backup_not_found.
        let missing = restore(
            State(backups_state.clone()),
            Extension(credential.clone()),
            Some(Json(BackupRestoreRequestDto {
                backup_id: "does-not-exist.zip".to_string(),
            })),
        )
        .await;
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);

        // A real backup exists and the server is Java + stopped -- this
        // restore should be admitted (guards all pass) and complete.
        let started = restore(
            State(backups_state.clone()),
            Extension(credential.clone()),
            Some(Json(BackupRestoreRequestDto {
                backup_id: backup_id.clone(),
            })),
        )
        .await;
        assert_eq!(started.status(), StatusCode::OK);
        let started: BackupRestoreResultDto = json_body(started).await;
        let record = wait_for_terminal(&lifecycle, &started.operation_id.unwrap()).await;
        assert_eq!(
            record.state,
            msc_api::dto::OperationStateDto::Succeeded,
            "{:?}",
            record.error
        );
    }

    // NOTE: a dedicated "restore against a Bedrock server is
    // capability_unavailable" integration test was attempted here but
    // dropped -- `LifecycleRoutesState::select_active_server` routes
    // through `LifecycleService::select_active_server`, which calls
    // `JavaServerRepository::load` (`AgentServerRegistry`); its
    // `config_server_to_lifecycle_server` returns `None` for any
    // non-Java `ConfigServer`, so a Bedrock server can never become the
    // active server in this agent at all today. That's consistent with
    // "no live Bedrock runtime before Phase 10," but it means
    // `restore`'s `server.server_type == ServerType::Bedrock` guard
    // (below) is currently unreachable through the full route path, not
    // merely rare -- flagged in the P6.21 report rather than papered
    // over with a test that can't actually exercise the route.

    #[tokio::test]
    async fn world_backup_routes_config_update_reconfigures_scheduler() {
        let lifecycle = LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );
        let server_dir = temp_server_dir("config-update");
        let server = java_server("java-config", &server_dir);
        lifecycle.merge_config_servers(vec![server]).unwrap();
        lifecycle
            .select_active_server("java-config".to_string())
            .unwrap();

        let scheduler = test_backup_scheduler();
        let backups_state = BackupsRoutesState {
            lifecycle: lifecycle.clone(),
            scheduler,
        };
        let response = update_config(
            State(backups_state.clone()),
            Extension(AuthenticatedCredential {
                credential_id: "named".to_string(),
                label: "console".to_string(),
                role: CredentialRole::Named,
                permissions: vec![PermissionCategoryDto::Settings],
            }),
            Some(Json(BackupConfigUpdateRequestDto {
                auto_backup_enabled: Some(true),
                auto_backup_interval_minutes: Some(60),
                auto_backup_max_count: None,
            })),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let result: BackupConfigUpdateResultDto = json_body(response).await;
        assert!(result.success);
        let config = result.config.unwrap();
        assert!(config.auto_backup_enabled);
        assert_eq!(config.auto_backup_interval_minutes, 60);

        // The persisted config actually changed -- the scheduler
        // reconfigure call is fire-and-forget (no assertion hook on the
        // fake backend), but the underlying `ConfigServer` update is the
        // half this test can directly verify.
        let updated = lifecycle
            .app_config_servers()
            .into_iter()
            .find(|s| s.id == "java-config")
            .unwrap();
        assert!(updated.auto_backup_enabled);
        assert_eq!(updated.auto_backup_interval_minutes, 60);
    }
}
