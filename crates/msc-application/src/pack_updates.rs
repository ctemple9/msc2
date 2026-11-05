//! Three-way pack comparison is independent of filesystem mutation. A later
//! activation consumes the pinned result, so comparison can never overwrite a
//! server merely because a newer pack contains a path with the same name.
use msc_domain::content_collection::{ContentCollection, ContentEntry};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathChange {
    pub path: String,
    pub old_sha512: Option<String>,
    pub current_sha512: Option<String>,
    pub new_sha512: Option<String>,
    pub action: String,
    pub conflict: bool,
    pub restored: bool,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeResult {
    pub files: BTreeMap<String, ContentEntry>,
    pub excluded_paths: Vec<String>,
    pub changes: Vec<PathChange>,
}
fn same_bytes(left: Option<&ContentEntry>, right: Option<&ContentEntry>) -> bool {
    left.map(|entry| (&entry.sha512, entry.byte_size))
        == right.map(|entry| (&entry.sha512, entry.byte_size))
}
fn owner_changed(current: &ContentEntry, old: &ContentEntry) -> bool {
    !same_bytes(Some(current), Some(old))
        || current.optional != old.optional
        || current.audience != old.audience
        || current
            .extra
            .get("ownerAddition")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
}
fn validate(files: &BTreeMap<String, ContentEntry>) -> Result<(), String> {
    let collection = ContentCollection {
        current_files: files.clone(),
        ..Default::default()
    };
    collection.validate()?;
    for path in files.keys() {
        crate::content_management::content_path(path)?;
    }
    Ok(())
}

/// `current` must represent observed bytes, not stale persisted hashes. Missing
/// old paths are intentional removals for this comparison and stay absent by
/// default. Restoring upstream is an explicit per-path decision, including a
/// deliberate deletion when the new upstream no longer contains that path.
pub fn compare(
    old: &BTreeMap<String, ContentEntry>,
    current: &BTreeMap<String, ContentEntry>,
    new: &BTreeMap<String, ContentEntry>,
    excluded: &[String],
    restore_upstream: &[String],
) -> Result<MergeResult, String> {
    for files in [old, current, new] {
        validate(files)?;
    }
    let paths: BTreeSet<_> = old
        .keys()
        .chain(current.keys())
        .chain(new.keys())
        .cloned()
        .collect();
    let mut removed: BTreeSet<_> = excluded.iter().cloned().collect();
    let mut restore: BTreeSet<_> = restore_upstream.iter().cloned().collect();
    if restore.len() != restore_upstream.len() {
        return Err("Duplicate upstream restoration choice".into());
    }
    // Provider identity joins renamed JARs across releases. Otherwise an edited
    // or intentionally removed old filename could coexist with a newly added
    // filename for the very same mod, resurrecting it or loading two versions.
    for (new_path, incoming) in new {
        let Some(source) = &incoming.source else {
            continue;
        };
        for (old_path, previous) in old {
            if old_path == new_path
                || !previous.source.as_ref().is_some_and(|old_source| {
                    old_source.provider == source.provider
                        && old_source.project_id == source.project_id
                })
            {
                continue;
            }
            if restore.contains(new_path) {
                restore.insert(old_path.clone());
            } else if current
                .get(old_path)
                .is_none_or(|entry| owner_changed(entry, previous))
                || removed.contains(old_path)
            {
                removed.insert(new_path.clone());
            }
        }
    }
    for path in removed.iter().chain(&restore) {
        crate::content_management::content_path(path)?;
    }
    if !restore.is_subset(&paths) {
        return Err("Restoration path is absent from this pack comparison".into());
    }
    let mut files = BTreeMap::new();
    let mut changes = Vec::new();
    for path in paths {
        let before = old.get(&path);
        let owner = current.get(&path);
        let upstream = new.get(&path);
        let intentional_removal = removed.contains(&path) || (before.is_some() && owner.is_none());
        if intentional_removal {
            removed.insert(path.clone());
        }
        let customized = match (before, owner) {
            (Some(before), Some(owner)) => owner_changed(owner, before),
            (None, Some(_)) => true,
            _ => false,
        };
        let restored = restore.contains(&path);
        let conflict = !same_bytes(owner, upstream)
            && (customized || intentional_removal)
            && !same_bytes(before, upstream);
        let (selected, reason) = if restored {
            removed.remove(&path);
            (
                upstream.cloned(),
                "Owner explicitly chose the new upstream version",
            )
        } else if intentional_removal {
            // A tombstone cannot silently delete a newly-created owner file.
            (
                owner.cloned(),
                "Preserve the owner's removal or subsequent explicit addition",
            )
        } else if customized {
            (
                owner.cloned(),
                "Preserve owner content and settings by default",
            )
        } else {
            (
                upstream.cloned(),
                "Unmodified pack content follows the new upstream",
            )
        };
        let action = match (owner, selected.as_ref()) {
            (None, Some(_)) => "add",
            (Some(_), None) => "remove",
            (Some(owner), Some(selected)) if !same_bytes(Some(owner), Some(selected)) => "replace",
            _ => "keep",
        };
        let mut selected = selected;
        if customized
            && !restored
            && before.is_none()
            && let Some(entry) = selected.as_mut()
        {
            entry
                .extra
                .insert("ownerAddition".into(), serde_json::Value::Bool(true));
        }
        if let Some(entry) = selected {
            files.insert(path.clone(), entry);
        }
        changes.push(PathChange {
            path,
            old_sha512: before.map(|entry| entry.sha512.clone()),
            current_sha512: owner.map(|entry| entry.sha512.clone()),
            new_sha512: upstream.map(|entry| entry.sha512.clone()),
            action: action.into(),
            conflict,
            restored,
            reason: reason.into(),
        });
    }
    validate(&files)?;
    Ok(MergeResult {
        files,
        excluded_paths: removed.into_iter().collect(),
        changes,
    })
}
