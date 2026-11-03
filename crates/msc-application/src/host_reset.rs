//! Host-reset filesystem workflow.
//!
//! The agent owns the orchestration and authorization, but this module owns
//! the destructive filesystem boundary. A small marker is written before any
//! deletion and removed only after the agent has revoked credentials and
//! rotated its host identity. If the process stops between those points, the
//! next boot resumes the same filesystem work. Explicit cancellation is
//! durable; a recorded failure pauses recovery and leaves management available.

use msc_infrastructure::atomic_write::atomic_write;
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
    Cancelled,
}

impl std::fmt::Display for HostResetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(
                f,
                "Reset cancelled. Files already removed cannot be restored."
            ),
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
    #[serde(default)]
    cancelled: bool,
    #[serde(default)]
    finalizing: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    operation_id: Option<String>,
    #[serde(default)]
    completion_token: Option<String>,
}

// Serialize each deletion boundary with durable cancellation. Never hold this
// lock for a whole tree: cancellation takes effect before the next entry.
static RESET_BOUNDARY: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetStatus {
    pub state: String,
    pub mode: Option<String>,
    pub message: String,
    pub can_cancel: bool,
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
        self.begin_for_operation(mode, None)
    }

    pub fn begin_for_operation(
        &self,
        mode: HostResetMode,
        operation_id: Option<String>,
    ) -> Result<(), HostResetError> {
        let _boundary = RESET_BOUNDARY.lock().unwrap();
        if let Some(mut pending) = read_marker(self.fs, &self.config_path)?
            && !pending.cancelled
        {
            if pending.mode != mode || pending.servers_root != self.servers_root {
                return Err(HostResetError::InvalidMarker(
                    "Resume the original pending reset mode and server folder, or cancel it first."
                        .into(),
                ));
            }
            pending.error = None;
            pending.operation_id = operation_id;
            pending.completion_token = Some(uuid::Uuid::new_v4().to_string());
            write_receipt(self.fs, &self.config_path, &pending, "running")?;
            return save_marker(self.fs, &self.config_path, &pending);
        }
        let marker = ResetMarker {
            mode,
            servers_root: self.servers_root.clone(),
            helper_cache: self.helper_cache.clone(),
            cancelled: false,
            finalizing: false,
            error: None,
            operation_id,
            completion_token: Some(uuid::Uuid::new_v4().to_string()),
        };
        write_receipt(self.fs, &self.config_path, &marker, "running")?;
        let bytes = serde_json::to_vec(&marker)
            .map_err(|error| HostResetError::InvalidMarker(error.to_string()))?;
        atomic_write(self.fs, &marker_path(&self.config_path), &bytes)
            .map_err(|error| HostResetError::Io(error.to_string()))
    }

    /// Applies only the paths named by the reset contract. Missing paths are
    /// already in the requested post-reset state and are therefore ignored.
    pub fn apply_files(&self, mode: HostResetMode) -> Result<(), HostResetError> {
        self.apply_files_cancellable(mode, &|| false)
    }

    pub fn apply_files_cancellable(
        &self,
        mode: HostResetMode,
        should_cancel: &dyn Fn() -> bool,
    ) -> Result<(), HostResetError> {
        let result: Result<(), HostResetError> = (|| {
            let finalizing =
                read_marker(self.fs, &self.config_path)?.is_some_and(|marker| marker.finalizing);
            if mode == HostResetMode::Everything && !finalizing {
                remove_tree(
                    self.fs,
                    &self.config_path,
                    &self.servers_root,
                    should_cancel,
                )?;
                if let Some(helper_cache) = &self.helper_cache {
                    remove_tree(self.fs, &self.config_path, helper_cache, should_cancel)?;
                }
            }
            let _boundary = RESET_BOUNDARY.lock().unwrap();
            check_cancel(self.fs, &self.config_path, should_cancel)?;
            remove_if_present(self.fs, &self.config_path)?;
            if let Some(mut marker) = read_marker(self.fs, &self.config_path)? {
                marker.finalizing = true;
                save_marker(self.fs, &self.config_path, &marker)?;
            }
            // After this boundary no further tree deletion occurs. Finish the
            // identity/credential transaction even if the desktop disconnects.
            Ok(())
        })();
        if let Err(error) = &result
            && *error != HostResetError::Cancelled
        {
            let _boundary = RESET_BOUNDARY.lock().unwrap();
            if let Some(mut marker) = read_marker(self.fs, &self.config_path)? {
                marker.error = Some(error.to_string());
                save_marker(self.fs, &self.config_path, &marker)?;
            }
        }
        result
    }

    pub fn finish(&self) -> Result<(), HostResetError> {
        finish_recovery(self.fs, &self.config_path)
    }

    pub fn completion_token(&self) -> Result<Option<String>, HostResetError> {
        Ok(read_marker(self.fs, &self.config_path)?.and_then(|marker| marker.completion_token))
    }

    pub fn pending(
        fs: &'fs dyn FileSystem,
        config_path: PathBuf,
    ) -> Result<Option<Self>, HostResetError> {
        let Some(marker) = read_marker(fs, &config_path)?.filter(|marker| !marker.cancelled) else {
            return Ok(None);
        };
        let mut workflow = Self::new(fs, config_path, marker.servers_root)?;
        if let Some(cache) = marker.helper_cache {
            workflow = workflow.with_helper_cache(cache)?;
        }
        Ok(Some(workflow))
    }
}

/// Completes an interrupted filesystem half before the normal app config is
/// loaded. The marker contains the original configured root, so a deleted
/// config cannot redirect recovery to a different path. Cancelled resets stay
/// cancelled; recorded failures wait for an explicit retry.
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
    if marker.cancelled {
        return Ok(None);
    }
    if let Some(error) = marker.error {
        return Err(HostResetError::Io(error));
    }
    let mut workflow = HostResetWorkflow::new(fs, config_path, marker.servers_root)?;
    if let Some(helper_cache) = marker.helper_cache {
        workflow = workflow.with_helper_cache(helper_cache)?;
    }
    workflow.apply_files(marker.mode)?;
    Ok(Some(marker.mode))
}

pub fn status(fs: &dyn FileSystem, config: &Path) -> Result<ResetStatus, HostResetError> {
    let marker = read_marker(fs, config)?;
    Ok(match marker {
        None => ResetStatus {
            state: "idle".into(),
            mode: None,
            message: String::new(),
            can_cancel: false,
        },
        Some(marker) => ResetStatus {
            state: if marker.cancelled {
                "cancelled"
            } else if marker.error.is_some() {
                "blocked"
            } else {
                "pending"
            }
            .into(),
            mode: Some(marker.mode.as_str().into()),
            message: marker.error.unwrap_or_else(|| {
                if marker.cancelled {
                    "Reset cancelled. Files already removed cannot be restored.".into()
                } else {
                    "An interrupted reset is pending recovery.".into()
                }
            }),
            can_cancel: !marker.cancelled && !marker.finalizing,
        },
    })
}

pub fn cancel_pending(
    fs: &dyn FileSystem,
    config: &Path,
    operation_id: Option<&str>,
) -> Result<(), HostResetError> {
    let _boundary = RESET_BOUNDARY.lock().unwrap();
    if let Some(mut marker) = read_marker(fs, config)? {
        if operation_id.is_some() && marker.operation_id.as_deref() != operation_id {
            return Err(HostResetError::InvalidMarker(
                "This operation does not own the pending reset.".into(),
            ));
        }
        if marker.finalizing {
            return Err(HostResetError::InvalidMarker("Reset file deletion is complete; identity cleanup must finish before another action.".into()));
        }
        marker.cancelled = true;
        marker.error = None;
        save_marker(fs, config, &marker)?;
        write_receipt(fs, config, &marker, "cancelled")?;
    } else {
        return Err(HostResetError::InvalidMarker(
            "There is no pending reset to cancel.".into(),
        ));
    }
    Ok(())
}

pub fn blocks_mutation(fs: &dyn FileSystem, config: &Path) -> bool {
    status(fs, config).map_or(true, |status| {
        matches!(status.state.as_str(), "pending" | "blocked")
    })
}

fn read_marker(fs: &dyn FileSystem, config: &Path) -> Result<Option<ResetMarker>, HostResetError> {
    match fs.read(&marker_path(config)) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| HostResetError::InvalidMarker(error.to_string())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(error)),
    }
}
fn save_marker(
    fs: &dyn FileSystem,
    config: &Path,
    marker: &ResetMarker,
) -> Result<(), HostResetError> {
    let bytes = serde_json::to_vec(marker)
        .map_err(|error| HostResetError::InvalidMarker(error.to_string()))?;
    atomic_write(fs, &marker_path(config), &bytes)
        .map_err(|error| HostResetError::Io(error.to_string()))
}
fn check_cancel(
    fs: &dyn FileSystem,
    config: &Path,
    should_cancel: &dyn Fn() -> bool,
) -> Result<(), HostResetError> {
    if let Some(mut marker) = read_marker(fs, config)? {
        if marker.cancelled {
            return Err(HostResetError::Cancelled);
        }
        if should_cancel() && !marker.finalizing {
            marker.cancelled = true;
            save_marker(fs, config, &marker)?;
            write_receipt(fs, config, &marker, "cancelled")?;
            return Err(HostResetError::Cancelled);
        }
    } else if should_cancel() {
        return Err(HostResetError::Cancelled);
    }
    Ok(())
}
fn remove_tree(
    fs: &dyn FileSystem,
    config: &Path,
    path: &Path,
    should_cancel: &dyn Fn() -> bool,
) -> Result<(), HostResetError> {
    {
        let _boundary = RESET_BOUNDARY.lock().unwrap();
        check_cancel(fs, config, should_cancel)?;
    }
    if fs.read_link(path).is_err() && fs.stat(path).is_ok_and(|metadata| metadata.is_dir) {
        for entry in fs.list(path).map_err(|error| {
            HostResetError::Io(format!("Cannot list {}: {error}", path.display()))
        })? {
            remove_tree(fs, config, &entry, should_cancel)?;
        }
    }
    let _boundary = RESET_BOUNDARY.lock().unwrap();
    check_cancel(fs, config, should_cancel)?;
    match fs.remove_entry(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(HostResetError::Io(format!(
            "Cannot remove {}: {error}",
            path.display()
        ))),
    }
}

pub fn finish_recovery(
    fs: &dyn FileSystem,
    config_path: impl Into<PathBuf>,
) -> Result<(), HostResetError> {
    let config_path = config_path.into();
    if let Some(marker) = read_marker(fs, &config_path)? {
        write_receipt(fs, &config_path, &marker, "succeeded")?;
    }
    remove_if_present(fs, &marker_path(&config_path))
}

#[derive(Serialize, Deserialize)]
struct ResetReceipt {
    token: String,
    state: String,
}

fn receipt_path(config: &Path) -> PathBuf {
    config.with_file_name("host-reset-result.json")
}
fn write_receipt(
    fs: &dyn FileSystem,
    config: &Path,
    marker: &ResetMarker,
    state: &str,
) -> Result<(), HostResetError> {
    let Some(token) = &marker.completion_token else {
        return Ok(());
    };
    let bytes = serde_json::to_vec(&ResetReceipt {
        token: token.clone(),
        state: state.into(),
    })
    .map_err(|error| HostResetError::Io(error.to_string()))?;
    atomic_write(fs, &receipt_path(config), &bytes)
        .map_err(|error| HostResetError::Io(error.to_string()))
}

/// A random, reset-specific read capability survives revocation of the old
/// administrator credential. It exposes only the outcome, never host data.
pub fn completion_state(fs: &dyn FileSystem, config: &Path, token: &str) -> Option<String> {
    let bytes = fs.read(&receipt_path(config)).ok()?;
    let receipt: ResetReceipt = serde_json::from_slice(&bytes).ok()?;
    (receipt.token == token).then_some(receipt.state)
}

pub fn record_failure(
    fs: &dyn FileSystem,
    config: &Path,
    message: &str,
) -> Result<(), HostResetError> {
    let _boundary = RESET_BOUNDARY.lock().unwrap();
    if let Some(mut marker) = read_marker(fs, config)?
        && !marker.cancelled
    {
        marker.error = Some(message.into());
        save_marker(fs, config, &marker)?;
        write_receipt(fs, config, &marker, "failed")?;
    }
    Ok(())
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
    fn reset_fs() -> FakeFileSystem {
        FakeFileSystem::new()
            .with_dir("/agent")
            .with_file("/agent/config.json", b"old".to_vec(), false)
            .with_file("/srv/msc2/servers/a", b"a".to_vec(), false)
            .with_file("/srv/msc2/servers/b", b"b".to_vec(), false)
    }

    #[test]
    fn cancellation_between_files_survives_recovery_and_preserves_configuration() {
        let fs = reset_fs();
        let workflow =
            HostResetWorkflow::new(&fs, "/agent/config.json", "/srv/msc2/servers").unwrap();
        workflow.begin(HostResetMode::Everything).unwrap();
        // Request cancellation only after the first file was really removed.
        let result = workflow.apply_files_cancellable(HostResetMode::Everything, &|| {
            fs.stat(Path::new("/srv/msc2/servers/a")).is_err()
        });
        assert_eq!(result, Err(HostResetError::Cancelled));
        assert!(fs.stat(Path::new("/srv/msc2/servers/b")).is_ok());
        assert!(fs.stat(Path::new("/agent/config.json")).is_ok());
        assert_eq!(recover_files(&fs, "/agent/config.json").unwrap(), None);
        assert!(fs.stat(Path::new("/srv/msc2/servers/b")).is_ok());
        let token = workflow.completion_token().unwrap().unwrap();
        assert_eq!(
            completion_state(&fs, Path::new("/agent/config.json"), &token).as_deref(),
            Some("cancelled")
        );
    }

    #[test]
    fn permission_failure_remains_blocked_until_explicit_retry_or_cancel() {
        let fs = reset_fs();
        let config = Path::new("/agent/config.json");
        let workflow = HostResetWorkflow::new(&fs, config, "/srv/msc2/servers").unwrap();
        workflow.begin(HostResetMode::Everything).unwrap();
        fs.set_remove_denied("/srv/msc2/servers/a", true);
        assert!(matches!(
            workflow.apply_files(HostResetMode::Everything),
            Err(HostResetError::Io(_))
        ));
        assert_eq!(status(&fs, config).unwrap().state, "blocked");
        assert!(fs.stat(config).is_ok());
        fs.set_remove_denied("/srv/msc2/servers/a", false);
        assert!(recover_files(&fs, config).is_err());
        assert!(fs.stat(Path::new("/srv/msc2/servers/b")).is_ok());
        cancel_pending(&fs, config, None).unwrap();
        assert_eq!(recover_files(&fs, config).unwrap(), None);
        workflow.begin(HostResetMode::Everything).unwrap();
        workflow.apply_files(HostResetMode::Everything).unwrap();
        let token = workflow.completion_token().unwrap().unwrap();
        assert_eq!(
            completion_state(&fs, config, &token).as_deref(),
            Some("running")
        );
        workflow.finish().unwrap();
        assert_eq!(
            completion_state(&fs, config, &token).as_deref(),
            Some("succeeded")
        );
        assert_eq!(completion_state(&fs, config, "wrong token"), None);
    }

    #[test]
    fn identity_finalization_resumes_without_repeating_tree_deletion() {
        let fs = reset_fs();
        let config = Path::new("/agent/config.json");
        let workflow = HostResetWorkflow::new(&fs, config, "/srv/msc2/servers").unwrap();
        workflow.begin(HostResetMode::Everything).unwrap();
        workflow.apply_files(HostResetMode::Everything).unwrap();
        assert!(cancel_pending(&fs, config, None).is_err());
        fs.write(
            Path::new("/srv/msc2/servers/keep"),
            b"created after deletion",
        )
        .unwrap();
        assert_eq!(
            recover_files(&fs, config).unwrap(),
            Some(HostResetMode::Everything)
        );
        assert!(fs.stat(Path::new("/srv/msc2/servers/keep")).is_ok());
    }
    #[test]
    fn authorized_operation_cancellation_is_durable_before_worker_observes_it() {
        use crate::operations::{
            HOST_RESET_OPERATION_TYPE, LifecycleOperationError, LifecycleOperations,
        };
        let fs = reset_fs().with_dir("/agent/reset-operation-journal");
        let config = Path::new("/agent/config.json");
        let operations = LifecycleOperations::new(&fs, "/agent/reset-operation-journal")
            .with_host_reset_config(config.into());
        let id = operations
            .begin_running_for_credential(
                HOST_RESET_OPERATION_TYPE,
                None,
                "reset",
                Some("owner".into()),
            )
            .unwrap();
        let workflow = HostResetWorkflow::new(&fs, config, "/srv/msc2/servers").unwrap();
        workflow
            .begin_for_operation(HostResetMode::Everything, Some(id.as_str().into()))
            .unwrap();
        assert!(matches!(
            operations.request_cancel_authorized(&id, "cancel", "guest", &[], false),
            Err(LifecycleOperationError::Forbidden)
        ));
        assert_eq!(status(&fs, config).unwrap().state, "pending");
        operations
            .request_cancel_authorized(&id, "cancel", "owner", &["admin".into()], true)
            .unwrap();
        assert_eq!(recover_files(&fs, config).unwrap(), None);
        assert!(fs.stat(Path::new("/srv/msc2/servers/a")).is_ok());
        assert!(fs.stat(config).is_ok());
    }
}
