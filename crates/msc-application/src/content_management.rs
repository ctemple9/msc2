//! Editable player content is independent of files used by the running server.
//! Reviews pin both metadata and current server bytes so an intervening install
//! cannot silently turn yesterday's player selection into today's export.
use crate::content_install::managed_path;
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::content_collection::{
    ContentAudience, ContentCollection, ContentEntry, validate_relative_path,
};
use msc_infrastructure::addon_provider::fingerprint::identify;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    pub path: String,
    pub state: String,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub collection: ContentCollection,
    pub server_baseline: BTreeMap<String, String>,
    pub requirements: Vec<Requirement>,
}

/// Never recursively collect the server directory. Additional config/scripts
/// enter the collection only through an explicit reviewed owner selection.
pub fn content_path(path: &str) -> Result<(), String> {
    validate_relative_path(path)?;
    let (folder, name) = path.split_once('/').ok_or("Choose a content folder")?;
    if name.is_empty() || path.split('/').any(|part| part.starts_with('.')) {
        return Err("Hidden files are not shareable content".into());
    }
    if !matches!(
        folder,
        "mods"
            | "plugins"
            | "resourcepacks"
            | "shaderpacks"
            | "config"
            | "scripts"
            | "kubejs"
            | "defaultconfigs"
            | "datapacks"
    ) {
        return Err("Choose a supported content folder".into());
    }
    if matches!(folder, "mods" | "plugins") && (name.contains('/') || !name.ends_with(".jar")) {
        return Err("Mods and plugins must be individual JAR files".into());
    }
    let lower = path.to_ascii_lowercase();
    if [
        "token",
        "credential",
        "secret",
        "password",
        "private_key",
        ".pem",
        ".key",
        "session",
        "account",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return Err("Credential or account files cannot be selected as content".into());
    }
    Ok(())
}
fn fresh_entry(
    path: &str,
    identity: &msc_infrastructure::addon_provider::fingerprint::FileIdentity,
) -> ContentEntry {
    ContentEntry {
        path: path.into(),
        sha512: identity.sha512.clone(),
        byte_size: identity.byte_size,
        audience: ContentAudience::Both,
        optional: false,
        source: None,
        redistribution_evidence: None,
        classification_evidence: Vec::new(),
        unresolved_reasons: vec![
            "Review client/server classification and player requirements".into(),
        ],
        extra: BTreeMap::new(),
    }
}

pub fn snapshot(server: &ConfigServer) -> Result<Snapshot, String> {
    let mut collection = server.content_collection.clone().unwrap_or_default();
    let mut baseline = BTreeMap::new();
    if let Some(kind) = server.java_flavor.add_on_kind() {
        let folder = kind.folder_name();
        let directory = managed_path(&server.server_dir, folder)?;
        let entries = match std::fs::read_dir(directory) {
            Ok(entries) => Some(entries),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        if let Some(entries) = entries {
            for (index, entry) in entries.enumerate() {
                if index >= 10_000 {
                    return Err("Too many server add-ons to review".into());
                }
                let entry = entry.map_err(|error| error.to_string())?;
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "Non-Unicode add-on name")?;
                if name.ends_with(".jar.disabled") {
                    let path = format!("{folder}/{name}");
                    let identity = identify(&managed_path(&server.server_dir, &path)?)?;
                    baseline.insert(path, identity.sha512);
                    continue;
                }
                if !name.ends_with(".jar") {
                    continue;
                }
                let path = format!("{folder}/{name}");
                content_path(&path)?;
                let identity = identify(&managed_path(&server.server_dir, &path)?)?;
                baseline.insert(path.clone(), identity.sha512.clone());
                let previous = collection.current_files.get(&path);
                if !previous.is_some_and(|previous| previous.sha512 == identity.sha512) {
                    let mut discovered = fresh_entry(&path, &identity);
                    if folder == "plugins" {
                        discovered.audience = ContentAudience::Server;
                        discovered
                            .classification_evidence
                            .push("Plugin runs on the server".into());
                        discovered.unresolved_reasons.clear();
                    }
                    collection.current_files.insert(path, discovered);
                }
            }
        }
    }
    for (path, entry) in &mut collection.current_files {
        if baseline.contains_key(path) {
            continue;
        }
        let actual = managed_path(&server.server_dir, path).and_then(|path| identify(&path));
        match actual {
            Ok(identity) => {
                baseline.insert(path.clone(), identity.sha512.clone());
                if identity.sha512 != entry.sha512 {
                    *entry = fresh_entry(path, &identity);
                }
            }
            Err(_) if baseline.get(&format!("{path}.disabled")) == Some(&entry.sha512) => {
                entry
                    .extra
                    .insert("serverEnabled".into(), serde_json::Value::Bool(false));
            }
            Err(_) => {
                entry
                    .unresolved_reasons
                    .push("Server file is missing or unreadable".into());
            }
        }
    }
    let mut requirements = Vec::new();
    for (path, entry) in &collection.current_files {
        let (state, reason) = if (path.ends_with(".jar.disabled") && baseline.contains_key(path))
            || baseline.get(&format!("{path}.disabled")) == Some(&entry.sha512)
        {
            (
                "serverDisabled",
                "Server content is disabled; existing player choices are preserved",
            )
        } else if !baseline.contains_key(path) {
            (
                "serverMissing",
                "The server file is absent; review removal or restore it",
            )
        } else if entry.classification_evidence.is_empty() {
            ("review", "Choose whether players need this server content")
        } else if entry.audience == ContentAudience::Server {
            (
                "serverOnly",
                "Confirmed server-only content does not belong in the player pack",
            )
        } else if let Some(player) = collection.player_files.get(path) {
            if player
                .extra
                .get("enabled")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
                && path.starts_with("mods/")
            {
                (
                    "playerDisabled",
                    "Players need this mod but it is disabled in the player selection",
                )
            } else if player.sha512 == entry.sha512 {
                ("matched", "Player and server content match")
            } else {
                (
                    "different",
                    "Keep the player customization or review the new server version",
                )
            }
        } else if entry.optional {
            (
                "optionalMissing",
                "Optional content is not selected for players",
            )
        } else {
            (
                "missing",
                "Required player content is absent; review copying the server file",
            )
        };
        requirements.push(Requirement {
            path: path.clone(),
            state: state.into(),
            reason: reason.into(),
        });
    }
    for (path, entry) in &collection.player_files {
        if let Some(source) = &entry.source {
            for (key, state, reason) in [
                (
                    "requiredProjects",
                    "dependencyMissing",
                    "Required player dependency is missing",
                ),
                (
                    "incompatibleProjects",
                    "dependencyConflict",
                    "Player selection includes an incompatible project",
                ),
            ] {
                for project in entry
                    .extra
                    .get(key)
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                {
                    let present = collection.player_files.values().any(|candidate| {
                        candidate
                            .extra
                            .get("enabled")
                            .and_then(serde_json::Value::as_bool)
                            != Some(false)
                            && candidate.source.as_ref().is_some_and(|candidate| {
                                candidate.provider == source.provider
                                    && candidate.project_id == project
                            })
                    });
                    if (key == "requiredProjects" && !present)
                        || (key == "incompatibleProjects" && present)
                    {
                        requirements.push(Requirement {
                            path: path.clone(),
                            state: state.into(),
                            reason: format!("{reason}: {} project {project}", source.provider),
                        });
                    }
                }
            }
        }
        if let Err(error) = player_source(server, entry) {
            requirements.push(Requirement {
                path: path.clone(),
                state: "playerMissing".into(),
                reason: error,
            });
        }
    }
    collection.validate()?;
    Ok(Snapshot {
        collection,
        server_baseline: baseline,
        requirements,
    })
}

pub fn player_source(server: &ConfigServer, entry: &ContentEntry) -> Result<PathBuf, String> {
    let retained = entry
        .extra
        .get("retainedPath")
        .and_then(serde_json::Value::as_str)
        .ok_or("Player content has no retained bytes")?;
    validate_relative_path(retained)?;
    if !retained.starts_with(".msc2-content/") {
        return Err("Player file is outside protected content storage".into());
    }
    let path = managed_path(&server.server_dir, retained)?;
    let actual = identify(&path)?;
    if actual.sha512 != entry.sha512 || actual.byte_size != entry.byte_size {
        return Err("Retained player bytes changed".into());
    }
    Ok(path)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Change {
    pub action: String,
    pub path: String,
    pub sha512: Option<String>,
    pub audience: Option<ContentAudience>,
    pub optional: Option<bool>,
    pub enabled: Option<bool>,
    pub redistribution_evidence: Option<String>,
    pub staged_upload_id: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub id: String,
    pub owner: String,
    pub server_id: String,
    pub server_dir: String,
    pub flavor: String,
    pub minecraft_version: Option<String>,
    pub loader_version: Option<String>,
    pub source_collection: Option<ContentCollection>,
    pub created_unix: u64,
    pub baseline: BTreeMap<String, String>,
    pub result: ContentCollection,
    pub changes: Vec<Change>,
    pub warnings: Vec<String>,
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn review_path(server: &ConfigServer, id: &str) -> Result<PathBuf, String> {
    uuid::Uuid::parse_str(id).map_err(|_| "Invalid content review identity")?;
    managed_path(
        &server.server_dir,
        &format!(".msc2-content/player-reviews/{id}.json"),
    )
}
pub(crate) fn retain_player(
    server: &ConfigServer,
    path: &str,
    source: &Path,
) -> Result<ContentEntry, String> {
    content_path(path)?;
    if path.starts_with("plugins/") {
        return Err("Server plugins cannot be added to a player launcher collection".into());
    }
    let identity = identify(source)?;
    let relative = format!(".msc2-content/player/blobs/{}", identity.sha512);
    let destination = managed_path(&server.server_dir, &relative)?;
    if !destination
        .try_exists()
        .map_err(|error| error.to_string())?
    {
        crate::pack_retention::copy_exact(source, &destination, identity.byte_size, &|| false)?;
    }
    let copied = identify(&destination)?;
    if copied.sha512 != identity.sha512 {
        return Err("Player file changed while copying; review again".into());
    }
    let mut entry = fresh_entry(path, &identity);
    entry.audience = ContentAudience::Players;
    entry.classification_evidence = vec!["Owner selected independent player content".into()];
    entry.unresolved_reasons.clear();
    if server
        .content_collection
        .as_ref()
        .is_some_and(|collection| {
            collection
                .current_files
                .values()
                .chain(collection.player_files.values())
                .any(|known| {
                    known.sha512 == entry.sha512
                        && known
                            .extra
                            .get("providerDistributionAllowed")
                            .and_then(serde_json::Value::as_bool)
                            == Some(false)
                })
        })
    {
        entry.extra.insert(
            "providerDistributionAllowed".into(),
            serde_json::Value::Bool(false),
        );
    }
    if path.starts_with("shaderpacks/") || path.starts_with("resourcepacks/") {
        entry
            .extra
            .insert("enabled".into(), serde_json::Value::Bool(false));
    }

    entry
        .extra
        .insert("retainedPath".into(), serde_json::Value::String(relative));
    Ok(entry)
}

pub fn review(
    server: &ConfigServer,
    owner: &str,
    revision: u64,
    changes: Vec<Change>,
    uploads: &BTreeMap<String, PathBuf>,
) -> Result<Review, String> {
    if changes.is_empty() || changes.len() > 256 {
        return Err("Review between 1 and 256 changes".into());
    }
    let snapshot = snapshot(server)?;
    if snapshot.collection.revision != revision {
        return Err("Content collection changed; refresh before reviewing".into());
    }
    let mut result = snapshot.collection;
    let mut touched = std::collections::BTreeSet::new();
    let mut warnings = Vec::new();
    for change in &changes {
        content_path(&change.path)?;
        if !touched.insert((change.action.clone(), change.path.to_ascii_lowercase())) {
            return Err("Duplicate content change".into());
        }
        match change.action.as_str() {
            "addPlayerFile" => {
                if result.player_files.contains_key(&change.path) {
                    return Err("Player path already exists; remove it in a separate review before replacing".into());
                }
                let source = change
                    .staged_upload_id
                    .as_ref()
                    .and_then(|id| uploads.get(id))
                    .ok_or("Player upload is unavailable")?;
                let mut entry = retain_player(server, &change.path, source)?;
                if change.sha512.as_deref() != Some(entry.sha512.as_str()) {
                    return Err("Uploaded bytes do not match the reviewed SHA512".into());
                }
                entry.optional = change.optional.unwrap_or(false);
                entry.redistribution_evidence = change.redistribution_evidence.clone();
                result.player_files.insert(change.path.clone(), entry);
            }
            "copyServerToPlayers" => {
                let original = result
                    .current_files
                    .get(&change.path)
                    .ok_or("Server content is absent")?
                    .clone();
                if change.sha512.as_deref() != Some(original.sha512.as_str()) {
                    return Err("Server file changed before review".into());
                }
                if result
                    .player_files
                    .get(&change.path)
                    .is_some_and(|player| player.sha512 != original.sha512)
                {
                    return Err(
                        "Player customization conflicts; remove or restore it explicitly".into(),
                    );
                }
                let source = managed_path(&server.server_dir, &change.path)?;
                let mut entry = retain_player(server, &change.path, &source)?;
                entry.source = original.source.clone();
                entry.optional = change.optional.unwrap_or(original.optional);
                entry.redistribution_evidence = original.redistribution_evidence.clone();
                entry.extra.extend(
                    original
                        .extra
                        .iter()
                        .filter(|(key, _)| key.as_str() != "retainedPath")
                        .map(|(k, v)| (k.clone(), v.clone())),
                );
                result.player_files.insert(change.path.clone(), entry);
                let server_entry = result
                    .current_files
                    .get_mut(&change.path)
                    .ok_or("Server content disappeared")?;
                server_entry.audience = ContentAudience::Both;
                server_entry.classification_evidence =
                    vec!["Owner reviewed shared server/player content".into()];
                server_entry.unresolved_reasons.clear();
            }
            "setServerAudience" => {
                let entry = result
                    .current_files
                    .get_mut(&change.path)
                    .ok_or("Server content is absent")?;
                if change.sha512.as_deref() != Some(entry.sha512.as_str()) {
                    return Err("Server file changed before review".into());
                }
                let audience = change.audience.ok_or("Choose an audience")?;
                entry.audience = audience;
                entry.optional = change.optional.unwrap_or(entry.optional);
                entry.classification_evidence =
                    vec!["Owner reviewed client/server classification".into()];
                entry.unresolved_reasons.clear();
                if audience == ContentAudience::Players {
                    warnings.push(format!("{} is still physically installed on the server; classification does not uninstall it", change.path));
                }
            }
            "setPlayerOptions" | "removePlayerFile" => {
                let entry = result
                    .player_files
                    .get_mut(&change.path)
                    .ok_or("Player content is absent")?;
                if change.sha512.as_deref() != Some(entry.sha512.as_str()) {
                    return Err("Player content changed before review".into());
                }
                if change.action == "removePlayerFile" {
                    result.player_files.remove(&change.path);
                    let tombstone = change.path.clone();
                    if !result.player_excluded_paths.contains(&tombstone) {
                        result.player_excluded_paths.push(tombstone);
                    }
                } else {
                    entry.optional = change.optional.unwrap_or(entry.optional);
                    if let Some(enabled) = change.enabled {
                        entry
                            .extra
                            .insert("enabled".into(), serde_json::Value::Bool(enabled));
                    }
                    if change.redistribution_evidence.is_some() {
                        if entry
                            .extra
                            .get("providerDistributionAllowed")
                            .and_then(serde_json::Value::as_bool)
                            == Some(false)
                        {
                            return Err(
                                "A known provider redistribution restriction cannot be overridden"
                                    .into(),
                            );
                        }
                        entry.redistribution_evidence = change.redistribution_evidence.clone();
                    }
                }
            }
            "restorePlayerUpstream" => {
                let entry = result
                    .upstream_player_files
                    .get(&change.path)
                    .ok_or("No retained upstream player file")?
                    .clone();
                if change.sha512.as_deref() != Some(entry.sha512.as_str()) {
                    return Err("Upstream bytes differ from review".into());
                }
                player_source(server, &entry)?;
                result.player_files.insert(change.path.clone(), entry);
                result
                    .excluded_paths
                    .retain(|path| path != &format!("players:{}", change.path));
            }
            _ => return Err("Unknown player content change".into()),
        }
    }
    if !result
        .retained_roots
        .iter()
        .any(|root| root == ".msc2-content/player")
    {
        result.retained_roots.push(".msc2-content/player".into());
    }
    create_review(
        server,
        owner,
        snapshot.server_baseline,
        result,
        changes,
        warnings,
    )
}

pub(crate) fn create_review(
    server: &ConfigServer,
    owner: &str,
    baseline: BTreeMap<String, String>,
    mut result: ContentCollection,
    changes: Vec<Change>,
    warnings: Vec<String>,
) -> Result<Review, String> {
    result.revision = server
        .content_collection
        .as_ref()
        .map_or(0, |collection| collection.revision)
        .checked_add(1)
        .ok_or("Content revision overflow")?;
    result.validate()?;
    let review = Review {
        id: uuid::Uuid::new_v4().to_string(),
        owner: owner.into(),
        server_id: server.id.clone(),
        server_dir: server.server_dir.clone(),
        flavor: format!("{:?}", server.java_flavor),
        minecraft_version: server.minecraft_version.clone(),
        loader_version: server.loader_version.clone(),
        source_collection: server.content_collection.clone(),
        created_unix: now(),
        baseline,
        result,
        changes,
        warnings,
    };
    let bytes = serde_json::to_vec(&review).map_err(|error| error.to_string())?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("Content review is too large".into());
    }
    msc_infrastructure::atomic_write::atomic_write(
        &msc_infrastructure::fs::StdFileSystem,
        &review_path(server, &review.id)?,
        &bytes,
    )
    .map_err(|error| error.to_string())?;
    Ok(review)
}

pub fn load_review(server: &ConfigServer, id: &str, owner: &str) -> Result<Review, String> {
    use msc_infrastructure::fs::FileSystem;
    let bytes = msc_infrastructure::fs::StdFileSystem
        .read_bounded(&review_path(server, id)?, 16 * 1024 * 1024)
        .map_err(|error| error.to_string())?;
    let review: Review = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if review.id != id
        || review.owner != owner
        || review.server_id != server.id
        || review.server_dir != server.server_dir
        || review.flavor != format!("{:?}", server.java_flavor)
        || review.minecraft_version != server.minecraft_version
        || review.loader_version != server.loader_version
        || review.source_collection != server.content_collection
        || now().saturating_sub(review.created_unix) > 1800
    {
        return Err("Content review expired or server changed".into());
    }
    if snapshot(server)?.server_baseline != review.baseline {
        return Err("Server bytes changed after content review".into());
    }
    review.result.validate()?;
    for entry in review.result.player_files.values() {
        player_source(server, entry)?;
    }
    Ok(review)
}
