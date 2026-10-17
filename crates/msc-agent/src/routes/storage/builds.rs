//! Only content-addressed desktop-staged builds with complete reference proof.
use super::*;
use msc_infrastructure::{managed_builds, storage};
use sha2::{Digest, Sha256};
use std::{io::Read, path::PathBuf};
#[derive(Clone)]
pub(super) struct Build {
    pub root: PathBuf,
    pub name: String,
    pub root_id: String,
    pub fingerprint: String,
    pub bytes: Option<u64>,
    pub reason: String,
    pub removable: bool,
}
fn references(root: &std::path::Path) -> Result<String, String> {
    let current = managed_builds::current(root).map_err(|e| e.to_string())?;
    let reports = super::inventory::backend().inspect()?;
    if reports.iter().any(|s| {
        s.state != msc_infrastructure::service::ServiceState::NotInstalled && s.definition.is_none()
    }) {
        return Err("A service definition could not be verified".into());
    }
    let processes = map_staging::process_arguments().map_err(|e| e.to_string())?;
    if processes
        .lines()
        .filter(|p| p.contains("msc2-desktop") || p.contains("MSC 2.app/Contents/MacOS"))
        .count()
        > 1
    {
        return Err("Multiple desktop processes may hold older repair sources".into());
    }
    if processes.lines().any(|line| {
        let command = line.split_whitespace().next().unwrap_or("");
        let basename = std::path::Path::new(command)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        matches!(basename, "msc" | "vantage" | "bedrock-map")
            && !std::path::Path::new(command).is_absolute()
    }) {
        return Err("A relative agent/helper command cannot be bound to an installed build".into());
    }
    // Installer helpers may hold a fallback path outside the current definition.
    if processes.lines().any(|p| {
        p.contains("msc")
            && (p.contains("agent-service-helper")
                || p.contains("msiexec")
                || p.contains("agent install")
                || p.contains("agent repair"))
    }) {
        return Err("Service installation may be in progress".into());
    }
    Ok(format!(
        "{current}\n{reports:?}\n{processes}\n{}",
        std::env::current_exe()
            .map_err(|e| e.to_string())?
            .display()
    ))
}
fn owned(path: &std::path::Path, name: &str) -> Result<(u64, String), String> {
    let names = if cfg!(windows) {
        ["msc.exe", "vantage.exe", "bedrock-map.exe"]
    } else {
        ["msc", "vantage", "bedrock-map"]
    };
    let files = std::fs::read_dir(path)
        .map_err(|e| e.to_string())?
        .take(4)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    if files.len() != 3
        || files
            .iter()
            .any(|f| !names.iter().any(|n| f.file_name() == *n))
    {
        return Err("Unrecognized build contents are protected".into());
    }
    let before = storage::observe_tree(path, &|| false).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    for file in names {
        let mut input = storage::open_owned_file(&path.join(file)).map_err(|e| e.to_string())?;
        let meta = input.metadata().map_err(|e| e.to_string())?;
        if !meta.is_file() || meta.len() > 512 * 1024 * 1024 {
            return Err("Build file cannot be verified within inspection budget".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if meta.nlink() != 1 {
                return Err("Shared build files are protected".into());
            }
        }
        let mut remaining = meta.len();
        while remaining > 0 {
            let max = buffer.len().min(remaining as usize);
            let n = input.read(&mut buffer[..max]).map_err(|e| e.to_string())?;
            if n == 0 {
                return Err("Build changed during inspection".into());
            }
            hash.update(&buffer[..n]);
            remaining -= n as u64;
        }
        if input.read(&mut buffer[..1]).map_err(|e| e.to_string())? != 0 {
            return Err("Build grew during inspection".into());
        }
    }
    if format!("{:x}", hash.finalize()) != name
        || storage::observe_tree(path, &|| false).map_err(|e| e.to_string())? != before
    {
        return Err("Build identity or contents changed".into());
    }
    Ok(before)
}
pub(super) fn candidates(
    lifecycle: &LifecycleRoutesState,
    host: &str,
) -> Result<Vec<Build>, String> {
    let manifest = super::inventory::discover(lifecycle, host)?;
    let mut result = vec![];
    for root in manifest.roots.iter().filter(|r| {
        r.rules
            .iter()
            .any(|r| r.relative == std::path::Path::new("agent/builds"))
    }) {
        let root = root.path.join("agent/builds");
        if !root.exists() {
            continue;
        }
        let _guard = managed_builds::guard(&root, false, false).map_err(|e| e.to_string())?;
        let root_id = storage::verified_root_id(host, &root).map_err(|e| e.to_string())?;
        let refs = references(&root);
        let mut children = std::fs::read_dir(&root)
            .map_err(|e| e.to_string())?
            .take(257)
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_str().is_some_and(managed_builds::valid_id))
            .collect::<Vec<_>>();
        children.sort_by_key(|e| std::cmp::Reverse(e.metadata().and_then(|m| m.modified()).ok()));
        let fallback = children
            .iter()
            .find(|e| {
                refs.as_ref()
                    .is_ok_and(|r| !r.contains(&e.file_name().to_string_lossy().to_string()))
            })
            .map(|e| e.file_name());
        for child in children.into_iter().take(256) {
            let name = child.file_name().to_string_lossy().to_string();
            let proof = refs.as_ref().map_err(Clone::clone).and_then(|refs| {
                if refs.contains(&name) || fallback.as_ref() == Some(&child.file_name()) {
                    Err("Current, referenced or rollback build is protected".into())
                } else {
                    owned(&child.path(), &name)
                }
            });
            result.push(Build {
                root: root.clone(),
                name,
                root_id: root_id.clone(),
                bytes: proof.as_ref().ok().map(|p| p.0),
                fingerprint: proof.as_ref().ok().map(|p| p.1.clone()).unwrap_or_default(),
                removable: proof.is_ok(),
                reason: proof
                    .err()
                    .unwrap_or_else(|| "Verified obsolete desktop-staged build".into()),
            });
        }
    }
    Ok(result)
}
pub(super) fn remove(build: &Build, host: &str) -> Result<u64, String> {
    let _guard = managed_builds::guard(&build.root, false, false).map_err(|e| e.to_string())?;
    if storage::verified_root_id(host, &build.root).map_err(|e| e.to_string())? != build.root_id {
        return Err("Build root changed".into());
    }
    let references = references(&build.root)?;
    let mut fallbacks = std::fs::read_dir(&build.root)
        .map_err(|e| e.to_string())?
        .take(257)
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_str().is_some_and(managed_builds::valid_id))
        .filter(|e| !references.contains(e.file_name().to_string_lossy().as_ref()))
        .collect::<Vec<_>>();
    fallbacks.sort_by_key(|e| std::cmp::Reverse(e.metadata().and_then(|m| m.modified()).ok()));
    if fallbacks
        .first()
        .is_some_and(|e| e.file_name() == build.name.as_str())
    {
        return Err("Build is now the retained rollback source".into());
    }
    if references.contains(&build.name) {
        return Err("Build is now referenced".into());
    }
    let path = build.root.join(&build.name);
    let current = owned(&path, &build.name)?;
    if Some(current.0) != build.bytes || current.1 != build.fingerprint {
        return Err("Build changed since review".into());
    }
    std::fs::remove_dir_all(path).map_err(|e| format!("Removal unconfirmed: {e}"))?;
    Ok(current.0)
}
