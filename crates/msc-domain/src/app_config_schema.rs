//! Pure decode/encode/defaulting half of MSC 1's `AppConfig.swift`
//! (`ConfigServer`, `RemoteAPISharedAccessEntry`, `AppConfig`; symbol
//! ledger rows `ConfigServer.init(from:)/encode(to:)`,
//! `AppConfig.init(from:)/encode(to:)`, `ConfigServer.minRamMB/maxRamMB`).
//! No I/O: `AppConfig::decode` takes an already-resolved `servers_root`
//! default via `defaults: &AppConfig` rather than reading the home
//! directory itself (that belongs to whichever layer wires this schema
//! through the repository, per `settings_schema.rs`'s own domain/infra
//! split).
//!
//! `AppConfig.init(from:)`'s decode-time normalization pass (P5.5) is
//! implemented here too: trimming/blank-to-`None` on
//! `remote_api_preferred_pairing_host`, and trim/fresh-id/drop-blank-token/
//! dedupe-by-token on `remote_api_shared_access`. See
//! `fixtures/app-config-normalization/` and
//! `tests/app_config_normalization.rs`.
//!
//! Two genuine source quirks are preserved exactly, not "fixed":
//! `remoteAPIToken` is never decoded from or encoded to JSON (Keychain
//! only — line 758/856) and `useVMBedrockBackend` is decoded but has no
//! corresponding line in `encode(to:)` at all, so it never round-trips
//! (confirmed absent from the source's `encode(to:)` body).
//!
//! Three referenced MSC 1 types have no Rust port yet
//! (`PluginSourceConfig`, `AddonLink`, `LoaderVersionRecord` — all
//! outside this step's scope) — their fields decode/encode as opaque
//! `serde_json::Value` pass-through rather than typed structs.

use crate::identity::{JavaServerFlavor, ServerType};
use serde_json::{Map, Value};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError(pub String);

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn err(msg: impl Into<String>) -> DecodeError {
    DecodeError(msg.into())
}

/// Both a missing key and an explicit JSON `null` decode to "absent" here,
/// matching Foundation's `decodeIfPresent` (it checks `contains(key)` and
/// then `decodeNil(forKey:)` before ever decoding a value).
fn present<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.get(key).filter(|x| !x.is_null())
}

fn req_str(v: &Value, key: &str) -> Result<String, DecodeError> {
    match present(v, key) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(err(format!("missing or invalid required field \"{key}\""))),
    }
}

fn req_f64(v: &Value, key: &str) -> Result<f64, DecodeError> {
    match present(v, key).and_then(Value::as_f64) {
        Some(n) => Ok(n),
        None => Err(err(format!("missing or invalid required field \"{key}\""))),
    }
}

/// A present-but-wrong-typed value is a real decode failure (matches a
/// plain `try c.decodeIfPresent(...)`, not a `try?`-wrapped one) —
/// callers needing the `try?`-swallow behavior use `lenient_opt_str`
/// instead.
fn opt_str(v: &Value, key: &str) -> Result<Option<String>, DecodeError> {
    match present(v, key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(err(format!("field \"{key}\" is not a string"))),
    }
}

fn opt_bool(v: &Value, key: &str, default: bool) -> Result<bool, DecodeError> {
    match present(v, key) {
        None => Ok(default),
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(err(format!("field \"{key}\" is not a bool"))),
    }
}

fn opt_i64(v: &Value, key: &str, default: i64) -> Result<i64, DecodeError> {
    match present(v, key) {
        None => Ok(default),
        Some(n) if n.is_i64() || n.is_u64() => Ok(n.as_i64().expect("checked i64/u64 above")),
        Some(_) => Err(err(format!("field \"{key}\" is not an integer"))),
    }
}

fn opt_i64_no_default(v: &Value, key: &str) -> Result<Option<i64>, DecodeError> {
    match present(v, key) {
        None => Ok(None),
        Some(n) if n.is_i64() || n.is_u64() => Ok(Some(n.as_i64().expect("checked i64/u64 above"))),
        Some(_) => Err(err(format!("field \"{key}\" is not an integer"))),
    }
}

/// `try?`-swallow: any type mismatch is treated the same as a missing key
/// (returns `None`) rather than failing the whole decode.
fn lenient_opt_str(v: &Value, key: &str) -> Option<String> {
    match present(v, key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// `try?`-swallow for `[String]`: a non-array value, or any non-string
/// element, drops the whole field to `None` rather than failing decode.
fn lenient_opt_str_array(v: &Value, key: &str) -> Option<Vec<String>> {
    match present(v, key) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| item.as_str().map(str::to_string))
            .collect(),
        _ => None,
    }
}

fn insert_str(m: &mut Map<String, Value>, key: &str, value: &str) {
    m.insert(key.to_string(), Value::String(value.to_string()));
}

fn insert_opt_str(m: &mut Map<String, Value>, key: &str, value: &Option<String>) {
    if let Some(s) = value {
        insert_str(m, key, s);
    }
}

fn insert_opt_i64(m: &mut Map<String, Value>, key: &str, value: Option<i64>) {
    if let Some(n) = value {
        m.insert(key.to_string(), Value::from(n));
    }
}

/// Mirrors `(defaultServersRootPath() as NSString).appendingPathComponent(_:)`
/// closely enough for this port: simple `/`-joining, no `..`/`.`
/// resolution (MSC 1 never receives those in this path).
fn join_path(root: &str, component: &str) -> String {
    format!("{}/{component}", root.trim_end_matches('/'))
}

// MARK: - XboxBroadcastIPMode

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XboxBroadcastIpMode {
    Auto,
    PublicIp,
    PrivateIp,
}

impl XboxBroadcastIpMode {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::PublicIp => "public_ip",
            Self::PrivateIp => "private_ip",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        match raw {
            "auto" => Some(Self::Auto),
            "public_ip" => Some(Self::PublicIp),
            "private_ip" => Some(Self::PrivateIp),
            _ => None,
        }
    }
}

// MARK: - ServerNotificationPrefs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ServerNotificationPrefs {
    pub notify_on_start: bool,
    pub notify_on_stop: bool,
    pub notify_on_player_join: bool,
    pub notify_on_player_leave: bool,
}

impl ServerNotificationPrefs {
    pub fn decode(v: &Value) -> Result<Self, DecodeError> {
        Ok(Self {
            notify_on_start: opt_bool(v, "notify_on_start", false)?,
            notify_on_stop: opt_bool(v, "notify_on_stop", false)?,
            notify_on_player_join: opt_bool(v, "notify_on_player_join", false)?,
            notify_on_player_leave: opt_bool(v, "notify_on_player_leave", false)?,
        })
    }

    pub fn encode(&self) -> Value {
        let mut m = Map::new();
        m.insert("notify_on_start".into(), Value::Bool(self.notify_on_start));
        m.insert("notify_on_stop".into(), Value::Bool(self.notify_on_stop));
        m.insert(
            "notify_on_player_join".into(),
            Value::Bool(self.notify_on_player_join),
        );
        m.insert(
            "notify_on_player_leave".into(),
            Value::Bool(self.notify_on_player_leave),
        );
        Value::Object(m)
    }
}

// MARK: - RemoteAPISharedAccessEntry

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteApiSharedAccessEntry {
    pub id: String,
    pub label: String,
    pub token: String,
    pub role: String,
    pub created_at_iso8601: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub expires_at_iso8601: Option<String>,
}

impl RemoteApiSharedAccessEntry {
    /// `id`/`label`/`token` are required (a real decode failure if
    /// missing/wrong-typed); `role`/`created_at`/`permissions`/
    /// `expires_at` are all `try?`-wrapped in the source, so a type
    /// mismatch on any of them silently falls back to their default
    /// instead of failing the entry.
    pub fn decode(v: &Value) -> Result<Self, DecodeError> {
        Ok(Self {
            id: req_str(v, "id")?,
            label: req_str(v, "label")?,
            token: req_str(v, "token")?,
            role: lenient_opt_str(v, "role").unwrap_or_else(|| "admin".to_string()),
            created_at_iso8601: lenient_opt_str(v, "created_at"),
            permissions: lenient_opt_str_array(v, "permissions"),
            expires_at_iso8601: lenient_opt_str(v, "expires_at"),
        })
    }

    pub fn encode(&self) -> Value {
        let mut m = Map::new();
        insert_str(&mut m, "id", &self.id);
        insert_str(&mut m, "label", &self.label);
        insert_str(&mut m, "token", &self.token);
        insert_str(&mut m, "role", &self.role);
        insert_opt_str(&mut m, "created_at", &self.created_at_iso8601);
        if let Some(p) = &self.permissions {
            m.insert(
                "permissions".into(),
                Value::Array(p.iter().cloned().map(Value::String).collect()),
            );
        }
        insert_opt_str(&mut m, "expires_at", &self.expires_at_iso8601);
        Value::Object(m)
    }
}

// MARK: - ConfigServer

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigServer {
    pub id: String,
    pub display_name: String,
    pub server_dir: String,
    pub paper_jar_path: String,
    /// RAM allocation in GB; fractional values are allowed. See
    /// `min_ram_mb`/`max_ram_mb` for the whole-MB conversion used by the
    /// JVM/VM flags.
    pub min_ram_gb: f64,
    pub max_ram_gb: f64,

    pub bedrock_port: Option<i64>,
    pub bedrock_enabled: bool,
    pub public_host_override: Option<String>,
    pub notes: String,
    pub banner_color_hex: Option<String>,
    pub join_card_color_hex: Option<String>,
    pub has_ever_started: bool,
    pub has_shown_first_start_popup: bool,

    pub auto_backup_enabled: bool,
    pub auto_backup_interval_minutes: i64,
    pub auto_backup_max_count: i64,

    pub xbox_broadcast_ip_mode: XboxBroadcastIpMode,
    pub xbox_broadcast_enabled: bool,
    pub xbox_broadcast_host_override: Option<String>,
    pub xbox_broadcast_port_override: Option<i64>,
    pub resource_pack_host_port: i64,
    pub xbox_broadcast_config_path: Option<String>,
    pub xbox_broadcast_alt_email: Option<String>,
    pub xbox_broadcast_alt_gamertag: Option<String>,
    /// Keychain-only; never decoded from or encoded to JSON.
    pub xbox_broadcast_alt_password: Option<String>,
    pub xbox_broadcast_alt_avatar_path: Option<String>,

    pub server_type: ServerType,
    pub bedrock_docker_image: Option<String>,
    pub bedrock_version: Option<String>,

    pub java_flavor: JavaServerFlavor,
    pub minecraft_version: Option<String>,
    pub loader_version: Option<String>,
    pub server_build: Option<String>,

    pub notification_prefs: ServerNotificationPrefs,
    /// `PluginSourceConfig` isn't ported yet; passed through opaquely.
    pub plugin_sources: Option<Value>,
    /// `AddonLink` isn't ported yet; passed through opaquely.
    pub addon_links: Option<Value>,

    pub playit_enabled: bool,
    pub playit_voice_chat_enabled: bool,
    pub svc_tunnel_prompt_dismissed: bool,
    pub svc_port_forwarding_confirmed: bool,
    pub auto_restart_on_crash: bool,

    pub pack_managed: bool,
    pub pack_name: Option<String>,
    pub pack_version: Option<String>,
}

impl ConfigServer {
    /// Mirrors Swift's implicit memberwise initializer: the six fields
    /// with no property default are required parameters, everything else
    /// takes the default the source declares on the property itself.
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        server_dir: impl Into<String>,
        paper_jar_path: impl Into<String>,
        min_ram_gb: f64,
        max_ram_gb: f64,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            server_dir: server_dir.into(),
            paper_jar_path: paper_jar_path.into(),
            min_ram_gb,
            max_ram_gb,

            bedrock_port: None,
            bedrock_enabled: false,
            public_host_override: None,
            notes: String::new(),
            banner_color_hex: None,
            join_card_color_hex: None,
            has_ever_started: false,
            has_shown_first_start_popup: false,

            auto_backup_enabled: false,
            auto_backup_interval_minutes: 30,
            auto_backup_max_count: 12,

            xbox_broadcast_ip_mode: XboxBroadcastIpMode::Auto,
            xbox_broadcast_enabled: false,
            xbox_broadcast_host_override: None,
            xbox_broadcast_port_override: None,
            resource_pack_host_port: 8123,
            xbox_broadcast_config_path: None,
            xbox_broadcast_alt_email: None,
            xbox_broadcast_alt_gamertag: None,
            xbox_broadcast_alt_password: None,
            xbox_broadcast_alt_avatar_path: None,

            server_type: ServerType::Java,
            bedrock_docker_image: None,
            bedrock_version: None,

            java_flavor: JavaServerFlavor::Paper,
            minecraft_version: None,
            loader_version: None,
            server_build: None,

            notification_prefs: ServerNotificationPrefs::default(),
            plugin_sources: None,
            addon_links: None,

            playit_enabled: false,
            playit_voice_chat_enabled: false,
            svc_tunnel_prompt_dismissed: false,
            svc_port_forwarding_confirmed: false,
            auto_restart_on_crash: false,

            pack_managed: false,
            pack_name: None,
            pack_version: None,
        }
    }

    /// Min heap in whole MB for the `-Xms` JVM flag. Rounds a fractional
    /// GB value the same way Swift's `.rounded()` does (half away from
    /// zero), which is also `f64::round`'s behavior.
    pub fn min_ram_mb(&self) -> i64 {
        (self.min_ram_gb * 1024.0).round() as i64
    }

    /// Max heap in whole MB for `-Xmx` / Docker `--memory` / VM memory.
    pub fn max_ram_mb(&self) -> i64 {
        (self.max_ram_gb * 1024.0).round() as i64
    }

    pub fn decode(v: &Value) -> Result<Self, DecodeError> {
        let id = req_str(v, "id")?;
        let display_name = req_str(v, "display_name")?;
        let server_dir = req_str(v, "server_dir")?;
        let paper_jar_path = req_str(v, "paper_jar_path")?;
        // Older configs wrote whole-number ints; JSON numbers decode into
        // f64 transparently either way, so no migration is needed.
        let min_ram_gb = req_f64(v, "min_ram_gb")?;
        let max_ram_gb = req_f64(v, "max_ram_gb")?;

        let bedrock_port = opt_i64_no_default(v, "bedrock_port")?;
        let bedrock_enabled = opt_bool(v, "bedrock_enabled", false)?;
        let public_host_override = opt_str(v, "public_host_override")?;
        let notes = opt_str(v, "notes")?.unwrap_or_default();
        let banner_color_hex = opt_str(v, "banner_color_hex")?;
        let join_card_color_hex = opt_str(v, "join_card_color_hex")?;
        let has_ever_started = opt_bool(v, "has_ever_started", false)?;
        let has_shown_first_start_popup = opt_bool(v, "has_shown_first_start_popup", false)?;

        let auto_backup_enabled = opt_bool(v, "auto_backup_enabled", false)?;
        let auto_backup_interval_minutes = opt_i64(v, "auto_backup_interval_minutes", 30)?;
        let auto_backup_max_count = opt_i64(v, "auto_backup_max_count", 12)?;

        let xbox_broadcast_ip_mode = match present(v, "xbox_broadcast_ip_mode") {
            None => XboxBroadcastIpMode::Auto,
            Some(Value::String(s)) => XboxBroadcastIpMode::from_raw_value(s)
                .ok_or_else(|| err(format!("unknown xbox_broadcast_ip_mode \"{s}\"")))?,
            Some(_) => return Err(err("field \"xbox_broadcast_ip_mode\" is not a string")),
        };
        let xbox_broadcast_enabled = opt_bool(v, "xbox_broadcast_enabled", false)?;
        let xbox_broadcast_host_override = opt_str(v, "xbox_broadcast_host_override")?;
        let xbox_broadcast_port_override = opt_i64_no_default(v, "xbox_broadcast_port_override")?;
        let resource_pack_host_port = opt_i64(v, "resource_pack_host_port", 8123)?;
        let xbox_broadcast_config_path = opt_str(v, "xbox_broadcast_config_path")?;
        let xbox_broadcast_alt_email = opt_str(v, "xbox_broadcast_alt_email")?;
        let xbox_broadcast_alt_gamertag = opt_str(v, "xbox_broadcast_alt_gamertag")?;
        // Keychain-only; never decoded from JSON.
        let xbox_broadcast_alt_password = None;
        let xbox_broadcast_alt_avatar_path = opt_str(v, "xbox_broadcast_alt_avatar_path")?;

        let server_type = match present(v, "server_type") {
            None => ServerType::Java,
            Some(Value::String(s)) => ServerType::from_raw_value(s)
                .ok_or_else(|| err(format!("unknown server_type \"{s}\"")))?,
            Some(_) => return Err(err("field \"server_type\" is not a string")),
        };
        let bedrock_docker_image = opt_str(v, "bedrock_docker_image")?;
        let bedrock_version = opt_str(v, "bedrock_version")?;

        // Forward-compat: an unknown/future java_flavor string never
        // invalidates the whole entry. Source wraps this decode in
        // `try?` and falls back to `.paper` — unlike every other typed
        // field above, which throws on a type/value mismatch.
        let java_flavor = match present(v, "java_flavor").and_then(Value::as_str) {
            Some(s) => JavaServerFlavor::from_raw_value(s).unwrap_or(JavaServerFlavor::Paper),
            None => JavaServerFlavor::Paper,
        };
        let minecraft_version = opt_str(v, "minecraft_version")?;
        let loader_version = opt_str(v, "loader_version")?;
        let server_build = opt_str(v, "server_build")?;

        let notification_prefs = match present(v, "notification_prefs") {
            None => ServerNotificationPrefs::default(),
            Some(obj) => ServerNotificationPrefs::decode(obj)?,
        };
        let plugin_sources = present(v, "plugin_sources").cloned();
        let addon_links = present(v, "addon_links").cloned();

        let playit_enabled = opt_bool(v, "playit_enabled", false)?;
        let playit_voice_chat_enabled = opt_bool(v, "playit_voice_chat_enabled", false)?;
        let svc_tunnel_prompt_dismissed = opt_bool(v, "svc_tunnel_prompt_dismissed", false)?;
        let svc_port_forwarding_confirmed = opt_bool(v, "svc_port_forwarding_confirmed", false)?;
        let auto_restart_on_crash = opt_bool(v, "auto_restart_on_crash", false)?;

        let pack_managed = opt_bool(v, "pack_managed", false)?;
        let pack_name = opt_str(v, "pack_name")?;
        let pack_version = opt_str(v, "pack_version")?;

        Ok(Self {
            id,
            display_name,
            server_dir,
            paper_jar_path,
            min_ram_gb,
            max_ram_gb,
            bedrock_port,
            bedrock_enabled,
            public_host_override,
            notes,
            banner_color_hex,
            join_card_color_hex,
            has_ever_started,
            has_shown_first_start_popup,
            auto_backup_enabled,
            auto_backup_interval_minutes,
            auto_backup_max_count,
            xbox_broadcast_ip_mode,
            xbox_broadcast_enabled,
            xbox_broadcast_host_override,
            xbox_broadcast_port_override,
            resource_pack_host_port,
            xbox_broadcast_config_path,
            xbox_broadcast_alt_email,
            xbox_broadcast_alt_gamertag,
            xbox_broadcast_alt_password,
            xbox_broadcast_alt_avatar_path,
            server_type,
            bedrock_docker_image,
            bedrock_version,
            java_flavor,
            minecraft_version,
            loader_version,
            server_build,
            notification_prefs,
            plugin_sources,
            addon_links,
            playit_enabled,
            playit_voice_chat_enabled,
            svc_tunnel_prompt_dismissed,
            svc_port_forwarding_confirmed,
            auto_restart_on_crash,
            pack_managed,
            pack_name,
            pack_version,
        })
    }

    pub fn encode(&self) -> Value {
        let mut m = Map::new();
        insert_str(&mut m, "id", &self.id);
        insert_str(&mut m, "display_name", &self.display_name);
        insert_str(&mut m, "server_dir", &self.server_dir);
        insert_str(&mut m, "paper_jar_path", &self.paper_jar_path);
        m.insert("min_ram_gb".into(), Value::from(self.min_ram_gb));
        m.insert("max_ram_gb".into(), Value::from(self.max_ram_gb));

        insert_opt_i64(&mut m, "bedrock_port", self.bedrock_port);
        insert_opt_str(&mut m, "bedrock_version", &self.bedrock_version);
        m.insert("bedrock_enabled".into(), Value::Bool(self.bedrock_enabled));
        insert_opt_str(&mut m, "public_host_override", &self.public_host_override);
        insert_str(&mut m, "notes", &self.notes);
        insert_opt_str(&mut m, "banner_color_hex", &self.banner_color_hex);
        insert_opt_str(&mut m, "join_card_color_hex", &self.join_card_color_hex);
        m.insert(
            "has_ever_started".into(),
            Value::Bool(self.has_ever_started),
        );
        m.insert(
            "has_shown_first_start_popup".into(),
            Value::Bool(self.has_shown_first_start_popup),
        );

        m.insert(
            "auto_backup_enabled".into(),
            Value::Bool(self.auto_backup_enabled),
        );
        m.insert(
            "auto_backup_interval_minutes".into(),
            Value::from(self.auto_backup_interval_minutes),
        );
        m.insert(
            "auto_backup_max_count".into(),
            Value::from(self.auto_backup_max_count),
        );

        insert_str(
            &mut m,
            "xbox_broadcast_ip_mode",
            self.xbox_broadcast_ip_mode.raw_value(),
        );
        m.insert(
            "xbox_broadcast_enabled".into(),
            Value::Bool(self.xbox_broadcast_enabled),
        );
        insert_opt_str(
            &mut m,
            "xbox_broadcast_host_override",
            &self.xbox_broadcast_host_override,
        );
        insert_opt_i64(
            &mut m,
            "xbox_broadcast_port_override",
            self.xbox_broadcast_port_override,
        );
        m.insert(
            "resource_pack_host_port".into(),
            Value::from(self.resource_pack_host_port),
        );
        insert_opt_str(
            &mut m,
            "xbox_broadcast_config_path",
            &self.xbox_broadcast_config_path,
        );
        insert_opt_str(
            &mut m,
            "xbox_broadcast_alt_email",
            &self.xbox_broadcast_alt_email,
        );
        insert_opt_str(
            &mut m,
            "xbox_broadcast_alt_gamertag",
            &self.xbox_broadcast_alt_gamertag,
        );
        // xbox_broadcast_alt_password intentionally omitted -- Keychain-only.
        insert_opt_str(
            &mut m,
            "xbox_broadcast_alt_avatar_path",
            &self.xbox_broadcast_alt_avatar_path,
        );

        insert_str(&mut m, "server_type", self.server_type.raw_value());
        insert_opt_str(&mut m, "bedrock_docker_image", &self.bedrock_docker_image);

        insert_str(&mut m, "java_flavor", self.java_flavor.raw_value());
        insert_opt_str(&mut m, "minecraft_version", &self.minecraft_version);
        insert_opt_str(&mut m, "loader_version", &self.loader_version);
        insert_opt_str(&mut m, "server_build", &self.server_build);

        m.insert(
            "notification_prefs".into(),
            self.notification_prefs.encode(),
        );
        if let Some(v) = &self.plugin_sources {
            m.insert("plugin_sources".into(), v.clone());
        }
        if let Some(v) = &self.addon_links {
            m.insert("addon_links".into(), v.clone());
        }

        m.insert("playit_enabled".into(), Value::Bool(self.playit_enabled));
        m.insert(
            "playit_voice_chat_enabled".into(),
            Value::Bool(self.playit_voice_chat_enabled),
        );
        m.insert(
            "svc_tunnel_prompt_dismissed".into(),
            Value::Bool(self.svc_tunnel_prompt_dismissed),
        );
        m.insert(
            "svc_port_forwarding_confirmed".into(),
            Value::Bool(self.svc_port_forwarding_confirmed),
        );
        m.insert(
            "auto_restart_on_crash".into(),
            Value::Bool(self.auto_restart_on_crash),
        );

        m.insert("pack_managed".into(), Value::Bool(self.pack_managed));
        insert_opt_str(&mut m, "pack_name", &self.pack_name);
        insert_opt_str(&mut m, "pack_version", &self.pack_version);

        Value::Object(m)
    }
}

// MARK: - AppConfig

#[derive(Debug, Clone, PartialEq)]
pub struct AppConfig {
    pub config_version: i64,
    pub java_path: String,
    pub extra_flags: String,
    pub servers_root: String,
    pub plugin_template_dir: String,
    pub paper_template_dir: String,
    pub servers: Vec<ConfigServer>,
    pub active_server_id: Option<String>,
    pub initial_setup_done: bool,

    pub remote_api_port: i64,
    /// Keychain-only; never decoded from or encoded to JSON.
    pub remote_api_token: String,
    pub remote_api_expose_on_lan: bool,
    pub remote_api_preferred_pairing_host: Option<String>,
    pub remote_api_shared_access: Vec<RemoteApiSharedAccessEntry>,

    pub duckdns_hostname: Option<String>,
    pub playit_java_address: Option<String>,
    pub playit_bedrock_address: Option<String>,
    pub playit_voice_address: Option<String>,
    pub playit_agent_id: Option<String>,

    pub has_shown_handbook: bool,
    pub has_shown_concept_guide: bool,

    pub xbox_broadcast_jar_path: Option<String>,
    pub xbox_broadcast_auto_start_enabled: bool,
    pub minecraft_username: Option<String>,
    pub minecraft_bedrock_gamertag: Option<String>,
    pub minecraft_avatar_edition_raw_value: Option<String>,
    pub default_banner_color_hex: Option<String>,
    pub error_popups_enabled: bool,
    pub save_downloaded_jars: bool,
    /// `LoaderVersionRecord` isn't ported yet; passed through opaquely.
    pub loader_version_records: Vec<Value>,
    pub use_vm_bedrock_backend: bool,
}

impl AppConfig {
    pub const DEFAULT_REMOTE_API_PORT: i64 = 48400;
    /// Bump when the persisted config schema changes incompatibly.
    pub const LATEST_CONFIG_VERSION: i64 = 1;

    /// Pure equivalent of `AppConfig.defaultConfig()`: the caller resolves
    /// `servers_root` (MSC 1 uses the user's home directory, which is I/O
    /// this crate deliberately doesn't perform) and this derives
    /// `plugin_template_dir`/`paper_template_dir` from it the same way
    /// `defaultPluginTemplateDirPath()`/`defaultPaperTemplateDirPath()` do.
    pub fn default_config(servers_root: impl Into<String>) -> Self {
        let servers_root = servers_root.into();
        let plugin_template_dir = join_path(&servers_root, "_plugin_templates");
        let paper_template_dir = join_path(&servers_root, "_paper_templates");
        Self {
            config_version: Self::LATEST_CONFIG_VERSION,
            java_path: "java".to_string(),
            extra_flags: String::new(),
            servers_root,
            plugin_template_dir,
            paper_template_dir,
            servers: Vec::new(),
            active_server_id: None,
            initial_setup_done: false,

            remote_api_port: Self::DEFAULT_REMOTE_API_PORT,
            remote_api_token: String::new(),
            remote_api_expose_on_lan: false,
            remote_api_preferred_pairing_host: None,
            remote_api_shared_access: Vec::new(),

            duckdns_hostname: None,
            playit_java_address: None,
            playit_bedrock_address: None,
            playit_voice_address: None,
            playit_agent_id: None,
            has_shown_handbook: false,
            has_shown_concept_guide: false,
            xbox_broadcast_jar_path: None,
            xbox_broadcast_auto_start_enabled: true,
            minecraft_username: None,
            minecraft_bedrock_gamertag: None,
            minecraft_avatar_edition_raw_value: None,
            default_banner_color_hex: None,
            error_popups_enabled: false,
            save_downloaded_jars: true,
            loader_version_records: Vec::new(),
            use_vm_bedrock_backend: true,
        }
    }

    /// `defaults` supplies every top-level fallback (typically
    /// `AppConfig::default_config(...)` with the caller's resolved
    /// `servers_root`). Only `servers` can make this fail: a malformed
    /// entry inside the array is a real decode error that propagates,
    /// same as every other field here except `java_flavor` isn't — see
    /// `ConfigServer::decode`.
    pub fn decode(v: &Value, defaults: &AppConfig) -> Result<Self, DecodeError> {
        let config_version = opt_i64(v, "config_version", defaults.config_version)?;
        let use_vm_bedrock_backend =
            opt_bool(v, "use_vm_bedrock_backend", defaults.use_vm_bedrock_backend)?;
        let java_path = opt_str(v, "java_path")?.unwrap_or_else(|| defaults.java_path.clone());
        let extra_flags =
            opt_str(v, "extra_flags")?.unwrap_or_else(|| defaults.extra_flags.clone());
        let servers_root =
            opt_str(v, "servers_root")?.unwrap_or_else(|| defaults.servers_root.clone());
        // Defaults are derived from the servers_root just decoded above,
        // not from `defaults.plugin_template_dir` — matches source lines
        // 738-743, which recompute from `self.serversRoot`.
        let plugin_template_dir = opt_str(v, "plugin_template_dir")?
            .unwrap_or_else(|| join_path(&servers_root, "_plugin_templates"));
        let paper_template_dir = opt_str(v, "paper_template_dir")?
            .unwrap_or_else(|| join_path(&servers_root, "_paper_templates"));

        let servers = match present(v, "servers") {
            None => Vec::new(),
            Some(Value::Array(items)) => items
                .iter()
                .map(ConfigServer::decode)
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err(err("field \"servers\" is not an array")),
        };
        let active_server_id = opt_str(v, "active_server_id")?;
        let initial_setup_done = opt_bool(v, "initial_setup_done", !servers.is_empty())?;

        let remote_api_port = opt_i64(v, "remote_api_port", defaults.remote_api_port)?;
        // Keychain-only; never decoded from JSON.
        let remote_api_token = String::new();
        let remote_api_expose_on_lan = opt_bool(
            v,
            "remote_api_expose_on_lan",
            defaults.remote_api_expose_on_lan,
        )?;

        // Trim; a value that is blank (or trims to blank) becomes `None`
        // rather than an empty string -- matches source lines 764-773.
        let remote_api_preferred_pairing_host = {
            let raw = opt_str(v, "remote_api_preferred_pairing_host")?
                .or_else(|| defaults.remote_api_preferred_pairing_host.clone());
            raw.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        };

        let decoded_shared_access = match present(v, "remote_api_shared_access") {
            None => defaults.remote_api_shared_access.clone(),
            Some(Value::Array(items)) => items
                .iter()
                .map(RemoteApiSharedAccessEntry::decode)
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err(err("field \"remote_api_shared_access\" is not an array")),
        };
        // Normalize: trim label/token, generate a fresh id for a blank one
        // (id itself is trimmed only to test for blankness, never stored
        // trimmed), drop entries whose token is blank after trimming, and
        // dedupe by token keeping the first -- matches source lines
        // 779-792 exactly.
        let mut seen_tokens = HashSet::new();
        let mut remote_api_shared_access = Vec::with_capacity(decoded_shared_access.len());
        for mut entry in decoded_shared_access {
            entry.label = entry.label.trim().to_string();
            entry.token = entry.token.trim().to_string();
            if entry.id.trim().is_empty() {
                entry.id = Uuid::new_v4().to_string().to_uppercase();
            }
            if entry.token.is_empty() {
                continue;
            }
            if !seen_tokens.insert(entry.token.clone()) {
                continue;
            }
            remote_api_shared_access.push(entry);
        }

        let duckdns_hostname =
            opt_str(v, "duckdns_hostname")?.or_else(|| defaults.duckdns_hostname.clone());
        let playit_java_address = opt_str(v, "playit_java_address")?;
        let playit_bedrock_address = opt_str(v, "playit_bedrock_address")?;
        let playit_voice_address = opt_str(v, "playit_voice_address")?;
        let playit_agent_id = opt_str(v, "playit_agent_id")?;

        let has_shown_handbook =
            opt_bool(v, "has_shown_welcome_guide", defaults.has_shown_handbook)?;
        // Default depends on the just-decoded handbook flag, not a static
        // default -- existing users who already saw the old "Welcome
        // Guide" skip the new concept guide automatically (source line
        // 811).
        let has_shown_concept_guide = opt_bool(v, "has_shown_concept_guide", has_shown_handbook)?;

        let xbox_broadcast_jar_path = opt_str(v, "xbox_broadcast_jar_path")?
            .or_else(|| defaults.xbox_broadcast_jar_path.clone());
        let xbox_broadcast_auto_start_enabled = opt_bool(
            v,
            "xbox_broadcast_auto_start_enabled",
            defaults.xbox_broadcast_auto_start_enabled,
        )?;
        let minecraft_username =
            opt_str(v, "minecraft_username")?.or_else(|| defaults.minecraft_username.clone());
        let minecraft_bedrock_gamertag = opt_str(v, "minecraft_bedrock_gamertag")?
            .or_else(|| defaults.minecraft_bedrock_gamertag.clone());
        let minecraft_avatar_edition_raw_value = opt_str(v, "minecraft_avatar_edition")?
            .or_else(|| defaults.minecraft_avatar_edition_raw_value.clone());
        let default_banner_color_hex = opt_str(v, "default_banner_color_hex")?
            .or_else(|| defaults.default_banner_color_hex.clone());
        let error_popups_enabled =
            opt_bool(v, "error_popups_enabled", defaults.error_popups_enabled)?;
        // Literal `true`, not `defaults.save_downloaded_jars` -- matches
        // source line 838 exactly (same value today, but not the same
        // expression).
        let save_downloaded_jars = opt_bool(v, "save_downloaded_jars", true)?;

        let loader_version_records = match present(v, "loader_version_records") {
            None => Vec::new(),
            Some(Value::Array(items)) => items.clone(),
            Some(_) => return Err(err("field \"loader_version_records\" is not an array")),
        };

        Ok(Self {
            config_version,
            java_path,
            extra_flags,
            servers_root,
            plugin_template_dir,
            paper_template_dir,
            servers,
            active_server_id,
            initial_setup_done,
            remote_api_port,
            remote_api_token,
            remote_api_expose_on_lan,
            remote_api_preferred_pairing_host,
            remote_api_shared_access,
            duckdns_hostname,
            playit_java_address,
            playit_bedrock_address,
            playit_voice_address,
            playit_agent_id,
            has_shown_handbook,
            has_shown_concept_guide,
            xbox_broadcast_jar_path,
            xbox_broadcast_auto_start_enabled,
            minecraft_username,
            minecraft_bedrock_gamertag,
            minecraft_avatar_edition_raw_value,
            default_banner_color_hex,
            error_popups_enabled,
            save_downloaded_jars,
            loader_version_records,
            use_vm_bedrock_backend,
        })
    }

    pub fn encode(&self) -> Value {
        let mut m = Map::new();
        m.insert("config_version".into(), Value::from(self.config_version));
        insert_str(&mut m, "java_path", &self.java_path);
        insert_str(&mut m, "extra_flags", &self.extra_flags);
        insert_str(&mut m, "servers_root", &self.servers_root);
        insert_str(&mut m, "plugin_template_dir", &self.plugin_template_dir);
        insert_str(&mut m, "paper_template_dir", &self.paper_template_dir);
        m.insert(
            "servers".into(),
            Value::Array(self.servers.iter().map(ConfigServer::encode).collect()),
        );
        insert_opt_str(&mut m, "active_server_id", &self.active_server_id);
        m.insert(
            "initial_setup_done".into(),
            Value::Bool(self.initial_setup_done),
        );
        m.insert("remote_api_port".into(), Value::from(self.remote_api_port));
        // remote_api_token intentionally omitted -- Keychain-only.
        m.insert(
            "remote_api_expose_on_lan".into(),
            Value::Bool(self.remote_api_expose_on_lan),
        );

        insert_opt_str(
            &mut m,
            "remote_api_preferred_pairing_host",
            &self.remote_api_preferred_pairing_host,
        );
        m.insert(
            "remote_api_shared_access".into(),
            Value::Array(
                self.remote_api_shared_access
                    .iter()
                    .map(RemoteApiSharedAccessEntry::encode)
                    .collect(),
            ),
        );

        insert_opt_str(&mut m, "duckdns_hostname", &self.duckdns_hostname);
        insert_opt_str(&mut m, "playit_java_address", &self.playit_java_address);
        insert_opt_str(
            &mut m,
            "playit_bedrock_address",
            &self.playit_bedrock_address,
        );
        insert_opt_str(&mut m, "playit_voice_address", &self.playit_voice_address);
        insert_opt_str(&mut m, "playit_agent_id", &self.playit_agent_id);

        m.insert(
            "has_shown_welcome_guide".into(),
            Value::Bool(self.has_shown_handbook),
        );
        m.insert(
            "has_shown_concept_guide".into(),
            Value::Bool(self.has_shown_concept_guide),
        );
        insert_opt_str(
            &mut m,
            "xbox_broadcast_jar_path",
            &self.xbox_broadcast_jar_path,
        );
        m.insert(
            "xbox_broadcast_auto_start_enabled".into(),
            Value::Bool(self.xbox_broadcast_auto_start_enabled),
        );
        insert_opt_str(&mut m, "minecraft_username", &self.minecraft_username);
        insert_opt_str(
            &mut m,
            "minecraft_bedrock_gamertag",
            &self.minecraft_bedrock_gamertag,
        );
        insert_opt_str(
            &mut m,
            "default_banner_color_hex",
            &self.default_banner_color_hex,
        );
        insert_opt_str(
            &mut m,
            "minecraft_avatar_edition",
            &self.minecraft_avatar_edition_raw_value,
        );
        m.insert(
            "error_popups_enabled".into(),
            Value::Bool(self.error_popups_enabled),
        );
        m.insert(
            "save_downloaded_jars".into(),
            Value::Bool(self.save_downloaded_jars),
        );
        if !self.loader_version_records.is_empty() {
            m.insert(
                "loader_version_records".into(),
                Value::Array(self.loader_version_records.clone()),
            );
        }
        // use_vm_bedrock_backend is deliberately never encoded: the source
        // decodes it (line 726-728) but `encode(to:)` has no line for it
        // at all, so it never round-trips through JSON. Preserved as-is,
        // not "fixed".

        Value::Object(m)
    }
}
