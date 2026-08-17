//! `POST /v1/operations`, `GET /v1/operations/{id}`, `POST
//! /v1/operations/{id}/cancel` — `operation-model.md` §4's three routes,
//! backed by Phase 4's application operation coordinator. Lifecycle
//! operations are journaled before real server mutation begins, and
//! snapshots fall back to the durable journal after an agent restart.
//!
//! The one `type` this skeletal agent accepts, `demo-install`
//! (`operation-model.md` §2), is driven by a background "ticker" task that
//! advances a freshly-created operation `queued → running → succeeded`
//! over a couple of seconds. **Cancellation is cooperative and truthful
//! (P6.40):** `POST .../cancel` only signals a flag
//! (`OperationsState::request_cancel`) and immediately returns `202`
//! while the worker is still stopping. Every real Phase 6 worker (`worlds::activate_slot`/
//! `world_conversion::convert_world`/`backups::create_backup`/
//! `backups::restore_backup`) now follows — to observe it at its own
//! safe boundary and finalize the transition itself
//! (`OperationsState::cancel`). The record stays `running` (and its
//! per-target exclusivity held) for as long as real work is still in
//! flight; a request that arrives after the work already finished
//! reports that real outcome, never a fabricated `cancelled`.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{ErrorDto, OperationDto, OperationProgressDto, OperationStateDto};
use msc_application::operations::{
    LifecycleOperationError, LifecycleOperationSnapshot, LifecycleOperations, lifecycle_error,
};
use msc_domain::operation::{OperationId, OperationState};
#[cfg(test)]
use msc_infrastructure::fs::FakeFileSystem;
use msc_infrastructure::fs::{FileSystem, StdFileSystem};
use serde::Deserialize;
use std::path::PathBuf;

/// Shared `msc-agent` operation store, injected into the three handlers
/// below via axum's `State` extractor.
#[derive(Clone)]
pub struct OperationsState {
    operations: Arc<LifecycleOperations<'static>>,
}

impl OperationsState {
    pub fn new(fs: &'static dyn FileSystem, dir: impl Into<PathBuf>) -> Self {
        let operations = LifecycleOperations::new(fs, dir);
        let _ = operations.reconcile_on_startup();
        Self {
            operations: Arc::new(operations),
        }
    }

    pub fn default_journaled() -> Self {
        let dir = operation_journal_dir();
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|error| panic!("failed to create {}: {error}", dir.display()));
        let fs = Box::leak(Box::new(StdFileSystem));
        Self::new(fs, dir)
    }

    #[cfg(test)]
    pub fn fake_journaled() -> Self {
        let fs = Box::leak(Box::new(FakeFileSystem::new().with_file(
            "/srv/agent/operations/.keep",
            Vec::new(),
            false,
        )));
        Self::new(fs, "/srv/agent/operations")
    }

    pub fn begin_lifecycle(
        &self,
        operation_type: &str,
        target: Option<String>,
        status_line: &str,
    ) -> Result<OperationId, LifecycleOperationError> {
        self.operations
            .begin_running(operation_type, target, status_line)
    }

    pub fn progress(
        &self,
        id: &OperationId,
        current: u64,
        total: u64,
        status_line: &str,
    ) -> Result<(), LifecycleOperationError> {
        self.operations
            .set_progress(id, current, total, status_line)
    }

    pub fn succeed(
        &self,
        id: &OperationId,
        status_line: &str,
        result: BTreeMap<String, String>,
    ) -> Result<(), LifecycleOperationError> {
        self.operations.succeed(id, status_line, result)
    }

    pub fn fail(
        &self,
        id: &OperationId,
        code: &str,
        message: String,
    ) -> Result<(), LifecycleOperationError> {
        self.operations.fail(id, lifecycle_error(code, message))
    }

    /// Finalizes a worker's own cooperative cancellation once it has
    /// actually observed [`Self::cancellation_check`]'s flag and stopped
    /// — see `LifecycleOperations::cancel`'s own doc for why only the
    /// worker itself, never the `POST .../cancel` handler, calls this.
    pub fn cancel(
        &self,
        id: &OperationId,
        status_line: &str,
    ) -> Result<(), LifecycleOperationError> {
        self.operations.cancel(id, status_line)
    }

    /// Signals cooperative cancellation for a non-terminal operation
    /// without transitioning its state — see
    /// `LifecycleOperations::request_cancel`'s own doc.
    pub fn request_cancel(
        &self,
        id: &OperationId,
        status_line: &str,
    ) -> Result<(), LifecycleOperationError> {
        self.operations.request_cancel(id, status_line)
    }

    /// A cheap, `'static` closure a real Phase 6 worker can poll at its
    /// own safe boundaries — see `LifecycleOperations::cancellation_check`'s
    /// own doc.
    pub fn cancellation_check(
        &self,
        id: &OperationId,
    ) -> impl Fn() -> bool + Clone + Send + Sync + 'static {
        self.operations.cancellation_check(id)
    }

    /// Current `OperationDTO` for `id`, or `None` if unknown. Used by the
    /// operation-progress WebSocket handler (P2.16) to existence-check
    /// before upgrading and to poll for changes afterward, without
    /// exposing the record type or the lock itself outside this module.
    pub fn snapshot(&self, id: &str) -> Option<OperationDto> {
        self.operations
            .snapshot(&OperationId::new(id.to_string()))
            .ok()
            .flatten()
            .map(to_dto)
    }
}

impl Default for OperationsState {
    fn default() -> Self {
        Self::default_journaled()
    }
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

    let id = match store.begin_lifecycle("demo-install", request.target, "Queued demo work") {
        Ok(id) => id,
        Err(error) => return operation_error_response(error),
    };
    let dto = store
        .snapshot(id.as_str())
        .expect("freshly-created operation is present");
    spawn_demo_ticker(store, id.clone());

    (StatusCode::ACCEPTED, Json(dto)).into_response()
}

/// `GET /v1/operations/{id}` — §4.2.
pub async fn get(State(store): State<OperationsState>, Path(id): Path<String>) -> Response {
    match store.snapshot(&id) {
        Some(record) => (StatusCode::OK, Json(record)).into_response(),
        None => not_found(&id),
    }
}

/// `POST /v1/operations/{id}/cancel` — §4.3. Legal only against a
/// non-terminal operation; a terminal one is `409 conflict`, per the same
/// transition table `msc_domain::operation::OperationState` enforces.
///
/// **Truthful cancellation (P6.40).** This handler never transitions the
/// operation to `cancelled` itself — it only signals the request
/// ([`OperationsState::request_cancel`]) and then re-reads the record
/// once. The worker that owns the operation must observe that signal at one of its
/// own safe boundaries and stop, exactly the way `spawn_demo_ticker`'s
/// own loop below and every real Phase 6 worker
/// (`worlds::activate_slot`/`world_conversion::convert_world`/
/// `backups::create_backup`/`backups::restore_backup`) do. Reporting
/// `cancelled` before that has happened would be a lie: the real
/// filesystem/process work could still be running, and the per-target
/// exclusivity lock (still held by the non-terminal journal entry) would
/// have been released early. If the worker has already finalized
/// cancellation by the one re-read, the response is `200`; otherwise it
/// is `202 Accepted` with the honest current record and the client keeps
/// polling `GET /v1/operations/{id}` or the operation stream.
pub async fn cancel(State(store): State<OperationsState>, Path(id): Path<String>) -> Response {
    let id = OperationId::new(id);
    let Some(record) = store.snapshot(id.as_str()) else {
        return not_found(id.as_str());
    };

    if matches!(
        record.state,
        OperationStateDto::Succeeded | OperationStateDto::Failed | OperationStateDto::Cancelled
    ) {
        return conflict(id.as_str());
    }

    if let Err(error) = store.request_cancel(&id, "Cancelling…") {
        return operation_error_response(error);
    }

    let snapshot = store
        .snapshot(id.as_str())
        .expect("operation ids are never removed once created");
    let status = if snapshot.state == OperationStateDto::Cancelled {
        StatusCode::OK
    } else {
        StatusCode::ACCEPTED
    };
    (status, Json(snapshot)).into_response()
}

/// Advances a freshly-created operation `queued → running → succeeded`
/// over ~1.5s, polling [`OperationsState::cancellation_check`]'s flag
/// before every write — the same cooperative shape every real Phase 6
/// worker uses (see `cancel`'s own doc): a `cancel` request only sets the
/// flag, so this ticker (not the HTTP handler) is what actually
/// transitions the record to `cancelled`, once it has itself stopped
/// doing further "work."
fn spawn_demo_ticker(store: OperationsState, id: OperationId) {
    let should_cancel = store.cancellation_check(&id);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        if should_cancel() {
            let _ = store.cancel(&id, "Cancelled by user.");
            return;
        }
        let _ = store.progress(&id, 0, 3, "Starting demo work");

        for step in 1..=3u64 {
            tokio::time::sleep(Duration::from_millis(300)).await;
            if should_cancel() {
                let _ = store.cancel(&id, "Cancelled by user.");
                return;
            }
            let _ = store.progress(&id, step, 3, &format!("Demo step {step}/3"));
        }

        tokio::time::sleep(Duration::from_millis(300)).await;
        if should_cancel() {
            let _ = store.cancel(&id, "Cancelled by user.");
            return;
        }
        let mut result = BTreeMap::new();
        result.insert("demo".to_string(), "true".to_string());
        let _ = store.succeed(&id, "Demo work complete", result);
    });
}

fn to_dto(record: LifecycleOperationSnapshot) -> OperationDto {
    OperationDto {
        id: record.id.as_str().to_string(),
        r#type: record.operation_type,
        target: record.target,
        state: to_state_dto(record.state),
        progress: record.progress.map(|progress| OperationProgressDto {
            current: progress.current,
            total: progress.total,
        }),
        status_line: record.status_line,
        result: record.result.map(|result| serde_json::json!(result)),
        error: record.error.map(error_to_dto),
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

fn error_to_dto(error: msc_domain::operation::OperationError) -> ErrorDto {
    ErrorDto {
        code: error.code,
        message: error.message,
        help_id: error.help_id,
        details: if error.details.is_empty() {
            None
        } else {
            Some(serde_json::json!(error.details))
        },
    }
}

pub fn operation_error_response(error: LifecycleOperationError) -> Response {
    match error {
        LifecycleOperationError::Conflict(error) => {
            (StatusCode::CONFLICT, Json(error_to_dto(error))).into_response()
        }
        LifecycleOperationError::UnknownOperation(id) => not_found(id.as_str()),
        LifecycleOperationError::IllegalTransition { id, .. } => conflict(id.as_str()),
        LifecycleOperationError::Journal(message) => {
            let body = ErrorDto {
                code: "internal_error".to_string(),
                message,
                help_id: None,
                details: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
        }
    }
}

fn operation_journal_dir() -> PathBuf {
    std::env::var_os("MSC2_OPERATION_JOURNAL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("msc2-operation-journal"))
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

// =====================================================================
// P6.30 tests — inline, not in `tests/operation_cancellation.rs`, for the
// same "no lib.rs" reason `routes/worlds.rs`/`routes/backups.rs`'s own
// test-module docs already give: an external integration test can't
// reach `OperationsState::request_cancel`/`cancellation_check` or the
// `demo-install` ticker directly. `tests/operation_cancellation.rs`
// itself only proves the real HTTP wiring (mounted + auth-gated);
// the truthful-cancellation *behavior* is proven here, against the real
// handlers and the real `spawn_demo_ticker`, with no HTTP/auth layer in
// the way.
// =====================================================================
#[cfg(test)]
mod tests {
    use super::*;

    fn body(json: serde_json::Value) -> Bytes {
        Bytes::from(json.to_string())
    }

    async fn json_body<T: serde::de::DeserializeOwned>(response: Response) -> T {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// `cancel` returns `202` while the worker still owns the operation,
    /// then normal polling observes the worker's truthful terminal
    /// transition.
    #[tokio::test]
    async fn operation_cancellation_cancel_returns_accepted_then_worker_stops() {
        let store = OperationsState::fake_journaled();
        let created = create(
            State(store.clone()),
            body(serde_json::json!({ "type": "demo-install" })),
        )
        .await;
        assert_eq!(created.status(), StatusCode::ACCEPTED);
        let created: OperationDto = json_body(created).await;

        let accepted = cancel(State(store.clone()), Path(created.id.clone())).await;
        assert_eq!(accepted.status(), StatusCode::ACCEPTED);
        let accepted: OperationDto = json_body(accepted).await;
        assert_eq!(accepted.state, OperationStateDto::Running);
        assert_eq!(accepted.status_line.as_deref(), Some("Cancelling…"));

        let mut reached_cancelled = false;
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if store
                .snapshot(&created.id)
                .is_some_and(|record| record.state == OperationStateDto::Cancelled)
            {
                reached_cancelled = true;
                break;
            }
        }
        assert!(reached_cancelled, "worker never finalized cancellation");

        let fetched = get(State(store), Path(created.id)).await;
        let fetched: OperationDto = json_body(fetched).await;
        assert_eq!(fetched.state, OperationStateDto::Cancelled);
    }

    /// While a cancellation is pending (signaled but not yet observed by
    /// the worker), the operation's target stays exclusively held — a
    /// second mutation against the same target is refused exactly as it
    /// would be for any other non-terminal operation. Uses
    /// `request_cancel` directly so the assertion lands in the genuinely
    /// pending window without racing the demo worker.
    #[tokio::test]
    async fn operation_cancellation_target_stays_held_while_cancellation_pending() {
        let store = OperationsState::fake_journaled();
        let id = store
            .begin_lifecycle("world-activate", Some("paper-1".to_string()), "Working.")
            .unwrap();

        store.request_cancel(&id, "Cancelling…").unwrap();
        let snapshot = store.snapshot(id.as_str()).unwrap();
        assert_eq!(snapshot.state, OperationStateDto::Running);

        let conflict =
            store.begin_lifecycle("world-activate", Some("paper-1".to_string()), "Working.");
        assert!(matches!(
            conflict,
            Err(LifecycleOperationError::Conflict(_))
        ));
    }

    /// A `cancel` that arrives after the real work already finished must
    /// report the truth (`409`, already terminal) rather than a
    /// fabricated `cancelled` — the same rule that keeps a *pending*
    /// cancellation from lying in the other direction.
    #[tokio::test]
    async fn operation_cancellation_cancel_after_natural_completion_is_conflict_not_a_lie() {
        let store = OperationsState::fake_journaled();
        let created = create(
            State(store.clone()),
            body(serde_json::json!({ "type": "demo-install" })),
        )
        .await;
        let created: OperationDto = json_body(created).await;

        // The demo ticker finishes in ~1.5s; wait well past that.
        let mut succeeded = false;
        for _ in 0..100 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if let Some(record) = store.snapshot(&created.id)
                && record.state == OperationStateDto::Succeeded
            {
                succeeded = true;
                break;
            }
        }
        assert!(succeeded, "demo-install operation never reached succeeded");

        let cancelled = cancel(State(store), Path(created.id)).await;
        assert_eq!(cancelled.status(), StatusCode::CONFLICT);
    }
}
