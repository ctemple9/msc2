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
                    reserved = reserved.saturating_add(
                        value
                            .trim()
                            .parse::<u64>()
                            .map_err(|_| io::Error::other("invalid staging reservation"))?,
                    );
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

#[cfg(test)]
mod tests {
    use super::*;
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
