//! Console output framing and bounded history for lifecycle-owned processes.
//!
//! MSC 1's `ServerProcessManager` receives arbitrary bytes from a merged
//! stdout/stderr pipe, emits only complete newline-terminated lines, and
//! flushes one trailing partial line when the process closes. `RemoteAPIServer`
//! then stores a single bounded console buffer that backs both HTTP tail and
//! WebSocket backfill.

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub const CONSOLE_HISTORY_LIMIT: usize = 5000;
pub const CONSOLE_WEBSOCKET_BACKFILL: usize = 200;
pub const CONSOLE_HTTP_TAIL_DEFAULT: usize = 200;
pub const CONSOLE_HTTP_TAIL_MIN: usize = 1;
pub const CONSOLE_HTTP_TAIL_MAX: usize = 2000;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleLine {
    pub ts: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    pub text: String,
}

impl ConsoleLine {
    pub fn new(source: impl Into<String>, level: Option<String>, text: impl Into<String>) -> Self {
        Self {
            ts: now_timestamp_millis(),
            source: source.into(),
            level,
            text: text.into(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ConsoleLineFramer {
    pending: Vec<u8>,
}

impl ConsoleLineFramer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) -> Vec<String> {
        self.pending.extend_from_slice(bytes);
        let mut lines = Vec::new();

        while let Some(newline_index) = self.pending.iter().position(|byte| *byte == b'\n') {
            let line = self.pending.drain(..newline_index).collect::<Vec<_>>();
            self.pending.drain(..1);
            lines.push(decode_line(line));
        }

        lines
    }

    pub fn flush(&mut self) -> Option<String> {
        if self.pending.is_empty() {
            return None;
        }
        Some(decode_line(std::mem::take(&mut self.pending)))
    }
}

#[derive(Debug, Default)]
pub struct ConsoleBuffer {
    lines: VecDeque<ConsoleLine>,
}

impl ConsoleBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, line: ConsoleLine) {
        self.lines.push_back(line);
        while self.lines.len() > CONSOLE_HISTORY_LIMIT {
            self.lines.pop_front();
        }
    }

    pub fn tail(&self, requested: usize) -> Vec<ConsoleLine> {
        let count = requested.clamp(CONSOLE_HTTP_TAIL_MIN, CONSOLE_HTTP_TAIL_MAX);
        self.tail_unclamped(count)
    }

    pub fn websocket_backfill(&self) -> Vec<ConsoleLine> {
        self.tail_unclamped(CONSOLE_WEBSOCKET_BACKFILL)
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn oldest(&self) -> Option<&ConsoleLine> {
        self.lines.front()
    }

    fn tail_unclamped(&self, count: usize) -> Vec<ConsoleLine> {
        let skip = self.lines.len().saturating_sub(count);
        self.lines.iter().skip(skip).cloned().collect()
    }
}

pub fn http_tail_count(raw: Option<&str>) -> usize {
    raw.and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(CONSOLE_HTTP_TAIL_DEFAULT)
        .clamp(CONSOLE_HTTP_TAIL_MIN, CONSOLE_HTTP_TAIL_MAX)
}

fn decode_line(mut bytes: Vec<u8>) -> String {
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn now_timestamp_millis() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the Unix epoch")
        .as_millis()
        .to_string()
}
