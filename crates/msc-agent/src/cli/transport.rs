//! Authenticated HTTP transport for the one-shot CLI.
//!
//! Named CLI commands use the same management API as the desktop
//! clients. This module owns only request construction and response decoding;
//! WebSocket streams remain agent routes for the desktop client, not part of
//! this one-shot command transport.

use std::io;
#[cfg(any(target_os = "linux", target_os = "macos", test))]
use std::path::PathBuf;
use std::time::Duration;

use axum::http::{Method, StatusCode, Uri};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
#[cfg(windows)]
use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::cli::CliError;

const LOCAL_API_BASE_URL: &str = "http://127.0.0.1:48001";
const LOCAL_RESPONSE_LIMIT: usize = 1024;
const LOCAL_EXCHANGE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Deserialize)]
struct LocalCliExchangeResponse {
    status: String,
    token: Option<String>,
    code: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SharedClient {
    base_url: String,
    token: String,
}

impl SharedClient {
    pub(crate) async fn connect_local() -> Result<Self, CliError> {
        Ok(Self {
            base_url: LOCAL_API_BASE_URL.to_string(),
            token: acquire_local_cli_token().await?,
        })
    }

    pub(crate) async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, CliError> {
        let response = self.request_raw(Method::GET, path, None, None).await?;
        decode_json(&response.body)
    }

    pub(crate) async fn post_json<Req: Serialize + ?Sized, Resp: DeserializeOwned>(
        &self,
        path: &str,
        body: &Req,
    ) -> Result<Resp, CliError> {
        let payload = serde_json::to_vec(body)
            .map_err(|err| CliError::internal(format!("failed to encode request body: {err}")))?;
        let response = self
            .request_raw(Method::POST, path, Some("application/json"), Some(payload))
            .await?;
        decode_json(&response.body)
    }

    /// Uploads raw bytes rather than a JSON body.
    pub(crate) async fn put_bytes<Resp: DeserializeOwned>(
        &self,
        path: &str,
        content_type: &str,
        body: Vec<u8>,
    ) -> Result<Resp, CliError> {
        let response = self
            .request_raw(Method::PUT, path, Some(content_type), Some(body))
            .await?;
        decode_json(&response.body)
    }

    pub(crate) async fn put_chunk(
        &self,
        path: &str,
        body: Vec<u8>,
    ) -> Result<Option<msc_api::dto::StagedUploadCompleteResultDto>, CliError> {
        let response = self
            .request_raw(
                Method::PUT,
                path,
                Some("application/octet-stream"),
                Some(body),
            )
            .await?;
        if response.body.is_empty() {
            Ok(None)
        } else {
            decode_json(&response.body).map(Some)
        }
    }
    pub(crate) async fn cancel_upload(&self, id: &str) -> Result<(), CliError> {
        self.request_raw(
            Method::DELETE,
            &format!("/v1/staged-uploads/{id}"),
            None,
            None,
        )
        .await?;
        Ok(())
    }
    /// Downloads a raw response body rather than decoding it as JSON.
    pub(crate) async fn get_raw_bytes(&self, path: &str) -> Result<Vec<u8>, CliError> {
        let response = self.request_raw(Method::GET, path, None, None).await?;
        Ok(response.body)
    }

    pub(crate) async fn connect_console_stream(
        &self,
    ) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, CliError> {
        let ticket: serde_json::Value = self
            .post_json("/v1/console/stream-ticket", &serde_json::json!({}))
            .await?;
        let ticket = ticket["ticket"]
            .as_str()
            .filter(|ticket| !ticket.is_empty())
            .ok_or_else(|| CliError::internal("agent returned no console stream ticket"))?;
        let url = format!(
            "ws://127.0.0.1:48001/v1/console/stream?ticket={}",
            encode_query_component(ticket)
        );
        let (socket, _) = connect_async(url).await.map_err(|error| {
            CliError::internal(format!("could not open local console stream: {error}"))
        })?;
        Ok(socket)
    }

    async fn request_raw(
        &self,
        method: Method,
        path: &str,
        content_type: Option<&str>,
        body: Option<Vec<u8>>,
    ) -> Result<RawHttpResponse, CliError> {
        let uri: Uri = format!("{}{}", self.base_url, path)
            .parse()
            .map_err(|err| CliError::usage(format!("invalid request URI: {err}")))?;
        if uri.scheme_str() == Some("https") {
            return Err(CliError::usage(
                "https base URLs are not implemented for the Phase 4 CLI yet",
            ));
        }
        let authority = uri
            .authority()
            .ok_or_else(|| CliError::usage("request URI is missing a host"))?;
        let host = authority.host().to_string();
        let port = authority.port_u16().unwrap_or(80);
        let target = uri
            .path_and_query()
            .map(|value| value.as_str().to_string())
            .unwrap_or_else(|| "/".to_string());

        let stream = tokio::net::TcpStream::connect((host.as_str(), port))
            .await
            .map_err(|err| CliError::internal(format!(
                "the local agent API at {host}:{port} is unavailable; confirm the agent is installed and running: {err}"
            )))?;
        let response = send_http_request(
            stream,
            &method,
            authority.as_str(),
            &target,
            &self.token,
            content_type,
            body,
        )
        .await
        .map_err(CliError::internal)?;
        let status = StatusCode::from_u16(response.status)
            .map_err(|err| CliError::internal(format!("response status was invalid: {err}")))?;

        if !status.is_success() {
            return Err(CliError::api(status, &response.body));
        }

        Ok(response)
    }
}

fn encode_query_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
async fn acquire_local_cli_token() -> Result<String, CliError> {
    let socket_path = local_cli_data_dir()?.join(local_cli_socket_name());
    let endpoint = socket_path.display().to_string();
    let stream = tokio::net::UnixStream::connect(&socket_path)
        .await
        .map_err(|error| local_endpoint_error(error, &endpoint))?;
    let mut reader = BufReader::new(stream);
    read_local_cli_response(&mut reader).await
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn local_cli_data_dir() -> Result<PathBuf, CliError> {
    let process_override = std::env::var_os("MSC2_DATA_DIR")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let service_override = if process_override.is_none() {
        crate::cli::service::installed_agent_data_dir()?
    } else {
        None
    };
    Ok(resolve_local_cli_data_dir(
        process_override,
        service_override,
        msc_infrastructure::config_repository::default_app_data_dir(),
    ))
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
fn resolve_local_cli_data_dir(
    process_override: Option<PathBuf>,
    service_override: Option<PathBuf>,
    default_dir: PathBuf,
) -> PathBuf {
    process_override.or(service_override).unwrap_or(default_dir)
}

#[cfg(target_os = "linux")]
fn local_cli_socket_name() -> &'static str {
    msc_platform_linux::local_cli::SOCKET_NAME
}

#[cfg(target_os = "macos")]
fn local_cli_socket_name() -> &'static str {
    msc_platform_macos::local_cli::SOCKET_NAME
}

#[cfg(windows)]
async fn acquire_local_cli_token() -> Result<String, CliError> {
    use msc_platform_windows::local_cli::PIPE_NAME;
    use tokio::net::windows::named_pipe::ClientOptions;

    let mut stream = ClientOptions::new()
        .open(PIPE_NAME)
        .map_err(|error| local_endpoint_error(error, PIPE_NAME))?;
    stream
        .write_all(b"{\"version\":1}\n")
        .await
        .map_err(|error| local_exchange_io_error(error, PIPE_NAME))?;
    stream
        .flush()
        .await
        .map_err(|error| local_exchange_io_error(error, PIPE_NAME))?;
    let mut reader = BufReader::new(stream);
    read_local_cli_response(&mut reader).await
}

async fn read_local_cli_response<R: AsyncRead + Unpin>(reader: &mut R) -> Result<String, CliError> {
    let line = tokio::time::timeout(LOCAL_EXCHANGE_TIMEOUT, read_bounded_line(reader))
        .await
        .map_err(|_| CliError::internal("local agent authorization exchange timed out"))?
        .map_err(|error| {
            CliError::internal(format!(
                "local agent authorization exchange failed: {error}"
            ))
        })?;
    let response: LocalCliExchangeResponse = serde_json::from_slice(&line).map_err(|error| {
        CliError::internal(format!(
            "local agent returned an invalid authorization response: {error}"
        ))
    })?;
    match response.status.as_str() {
        "ok" => response
            .token
            .filter(|token| !token.trim().is_empty())
            .ok_or_else(|| CliError::internal("local agent returned no CLI credential")),
        "error" => match response.code.as_deref() {
            Some("unauthorized") => Err(CliError::internal(
                "this OS account is not authorized to use the local MSC agent; run the CLI as the account that installed it",
            )),
            Some("busy") => Err(CliError::internal(
                "the local agent has reached its temporary CLI credential limit; retry shortly",
            )),
            Some("unsupported_version") => Err(CliError::internal(
                "the local agent and CLI use different local authorization protocols; update both",
            )),
            _ => Err(CliError::internal("local agent refused CLI authorization")),
        },
        _ => Err(CliError::internal(
            "local agent returned an unknown authorization status",
        )),
    }
}

async fn read_bounded_line<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Vec<u8>, String> {
    let mut line = Vec::with_capacity(128);
    loop {
        let mut byte = [0u8; 1];
        reader
            .read_exact(&mut byte)
            .await
            .map_err(|error| format!("reading local endpoint response: {error}"))?;
        if byte[0] == b'\n' {
            return Ok(line);
        }
        if line.len() >= LOCAL_RESPONSE_LIMIT {
            return Err("local endpoint response exceeded the size limit".to_string());
        }
        line.push(byte[0]);
    }
}

fn local_endpoint_error(error: io::Error, endpoint: &str) -> CliError {
    match error.kind() {
        io::ErrorKind::PermissionDenied => CliError::internal(
            "this OS account cannot access the local MSC agent; run the CLI as the account that installed it",
        ),
        io::ErrorKind::NotFound
        | io::ErrorKind::ConnectionRefused
        | io::ErrorKind::ConnectionReset
        | io::ErrorKind::BrokenPipe
        | io::ErrorKind::TimedOut => CliError::internal(format!(
            "the local agent is stopped or its authorization endpoint is unavailable at {endpoint}; confirm the agent is installed and running: {error}"
        )),
        _ if error.raw_os_error() == Some(231) => {
            CliError::internal("the local agent authorization pipe is busy; retry the command")
        }
        _ => CliError::internal(format!(
            "could not connect to the local agent authorization endpoint at {endpoint}: {error}"
        )),
    }
}

#[cfg(windows)]
fn local_exchange_io_error(error: io::Error, endpoint: &str) -> CliError {
    CliError::internal(format!(
        "the local agent authorization exchange ended at {endpoint}; the agent may be stopped or restarting: {error}"
    ))
}

#[cfg(test)]
mod tests {
    use super::resolve_local_cli_data_dir;
    use std::path::PathBuf;

    #[test]
    fn service_data_directory_is_used_when_cli_has_no_override() {
        let actual = resolve_local_cli_data_dir(
            None,
            Some(PathBuf::from("/service/data with spaces")),
            PathBuf::from("/cli/default"),
        );

        assert_eq!(actual, PathBuf::from("/service/data with spaces"));
    }

    #[test]
    fn explicit_cli_data_directory_still_takes_precedence() {
        let actual = resolve_local_cli_data_dir(
            Some(PathBuf::from("/explicit/override")),
            Some(PathBuf::from("/service/data")),
            PathBuf::from("/cli/default"),
        );

        assert_eq!(actual, PathBuf::from("/explicit/override"));
    }
}

struct RawHttpResponse {
    status: u16,
    body: Vec<u8>,
}

fn decode_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CliError> {
    serde_json::from_slice(bytes)
        .map_err(|err| CliError::internal(format!("failed to decode response JSON: {err}")))
}

async fn send_http_request(
    mut stream: tokio::net::TcpStream,
    method: &Method,
    authority: &str,
    target: &str,
    token: &str,
    content_type: Option<&str>,
    body: Option<Vec<u8>>,
) -> Result<RawHttpResponse, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut header = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nConnection: close\r\n",
        method.as_str(),
        target,
        authority,
        token
    );
    if let Some(body) = &body {
        if let Some(content_type) = content_type {
            header.push_str(&format!("Content-Type: {content_type}\r\n"));
        }
        header.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    header.push_str("\r\n");

    let mut request = header.into_bytes();
    if let Some(body) = body {
        request.extend_from_slice(&body);
    }

    stream
        .write_all(&request)
        .await
        .map_err(|err| format!("failed to write request: {err}"))?;
    let mut response = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut header_end = None;
    let mut expected_body_len = None;
    let response_timeout = std::env::var("MSC2_CLI_RESPONSE_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map(tokio::time::Duration::from_secs)
        .unwrap_or_else(|| tokio::time::Duration::from_secs(5));

    loop {
        let read = tokio::time::timeout(response_timeout, stream.read(&mut chunk))
            .await
            .map_err(|_| "timed out waiting for the agent response".to_string())?
            .map_err(|err| format!("failed to read response: {err}"))?;
        if read == 0 {
            break;
        }
        response.extend_from_slice(&chunk[..read]);

        if header_end.is_none() {
            header_end = response.windows(4).position(|window| window == b"\r\n\r\n");
            if let Some(end) = header_end {
                let headers = String::from_utf8(response[..end].to_vec())
                    .map_err(|err| format!("response headers were not valid UTF-8: {err}"))?;
                expected_body_len = parse_content_length(&headers)?;
                if expected_body_len == Some(0) {
                    break;
                }
            }
        }

        if let (Some(end), Some(body_len)) = (header_end, expected_body_len)
            && response.len() >= end + 4 + body_len
        {
            break;
        }
    }

    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "response did not contain a header/body separator".to_string())?;
    let body_bytes = &response[header_end + 4..];
    let headers = String::from_utf8(response[..header_end].to_vec())
        .map_err(|err| format!("response headers were not valid UTF-8: {err}"))?;
    let body = if let Some(body_len) = parse_content_length(&headers)? {
        body_bytes[..body_len.min(body_bytes.len())].to_vec()
    } else {
        body_bytes.to_vec()
    };
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| "response status line was malformed".to_string())?
        .parse::<u16>()
        .map_err(|err| format!("response status line was malformed: {err}"))?;

    Ok(RawHttpResponse { status, body })
}

fn parse_content_length(headers: &str) -> Result<Option<usize>, String> {
    let Some(line) = headers
        .lines()
        .find(|line| line.to_ascii_lowercase().starts_with("content-length:"))
    else {
        return Ok(None);
    };
    let value = line
        .split_once(':')
        .map(|(_, value)| value.trim())
        .ok_or_else(|| "content-length header was malformed".to_string())?;
    value
        .parse::<usize>()
        .map(Some)
        .map_err(|err| format!("content-length header was malformed: {err}"))
}
