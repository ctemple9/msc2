//! Backup filename/sidecar rules: `AppViewModel+Backups.swift`'s pure,
//! I/O-free logic — filename token constants, trigger-reason derivation,
//! display-name parsing, and the `BackupMeta` sidecar's decode/encode.
//! Directory listing, zip creation, and process/console interaction stay
//! out of this crate, per the same module-boundary rule `world.rs`'s own
//! doc comment already states (`msc-infrastructure::backup_store` owns
//! the I/O half; `msc-application::backups` owns orchestration).
//!
//! `fixtures/backups/`'s filename/sidecar/config cases (P6.6) are this
//! module's characterization; `effective_backup_association` (the
//! "which slot does this backup belong to" rule) already lives in
//! [`crate::world`], ported ahead of this step since P6.12's import
//! needed the same active-slot lookup.

use crate::world::{DecodeError, insert_opt_str, opt_str, req_str};
use serde_json::{Map, Value};

/// `AppViewModel+Backups.swift:17` — encodes an automatic backup's origin
/// in its filename.
pub const AUTO_TOKEN: &str = "_auto_";
/// `AppViewModel+Backups.swift:18` — encodes a manual (or manually
/// triggered, e.g. pre-restore) backup's origin in its filename.
pub const MANUAL_TOKEN: &str = "_manual_";

/// The `<filename>.meta.json` sidecar (source `AppModels.swift:210-217`).
/// Field names are literal Swift `Codable` output — `camelCase`, no
/// custom `CodingKeys` — since real MSC 1 corpus backups
/// (`corpus/backups/`) carry sidecars in exactly this shape and Phase 6's
/// exit gate (P6.26) reads them back, unlike `world.rs::WorldSlot`'s
/// `slot.json`, which is an entirely MSC-2-owned file free to use its own
/// snake_case convention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupMeta {
    pub server_id: Option<String>,
    pub server_display_name: Option<String>,
    pub slot_id: Option<String>,
    pub slot_name: Option<String>,
    pub world_seed: Option<String>,
    pub trigger_reason: String,
}

impl BackupMeta {
    /// A missing or wrong-typed `triggerReason` fails the whole decode,
    /// matching `JSONDecoder`'s synthesized `Codable` (`readBackupMeta`'s
    /// caller is what turns a decode failure into "treat as legacy",
    /// not this function —
    /// `fixtures/backups/sidecar-missing-or-corrupt-leaves-filename-derived-defaults.json`).
    pub fn decode(v: &Value) -> Result<BackupMeta, DecodeError> {
        Ok(BackupMeta {
            server_id: opt_str(v, "serverId")?,
            server_display_name: opt_str(v, "serverDisplayName")?,
            slot_id: opt_str(v, "slotId")?,
            slot_name: opt_str(v, "slotName")?,
            world_seed: opt_str(v, "worldSeed")?,
            trigger_reason: req_str(v, "triggerReason")?,
        })
    }

    pub fn encode(&self) -> Value {
        let mut m = Map::new();
        insert_opt_str(&mut m, "serverId", &self.server_id);
        insert_opt_str(&mut m, "serverDisplayName", &self.server_display_name);
        insert_opt_str(&mut m, "slotId", &self.slot_id);
        insert_opt_str(&mut m, "slotName", &self.slot_name);
        insert_opt_str(&mut m, "worldSeed", &self.world_seed);
        m.insert(
            "triggerReason".to_string(),
            Value::String(self.trigger_reason.clone()),
        );
        Value::Object(m)
    }
}

/// `createBackup`'s `token = isAutomatic ? autoBackupToken : manualBackupToken`
/// (source line 233).
pub fn creation_token(is_automatic: bool) -> &'static str {
    if is_automatic {
        AUTO_TOKEN
    } else {
        MANUAL_TOKEN
    }
}

/// `createBackup`'s sidecar-reason default, `triggerReason ?? (isAutomatic
/// ? "auto" : "manual")` (source line 280) — the fallback half only; an
/// explicit override (e.g. `restoreBackup`'s `"pre-restore"`) is the
/// caller's concern, not this function's.
pub fn default_trigger_reason(is_automatic: bool) -> &'static str {
    if is_automatic { "auto" } else { "manual" }
}

/// `AppModels.swift:240` — `var isAutomatic: Bool { triggerReason == "auto" }`.
pub fn is_automatic_trigger(trigger_reason: &str) -> bool {
    trigger_reason == "auto"
}

/// `loadBackupsForSelectedServer`'s `filenameTrigger` seed (source line
/// 70): `url.lastPathComponent.contains(autoBackupToken) ? "auto" :
/// "manual"` — checked before any sidecar is read, and only ever tests
/// for the *auto* token; a filename with neither token (or only the
/// manual one) defaults to `"manual"` regardless.
pub fn filename_trigger_reason(filename: &str) -> &'static str {
    if filename.contains(AUTO_TOKEN) {
        "auto"
    } else {
        "manual"
    }
}

/// `pruneAutoBackupsIfNeeded`'s `managedFiles` filter (source line
/// 544-548): only a filename carrying one of the two creation tokens is
/// ever eligible for pruning — a pre-replace safety backup
/// (`backupWorld`'s `<archiveBaseName>-<timestamp>.zip`, no token)
/// deliberately fails this check
/// (`fixtures/backups/pre-replace-backup-has-no-token-and-is-excluded-from-pruning.json`).
pub fn is_managed_backup_filename(filename: &str) -> bool {
    filename.contains(AUTO_TOKEN) || filename.contains(MANUAL_TOKEN)
}

/// `now` is a fixed-width ISO-8601 UTC string
/// (`"YYYY-MM-DDTHH:MM:SSZ"`, the same shape every `now: &str` caller in
/// this codebase already produces) — sliced directly into
/// `createBackup`'s `"yyyyMMdd-HHmmss"` filename-timestamp format (source
/// line 228-231) without ever going through a numeric day-count/epoch
/// conversion, since both formats already express the same fields in the
/// same fixed order.
pub fn filename_timestamp_from_iso8601(now: &str) -> String {
    let y = &now[0..4];
    let mo = &now[5..7];
    let d = &now[8..10];
    let h = &now[11..13];
    let mi = &now[14..16];
    let s = &now[17..19];
    format!("{y}{mo}{d}-{h}{mi}{s}")
}

/// Strict `"yyyyMMdd-HHmmss"` parse: exactly 15 characters, a literal `-`
/// at index 8, all other characters ASCII digits, and each numeric field
/// in a plausible calendar range. `DateFormatter`'s synthesized parse of
/// this same pattern (source lines 954-955/965-966) rejects anything
/// that doesn't fit the pattern's shape; a fixture like
/// `world_manual_notatimestamp`'s non-numeric suffix is exactly the case
/// this guards against
/// (`fixtures/backups/display-name-unparseable-suffix-falls-back-to-raw-filename.json`).
fn parse_filename_timestamp(s: &str) -> Option<(u32, u32, u32, u32, u32, u32)> {
    let bytes = s.as_bytes();
    if bytes.len() != 15 || bytes[8] != b'-' {
        return None;
    }
    if !bytes[0..8].iter().all(u8::is_ascii_digit) || !bytes[9..15].iter().all(u8::is_ascii_digit) {
        return None;
    }
    let year: u32 = s[0..4].parse().ok()?;
    let month: u32 = s[4..6].parse().ok()?;
    let day: u32 = s[6..8].parse().ok()?;
    let hour: u32 = s[9..11].parse().ok()?;
    let minute: u32 = s[11..13].parse().ok()?;
    let second: u32 = s[13..15].parse().ok()?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    Some((year, month, day, hour, minute, second))
}

fn format_display((y, mo, d, h, mi, s): (u32, u32, u32, u32, u32, u32)) -> String {
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

/// `makeDisplayName(for:fallbackDate:)`'s legacy-dash-suffix branch
/// (source line 962-969): the "last dash" in `range(of: "-", options:
/// .backwards)` only lands where the notes describing this fixture claim
/// it does ("prefix can itself contain dashes without breaking the
/// timestamp parse") when the split point is the dash that precedes a
/// *validly parseable* `yyyyMMdd-HHmmss` block, not literally the
/// right-most `-` character — a `<name>-<yyyyMMdd-HHmmss>` legacy
/// filename has two dashes (one before the date, one embedded in the
/// timestamp itself), and only the first one yields a suffix
/// [`parse_filename_timestamp`] accepts. Trying every `-` position
/// left-to-right and taking the first that parses reproduces the
/// documented fixture behavior for both that two-dash shape and a
/// single-dash one, without depending on iteration order to distinguish
/// them (`fixtures/backups/display-name-legacy-dash-timestamp-format.json`).
fn parse_legacy_display_name(base: &str) -> Option<String> {
    for (idx, _) in base.match_indices('-') {
        let prefix = &base[..idx];
        let ts_str = &base[idx + 1..];
        if let Some(parts) = parse_filename_timestamp(ts_str) {
            return Some(format!("{prefix} — {}", format_display(parts)));
        }
    }
    None
}

/// `makeDisplayName(for:fallbackDate:)` (source line 941-972): the
/// current-format token loop, then the legacy dash fallback, then the
/// filename verbatim. `filename_base` is the extension-stripped filename
/// (`url.deletingPathExtension().lastPathComponent`) — stripping the
/// extension is the caller's job (`backup_store::list_backups`), not
/// this function's.
pub fn make_display_name(filename_base: &str) -> String {
    for token in [AUTO_TOKEN, MANUAL_TOKEN] {
        if let Some(idx) = filename_base.find(token) {
            let ts_str = &filename_base[idx + token.len()..];
            if let Some(parts) = parse_filename_timestamp(ts_str) {
                return format_display(parts);
            }
        }
    }
    if let Some(display) = parse_legacy_display_name(filename_base) {
        return display;
    }
    filename_base.to_string()
}
