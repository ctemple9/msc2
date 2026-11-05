//! Host exports are confined to the selected server's exports directory, inside
//! the existing admin-authorized server filesystem root. They are never writes
//! to caller-supplied absolute paths or runtime content.
use msc_domain::app_config_schema::ConfigServer;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostDestination {
    pub relative_path: String,
    pub replace_sha512: Option<String>,
}
fn root(server: &ConfigServer) -> Result<PathBuf, String> {
    let server_root = std::fs::canonicalize(&server.server_dir).map_err(|e| e.to_string())?;
    let root = server_root.join("exports");
    match std::fs::symlink_metadata(&root) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return Err("Host exports must be a regular directory, without symlinks".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(&root).map_err(|e| e.to_string())?;
        }
        Err(e) => return Err(e.to_string()),
    }
    Ok(root)
}
pub fn target(server: &ConfigServer, request: &HostDestination) -> Result<PathBuf, String> {
    msc_domain::content_collection::validate_relative_path(&request.relative_path)?;
    if !request.relative_path.ends_with(".zip") {
        return Err("Choose a .zip export filename".into());
    }
    if request
        .replace_sha512
        .as_ref()
        .is_some_and(|hash| hash.len() != 128 || !hash.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err("Overwrite consent needs the existing file's exact SHA512".into());
    }
    let root = root(server)?;
    let target = root.join(&request.relative_path);
    let mut current = root.clone();
    let parent = target.parent().ok_or("Export destination has no parent")?;
    for part in parent
        .strip_prefix(&root)
        .map_err(|e| e.to_string())?
        .components()
    {
        current.push(part);
        let metadata = std::fs::symlink_metadata(&current)
            .map_err(|_| "Choose an existing export subdirectory")?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("Export directory must not contain symlinks".into());
        }
    }
    Ok(target)
}
fn identity(path: &Path) -> Result<Option<String>, String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            use sha2::{Digest, Sha512};
            use std::io::Read;
            if metadata.len() > 64 * 1024 * 1024 * 1024 {
                return Err("Existing export exceeds the 64 GiB review limit".into());
            }
            let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let mut digest = Sha512::new();
            let mut buffer = [0u8; 128 * 1024];
            let mut size = 0u64;
            loop {
                let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
                if count == 0 {
                    break;
                }
                size += count as u64;
                if size > metadata.len() {
                    return Err("Existing export changed during review".into());
                }
                digest.update(&buffer[..count]);
            }
            if size != metadata.len() {
                return Err("Existing export changed during review".into());
            }
            Ok(Some(format!("{:x}", digest.finalize())))
        }
        Ok(_) => Err("Choose a regular export destination, without symlinks".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
pub fn inspect(server: &ConfigServer, relative_path: String) -> Result<serde_json::Value, String> {
    let request = HostDestination {
        relative_path,
        replace_sha512: None,
    };
    let path = target(server, &request)?;
    Ok(
        serde_json::json!({"root":root(server)?,"relativePath":request.relative_path,"existingSha512":identity(&path)?}),
    )
}
pub fn validate(server: &ConfigServer, request: &HostDestination) -> Result<PathBuf, String> {
    let path = target(server, request)?;
    if identity(&path)? != request.replace_sha512 {
        return Err("Host destination changed or exists without matching overwrite consent; review it again".into());
    }
    Ok(path)
}
/// The archive is generated beside the destination so final publication stays
/// atomic on the same filesystem. Recheck the destination and authorization at
/// the final boundary; failed generation never touches an existing file.
pub fn commit(
    server: &ConfigServer,
    request: &HostDestination,
    archive: &Path,
    cancelled: &dyn Fn() -> bool,
) -> Result<PathBuf, String> {
    let path = validate(server, request)?;
    if archive.parent() != path.parent() {
        return Err("Host export staging directory changed".into());
    }
    if cancelled() {
        return Err("Host save cancelled before completion".into());
    }
    if request.replace_sha512.is_some() {
        std::fs::rename(archive, &path).map_err(|e| e.to_string())?;
    } else {
        std::fs::hard_link(archive, &path).map_err(|e| e.to_string())?;
        std::fs::remove_file(archive)
            .map_err(|e| format!("Host export saved but private staging cleanup failed: {e}"))?;
    }
    #[cfg(unix)]
    std::fs::File::open(path.parent().ok_or("Export has no parent")?)
        .and_then(|file| file.sync_all())
        .map_err(|e| format!("Host export saved but directory sync failed: {e}"))?;
    Ok(path)
}
