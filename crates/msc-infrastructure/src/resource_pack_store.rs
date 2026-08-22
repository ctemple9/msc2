//! A deliberately narrow on-disk store for Java resource packs.
//!
//! A public pack listener must ask this store for an approved file name; it
//! never receives a server-relative path from a request.  That makes the
//! listener incapable of accidentally becoming a general server file browser.

use msc_domain::networking::{ResourcePackError, resource_pack_sha1, validate_java_pack_filename};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Large enough for normal high-resolution packs while putting a firm bound on
/// both upload memory and the bytes a tiny pack listener may return.
pub const RESOURCE_PACK_MAX_BYTES: usize = 100 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedResourcePack {
    pub file_name: String,
    pub path: PathBuf,
    pub sha1: String,
    pub size: usize,
}

#[derive(Debug)]
pub enum ResourcePackStoreError {
    InvalidName(ResourcePackError),
    TooLarge { size: usize, maximum: usize },
    NotFound,
    Io(std::io::Error),
}

impl fmt::Display for ResourcePackStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(_) => write!(f, "resource-pack file name is unsafe or is not a .zip"),
            Self::TooLarge { size, maximum } => {
                write!(f, "resource pack is {size} bytes; maximum is {maximum}")
            }
            Self::NotFound => write!(f, "approved resource pack was not found"),
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ResourcePackStoreError {}

/// The staging record keeps enough prior state for the application transaction
/// to restore a replacement when the matching properties write cannot commit.
#[derive(Debug)]
pub struct PublishReceipt {
    pub pack: ApprovedResourcePack,
    previous_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct ResourcePackStore {
    directory: PathBuf,
}

impl ResourcePackStore {
    pub fn new(server_dir: impl AsRef<Path>) -> Self {
        Self {
            directory: server_dir.as_ref().join("resource-packs"),
        }
    }

    pub fn publish(
        &self,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<PublishReceipt, ResourcePackStoreError> {
        validate_name_and_size(file_name, bytes.len())?;
        fs::create_dir_all(&self.directory).map_err(ResourcePackStoreError::Io)?;
        let target = self.directory.join(file_name);
        let previous_bytes = match fs::read(&target) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(ResourcePackStoreError::Io(error)),
        };
        let staged = self.directory.join(format!(".{file_name}.staged"));
        fs::write(&staged, bytes).map_err(ResourcePackStoreError::Io)?;
        fs::rename(&staged, &target).map_err(|error| {
            let _ = fs::remove_file(&staged);
            ResourcePackStoreError::Io(error)
        })?;
        Ok(PublishReceipt {
            pack: ApprovedResourcePack {
                file_name: file_name.to_owned(),
                path: target,
                sha1: resource_pack_sha1(bytes),
                size: bytes.len(),
            },
            previous_bytes,
        })
    }

    pub fn rollback_publish(&self, receipt: &PublishReceipt) -> Result<(), ResourcePackStoreError> {
        match &receipt.previous_bytes {
            Some(bytes) => self.replace_atomically(&receipt.pack.file_name, bytes),
            None => match fs::remove_file(&receipt.pack.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(ResourcePackStoreError::Io(error)),
            },
        }
    }

    /// Returns bytes only for a name already admitted to the flat pack store.
    /// A caller cannot turn this into `../server.properties` or another path.
    pub fn approved_file(
        &self,
        file_name: &str,
    ) -> Result<ApprovedResourcePack, ResourcePackStoreError> {
        validate_java_pack_filename(file_name).map_err(ResourcePackStoreError::InvalidName)?;
        let path = self.directory.join(file_name);
        let bytes = fs::read(&path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ResourcePackStoreError::NotFound
            } else {
                ResourcePackStoreError::Io(error)
            }
        })?;
        validate_name_and_size(file_name, bytes.len())?;
        Ok(ApprovedResourcePack {
            file_name: file_name.to_owned(),
            path,
            sha1: resource_pack_sha1(&bytes),
            size: bytes.len(),
        })
    }

    pub fn read_approved_bytes(&self, file_name: &str) -> Result<Vec<u8>, ResourcePackStoreError> {
        let approved = self.approved_file(file_name)?;
        fs::read(approved.path).map_err(ResourcePackStoreError::Io)
    }

    pub fn remove(&self, file_name: &str) -> Result<(), ResourcePackStoreError> {
        validate_java_pack_filename(file_name).map_err(ResourcePackStoreError::InvalidName)?;
        fs::remove_file(self.directory.join(file_name)).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ResourcePackStoreError::NotFound
            } else {
                ResourcePackStoreError::Io(error)
            }
        })
    }

    fn replace_atomically(
        &self,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<(), ResourcePackStoreError> {
        let staged = self.directory.join(format!(".{file_name}.rollback"));
        fs::write(&staged, bytes).map_err(ResourcePackStoreError::Io)?;
        fs::rename(&staged, self.directory.join(file_name)).map_err(ResourcePackStoreError::Io)
    }
}

fn validate_name_and_size(file_name: &str, size: usize) -> Result<(), ResourcePackStoreError> {
    validate_java_pack_filename(file_name).map_err(ResourcePackStoreError::InvalidName)?;
    if size > RESOURCE_PACK_MAX_BYTES {
        return Err(ResourcePackStoreError::TooLarge {
            size,
            maximum: RESOURCE_PACK_MAX_BYTES,
        });
    }
    Ok(())
}
