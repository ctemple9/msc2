//! `GET /v1/operations/{id}/stream` — `websocket-v1.json`'s
//! operation-progress channel: same bearer auth as every HTTP route
//! (evaluated before the upgrade, same as P2.15's console channel), a
//! single-frame snapshot of the operation's current state sent immediately
//! on connect (there is no history to replay — `websocket-v1.json` is
//! explicit that this channel holds only the current `OperationDTO`, not a
//! log), then a fresh frame every time the operation's state/progress/
//! statusLine changes, closing the connection right after the first frame
//! carrying a terminal state.
//!
//! Unknown `id` is `404` with the same `ErrorDTO` shape
//! `GET /v1/operations/{id}` uses, checked *before* the upgrade is
//! attempted, per spec.
//!
//! P2.14's operation store has no event/broadcast hook of its own — this
//! polls `OperationsState::snapshot` on a short interval and only sends a
//! frame when the DTO actually changed, which observes the demo ticker's
//! `queued → running → succeeded` progression end-to-end without adding a
//! bespoke notification channel to already-committed P2.14 code.

use std::time::Duration;

use axum::Json;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{ErrorDto, OperationDto, OperationStateDto};

use crate::routes::operations::OperationsState;

const POLL_INTERVAL: Duration = Duration::from_millis(150);
/// Same cap P2.15's console channel enforces.
const MAX_INBOUND_FRAME_BYTES: usize = 64 * 1024;

pub async fn upgrade(
    ws: WebSocketUpgrade,
    State(store): State<OperationsState>,
    Path(id): Path<String>,
) -> Response {
    let Some(initial) = store.snapshot(&id) else {
        return not_found(&id);
    };

    ws.max_frame_size(MAX_INBOUND_FRAME_BYTES)
        .on_upgrade(move |socket| handle_socket(socket, store, id, initial))
}

async fn handle_socket(
    mut socket: WebSocket,
    store: OperationsState,
    id: String,
    initial: OperationDto,
) {
    if send_dto(&mut socket, &initial).await.is_err() {
        return;
    }
    if is_terminal(initial.state) {
        let _ = socket.send(Message::Close(None)).await;
        return;
    }

    let mut last = initial;
    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    // Inbound frames are ignored — this channel is
                    // observation-only, same rule as console.
                    Some(Ok(_)) => continue,
                    Some(Err(_)) | None => return,
                }
            }
            _ = tokio::time::sleep(POLL_INTERVAL) => {
                let Some(current) = store.snapshot(&id) else {
                    // Forgotten (no journal this phase) — nothing further
                    // to report.
                    let _ = socket.send(Message::Close(None)).await;
                    return;
                };
                if current == last {
                    continue;
                }
                if send_dto(&mut socket, &current).await.is_err() {
                    return;
                }
                let terminal = is_terminal(current.state);
                last = current;
                if terminal {
                    let _ = socket.send(Message::Close(None)).await;
                    return;
                }
            }
        }
    }
}

fn is_terminal(state: OperationStateDto) -> bool {
    matches!(
        state,
        OperationStateDto::Succeeded | OperationStateDto::Failed | OperationStateDto::Cancelled
    )
}

async fn send_dto(socket: &mut WebSocket, dto: &OperationDto) -> Result<(), axum::Error> {
    let text = serde_json::to_string(dto).expect("OperationDto always serializes");
    socket.send(Message::Text(text)).await
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
