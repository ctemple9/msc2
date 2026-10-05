//! Retain exact client inputs before the server-only importer excludes them.
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::map_assets::{MAX_ARCHIVE, MAX_JSON, MAX_SCAN, error, hash, safe_member};
use std::{io, path::Path};

pub fn retain(
    fs: &dyn FileSystem,
    server: &Path,
    staged: &Path,
    manifest: &str,
    overrides: &str,
) -> io::Result<()> {
    let source = staged.join(manifest);
    // Older internal callers may supply parsed metadata without a source archive.
    // No receipt is invented for those calls.
    if fs.stat(&source).is_err() {
        return Ok(());
    }
    let metadata = fs.stat(&source)?;
    if metadata.size > MAX_JSON {
        return Err(error("map_source_manifest_limit"));
    }
    let raw = fs.read(&source)?;
    let base = server.join(".msc-map-source");
    for ancestor in base.ancestors() {
        if fs.read_link(ancestor).is_ok() {
            return Err(error("linked_map_source"));
        }
    }
    fs.create_dir_all(&base)?;
    let root = base.join(format!(".staging-{}", uuid::Uuid::new_v4()));
    let mut receipts = std::collections::BTreeMap::from([(manifest.to_string(), hash(&raw))]);
    fs.create_dir_all(&root)?;
    fs.write(&root.join(manifest), &raw)?;
    let mut budget = 0u64;
    let mut count = 0;
    let folders = if manifest == "modrinth.index.json" {
        vec!["overrides", "client-overrides"]
    } else {
        vec![overrides]
    };
    for folder in folders {
        if !safe_member(folder) {
            return Err(error("unsafe_map_override_path"));
        }
        let initial = staged.join(folder);
        if fs.stat(&initial).is_err() {
            continue;
        }
        let mut stack = vec![initial.clone()];
        while let Some(dir) = stack.pop() {
            for path in fs.list(&dir)? {
                count += 1;
                if count > 100_000 {
                    return Err(error("map_source_entry_limit"));
                }
                if fs.read_link(&path).is_ok() {
                    return Err(error("linked_map_source"));
                }
                let relative = path
                    .strip_prefix(&initial)
                    .map_err(|_| error("unsafe_map_override_path"))?;
                let name = relative.to_string_lossy().replace('\\', "/");
                if !safe_member(&name) {
                    return Err(error("unsafe_map_override_path"));
                }
                let m = fs.stat(&path)?;
                if m.is_dir {
                    stack.push(path);
                    continue;
                }
                let keep = name.starts_with("assets/")
                    || name == "pack.mcmeta"
                    || (name.starts_with("mods/") && name.ends_with(".jar"))
                    || (name.starts_with("resourcepacks/") && name.ends_with(".zip"));
                if !keep {
                    continue;
                }
                if !m.is_file || m.size > MAX_ARCHIVE {
                    return Err(error("map_source_file_limit"));
                }
                budget = budget
                    .checked_add(m.size)
                    .ok_or_else(|| error("map_source_byte_limit"))?;
                if budget > MAX_SCAN {
                    return Err(error("map_source_byte_limit"));
                }
                let target = root.join(folder).join(relative);
                fs.create_dir_all(
                    target
                        .parent()
                        .ok_or_else(|| error("unsafe_map_override_path"))?,
                )?;
                let bytes = fs.read(&path)?;
                if bytes.len() as u64 != m.size {
                    return Err(error("map_source_changed"));
                }
                receipts.insert(format!("{folder}/{name}"), hash(&bytes));
                fs.write(&target, &bytes)?;
            }
        }
    }
    let identity = msc_infrastructure::map_assets::hash_json(&receipts)?;
    fs.write(
        &root.join("receipt.json"),
        &serde_json::to_vec(&receipts).map_err(|_| error("map_source_receipt_write"))?,
    )?;
    let target = base.join(&identity);
    if fs.stat(&target).is_err() {
        fs.rename(&root, &target)?;
    } else {
        fs.remove(&root)?;
    }
    // Only a fully retained source becomes discoverable. Identity is source bytes,
    // never filenames or a inferred pack name from installed server mods.
    msc_infrastructure::atomic_write::atomic_write(fs, &base.join("current"), identity.as_bytes())
        .map_err(|_| error("map_source_receipt_write"))
}
