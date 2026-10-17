//! A private, installing-user socket redeems an already admitted upload using
//! an open descriptor. This endpoint is never mounted on the HTTP router.
use crate::{
    auth::AuthState,
    routes::components::{ComponentsRoutesState, native_upload},
};
use axum::{
    Extension,
    body::Body,
    extract::{Path, State},
};
use msc_infrastructure::{
    config_repository::default_app_data_dir,
    local_source::{Identity, SOCKET_NAME, receive_file},
};
#[cfg(target_os = "linux")]
use msc_platform_linux::local_cli::{peer_uid, service_uid};
#[cfg(target_os = "macos")]
use msc_platform_macos::local_cli::{peer_uid, service_uid};
use std::{
    os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt},
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, Interest},
    net::{UnixListener, UnixStream},
};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    protocol: u32,
    agent_host_id: String,
    token: String,
    staged_upload_id: String,
    identity: Identity,
}

pub(crate) async fn start(auth: AuthState, state: ComponentsRoutesState) -> Result<(), String> {
    let root = default_app_data_dir();
    let metadata =
        std::fs::symlink_metadata(&root).map_err(|_| "Local data directory is unavailable.")?;
    if !root.is_absolute()
        || !metadata.is_dir()
        || metadata.uid() != service_uid()
        || metadata.mode() & 0o077 != 0
    {
        return Err("Local source socket requires the private service-user data directory.".into());
    }
    let path = root.join(SOCKET_NAME);
    match std::fs::symlink_metadata(&path) {
        Ok(meta) if meta.file_type().is_socket() && meta.uid() == service_uid() => {
            std::fs::remove_file(&path)
                .map_err(|_| "Could not remove stale local source socket.")?;
        }
        Ok(_) => return Err("Unexpected file at local source socket.".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("Could not inspect local source socket.".into()),
    }
    let listener = UnixListener::bind(path).map_err(|_| "Could not bind local source socket.")?;
    std::fs::set_permissions(
        root.join(SOCKET_NAME),
        std::fs::Permissions::from_mode(0o600),
    )
    .map_err(|_| "Could not restrict local source socket.")?;
    state
        .staging
        .native
        .local_handoff
        .store(true, Ordering::Release);
    tokio::spawn(async move {
        let permits = Arc::new(tokio::sync::Semaphore::new(16));
        while let Ok((stream, _)) = listener.accept().await {
            let Ok(permit) = permits.clone().try_acquire_owned() else {
                continue;
            };
            let state = state.clone();
            let auth = auth.clone();
            tokio::spawn(async move {
                let _permit = permit;
                // Errors intentionally contain no request/token/path data.
                let _ = exchange(auth, state, stream).await;
            });
        }
        state
            .staging
            .native
            .local_handoff
            .store(false, Ordering::Release);
    });
    Ok(())
}

async fn line(stream: &mut UnixStream) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    loop {
        let byte = stream.read_u8().await?;
        if byte == b'\n' {
            return Ok(bytes);
        }
        if bytes.len() >= 16 * 1024 {
            return Err(std::io::Error::other("metadata limit"));
        }
        bytes.push(byte);
    }
}
async fn write(stream: &mut UnixStream, value: serde_json::Value) -> Result<(), ()> {
    let mut bytes = serde_json::to_vec(&value).map_err(|_| ())?;
    bytes.push(b'\n');
    tokio::time::timeout(Duration::from_secs(10), stream.write_all(&bytes))
        .await
        .map_err(|_| ())?
        .map_err(|_| ())
}
pub(super) async fn exchange(
    auth: AuthState,
    state: ComponentsRoutesState,
    mut stream: UnixStream,
) -> Result<(), ()> {
    if peer_uid(&stream).map_err(|_| ())? != service_uid() {
        return Err(());
    }
    let request: Request = serde_json::from_slice(
        &tokio::time::timeout(Duration::from_secs(10), line(&mut stream))
            .await
            .map_err(|_| ())?
            .map_err(|_| ())?,
    )
    .map_err(|_| ())?;
    if request.protocol != 1 {
        return Err(());
    }
    let credential = auth
        .authenticate_local_source(&request.agent_host_id, &request.token)
        .ok_or(())?;
    let session = state
        .staging
        .native
        .session(&request.staged_upload_id, &credential)
        .ok_or(())?;
    if !session.can_handoff(&credential, &state)
        || session.local_progress().1 != request.identity.size
    {
        return Err(());
    }
    // Prove that this OS-authenticated agent holds this exact random admission
    // before the desktop passes its selected file capability.
    write(&mut stream, serde_json::json!({"ready": true, "agentHostId": request.agent_host_id, "stagedUploadId": request.staged_upload_id})).await?;
    let file = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            stream.readable().await?;
            match stream.try_io(Interest::READABLE, || receive_file(&stream)) {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                result => break result,
            }
        }
    })
    .await
    .map_err(|_| ())?
    .map_err(|_| ())?;
    request.identity.validate(&file).map_err(|_| ())?;
    let retained = Arc::new(file);
    let mut input = tokio::fs::File::from_std(retained.try_clone().map_err(|_| ())?);
    input.set_max_buf_size(256 * 1024);
    input
        .seek(std::io::SeekFrom::Start(0))
        .await
        .map_err(|_| ())?;
    let identity = request.identity;
    let total = identity.size;
    let source = futures_util::stream::try_unfold((input, 0u64), move |(mut input, offset)| {
        let retained = retained.clone();
        let identity = identity.clone();
        async move {
            if offset == total {
                tokio::task::spawn_blocking(move || identity.validate(&retained))
                    .await
                    .map_err(std::io::Error::other)??;
                return Ok::<_, std::io::Error>(None);
            }
            let mut block = vec![0u8; (total - offset).min(256 * 1024) as usize];
            input.read_exact(&mut block).await?;
            let next = offset + block.len() as u64;
            Ok(Some((block, (input, next))))
        }
    });
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap(),
    );
    headers.insert(
        axum::http::header::CONTENT_LENGTH,
        total.to_string().parse().map_err(|_| ())?,
    );
    let receive = native_upload::receive(
        State(state),
        Extension(credential),
        Extension(auth),
        Path(request.staged_upload_id),
        headers,
        Body::from_stream(source),
    );
    tokio::pin!(receive);
    let mut interval = tokio::time::interval(Duration::from_millis(250));
    let mut disconnect = [0u8; 1];
    let response = loop {
        tokio::select! {
            biased;
            _ = stream.read(&mut disconnect) => return Err(()),
            response = &mut receive => break response,
            _ = interval.tick() => {
                write(&mut stream, serde_json::json!({"receivedBytes": session.local_progress().0})).await?;
            }
        }
    };
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024)
        .await
        .map_err(|_| ())?;
    let body: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| ())?;
    write(
        &mut stream,
        serde_json::json!({"status": status, "body": body}),
    )
    .await
}
