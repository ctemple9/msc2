//! Minecraft world-time rules shared by every runtime and client surface.
//!
//! Minecraft stores world time as ticks. One in-game day is 24,000 ticks,
//! while the familiar dawn/dusk/night presets are positions within that day.
//! Keeping the arithmetic here prevents a client from accidentally turning a
//! time-of-day shortcut into an absolute reset to day zero.

use crate::identity::ServerType;

pub const MINECRAFT_DAY_TICKS: i64 = 24_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativeTimePreset {
    Dawn,
    Dusk,
    Night,
}

impl RelativeTimePreset {
    pub fn from_raw_value(raw: &str) -> Option<Self> {
        match raw {
            "dawn" => Some(Self::Dawn),
            "dusk" => Some(Self::Dusk),
            "night" => Some(Self::Night),
            _ => None,
        }
    }

    pub const fn raw_value(self) -> &'static str {
        match self {
            Self::Dawn => "dawn",
            Self::Dusk => "dusk",
            Self::Night => "night",
        }
    }

    pub const fn target_daytime_ticks(self) -> i64 {
        match self {
            Self::Dawn => 1_000,
            Self::Dusk => 13_000,
            Self::Night => 18_000,
        }
    }

    pub const fn all_raw_values() -> [&'static str; 3] {
        ["dawn", "dusk", "night"]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelativeTimeTarget {
    pub preset: RelativeTimePreset,
    pub current_absolute_ticks: i64,
    pub current_day: i64,
    pub current_daytime_ticks: i64,
    pub target_day: i64,
    pub target_daytime_ticks: i64,
    pub target_absolute_ticks: i64,
}

impl RelativeTimeTarget {
    /// Builds a same-day target from the runtime's daylight-cycle day and
    /// daytime values. `gametime` is deliberately not accepted here: it is
    /// total server uptime and can diverge from the world's current day when
    /// the daylight cycle has been changed or paused.
    pub fn for_current_day(
        preset: RelativeTimePreset,
        current_day: i64,
        current_daytime_ticks: i64,
    ) -> Self {
        let current_daytime_ticks = current_daytime_ticks.rem_euclid(MINECRAFT_DAY_TICKS);
        let current_absolute_ticks = current_day * MINECRAFT_DAY_TICKS + current_daytime_ticks;
        let target_daytime_ticks = preset.target_daytime_ticks();
        let target_absolute_ticks = current_day * MINECRAFT_DAY_TICKS + target_daytime_ticks;
        Self {
            preset,
            current_absolute_ticks,
            current_day,
            current_daytime_ticks,
            target_day: current_day,
            target_daytime_ticks,
            target_absolute_ticks,
        }
    }

    /// Returns the runtime command, keeping the runtime distinction at the
    /// agent boundary even though Java and Bedrock currently share syntax.
    pub fn absolute_command(self, server_type: ServerType) -> String {
        match server_type {
            ServerType::Java | ServerType::Bedrock => {
                format!("time set {}", self.target_absolute_ticks)
            }
        }
    }
}

/// Extracts the integer from a Java or Bedrock `time query` response.
/// Prefixes cover legacy servers, modern Paper wording, and the 1.21.4+
/// timeline wording. The caller decides whether the value represents the
/// daylight-cycle day or the daytime because both commands use the same
/// response shape on older runtimes.
pub fn parse_time_query_response(line: &str) -> Option<i64> {
    [
        "The game time is ",
        "Timeline minecraft:day is at ",
        "Timeline minecraft:daytime is at ",
        "Timeline minecraft:gametime is at ",
        "The time is ",
    ]
    .into_iter()
    .find_map(|prefix| {
        let tail = line
            .split_once(prefix)
            .map(|(_, tail)| tail)
            .or_else(|| split_bedrock_time_response(line, prefix))?
            .trim_start();
        let number = tail.strip_prefix('-').unwrap_or(tail);
        let digits = number
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>();
        if digits.is_empty() {
            return None;
        }
        let sign = if tail.starts_with('-') { -1 } else { 1 };
        digits.parse::<i64>().ok().map(|value| sign * value)
    })
}

fn split_bedrock_time_response<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let bedrock_prefix = match prefix {
        "The time is " => "daytime is ",
        _ => return None,
    };
    let lower = line.to_ascii_lowercase();
    let index = lower.find(bedrock_prefix)?;
    let value_start = index + bedrock_prefix.len();
    Some(&line[value_start..])
}

/// Parses the daylight-cycle day query into a day number across runtime
/// response formats. Legacy servers return the day number directly, while
/// modern Paper's timeline response reports the absolute day timeline in
/// ticks.
pub fn parse_day_query_response(line: &str) -> Option<i64> {
    if let Some(tail) = split_bedrock_day_response(line)
        && let Some(value) = parse_integer_prefix(tail)
    {
        return Some(value);
    }
    let value = parse_time_query_response(line)?;
    if line.contains("Timeline minecraft:day is at ") {
        Some(value.div_euclid(MINECRAFT_DAY_TICKS))
    } else {
        Some(value)
    }
}

fn split_bedrock_day_response(line: &str) -> Option<&str> {
    let lower = line.to_ascii_lowercase();
    let index = lower.find("day is ")?;
    let value_start = index + "day is ".len();
    Some(&line[value_start..])
}

fn parse_integer_prefix(tail: &str) -> Option<i64> {
    let tail = tail.trim_start();
    let number = tail.strip_prefix('-').unwrap_or(tail);
    let digits = number
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    let sign = if tail.starts_with('-') { -1 } else { 1 };
    digits.parse::<i64>().ok().map(|value| sign * value)
}
