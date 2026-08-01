//! `POST /v1/operations`, `GET /v1/operations/{id}`, `POST
//! /v1/operations/{id}/cancel` — `operation-model.md` §4's three routes,
//! backed by an in-memory (non-journaled) map of id → operation record.
//! Restart survival is Phase 3's operation journal, not this step's job
//! (`operation-model.md` §6).
//!
//! The one `type` this skeletal agent accepts, `demo-install`
//! (`operation-model.md` §2), is driven by a background "ticker" task that
//! advances a freshly-created operation `queued → running → succeeded`
//! over a couple of seconds, checking for external cancellation between
//! each step rather than blindly overwriting it.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{ErrorDto, OperationDto, OperationProgressDto, OperationStateDto};
use msc_domain::operation::OperationState;
use serde::Deserialize;

/// Shared `msc-agent` operation store, injected into the three handlers
/// below via axum's `State` extractor.
#[derive(Clone, Default)]
pub struct OperationsState(Arc<Mutex<HashMap<String, OperationRecord>>>);

struct OperationRecord {
    r#type: String,
    target: Option<String>,
    state: OperationState,
    progress: Option<(u64, u64)>,
    status_line: Option<String>,
    result: Option<serde_json::Value>,
    error: Option<ErrorDto>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Opaque and server-generated, per `operation-model.md` §2 — the format
/// is deliberately unremarkable rather than ULID-shaped, since clients are
/// never meant to parse it.
fn next_operation_id() -> String {
    format!("op-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed))
}

#[derive(Deserialize)]
pub struct CreateOperationRequest {
    r#type: String,
    #[serde(default)]
    target: Option<String>,
}

/// `POST /v1/operations` — §4.1. Only `demo-install` is a recognized
/// `type` this phase; anything else is `400 invalid_body`.
///
/// Takes the raw body and parses it with `serde_json` directly rather than
/// axum's `Json` extractor, which rejects a request that doesn't declare
/// `Content-Type: application/json` — a header this phase's own Verify
/// command (a bare `curl -d`) doesn't set. A skeletal dev-loop agent has no
/// reason to be pickier about that header than its own test harness.
pub async fn create(State(store): State<OperationsState>, body: Bytes) -> Response {
    let request: CreateOperationRequest = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(_) => return invalid_body("<unparseable request body>"),
    };
    if request.r#type != "demo-install" {
        return invalid_body(&request.r#type);
    }

    let id = next_operation_id();
    let record = OperationRecord {
        r#type: request.r#type,
        target: request.target,
        state: OperationState::Queued,
        progress: None,
        status_line: None,
        result: None,
        error: None,
    };
    let dto = to_dto(&id, &record);

    store
        .0
        .lock()
        .expect("operations lock poisoned")
        .insert(id.clone(), record);
    spawn_demo_ticker(store, id);

    (StatusCode::ACCEPTED, Json(dto)).into_response()
}

/// `GET /v1/operations/{id}` — §4.2.
pub async fn get(State(store): State<OperationsState>, Path(id): Path<String>) -> Response {
    let operations = store.0.lock().expect("operations lock poisoned");
    match operations.get(&id) {
        Some(record) => (StatusCode::OK, Json(to_dto(&id, record))).into_response(),
        None => not_found(&id),
    }
}

/// `POST /v1/operations/{id}/cancel` — §4.3. Legal only against a
/// non-terminal operation; a terminal one is `409 conflict`, per the same
/// transition table `msc_domain::operation::OperationState` enforces.
pub async fn cancel(State(store): State<OperationsState>, Path(id): Path<String>) -> Response {
    let mut operations = store.0.lock().expect("operations lock poisoned");
    let Some(record) = operations.get_mut(&id) else {
        return not_found(&id);
    };
    match record.state.transition_to(OperationState::Cancelled) {
        Ok(state) => {
            record.state = state;
            record.status_line = Some("Cancelled by user".to_string());
            (StatusCode::OK, Json(to_dto(&id, record))).into_response()
        }
        Err(_) => conflict(&id),
    }
}

/// Advances a freshly-created operation `queued → running → succeeded`
/// over ~1.5s, re-checking the record's state before every write so a
/// `cancel` request that lands mid-run is respected rather than clobbered.
fn spawn_demo_ticker(store: OperationsState, id: String) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        {
            let mut operations = store.0.lock().expect("operations lock poisoned");
            let Some(record) = operations.get_mut(&id) else {
                return;
            };
            if record.state != OperationState::Queued {
                return; // cancelled before it started
            }
            record.state = record
                .state
                .transition_to(OperationState::Running)
                .expect("queued->running is a legal transition");
            record.status_line = Some("Starting demo work".to_string());
        }

        for step in 1..=3u64 {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let mut operations = store.0.lock().expect("operations lock poisoned");
            let Some(record) = operations.get_mut(&id) else {
                return;
            };
            if record.state != OperationState::Running {
                return; // cancelled mid-flight
            }
            record.progress = Some((step, 3));
            record.status_line = Some(format!("Demo step {step}/3"));
        }

        tokio::time::sleep(Duration::from_millis(300)).await;
        let mut operations = store.0.lock().expect("operations lock poisoned");
        let Some(record) = operations.get_mut(&id) else {
            return;
        };
        if record.state != OperationState::Running {
            return; // cancelled mid-flight
        }
        record.state = record
            .state
            .transition_to(OperationState::Succeeded)
            .expect("running->succeeded is a legal transition");
        record.status_line = Some("Demo work complete".to_string());
        record.result = Some(serde_json::json!({ "demo": true }));
    });
}

fn to_dto(id: &str, record: &OperationRecord) -> OperationDto {
    OperationDto {
        id: id.to_string(),
        r#type: record.r#type.clone(),
        target: record.target.clone(),
        state: to_state_dto(record.state),
        progress: record
            .progress
            .map(|(current, total)| OperationProgressDto { current, total }),
        status_line: record.status_line.clone(),
        result: record.result.clone(),
        error: record.error.clone(),
    }
}

fn to_state_dto(state: OperationState) -> OperationStateDto {
    match state {
        OperationState::Queued => OperationStateDto::Queued,
        OperationState::Running => OperationStateDto::Running,
        OperationState::Succeeded => OperationStateDto::Succeeded,
        OperationState::Failed => OperationStateDto::Failed,
        OperationState::Cancelled => OperationStateDto::Cancelled,
    }
}

fn invalid_body(type_value: &str) -> Response {
    let body = ErrorDto {
        code: "invalid_body".to_string(),
        message: format!("Unknown operation type '{type_value}'."),
        help_id: None,
        details: None,
    };
    (StatusCode::BAD_REQUEST, Json(body)).into_response()
}

fn not_found(id: &str) -> Response {
    let body = ErrorDto {
        code: "not_found".to_string(),
        message: format!("No operation with id '{id}' exists."),
        help_id: Some("operations.not-found".to_string()),
        details: None,
    };
    (StatusCode::NOT_FOUND, Json(body)).into_response()
}

fn conflict(id: &str) -> Response {
    let body = ErrorDto {
        code: "conflict".to_string(),
        message: format!("Operation '{id}' is already finished; cancellation is not legal."),
        help_id: Some("operations.cancel-not-legal".to_string()),
        details: None,
    };
    (StatusCode::CONFLICT, Json(body)).into_response()
}
