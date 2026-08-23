//! Pure Bedrock settings, console, command, and player-identity rules.
//!
//! This module deliberately contains no filesystem, process, network, or
//! LevelDB work.  It is the part of the Bedrock port that can be tested from
//! the language-neutral fixture corpus without starting a BDS process.

use crate::properties::{ServerDifficulty, ServerGamemode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockPropertiesModel {
    pub level_name: String,
    pub max_players: i64,
    pub online_mode: bool,
    pub allow_cheats: bool,
    pub difficulty: ServerDifficulty,
    pub gamemode: ServerGamemode,
    pub server_port: i64,
    pub server_port_v6: i64,
}

impl Default for BedrockPropertiesModel {
    fn default() -> Self {
        Self {
            level_name: "Bedrock level".into(),
            max_players: 10,
            online_mode: true,
            allow_cheats: false,
            difficulty: ServerDifficulty::Easy,
            gamemode: ServerGamemode::Survival,
            server_port: 19132,
            server_port_v6: 19133,
        }
    }
}

pub fn parse_raw_properties(contents: &str) -> BTreeMap<String, String> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let (key, value) = trimmed.split_once('=')?;
            Some((key.trim().to_owned(), value.trim().to_owned()))
        })
        .collect()
}

pub fn render_raw_properties(properties: &BTreeMap<String, String>) -> String {
    let mut output = String::from("# Modified via MinecraftServerController\n");
    for (key, value) in properties {
        output.push_str(key);
        output.push('=');
        output.push_str(value);
        output.push('\n');
    }
    output
}

fn parse_i64(properties: &BTreeMap<String, String>, key: &str, default: i64) -> i64 {
    properties
        .get(key)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

impl BedrockPropertiesModel {
    pub fn from_raw(properties: &BTreeMap<String, String>) -> Self {
        let mut model = Self::default();
        if let Some(value) = properties.get("level-name") {
            model.level_name = value.clone();
        }
        model.max_players = parse_i64(properties, "max-players", model.max_players);
        if let Some(value) = properties.get("online-mode") {
            model.online_mode = value == "true";
        }
        if let Some(value) = properties.get("allow-cheats") {
            model.allow_cheats = value == "true";
        }
        model.server_port = parse_i64(properties, "server-port", model.server_port);
        model.server_port_v6 = parse_i64(properties, "server-portv6", model.server_port_v6);
        if let Some(value) = properties.get("difficulty")
            && let Some(parsed) = ServerDifficulty::from_raw_value(value)
        {
            model.difficulty = parsed;
        }
        if let Some(value) = properties.get("gamemode")
            && let Some(parsed) = ServerGamemode::from_raw_value(value)
        {
            model.gamemode = parsed;
        }
        model
    }

    /// Overlays the fields MSC edits while retaining every unknown BDS key.
    /// Values are intentionally not range-clamped: BDS validation owns that
    /// decision and MSC 1 preserves out-of-range values in its model.
    pub fn merged_into(&self, existing: &BTreeMap<String, String>) -> BTreeMap<String, String> {
        let mut properties = existing.clone();
        properties.insert("level-name".into(), self.level_name.clone());
        properties.insert("max-players".into(), self.max_players.to_string());
        properties.insert("online-mode".into(), self.online_mode.to_string());
        properties.insert("allow-cheats".into(), self.allow_cheats.to_string());
        properties.insert("difficulty".into(), self.difficulty.raw_value().into());
        properties.insert("gamemode".into(), self.gamemode.raw_value().into());
        properties.insert("server-port".into(), self.server_port.to_string());
        properties.insert("server-portv6".into(), self.server_port_v6.to_string());
        properties
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowlistEntry {
    pub name: String,
    #[serde(default)]
    pub xuid: Option<String>,
    #[serde(default, alias = "ignoresPlayerLimit")]
    pub ignores_player_limit: bool,
}

pub fn parse_allowlist(json: &str) -> Vec<AllowlistEntry> {
    serde_json::from_str(json).unwrap_or_default()
}

pub fn add_allowlist_entry(
    entries: &mut Vec<AllowlistEntry>,
    name: impl Into<String>,
    xuid: Option<String>,
) -> bool {
    let name = name.into();
    if entries
        .iter()
        .any(|entry| entry.name.eq_ignore_ascii_case(&name))
    {
        return false;
    }
    entries.push(AllowlistEntry {
        name,
        xuid,
        ignores_player_limit: false,
    });
    true
}

pub fn remove_allowlist_entry(entries: &mut Vec<AllowlistEntry>, name: &str) -> bool {
    let before = entries.len();
    entries.retain(|entry| !entry.name.eq_ignore_ascii_case(name));
    before != entries.len()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BedrockPermissionLevel {
    Visitor,
    Member,
    #[serde(rename = "operator")]
    Operator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionEntry {
    pub permission: BedrockPermissionLevel,
    pub xuid: String,
}

pub fn parse_permissions(json: &str) -> Result<Vec<PermissionEntry>, serde_json::Error> {
    serde_json::from_str(json)
}

pub fn set_permission(
    entries: &mut Vec<PermissionEntry>,
    xuid: impl Into<String>,
    permission: BedrockPermissionLevel,
) {
    let xuid = xuid.into();
    if let Some(entry) = entries.iter_mut().find(|entry| entry.xuid == xuid) {
        entry.permission = permission;
    } else {
        entries.push(PermissionEntry { permission, xuid });
    }
}

pub fn remove_permission(entries: &mut Vec<PermissionEntry>, xuid: &str) -> bool {
    let before = entries.len();
    entries.retain(|entry| entry.xuid != xuid);
    before != entries.len()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedrockCommandKind {
    Ordinary,
    AllowlistAdd,
    AllowlistRemove,
    SaveHold,
    SaveQuery,
    SaveResume,
    Stop,
}

pub fn select_command(command: &str) -> BedrockCommandKind {
    let normalized = command.trim().trim_start_matches('/');
    let mut words = normalized.split_whitespace();
    match (words.next(), words.next()) {
        (Some("allowlist"), Some("add")) => BedrockCommandKind::AllowlistAdd,
        (Some("allowlist"), Some("remove")) => BedrockCommandKind::AllowlistRemove,
        (Some("save"), Some("hold")) => BedrockCommandKind::SaveHold,
        (Some("save"), Some("query")) => BedrockCommandKind::SaveQuery,
        (Some("save"), Some("resume")) => BedrockCommandKind::SaveResume,
        (Some("stop"), None) => BedrockCommandKind::Stop,
        _ => BedrockCommandKind::Ordinary,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BedrockPlayer {
    pub name: String,
    pub xuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BedrockPlayerEvent {
    Connected(BedrockPlayer),
    Disconnected(BedrockPlayer),
}

pub fn parse_player_event(line: &str) -> Option<BedrockPlayerEvent> {
    let (prefix, connected) = if line.contains("Player connected:") {
        ("Player connected: ", true)
    } else if line.contains("Player disconnected:") {
        ("Player disconnected: ", false)
    } else {
        return None;
    };
    let after = line.split_once(prefix)?.1;
    let name = after.split(',').next()?.trim();
    if name.is_empty() {
        return None;
    }
    let xuid = after
        .split_once("xuid:")
        .map(|(_, value)| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let player = BedrockPlayer {
        name: name.to_owned(),
        xuid,
    };
    Some(if connected {
        BedrockPlayerEvent::Connected(player)
    } else {
        BedrockPlayerEvent::Disconnected(player)
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct BedrockStatsLine {
    pub cpu_percent: Option<f64>,
    pub memory_used_mb: Option<u64>,
    pub memory_total_mb: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BedrockConsoleEvent {
    Ready,
    Version(String),
    Player(BedrockPlayerEvent),
    Stats(BedrockStatsLine),
    GuestIp(IpAddr),
    Other,
}

pub fn classify_console_line(line: &str) -> BedrockConsoleEvent {
    if let Some(stats) = parse_stats_line(line) {
        return BedrockConsoleEvent::Stats(stats);
    }
    if line.to_ascii_lowercase().contains("server started") {
        return BedrockConsoleEvent::Ready;
    }
    if let Some(event) = parse_player_event(line) {
        return BedrockConsoleEvent::Player(event);
    }
    if let Some(version) = parse_version_line(line) {
        return BedrockConsoleEvent::Version(version);
    }
    if let Some(ip) = parse_guest_ip(line) {
        return BedrockConsoleEvent::GuestIp(ip);
    }
    BedrockConsoleEvent::Other
}

pub fn parse_version_line(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let start = lower.find("version ")? + "version ".len();
    line[start..].split_whitespace().next().map(str::to_owned)
}

pub fn parse_guest_ip(line: &str) -> Option<IpAddr> {
    let (_, value) = line.split_once("dhcp:")?;
    value
        .split_whitespace()
        .next()?
        .split('/')
        .next()?
        .parse()
        .ok()
}

pub fn parse_stats_line(line: &str) -> Option<BedrockStatsLine> {
    if !line.trim_start().starts_with("[MSCSTATS]") {
        return None;
    }
    let mut stats = BedrockStatsLine {
        cpu_percent: None,
        memory_used_mb: None,
        memory_total_mb: None,
    };
    for field in line.trim_start()["[MSCSTATS]".len()..].split_whitespace() {
        let Some((key, value)) = field.split_once('=') else {
            continue;
        };
        match key {
            "cpu" => stats.cpu_percent = value.parse().ok(),
            "memUsedMB" => stats.memory_used_mb = value.parse().ok(),
            "memTotalMB" => stats.memory_total_mb = value.parse().ok(),
            _ => {}
        }
    }
    Some(stats)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BedrockRuntimeStatus {
    Unavailable,
    Stopped,
    Starting,
    Running,
}

/// A client-safe status is a closed vocabulary.  Raw sidecar/host errors do
/// not become user-visible status strings and therefore cannot leak paths or
/// command output into a status card.
pub fn runtime_status(available: bool, running: bool, ready: bool) -> BedrockRuntimeStatus {
    if !available {
        BedrockRuntimeStatus::Unavailable
    } else if !running {
        BedrockRuntimeStatus::Stopped
    } else if ready {
        BedrockRuntimeStatus::Running
    } else {
        BedrockRuntimeStatus::Starting
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BedrockPlayerIdentity {
    Local,
    NumericXuid(String),
    ServerUuid { xuid: String, uuid: String },
}

pub fn player_identity_from_key(key: &str) -> Option<BedrockPlayerIdentity> {
    if key == "~local_player" {
        return Some(BedrockPlayerIdentity::Local);
    }
    let xuid = key.strip_prefix("player_")?;
    if xuid.is_empty() {
        return None;
    }
    if xuid.chars().all(|c| c.is_ascii_digit()) {
        return Some(BedrockPlayerIdentity::NumericXuid(xuid.to_owned()));
    }
    let uuid = xuid.strip_prefix("server_")?;
    if uuid.len() == 36
        && uuid.bytes().enumerate().all(|(index, byte)| {
            (index == 8 || index == 13 || index == 18 || index == 23) && byte == b'-'
                || (index != 8
                    && index != 13
                    && index != 18
                    && index != 23
                    && byte.is_ascii_hexdigit())
        })
    {
        return Some(BedrockPlayerIdentity::ServerUuid {
            xuid: xuid.to_owned(),
            uuid: uuid.to_owned(),
        });
    }
    None
}

pub fn player_display_name(identity: &BedrockPlayerIdentity) -> Option<&'static str> {
    match identity {
        BedrockPlayerIdentity::Local => Some("Local Player"),
        BedrockPlayerIdentity::NumericXuid(_) => None,
        BedrockPlayerIdentity::ServerUuid { .. } => Some("Unknown Player"),
    }
}

pub fn xuid_lookup_url(xuid: &str) -> String {
    format!("https://api.geysermc.org/v2/xbox/gamertag/{xuid}")
}

pub fn floodgate_lookup_path(gamertag: &str) -> String {
    let name = if gamertag.starts_with('.') {
        gamertag.to_owned()
    } else {
        format!(".{gamertag}")
    };
    let encoded = name
        .chars()
        .map(|character| match character {
            ' ' => "%20".to_owned(),
            _ => character.to_string(),
        })
        .collect::<String>();
    format!("/v2/utils/uuid/bedrock_or_java/{encoded}?prefix=.")
}

pub fn normalize_uuid(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.len() == 36
        && [8, 13, 18, 23]
            .into_iter()
            .all(|index| raw.as_bytes().get(index) == Some(&b'-'))
        && raw
            .bytes()
            .enumerate()
            .all(|(index, byte)| [8, 13, 18, 23].contains(&index) || byte.is_ascii_hexdigit())
    {
        return Some(raw.to_owned());
    }
    if raw.len() != 32 || !raw.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!(
        "{}-{}-{}-{}-{}",
        &raw[0..8],
        &raw[8..12],
        &raw[12..16],
        &raw[16..20],
        &raw[20..32]
    ))
}

pub fn trimmed_name_cache_record(
    mapping: &mut BTreeMap<String, String>,
    xuid: &str,
    name: &str,
) -> bool {
    let xuid = xuid.trim();
    let name = name.trim();
    if xuid.is_empty() || name.is_empty() {
        return false;
    }
    mapping.insert(xuid.to_owned(), name.to_owned());
    true
}

pub fn toggle_hidden_profile(hidden: &mut std::collections::BTreeSet<String>, xuid: &str) {
    if !hidden.insert(xuid.to_owned()) {
        hidden.remove(xuid);
    }
}

pub fn backfill_allowlist_xuid(
    selected_server_is_bedrock: bool,
    entries: &[AllowlistEntry],
    name: &str,
    xuid: &str,
) -> Option<Vec<AllowlistEntry>> {
    if !selected_server_is_bedrock {
        return None;
    }
    let mut entries = entries.to_vec();
    if let Some(entry) = entries
        .iter_mut()
        .find(|entry| entry.name.eq_ignore_ascii_case(name) && entry.xuid.is_none())
    {
        entry.xuid = Some(xuid.to_owned());
        return Some(entries);
    }
    None
}
