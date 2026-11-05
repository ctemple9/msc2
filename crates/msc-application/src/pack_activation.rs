//! Recoverable file activation shared by pack updates and replacement. Original
//! and replacement bytes are staged independently before any active path moves.
use crate::content_install::managed_path;
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::content_collection::{ContentCollection, ContentEntry};
use msc_infrastructure::addon_provider::fingerprint::identify;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub before_sha512: Option<String>,
    pub after: Option<ContentEntry>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activation {
    pub id: String,
    pub server_id: String,
    pub server_dir: String,
    pub collection_revision: u64,
    pub changes: Vec<FileChange>,
    pub collection: ContentCollection,
}
fn allowed(path: &str) -> Result<(), String> {
    msc_domain::content_collection::validate_relative_path(path)?;
    crate::content_management::content_path(path.strip_suffix(".disabled").unwrap_or(path))
}
fn marker(server: &ConfigServer) -> Result<PathBuf, String> {
    managed_path(&server.server_dir, ".msc2-content/pack-active.json")
}
fn staged(plan: &Activation, side: &str, path: &str) -> Result<PathBuf, String> {
    uuid::Uuid::parse_str(&plan.id).map_err(|_| "Invalid pack transaction identity")?;
    allowed(path)?;
    managed_path(
        &plan.server_dir,
        &format!(".msc2-content/pack-transactions/{}/{side}/{path}", plan.id),
    )
}
fn observed(path: &std::path::Path) -> Result<Option<String>, String> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => identify(path).map(|identity| Some(identity.sha512)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}
fn validate(server: &ConfigServer, plan: &Activation) -> Result<(), String> {
    if plan.server_id != server.id
        || plan.server_dir != server.server_dir
        || plan.changes.len() > 50_000
    {
        return Err("Pack transaction does not match this server".into());
    }
    uuid::Uuid::parse_str(&plan.id).map_err(|_| "Invalid pack transaction identity")?;
    plan.collection.validate()?;
    let mut names = std::collections::BTreeSet::new();
    for change in &plan.changes {
        allowed(&change.path)?;
        if !names.insert(change.path.to_lowercase()) {
            return Err("Pack transaction paths collide".into());
        }
        if let Some(after) = &change.after
            && (after.path != change.path
                || after.sha512.len() != 128
                || !after.sha512.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err("Invalid replacement content identity".into());
        }
        if change.before_sha512.as_ref().is_some_and(|hash| {
            hash.len() != 128 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            return Err("Invalid original content identity".into());
        }
    }
    Ok(())
}
fn stage_copy(
    source: &std::path::Path,
    target: &std::path::Path,
    size: u64,
    hash: &str,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    if cancelled() {
        return Err("Pack staging cancelled".into());
    }
    if let Some(existing) = observed(target)? {
        if existing != hash {
            return Err("Existing staged bytes changed; create a fresh pack review".into());
        }
        return Ok(());
    }
    crate::pack_retention::copy_exact(source, target, size, cancelled)?;
    if observed(target)?.as_deref() != Some(hash) {
        return Err("Content changed while staging pack transaction".into());
    }
    Ok(())
}

/// This prepares private copies only. The caller holds the operation lock;
/// stopped-server enforcement and authenticated review binding stay at the API.
pub fn prepare(
    server: &ConfigServer,
    plan: &Activation,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    validate(server, plan)?;
    if plan.collection_revision
        != server
            .content_collection
            .as_ref()
            .map_or(0, |collection| collection.revision)
    {
        return Err("Content revision changed".into());
    }
    if marker(server)?
        .try_exists()
        .map_err(|error| error.to_string())?
    {
        return Err("A previous pack transaction needs recovery".into());
    }
    for change in &plan.changes {
        if cancelled() {
            return Err("Pack staging cancelled".into());
        }
        let target = managed_path(&server.server_dir, &change.path)?;
        if observed(&target)? != change.before_sha512 {
            return Err(format!("Server content changed: {}", change.path));
        }
        if let Some(hash) = &change.before_sha512 {
            let source = identify(&target)?;
            let backup = staged(plan, "before", &change.path)?;
            stage_copy(&target, &backup, source.byte_size, hash, cancelled)?;
            if observed(&backup)?.as_ref() != Some(hash) {
                return Err("Original changed while staging rollback".into());
            }
        }
        if let Some(after) = &change.after {
            let source = crate::content_management::player_source(server, after)?;
            let destination = staged(plan, "after", &change.path)?;
            stage_copy(
                &source,
                &destination,
                after.byte_size,
                &after.sha512,
                cancelled,
            )?;
            if observed(&destination)?.as_deref() != Some(after.sha512.as_str()) {
                return Err("Replacement changed during staging".into());
            }
        }
    }
    Ok(())
}

/// Recovery never deletes a subsequently edited file to make rollback succeed.
/// An unresolved marker prevents launch until the preserved conflict is reviewed.
pub fn recover(server: &ConfigServer) -> Result<(), String> {
    use msc_infrastructure::fs::FileSystem;
    let marker = marker(server)?;
    if !marker.try_exists().map_err(|error| error.to_string())? {
        return Ok(());
    }
    let bytes = msc_infrastructure::fs::StdFileSystem
        .read_bounded(&marker, 32 * 1024 * 1024)
        .map_err(|error| error.to_string())?;
    let plan: Activation = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Pack recovery marker is invalid: {error}"))?;
    validate(server, &plan)?;
    let committed = server
        .content_collection
        .as_ref()
        .and_then(|collection| collection.extra.get("lastPackTransaction"))
        .and_then(serde_json::Value::as_str)
        == Some(plan.id.as_str());
    if !committed {
        let mut conflicts = Vec::new();
        for change in plan.changes.iter().rev() {
            let result = (|| {
                let target = managed_path(&server.server_dir, &change.path)?;
                let actual = observed(&target)?;
                if actual == change.before_sha512 {
                    return Ok(());
                }
                if actual.is_some()
                    && actual.as_deref() != change.after.as_ref().map(|entry| entry.sha512.as_str())
                {
                    return Err(format!("Owner-edited file preserved: {}", change.path));
                }
                let backup = if let Some(hash) = &change.before_sha512 {
                    let backup = staged(&plan, "before", &change.path)?;
                    if observed(&backup)?.as_ref() != Some(hash) {
                        return Err(format!("Rollback copy changed: {}", change.path));
                    }
                    Some(backup)
                } else {
                    None
                };
                if actual.is_some() {
                    std::fs::remove_file(&target).map_err(|error| error.to_string())?;
                }
                if let Some(backup) = backup {
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                    }
                    std::fs::hard_link(backup, target).map_err(|error| error.to_string())?;
                }
                Ok::<_, String>(())
            })();
            if let Err(error) = result {
                conflicts.push(error);
            }
        }
        if !conflicts.is_empty() {
            return Err(conflicts.join("; "));
        }
    }
    std::fs::remove_file(marker).map_err(|error| error.to_string())
}

pub fn activate(
    server: &ConfigServer,
    plan: &Activation,
    cancelled: &dyn Fn() -> bool,
    persist: &mut dyn FnMut(ContentCollection) -> Result<(), String>,
) -> Result<(), String> {
    validate(server, plan)?;
    if plan.collection_revision
        != server
            .content_collection
            .as_ref()
            .map_or(0, |collection| collection.revision)
    {
        return Err("Content revision changed".into());
    }
    let marker = marker(server)?;
    if marker.try_exists().map_err(|error| error.to_string())? {
        return Err("A previous pack transaction needs recovery".into());
    }
    for change in &plan.changes {
        if observed(&managed_path(&server.server_dir, &change.path)?)? != change.before_sha512 {
            return Err(format!("Server content changed: {}", change.path));
        }
        if let Some(hash) = &change.before_sha512
            && observed(&staged(plan, "before", &change.path)?)?.as_ref() != Some(hash)
        {
            return Err("Rollback bytes changed before activation".into());
        }
        if let Some(after) = &change.after
            && observed(&staged(plan, "after", &change.path)?)?.as_deref()
                != Some(after.sha512.as_str())
        {
            return Err("Staged replacement changed before activation".into());
        }
    }
    let bytes = serde_json::to_vec(plan).map_err(|error| error.to_string())?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("Pack activation exceeds journal size limit".into());
    }
    msc_infrastructure::atomic_write::atomic_write(
        &msc_infrastructure::fs::StdFileSystem,
        &marker,
        &bytes,
    )
    .map_err(|error| error.to_string())?;
    let result = (|| {
        for change in &plan.changes {
            if cancelled() {
                return Err("Pack activation cancelled".into());
            }
            let target = managed_path(&server.server_dir, &change.path)?;
            if observed(&target)? != change.before_sha512 {
                return Err(format!(
                    "Server file changed during activation: {}",
                    change.path
                ));
            }
            if change.before_sha512.is_some() {
                std::fs::remove_file(&target).map_err(|error| error.to_string())?;
            }
            if change.after.is_some() {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                std::fs::hard_link(staged(plan, "after", &change.path)?, &target)
                    .map_err(|error| error.to_string())?;
            }
        }
        if cancelled() {
            return Err("Pack activation cancelled before metadata commit".into());
        }
        for change in &plan.changes {
            let actual = observed(&managed_path(&server.server_dir, &change.path)?)?;
            if actual.as_deref() != change.after.as_ref().map(|entry| entry.sha512.as_str()) {
                return Err(format!(
                    "Server file changed before pack metadata commit: {}",
                    change.path
                ));
            }
        }
        let mut collection = plan.collection.clone();
        collection.extra.insert(
            "lastPackTransaction".into(),
            serde_json::Value::String(plan.id.clone()),
        );
        persist(collection)
    })();
    if let Err(error) = result {
        return match recover(server) {
            Ok(()) => Err(error),
            Err(recovery) => Err(format!("{error}; {recovery}")),
        };
    }
    std::fs::remove_file(marker)
        .map_err(|error| format!("Pack committed; journal cleanup needs recovery: {error}"))
}
