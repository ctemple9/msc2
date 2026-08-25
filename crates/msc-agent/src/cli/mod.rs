//! Phase 4 CLI commands. Every subcommand except `serve` talks to the
//! same HTTP API the iOS client uses.

pub mod service;

use std::collections::HashMap;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::http::{Method, StatusCode, Uri};
use clap::{Args, Subcommand};
use msc_api::dto::{
    ActiveServerRequestDto, AddonRemoveRequestDto, AddonRemoveResultDto, AddonUpdateResultDto,
    AddonsResponseDto, BackupConfigResponseDto, BackupConfigUpdateRequestDto,
    BackupConfigUpdateResultDto, BackupDeleteRequestDto, BackupNowResultDto,
    BackupRestoreRequestDto, BackupRestoreResultDto, BackupsResponseDto, BedrockRuntimeStateDto,
    BroadcastAuthPromptDto, BroadcastAutoStartDto, BroadcastCredentialsDto,
    BroadcastJarDownloadResultDto, BroadcastSimpleResultDto, BroadcastStatusDto, CapabilitiesDto,
    CatalogInstallRequestDto, CatalogInstallResultDto, CatalogSearchResponseDto,
    ClientExportResponseDto, CommandRequestDto, CommandResultDto, ComponentUpdateRequestDto,
    ConnectivityResponseDto, DuckDnsStatusResponseDto, DuckDnsUpdateRequestDto, ErrorDto,
    HealthProblemsResponseDto, HealthRepairRequestDto, HealthRepairResultDto, HealthResponseDto,
    JavaConfigResponseDto, JavaConfigSetRequestDto, JavaRuntimeInstallRequestDto,
    JavaRuntimeInstallResultDto, JavaRuntimesResponseDto, ModpackImportRequestDto,
    ModpackImportResultDto, ModpackInspectionRequestDto, ModpackInspectionResultDto,
    ModpackManualFileRequestDto, ModpackManualFileResultDto, OperationDto, OperationStateDto,
    PlayitActionResultDto, PlayitStatusDto, RemoteApiStatus, ResourcePackActivateRequestDto,
    ResourcePackMutationResultDto, ResourcePacksResponseDto, ServerCreateRequestDto,
    ServerCreateResultDto, ServerDeleteRequestDto, ServerDeleteResultDto, ServerDto,
    ServerEulaRequestDto, ServerEulaResultDto, ServerImportRequestDto, ServerImportResultDto,
    ServerImportScanResponseDto, ServerRenameRequestDto, ServerRenameResultDto,
    SettingsResponseDto, SettingsUpdateRequestDto, SettingsUpdateResultDto, SimpleResultDto,
    StagedUploadBeginRequestDto, StagedUploadBeginResultDto, StagedUploadCompleteResultDto,
    StagedUploadPurposeDto, TemplateMutationRequestDto, TemplateMutationResultDto,
    TemplatesResponseDto, VersionChangeRequestDto, VersionChangeResultDto, VersionsResponseDto,
    WorldActivateRequestDto, WorldActivateResultDto, WorldConvertRequestDto, WorldConvertResultDto,
    WorldCreateRequestDto, WorldDeleteRequestDto, WorldDuplicateRequestDto, WorldExportRequestDto,
    WorldExportResultDto, WorldImportRequestDto, WorldMutationResultDto, WorldRenameRequestDto,
    WorldReplaceActiveRequestDto, WorldReplaceActiveResultDto, WorldReplaceRequestDto,
    WorldSlotDto, WorldSlotsResponseDto,
};
use msc_infrastructure::archive::create_zip_from_folders;
use msc_infrastructure::console_buffer::ConsoleLine;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 48001;

#[derive(Debug, Clone, Args)]
pub struct CommonArgs {
    /// Full base URL for the agent, for example http://127.0.0.1:48001.
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
        #[arg(long, default_value = "127.0.0.1:48001")]
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
    /// Show the agent's host and token capability advertisement.
    Capabilities,
    /// Read connectivity and the DuckDNS hostname label.
    Network {
        #[command(subcommand)]
        command: NetworkCommand,
    },
    /// Control the Playit player-connectivity tunnel.
    Playit {
        #[command(subcommand)]
        command: PlayitCommand,
    },
    /// Control Xbox Broadcast and its account/helper state.
    Broadcast {
        #[command(subcommand)]
        command: BroadcastCommand,
    },
    /// List or mutate Java resource-pack publication.
    ResourcePack {
        #[command(subcommand)]
        command: ResourcePackCommand,
    },
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
    /// Inspect Bedrock players or mutate its allowlist through shared API routes.
    Bedrock {
        #[command(subcommand)]
        command: BedrockCommand,
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
    /// List or change the active server's available/current JAR version.
    Version {
        #[command(subcommand)]
        command: VersionCommand,
    },
    /// List Paper/plugin templates, export the active server as one, or
    /// create a server from one.
    Template {
        #[command(subcommand)]
        command: TemplateCommand,
    },
    /// List detected Java runtimes, or read/change/install one.
    Java {
        #[command(subcommand)]
        command: JavaCommand,
    },
    /// Show the active server's health cards and startup problems, or
    /// repair one.
    Doctor {
        #[command(subcommand)]
        command: Option<DoctorCommand>,
    },
    /// List, search, install, update, remove, link, or export add-ons.
    Addon {
        #[command(subcommand)]
        command: AddonCommand,
    },
    /// Inspect, import, replace, or resume a staged modpack import.
    Modpack {
        #[command(subcommand)]
        command: ModpackCommand,
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
pub enum NetworkCommand {
    /// Show the active server's player-connectivity diagnostics.
    Connectivity,
    /// Read or set the plain DuckDNS hostname label.
    Duckdns {
        #[command(subcommand)]
        command: DuckdnsCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum DuckdnsCommand {
    Get,
    Set {
        /// Empty the label with `--hostname ""`.
        #[arg(long)]
        hostname: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum PlayitCommand {
    Status,
    Start {
        #[arg(long)]
        no_wait: bool,
    },
    Stop,
}

#[derive(Debug, Clone, Subcommand)]
pub enum BroadcastCommand {
    Status,
    Start {
        #[arg(long)]
        no_wait: bool,
    },
    Stop,
    Restart {
        #[arg(long)]
        no_wait: bool,
    },
    DownloadJar {
        #[arg(long)]
        no_wait: bool,
    },
    AuthPrompt,
    DismissAuthPrompt,
    Autostart {
        #[command(subcommand)]
        command: BroadcastAutostartCommand,
    },
    Credentials {
        email: String,
        password: String,
        gamertag: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum BroadcastAutostartCommand {
    Get,
    Set { enabled: bool },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ResourcePackCommand {
    List,
    Activate {
        /// Existing approved ZIP name; omit to clear the active pack.
        pack_id: Option<String>,
        #[arg(long)]
        require: bool,
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
    /// Create a new Java server. Long-running for Forge/NeoForge (a real
    /// supervised installer run); always operation-backed.
    Create(ServerCreateArgs),
    /// Delete a server. Refuses a running server.
    Delete { server: String },
    /// Rename a server's display name (not its on-disk folder).
    Rename { server: String, name: String },
    /// Accept the Minecraft EULA for a server (writes `eula.txt`).
    Eula {
        /// Select a server by id or display name. Defaults to the active
        /// server.
        #[arg(long)]
        server: Option<String>,
    },
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Args)]
pub struct ServerCreateArgs {
    /// Display name for the new server.
    name: String,
    /// `java` (default) or `bedrock`.
    #[arg(long = "type")]
    server_type: Option<String>,
    /// `paper` (default), `purpur`, `vanilla`, `fabric`, `neoforge`, or
    /// `forge`.
    #[arg(long)]
    flavor: Option<String>,
    /// Game port. Defaults to 25565.
    #[arg(long)]
    port: Option<u16>,
    #[arg(long = "max-players")]
    max_players: Option<i64>,
    #[arg(long)]
    difficulty: Option<String>,
    #[arg(long)]
    gamemode: Option<String>,
    #[arg(long = "world-name")]
    world_name: Option<String>,
    #[arg(long = "world-seed")]
    world_seed: Option<String>,
    /// A specific version id from `msc version list --create --flavor
    /// <flavor>`, or omit for the latest.
    #[arg(long = "version-id")]
    version_id: Option<String>,
    #[arg(long = "loader-version")]
    loader_version: Option<String>,
    /// Accept the Minecraft EULA immediately after creation.
    #[arg(long)]
    accept_eula: bool,
    #[arg(long = "cross-play")]
    enable_cross_play: bool,
    #[arg(long = "cross-play-bedrock-port")]
    cross_play_bedrock_port: Option<u16>,
    #[arg(long)]
    playit: bool,
    #[arg(long = "xbox-broadcast")]
    xbox_broadcast: bool,
    /// Override the Java executable used for this create only (Forge/
    /// NeoForge installer runs).
    #[arg(long = "java-path")]
    java_path: Option<String>,
    /// Create the server from a local .mrpack or CurseForge archive.
    #[arg(long = "modpack")]
    modpack: Option<PathBuf>,
    /// Print the operation id and return immediately instead of waiting
    /// for creation to finish.
    #[arg(long)]
    no_wait: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum VersionCommand {
    /// List versions for the active server's flavor.
    List,
    /// List versions for the create flow, given a server type and Java
    /// flavor (neither needs an existing server).
    Create {
        #[arg(long = "type", default_value = "java")]
        server_type: String,
        #[arg(long)]
        flavor: Option<String>,
    },
    /// Change the active server's JAR version/build. Long-running for
    /// Forge/NeoForge; always operation-backed.
    Set {
        version_id: String,
        #[arg(long = "loader-version")]
        loader_version: Option<String>,
        /// Print the operation id and return immediately instead of
        /// waiting for the change to finish.
        #[arg(long)]
        no_wait: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum TemplateCommand {
    /// List Paper and plugin templates.
    List,
    /// Export the active server as a template.
    Export {
        #[arg(long = "no-plugins")]
        no_plugins: bool,
    },
    /// Create a new server from a template.
    Create {
        /// A template id from `msc template list`, e.g.
        /// `paper:paper-1.21.4-build100.jar`.
        template_id: String,
        name: String,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long = "world-name")]
        world_name: Option<String>,
        #[arg(long)]
        difficulty: Option<String>,
        #[arg(long)]
        gamemode: Option<String>,
        #[arg(long = "world-seed")]
        world_seed: Option<String>,
        #[arg(long)]
        accept_eula: bool,
        #[arg(long = "cross-play")]
        enable_cross_play: bool,
        #[arg(long = "cross-play-bedrock-port")]
        cross_play_bedrock_port: Option<u16>,
        #[arg(long)]
        playit: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum JavaCommand {
    /// List Java runtimes detected on this host.
    List,
    /// Show the global Java executable path override.
    Get,
    /// Set the global Java executable path override.
    Set { path: String },
    /// Install a Java runtime this agent manages itself. Always
    /// operation-backed (no synchronous variant).
    Install {
        /// One of 8, 17, 21, 25.
        major: i64,
        /// Print the operation id and return immediately instead of
        /// waiting for the install to finish.
        #[arg(long)]
        no_wait: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum DoctorCommand {
    /// Attempt a repair for a diagnosed startup problem.
    Repair {
        problem_id: String,
        /// `disable`, `delete`, `update`, or `install`.
        action: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum AddonCommand {
    /// List installed add-ons for the active server.
    List,
    /// Search the active server's filtered Modrinth catalog.
    Search {
        query: String,
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },
    /// Install one add-on from the active server's filtered Modrinth catalog.
    InstallCatalog {
        project_id: String,
        #[arg(long)]
        slug: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        no_wait: bool,
    },
    /// Upload and install one local jar into the active server.
    InstallLocal {
        path: PathBuf,
        #[arg(long)]
        no_wait: bool,
    },
    /// Update one installed add-on.
    Update {
        jar_stem: String,
        #[arg(long)]
        no_wait: bool,
    },
    /// Update every installed add-ons with a compatible update.
    UpdateAll {
        #[arg(long)]
        no_wait: bool,
    },
    /// Enable one disabled add-on.
    Enable { jar_stem: String },
    /// Disable one enabled add-on.
    Disable { jar_stem: String },
    /// Remove one installed add-on.
    Remove { jar_stem: String },
    /// Manually link one jar stem to a Modrinth project id.
    Link {
        jar_stem: String,
        project_id: String,
    },
    /// Set a plugin source URL for one jar stem.
    SetSource { jar_stem: String, url: String },
    /// Remove a plugin source URL for one jar stem.
    RemoveSource { jar_stem: String },
    /// Export client-side add-ons to a local file or stdout.
    Export {
        #[arg(long = "selected")]
        selected_ids: Vec<String>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ModpackCommand {
    /// Inspect a local .mrpack or CurseForge zip without mutating the server.
    Inspect { path: PathBuf },
    /// Import a local modpack into the active server.
    Import {
        path: PathBuf,
        #[arg(long)]
        no_wait: bool,
    },
    /// Explicitly replace the active server's current pack with a new local modpack.
    Replace {
        path: PathBuf,
        #[arg(long)]
        no_wait: bool,
    },
    /// Complete one pending author-blocked CurseForge file upload.
    ManualFile {
        operation_id: String,
        file_id: String,
        path: PathBuf,
    },
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
pub enum BedrockCommand {
    /// List player records discovered from the active Bedrock world's LevelDB.
    Players,
    /// Read or mutate the active Bedrock allowlist.
    Allowlist {
        #[command(subcommand)]
        command: BedrockAllowlistCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum BedrockAllowlistCommand {
    Get,
    Add { name: String },
    Remove { name: String },
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
    /// Replace the active/live world's on-disk content directly, from a
    /// local folder, a local ZIP, or fresh generation. Distinct from
    /// `copy` (a saved-slot-to-saved-slot copy that never touches the
    /// live world). Long-running: refuses a running server, takes a
    /// mandatory safety backup first, then swaps the live world folders.
    ReplaceActive {
        /// New level name to commit to server.properties. For a folder
        /// or ZIP `--source`, this must match the source's own top-level
        /// folder name — the agent does not rename folders on this
        /// route's behalf.
        new_level_name: String,
        /// A local world folder or ZIP file to upload as the
        /// replacement. Omit for a fresh (empty) world.
        #[arg(long)]
        source: Option<PathBuf>,
        /// Print the operation id and return immediately instead of
        /// waiting for replacement to finish.
        #[arg(long)]
        no_wait: bool,
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
        Command::Capabilities => run_capabilities(common).await,
        Command::Network { command } => run_network(common, command).await,
        Command::Playit { command } => run_playit(common, command).await,
        Command::Broadcast { command } => run_broadcast(common, command).await,
        Command::ResourcePack { command } => run_resource_pack(common, command).await,
        Command::Service { command } => service::run(common, command).await,
        Command::Server { command } => run_server(common, command).await,
        Command::Send(args) => run_command(common, args).await,
        Command::Console { command } => run_console(common, command).await,
        Command::Settings { command } => run_settings(common, command).await,
        Command::Bedrock { command } => run_bedrock(common, command).await,
        Command::World { command } => run_world(common, command).await,
        Command::Backup { command } => run_backup(common, command).await,
        Command::Version { command } => run_version(common, command).await,
        Command::Template { command } => run_template(common, command).await,
        Command::Java { command } => run_java(common, command).await,
        Command::Doctor { command } => run_doctor(common, command).await,
        Command::Addon { command } => run_addon(common, command).await,
        Command::Modpack { command } => run_modpack(common, command).await,
    }
}

async fn run_bedrock(common: CommonArgs, command: BedrockCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        BedrockCommand::Players => {
            let result: serde_json::Value = client.get_json("/v1/players").await?;
            if common.json {
                print_json(&result)?;
            } else {
                let count = result["count"].as_u64().unwrap_or(0);
                println!("players: {count}");
                if let Some(players) = result["players"].as_array() {
                    for player in players {
                        println!("- {}", player["name"].as_str().unwrap_or("unknown"));
                    }
                }
                print_runtime_value(result.get("runtime"));
            }
        }
        BedrockCommand::Allowlist { command } => match command {
            BedrockAllowlistCommand::Get => {
                let result: serde_json::Value = client.get_json("/v1/allowlist").await?;
                print_bedrock_json(&common, &result)?;
            }
            BedrockAllowlistCommand::Add { name } => {
                let result: serde_json::Value = client
                    .post_json(
                        "/v1/allowlist",
                        &serde_json::json!({"action": "add", "name": name}),
                    )
                    .await?;
                print_bedrock_json(&common, &result)?;
            }
            BedrockAllowlistCommand::Remove { name } => {
                let result: serde_json::Value = client
                    .post_json(
                        "/v1/allowlist",
                        &serde_json::json!({"action": "remove", "name": name}),
                    )
                    .await?;
                print_bedrock_json(&common, &result)?;
            }
        },
    }
    Ok(())
}

fn print_bedrock_json(common: &CommonArgs, value: &serde_json::Value) -> Result<(), CliError> {
    if common.json {
        print_json(value)
    } else {
        println!("{}", value["message"].as_str().unwrap_or("ok"));
        print_runtime_value(value.get("runtime"));
        Ok(())
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
            if !common.json {
                println!("{}", result.message);
                print_runtime(&result.runtime);
            }
            finish_operation(
                &client,
                common.json,
                false,
                result.operation_id,
                "server import",
            )
            .await
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
            if !common.json {
                println!("{}", result.message);
            }
            finish_operation(
                &client,
                common.json,
                false,
                result.operation_id,
                "server rescan",
            )
            .await
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
        ServerCommand::Create(args) => {
            let no_wait = args.no_wait;
            let staged_modpack_upload_id = if let Some(path) = args.modpack.as_ref() {
                Some(
                    stage_file_upload(
                        &client,
                        path,
                        StagedUploadPurposeDto::ModpackArchive,
                        None,
                        None,
                    )
                    .await?,
                )
            } else {
                None
            };
            let body = ServerCreateRequestDto {
                name: args.name,
                server_type: args.server_type,
                java_flavor: args.flavor,
                port: args.port.map(i64::from),
                max_players: args.max_players,
                enable_cross_play: args.enable_cross_play.then_some(true),
                cross_play_bedrock_port: args.cross_play_bedrock_port.map(i64::from),
                enable_playit: args.playit.then_some(true),
                enable_xbox_broadcast: args.xbox_broadcast.then_some(true),
                difficulty: args.difficulty,
                gamemode: args.gamemode,
                world_name: args.world_name,
                world_seed: args.world_seed,
                version_id: args.version_id,
                minecraft_version: None,
                loader_version: args.loader_version,
                accept_eula: args.accept_eula.then_some(true),
                bedrock_version: None,
                docker_image: None,
                java_path: args.java_path,
                staged_modpack_upload_id,
            };
            let result: ServerCreateResultDto =
                client.post_json("/v1/servers/create", &body).await?;
            if !common.json {
                println!("{}", result.message);
                if let Some(name) = &result.server_name {
                    println!("name: {name}");
                }
                print_runtime(&result.runtime);
            }
            finish_operation(
                &client,
                common.json,
                no_wait,
                result.operation_id,
                "server creation",
            )
            .await
        }
        ServerCommand::Delete { server } => {
            let resolved = resolve_server(&client, &server).await?;
            let body = ServerDeleteRequestDto {
                server_id: resolved.id,
            };
            let result: ServerDeleteResultDto =
                client.post_json("/v1/servers/delete", &body).await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
            }
            Ok(())
        }
        ServerCommand::Rename { server, name } => {
            let resolved = resolve_server(&client, &server).await?;
            let body = ServerRenameRequestDto {
                server_id: resolved.id,
                name,
            };
            let result: ServerRenameResultDto =
                client.post_json("/v1/servers/rename", &body).await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
            }
            Ok(())
        }
        ServerCommand::Eula { server } => {
            let server_id = if let Some(selector) = server.as_deref() {
                Some(resolve_server(&client, selector).await?.id)
            } else {
                let status: RemoteApiStatus = client.get_json("/v1/status").await?;
                Some(
                    status
                        .active_server_id
                        .ok_or_else(|| CliError::usage("no active server; pass --server"))?,
                )
            };
            let body = ServerEulaRequestDto { server_id };
            let result: ServerEulaResultDto = client.post_json("/v1/servers/eula", &body).await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
            }
            Ok(())
        }
    }
}

/// Zips a local world folder into bytes suitable for `PUT
/// /v1/staged-uploads/{id}`, with one top-level entry named after the
/// folder itself (`create_zip_from_folders(dest, folder.parent(),
/// [folder.file_name()])`) — the same "portable single-folder world"
/// layout `worlds::WorldReplaceSource::ExistingFolder` already produces
/// for in-process callers, reproduced here as a real ZIP because this
/// route only ever accepts a bounded staged upload, never a server-local
/// path (`routes/worlds.rs::replace_active`'s own doc note).
fn zip_folder_to_bytes(path: &Path) -> Result<Vec<u8>, CliError> {
    let folder_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| CliError::usage(format!("{} has no usable folder name", path.display())))?
        .to_string();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temp_zip = std::env::temp_dir().join(format!(
        "msc2-replace-active-{}-{}.zip",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    create_zip_from_folders(&temp_zip, parent, &[folder_name])
        .map_err(|err| CliError::usage(format!("failed to zip {}: {err}", path.display())))?;
    let bytes = std::fs::read(&temp_zip)
        .map_err(|err| CliError::internal(format!("failed to read temporary zip: {err}")))?;
    let _ = std::fs::remove_file(&temp_zip);
    Ok(bytes)
}

fn encode_uri_component(value: &str) -> String {
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

async fn stage_file_upload(
    client: &RemoteClient,
    path: &Path,
    purpose: StagedUploadPurposeDto,
    operation_id: Option<String>,
    file_id: Option<String>,
) -> Result<String, CliError> {
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|err| CliError::usage(format!("failed to read {}: {err}", path.display())))?;
    let begin: StagedUploadBeginResultDto = client
        .post_json(
            "/v1/staged-uploads",
            &StagedUploadBeginRequestDto {
                purpose,
                content_type: None,
                operation_id,
                file_id,
            },
        )
        .await?;
    let _uploaded: StagedUploadCompleteResultDto = client
        .put_bytes(&begin.upload_path, "application/octet-stream", bytes)
        .await?;
    Ok(begin.staged_upload_id)
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
        print_runtime(&result.runtime);
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

async fn run_capabilities(common: CommonArgs) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    let result: CapabilitiesDto = client.get_json("/v1/capabilities").await?;
    if common.json {
        print_json(&result)
    } else {
        println!(
            "agent: {} (API {}.{})",
            result.agent_version, result.api_major, result.api_minor
        );
        println!("host: {:?}", result.host_os);
        println!("permissions: {}", result.permissions.len());
        println!("playit: {}", result.helpers.playit);
        println!("duckdns: {}", result.helpers.duckdns);
        println!("geyser: {}", result.helpers.geyser);
        print_runtime(&result.server_types.bedrock.runtime);
        Ok(())
    }
}

async fn run_network(common: CommonArgs, command: NetworkCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        NetworkCommand::Connectivity => {
            let result: ConnectivityResponseDto = client.get_json("/v1/connectivity").await?;
            if common.json {
                print_json(&result)
            } else {
                println!("{}: {}", result.server_name, result.headline);
                if let Some(address) = result.join_address {
                    println!("join address: {address}");
                }
                println!("local: {}", result.port_diagnostics.local.outcome);
                println!("public: {}", result.port_diagnostics.public.outcome);
                Ok(())
            }
        }
        NetworkCommand::Duckdns { command } => match command {
            DuckdnsCommand::Get => {
                let result: DuckDnsStatusResponseDto = client.get_json("/v1/duckdns").await?;
                if common.json {
                    print_json(&result)
                } else {
                    println!(
                        "{}",
                        result.hostname.unwrap_or_else(|| "not configured".into())
                    );
                    Ok(())
                }
            }
            DuckdnsCommand::Set { hostname } => {
                let result: serde_json::Value = client
                    .post_json(
                        "/v1/duckdns",
                        &DuckDnsUpdateRequestDto {
                            hostname: Some(hostname),
                        },
                    )
                    .await?;
                if common.json {
                    print_json(&result)
                } else {
                    println!("DuckDNS label saved.");
                    Ok(())
                }
            }
        },
    }
}

async fn run_playit(common: CommonArgs, command: PlayitCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        PlayitCommand::Status => {
            let result: PlayitStatusDto = client.get_json("/v1/playit").await?;
            if common.json {
                print_json(&result)
            } else {
                println!("playit enabled: {}", result.playit_enabled);
                println!("running: {}", result.is_running);
                println!("secret configured: {}", result.has_secret_key);
                if let Some(address) = result.java_address {
                    println!("Java address: {address}");
                }
                Ok(())
            }
        }
        PlayitCommand::Start { no_wait } => {
            let result: PlayitActionResultDto = client
                .post_json("/v1/playit/start", &serde_json::json!({}))
                .await?;
            finish_operation(
                &client,
                common.json,
                no_wait,
                result.operation_id,
                "Playit start",
            )
            .await
        }
        PlayitCommand::Stop => {
            let result: PlayitActionResultDto = client
                .post_json("/v1/playit/stop", &serde_json::json!({}))
                .await?;
            if common.json {
                print_json(&result)
            } else {
                println!("{}", result.result);
                Ok(())
            }
        }
    }
}

async fn run_broadcast(common: CommonArgs, command: BroadcastCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        BroadcastCommand::Status => {
            let result: BroadcastStatusDto = client.get_json("/v1/broadcast/status").await?;
            if common.json {
                print_json(&result)
            } else {
                println!("Xbox Broadcast running: {}", result.xbox_broadcast_running);
                Ok(())
            }
        }
        BroadcastCommand::Start { no_wait } => {
            let result: BroadcastSimpleResultDto = client
                .post_json("/v1/broadcast/start", &serde_json::json!({}))
                .await?;
            finish_operation(
                &client,
                common.json,
                no_wait,
                result.operation_id,
                "Xbox Broadcast start",
            )
            .await
        }
        BroadcastCommand::Stop => {
            let result: BroadcastSimpleResultDto = client
                .post_json("/v1/broadcast/stop", &serde_json::json!({}))
                .await?;
            if common.json {
                print_json(&result)
            } else {
                println!("{}", result.result);
                Ok(())
            }
        }
        BroadcastCommand::Restart { no_wait } => {
            let result: BroadcastSimpleResultDto = client
                .post_json("/v1/broadcast/restart", &serde_json::json!({}))
                .await?;
            finish_operation(
                &client,
                common.json,
                no_wait,
                result.operation_id,
                "Xbox Broadcast restart",
            )
            .await
        }
        BroadcastCommand::DownloadJar { no_wait } => {
            let result: BroadcastJarDownloadResultDto = client
                .post_json("/v1/broadcast/download-jar", &serde_json::json!({}))
                .await?;
            finish_operation(
                &client,
                common.json,
                no_wait,
                result.operation_id,
                "Xbox Broadcast JAR download",
            )
            .await
        }
        BroadcastCommand::AuthPrompt => {
            let result: BroadcastAuthPromptDto =
                client.get_json("/v1/broadcast/auth-prompt").await?;
            if common.json {
                print_json(&result)
            } else {
                println!("present: {}", result.is_present);
                if let Some(code) = result.code {
                    println!("code: {code}");
                }
                Ok(())
            }
        }
        BroadcastCommand::DismissAuthPrompt => {
            let result: BroadcastSimpleResultDto = client
                .post_json("/v1/broadcast/auth-prompt/dismiss", &serde_json::json!({}))
                .await?;
            if common.json {
                print_json(&result)
            } else {
                println!("{}", result.result);
                Ok(())
            }
        }
        BroadcastCommand::Autostart { command } => match command {
            BroadcastAutostartCommand::Get => {
                let result: BroadcastAutoStartDto =
                    client.get_json("/v1/broadcast/autostart").await?;
                if common.json {
                    print_json(&result)
                } else {
                    println!("enabled: {}", result.enabled);
                    Ok(())
                }
            }
            BroadcastAutostartCommand::Set { enabled } => {
                let result: BroadcastAutoStartDto = client
                    .post_json(
                        "/v1/broadcast/autostart",
                        &BroadcastAutoStartDto { enabled },
                    )
                    .await?;
                if common.json {
                    print_json(&result)
                } else {
                    println!("enabled: {}", result.enabled);
                    Ok(())
                }
            }
        },
        BroadcastCommand::Credentials {
            email,
            password,
            gamertag,
        } => {
            let _: BroadcastSimpleResultDto = client
                .post_json(
                    "/v1/broadcast/credentials",
                    &BroadcastCredentialsDto {
                        email,
                        password,
                        gamertag,
                    },
                )
                .await?;
            if common.json {
                print_json(&serde_json::json!({"result":"credentials_saved"}))
            } else {
                println!("Broadcast credentials saved.");
                Ok(())
            }
        }
    }
}

async fn run_resource_pack(
    common: CommonArgs,
    command: ResourcePackCommand,
) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        ResourcePackCommand::List => {
            let result: ResourcePacksResponseDto = client.get_json("/v1/resourcepacks").await?;
            if common.json {
                print_json(&result)
            } else {
                if result.packs.is_empty() {
                    println!("No resource packs.");
                }
                for pack in result.packs {
                    println!(
                        "{} {}",
                        if pack.is_active { "*" } else { " " },
                        pack.file_name
                    );
                }
                Ok(())
            }
        }
        ResourcePackCommand::Activate { pack_id, require } => {
            let result: ResourcePackMutationResultDto = client
                .post_json(
                    "/v1/resourcepacks/activate",
                    &ResourcePackActivateRequestDto {
                        pack_id,
                        require: Some(require),
                    },
                )
                .await?;
            if common.json {
                print_json(&result)
            } else {
                println!("{}", result.message);
                Ok(())
            }
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
                        operation_id: None,
                        file_id: None,
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
        WorldCommand::ReplaceActive {
            new_level_name,
            source,
            no_wait,
        } => {
            let staged_upload_id = match source {
                Some(path) => {
                    let bytes = if path.is_dir() {
                        zip_folder_to_bytes(&path)?
                    } else {
                        tokio::fs::read(&path).await.map_err(|err| {
                            CliError::usage(format!("failed to read {}: {err}", path.display()))
                        })?
                    };
                    let begin: StagedUploadBeginResultDto = client
                        .post_json(
                            "/v1/staged-uploads",
                            &StagedUploadBeginRequestDto {
                                purpose: StagedUploadPurposeDto::ActiveWorldReplace,
                                content_type: None,
                                operation_id: None,
                                file_id: None,
                            },
                        )
                        .await?;
                    let _uploaded: StagedUploadCompleteResultDto = client
                        .put_bytes(&begin.upload_path, "application/octet-stream", bytes)
                        .await?;
                    Some(begin.staged_upload_id)
                }
                None => None,
            };
            let body = WorldReplaceActiveRequestDto {
                new_level_name,
                staged_upload_id,
            };
            let result: WorldReplaceActiveResultDto = client
                .post_json("/v1/worlds/replace-active-world", &body)
                .await?;
            finish_operation(
                &client,
                common.json,
                no_wait,
                result.operation_id,
                "replacement",
            )
            .await
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

async fn run_version(common: CommonArgs, command: VersionCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        VersionCommand::List => {
            let response: VersionsResponseDto = client.get_json("/v1/versions").await?;
            if common.json {
                print_json(&response)?;
            } else {
                print_versions(&response);
            }
            Ok(())
        }
        VersionCommand::Create {
            server_type,
            flavor,
        } => {
            let mut path = format!("/v1/versions/create?serverType={server_type}");
            if let Some(flavor) = &flavor {
                path.push_str(&format!("&javaFlavor={flavor}"));
            }
            let response: VersionsResponseDto = client.get_json(&path).await?;
            if common.json {
                print_json(&response)?;
            } else {
                print_versions(&response);
            }
            Ok(())
        }
        VersionCommand::Set {
            version_id,
            loader_version,
            no_wait,
        } => {
            let body = VersionChangeRequestDto {
                version_id,
                loader_version,
            };
            let result: VersionChangeResultDto =
                client.post_json("/v1/components/version", &body).await?;
            if !common.json {
                println!("{}", result.message);
                println!("requires restart: {}", result.requires_restart);
            }
            finish_operation(
                &client,
                common.json,
                no_wait,
                result.operation_id,
                "version change",
            )
            .await
        }
    }
}

fn print_versions(response: &VersionsResponseDto) {
    println!("flavor: {}", response.flavor_name);
    println!("supports versions: {}", response.supports_versions);
    if let Some(current) = &response.current_version {
        println!("current: {current}");
    }
    if let Some(note) = &response.note {
        println!("note: {note}");
    }
    print_runtime(&response.runtime);
    for entry in &response.versions {
        let latest = if entry.is_latest { " (latest)" } else { "" };
        println!("{} {}{}", entry.id, entry.display_label, latest);
    }
}

async fn run_template(common: CommonArgs, command: TemplateCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        TemplateCommand::List => {
            let response: TemplatesResponseDto = client.get_json("/v1/templates").await?;
            if common.json {
                print_json(&response)?;
            } else {
                print_templates(&response);
            }
            Ok(())
        }
        TemplateCommand::Export { no_plugins } => {
            let body = TemplateMutationRequestDto {
                action: "exportServer".to_string(),
                include_plugins: Some(!no_plugins),
                ..Default::default()
            };
            let result: TemplateMutationResultDto =
                client.post_json("/v1/templates", &body).await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
            }
            Ok(())
        }
        TemplateCommand::Create {
            template_id,
            name,
            port,
            world_name,
            difficulty,
            gamemode,
            world_seed,
            accept_eula,
            enable_cross_play,
            cross_play_bedrock_port,
            playit,
        } => {
            let body = TemplateMutationRequestDto {
                action: "createServer".to_string(),
                template_id: Some(template_id),
                name: Some(name),
                port: port.map(i64::from),
                world_name,
                difficulty,
                gamemode,
                world_seed,
                accept_eula: accept_eula.then_some(true),
                enable_cross_play: enable_cross_play.then_some(true),
                cross_play_bedrock_port: cross_play_bedrock_port.map(i64::from),
                enable_playit: playit.then_some(true),
                ..Default::default()
            };
            let result: TemplateMutationResultDto =
                client.post_json("/v1/templates", &body).await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
                if let Some(id) = &result.created_server_id {
                    println!("server id: {id}");
                }
            }
            Ok(())
        }
    }
}

fn print_templates(response: &TemplatesResponseDto) {
    println!("paper templates:");
    for template in &response.paper_templates {
        println!("  {} ({})", template.id, template.display_name);
    }
    println!("plugin templates:");
    for template in &response.plugin_templates {
        println!("  {} ({})", template.id, template.display_name);
    }
}

async fn run_java(common: CommonArgs, command: JavaCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        JavaCommand::List => {
            let response: JavaRuntimesResponseDto = client.get_json("/v1/java-runtimes").await?;
            if common.json {
                print_json(&response)?;
            } else if response.runtimes.is_empty() {
                println!("No Java runtimes detected.");
            } else {
                for runtime in &response.runtimes {
                    let major = runtime
                        .major_version
                        .map(|major| major.to_string())
                        .unwrap_or_else(|| "?".to_string());
                    println!(
                        "{} (java {major}) at {}",
                        runtime.name, runtime.executable_path
                    );
                }
            }
            Ok(())
        }
        JavaCommand::Get => {
            let response: JavaConfigResponseDto =
                client.get_json("/v1/config/java-runtime").await?;
            if common.json {
                print_json(&response)?;
            } else {
                println!(
                    "java path: {}",
                    response.executable_path.as_deref().unwrap_or("(default)")
                );
            }
            Ok(())
        }
        JavaCommand::Set { path } => {
            let body = JavaConfigSetRequestDto {
                executable_path: Some(path),
            };
            let response: JavaConfigResponseDto =
                client.post_json("/v1/config/java-runtime", &body).await?;
            if common.json {
                print_json(&response)?;
            } else {
                println!(
                    "java path: {}",
                    response.executable_path.as_deref().unwrap_or("(default)")
                );
            }
            Ok(())
        }
        JavaCommand::Install { major, no_wait } => {
            let body = JavaRuntimeInstallRequestDto { major };
            let result: JavaRuntimeInstallResultDto =
                client.post_json("/v1/java-runtimes/install", &body).await?;
            if !common.json {
                println!("{}", result.message);
            }
            finish_operation(
                &client,
                common.json,
                no_wait,
                Some(result.operation_id),
                "Java runtime install",
            )
            .await
        }
    }
}

async fn run_doctor(common: CommonArgs, command: Option<DoctorCommand>) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        None => {
            let health: HealthResponseDto = client.get_json("/v1/health").await?;
            let problems: HealthProblemsResponseDto =
                client.get_json("/v1/health/problems").await?;
            if common.json {
                print_json(&serde_json::json!({ "health": health, "problems": problems }))?;
            } else {
                print_health(&health);
                print_health_problems(&problems);
            }
            Ok(())
        }
        Some(DoctorCommand::Repair { problem_id, action }) => {
            let body = HealthRepairRequestDto { problem_id, action };
            let result: HealthRepairResultDto =
                client.post_json("/v1/health/repair", &body).await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
                if let Some(updated) = &result.updated {
                    print_health_problems(updated);
                }
            }
            if let Some(operation_id) = result.operation_id {
                if !common.json {
                    println!("operation id: {operation_id}");
                }
                poll_operation(&client, &operation_id, common.json).await
            } else if result.success {
                Ok(())
            } else {
                Err(CliError::usage(result.message))
            }
        }
    }
}

async fn run_addon(common: CommonArgs, command: AddonCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        AddonCommand::List => {
            let response: AddonsResponseDto = client.get_json("/v1/addons").await?;
            if common.json {
                print_json(&response)?;
            } else {
                print_addons(&response);
            }
            Ok(())
        }
        AddonCommand::Search { query, offset } => {
            let path = format!(
                "/v1/catalog/search?q={}&offset={offset}",
                encode_uri_component(&query)
            );
            let response: CatalogSearchResponseDto = client.get_json(&path).await?;
            if common.json {
                print_json(&response)?;
            } else {
                print_catalog_results(&response);
            }
            Ok(())
        }
        AddonCommand::InstallCatalog {
            project_id,
            slug,
            title,
            no_wait,
        } => {
            let result: CatalogInstallResultDto = client
                .post_json(
                    "/v1/components/install",
                    &CatalogInstallRequestDto {
                        project_id: Some(project_id.clone()),
                        slug,
                        title,
                        staged_upload_id: None,
                    },
                )
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
            }
            if result.operation_id.is_some() {
                finish_operation(
                    &client,
                    common.json,
                    no_wait,
                    result.operation_id,
                    "add-on install",
                )
                .await
            } else {
                Ok(())
            }
        }
        AddonCommand::InstallLocal { path, no_wait } => {
            let staged_upload_id = stage_file_upload(
                &client,
                &path,
                StagedUploadPurposeDto::AddonLocalFile,
                None,
                None,
            )
            .await?;
            let result: CatalogInstallResultDto = client
                .post_json(
                    "/v1/components/install",
                    &CatalogInstallRequestDto {
                        project_id: None,
                        slug: None,
                        title: None,
                        staged_upload_id: Some(staged_upload_id),
                    },
                )
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
            }
            finish_operation(
                &client,
                common.json,
                no_wait,
                result.operation_id,
                "local add-on install",
            )
            .await
        }
        AddonCommand::Update { jar_stem, no_wait } => {
            let result: AddonUpdateResultDto = client
                .post_json(
                    "/v1/components/update",
                    &ComponentUpdateRequestDto {
                        component: None,
                        jar_stem: Some(jar_stem.clone()),
                        update_all: None,
                        enabled: None,
                        link_project_id: None,
                        source_url: None,
                        remove_source: None,
                    },
                )
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.result);
            }
            if result.operation_id.is_some() {
                finish_operation(
                    &client,
                    common.json,
                    no_wait,
                    result.operation_id,
                    "add-on update",
                )
                .await
            } else {
                Ok(())
            }
        }
        AddonCommand::UpdateAll { no_wait } => {
            let result: AddonUpdateResultDto = client
                .post_json(
                    "/v1/components/update",
                    &ComponentUpdateRequestDto {
                        component: None,
                        jar_stem: None,
                        update_all: Some(true),
                        enabled: None,
                        link_project_id: None,
                        source_url: None,
                        remove_source: None,
                    },
                )
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.result);
            }
            if result.operation_id.is_some() {
                finish_operation(
                    &client,
                    common.json,
                    no_wait,
                    result.operation_id,
                    "add-on updates",
                )
                .await
            } else {
                Ok(())
            }
        }
        AddonCommand::Enable { jar_stem } => {
            let result: AddonUpdateResultDto = client
                .post_json(
                    "/v1/components/update",
                    &ComponentUpdateRequestDto {
                        component: None,
                        jar_stem: Some(jar_stem),
                        update_all: None,
                        enabled: Some(true),
                        link_project_id: None,
                        source_url: None,
                        remove_source: None,
                    },
                )
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.result);
            }
            Ok(())
        }
        AddonCommand::Disable { jar_stem } => {
            let result: AddonUpdateResultDto = client
                .post_json(
                    "/v1/components/update",
                    &ComponentUpdateRequestDto {
                        component: None,
                        jar_stem: Some(jar_stem),
                        update_all: None,
                        enabled: Some(false),
                        link_project_id: None,
                        source_url: None,
                        remove_source: None,
                    },
                )
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.result);
            }
            Ok(())
        }
        AddonCommand::Remove { jar_stem } => {
            let result: AddonRemoveResultDto = client
                .post_json("/v1/components/remove", &AddonRemoveRequestDto { jar_stem })
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
            }
            Ok(())
        }
        AddonCommand::Link {
            jar_stem,
            project_id,
        } => {
            let result: AddonUpdateResultDto = client
                .post_json(
                    "/v1/components/update",
                    &ComponentUpdateRequestDto {
                        component: None,
                        jar_stem: Some(jar_stem),
                        update_all: None,
                        enabled: None,
                        link_project_id: Some(project_id),
                        source_url: None,
                        remove_source: None,
                    },
                )
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.result);
            }
            Ok(())
        }
        AddonCommand::SetSource { jar_stem, url } => {
            let result: AddonUpdateResultDto = client
                .post_json(
                    "/v1/components/update",
                    &ComponentUpdateRequestDto {
                        component: None,
                        jar_stem: Some(jar_stem),
                        update_all: None,
                        enabled: None,
                        link_project_id: None,
                        source_url: Some(url),
                        remove_source: None,
                    },
                )
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.result);
            }
            Ok(())
        }
        AddonCommand::RemoveSource { jar_stem } => {
            let result: AddonUpdateResultDto = client
                .post_json(
                    "/v1/components/update",
                    &ComponentUpdateRequestDto {
                        component: None,
                        jar_stem: Some(jar_stem),
                        update_all: None,
                        enabled: None,
                        link_project_id: None,
                        source_url: None,
                        remove_source: Some(true),
                    },
                )
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.result);
            }
            Ok(())
        }
        AddonCommand::Export {
            selected_ids,
            output,
        } => {
            let path = if selected_ids.is_empty() {
                "/v1/components/client-export".to_string()
            } else {
                format!(
                    "/v1/components/client-export?selected={}",
                    encode_uri_component(&selected_ids.join(","))
                )
            };
            let result: ClientExportResponseDto = client.get_json(&path).await?;
            if common.json {
                print_json(&result)?;
                return Ok(());
            }
            if let Some(text) = &result.share_text {
                println!("{text}");
                return Ok(());
            }
            if let Some(staged_download_id) = &result.staged_download_id {
                let output = output.ok_or_else(|| {
                    CliError::usage("client export returned a zip; pass --output <path> to save it")
                })?;
                let bytes = client
                    .get_raw_bytes(&format!("/v1/staged-downloads/{staged_download_id}"))
                    .await?;
                tokio::fs::write(&output, &bytes).await.map_err(|err| {
                    CliError::internal(format!("failed to write {}: {err}", output.display()))
                })?;
                println!("exported {} bytes to {}", bytes.len(), output.display());
                return Ok(());
            }
            if let Some(note) = &result.note {
                println!("note: {note}");
            }
            Ok(())
        }
    }
}

async fn run_modpack(common: CommonArgs, command: ModpackCommand) -> Result<(), CliError> {
    let client = RemoteClient::from_common(&common)?;
    match command {
        ModpackCommand::Inspect { path } => {
            let staged_upload_id = stage_file_upload(
                &client,
                &path,
                StagedUploadPurposeDto::ModpackArchive,
                None,
                None,
            )
            .await?;
            let result: ModpackInspectionResultDto = client
                .post_json(
                    "/v1/modpacks/inspect",
                    &ModpackInspectionRequestDto { staged_upload_id },
                )
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                print_modpack_inspection(&result);
            }
            Ok(())
        }
        ModpackCommand::Import { path, no_wait } => {
            let result = import_modpack_command(&client, &path, "import").await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
            }
            finish_operation(
                &client,
                common.json,
                no_wait,
                Some(result.operation_id),
                "modpack import",
            )
            .await
        }
        ModpackCommand::Replace { path, no_wait } => {
            let result = import_modpack_command(&client, &path, "replace").await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
            }
            finish_operation(
                &client,
                common.json,
                no_wait,
                Some(result.operation_id),
                "modpack replacement",
            )
            .await
        }
        ModpackCommand::ManualFile {
            operation_id,
            file_id,
            path,
        } => {
            let staged_upload_id = stage_file_upload(
                &client,
                &path,
                StagedUploadPurposeDto::CurseforgeManualFile,
                Some(operation_id.clone()),
                Some(file_id.clone()),
            )
            .await?;
            let result: ModpackManualFileResultDto = client
                .post_json(
                    &format!("/v1/modpacks/{operation_id}/manual-file"),
                    &ModpackManualFileRequestDto {
                        file_id,
                        staged_upload_id,
                    },
                )
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
            }
            Ok(())
        }
    }
}

async fn import_modpack_command(
    client: &RemoteClient,
    path: &Path,
    action: &str,
) -> Result<ModpackImportResultDto, CliError> {
    let staged_upload_id = stage_file_upload(
        client,
        path,
        StagedUploadPurposeDto::ModpackArchive,
        None,
        None,
    )
    .await?;
    client
        .post_json(
            "/v1/modpacks/import",
            &ModpackImportRequestDto {
                staged_upload_id,
                action: action.to_string(),
            },
        )
        .await
}

fn print_health(response: &HealthResponseDto) {
    println!(
        "{} \"{}\" — overall: {}",
        response.server_type, response.server_name, response.overall_severity
    );
    for card in &response.cards {
        println!(
            "  [{}] {}: {}",
            card.severity,
            card.title,
            card.detail.as_deref().unwrap_or("")
        );
    }
    if let Some(note) = &response.note {
        println!("note: {note}");
    }
}

fn print_health_problems(response: &HealthProblemsResponseDto) {
    if response.problems.is_empty() {
        println!("No startup problems.");
        return;
    }
    for problem in &response.problems {
        println!(
            "{} [{}] {} — {}",
            problem.id, problem.kind_title, problem.offender_name, problem.raw_excerpt
        );
        if !problem.available_actions.is_empty() {
            println!("  actions: {}", problem.available_actions.join(", "));
        }
    }
}

fn print_addons(response: &AddonsResponseDto) {
    if let Some(note) = &response.note {
        println!("note: {note}");
    }
    if response.addons.is_empty() {
        println!("No add-ons.");
        return;
    }
    for addon in &response.addons {
        let enabled = if addon.is_enabled {
            "enabled"
        } else {
            "disabled"
        };
        println!("{} [{}] {}", addon.jar_stem, enabled, addon.bucket);
        if let Some(project_id) = &addon.project_id {
            println!("  project: {project_id}");
        }
        if let Some(version) = &addon.available_version {
            println!("  available: {version}");
        }
    }
}

fn print_catalog_results(response: &CatalogSearchResponseDto) {
    if let Some(note) = &response.note {
        println!("note: {note}");
    }
    if response.results.is_empty() {
        println!("No catalog results.");
        return;
    }
    for item in &response.results {
        println!("{} {}", item.project_id, item.title);
        println!("  slug: {}", item.slug);
    }
}

fn print_modpack_inspection(result: &ModpackInspectionResultDto) {
    println!("format: {}", result.format);
    if let Some(name) = &result.pack_name {
        println!("pack: {name}");
    }
    if let Some(version) = &result.pack_version {
        println!("version: {version}");
    }
    println!("files: {}", result.file_count);
    if !result.manual_files.is_empty() {
        println!("manual files:");
        for file in &result.manual_files {
            println!("  {} {}", file.file_id, file.file_name);
        }
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
    print_runtime(&status.runtime);
}

fn print_simple_result(prefix: &str, result: &SimpleResultDto) {
    println!("{prefix}");
    if let Some(active_server_id) = &result.active_server_id {
        println!("active server id: {active_server_id}");
    }
    if let Some(operation_id) = &result.operation_id {
        println!("operation id: {operation_id}");
    }
    print_runtime(&result.runtime);
}

fn print_settings(settings: &SettingsResponseDto) {
    println!("server: {}", settings.server_name);
    println!("editable: {}", settings.editable);
    if let Some(note) = &settings.note {
        println!("note: {note}");
    }
    print_runtime(&settings.runtime);
    for section in &settings.sections {
        println!("[{}]", section.title);
        for field in &section.fields {
            println!("  {} = {}", field.key, field.value);
        }
    }
}

fn print_settings_update(result: &SettingsUpdateResultDto) {
    println!("{}", result.message);
    print_runtime(&result.runtime);
    if !result.applied_keys.is_empty() {
        println!("applied: {}", result.applied_keys.join(", "));
    }
    if let Some(rejected) = &result.rejected {
        for rejection in rejected {
            println!("rejected {}: {}", rejection.key, rejection.reason);
        }
    }
}

fn print_runtime(runtime: &Option<BedrockRuntimeStateDto>) {
    if let Some(runtime) = runtime {
        println!("bedrock runtime: {}", runtime.state);
        if let Some(backend) = runtime.backend {
            println!("bedrock backend: {backend:?}");
        }
        if let Some(reason) = &runtime.reason_code {
            println!("bedrock reason: {reason}");
        }
    }
}

fn print_runtime_value(runtime: Option<&serde_json::Value>) {
    let Some(runtime) = runtime.filter(|value| !value.is_null()) else {
        return;
    };
    println!(
        "bedrock runtime: {}",
        runtime["state"].as_str().unwrap_or("unknown")
    );
    if let Some(backend) = runtime["backend"].as_str() {
        println!("bedrock backend: {backend}");
    }
    if let Some(reason) = runtime["reasonCode"].as_str() {
        println!("bedrock reason: {reason}");
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
