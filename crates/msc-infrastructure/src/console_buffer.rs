//! Console output framing and bounded history for lifecycle-owned processes.
//!
//! MSC 1's `ServerProcessManager` receives arbitrary bytes from a merged
//! stdout/stderr pipe, emits only complete newline-terminated lines, and
//! flushes one trailing partial line when the process closes. `RemoteAPIServer`
//! then stores bounded human history plus a separate diagnostic ring. HTTP
//! tail and WebSocket backfill read only the human-history ring; internal
//! waiters can still inspect the all-origin history boundary.

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub const CONSOLE_HISTORY_LIMIT: usize = 5000;
/// The public console history is intentionally larger than the optional
/// controller/helper diagnostic history. These are separate retention budgets:
/// noisy automation cannot evict the lines an operator is reading.
pub const CONSOLE_DIAGNOSTIC_HISTORY_LIMIT: usize = 200;
/// Internal waiters still need to observe every ingested line, including
/// controller responses that are not presented in the human console.
pub const CONSOLE_INTERNAL_HISTORY_LIMIT: usize = 5000;
pub const CONSOLE_WEBSOCKET_BACKFILL: usize = 200;
pub const CONSOLE_HTTP_TAIL_DEFAULT: usize = 200;
pub const CONSOLE_HTTP_TAIL_MIN: usize = 1;
pub const CONSOLE_HTTP_TAIL_MAX: usize = 2000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConsoleLineOrigin {
    User,
    #[default]
    Server,
    Controller,
    Helper,
}

impl ConsoleLineOrigin {
    /// User-entered commands and genuine server output belong in the bounded
    /// human history; controller and helper output uses separate retention.
    pub const fn belongs_in_human_history(self) -> bool {
        matches!(self, Self::User | Self::Server)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleLine {
    pub ts: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// True for output produced by an MSC controller or helper.
    /// Kept optional on the wire so older clients can continue to read the stream.
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto: bool,
    /// Provenance is additive on the wire. `server` is omitted so older
    /// clients continue to receive the historical line shape unchanged.
    #[serde(default, skip_serializing_if = "is_server_origin")]
    pub origin: ConsoleLineOrigin,
    pub text: String,
}

impl ConsoleLine {
    pub fn new(source: impl Into<String>, level: Option<String>, text: impl Into<String>) -> Self {
        Self {
            ts: now_timestamp_millis(),
            source: source.into(),
            level,
            auto: false,
            origin: ConsoleLineOrigin::Server,
            text: text.into(),
        }
    }

    pub fn with_origin(
        source: impl Into<String>,
        level: Option<String>,
        origin: ConsoleLineOrigin,
        text: impl Into<String>,
    ) -> Self {
        let mut line = Self::new(source, level, text);
        line.origin = origin;
        line.auto = origin.is_automatic();
        line
    }
}

impl ConsoleLineOrigin {
    pub const fn is_automatic(self) -> bool {
        matches!(self, Self::Controller | Self::Helper)
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
    /// Human/server output exposed through HTTP tail and WebSocket backfill.
    lines: VecDeque<ConsoleLine>,
    /// Controller/helper output retained for a future diagnostics surface.
    diagnostics: VecDeque<ConsoleLine>,
    /// All recent output for internal parsers and operation waiters. This is
    /// deliberately not the public delivery buffer.
    internal: VecDeque<ConsoleLine>,
}

impl ConsoleBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, mut line: ConsoleLine) -> ConsoleLine {
        classify_automatic_output(&mut line);
        line.auto = line.auto || line.origin.is_automatic();
        if line.auto
            && line.origin.belongs_in_human_history()
            && line.origin != ConsoleLineOrigin::User
        {
            line.origin = ConsoleLineOrigin::Controller;
        }
        self.internal.push_back(line.clone());
        while self.internal.len() > CONSOLE_INTERNAL_HISTORY_LIMIT {
            self.internal.pop_front();
        }

        if line.origin.belongs_in_human_history() {
            self.lines.push_back(line.clone());
            while self.lines.len() > CONSOLE_HISTORY_LIMIT {
                self.lines.pop_front();
            }
        } else {
            self.diagnostics.push_back(line.clone());
            while self.diagnostics.len() > CONSOLE_DIAGNOSTIC_HISTORY_LIMIT {
                self.diagnostics.pop_front();
            }
        }
        line
    }

    pub fn tail(&self, requested: usize) -> Vec<ConsoleLine> {
        let count = requested.clamp(CONSOLE_HTTP_TAIL_MIN, CONSOLE_HTTP_TAIL_MAX);
        self.tail_unclamped(count)
    }

    pub fn websocket_backfill(&self) -> Vec<ConsoleLine> {
        self.tail_unclamped(CONSOLE_WEBSOCKET_BACKFILL)
    }

    pub fn tail_with_diagnostics(&self, requested: usize) -> Vec<ConsoleLine> {
        let count = requested.clamp(CONSOLE_HTTP_TAIL_MIN, CONSOLE_HTTP_TAIL_MAX);
        self.with_diagnostics(count)
    }

    pub fn websocket_backfill_with_diagnostics(&self) -> Vec<ConsoleLine> {
        self.with_diagnostics(CONSOLE_WEBSOCKET_BACKFILL)
    }

    pub fn diagnostics_tail(&self, requested: usize) -> Vec<ConsoleLine> {
        let count = requested.clamp(1, CONSOLE_DIAGNOSTIC_HISTORY_LIMIT);
        let skip = self.diagnostics.len().saturating_sub(count);
        self.diagnostics.iter().skip(skip).cloned().collect()
    }

    pub fn internal_tail(&self, requested: usize) -> Vec<ConsoleLine> {
        let count = requested.clamp(1, CONSOLE_INTERNAL_HISTORY_LIMIT);
        let skip = self.internal.len().saturating_sub(count);
        self.internal.iter().skip(skip).cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn diagnostics_len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn oldest(&self) -> Option<&ConsoleLine> {
        self.lines.front()
    }

    fn tail_unclamped(&self, count: usize) -> Vec<ConsoleLine> {
        let skip = self.lines.len().saturating_sub(count);
        self.lines.iter().skip(skip).cloned().collect()
    }

    fn with_diagnostics(&self, count: usize) -> Vec<ConsoleLine> {
        let mut lines = self.tail_unclamped(count);
        lines.extend(self.diagnostics_tail(count));
        lines.sort_by(|left, right| {
            left.ts
                .parse::<u128>()
                .unwrap_or_default()
                .cmp(&right.ts.parse::<u128>().unwrap_or_default())
        });
        if lines.len() > count {
            lines.drain(..lines.len() - count);
        }
        lines
    }
}

fn classify_automatic_output(line: &mut ConsoleLine) {
    let lower = line.text.replace('\u{fffd}', "").to_ascii_lowercase();
    let source = line.source.to_ascii_lowercase();
    let xbox_broadcast = source.contains("xbox-broadcast");
    let playit = source.contains("playit");
    let spark_output = is_spark_metrics_output(&lower);

    if is_actionable_output(&lower) || is_user_action_output(&lower) {
        if line.origin.is_automatic() || line.auto || xbox_broadcast || playit || spark_output {
            if !line.origin.is_automatic() {
                line.origin = if xbox_broadcast || playit {
                    ConsoleLineOrigin::Helper
                } else {
                    ConsoleLineOrigin::Controller
                };
            }
            line.auto = true;
        }
        return;
    }

    let routine_status = lower.contains("[primary session] updated session!");
    let player_count = lower.contains("players online");
    if xbox_broadcast || playit || routine_status || player_count || spark_output {
        line.origin = if xbox_broadcast || playit {
            ConsoleLineOrigin::Helper
        } else {
            ConsoleLineOrigin::Controller
        };
        line.auto = true;
    }
}

fn is_actionable_output(lower: &str) -> bool {
    ["error", "warn", "warning", "failed", "failure", "exception"]
        .iter()
        .any(|marker| lower.contains(marker))
}

fn is_user_action_output(lower: &str) -> bool {
    [
        "please visit",
        "login page",
        "pairing code",
        "authorize",
        "authentication required",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn is_spark_metrics_output(lower: &str) -> bool {
    if lower.contains("[spark-worker-pool-") && lower.contains("/info]:") {
        return true;
    }

    let Some((_, report)) = lower.split_once("[spark/info]:") else {
        return lower.contains("tps from last 5s, 10s, 1m, 5m, 15m")
            || lower.contains("tick durations (min/med/95%ile/max ms)")
            || lower.contains("cpu usage from last 10s, 1m, 15m");
    };
    let report = report.replace("[⚡]", "");
    let report = report.trim();
    report.is_empty()
        || report.contains("tps from last 5s, 10s, 1m, 5m, 15m")
        || report.contains("tick durations (min/med/95%ile/max ms)")
        || report.contains("cpu usage from last 10s, 1m, 15m")
        || report.contains("%") && (report.contains("(system)") || report.contains("(process)"))
        || decimal_value_count(report) >= 5
}

fn decimal_value_count(text: &str) -> usize {
    text.split(|character: char| !character.is_ascii_digit() && character != '.')
        .filter(|token| token.contains('.') && token.parse::<f64>().is_ok())
        .count()
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

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_server_origin(value: &ConsoleLineOrigin) -> bool {
    *value == ConsoleLineOrigin::Server
}
