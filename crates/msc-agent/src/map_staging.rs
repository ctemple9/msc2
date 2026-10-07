//! Disk-backed map copies, with ownership lasting as long as their reader.
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;
const HEADROOM: u64 = 64 * 1024 * 1024;

pub(crate) struct Staging {
    path: PathBuf,
    lease: Option<File>,
}
impl Staging {
    pub(crate) fn create(kind: &str, estimate: u64) -> io::Result<Self> {
        let root =
            msc_infrastructure::config_repository::default_app_data_dir().join("map-staging");
        Self::create_at(&root, kind, estimate).map_err(|error| io::Error::new(error.kind(), format!("Map staging at {}: {error}. Check available disk space and this user's storage quota", root.display())))
    }
    fn create_at(root: &Path, kind: &str, estimate: u64) -> io::Result<Self> {
        if let Ok(meta) = fs::symlink_metadata(root)
            && (!meta.is_dir() || meta.file_type().is_symlink())
        {
            return Err(io::Error::other("staging root is not a regular directory"));
        }
        fs::create_dir_all(root)?;
        protect(root)?;
        let admission_path = root.join("admission.lock");
        if fs::symlink_metadata(&admission_path).is_ok_and(|meta| !meta.file_type().is_file()) {
            return Err(io::Error::other("invalid staging admission file"));
        }
        let admission = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(admission_path)?;
        admission.lock_exclusive()?;
        let mut reserved = 0u64;
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let lease_path = entry.path().join("lease");
            if !fs::symlink_metadata(&lease_path).is_ok_and(|meta| meta.file_type().is_file()) {
                continue;
            }
            let lease = match OpenOptions::new().read(true).write(true).open(&lease_path) {
                Ok(lease) => lease,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            match lease.try_lock_exclusive() {
                Ok(()) => {}
                Err(error)
                    if error.raw_os_error() == fs2::lock_contended_error().raw_os_error() =>
                {
                    // Windows locks prevent reads of the leased file itself.
                    // Read a separate immutable reservation while admission is locked.
                    let reservation_path = entry.path().join("reservation");
                    let metadata = match fs::symlink_metadata(&reservation_path) {
                        Ok(metadata) => metadata,
                        Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                        Err(error) => return Err(error),
                    };
                    if !metadata.file_type().is_file() {
                        return Err(io::Error::other("invalid staging reservation file"));
                    }
                    let mut value = String::new();
                    match File::open(reservation_path) {
                        Ok(file) => {
                            file.take(32).read_to_string(&mut value)?;
                        }
                        Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                        Err(error) => return Err(error),
                    }
                    let promised = value
                        .trim()
                        .parse::<u64>()
                        .map_err(|_| io::Error::other("invalid staging reservation"))?;
                    // Free-space reporting already subtracts bytes written.
                    // Reserve only the still-unwritten part of a live copy.
                    // A racing/failed inspection falls back to the full estimate.
                    let written = estimate_tree(&entry.path()).unwrap_or(0);
                    reserved = reserved.saturating_add(promised.saturating_sub(written));
                }
                Err(error) => return Err(error),
            }
            // An unlocked lease can belong to an orphaned renderer. Keep its
            // files: agent death alone does not prove the child stopped.
        }
        let reservation = estimate.saturating_add(HEADROOM);
        ensure_space(root, reserved.saturating_add(reservation))?;
        let path = root.join(format!("{kind}-{}", Uuid::new_v4()));
        fs::create_dir(&path)?;
        let mut staging = Self { path, lease: None };
        protect(&staging.path)?;
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(staging.path.join("lease"))?;
        lease.lock_exclusive()?;
        writeln!(
            File::create(staging.path.join("reservation"))?,
            "{reservation}"
        )?;
        staging.lease = Some(lease);
        Ok(staging)
    }
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}
impl Drop for Staging {
    fn drop(&mut self) {
        // Close the lease first so Windows can remove the directory. Callers
        // stop their child before dropping this guard.
        self.lease.take();
        if let Err(error) = fs::remove_dir_all(&self.path) {
            eprintln!(
                "Could not remove map staging {}: {error}",
                self.path.display()
            );
        }
    }
}
fn protect(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
pub(crate) fn ensure_space(path: &Path, needed: u64) -> io::Result<()> {
    let available = fs2::available_space(path)?;
    if available < needed {
        return Err(io::Error::other(format!(
            "{} needs {needed} free bytes; {available} available",
            path.display()
        )));
    }
    // Filesystem free space cannot certify per-user quota. Preserve write
    // failures rather than treating this preflight as a quota guarantee.
    Ok(())
}
pub(crate) fn estimate_tree(path: &Path) -> io::Result<u64> {
    fn walk(path: &Path, depth: usize, entries: &mut usize) -> io::Result<u64> {
        *entries += 1;
        if depth > 32 || *entries > 1_000_000 {
            return Err(io::Error::other(
                "map source exceeds bounded directory inspection",
            ));
        }
        let meta = fs::symlink_metadata(path)?;
        if meta.file_type().is_file() {
            return Ok(meta.len());
        }
        if !meta.file_type().is_dir() {
            return Err(io::Error::other(
                "map source contains a symlink or special file",
            ));
        }
        let mut total = 0u64;
        for entry in fs::read_dir(path)? {
            total = total.saturating_add(walk(&entry?.path(), depth + 1, entries)?);
        }
        Ok(total)
    }
    walk(path, 0, &mut 0)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupEntry {
    pub id: String,
    pub path: String,
    pub size_bytes: u64,
    pub removable: bool,
    pub reason: String,
    #[serde(skip)]
    fingerprint: String,
}

pub(crate) fn cleanup_root() -> PathBuf {
    msc_infrastructure::config_repository::default_app_data_dir().join("map-staging")
}

// Old agents did not give renderer children a lease. An unlocked parent lease
// is therefore insufficient: inspect live process arguments before offering it.
fn process_arguments() -> io::Result<String> {
    #[cfg(unix)]
    let output = std::process::Command::new("ps")
        .args(["-axww", "-o", "args="])
        .output()?;
    #[cfg(windows)]
    let output = std::process::Command::new("powershell.exe").args([
        "-NoProfile", "-NonInteractive", "-Command",
        "$ErrorActionPreference='Stop'; Get-CimInstance Win32_Process | ForEach-Object { if (!$_.CommandLine -and $_.Name -match 'java|vantage|msc') { throw 'Cannot inspect a terrain-related process' }; $_.CommandLine }"
    ]).output()?;
    #[cfg(not(any(unix, windows)))]
    return Err(io::Error::other(
        "Process inspection is unsupported on this platform",
    ));
    if !output.status.success() || output.stdout.len() > 8 * 1024 * 1024 {
        return Err(io::Error::other(
            "Cannot verify whether terrain helpers still use staging; cleanup refused",
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| io::Error::other("Cannot decode process inspection; cleanup refused"))
}

fn owned_name(name: &str) -> bool {
    ["java-renderer-", "snapshot-", "bedrock-tiles-"]
        .iter()
        .any(|prefix| {
            name.strip_prefix(prefix)
                .is_some_and(|suffix| Uuid::parse_str(suffix).is_ok())
        })
}

fn admission(root: &Path) -> io::Result<File> {
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::other("Cleanup root is not a regular directory"));
    }
    let path = root.join("admission.lock");
    if fs::symlink_metadata(&path).is_ok_and(|metadata| !metadata.file_type().is_file()) {
        return Err(io::Error::other("Invalid staging admission lock"));
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    file.lock_exclusive()?;
    Ok(file)
}

fn cleanup_fingerprint(path: &Path) -> io::Result<String> {
    use sha2::{Digest, Sha256};
    fn walk(path: &Path, hash: &mut Sha256, depth: usize, count: &mut usize) -> io::Result<()> {
        *count += 1;
        if depth > 32 || *count > 1_000_000 {
            return Err(io::Error::other("Staging tree exceeds inspection limits"));
        }
        let meta = fs::symlink_metadata(path)?;
        if !meta.is_dir() && !meta.is_file() {
            return Err(io::Error::other(
                "Staging contains a symlink or special file",
            ));
        }
        hash.update(path.as_os_str().as_encoded_bytes());
        hash.update(meta.len().to_le_bytes());
        let modified = meta
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?;
        hash.update(modified.as_nanos().to_le_bytes());
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            hash.update(meta.dev().to_le_bytes());
            hash.update(meta.ino().to_le_bytes());
        }
        if meta.is_dir() {
            let mut children = fs::read_dir(path)?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<io::Result<Vec<_>>>()?;
            children.sort();
            for child in children {
                walk(&child, hash, depth + 1, count)?;
            }
        }
        Ok(())
    }
    let mut hash = Sha256::new();
    walk(path, &mut hash, 0, &mut 0)?;
    Ok(format!("{:x}", hash.finalize()))
}

fn inspect_entry(
    root: &Path,
    id: &str,
    processes: &str,
) -> io::Result<(CleanupEntry, Option<File>)> {
    if !owned_name(id) {
        return Err(io::Error::other("Unrecognized staging directory"));
    }
    let path = root.join(id);
    let meta = fs::symlink_metadata(&path)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(io::Error::other("Not a regular staging directory"));
    }
    let lease_path = path.join("lease");
    let reservation = path.join("reservation");
    for marker in [&lease_path, &reservation] {
        if !fs::symlink_metadata(marker)?.file_type().is_file() {
            return Err(io::Error::other("Invalid staging ownership marker"));
        }
    }
    let lease = OpenOptions::new().read(true).write(true).open(lease_path)?;
    let locked = match lease.try_lock_exclusive() {
        Ok(()) => false,
        Err(error) if error.raw_os_error() == fs2::lock_contended_error().raw_os_error() => true,
        Err(error) => return Err(error),
    };
    let path_text = path
        .to_str()
        .ok_or_else(|| io::Error::other("Unsupported staging path"))?;
    let in_use = locked || processes.contains(path_text);
    let size_bytes = estimate_tree(&path)?;
    let fingerprint = cleanup_fingerprint(&path)?;
    let entry = CleanupEntry {
        id: id.to_string(),
        path: path_text.to_string(),
        size_bytes,
        fingerprint,
        removable: !in_use,
        reason: if in_use {
            "In use by a map or terrain helper; retained."
        } else {
            "Abandoned map copy/render output; recreated when the map opens."
        }
        .to_string(),
    };
    Ok((entry, if locked { None } else { Some(lease) }))
}

pub(crate) fn preview_cleanup() -> io::Result<Vec<CleanupEntry>> {
    let root = cleanup_root();
    if !root.try_exists()? {
        return Ok(Vec::new());
    }
    let _admission = admission(&root)?;
    let processes = process_arguments()?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let id = entry.file_name().to_string_lossy().to_string();
        if !owned_name(&id) {
            continue;
        }
        match inspect_entry(&root, &id, &processes) {
            Ok((item, _lease)) => entries.push(item),
            Err(error) => entries.push(CleanupEntry {
                id,
                path: entry.path().display().to_string(),
                size_bytes: 0,
                fingerprint: String::new(),
                removable: false,
                reason: format!("Cannot verify ownership or contents; retained: {error}"),
            }),
        }
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(entries)
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupResult {
    pub removed_bytes: u64,
    pub removed: Vec<String>,
    pub retained: Vec<String>,
}

pub(crate) fn execute_cleanup(
    approved: &[CleanupEntry],
    should_cancel: &impl Fn() -> bool,
) -> io::Result<CleanupResult> {
    let root = cleanup_root();
    let _admission = admission(&root)?;
    let processes = process_arguments()?;
    execute_cleanup_inner(&root, approved, &processes, should_cancel)
}

#[cfg(test)]
fn execute_cleanup_with_processes(
    root: &Path,
    approved: &[CleanupEntry],
    processes: &str,
) -> io::Result<CleanupResult> {
    execute_cleanup_inner(root, approved, processes, &|| false)
}

fn execute_cleanup_inner(
    root: &Path,
    approved: &[CleanupEntry],
    processes: &str,
    should_cancel: &impl Fn() -> bool,
) -> io::Result<CleanupResult> {
    let mut result = CleanupResult {
        removed_bytes: 0,
        removed: Vec::new(),
        retained: Vec::new(),
    };
    for item in approved.iter().filter(|item| item.removable) {
        if should_cancel() {
            result
                .retained
                .push(format!("{}: cleanup cancelled before removal", item.id));
            continue;
        }
        let (current, lease) = match inspect_entry(root, &item.id, processes) {
            Ok(current) => current,
            Err(error) => {
                result.retained.push(format!("{}: {error}", item.id));
                continue;
            }
        };
        if !current.removable
            || current.size_bytes != item.size_bytes
            || current.fingerprint != item.fingerprint
        {
            result.retained.push(format!(
                "{}: in use or changed since preview; scan again",
                item.id
            ));
            continue;
        }
        // Windows cannot delete an open lease; admission prevents a new MSC
        // owner from being admitted while the lease is closed for removal.
        drop(lease);
        match fs::remove_dir_all(root.join(&item.id)) {
            Ok(()) => {
                result.removed_bytes = result.removed_bytes.saturating_add(current.size_bytes);
                result.removed.push(item.id.clone());
            }
            Err(error) => result
                .retained
                .push(format!("{}: cleanup incomplete: {error}", item.id)),
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn abandoned(root: &Path) -> String {
        let id = format!("java-renderer-{}", Uuid::new_v4());
        let path = root.join(&id);
        fs::create_dir(&path).unwrap();
        fs::write(path.join("lease"), b"").unwrap();
        fs::write(path.join("reservation"), b"100").unwrap();
        fs::write(path.join("world-copy"), b"rebuild").unwrap();
        id
    }

    #[test]
    fn cleanup_retains_live_or_changed_copies_and_removes_only_approved_data() {
        let root = std::env::temp_dir().join(format!("msc-cleanup-test-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let source = root.join("original-world");
        fs::write(&source, b"original").unwrap();
        let id = abandoned(&root);
        let path = root.join(&id);
        let (approved, lease) = inspect_entry(&root, &id, "").unwrap();
        drop(lease);
        let result = execute_cleanup_with_processes(
            &root,
            std::slice::from_ref(&approved),
            &format!("renderer --out {}", path.display()),
        )
        .unwrap();
        assert!(result.removed.is_empty());
        assert!(path.exists());
        let active = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path.join("lease"))
            .unwrap();
        active.lock_exclusive().unwrap();
        assert!(!inspect_entry(&root, &id, "").unwrap().0.removable);
        drop(active);
        fs::write(path.join("new-file"), b"changed").unwrap();
        assert!(
            execute_cleanup_with_processes(&root, &[approved], "")
                .unwrap()
                .removed
                .is_empty()
        );
        let (approved, lease) = inspect_entry(&root, &id, "").unwrap();
        drop(lease);
        assert_eq!(
            execute_cleanup_with_processes(&root, &[approved], "")
                .unwrap()
                .removed,
            vec![id]
        );
        assert!(!path.exists());
        assert_eq!(fs::read(&source).unwrap(), b"original");
        assert!(inspect_entry(&root, "../original-world", "").is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_refuses_symlinked_worlds() {
        let root = std::env::temp_dir().join(format!("msc-cleanup-test-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let source = root.join("original-world");
        fs::write(&source, b"original").unwrap();
        let id = abandoned(&root);
        std::os::unix::fs::symlink(&source, root.join(&id).join("unsafe-link")).unwrap();
        assert!(inspect_entry(&root, &id, "").is_err());
        assert_eq!(fs::read(&source).unwrap(), b"original");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn staging_guard_removes_only_its_owned_copy() {
        let root = std::env::temp_dir().join(format!("msc-staging-test-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let source = root.join("original-world");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("level.dat"), b"original").unwrap();
        let abandoned = root.join("java-renderer-abandoned");
        fs::create_dir(&abandoned).unwrap();
        fs::write(abandoned.join("lease"), b"1").unwrap();
        fs::write(abandoned.join("world-copy"), b"retain").unwrap();
        let owned = root.join("snapshot-owned");
        fs::create_dir(&owned).unwrap();
        let lease = File::create(owned.join("lease")).unwrap();
        lease.lock_exclusive().unwrap();
        let staging = Staging {
            path: owned.clone(),
            lease: Some(lease),
        };
        fs::write(owned.join("world-copy"), b"copy").unwrap();
        drop(staging);
        assert!(!owned.exists());
        assert_eq!(fs::read(source.join("level.dat")).unwrap(), b"original");
        assert_eq!(fs::read(abandoned.join("world-copy")).unwrap(), b"retain");
        fs::remove_dir_all(root).unwrap();
    }
}
