//! Resource budgets for disk-backed world work. These are estimates, not
//! reservations: writes must still handle a disk filling after a successful check.
use std::io;
use std::path::Path;

pub const GIB: u64 = 1024 * 1024 * 1024;
pub const DISK_RESERVE: u64 = 2 * GIB;

/// Old privileged macOS Bedrock runs could create root-owned database trees.
/// Refuse before expensive conversion, rather than discovering this at cleanup.
pub fn require_world_cleanup_ownership(path: &Path) -> io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        if metadata.file_type().is_symlink() {
            return Err(io::Error::other(format!(
                "World path is a symbolic link: {}",
                path.display()
            )));
        }
        let owner = unsafe { libc::geteuid() };
        if metadata.uid() != owner {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "World data at {} belongs to user ID {}, but MSC runs as user ID {owner}. Repair this server's ownership before modifying its world.",
                    path.display(),
                    metadata.uid()
                ),
            ));
        }
        if metadata.is_dir() {
            for entry in std::fs::read_dir(path)? {
                require_world_cleanup_ownership(&entry?.path())?;
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = path;
    Ok(())
}

pub fn check_space(available: u64, bytes: u64, stage: &str) -> io::Result<()> {
    let required = bytes.saturating_add(DISK_RESERVE);
    if available < required {
        return Err(io::Error::other(format!(
            "Not enough disk space for {stage}: need approximately {:.1} GiB including a 2 GiB reserve; {:.1} GiB available",
            required as f64 / GIB as f64,
            available as f64 / GIB as f64,
        )));
    }
    Ok(())
}

pub fn require_space(path: &Path, bytes: u64, stage: &str) -> io::Result<()> {
    let mut existing = path;
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| {
            io::Error::other(format!("Cannot locate disk for {}", path.display()))
        })?;
    }
    check_space(fs2::available_space(existing)?, bytes, stage)
        .map_err(|error| io::Error::other(format!("{error} (location: {})", path.display())))
}

/// Symlinks are not followed, so an unrelated tree cannot inflate the estimate
/// or cause a recursive cycle. Destination safety remains the archive's job.
pub fn tree_size(path: &Path) -> io::Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Ok(0);
    }
    let mut total = 0_u64;
    for entry in std::fs::read_dir(path)? {
        total = total.saturating_add(tree_size(&entry?.path())?);
    }
    Ok(total)
}

/// Leave half the currently available memory for the OS, MSC and JVM native
/// allocations; cap the heap at 4 GiB. Never silently launch an unbounded JVM.
pub fn heap_mib(available: u64) -> Result<u64, String> {
    let budget = (available / 2 / (1024 * 1024)).min(4096);
    if budget < 512 {
        return Err("Not enough available memory for conversion: close other apps or stop other servers, then retry (at least 1 GiB available memory required).".into());
    }
    Ok(budget)
}

pub fn available_heap_mib() -> Result<u64, String> {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    heap_mib(system.available_memory())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn budgets_preserve_reserves_and_reject_shortages() {
        assert!(check_space(14 * GIB, 12 * GIB, "conversion").is_ok());
        assert!(check_space(14 * GIB - 1, 12 * GIB, "conversion").is_err());
        assert!(check_space(u64::MAX - 1, u64::MAX, "conversion").is_err());
        assert!(heap_mib(GIB - 1).is_err());
        assert_eq!(heap_mib(GIB), Ok(512));
        assert_eq!(heap_mib(4 * GIB), Ok(2048));
        assert_eq!(heap_mib(32 * GIB), Ok(4096));
    }
}

/// Recheck free space every 64 MiB while extracting or packaging, so another
/// process consuming disk space cannot turn a stale preflight into a full disk.
pub struct ReservedFile {
    file: std::fs::File,
    path: std::path::PathBuf,
    until_check: usize,
}
impl ReservedFile {
    pub fn create(path: &Path) -> io::Result<Self> {
        require_space(path, 0, "writing world data")?;
        Ok(Self {
            file: std::fs::File::create(path)?,
            path: path.into(),
            until_check: 0,
        })
    }
}
impl io::Write for ReservedFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.until_check == 0 {
            require_space(&self.path, 64 * 1024 * 1024, "writing world data")?;
            self.until_check = 64 * 1024 * 1024;
        }
        let count = self
            .file
            .write(&bytes[..bytes.len().min(self.until_check)])?;
        self.until_check -= count;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
impl io::Seek for ReservedFile {
    fn seek(&mut self, position: io::SeekFrom) -> io::Result<u64> {
        self.file.seek(position)
    }
}
