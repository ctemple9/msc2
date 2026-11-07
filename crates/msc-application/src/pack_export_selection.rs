//! Full-pack selection retains client filtering separately from owner edits.
//! This is a read-only export projection; live files and retained originals stay intact.
use crate::{content_install::managed_path, content_management};
use msc_domain::{
    app_config_schema::ConfigServer,
    content_collection::{ContentAudience, ContentEntry},
};
use msc_infrastructure::addon_provider::fingerprint::identify;
use std::collections::BTreeMap;

fn present(server: &ConfigServer, path: &str) -> Result<bool, String> {
    managed_path(&server.server_dir, path)?
        .try_exists()
        .map_err(|error| error.to_string())
}
fn selected(
    mut entry: ContentEntry,
    original: Option<&ContentEntry>,
    source: Option<&str>,
) -> ContentEntry {
    let original_bytes = original.is_some_and(|base| {
        base.sha512 == entry.sha512
            && base
                .extra
                .get("retainedPath")
                .and_then(|v| v.as_str())
                .is_some_and(|path| path.starts_with(".msc2-content/packs/"))
    });
    entry
        .extra
        .insert("originalPackContent".into(), original_bytes.into());
    if let Some(source) = source {
        entry.extra.insert("retainedPath".into(), source.into());
    }
    entry.classification_evidence = vec!["Selected by the owner’s modpack export mode".into()];
    entry.unresolved_reasons.retain(|reason| {
        !reason.contains("classification")
            && !reason.contains("differs from the retained pack baseline")
            && !reason.contains("missing or was excluded")
    });
    entry
}
fn walk(
    server: &ConfigServer,
    relative: &str,
    files: &mut BTreeMap<String, ContentEntry>,
    count: &mut usize,
    depth: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    if depth > 16 || *count > 50_000 {
        return Err("Content folder exceeds the export inspection limit".into());
    }
    if cancelled() {
        return Err("Modpack export review cancelled".into());
    }
    let path = managed_path(&server.server_dir, relative)?;
    if !path.try_exists().map_err(|e| e.to_string())? {
        return Ok(());
    }
    for child in std::fs::read_dir(path).map_err(|e| e.to_string())? {
        if cancelled() {
            return Err("Modpack export review cancelled".into());
        }
        *count += 1;
        if *count > 50_000 {
            return Err("Too many content files to export".into());
        }
        let child = child.map_err(|e| e.to_string())?;
        let name = child
            .file_name()
            .into_string()
            .map_err(|_| "Content filename is not Unicode")?;
        let relative = format!("{relative}/{name}");
        let kind = child.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err("Content folders contain a link; export requires regular files".into());
        }
        if kind.is_dir() {
            walk(server, &relative, files, count, depth + 1, cancelled)?;
        } else if kind.is_file()
            && content_management::content_path(&relative).is_ok()
            && !files.contains_key(&relative)
        {
            let identity = identify(&managed_path(&server.server_dir, &relative)?)?;
            files.insert(
                relative.clone(),
                ContentEntry {
                    path: relative,
                    sha512: identity.sha512,
                    byte_size: identity.byte_size,
                    audience: ContentAudience::Both,
                    optional: false,
                    source: None,
                    redistribution_evidence: None,
                    classification_evidence: vec![],
                    unresolved_reasons: vec![],
                    extra: BTreeMap::new(),
                },
            );
        }
    }
    Ok(())
}

pub fn project(
    server: &ConfigServer,
    snapshot: &mut content_management::Snapshot,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    let stored = server.content_collection.as_ref();
    let mut live = snapshot.collection.current_files.clone();
    let mut count = 0;
    // Only known content folders, never the server directory, worlds, accounts or runtime.
    for folder in [
        "resourcepacks",
        "shaderpacks",
        "config",
        "scripts",
        "kubejs",
        "defaultconfigs",
    ] {
        walk(server, folder, &mut live, &mut count, 0, cancelled)?;
    }
    live.retain(|path, _| {
        snapshot.server_baseline.contains_key(path)
            || snapshot
                .server_baseline
                .contains_key(&format!("{path}.disabled"))
            || !path.starts_with("mods/") && !path.starts_with("plugins/")
    });
    let mut players = snapshot.collection.upstream_player_files.clone();
    for (path, base) in &snapshot.collection.upstream_files {
        players.entry(path.clone()).or_insert_with(|| {
            let mut entry = base.clone();
            if entry.audience == ContentAudience::Server {
                entry.optional = true;
                entry.extra.insert("enabled".into(), false.into());
            }
            entry
        });
    }
    players.extend(snapshot.collection.player_files.clone());
    players.retain(|path, _| !path.starts_with("plugins/"));
    let mut removed = Vec::new();
    for (path, entry) in &mut players {
        let original = snapshot
            .collection
            .upstream_player_files
            .get(path)
            .or_else(|| snapshot.collection.upstream_files.get(path));
        *entry = selected(entry.clone(), original, None);
        entry.audience = ContentAudience::Both;
    }
    if let Some(stored) = stored {
        for (stored_path, old) in &stored.current_files {
            let path = stored_path.strip_suffix(".disabled").unwrap_or(stored_path);
            if content_management::content_path(path).is_err() {
                continue;
            }
            // Import filtering is recorded before user edits. Older retained records
            // explicitly marked files absent during import; do not call those deletions.
            let was_installed = old
                .extra
                .get("packServerInstalled")
                .and_then(|v| v.as_bool())
                .unwrap_or_else(|| {
                    !old.unresolved_reasons
                        .iter()
                        .any(|reason| reason.contains("missing or was excluded"))
                });
            if was_installed
                && !present(server, path)?
                && !present(server, &format!("{path}.disabled"))?
            {
                players.remove(path);
                removed.push(path.to_owned());
            }
        }
        for path in stored
            .player_excluded_paths
            .iter()
            .chain(stored.excluded_paths.iter())
        {
            let path = path.strip_prefix("players:").unwrap_or(path);
            if !present(server, path)? {
                players.remove(path);
                removed.push(path.to_owned());
            }
        }
    }
    let mut server_files = BTreeMap::new();
    for (path, entry) in live {
        if cancelled() {
            return Err("Modpack export review cancelled".into());
        }
        let active_path = path.strip_suffix(".disabled").unwrap_or(&path);
        // A separately retained disabled copy must not disable an active JAR
        // with the same name. Keep both files when both physically exist.
        let canonical = if path.ends_with(".jar.disabled") && present(server, active_path)? {
            path.as_str()
        } else {
            active_path
        };
        let disabled = entry.extra.get("serverEnabled").and_then(|v| v.as_bool()) == Some(false)
            || path.ends_with(".jar.disabled");
        let source = if disabled && !path.ends_with(".disabled") {
            format!("{path}.disabled")
        } else {
            path.clone()
        };
        if !present(server, &source)? {
            continue;
        }
        let base = snapshot.collection.upstream_files.get(canonical);
        let mut current = selected(entry, base, Some(&source));
        current.audience = ContentAudience::Both;
        if disabled {
            current.extra.insert("enabled".into(), false.into());
            current.optional = true;
        }
        server_files.insert(path.clone(), current.clone());
        if path.starts_with("plugins/") {
            continue;
        }
        let existing = players.get(canonical);
        let server_changed = base.is_none_or(|base| base.sha512 != current.sha512);
        // Preserve client-specific overrides unless the owner changed the server copy.
        if existing.is_none() || server_changed {
            current.path = canonical.into();
            players.insert(canonical.into(), current);
        }
        if let Some(player) = players.get_mut(canonical) {
            if disabled {
                player.extra.insert("enabled".into(), false.into());
                player.optional = true;
            } else if base.is_some_and(|base| base.audience != ContentAudience::Server) {
                player.extra.insert("enabled".into(), true.into());
            }
        }
    }
    if let Some(stored) = stored {
        for path in &stored.player_excluded_paths {
            players.remove(path);
            removed.push(path.to_owned());
        }
    }
    if let Some(missing) = snapshot
        .collection
        .extra
        .get_mut("retainedPackUnresolved")
        .and_then(serde_json::Value::as_object_mut)
    {
        for path in removed {
            missing.remove(&path);
        }
    }
    snapshot.collection.current_files = server_files;
    snapshot.collection.player_files = players;
    snapshot.requirements.clear();
    Ok(())
}
