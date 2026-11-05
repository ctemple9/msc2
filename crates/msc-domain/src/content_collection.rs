//! Durable content provenance is separate from the legacy Modrinth link map.
//! Missing collections remain absent on older servers: installed files alone
//! cannot establish an upstream baseline or reconstruct client-only content.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSource {
    pub provider: String,
    pub project_id: String,
    pub version_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentAudience {
    Server,
    Players,
    Both,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentEntry {
    pub path: String,
    pub sha512: String,
    pub byte_size: u64,
    pub audience: ContentAudience,
    pub optional: bool,
    pub source: Option<ContentSource>,
    /// Absence means unresolved, not a permission to redistribute.
    pub redistribution_evidence: Option<String>,
    pub classification_evidence: Vec<String>,
    pub unresolved_reasons: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentCollection {
    pub schema_version: u32,
    pub revision: u64,
    pub upstream: Option<ContentSource>,
    pub upstream_files: BTreeMap<String, ContentEntry>,
    pub current_files: BTreeMap<String, ContentEntry>,
    /// Client and server overrides may have different bytes at the same game path.
    #[serde(default)]
    pub player_files: BTreeMap<String, ContentEntry>,
    #[serde(default)]
    pub upstream_player_files: BTreeMap<String, ContentEntry>,
    /// Preserve intentional owner removals when comparing future upstream packs.
    pub excluded_paths: Vec<String>,
    #[serde(default)]
    pub player_excluded_paths: Vec<String>,
    /// Paths relative to the registered server root, never arbitrary host paths.
    pub retained_roots: Vec<String>,
    pub publication_ids: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for ContentCollection {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            upstream: None,
            upstream_files: BTreeMap::new(),
            current_files: BTreeMap::new(),
            player_files: BTreeMap::new(),
            upstream_player_files: BTreeMap::new(),
            excluded_paths: Vec::new(),
            player_excluded_paths: Vec::new(),
            retained_roots: Vec::new(),
            publication_ids: Vec::new(),
            extra: BTreeMap::new(),
        }
    }
}

impl ContentCollection {
    /// Validate portable archive paths before persisted metadata can direct I/O.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "unsupported content schema {}",
                self.schema_version
            ));
        }
        for files in [
            &self.upstream_files,
            &self.current_files,
            &self.player_files,
            &self.upstream_player_files,
        ] {
            let mut folded = std::collections::BTreeSet::new();
            for (path, entry) in files {
                validate_relative_path(path)?;
                if path != &entry.path {
                    return Err("content entry path differs from its collection key".into());
                }
                if !folded.insert(path.to_lowercase()) {
                    return Err("content paths collide on case-insensitive filesystems".into());
                }
                if entry.sha512.len() != 128
                    || !entry.sha512.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return Err("content requires an exact SHA-512 identity".into());
                }
            }
        }
        for path in self
            .excluded_paths
            .iter()
            .chain(&self.player_excluded_paths)
        {
            validate_relative_path(path)?;
        }
        for path in &self.retained_roots {
            validate_relative_path(path)?;
            if path != ".msc2-content" && !path.starts_with(".msc2-content/") {
                return Err("retained content must stay in .msc2-content".into());
            }
        }
        for id in &self.publication_ids {
            if id.is_empty()
                || !id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            {
                return Err("invalid publication identity".into());
            }
        }
        Ok(())
    }
}

pub fn validate_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.contains(['\\', ':'])
        || path.chars().any(char::is_control)
        || path.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || part.ends_with(['.', ' '])
                || matches!(
                    part.split('.')
                        .next()
                        .unwrap_or("")
                        .to_ascii_uppercase()
                        .as_str(),
                    "CON"
                        | "PRN"
                        | "AUX"
                        | "NUL"
                        | "COM1"
                        | "COM2"
                        | "COM3"
                        | "COM4"
                        | "COM5"
                        | "COM6"
                        | "COM7"
                        | "COM8"
                        | "COM9"
                        | "LPT1"
                        | "LPT2"
                        | "LPT3"
                        | "LPT4"
                        | "LPT5"
                        | "LPT6"
                        | "LPT7"
                        | "LPT8"
                        | "LPT9"
                )
        })
    {
        return Err(format!("unsafe content path: {path:?}"));
    }
    Ok(())
}
