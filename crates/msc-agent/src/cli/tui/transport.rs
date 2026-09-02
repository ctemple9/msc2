//! Authenticated client transport shared by the one-shot CLI and the TUI.
//!
//! The CLI originally kept this HTTP code beside command dispatch. Keeping the
//! same request implementation here gives the TUI the exact bearer, error, and
//! response behavior without creating a second management API.

#![allow(dead_code)]

use std::time::Duration;

use axum::http::{Method, StatusCode, Uri};
use futures_util::{SinkExt, StreamExt};
use msc_api::dto::{OperationDto, OperationStateDto};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

#[cfg(not(test))]
use super::super::{CliError, CommonArgs, resolve_base_url, resolve_token};
#[cfg(test)]
use crate::test_cli::{CliError, CommonArgs, resolve_base_url, resolve_token};

const INITIAL_RECONNECT_DELAY: Duration = Duration::from_millis(100);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub(crate) struct SharedClient {
    base_url: String,
    token: String,
}

impl SharedClient {
    pub(crate) fn from_common(common: &CommonArgs) -> Result<Self, CliError> {
        Ok(Self {
            base_url: resolve_base_url(common),
            token: resolve_token(common)?,
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

    /// Uploads raw bytes (a staged world-import ZIP) rather than a JSON body.
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

    /// Downloads a raw response body rather than decoding it as JSON.
    pub(crate) async fn get_raw_bytes(&self, path: &str) -> Result<Vec<u8>, CliError> {
        let response = self.request_raw(Method::GET, path, None, None).await?;
        Ok(response.body)
    }

    pub(crate) async fn open_stream(
        &self,
        channel: StreamChannel,
    ) -> Result<AuthenticatedStream, CliError> {
        let mut stream = AuthenticatedStream {
            client: self.clone(),
            channel,
            socket: None,
            reconnect_attempt: 0,
            terminal_seen: false,
        };
        stream.connect_once().await?;
        Ok(stream)
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
            .map_err(|err| {
                CliError::internal(format!("failed to connect to {host}:{port}: {err}"))
            })?;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StreamChannel {
    Console,
    Notifications,
    Operation { id: String },
}

impl StreamChannel {
    pub(crate) fn path(&self) -> String {
        match self {
            Self::Console => "/v1/console/stream".to_string(),
            Self::Notifications => "/v1/notifications/stream".to_string(),
            Self::Operation { id } => format!("/v1/operations/{id}/stream"),
        }
    }

    fn is_operation(&self) -> bool {
        matches!(self, Self::Operation { .. })
    }
}

/// A one-way authenticated stream. A dropped console or notification socket
/// reconnects and receives the channel's bounded backfill again. Operation
/// streams additionally re-fetch their current snapshot before reconnecting;
/// the server closes them normally after a terminal snapshot.
pub(crate) struct AuthenticatedStream {
    client: SharedClient,
    channel: StreamChannel,
    socket: Option<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>,
    reconnect_attempt: u32,
    terminal_seen: bool,
}

impl AuthenticatedStream {
    pub(crate) async fn next_json<T: DeserializeOwned>(&mut self) -> Result<Option<T>, CliError> {
        loop {
            if self.socket.is_none() {
                self.reconnect().await?;
            }

            let message = self
                .socket
                .as_mut()
                .expect("stream socket exists after reconnect")
                .next()
                .await;
            match message {
                Some(Ok(Message::Text(text))) => {
                    if self.channel.is_operation()
                        && serde_json::from_str::<OperationDto>(text.as_ref())
                            .is_ok_and(|operation| is_terminal(operation.state))
                    {
                        self.terminal_seen = true;
                    }
                    let value = serde_json::from_str(text.as_ref()).map_err(|err| {
                        CliError::internal(format!("failed to decode WebSocket JSON: {err}"))
                    })?;
                    return Ok(Some(value));
                }
                Some(Ok(Message::Ping(payload))) => {
                    self.socket
                        .as_mut()
                        .expect("stream socket exists while handling ping")
                        .send(Message::Pong(payload))
                        .await
                        .map_err(|err| {
                            CliError::internal(format!("WebSocket write failed: {err}"))
                        })?;
                }
                Some(Ok(Message::Close(_))) | Some(Err(WebSocketError::ConnectionClosed)) => {
                    self.socket = None;
                    if self.terminal_seen {
                        return Ok(None);
                    }
                }
                Some(Ok(Message::Pong(_)))
                | Some(Ok(Message::Binary(_)))
                | Some(Ok(Message::Frame(_))) => {}
                Some(Err(error)) => {
                    self.socket = None;
                    if self.terminal_seen {
                        return Ok(None);
                    }
                    if !is_reconnectable(&error) {
                        return Err(CliError::internal(format!(
                            "authenticated WebSocket stream failed: {error}"
                        )));
                    }
                }
                None => {
                    self.socket = None;
                    if self.terminal_seen {
                        return Ok(None);
                    }
                }
            }
        }
    }

    async fn reconnect(&mut self) -> Result<(), CliError> {
        if self.reconnect_attempt > 0 {
            tokio::time::sleep(reconnect_delay(self.reconnect_attempt)).await;
        }
        if let StreamChannel::Operation { id } = &self.channel {
            let _: OperationDto = self
                .client
                .get_json(&format!("/v1/operations/{id}"))
                .await?;
        }
        self.connect_once().await
    }

    async fn connect_once(&mut self) -> Result<(), CliError> {
        let url = self.websocket_url(&self.channel.path())?;
        let uri: Uri = url
            .parse()
            .map_err(|err| CliError::usage(format!("invalid WebSocket request URI: {err}")))?;
        let authority = uri
            .authority()
            .ok_or_else(|| CliError::usage("WebSocket request URI is missing a host"))?;
        let host = authority.as_str().to_string();
        let request = tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(uri)
            .header("Host", host)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", generate_key())
            .header("Authorization", format!("Bearer {}", self.client.token))
            .body(())
            .map_err(|err| CliError::usage(format!("invalid WebSocket request: {err}")))?;
        match connect_async(request).await {
            Ok((socket, _response)) => {
                self.socket = Some(socket);
                self.reconnect_attempt = 0;
                Ok(())
            }
            Err(WebSocketError::Http(response)) => {
                let status = StatusCode::from_u16(response.status().as_u16()).map_err(|err| {
                    CliError::internal(format!("WebSocket response status was invalid: {err}"))
                })?;
                Err(CliError::api(
                    status,
                    response.body().as_deref().unwrap_or_default(),
                ))
            }
            Err(error) => {
                self.reconnect_attempt = self.reconnect_attempt.saturating_add(1);
                Err(CliError::internal(format!(
                    "failed to connect authenticated WebSocket: {error}"
                )))
            }
        }
    }

    fn websocket_url(&self, path: &str) -> Result<String, CliError> {
        let uri: Uri = format!("{}{}", self.client.base_url, path)
            .parse()
            .map_err(|err| CliError::usage(format!("invalid WebSocket URI: {err}")))?;
        if uri.scheme_str() == Some("https") {
            return Err(CliError::usage(
                "wss base URLs are not implemented for the Phase 4 CLI yet",
            ));
        }
        let authority = uri
            .authority()
            .ok_or_else(|| CliError::usage("WebSocket URI is missing a host"))?;
        let target = uri
            .path_and_query()
            .map(|value| value.as_str())
            .unwrap_or("/");
        Ok(format!("ws://{}{target}", authority.as_str()))
    }
}

fn is_terminal(state: OperationStateDto) -> bool {
    matches!(
        state,
        OperationStateDto::Succeeded | OperationStateDto::Failed | OperationStateDto::Cancelled
    )
}

fn is_reconnectable(error: &WebSocketError) -> bool {
    matches!(
        error,
        WebSocketError::ConnectionClosed | WebSocketError::AlreadyClosed | WebSocketError::Io(_)
    )
}

pub(crate) fn reconnect_delay(attempt: u32) -> Duration {
    let shift = attempt.saturating_sub(1).min(5);
    (INITIAL_RECONNECT_DELAY * 2u32.pow(shift)).min(MAX_RECONNECT_DELAY)
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
