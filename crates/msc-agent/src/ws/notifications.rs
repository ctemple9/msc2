//! `GET /v1/notifications/stream` — bounded notification history followed by
//! live agent events.  Native OS notification presentation remains a client
//! concern; this channel carries only safe, typed status text.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use msc_api::dto::NotificationEventDto;
use tokio::sync::broadcast;

#[allow(dead_code)]
const HISTORY_LIMIT: usize = 200;
const CHANNEL_CAPACITY: usize = 200;
const MAX_INBOUND_FRAME_BYTES: usize = 64 * 1024;

static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct NotificationState {
    history: Arc<Mutex<VecDeque<NotificationEventDto>>>,
    sender: broadcast::Sender<NotificationEventDto>,
}

impl Default for NotificationState {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            history: Arc::new(Mutex::new(VecDeque::new())),
            sender,
        }
    }
}

impl NotificationState {
    /// Called by the server/player/helper event producers when those feeds
    /// are connected to the notification service.
    #[allow(dead_code)]
    pub fn push(&self, event: NotificationEventDto) {
        let mut history = self
            .history
            .lock()
            .expect("notification history lock poisoned");
        history.push_back(event.clone());
        while history.len() > HISTORY_LIMIT {
            history.pop_front();
        }
        drop(history);
        let _ = self.sender.send(event);
    }

    /// Connects the lifecycle producer to the same agent-owned feed used by
    /// the TUI and other clients. The event is deliberately the frozen
    /// NotificationEventDto shape; no terminal-specific event is introduced.
    pub fn push_lifecycle(&self, server_id: &str, server_name: &str, started: bool) {
        self.push(NotificationEventDto {
            id: format!(
                "notification-{}",
                NEXT_EVENT_ID.fetch_add(1, Ordering::Relaxed)
            ),
            server_id: server_id.to_string(),
            occurred_at_iso8601: iso8601_now(),
            kind: if started {
                "server_started"
            } else {
                "server_stopped"
            }
            .to_string(),
            title: if started {
                "Server Started"
            } else {
                "Server Stopped"
            }
            .to_string(),
            body: if started {
                format!("{server_name} is now online.")
            } else {
                format!("{server_name} has stopped.")
            },
            help_id: None,
        });
    }

    fn backfill(&self) -> Vec<NotificationEventDto> {
        self.history
            .lock()
            .expect("notification history lock poisoned")
            .iter()
            .cloned()
            .collect()
    }
}

pub async fn upgrade(ws: WebSocketUpgrade, State(state): State<NotificationState>) -> Response {
    ws.max_frame_size(MAX_INBOUND_FRAME_BYTES)
        .on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: NotificationState) {
    let mut live = state.sender.subscribe();
    for event in state.backfill() {
        if send_event(&mut socket, &event).await.is_err() {
            return;
        }
    }
    loop {
        tokio::select! {
            incoming = socket.recv() => match incoming {
                Some(Ok(_)) => continue,
                Some(Err(_)) | None => return,
            },
            event = live.recv() => match event {
                Ok(event) => {
                    if send_event(&mut socket, &event).await.is_err() { return; }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    }
}

async fn send_event(
    socket: &mut WebSocket,
    event: &NotificationEventDto,
) -> Result<(), axum::Error> {
    socket
        .send(Message::Text(
            serde_json::to_string(event).expect("notification DTO serializes"),
        ))
        .await
}

fn iso8601_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    crate::routes::system_time_to_iso8601(seconds).unwrap_or_else(|| "1970-01-01T00:00:00Z".into())
}
