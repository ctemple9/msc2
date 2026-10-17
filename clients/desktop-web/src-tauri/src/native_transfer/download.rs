use super::*;
use tokio::io::AsyncWriteExt;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Request {
    agent_host_id: String,
    operation_id: String,
    staged_download_id: String,
    file_name: String,
    expected_bytes: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Saved {
    agent_host_id: String,
    operation_id: String,
    bytes_written: u64,
    sha256: String,
}
fn saved(result: UploadResult) -> Saved {
    Saved {
        agent_host_id: result.agent_host_id,
        operation_id: result.operation_id,
        bytes_written: result.received_bytes,
        sha256: result.sha256,
    }
}

fn reserve(request: &Request, progress: Channel<Progress>) -> Result<Arc<Operation>, UploadError> {
    let mut registry = manager().registry.lock().unwrap();
    registry
        .cancelled_downloads
        .retain(|_, time| time.elapsed() < SOURCE_TTL);
    let cancelled_before_start = registry
        .cancelled_downloads
        .remove(&(request.agent_host_id.clone(), request.operation_id.clone()))
        .is_some();
    if registry.operations.contains_key(&request.operation_id)
        || registry.sources.contains_key(&request.operation_id)
    {
        return Err(UploadError::new(
            "operation_replayed",
            "This transfer identity was already used.",
        ));
    }
    let active = registry
        .operations
        .values()
        .filter(|op| op.done.borrow().is_none())
        .collect::<Vec<_>>();
    if active.len() >= 2 || active.iter().any(|op| op.host == request.agent_host_id) {
        return Err(UploadError::new(
            "transfer_capacity",
            "A transfer is already active for this host, or both desktop slots are busy.",
        ));
    }
    let operation = Operation::new(
        &request.agent_host_id,
        &request.operation_id,
        request.expected_bytes,
        Arc::new(move |event| {
            let _ = progress.send(event);
        }),
    );
    if cancelled_before_start {
        operation.signal_cancel();
    }
    let mut terminal = registry
        .operations
        .iter()
        .filter(|(_, op)| op.done.borrow().is_some())
        .map(|(id, op)| (id.clone(), op.started))
        .collect::<Vec<_>>();
    terminal.sort_by_key(|(_, time)| *time);
    for (id, _) in terminal.iter().take(terminal.len().saturating_sub(63)) {
        registry.operations.remove(id);
    }
    registry
        .operations
        .insert(request.operation_id.clone(), operation.clone());
    Ok(operation)
}
#[tauri::command]
pub(crate) async fn cancel_native_download(
    window: tauri::WebviewWindow,
    agent_host_id: String,
    operation_id: String,
) -> Result<Saved, UploadError> {
    main_window(&window).map_err(|e| UploadError::new("wrong_window", e))?;
    if Uuid::parse_str(&operation_id).is_err() {
        return Err(UploadError::new(
            "invalid_operation",
            "Invalid download identity.",
        ));
    }
    let operation = {
        let mut registry = manager().registry.lock().unwrap();
        if let Some(op) = registry
            .operations
            .get(&operation_id)
            .filter(|op| op.host == agent_host_id)
        {
            Some(op.clone())
        } else {
            registry
                .cancelled_downloads
                .retain(|_, time| time.elapsed() < SOURCE_TTL);
            if registry.cancelled_downloads.len() >= 64 {
                return Err(UploadError::new(
                    "transfer_capacity",
                    "Too many pending cancellations.",
                ));
            }
            registry
                .cancelled_downloads
                .insert((agent_host_id, operation_id), Instant::now());
            None
        }
    };
    let Some(operation) = operation else {
        let mut error = UploadError::new("download_unconfirmed","Cancellation was recorded for delayed startup, but an earlier download completion cannot be confirmed.");
        error.cleanup = "unconfirmed";
        return Err(error);
    };
    operation.signal_cancel();
    operation.wait().await.map(saved)
}
#[tauri::command]
pub(crate) async fn download_staged_file(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    request: Request,
    progress: Channel<Progress>,
) -> Result<Saved, UploadError> {
    main_window(&window).map_err(|e| UploadError::new("wrong_window", e))?;
    if Uuid::parse_str(&request.operation_id).is_err()
        || Uuid::parse_str(&request.staged_download_id).is_err()
        || request.expected_bytes == 0
        || request.agent_host_id.is_empty()
        || request.agent_host_id.len() > 256
        || request.file_name.is_empty()
        || request.file_name.len() > 255
        || request.file_name.contains(['/', '\\', '\0'])
    {
        return Err(UploadError::new(
            "invalid_download",
            "Invalid native download metadata.",
        ));
    }
    let operation = reserve(&request, progress)?;
    let worker = operation.clone();
    tauri::async_runtime::spawn(async move {
        run(app, request, worker).await;
    });
    operation.wait().await.map(saved)
}
#[derive(Default)]
struct Attempt {
    partial: Option<PathBuf>,
    cleanup: Option<bool>,
}
async fn remove_export(context: &Context, id: &str) -> bool {
    let Ok(request) =
        context.original_request(Method::DELETE, &format!("/v1/staged-downloads/{id}"))
    else {
        return false;
    };
    matches!(request.timeout(Duration::from_secs(20)).send().await, Ok(response) if response.status() == reqwest::StatusCode::NO_CONTENT)
}
async fn run(app: tauri::AppHandle, request: Request, operation: Arc<Operation>) {
    let mut attempt = Attempt::default();
    let host = operation.host.clone();
    // Resolve the original credential even after an early cancellation: the
    // export already exists and still needs its owned cleanup request.
    let context = tauri::async_runtime::spawn_blocking(move || Context::load(host))
        .await
        .map_err(|_| network_error())
        .and_then(|result| result);
    let result = match &context {
        Ok(context) => {
            std::panic::AssertUnwindSafe(drive(app, &request, &operation, context, &mut attempt))
                .catch_unwind()
                .await
                .unwrap_or_else(|_| {
                    Err(UploadError::new(
                        "download_failed",
                        "The native download stopped unexpectedly.",
                    ))
                })
        }
        Err(error) => Err(error.clone()),
    };
    match result {
        Ok(result) => {
            operation.progress(
                "complete",
                result.received_bytes,
                Some(Transport::Stream),
                true,
            );
        }
        Err(mut error) => {
            let partial_removed = match attempt.partial {
                Some(path) => match tokio::fs::remove_file(path).await {
                    Ok(()) => true,
                    Err(e) => e.kind() == std::io::ErrorKind::NotFound,
                },
                None => true,
            };
            if attempt.cleanup.is_none() {
                attempt.cleanup = Some(match &context {
                    Ok(context) => remove_export(context, &request.staged_download_id).await,
                    Err(_) => false,
                });
            }
            let removed = partial_removed && attempt.cleanup == Some(true);
            error.cleanup = if removed { "confirmed" } else { "unconfirmed" };
            error.agent_host_id = Some(operation.host.clone().into_boxed_str());
            error.operation_id = Some(operation.id.clone().into_boxed_str());
            if !removed {
                error.message.push_str(
                    " Download cleanup could not be confirmed on the desktop or original agent.",
                );
            }
            let transferred = operation.progress.lock().unwrap().bytes_transferred;
            operation.progress(
                if error.code == "cancelled" && removed {
                    "cancelled"
                } else {
                    "failed"
                },
                transferred,
                Some(Transport::Stream),
                true,
            );
            operation.done.send_replace(Some(Err(error)));
        }
    }
}
async fn drive(
    app: tauri::AppHandle,
    request: &Request,
    operation: &Arc<Operation>,
    context: &Context,
    attempt: &mut Attempt,
) -> Outcome {
    operation.progress("preparing", 0, Some(Transport::Stream), true);
    let name = request.file_name.clone();
    let selected = cancellable(operation, async {
        tauri::async_runtime::spawn_blocking(move || {
            app.dialog().file().set_file_name(name).blocking_save_file()
        })
        .await
        .map_err(|_| UploadError::new("save_picker", "Could not choose a download destination."))?
        .ok_or_else(cancelled)?
        .into_path()
        .map_err(|_| UploadError::new("save_picker", "Choose a filesystem destination."))
    })
    .await?;
    let before = destination_identity(&selected)?;
    let path = selected.with_file_name(format!(".msc-transfer-{}.part", operation.id));
    let create_path = path.clone();
    let file = tauri::async_runtime::spawn_blocking(move || {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(create_path)
    })
    .await
    .map_err(|_| network_error())?
    .map_err(|_| {
        UploadError::new(
            "destination_write",
            "Could not create a private partial download.",
        )
    })?;
    attempt.partial = Some(path.clone());
    let mut file = tokio::fs::File::from_std(file);
    file.set_max_buf_size(BLOCK);
    let (count, digest, expected_digest) = cancellable(operation, async {
        let response = context
            .request(
                Method::GET,
                &format!("/v1/staged-downloads/{}", request.staged_download_id),
            )?
            .send()
            .await
            .map_err(|_| network_error())?;
        if !response.status().is_success() {
            let _: serde_json::Value = context.json(response).await?;
            return Err(network_error());
        }
        if response.content_length() != Some(request.expected_bytes) {
            return Err(UploadError::new(
                "integrity_mismatch",
                "The download length does not match the export.",
            ));
        }
        let expected = response
            .headers()
            .get("X-MSC-SHA256")
            .and_then(|h| h.to_str().ok())
            .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| {
                UploadError::new(
                    "incompatible_agent",
                    "This host cannot provide a verified native download. Update the agent.",
                )
            })?;
        let mut stream = response.bytes_stream();
        let mut hash = Sha256::new();
        let mut count = 0u64;
        while let Some(bytes) = tokio::time::timeout(Duration::from_secs(120), stream.next())
            .await
            .map_err(|_| network_error())?
        {
            let bytes = bytes.map_err(|_| network_error())?;
            count = count
                .checked_add(bytes.len() as u64)
                .filter(|n| *n <= request.expected_bytes)
                .ok_or_else(|| {
                    UploadError::new(
                        "integrity_mismatch",
                        "The download exceeded its expected length.",
                    )
                })?;
            for block in bytes.chunks(BLOCK) {
                file.write_all(block).await.map_err(|_| {
                    UploadError::new("destination_write", "Could not write the partial download.")
                })?;
                hash.update(block);
            }
            operation.progress("transferring", count, Some(Transport::Stream), false);
        }
        operation.progress("verifying", count, Some(Transport::Stream), true);
        let digest = format!("{:x}", hash.finalize());
        file.sync_all().await.map_err(|_| {
            UploadError::new(
                "destination_write",
                "Could not sync the downloaded archive.",
            )
        })?;
        Ok((count, digest, expected))
    })
    .await?;
    drop(file);
    attempt.cleanup = Some(remove_export(context, &request.staged_download_id).await);
    if attempt.cleanup != Some(true) {
        return Err(UploadError::new("download_cleanup","The original agent could not confirm export cleanup. The destination was not replaced."));
    }
    let _finish = operation.finish.lock().unwrap();
    if *operation.cancel.borrow() {
        return Err(cancelled());
    }
    let after = destination_identity(&selected)?;
    if before != after {
        return Err(UploadError::new(
            "destination_changed",
            "The selected destination changed during download.",
        ));
    }
    commit_verified(
        &path,
        &selected,
        request.expected_bytes,
        count,
        &digest,
        &expected_digest,
        *operation.cancel.borrow(),
    )?;
    attempt.partial = None;
    let result = UploadResult {
        operation_id: operation.id.clone(),
        agent_host_id: operation.host.clone(),
        staged_upload_id: request.staged_download_id.clone(),
        received_bytes: request.expected_bytes,
        sha256: digest,
        transport: Transport::Stream,
    };
    operation.done.send_replace(Some(Ok(result.clone())));
    Ok(result)
}

fn destination_identity(path: &Path) -> Result<Option<Identity>, UploadError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
            Identity::read(&open_source(path).map_err(|_| {
                UploadError::new(
                    "destination_changed",
                    "Could not inspect the selected destination.",
                )
            })?)
            .map(Some)
            .map_err(|_| {
                UploadError::new(
                    "destination_changed",
                    "Could not identify the selected destination.",
                )
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        _ => Err(UploadError::new(
            "destination_changed",
            "Choose a regular destination file.",
        )),
    }
}
fn commit_verified(
    partial: &Path,
    destination: &Path,
    expected_bytes: u64,
    received: u64,
    digest: &str,
    expected_digest: &str,
    cancelled_now: bool,
) -> Result<(), UploadError> {
    if cancelled_now {
        return Err(cancelled());
    }
    if received != expected_bytes || digest != expected_digest {
        return Err(UploadError::new(
            "integrity_mismatch",
            "The downloaded archive failed its byte count or digest check.",
        ));
    }
    std::fs::rename(partial, destination).map_err(|_| {
        UploadError::new(
            "destination_write",
            "Could not commit the verified download.",
        )
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    // Essential: failed or cancelled downloads must preserve an existing export.
    #[test]
    fn native_download_commits_only_verified_complete_uncancelled_bytes() {
        let root = std::env::temp_dir().join(format!("msc-download-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        struct Scratch(PathBuf);
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _scratch = Scratch(root.clone());
        let partial = root.join("partial");
        let destination = root.join("saved.msctransfer");
        std::fs::write(&destination, b"original").unwrap();
        std::fs::write(&partial, b"new").unwrap();
        for (count, digest, cancelled) in [
            (2, "valid", false),
            (3, "corrupt", false),
            (3, "valid", true),
        ] {
            assert!(
                commit_verified(&partial, &destination, 3, count, digest, "valid", cancelled)
                    .is_err()
            );
            assert_eq!(std::fs::read(&destination).unwrap(), b"original");
            assert!(partial.exists());
        }
        commit_verified(&partial, &destination, 3, 3, "valid", "valid", false).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"new");
        assert!(!partial.exists());
    }
}
