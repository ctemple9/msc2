//! `GET /v1/console/stream` — P0.24's one real MSC 1 WebSocket channel,
//! carried forward at a versioned path per `websocket-v1.json` (P2.7):
//! same bearer auth as every HTTP route (the existing middleware layer
//! already runs before this handler, since the route sits inside
//! `main.rs`'s auth-gated router), the 200-line-backfill-then-live
//! delivery model, a 5000-line ring buffer, and a 64 KB inbound-frame cap.
//!
//! With no real server process yet, the buffer is seeded with a canned,
//! honestly-labeled boot log and grows from a demo ticker instead of real
//! process output — this is what makes the bounded-history-then-live
//! behavior observable end-to-end this phase, per this step's own brief.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use serde::Serialize;
use tokio::sync::broadcast;

/// `tailConsoleLines(n: 200)` in the P0.24 baseline.
const BACKFILL_COUNT: usize = 200;
/// `consoleBufferLimit` in the P0.24 baseline.
const RING_BUFFER_LIMIT: usize = 5000;
/// axum's inbound frame-size guard, matching `maxWebSocketClientFrameBytes`.
const MAX_INBOUND_FRAME_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsoleLineDto {
    ts: String,
    source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    level: Option<String>,
    text: String,
}

/// Shared agent-lifetime console buffer plus a broadcast channel fanning
/// new lines out to every currently-connected client — one buffer for the
/// whole agent, not one per connection, matching MSC 1's single
/// `consoleBuffer`.
#[derive(Clone)]
pub struct ConsoleState {
    buffer: Arc<Mutex<VecDeque<ConsoleLineDto>>>,
    sender: broadcast::Sender<ConsoleLineDto>,
}

impl Default for ConsoleState {
    fn default() -> Self {
        let (sender, _receiver) = broadcast::channel(RING_BUFFER_LIMIT);
        let state = Self {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            sender,
        };
        state.seed_canned_boot_log();
        state.spawn_demo_ticker();
        state
    }
}

impl ConsoleState {
    fn push(&self, line: ConsoleLineDto) {
        let mut buffer = self.buffer.lock().expect("console buffer lock poisoned");
        buffer.push_back(line.clone());
        while buffer.len() > RING_BUFFER_LIMIT {
            buffer.pop_front();
        }
        drop(buffer);
        // No connected clients is the normal case; a send error just means
        // nobody's listening right now.
        let _ = self.sender.send(line);
    }

    fn backfill(&self) -> Vec<ConsoleLineDto> {
        let buffer = self.buffer.lock().expect("console buffer lock poisoned");
        let skip = buffer.len().saturating_sub(BACKFILL_COUNT);
        buffer.iter().skip(skip).cloned().collect()
    }

    fn seed_canned_boot_log(&self) {
        const LINES: &[&str] = &[
            "Starting minecraft server version 1.21.1",
            "Loading properties",
            "Default game type: SURVIVAL",
            "Generating keypair",
            "Starting Minecraft server on *:25565",
            "Preparing level \"world\"",
            "Preparing start region for dimension minecraft:overworld",
            "Time elapsed: 4028 ms",
            "Done (8.412s)! For help, type \"help\"",
            "[msc-agent] Placeholder console output — msc-agent's skeletal \
             P2.15 handler, not a real server process.",
        ];
        for text in LINES {
            self.push(ConsoleLineDto {
                ts: now_ts(),
                source: "server".to_string(),
                level: Some("info".to_string()),
                text: (*text).to_string(),
            });
        }
    }

    /// Stands in for real process output: appends one synthetic line a
    /// second for as long as the agent runs, so a client watching the live
    /// tail after backfill actually sees something arrive.
    fn spawn_demo_ticker(&self) {
        let state = self.clone();
        tokio::spawn(async move {
            let mut tick: u64 = 0;
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                tick += 1;
                state.push(ConsoleLineDto {
                    ts: now_ts(),
                    source: "demo".to_string(),
                    level: Some("info".to_string()),
                    text: format!("[demo] heartbeat tick {tick}"),
                });
            }
        });
    }
}

fn now_ts() -> String {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch");
    since_epoch.as_millis().to_string()
}

pub async fn upgrade(ws: WebSocketUpgrade, State(state): State<ConsoleState>) -> Response {
    ws.max_frame_size(MAX_INBOUND_FRAME_BYTES)
        .on_upgrade(move |socket| handle_socket(socket, state))
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

async fn send_line(socket: &mut WebSocket, line: &ConsoleLineDto) -> Result<(), axum::Error> {
    let text = serde_json::to_string(line).expect("ConsoleLineDto always serializes");
    socket.send(Message::Text(text)).await
}
