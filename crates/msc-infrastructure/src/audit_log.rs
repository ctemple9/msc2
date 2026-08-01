//! `AuditLog`: JSONL audit trail for Remote API mutations and auth
//! failures, one file per UTC day.
//!
//! Ported from `AuditLogger.swift` (139 lines): a dedicated serial writer
//! so concurrent callers never interleave two entries' bytes together,
//! manual JSON-line construction rather than `JSONEncoder` ("hot-path
//! constraint", `AuditLogger.swift:106`), and 30-day retention
//! (`pruneOldFiles`, lines 74-90). No MSC 1 test file exercises this
//! component directly — `RemoteAPIAuditLogTests` in
//! `RemoteAPIIntegrationTests.swift` only checks that an entry is
//! captured via a test observer, already covered by
//! `fixtures/remote-api-integration/` — so these fixtures were
//! characterized straight from the source file instead.
//!
//! Unlike `AuditLogger.swift`, this primitive does not create its own
//! output directory before writing — the same "caller ensures the parent
//! exists" rule [`crate::atomic_write`] and [`crate::config_repository`]
//! already established for this phase, applied here rather than adding a
//! third convention (and the `FileSystem` trait this crate builds on has
//! no directory-creation method to call).

use crate::fs::FileSystem;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RETENTION_DAYS: i64 = 30;

#[derive(Debug, Clone)]
pub struct Entry {
    pub timestamp: SystemTime,
    pub client_ip: String,
    pub token_label: String,
    pub method: String,
    pub path: String,
    pub status_code: u16,
}

/// One writer lock per instance: `log` and `prune_old_files` both take
/// it, so a prune sweep never races a write and two calls to `log`
/// (however they're invoked — including from separate threads) never
/// interleave their bytes into the same line. This is the same guarantee
/// `AuditLogger`'s serial `DispatchQueue` gives, without needing an
/// actual background thread to get it.
pub struct AuditLog<'fs> {
    fs: &'fs dyn FileSystem,
    dir: PathBuf,
    lock: Mutex<()>,
}

impl<'fs> AuditLog<'fs> {
    pub fn new(fs: &'fs dyn FileSystem, dir: impl Into<PathBuf>) -> Self {
        Self {
            fs,
            dir: dir.into(),
            lock: Mutex::new(()),
        }
    }

    /// Appends `entry` as one JSONL line to the file for its UTC day.
    /// Never parses the file it's appending to — an existing corrupt or
    /// partial trailing line is left exactly as it was, matching
    /// `AuditLogger.write`'s own `FileHandle.seekToEndOfFile` plus raw
    /// `write(data)` (lines 114-121): neither reads nor repairs what's
    /// already there.
    pub fn log(&self, entry: &Entry) -> io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        let path = self.file_path_for(entry.timestamp);
        let mut bytes = self.fs.read(&path).unwrap_or_default();
        bytes.extend_from_slice(format_line(entry).as_bytes());
        self.fs.write(&path, &bytes)
    }

    /// Removes every `audit-<date>.jsonl` file under `dir` more than
    /// [`RETENTION_DAYS`] days older than `now`'s UTC date; a file
    /// exactly `RETENTION_DAYS` days old is kept. Mirrors `pruneOldFiles`
    /// (lines 74-90), with one deliberate change: MSC 1 compares each
    /// file's real filesystem creation timestamp against a fractional-day
    /// cutoff, but [`FileSystem`] (`crate::fs`) exposes no creation time.
    /// Since [`Self::file_path_for`] is the only thing that ever names an
    /// audit file, the date already in its filename carries the same
    /// fact a creation timestamp would — so pruning reads it from there,
    /// in whole UTC days, instead.
    pub fn prune_old_files(&self, now: SystemTime) -> io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        let today = days_since_epoch(now);
        for path in self.fs.list(&self.dir)? {
            let Some(file_day) = audit_file_day(&path) else {
                continue;
            };
            if today - file_day > RETENTION_DAYS {
                self.fs.remove(&path)?;
            }
        }
        Ok(())
    }

    fn file_path_for(&self, timestamp: SystemTime) -> PathBuf {
        self.dir
            .join(format!("audit-{}.jsonl", date_string_utc(timestamp)))
    }
}

fn format_line(entry: &Entry) -> String {
    format!(
        "{{\"ts\":\"{}\",\"ip\":\"{}\",\"token\":\"{}\",\"method\":\"{}\",\"path\":\"{}\",\"status\":{}}}\n",
        escape(&format_iso8601(entry.timestamp)),
        escape(&entry.client_ip),
        escape(&entry.token_label),
        escape(&entry.method),
        escape(&entry.path),
        entry.status_code,
    )
}

/// Mirrors `AuditLogger.esc` (lines 133-138) exactly: only these four
/// characters are escaped, in this order — matching what MSC 1 actually
/// guards against, not a general-purpose JSON-string escaper.
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// `audit-<date>.jsonl`'s embedded date, or `None` for anything that
/// doesn't match the pattern — mirrors `pruneOldFiles`'s own filter
/// (`pathExtension == "jsonl" && hasPrefix("audit-")`, lines 83-84), so a
/// stray file left by something else in the directory is never touched.
fn audit_file_day(path: &Path) -> Option<i64> {
    let name = path.file_name()?.to_str()?;
    let date = name.strip_prefix("audit-")?.strip_suffix(".jsonl")?;
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn date_string_utc(timestamp: SystemTime) -> String {
    let (year, month, day) = civil_from_days(days_since_epoch(timestamp));
    format!("{year:04}-{month:02}-{day:02}")
}

/// Matches `ISO8601DateFormatter`'s `.withFractionalSeconds` output shape
/// (milliseconds, not nanoseconds) — `AuditLogger.isoFormatter`, lines
/// 31-35.
fn format_iso8601(timestamp: SystemTime) -> String {
    let duration = timestamp
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    let total_secs = duration.as_secs() as i64;
    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;
    let millis = duration.subsec_millis();
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

fn days_since_epoch(timestamp: SystemTime) -> i64 {
    let duration = timestamp
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    (duration.as_secs() as i64).div_euclid(86_400)
}

/// Howard Hinnant's `days_from_civil`/`civil_from_days`
/// (public domain, <http://howardhinnant.github.io/date_algorithms.html>)
/// — proleptic-Gregorian day arithmetic, days counted from the Unix
/// epoch. Written out here rather than pulled from a date/time crate:
/// every other Phase 0-3 primitive in this workspace stays
/// dependency-free beyond `serde_json`, and this is the first place date
/// arithmetic (as opposed to an opaque `SystemTime`) is actually needed.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (i64::from(month) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}
