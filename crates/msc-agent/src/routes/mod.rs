//! Route handlers for `msc-agent`'s `/v1/` endpoints.

pub mod backups;
pub mod bedrock;
pub mod bedrock_runtime;
pub mod browser_session;
pub mod capabilities;
pub mod commands;
pub mod components;
pub mod desktop_session;
pub mod geyser;
pub mod health;
pub mod help;
pub mod lifecycle;
pub mod network_diagnostics;
pub mod networking;
pub mod operations;
pub mod performance;
pub mod players;
pub mod servers;
pub mod settings;
pub mod status;
pub mod templates;
pub mod users;
pub mod versions;
pub mod worlds;

pub(crate) fn system_time_to_iso8601(epoch_secs: u64) -> Option<String> {
    let days = epoch_secs / 86_400;
    let remainder = epoch_secs % 86_400;
    let (hour, minute, second) = (remainder / 3600, (remainder % 3600) / 60, remainder % 60);
    let (year, month, day) = civil_from_days(days as i64);
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

/// Howard Hinnant's `civil_from_days` converts Unix-epoch days to a
/// proleptic Gregorian date without adding a date/time dependency.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
