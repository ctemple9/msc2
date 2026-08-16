//! Phase 4 CLI commands. Every subcommand except `serve` talks to the
//! same HTTP API the iOS client uses.

pub mod service;

use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::http::{Method, StatusCode, Uri};
use clap::{Args, Subcommand};
use msc_api::dto::{
    ActiveServerRequestDto, BackupConfigResponseDto, BackupConfigUpdateRequestDto,
    BackupConfigUpdateResultDto, BackupDeleteRequestDto, BackupNowResultDto,
    BackupRestoreRequestDto, BackupRestoreResultDto, BackupsResponseDto, CommandRequestDto,
    CommandResultDto, ErrorDto, OperationDto, OperationStateDto, RemoteApiStatus, ServerDto,
    ServerImportRequestDto, ServerImportResultDto, ServerImportScanResponseDto,
    SettingsResponseDto, SettingsUpdateRequestDto, SettingsUpdateResultDto, SimpleResultDto,
    StagedUploadBeginRequestDto, StagedUploadBeginResultDto, StagedUploadCompleteResultDto,
    StagedUploadPurposeDto, WorldActivateRequestDto, WorldActivateResultDto,
    WorldConvertRequestDto, WorldConvertResultDto, WorldCreateRequestDto, WorldDeleteRequestDto,
    WorldDuplicateRequestDto, WorldExportRequestDto, WorldExportResultDto, WorldImportRequestDto,
    WorldMutationResultDto, WorldRenameRequestDto, WorldReplaceRequestDto, WorldSlotDto,
    WorldSlotsResponseDto,
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
    /// Hidden root-run helper used by the Linux service unit.
    #[cfg(target_os = "linux")]
    #[command(name = "credential-helper", hide = true)]
    CredentialHelper {
        #[command(subcommand)]
        command: CredentialHelperCommand,
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
    /// List, mutate, or convert world slots on the active server.
    World {
        #[command(subcommand)]
        command: WorldCommand,
    },
    /// List, create, restore, delete, or configure backups on the active
    /// server.
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
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

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Subcommand)]
pub enum CredentialHelperCommand {
    /// Serve the Linux privileged credential helper protocol.
    Serve {
        /// The only unprivileged UID allowed to use the helper socket.
        #[arg(long)]
        allowed_uid: u32,

        /// Root-owned directory where encrypted credential blobs are stored.
        #[arg(long)]
        store_dir: PathBuf,

        /// Bind a socket directly instead of using systemd socket activation.
        #[arg(long)]
        socket_path: Option<PathBuf>,
    },
}

// `Import` is far larger than `Start`/`Stop`/`Restart` because it alone
// carries every raw-import override and transfer flag — clippy would have
// us box fields to shrink the enum, but this is a CLI arg enum matched
// once per invocation, not a hot path; the added indirection wouldn't pay
// for itself.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Subcommand)]
pub enum ServerCommand {
    /// Import an existing server directory/ZIP, or an MSC 1
    /// `.msctransfer` package. Pass `--scan` first to preview what a raw
    /// folder/ZIP contains before importing it.
    Import {
        path: String,
        #[arg(long)]
        name: Option<String>,
        /// `folder|zip|transfer|auto`. Defaults to `transfer` when `path`
        /// ends in `.msctransfer`, `zip` when it ends in `.zip`, otherwise
        /// `folder`.
        #[arg(long)]
        kind: Option<String>,
        /// Preview a raw folder/ZIP's contents instead of importing it.
        #[arg(long)]
        scan: bool,
        /// `java` or `bedrock`. When omitted for a folder/ZIP import, the
        /// agent scans the source and infers the type.
        #[arg(long = "type")]
        server_type: Option<String>,
        /// Override the imported server's game port.
        #[arg(long = "game-port")]
        game_port: Option<i64>,
        /// Override the imported server's max player count.
        #[arg(long = "max-players")]
        max_players: Option<i64>,
        /// Override the imported server's active/default world name
        /// (Java only).
        #[arg(long = "world-name")]
        world_name: Option<String>,
        /// Accept the EULA on import by writing `eula.txt` (Java only).
        /// Omitting this flag leaves any existing `eula.txt` untouched.
        #[arg(long)]
        eula: bool,
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
    /// Rescan the managed servers root and register untracked servers in place.
    Rescan,
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

#[derive(Debug, Clone, Subcommand)]
pub enum WorldCommand {
    /// List world slots for the active server.
    List,
    /// Create a new slot archived from the current live world.
    Create {
        name: String,
        #[arg(long)]
        seed: Option<String>,
    },
    /// Rename a slot's metadata. Does not touch any files or the live
    /// world — see `worlds/rename-active-world` in the API contract for
    /// that.
    Rename { slot_id: String, name: String },
    /// Delete a non-active slot.
    Delete { slot_id: String },
    /// Duplicate a slot under a new name.
    Duplicate { slot_id: String },
    /// Copy one saved slot's contents into another, overwriting it.
    /// Neither slot needs to be active; the live world is untouched.
    Copy {
        /// The existing destination slot being overwritten.
        #[arg(long)]
        into: String,
        /// The source slot whose saved contents replace it.
        #[arg(long)]
        from: String,
    },
    /// Import a local world ZIP as a new slot.
    Import {
        /// Path to a local world ZIP file.
        path: PathBuf,
        /// Name for the new slot.
        name: String,
    },
    /// Export a slot's saved archive to a local ZIP file.
    Export {
        slot_id: String,
        /// Local path to write the exported ZIP to.
        #[arg(long)]
        output: PathBuf,
    },
    /// Activate a slot as the active/live world. Long-running: refuses a
    /// running server, takes a mandatory safety backup first, then swaps
    /// the live world folders.
    Activate {
        slot_id: String,
        /// Print the operation id and return immediately instead of
        /// waiting for activation to finish.
        #[arg(long)]
        no_wait: bool,
    },
    /// Convert a slot to another server's edition/format via Chunker.
    /// Long-running and always operation-backed — there is no
    /// synchronous variant.
    Convert {
        source_slot_id: String,
        /// The server the converted world is placed on (must be the
        /// opposite edition from the source server).
        #[arg(long = "target-server")]
        target_server_id: String,
        /// The Chunker format to convert to, from
        /// `msc world convert-formats` (not yet a CLI command — see
        /// `GET /v1/worlds` capability output or ask the target agent).
        #[arg(long = "target-format")]
        target_format: String,
        /// Place the result into a fresh slot with this name. Exactly
        /// one of `--target-name`/`--target-slot` is required.
        #[arg(long = "target-name")]
        target_name: Option<String>,
        /// Overwrite this existing slot on the target server instead.
        #[arg(long = "target-slot")]
        target_slot_id: Option<String>,
        /// Print the operation id and return immediately instead of
        /// waiting for conversion to finish.
        #[arg(long)]
        no_wait: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum BackupCommand {
    /// List backups for the active server.
    List,
    /// Trigger a manual backup now. Long-running: pauses/resumes saves
    /// on a running server and verifies the archive before it is listed.
    Now {
        /// Print the operation id and return immediately instead of
        /// waiting for the backup to finish.
        #[arg(long)]
        no_wait: bool,
    },
    /// Delete a backup. Refuses to delete the last remaining verified
    /// backup.
    Delete { backup_id: String },
    /// Restore a backup into the active server. Long-running: refuses a
    /// running server, takes a mandatory safety backup first, then
    /// verifies and installs the requested backup.
    Restore {
        backup_id: String,
        /// Print the operation id and return immediately instead of
        /// waiting for the restore to finish.
        #[arg(long)]
        no_wait: bool,
    },
    /// Read or change the active server's backup schedule and retention.
    Config {
        #[command(subcommand)]
        command: BackupConfigCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum BackupConfigCommand {
    /// Show the active server's current backup configuration.
    Get,
    /// Change one or more backup configuration fields.
    Set {
        #[arg(long)]
        enabled: Option<bool>,
        #[arg(long)]
        interval_minutes: Option<i64>,
        #[arg(long)]
        max_count: Option<i64>,
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

    fn api(status: StatusCode, body: &[u8]) -> Self {
        let text = String::from_utf8_lossy(body);
        let message = match serde_json::from_slice::<ErrorDto>(body) {
            Ok(error) => format!("API {} {}: {}", status.as_u16(), error.code, error.message),
            Err(_) => format!("API {}: {}", status.as_u16(), text.trim()),
        };
        Self {
            exit_code: 3,
            message,
            json_message: Some(text.into_owned()),
        }
    }

    /// A long-running operation reached `failed`. Exit code 3, matching
    /// [`Self::api`] — from the caller's perspective this is the same
    /// class of "the agent refused/couldn't complete the request" error,
    /// just discovered by polling instead of from the initiating
    /// response.
    fn operation_failed(operation: &OperationDto) -> Self {
        let message = match &operation.error {
            Some(error) => format!("operation failed: {} {}", error.code, error.message),
            None => "operation failed".to_string(),
        };
        Self {
            exit_code: 3,
            message,
            json_message: serde_json::to_string(operation).ok(),
        }
    }

    /// A long-running operation reached `cancelled` — a distinct outcome
    /// from a failure, so it gets its own exit code rather than
    /// overloading [`Self::operation_failed`]'s.
    fn operation_cancelled(operation: &OperationDto) -> Self {
        Self {
            exit_code: 4,
            message: "operation was cancelled".to_string(),
            json_message: serde_json::to_string(operation).ok(),
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
        #[cfg(target_os = "linux")]
        Command::CredentialHelper { .. } => {
            Err(CliError::internal("credential-helper is handled in main"))
        }
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
        Command::World { command } => run_world(common, command).await,
        Command::Backup { command } => run_backup(common, command).await,
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
            scan,
            server_type,
            game_port,
            max_players,
            world_name,
            eula,
            transfer_mode,
            backup_path,
            java_port_overrides,
            bedrock_port_overrides,
        } => {
            let resolved_kind = kind.clone().unwrap_or_else(|| {
                let lower = path.to_ascii_lowercase();
                if lower.ends_with(".msctransfer") {
                    "transfer".to_string()
                } else if lower.ends_with(".zip") {
                    "zip".to_string()
                } else {
                    "folder".to_string()
                }
            });
            let is_transfer = resolved_kind == "transfer";
            let action = if scan {
                "scan"
            } else if is_transfer {
                "importTransfer"
            } else {
                "importExisting"
            };
            let body = ServerImportRequestDto {
                action: Some(action.to_string()),
                source_path: Some(path.clone()),
                import_kind: Some(resolved_kind),
                display_name: name.clone(),
                server_type: server_type.clone(),
                active_world_name: world_name.clone(),
                port: game_port,
                max_players,
                accept_eula: eula.then_some(true),
                enable_playit: None,
                transfer_mode: transfer_mode.clone(),
                backup_path: backup_path.clone(),
                java_port_overrides: parse_port_overrides(&java_port_overrides)?,
                bedrock_port_overrides: parse_port_overrides(&bedrock_port_overrides)?,
            };

            if scan {
                let result: ServerImportScanResponseDto =
                    client.post_json("/v1/servers/import", &body).await?;
                if common.json {
                    print_json(&result)?;
                } else {
                    println!("{}", result.message);
                    if let Some(server_type) = &result.server_type {
                        println!("type: {server_type}");
                    }
                    if let Some(is_zip) = result.is_zip {
                        println!("zip: {is_zip}");
                    }
                    if let Some(flavor) = &result.java_flavor {
                        println!("java flavor: {flavor}");
                    }
                    if let Some(mc_version) = &result.detected_mc_version {
                        println!("minecraft version: {mc_version}");
                    }
                    if let Some(loader_version) = &result.detected_loader_version {
                        println!("loader version: {loader_version}");
                    }
                    if let Some(port) = result.port {
                        println!("port: {port}");
                    }
                    if let Some(max_players) = result.max_players {
                        println!("max players: {max_players}");
                    }
                    if let Some(eula_accepted) = result.eula_accepted {
                        println!("eula accepted: {eula_accepted}");
                    }
                    if let Some(default_world) = &result.default_world_name {
                        println!("default world: {default_world}");
                    }
                    for world in &result.worlds {
                        println!(
                            "world: {} ({}, {} bytes)",
                            world.name, world.dimensions_label, world.size_bytes
                        );
                    }
                }
                return Ok(());
            }

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
        ServerCommand::Rescan => {
            let body = ServerImportRequestDto {
                action: Some("rescan".to_string()),
                source_path: None,
                import_kind: None,
                display_name: None,
                server_type: None,
                active_world_name: None,
                port: None,
                max_players: None,
                accept_eula: None,
                enable_playit: None,
                transfer_mode: None,
                backup_path: None,
                java_port_overrides: HashMap::new(),
                bedrock_port_overrides: HashMap::new(),
            };
            let result: ServerImportResultDto =
                client.post_json("/v1/servers/import", &body).await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
                if let Some(imported) = result.imported {
                    println!("imported: {imported}");
                }
                if let Some(skipped) = result.skipped {
                    println!("skipped: {skipped}");
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

async fn run_world(common: CommonArgs, command: WorldCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        WorldCommand::List => {
            let slots: WorldSlotsResponseDto = client.get_json("/v1/worlds").await?;
            if common.json {
                print_json(&slots)?;
            } else {
                print_world_slots(&slots);
            }
            Ok(())
        }
        WorldCommand::Create { name, seed } => {
            let body = WorldCreateRequestDto { name, seed };
            let result: WorldMutationResultDto =
                client.post_json("/v1/worlds/create", &body).await?;
            print_world_mutation_result(common.json, &result)
        }
        WorldCommand::Rename { slot_id, name } => {
            let body = WorldRenameRequestDto { slot_id, name };
            let result: WorldMutationResultDto =
                client.post_json("/v1/worlds/rename", &body).await?;
            print_world_mutation_result(common.json, &result)
        }
        WorldCommand::Delete { slot_id } => {
            let body = WorldDeleteRequestDto { slot_id };
            let result: WorldMutationResultDto =
                client.post_json("/v1/worlds/delete", &body).await?;
            print_world_mutation_result(common.json, &result)
        }
        WorldCommand::Duplicate { slot_id } => {
            let body = WorldDuplicateRequestDto { slot_id };
            let result: WorldMutationResultDto =
                client.post_json("/v1/worlds/duplicate", &body).await?;
            print_world_mutation_result(common.json, &result)
        }
        WorldCommand::Copy { into, from } => {
            let body = WorldReplaceRequestDto {
                slot_id: into,
                source_slot_id: from,
            };
            let result: WorldMutationResultDto =
                client.post_json("/v1/worlds/replace", &body).await?;
            print_world_mutation_result(common.json, &result)
        }
        WorldCommand::Import { path, name } => {
            let bytes = tokio::fs::read(&path).await.map_err(|err| {
                CliError::usage(format!("failed to read {}: {err}", path.display()))
            })?;
            let begin: StagedUploadBeginResultDto = client
                .post_json(
                    "/v1/staged-uploads",
                    &StagedUploadBeginRequestDto {
                        purpose: StagedUploadPurposeDto::WorldImport,
                        content_type: None,
                    },
                )
                .await?;
            let _uploaded: StagedUploadCompleteResultDto = client
                .put_bytes(&begin.upload_path, "application/octet-stream", bytes)
                .await?;
            let body = WorldImportRequestDto {
                name,
                staged_upload_id: begin.staged_upload_id,
            };
            let result: WorldMutationResultDto =
                client.post_json("/v1/worlds/import", &body).await?;
            print_world_mutation_result(common.json, &result)
        }
        WorldCommand::Export { slot_id, output } => {
            let body = WorldExportRequestDto { slot_id };
            let result: WorldExportResultDto = client.post_json("/v1/worlds/export", &body).await?;
            let bytes = client
                .get_raw_bytes(&format!(
                    "/v1/staged-downloads/{}",
                    result.staged_download_id
                ))
                .await?;
            tokio::fs::write(&output, &bytes).await.map_err(|err| {
                CliError::internal(format!("failed to write {}: {err}", output.display()))
            })?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("exported {} bytes to {}", bytes.len(), output.display());
            }
            Ok(())
        }
        WorldCommand::Activate { slot_id, no_wait } => {
            let body = WorldActivateRequestDto { slot_id };
            let result: WorldActivateResultDto =
                client.post_json("/v1/worlds/activate", &body).await?;
            finish_operation(
                &client,
                common.json,
                no_wait,
                result.operation_id,
                "activation",
            )
            .await
        }
        WorldCommand::Convert {
            source_slot_id,
            target_server_id,
            target_format,
            target_name,
            target_slot_id,
            no_wait,
        } => {
            match (&target_name, &target_slot_id) {
                (Some(_), None) | (None, Some(_)) => {}
                _ => {
                    return Err(CliError::usage(
                        "exactly one of --target-name or --target-slot must be given",
                    ));
                }
            }
            let body = WorldConvertRequestDto {
                source_slot_id,
                target_server_id,
                target_format,
                target_name,
                target_slot_id,
            };
            let result: WorldConvertResultDto =
                client.post_json("/v1/worlds/convert", &body).await?;
            finish_operation(
                &client,
                common.json,
                no_wait,
                Some(result.operation_id),
                "conversion",
            )
            .await
        }
    }
}

async fn run_backup(common: CommonArgs, command: BackupCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        BackupCommand::List => {
            let backups: BackupsResponseDto = client.get_json("/v1/backups").await?;
            if common.json {
                print_json(&backups)?;
            } else {
                print_backups(&backups);
            }
            Ok(())
        }
        BackupCommand::Now { no_wait } => {
            let result: BackupNowResultDto = client
                .post_json("/v1/backups/now", &serde_json::json!({}))
                .await?;
            finish_operation(&client, common.json, no_wait, result.operation_id, "backup").await
        }
        BackupCommand::Delete { backup_id } => {
            let body = BackupDeleteRequestDto { backup_id };
            let result: SimpleResultDto = client.post_json("/v1/backups/delete", &body).await?;
            if common.json {
                print_json(&result)?;
            } else {
                print_simple_result("Backup deleted.", &result);
            }
            Ok(())
        }
        BackupCommand::Restore { backup_id, no_wait } => {
            let body = BackupRestoreRequestDto { backup_id };
            let result: BackupRestoreResultDto =
                client.post_json("/v1/backups/restore", &body).await?;
            finish_operation(
                &client,
                common.json,
                no_wait,
                result.operation_id,
                "restore",
            )
            .await
        }
        BackupCommand::Config { command } => match command {
            BackupConfigCommand::Get => {
                let config: BackupConfigResponseDto = client.get_json("/v1/backups/config").await?;
                if common.json {
                    print_json(&config)?;
                } else {
                    print_backup_config(&config);
                }
                Ok(())
            }
            BackupConfigCommand::Set {
                enabled,
                interval_minutes,
                max_count,
            } => {
                if enabled.is_none() && interval_minutes.is_none() && max_count.is_none() {
                    return Err(CliError::usage(
                        "at least one of --enabled/--interval-minutes/--max-count must be given",
                    ));
                }
                let body = BackupConfigUpdateRequestDto {
                    auto_backup_enabled: enabled,
                    auto_backup_interval_minutes: interval_minutes,
                    auto_backup_max_count: max_count,
                };
                let result: BackupConfigUpdateResultDto =
                    client.post_json("/v1/backups/config", &body).await?;
                if common.json {
                    print_json(&result)?;
                } else {
                    println!("{}", result.message);
                    if let Some(config) = &result.config {
                        print_backup_config(config);
                    }
                }
                Ok(())
            }
        },
    }
}

/// Shared tail for every async world/backup operation
/// (`world activate`/`convert`, `backup now`/`restore`): print the
/// operation id, then either return immediately (`--no-wait`) or poll it
/// to a terminal state.
async fn finish_operation(
    client: &RemoteClient,
    json: bool,
    no_wait: bool,
    operation_id: Option<String>,
    label: &str,
) -> Result<(), CliError> {
    let Some(operation_id) = operation_id else {
        return Err(CliError::internal(format!(
            "the agent did not return an operation id for this {label}"
        )));
    };
    if no_wait {
        if json {
            print_json(&serde_json::json!({ "operationId": operation_id }))?;
        } else {
            println!("operation id: {operation_id}");
            println!(
                "not waiting (--no-wait was given); poll GET /v1/operations/{operation_id} yourself."
            );
        }
        return Ok(());
    }
    if !json {
        println!("operation id: {operation_id}");
    }
    poll_operation(client, &operation_id, json).await
}

/// Polls `GET /v1/operations/{id}` to a terminal state, printing each
/// distinct `statusLine` change in human mode. A Ctrl-C during the wait
/// sends one `POST /v1/operations/{id}/cancel` and keeps polling — the
/// operation's own record moves to `cancelled` when the agent honors it
/// (see `routes/worlds.rs`'s module doc: cancellation is real at the
/// operation-record level; the underlying filesystem/process work may
/// still run to completion in the background).
async fn poll_operation(
    client: &RemoteClient,
    operation_id: &str,
    json: bool,
) -> Result<(), CliError> {
    let cancel_requested = Arc::new(AtomicBool::new(false));
    let watcher_flag = cancel_requested.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            watcher_flag.store(true, Ordering::SeqCst);
        }
    });

    let mut last_status_line: Option<String> = None;
    let mut cancel_sent = false;
    loop {
        let operation: OperationDto = client
            .get_json(&format!("/v1/operations/{operation_id}"))
            .await?;
        if !json && operation.status_line != last_status_line {
            if let Some(line) = &operation.status_line {
                println!("{line}");
            }
            last_status_line = operation.status_line.clone();
        }
        match operation.state {
            OperationStateDto::Succeeded => {
                if json {
                    print_json(&operation)?;
                } else {
                    println!("done.");
                }
                return Ok(());
            }
            OperationStateDto::Failed => {
                return Err(CliError::operation_failed(&operation));
            }
            OperationStateDto::Cancelled => {
                return Err(CliError::operation_cancelled(&operation));
            }
            OperationStateDto::Queued | OperationStateDto::Running => {}
        }
        if cancel_requested.load(Ordering::SeqCst) && !cancel_sent {
            if !json {
                println!("cancellation requested; asking the agent to cancel {operation_id}...");
            }
            let _: Result<serde_json::Value, CliError> = client
                .post_json(
                    &format!("/v1/operations/{operation_id}/cancel"),
                    &serde_json::json!({}),
                )
                .await;
            cancel_sent = true;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}

fn print_world_slots(response: &WorldSlotsResponseDto) {
    println!("server running: {}", response.server_running);
    if let Some(active) = &response.active_slot_id {
        println!("active slot: {active}");
    }
    if response.slots.is_empty() {
        println!("No world slots.");
    }
    for slot in &response.slots {
        print_world_slot(slot);
    }
}

fn print_world_slot(slot: &WorldSlotDto) {
    let marker = if slot.is_active { "*" } else { " " };
    println!(
        "{marker} {} ({}) created {}",
        slot.name, slot.id, slot.created_at
    );
}

fn print_world_mutation_result(
    json: bool,
    result: &WorldMutationResultDto,
) -> Result<(), CliError> {
    if json {
        print_json(result)?;
    } else {
        println!("{}", result.message);
        if let Some(updated) = &result.updated {
            print_world_slots(updated);
        }
    }
    if result.success {
        Ok(())
    } else {
        Err(CliError::usage(result.message.clone()))
    }
}

fn print_backups(response: &BackupsResponseDto) {
    if response.backups.is_empty() {
        println!("No backups.");
        return;
    }
    for backup in &response.backups {
        let trigger = if backup.is_automatic {
            "auto"
        } else {
            "manual"
        };
        let size = backup
            .file_size
            .map(|bytes| bytes.to_string())
            .unwrap_or_else(|| "?".to_string());
        println!(
            "{} [{trigger}/{}] {} bytes {}",
            backup.id, backup.trigger_reason, size, backup.display_name
        );
    }
}

fn print_backup_config(config: &BackupConfigResponseDto) {
    println!("server: {}", config.server_name);
    println!("enabled: {}", config.auto_backup_enabled);
    println!("interval minutes: {}", config.auto_backup_interval_minutes);
    println!("max count: {}", config.auto_backup_max_count);
    if !config.interval_options.is_empty() {
        let options: Vec<String> = config
            .interval_options
            .iter()
            .map(|value| value.to_string())
            .collect();
        println!("interval options: {}", options.join(", "));
    }
    if let Some(note) = &config.note {
        println!("note: {note}");
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
        let response = self.request_raw(Method::GET, path, None, None).await?;
        decode_json(&response.body)
    }

    async fn post_json<Req: Serialize + ?Sized, Resp: DeserializeOwned>(
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

    /// Uploads raw bytes (a staged world-import ZIP) rather than a JSON
    /// body — the one non-JSON request this CLI makes.
    async fn put_bytes<Resp: DeserializeOwned>(
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

    /// Downloads a raw response body (a staged world-export ZIP) instead
    /// of decoding it as JSON.
    async fn get_raw_bytes(&self, path: &str) -> Result<Vec<u8>, CliError> {
        let response = self.request_raw(Method::GET, path, None, None).await?;
        Ok(response.body)
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

fn decode_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CliError> {
    serde_json::from_slice(bytes)
        .map_err(|err| CliError::internal(format!("failed to decode response JSON: {err}")))
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
    body: Vec<u8>,
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
    let header_bytes = &response[..header_end];
    let body_bytes = &response[header_end + 4..];
    let headers = String::from_utf8(header_bytes.to_vec())
        .map_err(|err| format!("response headers were not valid UTF-8: {err}"))?;
    let body = if let Some(body_len) = parse_content_length(&headers)? {
        let wanted = body_len.min(body_bytes.len());
        body_bytes[..wanted].to_vec()
    } else {
        body_bytes.to_vec()
    };
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
