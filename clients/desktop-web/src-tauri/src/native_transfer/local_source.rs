//! Only the verified local host may receive a selected read-only descriptor.
use super::*;
use msc_infrastructure::{
    config_repository::default_app_data_dir,
    local_source::{send_file, Identity, SOCKET_NAME},
};
#[cfg(target_os = "linux")]
use msc_platform_linux::local_cli::{peer_uid, service_uid};
#[cfg(target_os = "macos")]
use msc_platform_macos::local_cli::{peer_uid, service_uid};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use tokio::{
    io::{AsyncWriteExt, Interest},
    net::UnixStream,
};

async fn line(stream: &mut UnixStream) -> Result<serde_json::Value, UploadError> {
    let mut bytes = Vec::new();
    loop {
        let byte = stream.read_u8().await.map_err(|_| failed())?;
        if byte == b'\n' {
            return serde_json::from_slice(&bytes).map_err(|_| failed());
        }
        if bytes.len() >= 16 * 1024 {
            return Err(failed());
        }
        bytes.push(byte);
    }
}
fn failed() -> UploadError {
    UploadError::new(
        "local_handoff",
        "The local agent could not confirm the selected file copy.",
    )
}
pub(super) async fn copy(
    context: &Context,
    source: Arc<Source>,
    operation: Arc<Operation>,
    slot: &Slot,
    attempt: &mut Attempt,
) -> Result<Option<Completion>, UploadError> {
    let root = default_app_data_dir();
    let path = root.join(SOCKET_NAME);
    // Refuse untrusted socket locations before sending the native credential.
    let private_root = std::fs::symlink_metadata(&root).is_ok_and(|m| {
        root.is_absolute() && m.is_dir() && m.uid() == service_uid() && m.mode() & 0o077 == 0
    });
    let private_socket = std::fs::symlink_metadata(&path).is_ok_and(|m| {
        m.file_type().is_socket() && m.uid() == service_uid() && m.mode() & 0o077 == 0
    });
    if !private_root || !private_socket {
        return Ok(None);
    }
    let Ok(Ok(mut socket)) =
        tokio::time::timeout(Duration::from_secs(10), UnixStream::connect(path)).await
    else {
        return Ok(None);
    };
    if peer_uid(&socket).map_err(|_| failed())? != service_uid() {
        return Err(failed());
    }
    let source_for_identity = source.clone();
    let identity =
        tauri::async_runtime::spawn_blocking(move || Identity::read(&source_for_identity.file))
            .await
            .map_err(|_| failed())?
            .map_err(|_| failed())?;
    let mut request = serde_json::to_vec(&serde_json::json!({
        "protocol": 1, "agentHostId": context.host, "token": context.record.token,
        "stagedUploadId": slot.staged_upload_id, "identity": identity,
    }))
    .map_err(|_| failed())?;
    request.push(b'\n');
    tokio::time::timeout(Duration::from_secs(10), socket.write_all(&request))
        .await
        .map_err(|_| failed())?
        .map_err(|_| failed())?;
    let ready = tokio::time::timeout(Duration::from_secs(10), line(&mut socket))
        .await
        .map_err(|_| failed())??;
    if ready.get("ready").and_then(|v| v.as_bool()) != Some(true)
        || ready.get("agentHostId").and_then(|v| v.as_str()) != Some(context.host.as_str())
        || ready.get("stagedUploadId").and_then(|v| v.as_str())
            != Some(slot.staged_upload_id.as_str())
    {
        return Err(failed());
    }
    // No fallback after passing the descriptor: the agent may already be copying.
    if manager().generation(&context.host) != context.generation {
        return Err(cancelled());
    }
    if *operation.cancel.borrow() {
        return Err(cancelled());
    }
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            socket.writable().await?;
            match socket.try_io(Interest::WRITABLE, || send_file(&socket, &source.file)) {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                result => break result,
            }
        }
    })
    .await
    .map_err(|_| failed())?
    .map_err(|_| failed())?;
    attempt.transport = Some(Transport::LocalCopy);
    operation.progress("local-copying", 0, Some(Transport::LocalCopy), true);
    loop {
        let event = tokio::time::timeout(Duration::from_secs(120), line(&mut socket))
            .await
            .map_err(|_| failed())??;
        if let Some(status) = event.get("status").and_then(|v| v.as_u64()) {
            if status != 200 {
                return Err(failed());
            }
            let completion: Completion =
                serde_json::from_value(event.get("body").cloned().ok_or_else(failed)?)
                    .map_err(|_| failed())?;
            operation.progress(
                "verifying",
                completion.received_bytes,
                Some(Transport::LocalCopy),
                true,
            );
            return Ok(Some(completion));
        }
        let count = event
            .get("receivedBytes")
            .and_then(|v| v.as_u64())
            .filter(|n| *n <= source.selected.size)
            .ok_or_else(failed)?;
        operation.progress("local-copying", count, Some(Transport::LocalCopy), false);
    }
}
