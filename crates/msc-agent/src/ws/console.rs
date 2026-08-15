//! `GET /v1/console/stream` — P0.24's one real MSC 1 WebSocket channel,
//! carried forward at a versioned path per `websocket-v1.json` (P2.7):
//! same bearer auth as every HTTP route (the existing middleware layer
//! already runs before this handler, since the route sits inside
//! `main.rs`'s auth-gated router), the 200-line-backfill-then-live
//! delivery model, a 5000-line ring buffer, and a 64 KB inbound-frame cap.

use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Json;
use axum::response::Response;
use msc_infrastructure::console_buffer::{
    CONSOLE_HISTORY_LIMIT, ConsoleBuffer, ConsoleLine, http_tail_count,
};
use serde::Deserialize;
use tokio::sync::broadcast;

/// axum's inbound frame-size guard, matching `maxWebSocketClientFrameBytes`.
const MAX_INBOUND_FRAME_BYTES: usize = 64 * 1024;

#[derive(Debug, Deserialize)]
pub struct TailQuery {
    n: Option<String>,
}

/// Shared agent-lifetime console buffer plus a broadcast channel fanning
/// new lines out to every currently-connected client — one buffer for the
/// whole agent, not one per connection, matching MSC 1's single
/// `consoleBuffer`.
#[derive(Clone)]
pub struct ConsoleState {
    buffer: Arc<Mutex<ConsoleBuffer>>,
    sender: broadcast::Sender<ConsoleLine>,
}

impl Default for ConsoleState {
    fn default() -> Self {
        let (sender, _receiver) = broadcast::channel(CONSOLE_HISTORY_LIMIT);
        Self {
            buffer: Arc::new(Mutex::new(ConsoleBuffer::new())),
            sender,
        }
    }
}

impl ConsoleState {
    pub fn push(&self, line: ConsoleLine) {
        let mut buffer = self.buffer.lock().expect("console buffer lock poisoned");
        buffer.push(line.clone());
        drop(buffer);
        // No connected clients is the normal case; a send error just means
        // nobody's listening right now.
        let _ = self.sender.send(line);
    }

    fn backfill(&self) -> Vec<ConsoleLine> {
        let buffer = self.buffer.lock().expect("console buffer lock poisoned");
        buffer.websocket_backfill()
    }

    fn tail(&self, raw_n: Option<&str>) -> Vec<ConsoleLine> {
        let count = http_tail_count(raw_n);
        self.buffer
            .lock()
            .expect("console buffer lock poisoned")
            .tail(count)
    }

    /// The most recent `count` console lines — P6.21's `LiveBackupConsole`
    /// needs read access to whatever the save-pause protocol's console
    /// commands (`save-all`, `save query`, ...) just printed, the same
    /// buffer `GET /v1/console/tail` already exposes over HTTP.
    pub fn recent_lines(&self, count: usize) -> Vec<ConsoleLine> {
        self.buffer
            .lock()
            .expect("console buffer lock poisoned")
            .tail(count)
    }
}

pub async fn upgrade(ws: WebSocketUpgrade, State(state): State<ConsoleState>) -> Response {
    ws.max_frame_size(MAX_INBOUND_FRAME_BYTES)
        .on_upgrade(move |socket| handle_socket(socket, state))
}

pub async fn tail(
    State(state): State<ConsoleState>,
    Query(query): Query<TailQuery>,
) -> Json<Vec<ConsoleLine>> {
    Json(state.tail(query.n.as_deref()))
}

async fn handle_socket(mut socket: WebSocket, state: ConsoleState) {
    // Subscribed before reading backfill so a line pushed mid-backfill is
    // queued for live delivery rather than silently missed.
    let mut live = state.sender.subscribe();

    for line in state.backfill() {
        if send_line(&mut socket, &line).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                match incoming {
                    // Inbound text/binary is intentionally ignored — this
                    // channel is server-to-client only (websocket-v1.json).
                    // Ping/pong/close are handled by axum's WebSocket type
                    // itself; a closed/errored connection ends the loop.
                    Some(Ok(_)) => continue,
                    Some(Err(_)) | None => return,
                }
            }
            line = live.recv() => {
                match line {
                    Ok(line) => {
                        if send_line(&mut socket, &line).await.is_err() {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        }
    }
}

async fn send_line(socket: &mut WebSocket, line: &ConsoleLine) -> Result<(), axum::Error> {
    let text = serde_json::to_string(line).expect("ConsoleLine always serializes");
    socket.send(Message::Text(text)).await
}
