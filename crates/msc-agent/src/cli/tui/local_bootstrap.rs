//! TUI client for the local agent bootstrap socket on macOS.
//!
//! The agent returns a short-lived bearer credential after the caller proves
//! possession of the installation key. The credential stays in this process;
//! it is never printed or written to disk.

use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::transport::SharedClient;

const PROTOCOL_VERSION: u32 = 1;
const PROOF_DOMAIN: &[u8] = b"msc2-local-bootstrap-v1\0";
const IO_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientHello {
    version: u32,
    client_kind: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChallengeResponse {
    status: String,
    version: u32,
    host_id: String,
    challenge: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SuccessResponse {
    status: String,
    version: u32,
    agent_host_id: String,
    token: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    code: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientProof {
    version: u32,
    host_id: String,
    proof: String,
}

pub(crate) fn connect_for_host(host: &str) -> Result<SharedClient, String> {
    if !is_loopback_host(host) {
        return Err("local bootstrap is only available for a loopback agent".to_string());
    }
    connect(normalize_base_url(host))
}

fn connect(base_url: String) -> Result<SharedClient, String> {
    let data_dir = data_dir()?;
    let socket_path = std::env::var_os("MSC2_LOCAL_BOOTSTRAP_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("bootstrap.sock"));
    let secrets_dir = std::env::var_os("MSC2_MACOS_SECRET_STORE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("secrets"));
    let key_path = msc_platform_macos::service::local_bootstrap_key_path(&secrets_dir);
    let key = std::fs::read_to_string(&key_path)
        .map_err(|error| {
            format!(
                "reading local bootstrap key {}: {error}",
                key_path.display()
            )
        })?
        .trim()
        .to_string();
    if key.is_empty() {
        return Err("local bootstrap installation key is empty".to_string());
    }

    let mut stream = UnixStream::connect(&socket_path).map_err(|error| {
        format!(
            "connecting to local agent {}: {error}",
            socket_path.display()
        )
    })?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|error| format!("setting bootstrap read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|error| format!("setting bootstrap write timeout: {error}"))?;

    write_json(
        &mut stream,
        &ClientHello {
            version: PROTOCOL_VERSION,
            client_kind: "cli",
        },
    )?;
    let mut reader = BufReader::new(
        stream
            .try_clone()
            .map_err(|error| format!("duplicating bootstrap connection: {error}"))?,
    );
    let challenge: ChallengeResponse = read_json(&mut reader)?;
    if challenge.status != "challenge" || challenge.version != PROTOCOL_VERSION {
        return Err("local agent returned an invalid bootstrap challenge".to_string());
    }
    write_json(
        &mut stream,
        &ClientProof {
            version: PROTOCOL_VERSION,
            host_id: challenge.host_id.clone(),
            proof: expected_proof(&key, &challenge.challenge, &challenge.host_id),
        },
    )?;
    let response: serde_json::Value = read_json(&mut reader)?;
    let status = response
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if status == "error" {
        let error: ErrorResponse = serde_json::from_value(response)
            .map_err(|error| format!("decoding bootstrap error: {error}"))?;
        return Err(format!("local agent bootstrap failed: {}", error.code));
    }
    let success: SuccessResponse = serde_json::from_value(response)
        .map_err(|error| format!("decoding bootstrap response: {error}"))?;
    if success.status != "ok"
        || success.version != PROTOCOL_VERSION
        || success.agent_host_id.trim().is_empty()
        || success.token.trim().is_empty()
    {
        return Err("local agent returned an invalid bootstrap credential".to_string());
    }
    let _ = stream.shutdown(Shutdown::Both);
    Ok(SharedClient::from_parts(base_url, success.token))
}

fn data_dir() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("MSC2_DATA_DIR") {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set; cannot locate the local agent".to_string())?;
    Ok(home.join("Library/Application Support/MSC 2"))
}

fn normalize_base_url(host: &str) -> String {
    if host.starts_with("http://") || host.starts_with("https://") {
        host.trim_end_matches('/').to_string()
    } else {
        format!("http://{}", host.trim_end_matches('/'))
    }
}

fn is_loopback_host(host: &str) -> bool {
    let authority = host
        .strip_prefix("http://")
        .or_else(|| host.strip_prefix("https://"))
        .unwrap_or(host)
        .split('/')
        .next()
        .unwrap_or(host);
    let hostname = authority
        .strip_prefix('[')
        .and_then(|value| value.split(']').next())
        .or_else(|| authority.rsplit_once(':').map(|(value, _)| value))
        .unwrap_or(authority);
    matches!(hostname, "127.0.0.1" | "localhost" | "::1")
}

fn write_json<T: Serialize>(stream: &mut UnixStream, value: &T) -> Result<(), String> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| format!("encoding bootstrap message: {error}"))?;
    stream
        .write_all(&encoded)
        .and_then(|_| stream.write_all(b"\n"))
        .and_then(|_| stream.flush())
        .map_err(|error| format!("writing bootstrap message: {error}"))
}

fn read_json<T: for<'de> Deserialize<'de>>(
    reader: &mut BufReader<UnixStream>,
) -> Result<T, String> {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|error| format!("reading bootstrap response: {error}"))?;
    serde_json::from_str(&line).map_err(|error| format!("decoding bootstrap response: {error}"))
}

fn expected_proof(key: &str, challenge: &str, host_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(PROOF_DOMAIN);
    digest.update(key.as_bytes());
    digest.update(challenge.as_bytes());
    digest.update(host_id.as_bytes());
    hex_lower(&digest.finalize())
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
