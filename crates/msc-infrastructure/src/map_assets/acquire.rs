//! Exact published identities only; no latest-version, filename, or project substitution.
use super::*;
use crate::addon_provider::{self as provider, AddonTransport};
use crate::download_staging::{md5_hex, sha1_hex, sha512_hex};
use crate::secret_store::SecretStore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequiredSource {
    pub identity: String,
    pub path: String,
    pub provider: String,
    pub project_id: Option<String>,
    pub release_id: Option<String>,
    pub file_id: Option<String>,
    pub hashes: BTreeMap<String, String>,
    pub bytes: u64,
    pub urls: Vec<String>,
    pub reason: Option<String>,
}
impl RequiredSource {
    pub fn validate(&self, bytes: &[u8]) -> bool {
        !self.hashes.is_empty()
            && self.bytes > 0
            && self.bytes <= MAX_ARCHIVE
            && bytes.len() as u64 == self.bytes
            && self.hashes.iter().all(|(algorithm, expected)| {
                let actual = match algorithm.as_str() {
                    "sha1" => sha1_hex(bytes),
                    "sha256" => hash(bytes),
                    "sha512" => sha512_hex(bytes),
                    "md5" => md5_hex(bytes),
                    _ => return false,
                };
                actual.eq_ignore_ascii_case(expected)
            })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingSource {
    pub source: Box<RequiredSource>,
    pub code: String,
    pub remedy: String,
}

pub fn requests(
    root: &Path,
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    game: &str,
    loader: &str,
    loader_version: Option<&str>,
    cancel: &dyn Fn() -> bool,
) -> io::Result<Vec<RequiredSource>> {
    let mr = root.join("modrinth.index.json");
    if mr.exists() {
        let value: serde_json::Value = serde_json::from_slice(&read(&mr, MAX_JSON)?)
            .map_err(|_| error("invalid_map_source_manifest"))?;
        if value["dependencies"]["minecraft"].as_str() != Some(game) {
            return Err(error("map_source_game_mismatch"));
        }
        let loader_key = match loader {
            "fabric" => "fabric-loader",
            "neoforge" => "neoforge",
            "forge" => "forge",
            _ => "",
        };
        if !loader_key.is_empty()
            && (value["dependencies"][loader_key].as_str().is_none()
                || loader_version
                    .is_some_and(|v| value["dependencies"][loader_key].as_str() != Some(v)))
        {
            return Err(error("map_source_loader_mismatch"));
        }
        let files = value["files"]
            .as_array()
            .ok_or_else(|| error("invalid_map_source_manifest"))?;
        if files.len() > 10_000 {
            return Err(error("inventory_source_limit"));
        }
        let mut result = Vec::new();
        for file in files {
            poll(cancel)?;
            let path = file["path"]
                .as_str()
                .ok_or_else(|| error("invalid_map_source_manifest"))?;
            if !safe_member(path) {
                return Err(error("unsafe_map_source_path"));
            }
            if !(path.starts_with("mods/") && path.ends_with(".jar")
                || path.starts_with("resourcepacks/") && path.ends_with(".zip"))
            {
                continue;
            }
            if matches!(
                file["env"]["client"].as_str(),
                Some("unsupported" | "optional")
            ) {
                continue;
            }
            let hashes: BTreeMap<String, String> = serde_json::from_value(file["hashes"].clone())
                .map_err(|_| error("missing_published_hash"))?;
            let bytes = file["fileSize"]
                .as_u64()
                .ok_or_else(|| error("missing_published_size"))?;
            let urls: Vec<String> = serde_json::from_value(file["downloads"].clone())
                .map_err(|_| error("invalid_map_source_manifest"))?;
            result.push(RequiredSource {
                identity: hash_json(&(path, &hashes, bytes))?,
                path: path.into(),
                provider: "modrinth_manifest".into(),
                project_id: None,
                release_id: None,
                file_id: None,
                hashes,
                bytes,
                urls,
                reason: None,
            });
        }
        return Ok(result);
    }
    let cf = root.join("manifest.json");
    if !cf.exists() {
        return Ok(vec![]);
    }
    let raw = read(&cf, MAX_JSON)?;
    let text = std::str::from_utf8(&raw).map_err(|_| error("invalid_map_source_manifest"))?;
    let manifest = msc_domain::modpack_manifest::parse_curseforge_metadata(text)
        .map_err(|_| error("invalid_map_source_manifest"))?;
    if manifest.minecraft_version != game {
        return Err(error("map_source_game_mismatch"));
    }
    if manifest
        .loader_flavor
        .is_some_and(|f| !f.label().eq_ignore_ascii_case(loader))
        || loader_version.is_some_and(|v| manifest.loader_version.as_deref() != Some(v))
    {
        return Err(error("map_source_loader_mismatch"));
    }
    let ids = manifest
        .files
        .iter()
        .filter(|f| f.required)
        .map(|f| f.file_id)
        .collect::<Vec<_>>();
    let response = provider::curseforge_files(transport, secrets, &ids);
    let mut result = Vec::new();
    for file in manifest.files.iter().filter(|f| f.required) {
        poll(cancel)?;
        let resolved = response.as_ref().ok().and_then(|r| {
            r.iter()
                .find(|r| r.id == file.file_id && r.mod_id == file.project_id)
        });
        let hashes = resolved
            .map(|r| {
                r.hashes
                    .iter()
                    .filter_map(|h| match h.algo {
                        1 => Some(("sha1".into(), h.value.clone())),
                        2 => Some(("md5".into(), h.value.clone())),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        result.push(RequiredSource {
            identity: format!("curseforge:{}:{}", file.project_id, file.file_id),
            path: resolved
                .map(|r| format!("mods/{}", r.file_name))
                .unwrap_or_else(|| format!("mods/file-{}.jar", file.file_id)),
            provider: "curseforge".into(),
            project_id: Some(file.project_id.to_string()),
            release_id: None,
            file_id: Some(file.file_id.to_string()),
            hashes,
            bytes: resolved.map_or(0, |r| r.file_length),
            urls: resolved
                .and_then(|r| r.download_url.clone())
                .into_iter()
                .collect(),
            reason: if resolved.is_none() {
                Some("provider_identity_unavailable".into())
            } else {
                None
            },
        });
    }
    Ok(result)
}

pub fn allowed_download(url: &str) -> bool {
    let Ok(uri) = url.parse::<ureq::http::Uri>() else {
        return false;
    };
    uri.scheme_str() == Some("https")
        && uri.authority().is_some_and(|a| {
            matches!(
                a.as_str(),
                "cdn.modrinth.com" | "edge.forgecdn.net" | "mediafilez.forgecdn.net"
            )
        })
}

/// Verified completed downloads survive a later provider failure; partial bytes never do.
pub fn acquire(
    source: &RequiredSource,
    server: &Path,
    cache: &Path,
    transport: &dyn AddonTransport,
    offline: bool,
    cancel: &dyn Fn() -> bool,
) -> Result<PathBuf, MissingSource> {
    let missing = |code: &str| {
        MissingSource { source:Box::new(source.clone()),code:code.into(),remedy:"Import the exact file with these published hashes and size through the matching-client resource importer.".into() }
    };
    if !safe_member(&source.path) {
        return Err(missing("unsafe_map_source_path"));
    }
    let key = hash_json(&(source.identity.clone(), &source.hashes, source.bytes))
        .map_err(|_| missing("invalid_source_identity"))?;
    let destination = cache.join(key);
    let installed = server.join(&source.path);
    let disabled = server.join(format!("{}.disabled", source.path));
    for path in [&installed, &disabled, &destination] {
        poll(cancel).map_err(|_| missing("cancelled"))?;
        if path.exists()
            && let Ok(bytes) = read(path, MAX_ARCHIVE)
            && source.validate(&bytes)
        {
            return Ok(path.to_path_buf());
        }
    }
    if let Some(code) = &source.reason {
        return Err(missing(code));
    }
    if source.urls.is_empty() {
        return Err(missing("manual_download_required"));
    }
    if offline {
        return Err(missing("offline"));
    }
    if source.bytes == 0 || source.bytes > MAX_ARCHIVE || source.hashes.is_empty() {
        return Err(missing("missing_published_identity"));
    }
    fs::create_dir_all(cache).map_err(|_| missing("content_store_unavailable"))?;
    safe_path(cache).map_err(|_| missing("content_store_unavailable"))?;
    if source.urls.len() > 8 {
        return Err(missing("source_mirror_limit"));
    }
    let mut total = 0u64;
    let entries = fs::read_dir(cache).map_err(|_| missing("content_store_unavailable"))?;
    for (count, entry) in entries.enumerate() {
        if count >= 10_000 {
            return Err(missing("content_store_quota"));
        }
        let entry = entry.map_err(|_| missing("content_store_unavailable"))?;
        let m =
            fs::symlink_metadata(entry.path()).map_err(|_| missing("content_store_unavailable"))?;
        if !m.is_file() {
            return Err(missing("linked_content_store"));
        }
        total = total.saturating_add(m.len());
    }
    if total.saturating_add(source.bytes) > 16 * 1024 * 1024 * 1024
        || fs2::available_space(cache).unwrap_or(0) < source.bytes + 64 * 1024 * 1024
    {
        return Err(missing("content_store_quota"));
    }
    let mut code = "provider_unavailable";
    for url in &source.urls {
        poll(cancel).map_err(|_| missing("cancelled"))?;
        if !allowed_download(url) {
            code = "unapproved_download_host";
            continue;
        }
        let Ok(response) = transport.get(url, "Exact map resource", &[], source.bytes) else {
            continue;
        };
        if response.status != 200 {
            code = if matches!(response.status, 401 | 403) {
                "provider_access_required"
            } else {
                "provider_unavailable"
            };
            continue;
        }
        if !source.validate(&response.body) {
            code = "published_checksum_mismatch";
            continue;
        }
        poll(cancel).map_err(|_| missing("cancelled"))?;
        let temp = cache.join(format!(".{}", uuid::Uuid::new_v4()));
        // No server file is written, enabled or replaced by this consumer.
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            use std::io::Write;
            file.write_all(&response.body)?;
            file.sync_all()?;
            fs::rename(&temp, &destination)
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
            return Err(missing("content_store_unavailable"));
        }
        return Ok(destination);
    }
    Err(missing(code))
}
