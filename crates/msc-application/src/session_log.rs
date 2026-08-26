//! Durable Java player session history.
//!
//! MSC 1 stores the selected server's join/leave history in
//! `{serverDir}/session_log.json`.  The application layer keeps the same
//! JSON shape and uses the shared atomic writer so a console event never
//! leaves a partially-written history file behind.

use std::fmt;
use std::io;
use std::path::Path;

use msc_infrastructure::atomic_write::atomic_write;
use msc_infrastructure::fs::FileSystem;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

const SESSION_LOG_FILE: &str = "session_log.json";

/// A single player join or leave event, persisted to `session_log.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEvent {
    #[serde(with = "uuid_string")]
    pub id: Uuid,
    pub player_name: String,
    pub event_type: SessionEventType,
    /// ISO8601, supplied by the lifecycle caller so this module stays
    /// deterministic and does not read the clock itself.
    pub timestamp: String,
}

/// The two session transitions emitted by the Java console parser.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionEventType {
    Joined,
    Left,
}

mod uuid_string {
    use super::*;

    pub fn serialize<S>(id: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&id.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Uuid::parse_str(&raw).map_err(D::Error::custom)
    }
}

#[derive(Debug)]
pub enum SessionLogError {
    Io(io::Error),
}

impl fmt::Display for SessionLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SessionLogError {}

impl From<io::Error> for SessionLogError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn log_path(server_dir: &Path) -> std::path::PathBuf {
    server_dir.join(SESSION_LOG_FILE)
}

/// Loads all persisted events. Missing or malformed files become an empty
/// history, matching MSC 1's silent-failure behavior.
pub fn load_events(fs: &dyn FileSystem, server_dir: &Path) -> Vec<SessionEvent> {
    let Ok(bytes) = fs.read(&log_path(server_dir)) else {
        return Vec::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

/// Appends one event, atomically persists the complete history, and returns
/// that updated history on success.
pub fn append_event(
    fs: &dyn FileSystem,
    server_dir: &Path,
    player_name: &str,
    event_type: SessionEventType,
    timestamp: String,
) -> Result<Vec<SessionEvent>, SessionLogError> {
    let mut events = load_events(fs, server_dir);
    events.push(SessionEvent {
        id: Uuid::new_v4(),
        player_name: player_name.to_string(),
        event_type,
        timestamp,
    });
    let bytes = serde_json::to_vec_pretty(&events).map_err(io::Error::other)?;
    atomic_write(fs, &log_path(server_dir), &bytes)
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(events)
}

/// Removes the history file. A missing file is already clear and is not an
/// error, matching MSC 1's file-exists guard.
pub fn clear_events(fs: &dyn FileSystem, server_dir: &Path) -> Result<(), SessionLogError> {
    let path = log_path(server_dir);
    if fs.stat(&path).is_err() {
        return Ok(());
    }
    match fs.remove(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
