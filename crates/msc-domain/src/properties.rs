//! Editable representation of `server.properties` with round-trip
//! preservation of unknown keys.
//!
//! Ported from `ServerPropertiesModel` in `AppViewModelModels.swift`.
//! Silently rewriting `server.properties` with only the recognized keys
//! would destroy anything a plugin/mod added on its own (RCON settings,
//! Geyser knobs, etc.) — `merged_into` overlays the model's known fields
//! onto an existing key/value map instead of replacing it outright. The
//! DTO/UI field-building half of MSC 1's `ServerPropertiesModel` story
//! (`javaSections` et al, in `RemoteAPIServer+Settings.swift`) belongs to
//! the API layer, not this crate, and isn't ported here.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerDifficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl ServerDifficulty {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Peaceful => "peaceful",
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        match raw {
            "peaceful" => Some(Self::Peaceful),
            "easy" => Some(Self::Easy),
            "normal" => Some(Self::Normal),
            "hard" => Some(Self::Hard),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerGamemode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl ServerGamemode {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Survival => "survival",
            Self::Creative => "creative",
            Self::Adventure => "adventure",
            Self::Spectator => "spectator",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        match raw {
            "survival" => Some(Self::Survival),
            "creative" => Some(Self::Creative),
            "adventure" => Some(Self::Adventure),
            "spectator" => Some(Self::Spectator),
            _ => None,
        }
    }
}

/// World type for `level-type` in server.properties. `raw_value` matches the
/// modern namespaced on-disk format (e.g. `minecraft\:normal`, a single
/// escaped backslash before the colon — matching how Minecraft itself
/// escapes namespaced ids in the file).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelType {
    Normal,
    Flat,
    LargeBiomes,
    Amplified,
}

impl LevelType {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Normal => "minecraft\\:normal",
            Self::Flat => "minecraft\\:flat",
            Self::LargeBiomes => "minecraft\\:large_biomes",
            Self::Amplified => "minecraft\\:amplified",
        }
    }

    /// Parses both the modern namespaced format and the legacy ALL-CAPS
    /// form; unrecognized input falls back to `Normal` rather than erroring.
    pub fn from_legacy_or_namespaced(raw: &str) -> Self {
        let lower = raw.to_lowercase().replace("\\:", ":");
        match lower.as_str() {
            "minecraft:flat" | "flat" => Self::Flat,
            "minecraft:large_biomes" | "largebiomes" | "large_biomes" => Self::LargeBiomes,
            "minecraft:amplified" | "amplified" => Self::Amplified,
            _ => Self::Normal,
        }
    }
}

/// Editable representation of `server.properties`.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerPropertiesModel {
    pub motd: String,
    pub max_players: i64,
    pub online_mode: bool,
    pub server_port: i64,
    pub difficulty: ServerDifficulty,
    pub gamemode: ServerGamemode,
    pub hardcore: bool,
    pub pvp: bool,
    pub allow_nether: bool,
    pub allow_flight: bool,
    pub force_gamemode: bool,
    pub spawn_monsters: bool,
    pub spawn_animals: bool,
    pub spawn_npcs: bool,
    pub spawn_protection: i64,
    pub level_type: LevelType,
    pub view_distance: i64,
    pub simulation_distance: i64,
    pub whitelist: bool,
    pub enforce_whitelist: bool,
    pub player_idle_timeout: i64,
    pub op_permission_level: i64,
    /// Raw properties dictionary so unknown keys survive a round trip.
    pub raw_properties: HashMap<String, String>,
}

fn int_val(dict: &HashMap<String, String>, key: &str, default: i64) -> i64 {
    dict.get(key)
        .and_then(|s| s.trim().parse::<i64>().ok())
        .unwrap_or(default)
}

fn bool_val(dict: &HashMap<String, String>, key: &str, default: bool) -> bool {
    match dict.get(key).map(|s| s.trim().to_lowercase()) {
        Some(s) if s == "true" => true,
        Some(s) if s == "false" => false,
        _ => default,
    }
}

impl ServerPropertiesModel {
    /// Build from a raw `[String: String]` dictionary with sane defaults,
    /// matching `ServerPropertiesModel.init(from:fallbackMotd:)`.
    pub fn from_dict(dict: &HashMap<String, String>, fallback_motd: Option<&str>) -> Self {
        let motd = dict
            .get("motd")
            .cloned()
            .or_else(|| fallback_motd.map(str::to_string))
            .unwrap_or_else(|| "A Minecraft Server".to_string());

        let difficulty = dict
            .get("difficulty")
            .and_then(|raw| ServerDifficulty::from_raw_value(&raw.trim().to_lowercase()))
            .unwrap_or(ServerDifficulty::Normal);

        let gamemode = dict
            .get("gamemode")
            .and_then(|raw| ServerGamemode::from_raw_value(&raw.trim().to_lowercase()))
            .unwrap_or(ServerGamemode::Survival);

        Self {
            motd,
            max_players: int_val(dict, "max-players", 20),
            online_mode: bool_val(dict, "online-mode", true),
            server_port: int_val(dict, "server-port", 25565),
            difficulty,
            gamemode,
            hardcore: bool_val(dict, "hardcore", false),
            pvp: bool_val(dict, "pvp", true),
            allow_nether: bool_val(dict, "allow-nether", true),
            allow_flight: bool_val(dict, "allow-flight", false),
            force_gamemode: bool_val(dict, "force-gamemode", false),
            spawn_monsters: bool_val(dict, "spawn-monsters", true),
            spawn_animals: bool_val(dict, "spawn-animals", true),
            spawn_npcs: bool_val(dict, "spawn-npcs", true),
            spawn_protection: int_val(dict, "spawn-protection", 16),
            level_type: LevelType::from_legacy_or_namespaced(
                dict.get("level-type").map(String::as_str).unwrap_or(""),
            ),
            view_distance: int_val(dict, "view-distance", 10),
            simulation_distance: int_val(dict, "simulation-distance", 10),
            whitelist: bool_val(dict, "white-list", false),
            enforce_whitelist: bool_val(dict, "enforce-whitelist", false),
            player_idle_timeout: int_val(dict, "player-idle-timeout", 0),
            op_permission_level: int_val(dict, "op-permission-level", 4),
            raw_properties: dict.clone(),
        }
    }

    /// Returns a new map with this model's known fields overlaid on top of
    /// `existing`, preserving any keys `existing` has that aren't among the
    /// recognized `server.properties` keys.
    pub fn merged_into(&self, existing: &HashMap<String, String>) -> HashMap<String, String> {
        let mut r = existing.clone();
        r.insert("motd".to_string(), self.motd.clone());
        r.insert("max-players".to_string(), self.max_players.to_string());
        r.insert("online-mode".to_string(), self.online_mode.to_string());
        r.insert("server-port".to_string(), self.server_port.to_string());
        r.insert(
            "difficulty".to_string(),
            self.difficulty.raw_value().to_string(),
        );
        r.insert(
            "gamemode".to_string(),
            self.gamemode.raw_value().to_string(),
        );
        r.insert("hardcore".to_string(), self.hardcore.to_string());
        r.insert("pvp".to_string(), self.pvp.to_string());
        r.insert("allow-nether".to_string(), self.allow_nether.to_string());
        r.insert("allow-flight".to_string(), self.allow_flight.to_string());
        r.insert(
            "force-gamemode".to_string(),
            self.force_gamemode.to_string(),
        );
        r.insert(
            "spawn-monsters".to_string(),
            self.spawn_monsters.to_string(),
        );
        r.insert("spawn-animals".to_string(), self.spawn_animals.to_string());
        r.insert("spawn-npcs".to_string(), self.spawn_npcs.to_string());
        r.insert(
            "spawn-protection".to_string(),
            self.spawn_protection.to_string(),
        );
        r.insert(
            "level-type".to_string(),
            self.level_type.raw_value().to_string(),
        );
        r.insert("view-distance".to_string(), self.view_distance.to_string());
        r.insert(
            "simulation-distance".to_string(),
            self.simulation_distance.to_string(),
        );
        r.insert("white-list".to_string(), self.whitelist.to_string());
        r.insert(
            "enforce-whitelist".to_string(),
            self.enforce_whitelist.to_string(),
        );
        r.insert(
            "player-idle-timeout".to_string(),
            self.player_idle_timeout.to_string(),
        );
        r.insert(
            "op-permission-level".to_string(),
            self.op_permission_level.to_string(),
        );
        r
    }

    /// Overlays only server-owned fields. World difficulty, default
    /// gamemode, hardcore, and generation type travel with a world slot and
    /// must survive a settings save unchanged.
    pub fn merged_server_into(
        &self,
        existing: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut r = existing.clone();
        r.insert("motd".to_string(), self.motd.clone());
        r.insert("max-players".to_string(), self.max_players.to_string());
        r.insert("online-mode".to_string(), self.online_mode.to_string());
        r.insert("server-port".to_string(), self.server_port.to_string());
        r.insert("pvp".to_string(), self.pvp.to_string());
        r.insert("allow-nether".to_string(), self.allow_nether.to_string());
        r.insert("allow-flight".to_string(), self.allow_flight.to_string());
        r.insert(
            "force-gamemode".to_string(),
            self.force_gamemode.to_string(),
        );
        r.insert(
            "spawn-monsters".to_string(),
            self.spawn_monsters.to_string(),
        );
        r.insert("spawn-animals".to_string(), self.spawn_animals.to_string());
        r.insert("spawn-npcs".to_string(), self.spawn_npcs.to_string());
        r.insert(
            "spawn-protection".to_string(),
            self.spawn_protection.to_string(),
        );
        r.insert("view-distance".to_string(), self.view_distance.to_string());
        r.insert(
            "simulation-distance".to_string(),
            self.simulation_distance.to_string(),
        );
        r.insert("white-list".to_string(), self.whitelist.to_string());
        r.insert(
            "enforce-whitelist".to_string(),
            self.enforce_whitelist.to_string(),
        );
        r.insert(
            "player-idle-timeout".to_string(),
            self.player_idle_timeout.to_string(),
        );
        r.insert(
            "op-permission-level".to_string(),
            self.op_permission_level.to_string(),
        );
        r
    }
}
