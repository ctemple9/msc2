//! Transactional Java resource-pack publication.

use msc_domain::networking::hosted_resource_pack_url;
use msc_infrastructure::atomic_write::atomic_write;
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::resource_pack_store::{
    ApprovedResourcePack, ResourcePackStore, ResourcePackStoreError,
};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ResourcePackServiceError {
    Store(ResourcePackStoreError),
    InvalidHost,
    Properties(std::io::Error),
    RollbackFailed { original: String, rollback: String },
}

impl fmt::Display for ResourcePackServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => write!(f, "{error}"),
            Self::InvalidHost => write!(f, "resource-pack host is invalid"),
            Self::Properties(error) => write!(f, "{error}"),
            Self::RollbackFailed { original, rollback } => {
                write!(f, "{original}; rollback also failed: {rollback}")
            }
        }
    }
}
impl std::error::Error for ResourcePackServiceError {}

pub struct ResourcePackService<'a> {
    server_dir: PathBuf,
    fs: &'a dyn FileSystem,
}

impl<'a> ResourcePackService<'a> {
    pub fn new(server_dir: impl AsRef<Path>, fs: &'a dyn FileSystem) -> Self {
        Self {
            server_dir: server_dir.as_ref().to_path_buf(),
            fs,
        }
    }

    /// Publishes bytes first, then atomically changes Java's one active-pack
    /// setting. If that configuration change cannot commit, the prior pack is
    /// restored (or a newly-added pack is removed).
    pub fn publish_and_activate(
        &self,
        file_name: &str,
        bytes: &[u8],
        host: &str,
        port: u16,
        require: bool,
    ) -> Result<ApprovedResourcePack, ResourcePackServiceError> {
        let store = ResourcePackStore::new(&self.server_dir);
        let receipt = store
            .publish(file_name, bytes)
            .map_err(ResourcePackServiceError::Store)?;
        let url = hosted_resource_pack_url(host, port, file_name)
            .map_err(|_| ResourcePackServiceError::InvalidHost)?;
        if let Err(error) = self.write_active_properties(Some((&url, &receipt.pack.sha1, require)))
        {
            return match store.rollback_publish(&receipt) {
                Ok(()) => Err(ResourcePackServiceError::Properties(error)),
                Err(rollback) => Err(ResourcePackServiceError::RollbackFailed {
                    original: error.to_string(),
                    rollback: rollback.to_string(),
                }),
            };
        }
        Ok(receipt.pack)
    }

    pub fn disable(&self) -> Result<(), ResourcePackServiceError> {
        self.write_active_properties(None)
            .map_err(ResourcePackServiceError::Properties)
    }

    /// Applies a caller-supplied hosted URL without copying bytes into the
    /// approved local store.  This is the explicit custom-URL escape hatch in
    /// the API contract; it still changes only the three resource-pack keys,
    /// through the same atomic properties write as local publication.
    pub fn set_external_url(
        &self,
        url: &str,
        sha1: Option<&str>,
        require: bool,
    ) -> Result<(), ResourcePackServiceError> {
        if url.trim().is_empty() || url.contains(['\n', '\r']) {
            return Err(ResourcePackServiceError::InvalidHost);
        }
        self.write_active_properties(Some((url.trim(), sha1.unwrap_or(""), require)))
            .map_err(ResourcePackServiceError::Properties)
    }

    pub fn remove(&self, file_name: &str) -> Result<(), ResourcePackServiceError> {
        let store = ResourcePackStore::new(&self.server_dir);
        store
            .remove(file_name)
            .map_err(ResourcePackServiceError::Store)?;
        let props = read_properties(self.fs, &self.properties_path());
        if props
            .get("resource-pack")
            .is_some_and(|url| url.contains(file_name))
        {
            self.disable()?;
        }
        Ok(())
    }

    pub fn approved_bytes(&self, file_name: &str) -> Result<Vec<u8>, ResourcePackServiceError> {
        ResourcePackStore::new(&self.server_dir)
            .read_approved_bytes(file_name)
            .map_err(ResourcePackServiceError::Store)
    }

    fn properties_path(&self) -> PathBuf {
        self.server_dir.join("server.properties")
    }

    fn write_active_properties(
        &self,
        active: Option<(&str, &str, bool)>,
    ) -> Result<(), std::io::Error> {
        let path = self.properties_path();
        let mut props = read_properties(self.fs, &path);
        match active {
            Some((url, sha1, require)) => {
                props.insert("resource-pack".into(), url.into());
                props.insert("resource-pack-sha1".into(), sha1.into());
                props.insert("require-resource-pack".into(), require.to_string());
            }
            None => {
                props.insert("resource-pack".into(), String::new());
                props.insert("resource-pack-sha1".into(), String::new());
                props.insert("require-resource-pack".into(), "false".into());
            }
        }
        let mut output = String::from("# Modified via MSC 2\n");
        for (key, value) in props {
            output.push_str(&format!("{key}={value}\n"));
        }
        atomic_write(self.fs, &path, output.as_bytes()).map_err(|error| match error {
            msc_infrastructure::atomic_write::AtomicWriteError::Io(error) => error,
            msc_infrastructure::atomic_write::AtomicWriteError::MissingParentDirectory(path) => {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("{} does not exist", path.display()),
                )
            }
        })
    }
}

fn read_properties(fs: &dyn FileSystem, path: &Path) -> BTreeMap<String, String> {
    let Ok(bytes) = fs.read(path) else {
        return BTreeMap::new();
    };
    String::from_utf8_lossy(&bytes)
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            (!line.is_empty() && !line.starts_with('#'))
                .then(|| line.split_once('='))
                .flatten()
                .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}
