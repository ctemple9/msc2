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
    /// True when the agent recognizes this as routine automatic output.
    /// Kept optional on the wire so older clients can continue to read the stream.
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto: bool,
    pub text: String,
}

impl ConsoleLine {
    pub fn new(source: impl Into<String>, level: Option<String>, text: impl Into<String>) -> Self {
        Self {
            ts: now_timestamp_millis(),
            source: source.into(),
            level,
            auto: false,
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
    classifier: ConsoleAutoClassifier,
}

impl ConsoleBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, mut line: ConsoleLine) -> ConsoleLine {
        line.auto = line.auto || self.classifier.classify(&line);
        self.lines.push_back(line);
        while self.lines.len() > CONSOLE_HISTORY_LIMIT {
            self.lines.pop_front();
        }
        self.lines.back().expect("a line was just pushed").clone()
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

/// Classifies only output whose shape or producer identifies it as routine.
/// Repetition alone is deliberately not enough: a repeated warning can still
/// describe a live failure, and metric values change between polls.
#[derive(Debug, Default)]
struct ConsoleAutoClassifier {
    spark: SparkState,
}

#[derive(Debug, Default)]
enum SparkState {
    #[default]
    Idle,
    ExpectValues {
        guard: u8,
    },
    Body {
        guard: u8,
        cpu_values_remaining: u8,
    },
}

impl ConsoleAutoClassifier {
    fn classify(&mut self, line: &ConsoleLine) -> bool {
        let text = strip_ansi(&line.text);
        let lower = text.to_ascii_lowercase();

        if is_routine_helper_line(line.source.as_str(), &lower) {
            return true;
        }

        if lower.contains("tps from last 5s, 10s, 1m, 5m, 15m") {
            self.spark = SparkState::ExpectValues { guard: 12 };
            return true;
        }

        if !matches!(self.spark, SparkState::Idle) {
            return self.classify_spark_continuation(&text);
        }

        is_known_metric_line(&lower)
    }

    fn classify_spark_continuation(&mut self, text: &str) -> bool {
        let state = std::mem::take(&mut self.spark);
        match state {
            SparkState::Idle => false,
            SparkState::ExpectValues { guard } => {
                if !has_five_decimal_values(text) {
                    return false;
                }
                self.spark = SparkState::Body {
                    guard: guard.saturating_sub(1),
                    cpu_values_remaining: 0,
                };
                true
            }
            SparkState::Body {
                guard,
                cpu_values_remaining,
            } => {
                if guard == 0 {
                    return false;
                }
                let next_cpu_values = if text
                    .to_ascii_lowercase()
                    .contains("cpu usage from last 10s, 1m, 15m")
                {
                    2
                } else if cpu_values_remaining > 0 {
                    cpu_values_remaining - 1
                } else {
                    0
                };
                if cpu_values_remaining == 1 {
                    self.spark = SparkState::Idle;
                } else if next_cpu_values > 0 || guard > 1 {
                    self.spark = SparkState::Body {
                        guard: guard.saturating_sub(1),
                        cpu_values_remaining: next_cpu_values,
                    };
                }
                // The second CPU values line ends the bounded block. The current
                // line still belongs to it, but the next line must be reclassified.
                true
            }
        }
    }
}

fn is_known_metric_line(lower: &str) -> bool {
    (lower.contains("tps from last 1m, 5m, 15m"))
        || (lower.contains("mean tick time") && lower.contains("mean tps"))
        || (lower.contains("overall:") && lower.contains("tps (") && lower.contains("ms/tick"))
        || lower.contains("average time per tick")
        || lower.contains("target tick rate")
        || lower.contains("percentiles: p50")
        || (lower.contains("there are") && lower.contains("players online"))
        || lower.contains("[primary session] updated session")
}

fn is_routine_helper_line(source: &str, lower: &str) -> bool {
    if !matches!(source, "playit" | "xbox-broadcast") {
        return false;
    }

    // Stderr and actionable helper failures/prompts must remain visible even
    // when the user hides routine automatic output.
    [
        "stderr",
        "error",
        "warn",
        "failed",
        "failure",
        "unable",
        "invalid",
        "denied",
        "refused",
        "timeout",
        "timed out",
        "please",
        "visit ",
        "device code",
        "sign in",
        "authenticate",
        "password",
        "token",
    ]
    .iter()
    .all(|marker| !lower.contains(marker))
}

fn has_five_decimal_values(text: &str) -> bool {
    text.split(|character: char| !character.is_ascii_digit() && character != '.')
        .filter(|token| token.contains('.'))
        .filter(|token| token.parse::<f64>().is_ok())
        .count()
        >= 5
}

fn strip_ansi(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let characters = text.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < characters.len() {
        let starts_escape = characters[index] == '\u{1b}'
            || (characters[index] == '\u{fffd}' && characters.get(index + 1) == Some(&'['));
        if starts_escape {
            // Some terminals replace the non-printing ESC byte with U+FFFD
            // before the line reaches the UI, leaving visible fragments such
            // as "�[36;1m". Treat both forms as the same ANSI sequence.
            index += if characters[index] == '\u{fffd}' {
                2
            } else {
                1
            };
            if characters.get(index) == Some(&'[') {
                index += 1;
            }
            while let Some(character) = characters.get(index) {
                index += 1;
                if character.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        output.push(characters[index]);
        index += 1;
    }
    output
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
