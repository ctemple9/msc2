//! `POST /v1/host/reset`: the authenticated, host-local destructive reset.
//!
//! The response is admitted before the destructive work starts. The worker
//! then advances the shared operation journal through filesystem cleanup,
//! in-memory cleanup, credential revocation, host-ID rotation, and marker
//! removal. A remote caller can therefore lose its old credential without
//! the route pretending that the reset is still reversible.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::Json;
use axum::extract::{Extension, State, rejection::JsonRejection};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{HostResetAcceptedDto, HostResetRequestDto, PermissionCategoryDto};
use msc_application::host_reset::{HostResetMode, HostResetWorkflow};

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
    in_progress: Arc<AtomicBool>,
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
            in_progress: Arc::new(AtomicBool::new(false)),
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
    if state.lifecycle.status_snapshot().running {
        return error_response(
            StatusCode::CONFLICT,
            "server_running",
            "Stop the running server before resetting this host.",
        );
    }
    if state
        .in_progress
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return error_response(
            StatusCode::CONFLICT,
            "reset_in_progress",
            "A host reset is already in progress.",
        );
    }

    let operation_id =
        match state
            .operations
            .begin_lifecycle("host-reset", None, "Preparing host reset.")
        {
            Ok(id) => id,
            Err(error) => {
                state.in_progress.store(false, Ordering::Release);
                return operation_error_response(error);
            }
        };
    let workflow = match HostResetWorkflow::new(
        &msc_infrastructure::fs::StdFileSystem,
        state.lifecycle.app_config_path(),
        state.lifecycle.servers_root(),
    )
    .and_then(|workflow| {
        workflow.with_helper_cache(
            msc_infrastructure::config_repository::default_app_data_dir().join("helpers"),
        )
    }) {
        Ok(workflow) => workflow,
        Err(error) => {
            let _ = state
                .operations
                .fail(&operation_id, "invalid_reset_target", error.to_string());
            state.in_progress.store(false, Ordering::Release);
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &error.to_string(),
            );
        }
    };
    if let Err(error) = workflow.begin(mode) {
        let _ = state
            .operations
            .fail(&operation_id, "reset_prepare_failed", error.to_string());
        state.in_progress.store(false, Ordering::Release);
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        );
    }

    let previous_server_ids = state
        .lifecycle
        .app_config_servers()
        .into_iter()
        .map(|server| server.id)
        .collect::<Vec<_>>();
    let worker_state = state.clone();
    let returned_host_id = host_id.clone();
    let response_operation_id = operation_id.clone();
    tokio::spawn(async move {
        let result = run_reset(
            &worker_state,
            workflow,
            operation_id.clone(),
            mode,
            previous_server_ids,
        )
        .await;
        if let Err((code, message)) = result {
            let _ = worker_state.operations.fail(&operation_id, &code, message);
        }
        worker_state.in_progress.store(false, Ordering::Release);
    });

    (
        StatusCode::ACCEPTED,
        Json(HostResetAcceptedDto {
            operation_id: response_operation_id.as_str().to_string(),
            host_id: returned_host_id,
            mode: request.mode,
            agent_state: "needs_pairing".to_string(),
            message: "Host reset accepted. Pair this host again after the agent is available."
                .to_string(),
        }),
    )
        .into_response()
}

async fn run_reset(
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
            "Stopping managed helpers and clearing in-memory host state.",
        )
        .map_err(operation_failure)?;
    // Playit owns a host-local secret bridge inside a server directory. Stop
    // and reconcile it while those approved paths still exist, before the
    // destructive part of the host reset removes them.
    state.lifecycle.reset_after_host_reset();
    state
        .operations
        .progress(&operation_id, 2, 4, "Removing approved host files.")
        .map_err(operation_failure)?;
    workflow
        .apply_files(mode)
        .map_err(|error| ("reset_files_failed".to_string(), error.to_string()))?;
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
