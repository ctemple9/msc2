//! `POST /v1/host/reset`: the authenticated, host-local destructive reset.
//!
//! The response is admitted before the destructive work starts. The worker
//! then advances the shared operation journal through filesystem cleanup,
//! in-memory cleanup, credential revocation, host-ID rotation, and marker
//! removal. A remote caller can therefore lose its old credential without
//! the route pretending that the reset is still reversible.
//! The host-wide journal reservation is acquired before reset preconditions
//! and stays active until the worker stops. The durable reset marker keeps
//! ordinary management mutations blocked while failed recovery is pending.

use axum::Json;
use axum::extract::{Extension, State, rejection::JsonRejection};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{
    HostResetAcceptedDto, HostResetCompletionDto, HostResetRequestDto, HostResetStatusDto,
    PermissionCategoryDto,
};
use msc_application::host_reset::{HostResetMode, HostResetWorkflow};
use msc_application::operations::HOST_RESET_OPERATION_TYPE;

use crate::auth::{AuthState, AuthenticatedCredential, DesktopPairingError};
use crate::routes::lifecycle::{
    LifecycleRoutesState, error_response, invalid_body, require_permission,
};
use crate::routes::operations::{OperationsState, operation_error_response};

const RESET_CONFIRMATION: &str = "RESET AGENT";

#[derive(Clone)]
pub struct HostResetRoutesState {
    pub lifecycle: LifecycleRoutesState,
    pub auth: AuthState,
    pub operations: OperationsState,
}

impl HostResetRoutesState {
    pub fn new(
        lifecycle: LifecycleRoutesState,
        auth: AuthState,
        operations: OperationsState,
    ) -> Self {
        Self {
            lifecycle,
            auth,
            operations,
        }
    }
}

pub async fn reset(
    State(state): State<HostResetRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<HostResetRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Admin) {
        return response;
    }
    let Json(request) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_body", "Request body must be valid JSON."),
    };
    let mode = match request.mode.as_str() {
        "configuration" => HostResetMode::Configuration,
        "everything" => HostResetMode::Everything,
        _ => return invalid_body("invalid_mode", "The host reset mode is not recognized."),
    };
    let host_id = match state.auth.agent_host_id() {
        Ok(host_id) => host_id,
        Err(DesktopPairingError::Store(message)) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &message,
            );
        }
        Err(_) => unreachable!("reading an agent host id only has store failures"),
    };
    if request.confirmation != RESET_CONFIRMATION {
        return error_response(
            StatusCode::BAD_REQUEST,
            "confirmation_mismatch",
            "Confirmation must exactly match RESET AGENT.",
        );
    }
    let operation_id = match state.operations.begin_lifecycle(
        HOST_RESET_OPERATION_TYPE,
        None,
        "Preparing host reset.",
    ) {
        Ok(id) => id,
        Err(error) => return operation_error_response(error),
    };
    if state.lifecycle.status_snapshot().running {
        if let Err(error) = state.operations.fail(
            &operation_id,
            "server_running",
            "Stop the running server before resetting this host.".to_string(),
        ) {
            return operation_error_response(error);
        }
        return error_response(
            StatusCode::CONFLICT,
            "server_running",
            "Stop the running server before resetting this host.",
        );
    }
    let workflow = match HostResetWorkflow::pending(
        &msc_infrastructure::fs::StdFileSystem,
        state.lifecycle.app_config_path(),
    )
    .and_then(|pending| match pending {
        Some(workflow) => Ok(workflow),
        None => HostResetWorkflow::new(
            &msc_infrastructure::fs::StdFileSystem,
            state.lifecycle.app_config_path(),
            state.lifecycle.servers_root(),
        )
        .and_then(|workflow| {
            workflow.with_helper_cache(
                msc_infrastructure::config_repository::default_app_data_dir().join("helpers"),
            )
        }),
    }) {
        Ok(workflow) => workflow,
        Err(error) => {
            let _ = state
                .operations
                .fail(&operation_id, "invalid_reset_target", error.to_string());
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &error.to_string(),
            );
        }
    };
    if let Err(error) = workflow.begin_for_operation(mode, Some(operation_id.as_str().to_string()))
    {
        let _ = state
            .operations
            .fail(&operation_id, "reset_prepare_failed", error.to_string());
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        );
    }

    let completion_token = workflow.completion_token().ok().flatten();
    let previous_server_ids = state
        .lifecycle
        .app_config_servers()
        .into_iter()
        .map(|server| server.id)
        .collect::<Vec<_>>();
    let worker_state = state.clone();
    let returned_host_id = host_id.clone();
    let response_operation_id = operation_id.clone();
    tokio::task::spawn_blocking(move || {
        let result = run_reset(
            &worker_state,
            workflow,
            operation_id.clone(),
            mode,
            previous_server_ids.clone(),
        );
        if let Err((code, message)) = result {
            if code == "reset_cancelled" {
                let _ = worker_state.operations.cancel(&operation_id, &message);
            } else {
                let _ = msc_application::host_reset::record_failure(
                    &msc_infrastructure::fs::StdFileSystem,
                    &worker_state.lifecycle.app_config_path(),
                    &message,
                );
                let _ = worker_state.operations.fail(&operation_id, &code, message);
            }
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(HostResetAcceptedDto {
            operation_id: response_operation_id.as_str().to_string(),
            completion_token,
            host_id: returned_host_id,
            mode: request.mode,
            agent_state: "needs_pairing".to_string(),
            message: "Host reset accepted. Pair this host again after the agent is available."
                .to_string(),
        }),
    )
        .into_response()
}

fn run_reset(
    state: &HostResetRoutesState,
    workflow: HostResetWorkflow<'static>,
    operation_id: msc_domain::operation::OperationId,
    mode: HostResetMode,
    previous_server_ids: Vec<String>,
) -> Result<(), (String, String)> {
    state
        .operations
        .progress(
            &operation_id,
            1,
            4,
            "Stopping managed helpers before file cleanup.",
        )
        .map_err(operation_failure)?;
    // Playit owns a host-local secret bridge inside a server directory. Stop
    // and reconcile it while those approved paths still exist, before the
    // destructive part of the host reset removes them.
    state.lifecycle.stop_all_playit_helpers();
    state
        .operations
        .progress(&operation_id, 2, 4, "Removing approved host files.")
        .map_err(operation_failure)?;
    workflow
        .apply_files_cancellable(mode, &state.operations.cancellation_check(&operation_id))
        .map_err(|error| {
            (
                if error == msc_application::host_reset::HostResetError::Cancelled {
                    "reset_cancelled"
                } else {
                    "reset_files_failed"
                }
                .to_string(),
                error.to_string(),
            )
        })?;
    state.lifecycle.reset_after_host_reset();
    state
        .operations
        .progress(&operation_id, 3, 4, "Cleared approved host files.")
        .map_err(operation_failure)?;
    state
        .auth
        .reset_for_host_reset(&previous_server_ids)
        .map_err(|error| ("reset_auth_failed".to_string(), error.to_string()))?;
    state
        .operations
        .progress(
            &operation_id,
            4,
            4,
            "Revoked credentials and rotated host identity.",
        )
        .map_err(operation_failure)?;
    workflow
        .finish()
        .map_err(|error| ("reset_finalize_failed".to_string(), error.to_string()))?;
    let mut result = std::collections::BTreeMap::new();
    result.insert("mode".to_string(), mode.as_str().to_string());
    result.insert("agentState".to_string(), "needs_pairing".to_string());
    state
        .operations
        .succeed(
            &operation_id,
            "Host reset complete; fresh pairing required.",
            result,
        )
        .map_err(operation_failure)
}

fn operation_failure(error: impl std::fmt::Display) -> (String, String) {
    ("reset_operation_failed".to_string(), error.to_string())
}

pub async fn status(
    State(state): State<HostResetRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Admin) {
        return response;
    }
    match msc_application::host_reset::status(
        &msc_infrastructure::fs::StdFileSystem,
        &state.lifecycle.app_config_path(),
    ) {
        Ok(status) => Json(HostResetStatusDto {
            state: status.state,
            mode: status.mode,
            message: status.message,
            can_cancel: status.can_cancel,
        })
        .into_response(),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "reset_status_failed",
            &error.to_string(),
        ),
    }
}

pub async fn cancel(
    State(state): State<HostResetRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Admin) {
        return response;
    }
    match msc_application::host_reset::cancel_pending(
        &msc_infrastructure::fs::StdFileSystem,
        &state.lifecycle.app_config_path(),
        None,
    ) {
        Ok(()) => status(State(state), Extension(credential)).await,
        Err(error) => error_response(
            StatusCode::CONFLICT,
            "reset_cancel_failed",
            &error.to_string(),
        ),
    }
}

pub async fn completion(axum::extract::Path(token): axum::extract::Path<String>) -> Response {
    match msc_application::host_reset::completion_state(
        &msc_infrastructure::fs::StdFileSystem,
        &msc_infrastructure::config_repository::default_app_config_path(),
        &token,
    ) {
        Some(state) => Json(HostResetCompletionDto { state }).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
