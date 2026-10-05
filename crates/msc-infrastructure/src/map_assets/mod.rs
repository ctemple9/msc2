//! Bounded read-only inputs and a private immutable diagnostic-generation store.
pub mod acquire;
pub mod adapter;
pub mod adoption;
pub mod bundle;
pub mod capture_client;
pub mod compose;
pub mod inventory;
pub mod resolver;
pub mod saved_terrain;
pub mod store;
pub mod supplemental;
use sha2::{Digest, Sha256};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read};
use std::path::{Component, Path};

pub const MAX_ARCHIVE: u64 = 2 * 1024 * 1024 * 1024;
pub const MAX_ENTRY: u64 = 256 * 1024 * 1024;
pub const MAX_SCAN: u64 = 16 * 1024 * 1024 * 1024;
pub const MAX_JSON: u64 = 8 * 1024 * 1024;
pub const MAX_DOCUMENTS: u64 = 256 * 1024 * 1024;

pub fn error(code: &str) -> io::Error {
    io::Error::other(code.to_string())
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn hash_json(value: &impl serde::Serialize) -> io::Result<String> {
    Ok(hash(
        &serde_json::to_vec(value).map_err(|_| error("serialization_failed"))?,
    ))
}
pub fn poll(cancel: &dyn Fn() -> bool) -> io::Result<()> {
    if cancel() {
        Err(error("cancelled"))
    } else {
        Ok(())
    }
}
pub fn safe_member(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains('\\')
        && name.len() <= 1024
        && !name
            .chars()
            .any(|c| c.is_control() || ":<>\"|?*".contains(c))
        && name.trim_end_matches('/').split('/').all(|part| {
            let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.ends_with([' ', '.'])
                && !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                && !(stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && stem.as_bytes()[3].is_ascii_digit())
        })
}
pub fn safe_path(path: &Path) -> io::Result<()> {
    let mut current = std::path::PathBuf::new();
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(error("unsafe_path"));
        }
        current.push(component);
        let metadata = fs::symlink_metadata(&current)?;
        if metadata.file_type().is_symlink() {
            return Err(error("linked_input"));
        }
    }
    Ok(())
}
pub fn open(path: &Path) -> io::Result<File> {
    safe_path(path)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(error("non_regular_input"));
    }
    Ok(file)
}
pub fn stamp(metadata: &Metadata) -> String {
    let mut identity = format!(
        "{}:{:?}:{:?}",
        metadata.len(),
        metadata.modified().ok(),
        metadata.created().ok()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        identity.push_str(&format!(
            ":{}:{}:{}:{}",
            metadata.dev(),
            metadata.ino(),
            metadata.ctime(),
            metadata.ctime_nsec()
        ));
    }
    identity
}
pub fn read(path: &Path, max: u64) -> io::Result<Vec<u8>> {
    let file = open(path)?;
    let before = stamp(&file.metadata()?);
    if file.metadata()?.len() > max {
        return Err(error("input_byte_limit"));
    }
    let mut raw = Vec::new();
    (&file).take(max + 1).read_to_end(&mut raw)?;
    if raw.len() as u64 > max {
        return Err(error("input_byte_limit"));
    }
    if before != stamp(&file.metadata()?) || before != stamp(&fs::symlink_metadata(path)?) {
        return Err(error("input_changed"));
    }
    Ok(raw)
}
pub fn file_hash(path: &Path, max: u64, cancel: &dyn Fn() -> bool) -> io::Result<String> {
    let mut file = open(path)?;
    let before = stamp(&file.metadata()?);
    let mut bytes = 0u64;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        poll(cancel)?;
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        if bytes > max {
            return Err(error("input_byte_limit"));
        }
        digest.update(&buffer[..n]);
    }
    if before != stamp(&file.metadata()?) || before != stamp(&fs::symlink_metadata(path)?) {
        return Err(error("input_changed"));
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub fn available_space(path: impl AsRef<Path>) -> io::Result<u64> {
    fs2::available_space(path)
}
