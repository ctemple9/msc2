//! Same-machine desktop bootstrap over a Unix-domain socket.
//!
//! The socket is intentionally separate from HTTP. macOS supplies the peer
//! PID and UID, Security.framework validates that PID against the designated
//! requirement recorded by the installer, and the installation secret proves
//! that the caller is the installed desktop shell.

use std::path::PathBuf;

use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use super::{AuthState, DesktopPairingError};

const PROTOCOL_VERSION: u32 = 1;
const PROOF_DOMAIN: &[u8] = b"msc2-local-bootstrap-v1\0";

#[derive(Debug, Deserialize)]
struct ClientHello {
    version: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientProof {
    version: u32,
    host_id: String,
    proof: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChallengeResponse {
    status: &'static str,
    version: u32,
    host_id: String,
    challenge: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SuccessResponse {
    status: &'static str,
    version: u32,
    agent_host_id: String,
    token: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    status: &'static str,
    code: &'static str,
}

/// Starts the listener only when the installer supplied a complete macOS
/// bootstrap configuration. A missing configuration leaves the HTTP agent
/// available for ordinary remote pairing instead of preventing service start.
pub(crate) fn spawn(auth: AuthState) {
    let Some(socket_path) = std::env::var_os("MSC2_LOCAL_BOOTSTRAP_SOCKET") else {
        return;
    };
    let socket_path = PathBuf::from(socket_path);
    tokio::spawn(async move {
        if let Err(error) = serve(auth, socket_path).await {
            eprintln!("msc: local desktop bootstrap unavailable: {error}");
        }
    });
}

async fn serve(auth: AuthState, socket_path: PathBuf) -> Result<(), String> {
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)
            .map_err(|error| format!("removing stale bootstrap socket: {error}"))?;
    }
    let listener = UnixListener::bind(&socket_path).map_err(|error| {
        format!(
            "binding bootstrap socket {}: {error}",
            socket_path.display()
        )
    })?;
    set_owner_only_socket(&socket_path)?;
    println!(
        "msc: local desktop bootstrap listening on {}",
        socket_path.display()
    );

    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|error| format!("accepting bootstrap connection: {error}"))?;
        let auth = auth.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_connection(auth, stream).await {
                eprintln!("msc: local desktop bootstrap refused: {error}");
            }
        });
    }
}

async fn handle_connection(auth: AuthState, stream: UnixStream) -> Result<(), String> {
    let peer = stream
        .peer_cred()
        .map_err(|error| format!("reading bootstrap peer credentials: {error}"))?;
    let expected_uid = unsafe { libc::geteuid() };
    if peer.uid() != expected_uid {
        return Err("bootstrap peer is not the service user".to_string());
    }
    let pid = peer
        .pid()
        .ok_or_else(|| "bootstrap peer PID is unavailable".to_string())?;
    let requirement = std::env::var("MSC2_MACOS_DESKTOP_REQUIREMENT")
        .map_err(|_| "desktop code requirement is not installed".to_string())?;
    msc_platform_macos::service::verify_process_code_identity(pid as u32, &requirement)?;

    let secrets_dir = std::env::var_os("MSC2_MACOS_SECRET_STORE_DIR")
        .ok_or_else(|| "MSC2_MACOS_SECRET_STORE_DIR is not set".to_string())?;
    let key_path =
        msc_platform_macos::service::local_bootstrap_key_path(std::path::Path::new(&secrets_dir));
    let key = std::fs::read_to_string(&key_path)
        .map_err(|error| format!("reading bootstrap installation key: {error}"))?
        .trim()
        .to_string();

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|error| format!("reading bootstrap hello: {error}"))?;
    let hello: ClientHello =
        serde_json::from_str(&line).map_err(|_| "bootstrap hello is invalid".to_string())?;
    if hello.version != PROTOCOL_VERSION {
        write_error(&mut writer, "unsupported_version").await?;
        return Err("bootstrap protocol version is unsupported".to_string());
    }

    let host_id = auth
        .agent_host_id()
        .map_err(|error| format_pairing_error(&error))?;
    let challenge = random_hex(32);
    write_json(
        &mut writer,
        &ChallengeResponse {
            status: "challenge",
            version: PROTOCOL_VERSION,
            host_id: host_id.clone(),
            challenge: challenge.clone(),
        },
    )
    .await?;

    line.clear();
    reader
        .read_line(&mut line)
        .await
        .map_err(|error| format!("reading bootstrap proof: {error}"))?;
    let proof: ClientProof =
        serde_json::from_str(&line).map_err(|_| "bootstrap proof is invalid".to_string())?;
    if proof.version != PROTOCOL_VERSION
        || proof.host_id != host_id
        || proof.proof != expected_proof(&key, &challenge, &host_id)
    {
        write_error(&mut writer, "proof_failed").await?;
        return Err("bootstrap proof did not match".to_string());
    }

    let credential = auth
        .issue_local_bootstrap_credential()
        .map_err(|error| format_pairing_error(&error))?;
    write_json(
        &mut writer,
        &SuccessResponse {
            status: "ok",
            version: PROTOCOL_VERSION,
            agent_host_id: credential.agent_host_id,
            token: credential.issued.token,
        },
    )
    .await
}

async fn write_error<W: AsyncWrite + Unpin>(
    writer: &mut W,
    code: &'static str,
) -> Result<(), String> {
    write_json(
        writer,
        &ErrorResponse {
            status: "error",
            code,
        },
    )
    .await
}

async fn write_json<W: AsyncWrite + Unpin, T: Serialize>(
    writer: &mut W,
    value: &T,
) -> Result<(), String> {
    let encoded = serde_json::to_string(value)
        .map_err(|error| format!("encoding bootstrap response: {error}"))?;
    writer
        .write_all(encoded.as_bytes())
        .await
        .map_err(|error| format!("writing bootstrap response: {error}"))?;
    writer
        .write_all(b"\n")
        .await
        .map_err(|error| format!("writing bootstrap response: {error}"))?;
    writer
        .flush()
        .await
        .map_err(|error| format!("writing bootstrap response: {error}"))
}

fn expected_proof(key: &str, challenge: &str, host_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(PROOF_DOMAIN);
    digest.update(key.as_bytes());
    digest.update(challenge.as_bytes());
    digest.update(host_id.as_bytes());
    hex_lower(&digest.finalize())
}

fn random_hex(length: usize) -> String {
    let mut bytes = vec![0; length];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex_lower(&bytes)
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn format_pairing_error(error: &DesktopPairingError) -> String {
    format!("local desktop credential could not be issued: {error:?}")
}

fn set_owner_only_socket(path: &PathBuf) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)
        .map_err(|error| format!("reading bootstrap socket permissions: {error}"))?
        .permissions();
    permissions.set_mode(0o600);
    std::fs::set_permissions(path, permissions)
        .map_err(|error| format!("restricting bootstrap socket permissions: {error}"))
}
