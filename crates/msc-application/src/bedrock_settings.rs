//! Bedrock `server.properties` service.
//!
//! The domain model deliberately preserves unknown BDS keys.  This module
//! adds the application concerns around that model: validating user edits,
//! merging them into the existing file, and swapping the result through the
//! shared atomic-write primitive.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io;
use std::path::Path;

use msc_domain::bedrock::{BedrockPropertiesModel, parse_raw_properties, render_raw_properties};
use msc_infrastructure::atomic_write::atomic_write;
use msc_infrastructure::fs::FileSystem;

const PROPERTIES_FILE: &str = "server.properties";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BedrockSettings {
    pub model: BedrockPropertiesModel,
    pub raw: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingRejection {
    pub key: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsUpdate {
    pub applied_keys: Vec<String>,
    pub rejected: Vec<SettingRejection>,
    pub settings: BedrockSettings,
}

#[derive(Debug)]
pub enum BedrockSettingsError {
    Io(io::Error),
    AtomicWrite(String),
    InvalidPort(String),
    InvalidNethernetMapping(String),
}

impl fmt::Display for BedrockSettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::AtomicWrite(error) => write!(f, "{error}"),
            Self::InvalidPort(error) => write!(f, "{error}"),
            Self::InvalidNethernetMapping(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for BedrockSettingsError {}

impl From<io::Error> for BedrockSettingsError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn load(fs: &dyn FileSystem, server_dir: &Path) -> BedrockSettings {
    let raw = fs
        .read(&server_dir.join(PROPERTIES_FILE))
        .ok()
        .map(|bytes| parse_raw_properties(&String::from_utf8_lossy(&bytes)))
        .unwrap_or_default();
    BedrockSettings {
        model: BedrockPropertiesModel::from_raw(&raw),
        raw,
    }
}

/// BDS 1.26.51 accepts at most sixteen individual NetherNet mappings.
pub const NETHERNET_UDP_PORT_COUNT: u16 = 16;

/// Run the macOS VM through the legacy single-port transport as a bounded
/// compatibility fallback for BDS 1.26.51's broken remote TLS handshake.
/// The NetherNet-only keys must go away as a pair: leaving either one behind
/// changes how BDS advertises and selects its player path.
pub fn ensure_sidecar_raknet_transport(
    fs: &dyn FileSystem,
    server_dir: &Path,
) -> Result<bool, BedrockSettingsError> {
    let current = load(fs, server_dir);
    let already_configured = current
        .raw
        .get("transport")
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("raknet"))
        && !current.raw.contains_key("server-udp-ports")
        && !current.raw.contains_key("server-ip");
    if already_configured {
        return Ok(false);
    }

    let mut raw = current.raw;
    raw.insert("transport".to_owned(), "raknet".to_owned());
    raw.remove("server-udp-ports");
    raw.remove("server-ip");
    atomic_write(
        fs,
        &server_dir.join(PROPERTIES_FILE),
        render_raw_properties(&raw).as_bytes(),
    )
    .map_err(|error| BedrockSettingsError::AtomicWrite(error.to_string()))?;
    Ok(true)
}

/// Configure the macOS VM for NetherNet's TCP signaling socket and bounded
/// UDP gameplay range. Advertised addresses name the host-side relays rather
/// than the guest's private VZ address. BDS 1.26.51 is more reliable when the
/// NAT mapping is enumerated one port at a time instead of using range syntax.
pub fn ensure_sidecar_nethernet_transport(
    fs: &dyn FileSystem,
    server_dir: &Path,
    server_port: u16,
    advertised_addresses: &[String],
) -> Result<bool, BedrockSettingsError> {
    let address = parse_advertised_address(advertised_addresses)?;
    let udp_ports = render_nethernet_udp_mappings(server_port, address)?;
    validate_nethernet_udp_mappings(&udp_ports, server_port, address)?;

    let changed = ensure_properties(
        fs,
        server_dir,
        &[("transport", "nethernet"), ("server-udp-ports", &udp_ports)],
    )?;

    let configured = load(fs, server_dir)
        .raw
        .get("server-udp-ports")
        .cloned()
        .ok_or_else(|| {
            BedrockSettingsError::InvalidNethernetMapping(
                "Bedrock server.properties is missing server-udp-ports after NetherNet configuration"
                    .to_owned(),
            )
        })?;
    validate_nethernet_udp_mappings(&configured, server_port, address)?;
    Ok(changed)
}

/// Keep native Linux and Windows BDS installations on their supported
/// default transport without imposing the macOS sidecar's UDP range.
pub fn ensure_nethernet_transport(
    fs: &dyn FileSystem,
    server_dir: &Path,
) -> Result<bool, BedrockSettingsError> {
    ensure_properties(fs, server_dir, &[("transport", "nethernet")])
}

pub fn nethernet_udp_port_range(server_port: u16) -> Option<(u16, u16)> {
    if server_port <= u16::MAX - NETHERNET_UDP_PORT_COUNT {
        Some((server_port + 1, server_port + NETHERNET_UDP_PORT_COUNT))
    } else if server_port > NETHERNET_UDP_PORT_COUNT {
        Some((server_port - NETHERNET_UDP_PORT_COUNT, server_port - 1))
    } else {
        None
    }
}

fn parse_advertised_address(
    advertised_addresses: &[String],
) -> Result<std::net::IpAddr, BedrockSettingsError> {
    let [address] = advertised_addresses else {
        return Err(BedrockSettingsError::InvalidNethernetMapping(format!(
            "NetherNet requires exactly one public or LAN advertised IP address; discovered {}",
            advertised_addresses.len()
        )));
    };
    let address = address.trim().parse::<std::net::IpAddr>().map_err(|_| {
        BedrockSettingsError::InvalidNethernetMapping(format!(
            "NetherNet advertised address is not a valid IP address: {address:?}"
        ))
    })?;
    if address.is_unspecified() {
        return Err(BedrockSettingsError::InvalidNethernetMapping(
            "NetherNet advertised address cannot be an unspecified address".to_owned(),
        ));
    }
    Ok(address)
}

fn render_nethernet_udp_mappings(
    server_port: u16,
    address: std::net::IpAddr,
) -> Result<String, BedrockSettingsError> {
    let (udp_start, udp_end) = nethernet_udp_port_range(server_port).ok_or_else(|| {
        BedrockSettingsError::InvalidPort(format!(
            "port {server_port} leaves no room for the NetherNet UDP relay range"
        ))
    })?;
    if server_port == 0 {
        return Err(BedrockSettingsError::InvalidPort(
            "NetherNet signaling port must be between 1 and 65535".to_owned(),
        ));
    }

    let mappings = (udp_start..=udp_end)
        .map(|port| match address {
            std::net::IpAddr::V4(address) => format!("{address}:{port}:{port}"),
            std::net::IpAddr::V6(address) => format!("[{address}]:{port}:{port}"),
        })
        .collect::<Vec<_>>();
    Ok(mappings.join(","))
}

fn validate_nethernet_udp_mappings(
    configured: &str,
    server_port: u16,
    address: std::net::IpAddr,
) -> Result<(), BedrockSettingsError> {
    let expected = render_nethernet_udp_mappings(server_port, address)?;
    let mapping_count = configured
        .split(',')
        .filter(|mapping| !mapping.trim().is_empty())
        .count();
    if mapping_count != usize::from(NETHERNET_UDP_PORT_COUNT) || configured != expected {
        let (udp_start, udp_end) = nethernet_udp_port_range(server_port).ok_or_else(|| {
            BedrockSettingsError::InvalidPort(format!(
                "port {server_port} leaves no room for the NetherNet UDP relay range"
            ))
        })?;
        return Err(BedrockSettingsError::InvalidNethernetMapping(format!(
            "server-udp-ports must contain {NETHERNET_UDP_PORT_COUNT} individual public-to-private mappings for {address} covering UDP {udp_start}-{udp_end}; found {mapping_count} mappings"
        )));
    }
    Ok(())
}

fn ensure_properties(
    fs: &dyn FileSystem,
    server_dir: &Path,
    expected: &[(&str, &str)],
) -> Result<bool, BedrockSettingsError> {
    let current = load(fs, server_dir);
    let already_configured = expected.iter().all(|(key, expected_value)| {
        current.raw.get(*key).is_some_and(|value| {
            if *key == "server-udp-ports" {
                value == *expected_value
            } else {
                value.trim().eq_ignore_ascii_case(expected_value)
            }
        })
    });
    if already_configured {
        return Ok(false);
    }

    let mut raw = current.raw;
    for (key, value) in expected {
        raw.insert((*key).to_string(), (*value).to_string());
    }
    atomic_write(
        fs,
        &server_dir.join(PROPERTIES_FILE),
        render_raw_properties(&raw).as_bytes(),
    )
    .map_err(|error| BedrockSettingsError::AtomicWrite(error.to_string()))?;
    Ok(true)
}

/// Validate and apply a sparse update. Invalid fields are rejected without
/// changing the candidate, so a mixed request can safely save only its valid
/// subset while retaining the prior file for every rejected value.
pub fn update(
    fs: &dyn FileSystem,
    server_dir: &Path,
    changes: &BTreeMap<String, String>,
) -> Result<SettingsUpdate, BedrockSettingsError> {
    let current = load(fs, server_dir);
    let mut raw = current.raw.clone();
    let mut applied_keys = Vec::new();
    let mut rejected = Vec::new();

    for (key, value) in changes {
        match validate_change(key, value) {
            Ok(()) => {
                raw.insert(key.clone(), value.clone());
                applied_keys.push(key.clone());
            }
            Err(reason) => rejected.push(SettingRejection {
                key: key.clone(),
                reason: reason.to_owned(),
            }),
        }
    }

    if !applied_keys.is_empty() {
        atomic_write(
            fs,
            &server_dir.join(PROPERTIES_FILE),
            render_raw_properties(&raw).as_bytes(),
        )
        .map_err(|error| BedrockSettingsError::AtomicWrite(error.to_string()))?;
    }

    Ok(SettingsUpdate {
        applied_keys,
        rejected,
        settings: BedrockSettings {
            model: BedrockPropertiesModel::from_raw(&raw),
            raw,
        },
    })
}

fn validate_change(key: &str, value: &str) -> Result<(), &'static str> {
    const BOOLEAN_KEYS: &[&str] = &[
        "online-mode",
        "allow-cheats",
        "allow-list",
        "enable-lan-visibility",
        "disable-custom-skins",
        "disable-player-interaction",
        "content-log-file-enabled",
    ];
    const INTEGER_KEYS: &[&str] = &[
        "max-players",
        "server-port",
        "server-portv6",
        "player-idle-timeout",
        "view-distance",
        "tick-distance",
        "max-threads",
        "compression-threshold",
    ];
    const ENUM_KEYS: &[(&str, &[&str])] = &[
        ("difficulty", &["peaceful", "easy", "normal", "hard"]),
        (
            "gamemode",
            &["survival", "creative", "adventure", "spectator"],
        ),
        (
            "default-player-permission-level",
            &["visitor", "member", "operator"],
        ),
        ("chat-restriction", &["None", "Dropped", "Disabled"]),
        ("compression-algorithm", &["zlib", "snappy"]),
    ];

    if key == "server-name" {
        return (!value.trim().is_empty() && !value.contains(';'))
            .then_some(())
            .ok_or("server-name must be non-empty and cannot contain a semicolon");
    }
    if key == "level-name" {
        return (!value.trim().is_empty())
            .then_some(())
            .ok_or("level-name cannot be empty");
    }
    if BOOLEAN_KEYS.contains(&key) {
        return (value == "true" || value == "false")
            .then_some(())
            .ok_or("value must be true or false");
    }
    if INTEGER_KEYS.contains(&key) {
        let parsed = value
            .parse::<i64>()
            .map_err(|_| "value must be an integer")?;
        if matches!(key, "server-port" | "server-portv6") && !(1..=65_535).contains(&parsed) {
            return Err("port must be between 1 and 65535");
        }
        match key {
            "max-players" if parsed < 1 => return Err("max-players must be positive"),
            "player-idle-timeout" if parsed < 0 => {
                return Err("player-idle-timeout cannot be negative");
            }
            "view-distance" if parsed < 5 => {
                return Err("view-distance must be at least 5");
            }
            "tick-distance" if !(4..=12).contains(&parsed) => {
                return Err("tick-distance must be between 4 and 12");
            }
            "max-threads" if parsed < 0 => return Err("max-threads cannot be negative"),
            "compression-threshold" if !(0..=65_535).contains(&parsed) => {
                return Err("compression-threshold must be between 0 and 65535");
            }
            _ => {}
        }
        return Ok(());
    }
    if let Some((_, options)) = ENUM_KEYS.iter().find(|(known, _)| *known == key) {
        return options
            .contains(&value)
            .then_some(())
            .ok_or("value is not a recognized Bedrock option");
    }
    // BDS grows new keys over time. Unknown keys are valid and are retained.
    Ok(())
}

/// Keys that the settings surface describes as editable. Unknown keys still
/// survive a round trip, but are not advertised as typed controls.
pub fn editable_keys() -> BTreeSet<&'static str> {
    [
        "level-name",
        "max-players",
        "online-mode",
        "allow-cheats",
        "difficulty",
        "gamemode",
        "server-port",
        "server-portv6",
        "server-name",
        "allow-list",
        "default-player-permission-level",
        "enable-lan-visibility",
        "player-idle-timeout",
        "view-distance",
        "tick-distance",
        "max-threads",
        "disable-custom-skins",
        "chat-restriction",
        "compression-threshold",
        "compression-algorithm",
        "disable-player-interaction",
        "content-log-file-enabled",
    ]
    .into_iter()
    .collect()
}
