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
}

impl fmt::Display for BedrockSettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::AtomicWrite(error) => write!(f, "{error}"),
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
    const BOOLEAN_KEYS: &[&str] = &["online-mode", "allow-cheats"];
    const INTEGER_KEYS: &[&str] = &["max-players", "server-port", "server-portv6"];
    const ENUM_KEYS: &[(&str, &[&str])] = &[
        ("difficulty", &["peaceful", "easy", "normal", "hard"]),
        (
            "gamemode",
            &["survival", "creative", "adventure", "spectator"],
        ),
    ];

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
    ]
    .into_iter()
    .collect()
}
