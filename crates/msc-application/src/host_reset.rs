//! Host-reset filesystem workflow.
//!
//! The agent owns the orchestration and authorization, but this module owns
//! the destructive filesystem boundary. A small marker is written before any
//! deletion and removed only after the agent has revoked credentials and
//! rotated its host identity. If the process stops between those points, the
//! next boot can finish the same idempotent filesystem work before serving
//! the old configuration.

use msc_infrastructure::fs::FileSystem;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Component, Path, PathBuf};

const MARKER_FILE_NAME: &str = ".msc2-host-reset.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostResetMode {
    Configuration,
    Everything,
}

impl HostResetMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::Everything => "everything",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostResetError {
    InvalidServersRoot(String),
    Io(String),
    InvalidMarker(String),
}

impl std::fmt::Display for HostResetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidServersRoot(message)
            | Self::Io(message)
            | Self::InvalidMarker(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for HostResetError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResetMarker {
    mode: HostResetMode,
    servers_root: PathBuf,
    #[serde(default)]
    helper_cache: Option<PathBuf>,
}

pub struct HostResetWorkflow<'fs> {
    fs: &'fs dyn FileSystem,
    config_path: PathBuf,
    servers_root: PathBuf,
    helper_cache: Option<PathBuf>,
}

impl<'fs> HostResetWorkflow<'fs> {
    pub fn new(
        fs: &'fs dyn FileSystem,
        config_path: impl Into<PathBuf>,
        servers_root: impl Into<PathBuf>,
    ) -> Result<Self, HostResetError> {
        let servers_root = servers_root.into();
        validate_reset_root(&servers_root)?;
        Ok(Self {
            fs,
            config_path: config_path.into(),
            servers_root,
            helper_cache: None,
        })
    }

    /// Adds the agent-owned downloaded-helper cache to a full reset without
    /// changing the existing constructor used by other reset tests/callers.
    pub fn with_helper_cache(
        mut self,
        helper_cache: impl Into<PathBuf>,
    ) -> Result<Self, HostResetError> {
        let helper_cache = helper_cache.into();
        validate_reset_root(&helper_cache)?;
        self.helper_cache = Some(helper_cache);
        Ok(self)
    }

    pub fn begin(&self, mode: HostResetMode) -> Result<(), HostResetError> {
        let marker = ResetMarker {
            mode,
            servers_root: self.servers_root.clone(),
            helper_cache: self.helper_cache.clone(),
        };
        let bytes = serde_json::to_vec(&marker)
            .map_err(|error| HostResetError::InvalidMarker(error.to_string()))?;
        self.fs
            .write(&marker_path(&self.config_path), &bytes)
            .map_err(io_error)
    }

    /// Applies only the paths named by the reset contract. Missing paths are
    /// already in the requested post-reset state and are therefore ignored.
    pub fn apply_files(&self, mode: HostResetMode) -> Result<(), HostResetError> {
        remove_if_present(self.fs, &self.config_path)?;
        if mode == HostResetMode::Everything {
            remove_if_present(self.fs, &self.servers_root)?;
            if let Some(helper_cache) = &self.helper_cache {
                remove_if_present(self.fs, helper_cache)?;
            }
        }
        Ok(())
    }

    pub fn finish(&self) -> Result<(), HostResetError> {
        remove_if_present(self.fs, &marker_path(&self.config_path))
    }
}

/// Completes an interrupted filesystem half before the normal app config is
/// loaded. The marker contains the original configured root, so a deleted
/// config cannot redirect recovery to a different path.
pub fn recover_files(
    fs: &dyn FileSystem,
    config_path: impl Into<PathBuf>,
) -> Result<Option<HostResetMode>, HostResetError> {
    let config_path = config_path.into();
    let marker_path = marker_path(&config_path);
    let bytes = match fs.read(&marker_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error(error)),
    };
    let marker: ResetMarker = serde_json::from_slice(&bytes)
        .map_err(|error| HostResetError::InvalidMarker(error.to_string()))?;
    let mut workflow = HostResetWorkflow::new(fs, config_path, marker.servers_root)?;
    if let Some(helper_cache) = marker.helper_cache {
        workflow = workflow.with_helper_cache(helper_cache)?;
    }
    let mode = marker.mode;
    workflow.apply_files(mode)?;
    Ok(Some(mode))
}

pub fn finish_recovery(
    fs: &dyn FileSystem,
    config_path: impl Into<PathBuf>,
) -> Result<(), HostResetError> {
    remove_if_present(fs, &marker_path(&config_path.into()))
}

fn marker_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(MARKER_FILE_NAME)
}

fn remove_if_present(fs: &dyn FileSystem, path: &Path) -> Result<(), HostResetError> {
    match fs.remove(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(error)),
    }
}

fn validate_reset_root(path: &Path) -> Result<(), HostResetError> {
    let components: Vec<Component<'_>> = path.components().collect();
    let normal_count = components
        .iter()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count();
    if path.as_os_str().is_empty()
        || normal_count < 2
        || components
            .iter()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(HostResetError::InvalidServersRoot(format!(
            "refusing to reset an unsafe host-data root: {}",
            path.display()
        )));
    }
    Ok(())
}

fn io_error(error: io::Error) -> HostResetError {
    HostResetError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use msc_infrastructure::fs::{FakeFileSystem, FileSystem};

    #[test]
    fn configuration_reset_preserves_servers_root() {
        let fs: &'static dyn FileSystem = Box::leak(Box::new(
            FakeFileSystem::new()
                .with_dir("/agent")
                .with_file("/agent/config.json", b"old".to_vec(), false)
                .with_file("/srv/msc2/servers/paper/world", b"world".to_vec(), false),
        ));
        let workflow =
            HostResetWorkflow::new(fs, "/agent/config.json", "/srv/msc2/servers").unwrap();
        workflow.begin(HostResetMode::Configuration).unwrap();
        workflow.apply_files(HostResetMode::Configuration).unwrap();
        assert!(fs.stat(Path::new("/agent/config.json")).is_err());
        assert!(fs.stat(Path::new("/srv/msc2/servers/paper/world")).is_ok());
        workflow.finish().unwrap();
        assert!(fs.stat(Path::new("/agent/.msc2-host-reset.json")).is_err());
    }

    #[test]
    fn everything_reset_removes_only_the_configured_tree() {
        let fs: &'static dyn FileSystem = Box::leak(Box::new(
            FakeFileSystem::new()
                .with_dir("/agent")
                .with_file("/agent/config.json", b"old".to_vec(), false)
                .with_file("/srv/msc2/servers/paper/world", b"world".to_vec(), false)
                .with_file("/srv/msc2/other/keep", b"keep".to_vec(), false),
        ));
        let workflow =
            HostResetWorkflow::new(fs, "/agent/config.json", "/srv/msc2/servers").unwrap();
        workflow.apply_files(HostResetMode::Everything).unwrap();
        assert!(fs.stat(Path::new("/srv/msc2/servers/paper/world")).is_err());
        assert!(fs.stat(Path::new("/srv/msc2/other/keep")).is_ok());
    }

    #[test]
    fn recovery_replays_marker_and_file_deletion() {
        let fs: &'static dyn FileSystem = Box::leak(Box::new(
            FakeFileSystem::new()
                .with_dir("/agent")
                .with_file("/agent/config.json", b"old".to_vec(), false)
                .with_file("/srv/msc2/servers/paper/world", b"world".to_vec(), false),
        ));
        let workflow =
            HostResetWorkflow::new(fs, "/agent/config.json", "/srv/msc2/servers").unwrap();
        workflow.begin(HostResetMode::Everything).unwrap();
        let mode = recover_files(fs, "/agent/config.json").unwrap();
        assert_eq!(mode, Some(HostResetMode::Everything));
        assert!(fs.stat(Path::new("/agent/config.json")).is_err());
        assert!(fs.stat(Path::new("/srv/msc2/servers/paper/world")).is_err());
    }
}
