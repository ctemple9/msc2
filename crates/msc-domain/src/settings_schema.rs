//! Pure schema validator for editing `server.properties` via a typed wire
//! format.
//!
//! Ported from the `ServerSettingsSchema` enum in
//! `RemoteAPIServer+Settings.swift`'s pure subset: `applyJava` (validate +
//! clamp a sparse set of string changes onto a `ServerPropertiesModel`) plus
//! the `level-type` wire-token helpers it depends on. The DTO-building half
//! of that file (`javaSections`, `bedrockSections`) is UI/API wiring, not a
//! domain rule, and isn't ported here; Bedrock's `applyBedrock` has no
//! fixtures (MSC 1's `ServerSettingsSchemaTests.swift` only covers Java) so
//! it stays unported too.

use crate::properties::{LevelType, ServerDifficulty, ServerGamemode, ServerPropertiesModel};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejection {
    pub key: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApplyResult {
    pub applied: Vec<String>,
    pub rejected: Vec<Rejection>,
}

fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_lowercase().as_str() {
        "true" | "1" | "on" | "yes" => Some(true),
        "false" | "0" | "off" | "no" => Some(false),
        _ => None,
    }
}

/// The wire token for a `level-type` value: the clean underscore form, e.g.
/// `large_biomes` — distinct from `LevelType::raw_value`'s escaped on-disk
/// form (`minecraft\:large_biomes`).
pub fn level_token(t: LevelType) -> &'static str {
    match t {
        LevelType::Normal => "normal",
        LevelType::Flat => "flat",
        LevelType::LargeBiomes => "large_biomes",
        LevelType::Amplified => "amplified",
    }
}

pub fn level_from_token(s: &str) -> Option<LevelType> {
    match s {
        "normal" => Some(LevelType::Normal),
        "flat" => Some(LevelType::Flat),
        "large_biomes" => Some(LevelType::LargeBiomes),
        "amplified" => Some(LevelType::Amplified),
        _ => None,
    }
}

/// Mutates `m` with the provided string changes. Ints clamp to their range,
/// bools/enums validate against their allowed set, unknown keys are
/// rejected. Note: MSC 1's `String(raw.prefix(200))` for `motd` truncates by
/// extended grapheme cluster; this ports it as a truncation by Unicode
/// scalar (`chars().take(200)`) instead, since `msc-domain` has no
/// grapheme-cluster-aware string crate and no fixture exercises the
/// difference (multi-codepoint graphemes near the 200 boundary).
pub fn apply_java(changes: &HashMap<String, String>, m: &mut ServerPropertiesModel) -> ApplyResult {
    let mut applied = Vec::new();
    let mut rejected = Vec::new();

    macro_rules! reject {
        ($key:expr, $reason:expr) => {
            rejected.push(Rejection {
                key: $key.to_string(),
                reason: $reason.to_string(),
            })
        };
    }
    macro_rules! apply_int {
        ($key:expr, $raw:expr, $lo:expr, $hi:expr, $set:expr) => {
            match $raw.trim().parse::<i64>() {
                Ok(v) => {
                    $set(v.clamp($lo, $hi));
                    applied.push($key.to_string());
                }
                Err(_) => reject!($key, "not_an_integer"),
            }
        };
    }
    macro_rules! apply_bool {
        ($key:expr, $raw:expr, $set:expr) => {
            match parse_bool($raw) {
                Some(b) => {
                    $set(b);
                    applied.push($key.to_string());
                }
                None => reject!($key, "not_a_boolean"),
            }
        };
    }

    for (key, raw) in changes {
        match key.as_str() {
            "difficulty" => match ServerDifficulty::from_raw_value(&raw.trim().to_lowercase()) {
                Some(v) => {
                    m.difficulty = v;
                    applied.push(key.clone());
                }
                None => reject!(key, "invalid_value"),
            },
            "gamemode" => match ServerGamemode::from_raw_value(&raw.trim().to_lowercase()) {
                Some(v) => {
                    m.gamemode = v;
                    applied.push(key.clone());
                }
                None => reject!(key, "invalid_value"),
            },
            "level-type" => match level_from_token(&raw.trim().to_lowercase()) {
                Some(v) => {
                    m.level_type = v;
                    applied.push(key.clone());
                }
                None => reject!(key, "invalid_value"),
            },
            "op-permission-level" => match raw.trim().parse::<i64>() {
                Ok(v) if (1..=4).contains(&v) => {
                    m.op_permission_level = v;
                    applied.push(key.clone());
                }
                _ => reject!(key, "invalid_value"),
            },

            "hardcore" => apply_bool!(key, raw, |b| m.hardcore = b),
            "pvp" => apply_bool!(key, raw, |b| m.pvp = b),
            "spawn-monsters" => apply_bool!(key, raw, |b| m.spawn_monsters = b),
            "spawn-animals" => apply_bool!(key, raw, |b| m.spawn_animals = b),
            "spawn-npcs" => apply_bool!(key, raw, |b| m.spawn_npcs = b),
            "allow-nether" => apply_bool!(key, raw, |b| m.allow_nether = b),
            "allow-flight" => apply_bool!(key, raw, |b| m.allow_flight = b),
            "force-gamemode" => apply_bool!(key, raw, |b| m.force_gamemode = b),
            "online-mode" => apply_bool!(key, raw, |b| m.online_mode = b),
            "white-list" => apply_bool!(key, raw, |b| m.whitelist = b),
            "enforce-whitelist" => apply_bool!(key, raw, |b| m.enforce_whitelist = b),

            "spawn-protection" => apply_int!(key, raw, 0, 10_000, |v| m.spawn_protection = v),
            "max-players" => apply_int!(key, raw, 1, 1000, |v| m.max_players = v),
            "view-distance" => apply_int!(key, raw, 3, 32, |v| m.view_distance = v),
            "simulation-distance" => apply_int!(key, raw, 3, 32, |v| m.simulation_distance = v),
            "player-idle-timeout" => apply_int!(key, raw, 0, 1440, |v| m.player_idle_timeout = v),
            "server-port" => apply_int!(key, raw, 1, 65_535, |v| m.server_port = v),

            "motd" => {
                m.motd = raw.chars().take(200).collect();
                applied.push(key.clone());
            }

            _ => reject!(key, "unknown_key"),
        }
    }

    ApplyResult { applied, rejected }
}
