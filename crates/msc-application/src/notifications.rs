//! Agent-owned notification events.
//!
//! Clients decide whether and how to show these events as native OS
//! notifications.  The feed therefore contains safe status text only; it is
//! never a place to put helper credentials or management addresses.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const NOTIFICATION_HISTORY_LIMIT: usize = 200;

static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    ServerStarted,
    ServerStopped,
    PlayerJoined,
    PlayerLeft,
    HelperFailed,
    ConnectivityChanged,
}

impl NotificationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ServerStarted => "server_started",
            Self::ServerStopped => "server_stopped",
            Self::PlayerJoined => "player_joined",
            Self::PlayerLeft => "player_left",
            Self::HelperFailed => "helper_failed",
            Self::ConnectivityChanged => "connectivity_changed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationEvent {
    pub id: String,
    pub server_id: String,
    pub occurred_at_iso8601: String,
    pub kind: NotificationKind,
    pub title: String,
    pub body: String,
    pub help_id: Option<String>,
}

#[derive(Debug, Default)]
pub struct NotificationService {
    events: VecDeque<NotificationEvent>,
}

impl NotificationService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn emit_server_started(&mut self, server_id: &str, server_name: &str) -> NotificationEvent {
        self.emit(
            server_id,
            NotificationKind::ServerStarted,
            "Server Started",
            format!("{server_name} is now online."),
            None,
        )
    }

    pub fn emit_server_stopped(&mut self, server_id: &str, server_name: &str) -> NotificationEvent {
        self.emit(
            server_id,
            NotificationKind::ServerStopped,
            "Server Stopped",
            format!("{server_name} has stopped."),
            None,
        )
    }

    pub fn emit_player_joined(
        &mut self,
        server_id: &str,
        server_name: &str,
        player_name: &str,
    ) -> NotificationEvent {
        self.emit(
            server_id,
            NotificationKind::PlayerJoined,
            "Player Joined",
            format!("{player_name} joined {server_name}"),
            None,
        )
    }

    pub fn emit_player_left(
        &mut self,
        server_id: &str,
        server_name: &str,
        player_name: &str,
    ) -> NotificationEvent {
        self.emit(
            server_id,
            NotificationKind::PlayerLeft,
            "Player Left",
            format!("{player_name} left {server_name}"),
            None,
        )
    }

    pub fn emit_helper_failed(&mut self, server_id: &str, helper_name: &str) -> NotificationEvent {
        self.emit(
            server_id,
            NotificationKind::HelperFailed,
            "Helper Failed",
            format!("{helper_name} stopped unexpectedly."),
            Some("helpers.failed".into()),
        )
    }

    pub fn emit_connectivity_changed(
        &mut self,
        server_id: &str,
        detail: &str,
    ) -> NotificationEvent {
        self.emit(
            server_id,
            NotificationKind::ConnectivityChanged,
            "Connectivity Changed",
            detail.to_string(),
            Some("connectivity.changed".into()),
        )
    }

    pub fn recent(&self) -> impl Iterator<Item = &NotificationEvent> {
        self.events.iter()
    }

    fn emit(
        &mut self,
        server_id: &str,
        kind: NotificationKind,
        title: &str,
        body: String,
        help_id: Option<String>,
    ) -> NotificationEvent {
        let event = NotificationEvent {
            id: format!(
                "notification-{}",
                NEXT_EVENT_ID.fetch_add(1, Ordering::Relaxed)
            ),
            server_id: server_id.to_string(),
            occurred_at_iso8601: iso8601_now(),
            kind,
            title: title.to_string(),
            body,
            help_id,
        };
        self.events.push_back(event.clone());
        while self.events.len() > NOTIFICATION_HISTORY_LIMIT {
            self.events.pop_front();
        }
        event
    }
}

fn iso8601_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        day_seconds / 3_600,
        (day_seconds % 3_600) / 60,
        day_seconds % 60
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}
