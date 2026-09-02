use std::net::SocketAddr;
use std::time::Duration;

mod test_cli {
    #[derive(Debug, Clone)]
    pub struct CommonArgs {
        pub base_url: Option<String>,
        pub host: String,
        pub port: u16,
        pub token: Option<String>,
    }

    #[derive(Debug)]
    pub struct CliError {
        code: u8,
        message: String,
    }

    impl CliError {
        pub(crate) fn usage(message: impl Into<String>) -> Self {
            Self {
                code: 2,
                message: message.into(),
            }
        }

        pub(crate) fn internal(message: impl Into<String>) -> Self {
            Self {
                code: 1,
                message: message.into(),
            }
        }

        pub(crate) fn api(status: axum::http::StatusCode, body: &[u8]) -> Self {
            let parsed = serde_json::from_slice::<serde_json::Value>(body).ok();
            let code = parsed
                .as_ref()
                .and_then(|value| value["code"].as_str())
                .unwrap_or("unknown");
            let detail = parsed
                .as_ref()
                .and_then(|value| value["message"].as_str())
                .unwrap_or("request failed");
            Self {
                code: 3,
                message: format!("API {} {code}: {detail}", status.as_u16()),
            }
        }

        pub fn exit_code(&self) -> u8 {
            self.code
        }
    }

    impl std::fmt::Display for CliError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(&self.message)
        }
    }

    pub fn resolve_base_url(common: &CommonArgs) -> String {
        common
            .base_url
            .clone()
            .unwrap_or_else(|| format!("http://{}:{}", common.host, common.port))
    }

    pub fn resolve_token(common: &CommonArgs) -> Result<String, CliError> {
        common
            .token
            .clone()
            .filter(|token| !token.trim().is_empty())
            .ok_or_else(|| CliError::usage("no bearer token"))
    }
}

#[path = "../src/cli/tui/transport.rs"]
mod transport;

use futures_util::SinkExt;
use msc_api::dto::{OperationDto, OperationStateDto};
use test_cli::CommonArgs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::Message;
use transport::{SharedClient, StreamChannel, reconnect_delay};

fn common(base_url: String) -> CommonArgs {
    CommonArgs {
        base_url: Some(base_url),
        host: "127.0.0.1".to_string(),
        port: 48001,
        token: Some("transport-test-token".to_string()),
    }
}

#[tokio::test]
async fn shared_http_transport_preserves_bearer_and_api_errors() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_headers(&mut stream).await;
        assert!(request.contains("GET /v1/status HTTP/1.1"));
        assert!(request.contains("Authorization: Bearer transport-test-token"));
        let body = br#"{"code":"unauthorized","message":"token rejected"}"#;
        let response = format!(
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        stream.write_all(body).await.unwrap();
    });

    let client = SharedClient::from_common(&common(http_url(address))).unwrap();
    let error = tokio::time::timeout(
        Duration::from_secs(5),
        client.get_json::<serde_json::Value>("/v1/status"),
    )
    .await
    .expect("HTTP exchange completes")
    .expect_err("the mock API rejects the bearer");
    assert_eq!(error.exit_code(), 3);
    assert!(error.to_string().contains("API 401 unauthorized"));
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("HTTP mock server completes")
        .unwrap();
}

#[tokio::test]
async fn websocket_transport_authenticates_and_decodes_console_frames() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let callback =
            |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
             response: tokio_tungstenite::tungstenite::handshake::server::Response| {
                assert_eq!(request.uri().path(), "/v1/console/stream");
                assert_eq!(
                    request.headers().get("authorization").unwrap(),
                    "Bearer transport-test-token"
                );
                Ok(response)
            };
        let mut socket = accept_hdr_async(stream, callback).await.unwrap();
        socket
            .send(Message::Text(
                r#"{"ts":"1","source":"server","text":"ready"}"#.into(),
            ))
            .await
            .unwrap();
        socket.close(None).await.unwrap();
    });

    let client = SharedClient::from_common(&common(http_url(address))).unwrap();
    let mut stream = client.open_stream(StreamChannel::Console).await.unwrap();
    let line: serde_json::Value = tokio::time::timeout(Duration::from_secs(5), stream.next_json())
        .await
        .expect("WebSocket exchange completes")
        .unwrap()
        .unwrap();
    assert_eq!(line["text"], "ready");
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("WebSocket mock server completes")
        .unwrap();
}

#[tokio::test]
async fn operation_terminal_close_is_not_reconnected() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let operation = OperationDto {
        id: "op-1".to_string(),
        r#type: "backup".to_string(),
        target: None,
        state: OperationStateDto::Succeeded,
        progress: None,
        status_line: Some("done".to_string()),
        result: None,
        error: None,
    };
    let payload = serde_json::to_string(&operation).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        socket.send(Message::Text(payload.into())).await.unwrap();
        socket.close(None).await.unwrap();
    });

    let client = SharedClient::from_common(&common(http_url(address))).unwrap();
    let mut stream = client
        .open_stream(StreamChannel::Operation {
            id: "op-1".to_string(),
        })
        .await
        .unwrap();
    let received: OperationDto = tokio::time::timeout(Duration::from_secs(5), stream.next_json())
        .await
        .expect("operation snapshot arrives")
        .unwrap()
        .unwrap();
    assert_eq!(received.state, OperationStateDto::Succeeded);
    assert!(
        tokio::time::timeout(Duration::from_secs(5), stream.next_json::<OperationDto>())
            .await
            .expect("terminal close arrives")
            .unwrap()
            .is_none()
    );
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("operation mock server completes")
        .unwrap();
}

#[test]
fn reconnect_delay_is_exponential_and_bounded() {
    assert_eq!(reconnect_delay(1).as_millis(), 100);
    assert_eq!(reconnect_delay(2).as_millis(), 200);
    assert_eq!(reconnect_delay(5).as_millis(), 1600);
    assert_eq!(reconnect_delay(6).as_millis(), 2000);
    assert_eq!(reconnect_delay(50).as_millis(), 2000);
}

fn http_url(address: SocketAddr) -> String {
    format!("http://{address}")
}

async fn read_headers(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        let count = stream.read(&mut chunk).await.unwrap();
        assert!(count > 0, "request closed before headers completed");
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8(bytes).unwrap();
        }
    }
}
