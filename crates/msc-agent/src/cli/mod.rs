//! Phase 4 CLI commands. Every subcommand except `serve` talks to the
//! same HTTP API the iOS client uses.

pub mod service;

use std::collections::HashMap;
use std::fmt::Display;

use axum::http::{Method, StatusCode, Uri};
use clap::{Args, Subcommand};
use msc_api::dto::{
    ActiveServerRequestDto, CommandRequestDto, CommandResultDto, ErrorDto, RemoteApiStatus,
    ServerDto, ServerImportRequestDto, ServerImportResultDto, SettingsResponseDto,
    SettingsUpdateRequestDto, SettingsUpdateResultDto, SimpleResultDto,
};
use msc_infrastructure::console_buffer::ConsoleLine;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 48400;

#[derive(Debug, Clone, Args)]
pub struct CommonArgs {
    /// Full base URL for the agent, for example http://127.0.0.1:48400.
    #[arg(long, global = true, conflicts_with_all = ["host", "port"])]
    pub base_url: Option<String>,

    /// Hostname or IP for the target agent.
    #[arg(long, global = true, default_value = DEFAULT_HOST)]
    pub host: String,

    /// TCP port for the target agent.
    #[arg(long, global = true, default_value_t = DEFAULT_PORT)]
    pub port: u16,

    /// Bearer token for the target agent.
    #[arg(long, global = true)]
    pub token: Option<String>,

    /// Emit JSON instead of human-readable output.
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Start the agent's HTTP management API.
    Serve {
        /// Address to bind the management API to. Loopback by default
        /// (`msc2-engineering.md` §10: "the management API binds to
        /// loopback by default") — LAN/Tailscale binding is opt-in and
        /// not implemented by the Phase 4 slice.
        #[arg(long, default_value = "127.0.0.1:48400")]
        bind: std::net::SocketAddr,
    },
    /// Print a bearer token the CLI can already resolve.
    Token {
        #[command(subcommand)]
        command: TokenCommand,
    },
    /// Import, start, stop, or restart the selected Java server.
    Server {
        #[command(subcommand)]
        command: ServerCommand,
    },
    /// Send one command to the active Java server.
    #[command(name = "command")]
    Send(CommandArgs),
    /// Show the active server's current lifecycle state.
    Status,
    /// Install or inspect the background service registration.
    Service {
        #[command(subcommand)]
        command: service::ServiceCommand,
    },
    /// Read recent console lines from the active server.
    Console {
        #[command(subcommand)]
        command: ConsoleCommand,
    },
    /// Read or change the active server's settings.
    Settings {
        #[command(subcommand)]
        command: SettingsCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum TokenCommand {
    /// Print a token already supplied to the CLI.
    Print {
        /// Print the Phase 4 test bootstrap token from
        /// `MSC2_TEST_BOOTSTRAP_TOKEN`.
        #[arg(long)]
        test: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ServerCommand {
    /// Import an existing Paper directory, or an MSC 1 `.msctransfer` package.
    Import {
        path: String,
        #[arg(long)]
        name: Option<String>,
        /// `folder|zip|transfer|auto`. Defaults to `transfer` when `path`
        /// ends in `.msctransfer`, otherwise `folder`.
        #[arg(long)]
        kind: Option<String>,
        /// `merge` (default) or `replaceAll`, for a transfer import.
        #[arg(long = "transfer-mode")]
        transfer_mode: Option<String>,
        /// Where to back up the current server set before a `replaceAll`
        /// transfer import. Required when `--transfer-mode replaceAll` is
        /// given.
        #[arg(long = "backup-path")]
        backup_path: Option<String>,
        /// Override a transferred Java server's port: `<source-server-id>=<port>`.
        #[arg(long = "java-port-override")]
        java_port_overrides: Vec<String>,
        /// Override a transferred Bedrock server's port: `<source-server-id>=<port>`.
        #[arg(long = "bedrock-port-override")]
        bedrock_port_overrides: Vec<String>,
    },
    /// Start the selected server, or the current active server if omitted.
    Start { server: Option<String> },
    /// Stop the selected server, or the current active server if omitted.
    Stop { server: Option<String> },
    /// Restart the selected server, or the current active server if omitted.
    Restart { server: Option<String> },
}

#[derive(Debug, Clone, Args)]
pub struct CommandArgs {
    /// Select a server by id or display name before sending the command.
    #[arg(long)]
    server: Option<String>,

    /// The exact console command to send.
    text: String,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConsoleCommand {
    /// Show the most recent console lines.
    Tail {
        /// Select a server by id or display name before fetching the tail.
        #[arg(long)]
        server: Option<String>,

        /// Number of lines to fetch.
        #[arg(short = 'n', long, default_value_t = 200)]
        lines: usize,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum SettingsCommand {
    /// Show the active server's current settings.
    Get {
        /// Select a server by id or display name before reading settings.
        #[arg(long)]
        server: Option<String>,
    },
    /// Apply one or more `key=value` changes to the active server's settings.
    Set {
        /// Select a server by id or display name before applying changes.
        #[arg(long)]
        server: Option<String>,

        /// One or more `key=value` pairs, for example `max-players=42`.
        #[arg(required = true)]
        changes: Vec<String>,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RestartResult {
    result: String,
    active_server_id: Option<String>,
    operation_id: Option<String>,
}

#[derive(Debug, Clone)]
struct RemoteClient {
    base_url: String,
    token: String,
}

#[derive(Debug)]
pub struct CliError {
    exit_code: u8,
    message: String,
    json_message: Option<String>,
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            exit_code: 2,
            message: message.into(),
            json_message: None,
        }
    }

    fn api(status: StatusCode, body: &str) -> Self {
        let message = match serde_json::from_str::<ErrorDto>(body) {
            Ok(error) => format!("API {} {}: {}", status.as_u16(), error.code, error.message),
            Err(_) => format!("API {}: {}", status.as_u16(), body.trim()),
        };
        Self {
            exit_code: 3,
            message,
            json_message: Some(body.to_string()),
        }
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self {
            exit_code: 1,
            message: message.into(),
            json_message: None,
        }
    }

    pub fn print(&self) {
        if let Some(json) = &self.json_message {
            eprintln!("{json}");
        } else {
            eprintln!("{}", self.message);
        }
    }

    pub fn exit_code(&self) -> u8 {
        self.exit_code
    }
}

pub async fn run(common: CommonArgs, command: Command) -> Result<(), CliError> {
    match command {
        Command::Serve { .. } => Err(CliError::internal("serve is handled in main")),
        Command::Token { command } => run_token(common, command),
        Command::Status => {
            let client = RemoteClient::from_common(&common)?;
            let status: RemoteApiStatus = client.get_json("/v1/status").await?;
            if common.json {
                print_json(&status)?;
            } else {
                print_status(&status);
            }
            Ok(())
        }
        Command::Service { command } => service::run(common, command).await,
        Command::Server { command } => run_server(common, command).await,
        Command::Send(args) => run_command(common, args).await,
        Command::Console { command } => run_console(common, command).await,
        Command::Settings { command } => run_settings(common, command).await,
    }
}

fn run_token(common: CommonArgs, command: TokenCommand) -> Result<(), CliError> {
    match command {
        TokenCommand::Print { test } => {
            let token = if test {
                std::env::var("MSC2_TEST_BOOTSTRAP_TOKEN").map_err(|_| {
                    CliError::usage(
                        "MSC2_TEST_BOOTSTRAP_TOKEN is not set, so there is no test token to print.",
                    )
                })?
            } else {
                resolve_token(&common)?
            };
            if common.json {
                print_json(&serde_json::json!({ "token": token }))?;
            } else {
                println!("{token}");
            }
            Ok(())
        }
    }
}

async fn run_server(common: CommonArgs, command: ServerCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        ServerCommand::Import {
            path,
            name,
            kind,
            transfer_mode,
            backup_path,
            java_port_overrides,
            bedrock_port_overrides,
        } => {
            let resolved_kind = kind.clone().unwrap_or_else(|| {
                if path.to_ascii_lowercase().ends_with(".msctransfer") {
                    "transfer".to_string()
                } else {
                    "folder".to_string()
                }
            });
            let is_transfer = resolved_kind == "transfer";
            let body = ServerImportRequestDto {
                action: Some(
                    if is_transfer {
                        "importTransfer"
                    } else {
                        "importExisting"
                    }
                    .to_string(),
                ),
                source_path: Some(path.clone()),
                import_kind: Some(resolved_kind),
                display_name: name.clone(),
                server_type: None,
                active_world_name: None,
                port: None,
                max_players: None,
                accept_eula: None,
                enable_playit: None,
                transfer_mode: transfer_mode.clone(),
                backup_path: backup_path.clone(),
                java_port_overrides: parse_port_overrides(&java_port_overrides)?,
                bedrock_port_overrides: parse_port_overrides(&bedrock_port_overrides)?,
            };
            let result: ServerImportResultDto =
                client.post_json("/v1/servers/import", &body).await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
                if let Some(server_name) = &result.server_name {
                    println!("server: {server_name}");
                }
                if let Some(server_id) = &result.server_id {
                    println!("server id: {server_id}");
                }
                if let Some(operation_id) = &result.operation_id {
                    println!("operation id: {operation_id}");
                }
            }
            Ok(())
        }
        ServerCommand::Start { server } => {
            if server.is_some() {
                ensure_active_server(&client, server.as_deref()).await?;
            }
            let result: SimpleResultDto = client
                .post_json("/v1/start", &serde_json::json!({}))
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                print_simple_result("Start requested.", &result);
            }
            Ok(())
        }
        ServerCommand::Stop { server } => {
            if server.is_some() {
                ensure_active_server(&client, server.as_deref()).await?;
            }
            let result: SimpleResultDto =
                client.post_json("/v1/stop", &serde_json::json!({})).await?;
            if common.json {
                print_json(&result)?;
            } else {
                print_simple_result("Stop requested.", &result);
            }
            Ok(())
        }
        ServerCommand::Restart { server } => {
            if server.is_some() {
                ensure_active_server(&client, server.as_deref()).await?;
            }
            let initial_status: RemoteApiStatus = client.get_json("/v1/status").await?;
            if initial_status.running {
                let _: SimpleResultDto =
                    client.post_json("/v1/stop", &serde_json::json!({})).await?;
                wait_for_stopped(&client).await?;
            }
            let result: SimpleResultDto = client
                .post_json("/v1/start", &serde_json::json!({}))
                .await?;
            let restart = RestartResult {
                result: "restart_requested".to_string(),
                active_server_id: result.active_server_id,
                operation_id: result.operation_id,
            };
            if common.json {
                print_json(&restart)?;
            } else {
                print_restart_result(&restart);
            }
            Ok(())
        }
    }
}

/// Parses `<source-server-id>=<port>` pairs, matching `settings set`'s
/// `key=value` parsing convention.
fn parse_port_overrides(pairs: &[String]) -> Result<HashMap<String, i64>, CliError> {
    let mut parsed = HashMap::new();
    for pair in pairs {
        let (id, port) = pair.split_once('=').ok_or_else(|| {
            CliError::usage(format!("invalid port override {pair:?}; expected id=port"))
        })?;
        let id = id.trim();
        if id.is_empty() {
            return Err(CliError::usage(format!(
                "invalid port override {pair:?}; id cannot be empty"
            )));
        }
        let port = port.trim().parse::<i64>().map_err(|_| {
            CliError::usage(format!(
                "invalid port override {pair:?}; port must be an integer"
            ))
        })?;
        parsed.insert(id.to_string(), port);
    }
    Ok(parsed)
}

async fn run_command(common: CommonArgs, args: CommandArgs) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    if args.server.is_some() {
        ensure_active_server(&client, args.server.as_deref()).await?;
    }
    let body = CommandRequestDto {
        command: Some(args.text.clone()),
    };
    let result: CommandResultDto = client.post_json("/v1/command", &body).await?;
    if common.json {
        print_json(&result)?;
    } else {
        println!("Sent command: {}", result.command);
        if let Some(active_server_id) = result.active_server_id {
            println!("active server id: {active_server_id}");
        }
    }
    Ok(())
}

async fn run_console(common: CommonArgs, command: ConsoleCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        ConsoleCommand::Tail { server, lines } => {
            if server.is_some() {
                ensure_active_server(&client, server.as_deref()).await?;
            }
            let tail: Vec<ConsoleLine> = client
                .get_json(&format!("/v1/console/tail?n={lines}"))
                .await?;
            if common.json {
                print_json(&tail)?;
            } else if tail.is_empty() {
                println!("No console lines yet.");
            } else {
                for line in &tail {
                    println!("[{}] {} {}", line.ts, line.source, line.text);
                }
            }
            Ok(())
        }
    }
}

async fn run_settings(common: CommonArgs, command: SettingsCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        SettingsCommand::Get { server } => {
            if server.is_some() {
                ensure_active_server(&client, server.as_deref()).await?;
            }
            let settings: SettingsResponseDto = client.get_json("/v1/settings").await?;
            if common.json {
                print_json(&settings)?;
            } else {
                print_settings(&settings);
            }
            Ok(())
        }
        SettingsCommand::Set { server, changes } => {
            if server.is_some() {
                ensure_active_server(&client, server.as_deref()).await?;
            }
            let mut parsed = HashMap::new();
            for change in &changes {
                let (key, value) = change.split_once('=').ok_or_else(|| {
                    CliError::usage(format!("invalid change {change:?}; expected key=value"))
                })?;
                let key = key.trim();
                if key.is_empty() {
                    return Err(CliError::usage(format!(
                        "invalid change {change:?}; key cannot be empty"
                    )));
                }
                parsed.insert(key.to_string(), value.trim().to_string());
            }
            let body = SettingsUpdateRequestDto { changes: parsed };
            let result: SettingsUpdateResultDto = client.post_json("/v1/settings", &body).await?;
            if common.json {
                print_json(&result)?;
            } else {
                print_settings_update(&result);
            }
            Ok(())
        }
    }
}

impl RemoteClient {
    fn from_common(common: &CommonArgs) -> Result<Self, CliError> {
        Ok(Self {
            base_url: resolve_base_url(common),
            token: resolve_token(common)?,
        })
    }

    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, CliError> {
        self.request_json::<(), T>(Method::GET, path, None).await
    }

    async fn post_json<Req: Serialize + ?Sized, Resp: DeserializeOwned>(
        &self,
        path: &str,
        body: &Req,
    ) -> Result<Resp, CliError> {
        self.request_json(Method::POST, path, Some(body)).await
    }

    async fn request_json<Req: Serialize + ?Sized, Resp: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&Req>,
    ) -> Result<Resp, CliError> {
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

        let body = if let Some(body) = body {
            Some(serde_json::to_string(body).map_err(|err| {
                CliError::internal(format!("failed to encode request body: {err}"))
            })?)
        } else {
            None
        };

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
            body,
        )
        .await
        .map_err(CliError::internal)?;
        let status = StatusCode::from_u16(response.status)
            .map_err(|err| CliError::internal(format!("response status was invalid: {err}")))?;
        let body = response.body;

        if !status.is_success() {
            return Err(CliError::api(status, &body));
        }

        serde_json::from_str(&body)
            .map_err(|err| CliError::internal(format!("failed to decode response JSON: {err}")))
    }
}

async fn ensure_active_server(
    client: &RemoteClient,
    selector: Option<&str>,
) -> Result<ServerDto, CliError> {
    let selector = selector
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CliError::usage("server selector cannot be empty"))?;
    let server = resolve_server(client, selector).await?;
    let body = ActiveServerRequestDto {
        server_id: Some(server.id.clone()),
    };
    let _: SimpleResultDto = client.post_json("/v1/active-server", &body).await?;
    Ok(server)
}

async fn resolve_server(client: &RemoteClient, selector: &str) -> Result<ServerDto, CliError> {
    let servers: Vec<ServerDto> = client.get_json("/v1/servers").await?;
    if servers.is_empty() {
        return Err(CliError::usage("the agent reports no imported servers"));
    }

    if let Some(server) = servers.iter().find(|server| server.id == selector) {
        return Ok(server.clone());
    }

    let exact_name_matches: Vec<&ServerDto> = servers
        .iter()
        .filter(|server| server.name == selector)
        .collect();
    if exact_name_matches.len() == 1 {
        return Ok(exact_name_matches[0].clone());
    }
    if exact_name_matches.len() > 1 {
        return Err(CliError::usage(format!(
            "multiple servers are named {selector:?}; use the server id instead"
        )));
    }

    let folded = selector.to_ascii_lowercase();
    let folded_matches: Vec<&ServerDto> = servers
        .iter()
        .filter(|server| server.name.to_ascii_lowercase() == folded)
        .collect();
    if folded_matches.len() == 1 {
        return Ok(folded_matches[0].clone());
    }
    if folded_matches.len() > 1 {
        return Err(CliError::usage(format!(
            "multiple servers match {selector:?}; use the server id instead"
        )));
    }

    Err(CliError::usage(format!(
        "no imported server matched {selector:?}"
    )))
}

async fn wait_for_stopped(client: &RemoteClient) -> Result<(), CliError> {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(30);
    loop {
        let status: RemoteApiStatus = client.get_json("/v1/status").await?;
        if !status.running {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(CliError::internal(
                "timed out waiting for the server to stop before restart",
            ));
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
    }
}

fn resolve_base_url(common: &CommonArgs) -> String {
    if let Some(base_url) = &common.base_url {
        base_url.trim_end_matches('/').to_string()
    } else {
        format!("http://{}:{}", common.host, common.port)
    }
}

fn resolve_token(common: &CommonArgs) -> Result<String, CliError> {
    common
        .token
        .clone()
        .or_else(|| std::env::var("MSC2_CLI_TOKEN").ok())
        .or_else(|| std::env::var("MSC2_TEST_BOOTSTRAP_TOKEN").ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            CliError::usage("no bearer token was provided; pass --token or set MSC2_CLI_TOKEN")
        })
}

fn print_json<T: Serialize>(value: &T) -> Result<(), CliError> {
    let text = serde_json::to_string(value)
        .map_err(|err| CliError::internal(format!("failed to encode JSON output: {err}")))?;
    println!("{text}");
    Ok(())
}

fn print_status(status: &RemoteApiStatus) {
    let state = if status.running { "RUNNING" } else { "STOPPED" };
    println!("status: {state}");
    if let Some(active_server_id) = &status.active_server_id {
        println!("active server id: {active_server_id}");
    }
    if let Some(pid) = status.pid {
        println!("pid: {pid}");
    }
    if let Some(server_type) = &status.server_type {
        println!("server type: {server_type}");
    }
}

fn print_simple_result(prefix: &str, result: &SimpleResultDto) {
    println!("{prefix}");
    if let Some(active_server_id) = &result.active_server_id {
        println!("active server id: {active_server_id}");
    }
    if let Some(operation_id) = &result.operation_id {
        println!("operation id: {operation_id}");
    }
}

fn print_settings(settings: &SettingsResponseDto) {
    println!("server: {}", settings.server_name);
    println!("editable: {}", settings.editable);
    if let Some(note) = &settings.note {
        println!("note: {note}");
    }
    for section in &settings.sections {
        println!("[{}]", section.title);
        for field in &section.fields {
            println!("  {} = {}", field.key, field.value);
        }
    }
}

fn print_settings_update(result: &SettingsUpdateResultDto) {
    println!("{}", result.message);
    if !result.applied_keys.is_empty() {
        println!("applied: {}", result.applied_keys.join(", "));
    }
    if let Some(rejected) = &result.rejected {
        for rejection in rejected {
            println!("rejected {}: {}", rejection.key, rejection.reason);
        }
    }
}

fn print_restart_result(result: &RestartResult) {
    println!("Restart requested.");
    if let Some(active_server_id) = &result.active_server_id {
        println!("active server id: {active_server_id}");
    }
    if let Some(operation_id) = &result.operation_id {
        println!("operation id: {operation_id}");
    }
}

impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

struct RawHttpResponse {
    status: u16,
    body: String,
}

async fn send_http_request(
    mut stream: tokio::net::TcpStream,
    method: &Method,
    authority: &str,
    target: &str,
    token: &str,
    body: Option<String>,
) -> Result<RawHttpResponse, String> {
    let mut request = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nConnection: close\r\n",
        method.as_str(),
        target,
        authority,
        token
    );
    if let Some(body) = &body {
        request.push_str("Content-Type: application/json\r\n");
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    request.push_str("\r\n");
    if let Some(body) = &body {
        request.push_str(body);
    }

    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|err| format!("failed to write request: {err}"))?;
    let mut response = Vec::new();
    let mut chunk = [0u8; 4096];
    let mut header_end = None;
    let mut expected_body_len = None;

    loop {
        let read =
            tokio::time::timeout(tokio::time::Duration::from_secs(5), stream.read(&mut chunk))
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
    let header_bytes = &response[..header_end];
    let body_bytes = &response[header_end + 4..];
    let headers = String::from_utf8(header_bytes.to_vec())
        .map_err(|err| format!("response headers were not valid UTF-8: {err}"))?;
    let body_bytes = if let Some(body_len) = parse_content_length(&headers)? {
        let wanted = body_len.min(body_bytes.len());
        &body_bytes[..wanted]
    } else {
        body_bytes
    };
    let body = String::from_utf8(body_bytes.to_vec())
        .map_err(|err| format!("response body was not valid UTF-8: {err}"))?;
    let mut lines = headers.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| "response was missing a status line".to_string())?;
    let status = status_line
        .split_whitespace()
        .nth(1)
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
