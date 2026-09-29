//! Installing-user CLI exchange over a local Windows named pipe.

use std::time::Duration;

use msc_platform_windows::local_cli::{create_server_pipe, peer_sid, service_sid};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::NamedPipeServer;

use super::AuthState;
use super::local_cli::{LocalCliExchangeError, LocalOsAccount};

const HELLO_LIMIT: usize = 128;

pub(crate) async fn start(auth: AuthState) -> Result<(), String> {
    let service_sid = service_sid()?;
    let first = create_server_pipe(&service_sid, true)?;
    tokio::spawn(async move {
        if let Err(error) = serve(auth, service_sid, first).await {
            eprintln!("msc: local CLI named pipe stopped: {error}");
        }
    });
    Ok(())
}

async fn serve(
    auth: AuthState,
    service_sid: String,
    mut pipe: NamedPipeServer,
) -> Result<(), String> {
    loop {
        pipe.connect()
            .await
            .map_err(|error| format!("accepting local CLI named pipe connection: {error}"))?;
        let connected_pipe = pipe;
        pipe = create_server_pipe(&service_sid, false)?;
        let auth = auth.clone();
        let service_sid = service_sid.clone();
        tokio::spawn(async move {
            if let Err(error) = exchange(auth, service_sid, connected_pipe).await {
                eprintln!("msc: local CLI exchange failed: {error}");
            }
        });
    }
}

async fn exchange(
    auth: AuthState,
    service_sid: String,
    mut pipe: NamedPipeServer,
) -> Result<(), String> {
    let hello = tokio::time::timeout(Duration::from_secs(2), read_hello(&mut pipe))
        .await
        .map_err(|_| "local CLI hello timed out".to_string())??;
    if !valid_hello(&hello) {
        return write_error(&mut pipe, "unsupported_version").await;
    }

    let peer_sid = match peer_sid(&pipe) {
        Ok(sid) => sid,
        Err(_) => return write_error(&mut pipe, "unauthorized").await,
    };
    let issued = auth.issue_for_local_cli_peer(
        LocalOsAccount::WindowsSid(peer_sid),
        LocalOsAccount::WindowsSid(service_sid),
    );
    match issued {
        Ok(issued) => {
            write_json(
                &mut pipe,
                &serde_json::json!({ "status": "ok", "token": issued.token }),
            )
            .await
        }
        Err(LocalCliExchangeError::Unauthorized) => write_error(&mut pipe, "unauthorized").await,
        Err(LocalCliExchangeError::Busy) => write_error(&mut pipe, "busy").await,
    }
}

async fn read_hello(pipe: &mut NamedPipeServer) -> Result<Vec<u8>, String> {
    let mut hello = Vec::with_capacity(32);
    loop {
        let mut byte = [0u8; 1];
        pipe.read_exact(&mut byte)
            .await
            .map_err(|error| format!("reading local CLI hello: {error}"))?;
        if byte[0] == b'\n' {
            return Ok(hello);
        }
        if hello.len() >= HELLO_LIMIT {
            return Err("local CLI hello exceeds the size limit".to_string());
        }
        hello.push(byte[0]);
    }
}

fn valid_hello(bytes: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|value| value.get("version")?.as_u64())
        == Some(1)
}

async fn write_error(pipe: &mut NamedPipeServer, code: &str) -> Result<(), String> {
    write_json(
        pipe,
        &serde_json::json!({ "status": "error", "code": code }),
    )
    .await
}

async fn write_json(pipe: &mut NamedPipeServer, value: &serde_json::Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec(value)
        .map_err(|error| format!("encoding local CLI response: {error}"))?;
    bytes.push(b'\n');
    tokio::time::timeout(Duration::from_secs(2), pipe.write_all(&bytes))
        .await
        .map_err(|_| "local CLI response timed out".to_string())?
        .map_err(|error| format!("writing local CLI response: {error}"))
}
