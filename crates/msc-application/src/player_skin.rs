//! Durable player-skin lookup overrides.
//!
//! The agent owns the HTTP route, while this module owns the small JSON
//! sidecar that persists the operator's chosen lookup identifier. Keeping the
//! file operation here makes the route easy to test without a running server
//! or a real filesystem.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use msc_infrastructure::atomic_write::atomic_write;
use msc_infrastructure::fs::FileSystem;
use serde::{Deserialize, Serialize};

const OVERRIDES_FILE: &str = "player_overrides.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSkinOverride {
    pub lookup_identifier: Option<String>,
    pub skin_file_name: Option<String>,
}

pub type PlayerSkinOverrides = BTreeMap<String, PlayerSkinOverride>;

/// Loads MSC 1's server-root override sidecar. Missing or malformed files
/// behave like MSC 1's `try?` decode: no override is safer than refusing the
/// whole player list because an optional sidecar is damaged.
pub fn load_overrides(fs: &dyn FileSystem, server_dir: &Path) -> PlayerSkinOverrides {
    let Ok(bytes) = fs.read(&server_dir.join(OVERRIDES_FILE)) else {
        return BTreeMap::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

/// Sets or clears one profile's lookup identifier and writes the complete
/// sidecar atomically. A future skin filename is preserved if one ever exists;
/// this step deliberately never creates one because the frozen request has no
/// skin-file upload field.
pub fn set_lookup_override(
    fs: &dyn FileSystem,
    server_dir: &Path,
    profile_id: &str,
    lookup_identifier: Option<String>,
) -> io::Result<Option<String>> {
    let mut overrides = load_overrides(fs, server_dir);
    match lookup_identifier {
        Some(identifier) => {
            overrides
                .entry(profile_id.to_owned())
                .or_default()
                .lookup_identifier = Some(identifier.clone());
            write_overrides(fs, server_dir, &overrides)?;
            Ok(Some(identifier))
        }
        None => {
            if let Some(override_value) = overrides.get_mut(profile_id) {
                override_value.lookup_identifier = None;
                if override_value.skin_file_name.is_none() {
                    overrides.remove(profile_id);
                }
            }
            write_overrides(fs, server_dir, &overrides)?;
            Ok(None)
        }
    }
}

fn write_overrides(
    fs: &dyn FileSystem,
    server_dir: &Path,
    overrides: &PlayerSkinOverrides,
) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(overrides).map_err(io::Error::other)?;
    atomic_write(fs, &server_dir.join(OVERRIDES_FILE), &bytes)
        .map_err(|error| io::Error::other(error.to_string()))
}
