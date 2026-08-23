//! P10.13's offline CLI proof.  The real `msc` binary talks to a disposable
//! loopback HTTP server, so this checks CLI-to-contract behavior without
//! starting a server, reading credentials, or touching the public network.

use serde_json::{Value, json};
use std::process::Command;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Default)]
struct Requests {
    paths: Vec<String>,
    commands: Vec<String>,
}

async fn serve(listener: TcpListener, requests: Arc<Mutex<Requests>>) {
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let requests = requests.clone();
        tokio::spawn(async move {
            respond(stream, requests).await;
        });
    }
}

async fn respond(mut stream: TcpStream, requests: Arc<Mutex<Requests>>) {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let count = stream.read(&mut buffer).await.unwrap();
        if count == 0 {
            return;
        }
        request.extend_from_slice(&buffer[..count]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    let header_end = request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap()
        + 4;
    let (method, path, content_length) = {
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let first_line = headers.lines().next().unwrap();
        let mut parts = first_line.split_whitespace();
        let method = parts.next().unwrap().to_owned();
        let path = parts.next().unwrap().to_owned();
        let content_length = headers
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        (method, path, content_length)
    };
    while request.len() < header_end + content_length {
        let count = stream.read(&mut buffer).await.unwrap();
        if count == 0 {
            return;
        }
        request.extend_from_slice(&buffer[..count]);
    }
    let body = &request[header_end..header_end + content_length];
    {
        let mut recorded = requests.lock().unwrap();
        recorded.paths.push(path.clone());
        if path == "/v1/command" {
            recorded.commands.push(
                serde_json::from_slice::<Value>(body).unwrap()["command"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
    }

    let response = match (method.as_str(), path.as_str()) {
        ("GET", "/v1/status") => json!({
            "running": false,
            "activeServerId": "bedrock-linux",
            "pid": null,
            "serverType": "bedrock"
        }),
        ("POST", "/v1/start") => json!({
            "result": "started",
            "activeServerId": "bedrock-linux",
            "operationId": "synthetic-start"
        }),
        ("POST", "/v1/command") => json!({
            "result": "sent",
            "activeServerId": "bedrock-linux",
            "command": serde_json::from_slice::<Value>(body).unwrap()["command"]
        }),
        ("POST", "/v1/stop") => json!({
            "result": "stopped",
            "activeServerId": "bedrock-linux",
            "operationId": "synthetic-stop"
        }),
        ("GET", "/v1/capabilities") => json!({
            "agentVersion": "0.1.0",
            "apiMajor": 1,
            "apiMinor": 0,
            "hostOs": "linux",
            "permissions": [],
            "serverTypes": {
                "vanilla": false,
                "paper": false,
                "fabric": false,
                "forge": false,
                "neoforge": false,
                "bedrock": {"supported": false, "backend": null}
            },
            "helpers": {"playit": false, "duckdns": false, "geyser": false}
        }),
        _ => json!({"error": "unexpected synthetic request"}),
    };
    let bytes = serde_json::to_vec(&response).unwrap();
    let wire = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    );
    stream.write_all(wire.as_bytes()).await.unwrap();
    stream.write_all(&bytes).await.unwrap();
}

fn cli(url: &str, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_msc"))
        .args(["--base-url", url, "--token", "synthetic-token", "--json"])
        .args(args)
        .output()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn linux_cli_drives_lifecycle_and_reports_unavailability() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Requests::default()));
    let server = tokio::spawn(serve(listener, requests.clone()));
    let url = format!("http://{address}");

    let status = cli(&url, &["status"]);
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&status.stdout).unwrap()["running"],
        false
    );
    assert!(cli(&url, &["server", "start"]).status.success());
    let command = cli(&url, &["command", "say hello from cli"]);
    assert!(
        command.status.success(),
        "{}",
        String::from_utf8_lossy(&command.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&command.stdout).unwrap()["command"],
        "say hello from cli"
    );
    assert!(cli(&url, &["server", "stop"]).status.success());
    let capabilities = cli(&url, &["capabilities"]);
    assert!(
        capabilities.status.success(),
        "{}",
        String::from_utf8_lossy(&capabilities.stderr)
    );
    let capabilities = serde_json::from_slice::<Value>(&capabilities.stdout).unwrap();
    assert_eq!(capabilities["serverTypes"]["bedrock"]["supported"], false);
    assert!(capabilities["serverTypes"]["bedrock"]["backend"].is_null());

    let recorded = requests.lock().unwrap();
    assert_eq!(recorded.commands, vec!["say hello from cli"]);
    assert!(recorded.paths.contains(&"/v1/status".to_owned()));
    assert!(recorded.paths.contains(&"/v1/start".to_owned()));
    assert!(recorded.paths.contains(&"/v1/command".to_owned()));
    assert!(recorded.paths.contains(&"/v1/stop".to_owned()));
    assert!(recorded.paths.contains(&"/v1/capabilities".to_owned()));
    server.abort();
}
