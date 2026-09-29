//! Installing-user CLI exchange over a private Unix socket.

use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use msc_infrastructure::config_repository::default_app_data_dir;
use msc_platform_macos::local_cli::{SOCKET_NAME, peer_uid, service_uid};
use tokio::io::AsyncWriteExt;
use tokio::net::{UnixListener, UnixStream};

use super::AuthState;
use super::local_cli::{LocalCliExchangeError, LocalOsAccount};

pub(crate) async fn start(auth: AuthState) -> Result<(), String> {
    let path = socket_path()?;
    let listener = UnixListener::bind(&path)
        .map_err(|error| format!("binding local CLI socket {}: {error}", path.display()))?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("restricting local CLI socket: {error}"))?;
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let auth = auth.clone();
                    tokio::spawn(async move {
                        if let Err(error) = exchange(auth, stream).await {
                            eprintln!("msc: local CLI exchange failed: {error}");
                        }
                    });
                }
                Err(error) => {
                    eprintln!("msc: local CLI listener stopped: {error}");
                    break;
                }
            }
        }
    });
    Ok(())
}

fn socket_path() -> Result<PathBuf, String> {
    let root = default_app_data_dir();
    if !root.is_absolute() {
        return Err("local CLI data directory must be absolute".to_string());
    }
    std::fs::create_dir_all(&root)
        .map_err(|error| format!("creating local CLI data directory: {error}"))?;
    let metadata = std::fs::symlink_metadata(&root)
        .map_err(|error| format!("inspecting local CLI data directory: {error}"))?;
    if !metadata.is_dir() || metadata.uid() != service_uid() {
        return Err("local CLI data directory must be owned by the service user".to_string());
    }
    if metadata.mode() & 0o077 != 0 {
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("restricting local CLI data directory: {error}"))?;
    }
    let path = root.join(SOCKET_NAME);
    remove_stale_socket(&path)?;
    Ok(path)
}

fn remove_stale_socket(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_socket() && metadata.uid() == service_uid() => {
            std::fs::remove_file(path)
                .map_err(|error| format!("removing stale local CLI socket: {error}"))
        }
        Ok(_) => Err("local CLI socket path contains an unexpected file".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("inspecting local CLI socket: {error}")),
    }
}

async fn exchange(auth: AuthState, mut stream: UnixStream) -> Result<(), String> {
    let response = match peer_uid(&stream) {
        Ok(uid) => match auth.issue_for_local_cli_peer(
            LocalOsAccount::UnixUid(uid),
            LocalOsAccount::UnixUid(service_uid()),
        ) {
            Ok(issued) => serde_json::json!({ "status": "ok", "token": issued.token }),
            Err(LocalCliExchangeError::Unauthorized) => {
                serde_json::json!({ "status": "error", "code": "unauthorized" })
            }
            Err(LocalCliExchangeError::Busy) => {
                serde_json::json!({ "status": "error", "code": "busy" })
            }
        },
        Err(_) => serde_json::json!({ "status": "error", "code": "unauthorized" }),
    };
    let mut bytes = serde_json::to_vec(&response)
        .map_err(|error| format!("encoding local CLI response: {error}"))?;
    bytes.push(b'\n');
    tokio::time::timeout(std::time::Duration::from_secs(2), stream.write_all(&bytes))
        .await
        .map_err(|_| "local CLI response timed out".to_string())?
        .map_err(|error| format!("writing local CLI response: {error}"))
}
