//! CLI commands. API tasks use the same HTTP routes as the desktop client,
//! with authorization obtained from this host's operating system.

pub mod pairing;
pub mod service;
pub(crate) mod session;
pub(crate) mod transport;
pub mod update;

use self::transport::SharedClient as ApiClient;

use std::collections::HashMap;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::http::StatusCode;
use clap::{Args, Subcommand};
use futures_util::StreamExt;
use msc_api::dto::{
    ActiveServerRequestDto, AddonRemoveRequestDto, AddonRemoveResultDto, AddonUpdateResultDto,
    AddonsResponseDto, BackupConfigResponseDto, BackupConfigUpdateRequestDto,
    BackupConfigUpdateResultDto, BackupDeleteRequestDto, BackupNowResultDto,
    BackupRestoreRequestDto, BackupRestoreResultDto, BackupsResponseDto, BedrockRuntimeStateDto,
    BroadcastAuthPromptDto, BroadcastAutoStartDto, BroadcastCredentialsDto,
    BroadcastJarDownloadResultDto, BroadcastSimpleResultDto, BroadcastStatusDto, CapabilitiesDto,
    CatalogInstallRequestDto, CatalogInstallResultDto, CatalogSearchResponseDto,
    ClientExportResponseDto, CommandResultDto, ComponentUpdateRequestDto, ConnectivityResponseDto,
    DuckDnsStatusResponseDto, DuckDnsUpdateRequestDto, ErrorDto, HealthProblemsResponseDto,
    HealthRepairRequestDto, HealthRepairResultDto, HealthResponseDto, HostResetAcceptedDto,
    JavaConfigResponseDto, JavaConfigSetRequestDto, JavaRuntimeInstallRequestDto,
    JavaRuntimeInstallResultDto, JavaRuntimesResponseDto, ModpackImportRequestDto,
    ModpackImportResultDto, ModpackInspectionRequestDto, ModpackInspectionResultDto,
    ModpackManualFileRequestDto, ModpackManualFileResultDto, OperationDto, OperationStateDto,
    PerformanceMetricNumberDto, PerformanceSnapshotDto, PlayitActionResultDto, PlayitStatusDto,
    RemoteApiStatus, ResourcePackActivateRequestDto, ResourcePackMutationResultDto,
    ResourcePacksResponseDto, ServerCreateRequestDto, ServerCreateResultDto,
    ServerDeleteRequestDto, ServerDeleteResultDto, ServerDirectorySizeResponseDto, ServerDto,
    ServerEulaRequestDto, ServerEulaResultDto, ServerImportRequestDto, ServerImportResultDto,
    ServerImportScanResponseDto, ServerNotesRequestDto, ServerNotesResultDto,
    ServerRenameRequestDto, ServerRenameResultDto, ServerTransferExportResultDto,
    SettingsResponseDto, SettingsUpdateResultDto, SimpleResultDto, StagedUploadBeginRequestDto,
    StagedUploadBeginResultDto, StagedUploadCompleteResultDto, StagedUploadPurposeDto,
    VersionChangeRequestDto, VersionChangeResultDto, VersionsResponseDto, WorldActivateResultDto,
    WorldConvertRequestDto, WorldConvertResultDto, WorldCreateRequestDto, WorldDeleteRequestDto,
    WorldDuplicateRequestDto, WorldExportRequestDto, WorldExportResultDto, WorldImportRequestDto,
    WorldMutationResultDto, WorldRenameRequestDto, WorldReplaceActiveRequestDto,
    WorldReplaceActiveResultDto, WorldReplaceRequestDto, WorldSlotDto, WorldSlotsResponseDto,
};
use msc_infrastructure::archive::create_zip_from_folders;
use msc_infrastructure::console_buffer::ConsoleLine;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationTarget {
    Command,
    Usage,
}

/// Selects the root invocation. Bare invocations show ordinary CLI usage;
/// named commands continue through the scriptable HTTP client.
pub fn select_invocation(has_named_command: bool) -> InvocationTarget {
    if has_named_command {
        InvocationTarget::Command
    } else {
        InvocationTarget::Usage
    }
}

#[derive(Debug, Clone, Args)]
pub struct CommonArgs {
    /// Emit JSON instead of human-readable output.
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Start the installed local agent service.
    Start { target: service::AgentTarget },
    /// Stop the installed local agent service.
    Stop { target: service::AgentTarget },
    /// Enable the local agent service at boot.
    Enable { target: service::AgentTarget },
    /// Disable the local agent service at boot.
    Disable { target: service::AgentTarget },
    /// Internal entry point registered with Windows Service Control Manager.
    #[cfg(target_os = "windows")]
    #[command(name = "service-run", hide = true)]
    ServiceRun {
        #[arg(long)]
        service_name: String,
        #[arg(long, default_value = "127.0.0.1:48001")]
        bind: std::net::SocketAddr,
    },
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
    /// Fixed-purpose elevated helper for the desktop's Linux systemd unit.
    #[cfg(target_os = "linux")]
    #[command(name = "desktop-service-helper", hide = true)]
    DesktopServiceHelper {
        #[command(subcommand)]
        command: DesktopServiceHelperCommand,
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
    Status {
        target: Option<service::AgentTarget>,
    },
    /// Show the latest performance and resource measurements.
    Metrics { server: Option<String> },
    /// Read recent player join and leave events.
    Sessions { server: Option<String> },
    /// Clear the active server's join/leave history.
    ClearSessions,
    /// Show the agent's host and capabilities.
    Capabilities,
    /// Inspect this authorization or administer delegated API tokens.
    Access {
        #[command(subcommand)]
        command: AccessCommand,
    },
    /// Configure host-wide settings and helpers through the authenticated API.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
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
    /// Inspect components installed in the active server.
    Components,
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
    /// Check for or install a signed update for this local MSC installation.
    /// These commands never use the remote management API.
    Update {
        #[command(subcommand)]
        command: update::UpdateCommand,
    },
    /// Create a local one-use code for remote recovery pairing.
    Pairing {
        #[command(subcommand)]
        command: pairing::PairingCommand,
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
    /// Inspect players and perform supported moderation actions.
    Player {
        #[command(subcommand)]
        command: PlayerCommand,
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
    /// Inspect or cancel a long-running agent operation.
    Operation {
        #[command(subcommand)]
        command: OperationCommand,
    },
    /// Reset host-wide MSC configuration or all managed data.
    HostReset {
        #[command(subcommand)]
        command: HostResetCommand,
    },
    /// Browse or read permitted files in the active server directory.
    File {
        #[command(subcommand)]
        command: FileCommand,
    },
    /// Search and read the embedded handbook and setup guides.
    Help {
        #[command(subcommand)]
        command: HelpCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum OperationCommand {
    /// Show the current state and result of an operation by ID.
    Show { operation_id: String },
    /// Ask the operation's owner to stop it at a safe point.
    Cancel { operation_id: String },
}

#[derive(Debug, Clone, Subcommand)]
pub enum HostResetCommand {
    /// Clear host configuration while preserving managed server data.
    Configuration {
        /// Must be exactly `RESET AGENT`.
        #[arg(long)]
        confirm: String,
    },
    /// Clear host configuration and managed server data.
    Everything {
        /// Must be exactly `RESET AGENT`.
        #[arg(long)]
        confirm: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum FileCommand {
    /// List a permitted directory in the active server tree.
    Browse { path: Option<String> },
    /// Read a permitted text file in the active server tree.
    Read { path: String },
}

#[derive(Debug, Clone, Subcommand)]
pub enum HelpCommand {
    /// List handbook topics and their IDs.
    Catalog,
    /// Read a handbook topic by its help ID.
    Topic { help_id: String },
    /// Read first-launch onboarding guidance.
    Onboarding,
    /// List router-guide topics and troubleshooting entries.
    RouterCatalog,
    /// Search router guides by symptom or description.
    RouterSearch { query: String },
    /// Read a resolved router guide by ID.
    RouterGuide { guide_id: String },
}

#[derive(Debug, Clone, Subcommand)]
pub enum AccessCommand {
    /// Show the role and permissions used by this local CLI invocation.
    Me,
    /// List delegated named tokens. Admin permission required.
    List,
    /// Create a named token; its secret is shown once in a terminal. Admin permission required.
    Create {
        label: String,
        #[arg(long, default_value = "named")]
        role: String,
        #[arg(long = "permission", value_delimiter = ',')]
        permissions: Vec<String>,
        #[arg(long = "expires-in-days")]
        expires_in_days: Option<i64>,
    },
    /// Update a delegated token. Negative expiry clears its expiry; omit to leave it unchanged.
    Update {
        user_id: String,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        role: Option<String>,
        #[arg(long = "permission", value_delimiter = ',')]
        permissions: Vec<String>,
        #[arg(long, conflicts_with = "permissions")]
        clear_permissions: bool,
        #[arg(long = "expires-in-days")]
        expires_in_days: Option<i64>,
    },
    /// Revoke a delegated token immediately. Admin permission required.
    Revoke { user_id: String },
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
    Setup {
        email: String,
        #[arg(long)]
        password_stdin: bool,
    },
    Reset,
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
        gamertag: String,
        #[arg(long)]
        password_stdin: bool,
    },
    ClearCredentials {
        #[arg(long)]
        confirm: bool,
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
    SetUrl {
        url: String,
        #[arg(long)]
        sha1: Option<String>,
        #[arg(long)]
        require: bool,
    },
    ClearUrl,
    Remove {
        pack_id: String,
    },
    Activate {
        /// Existing approved ZIP name; omit to clear the active pack.
        pack_id: Option<String>,
        #[arg(long)]
        require: bool,
    },
    Toggle {
        pack_id: String,
        enabled: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConfigCommand {
    /// Read or change RAM allocation for the active server.
    Ram {
        #[command(subcommand)]
        command: RamCommand,
    },
    HostSetup,
    CompleteHostSetup,
    ServersRoot,
    SetServersRoot {
        path: String,
    },
    Geyser,
    SetGeyser {
        #[arg(long)]
        address: Option<String>,
        #[arg(long)]
        port: Option<i64>,
    },
    CurseForge,
    SetCurseForgeKey {
        #[arg(long)]
        key_stdin: bool,
    },
    Watchdog,
    WatchdogEnable,
    WatchdogDisable,
}

#[derive(Debug, Clone, Subcommand)]
pub enum RamCommand {
    Get,
    Set {
        #[arg(long)]
        min_gb: Option<f64>,
        #[arg(long)]
        max_gb: Option<f64>,
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

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Subcommand)]
pub enum DesktopServiceHelperCommand {
    /// Install or repair the one MSC desktop service.
    Install {
        #[arg(long)]
        binary_path: PathBuf,
        #[arg(long)]
        working_directory: PathBuf,
        #[arg(long)]
        log_path: PathBuf,
        #[arg(long)]
        run_user: String,
        #[arg(long)]
        expected_port: u16,
        #[arg(long = "arg")]
        arguments: Vec<String>,
        #[arg(long = "env", value_name = "KEY=VALUE")]
        environment: Vec<String>,
    },
    /// Remove the one MSC desktop service.
    Uninstall,
}

// `Import` is far larger than `Start`/`Stop`/`Restart` because it alone
// carries every raw-import override and transfer flag — clippy would have
// us box fields to shrink the enum, but this is a CLI arg enum matched
// once per invocation, not a hot path; the added indirection wouldn't pay
// for itself.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Subcommand)]
pub enum ServerCommand {
    /// List every registered server and mark the current active server.
    List,
    /// Show one registered server by exact name or id.
    Show {
        server: String,
    },
    /// Select a server as the active context for following commands.
    Use {
        server: String,
    },
    /// Measure the managed directory for one server.
    Size {
        server: String,
    },
    /// Read or replace the notes attached to one server.
    Notes {
        server: String,
        text: Option<String>,
    },
    BedrockTransport {
        server: String,
        transport: String,
    },
    Playit {
        server: String,
        enabled: bool,
    },
    XboxBroadcast {
        server: String,
        enabled: bool,
    },
    /// Export all registered servers to one MSC transfer archive.
    Export {
        /// Local file to write the transfer archive to.
        #[arg(long, default_value = "MinecraftServers.msctransfer")]
        output: PathBuf,
    },
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
    Start {
        server: Option<String>,
    },
    /// Stop the selected server, or the current active server if omitted.
    Stop {
        server: Option<String>,
    },
    /// Restart the selected server, or the current active server if omitted.
    Restart {
        server: Option<String>,
    },
    /// Create a new Java server. Long-running for Forge/NeoForge (a real
    /// supervised installer run); always operation-backed.
    Create(ServerCreateArgs),
    /// Delete a server. Refuses a running server.
    Delete {
        server: String,
    },
    /// Rename a server's display name (not its on-disk folder).
    Rename {
        server: String,
        name: String,
    },
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
    /// Acknowledgement token returned by the agent for a safety-sensitive
    /// world choice, for example `bedrock_achievements`.
    #[arg(long)]
    confirm: Option<String>,
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
    /// Inspect a Modrinth project and its versions before choosing one to install.
    Inspect { project_id: String },
    /// Install one add-on from the active server's filtered Modrinth catalog.
    InstallCatalog {
        project_id: String,
        #[arg(long = "version-id")]
        version_id: String,
        /// Confirm the reviewed project/version and its required dependencies.
        #[arg(long, required = true)]
        confirm: bool,
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
        #[arg(long, required = true)]
        target_server: String,
        #[arg(long, required = true)]
        confirm: bool,
        #[arg(long)]
        no_wait: bool,
    },
    /// Explicitly replace the active server's current pack with a new local modpack.
    Replace {
        path: PathBuf,
        #[arg(long, required = true)]
        target_server: String,
        #[arg(long, required = true)]
        confirm: bool,
        #[arg(long)]
        no_wait: bool,
    },
    /// Complete one pending author-blocked CurseForge file upload.
    ManualFile {
        operation_id: String,
        file_id: String,
        path: PathBuf,
    },
    /// Cancel a modpack operation that is still waiting on downloads or manual files.
    Cancel { operation_id: String },
}

#[derive(Debug, Clone, Args)]
pub struct CommandArgs {
    /// Select a server by id or display name before sending the command.
    #[arg(long)]
    server: Option<String>,

    /// The exact console command to send.
    text: String,

    /// Acknowledgement token returned by the agent for a safety-sensitive
    /// command, for example `bedrock_achievements`.
    #[arg(long)]
    confirm: Option<String>,
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
    /// Follow live console output until Ctrl-C, starting with bounded history.
    Follow {
        /// Select a server by id or display name before following its console.
        #[arg(long)]
        server: Option<String>,
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

        /// Acknowledgement token returned by the agent for a safety-sensitive
        /// setting change.
        #[arg(long)]
        confirm: Option<String>,
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
pub enum PlayerCommand {
    /// List players currently online.
    Online {
        #[arg(long)]
        server: Option<String>,
    },
    /// List known player profiles without reading saved stats or inventory.
    Profiles {
        #[arg(long)]
        server: Option<String>,
    },
    /// Inspect, duplicate, delete, or migrate saved player data.
    Data {
        #[command(subcommand)]
        command: PlayerDataCommand,
    },
    SkinOverride {
        profile_id: String,
        #[arg(long)]
        lookup_identifier: Option<String>,
        #[arg(long)]
        server: Option<String>,
    },
    Hide {
        profile_id: String,
        #[arg(long, action = clap::ArgAction::Set)]
        hidden: bool,
        #[arg(long)]
        server: Option<String>,
    },
    Identify {
        profile_id: String,
        gamertag: String,
        #[arg(long)]
        server: Option<String>,
    },
    /// Send a private message to a player.
    Message {
        player: String,
        message: String,
        #[arg(long)]
        server: Option<String>,
    },
    /// Disconnect a player, optionally with a reason.
    Kick {
        player: String,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        server: Option<String>,
    },
    /// Ban a player, optionally with a reason.
    Ban {
        player: String,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        server: Option<String>,
    },
    /// Remove a player's ban.
    Pardon {
        player: String,
        #[arg(long)]
        server: Option<String>,
    },
    /// Grant operator permission.
    Op {
        player: String,
        #[arg(long)]
        server: Option<String>,
    },
    /// Remove operator permission.
    Deop {
        player: String,
        #[arg(long)]
        server: Option<String>,
    },
    /// Change the Java whitelist.
    Whitelist {
        #[command(subcommand)]
        command: PlayerWhitelistCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum PlayerDataCommand {
    /// Show available saved statistics and inventory.
    Show {
        player: String,
        #[arg(long)]
        server: Option<String>,
    },
    /// Duplicate saved data under a new player identifier. Bedrock servers must be stopped.
    Duplicate {
        player: String,
        #[arg(long)]
        server: Option<String>,
    },
    /// Permanently delete saved player data. Requires --confirm. Bedrock servers must be stopped.
    Delete {
        player: String,
        #[arg(long)]
        server: Option<String>,
        #[arg(long)]
        confirm: bool,
    },
    /// Move Java data to the account's offline UUID. Requires --confirm; Bedrock has no UUID migration.
    MigrateOffline {
        player: String,
        #[arg(long)]
        server: Option<String>,
        #[arg(long)]
        confirm: bool,
    },
    /// Move Java data to a custom UUID. Requires --confirm; Bedrock has no UUID migration.
    Migrate {
        player: String,
        target_uuid: String,
        #[arg(long)]
        server: Option<String>,
        #[arg(long)]
        confirm: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum PlayerWhitelistCommand {
    Add {
        player: String,
        #[arg(long)]
        server: Option<String>,
    },
    Remove {
        player: String,
        #[arg(long)]
        server: Option<String>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum WorldCommand {
    /// List world slots for the active server.
    List,
    /// Save the active live world into its current slot.
    SaveCurrent,
    /// Show one slot's profile and identity fields.
    Profile { slot_id: String },
    /// Repair the active Bedrock world while the server is stopped.
    Repair {
        slot_id: String,
        #[arg(long)]
        no_wait: bool,
    },
    /// Rename the active world's live folder, respecting the server's stopped/running rules.
    RenameActive { name: String },
    /// Show Chunker's installed state and supported conversion formats.
    ConvertFormats,
    /// Download or update Chunker for world conversion.
    AcquireChunker {
        #[arg(long)]
        no_wait: bool,
    },
    /// Inspect and manage packs attached to a specific world slot.
    Pack {
        #[command(subcommand)]
        command: WorldPackCommand,
    },
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
        /// Acknowledgement token returned by the agent for activating a
        /// safety-sensitive world profile.
        #[arg(long)]
        confirm: Option<String>,
        /// Print the operation id and return immediately instead of
        /// waiting for activation to finish.
        #[arg(long)]
        no_wait: bool,
    },
    /// Update one or more world-profile fields as `key=value` pairs.
    ProfileSet {
        slot_id: String,
        /// One or more profile changes, for example
        /// `gameplay.default-game-mode=creative`.
        #[arg(long = "change", required = true)]
        changes: Vec<String>,
        /// Acknowledgement token returned by the agent for a safety-sensitive
        /// profile change.
        #[arg(long)]
        confirm: Option<String>,
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
pub enum WorldPackCommand {
    /// List the packs recorded on one world slot.
    List { slot_id: String },
    /// Search the provider catalog for the selected world's edition.
    Search { slot_id: String, query: String },
    /// Inspect compatible versions, files, and dependencies before installation.
    Inspect { slot_id: String, project_id: String },
    /// Install a reviewed provider version or file into this exact world slot.
    Install {
        slot_id: String,
        project_id: String,
        version_or_file_id: String,
        #[arg(long, required = true)]
        confirm: bool,
        #[arg(long)]
        no_wait: bool,
    },
    /// Enable one installed Bedrock behavior pack.
    Enable { slot_id: String, pack_id: String },
    /// Disable one installed Bedrock behavior pack.
    Disable { slot_id: String, pack_id: String },
    /// Remove one installed Bedrock behavior pack from the world.
    Remove { slot_id: String, pack_id: String },
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

#[derive(Debug)]
pub struct CliError {
    exit_code: u8,
    message: String,
    json_message: Option<String>,
}

impl CliError {
    pub(crate) fn usage(message: impl Into<String>) -> Self {
        Self {
            exit_code: 2,
            message: message.into(),
            json_message: None,
        }
    }

    fn api(status: StatusCode, body: &[u8]) -> Self {
        let text = String::from_utf8_lossy(body);
        let message = match serde_json::from_slice::<ErrorDto>(body) {
            Ok(error) => {
                let confirmation = error
                    .details
                    .as_ref()
                    .and_then(|details| details.get("confirmation"))
                    .and_then(|confirmation| confirmation.get("acknowledgement"))
                    .and_then(serde_json::Value::as_str);
                match confirmation {
                    Some(token) => format!(
                        "API {} {}: {} Re-run with --confirm {token}.",
                        status.as_u16(),
                        error.code,
                        error.message
                    ),
                    None => format!("API {} {}: {}", status.as_u16(), error.code, error.message),
                }
            }
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
        #[cfg(target_os = "windows")]
        Command::ServiceRun { .. } => Err(CliError::internal("service-run is handled in main")),
        #[cfg(target_os = "linux")]
        Command::CredentialHelper { .. } => {
            Err(CliError::internal("credential-helper is handled in main"))
        }
        #[cfg(target_os = "linux")]
        Command::DesktopServiceHelper { .. } => Err(CliError::internal(
            "desktop-service-helper is handled in main",
        )),
        Command::Status {
            target: Some(target),
        } => service::run_agent(common, target, service::AgentAction::Status),
        Command::Status { target: None } => {
            let client = ApiClient::connect_local().await?;
            let status: RemoteApiStatus = client.get_json("/v1/status").await?;
            if common.json {
                print_json(&status)?;
            } else {
                print_status(&status);
            }
            Ok(())
        }
        Command::Metrics { server } => run_metrics(common, server).await,
        Command::Sessions { server } => run_sessions(common, server).await,
        Command::Capabilities => run_capabilities(common).await,
        Command::Access { command } => run_access(common, command).await,
        Command::Config { command } => run_config(common, command).await,
        Command::Components => run_components(common).await,
        Command::ClearSessions => {
            let client = ApiClient::connect_local().await?;
            let value: serde_json::Value = client
                .post_json("/v1/session-log/clear", &serde_json::json!({}))
                .await?;
            output_value(common.json, &value)
        }
        Command::Network { command } => run_network(common, command).await,
        Command::Playit { command } => run_playit(common, command).await,
        Command::Broadcast { command } => run_broadcast(common, command).await,
        Command::ResourcePack { command } => run_resource_pack(common, command).await,
        Command::Service { command } => service::run(common, command).await,
        Command::Start { target } => {
            service::run_agent(common, target, service::AgentAction::Start)
        }
        Command::Stop { target } => service::run_agent(common, target, service::AgentAction::Stop),
        Command::Enable { target } => {
            service::run_agent(common, target, service::AgentAction::Enable)
        }
        Command::Disable { target } => {
            service::run_agent(common, target, service::AgentAction::Disable)
        }
        Command::Update { command } => update::run(common, command),
        Command::Pairing { command } => pairing::run(common, command),
        Command::Server { command } => run_server(common, command).await,
        Command::Send(args) => run_command(common, args).await,
        Command::Console { command } => run_console(common, command).await,
        Command::Settings { command } => run_settings(common, command).await,
        Command::Bedrock { command } => run_bedrock(common, command).await,
        Command::Player { command } => run_player(common, command).await,
        Command::World { command } => run_world(common, command).await,
        Command::Backup { command } => run_backup(common, command).await,
        Command::Version { command } => run_version(common, command).await,
        Command::Java { command } => run_java(common, command).await,
        Command::Doctor { command } => run_doctor(common, command).await,
        Command::Addon { command } => run_addon(common, command).await,
        Command::Modpack { command } => run_modpack(common, command).await,
        Command::Operation { command } => run_operation(common, command).await,
        Command::HostReset { command } => run_host_reset(common, command).await,
        Command::File { command } => run_file(common, command).await,
        Command::Help { command } => run_help(common, command).await,
    }
}

async fn run_operation(common: CommonArgs, command: OperationCommand) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
    let operation = match command {
        OperationCommand::Show { operation_id } => {
            client
                .get_json::<OperationDto>(&format!("/v1/operations/{operation_id}"))
                .await?
        }
        OperationCommand::Cancel { operation_id } => {
            client
                .post_json::<_, OperationDto>(
                    &format!("/v1/operations/{operation_id}/cancel"),
                    &serde_json::json!({}),
                )
                .await?
        }
    };
    if common.json {
        print_json(&operation)?;
    } else {
        println!("operation: {}", operation.id);
        println!("type: {}", operation.r#type);
        println!("state: {:?}", operation.state);
        if let Some(target) = operation.target {
            println!("target: {target}");
        }
        if let Some(status) = operation.status_line {
            println!("status: {status}");
        }
        if let Some(progress) = operation.progress {
            println!("progress: {}/{}", progress.current, progress.total);
        }
        if let Some(result) = operation.result {
            println!("result: {result}");
        }
        if let Some(error) = operation.error {
            println!("error: {} — {}", error.code, error.message);
        }
    }
    Ok(())
}

async fn run_host_reset(common: CommonArgs, command: HostResetCommand) -> Result<(), CliError> {
    let (mode, confirmation) = match command {
        HostResetCommand::Configuration { confirm } => ("configuration", confirm),
        HostResetCommand::Everything { confirm } => ("everything", confirm),
    };
    if confirmation != "RESET AGENT" {
        return Err(CliError::internal(
            "confirmation must exactly match RESET AGENT",
        ));
    }
    let client = ApiClient::connect_local().await?;
    let accepted: HostResetAcceptedDto = client
        .post_json(
            "/v1/host/reset",
            &serde_json::json!({ "mode": mode, "confirmation": confirmation }),
        )
        .await?;
    if common.json {
        print_json(&accepted)?;
    } else {
        println!("{}", accepted.message);
        println!("operation id: {}", accepted.operation_id);
        println!("host id: {}", accepted.host_id);
        println!("agent state: {}", accepted.agent_state);
    }
    Ok(())
}

async fn run_file(common: CommonArgs, command: FileCommand) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
    let (path, result, reading) = match command {
        FileCommand::Browse { path } => {
            let route = path
                .as_deref()
                .map(|path| format!("/v1/files?path={}", encode_uri_component(path)))
                .unwrap_or_else(|| "/v1/files".to_owned());
            (
                path,
                client.get_json::<serde_json::Value>(&route).await?,
                false,
            )
        }
        FileCommand::Read { path } => {
            let route = format!("/v1/files/read?path={}", encode_uri_component(&path));
            (
                Some(path),
                client.get_json::<serde_json::Value>(&route).await?,
                true,
            )
        }
    };
    if common.json {
        print_json(&result)?;
    } else if reading {
        let name = result
            .get("name")
            .and_then(serde_json::Value::as_str)
            .or(path.as_deref())
            .unwrap_or("file");
        println!("{name}:");
        println!(
            "{}",
            result
                .get("content")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
        );
        if result.get("truncated").and_then(serde_json::Value::as_bool) == Some(true) {
            eprintln!("note: the API limited this preview; the file continues beyond this output");
        }
    } else {
        if let Some(server) = result.get("serverName").and_then(serde_json::Value::as_str) {
            println!("server: {server}");
        }
        println!(
            "path: {}",
            result
                .get("path")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(".")
        );
        if let Some(note) = result.get("note").and_then(serde_json::Value::as_str) {
            println!("note: {note}");
        }
        if let Some(items) = result.get("items").and_then(serde_json::Value::as_array) {
            for item in items {
                let name = item
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("?");
                let kind =
                    if item.get("isDirectory").and_then(serde_json::Value::as_bool) == Some(true) {
                        "directory"
                    } else {
                        "file"
                    };
                let size = item.get("sizeBytes").and_then(serde_json::Value::as_u64);
                if let Some(size) = size {
                    println!("{kind:9} {size:>10} B  {name}");
                } else {
                    println!("{kind:9}              {name}");
                }
            }
        }
    }
    Ok(())
}

async fn run_help(common: CommonArgs, command: HelpCommand) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
    let (result, topic_view) = match command {
        HelpCommand::Catalog => (
            client
                .get_json::<serde_json::Value>("/v1/help/catalog")
                .await?,
            false,
        ),
        HelpCommand::Topic { help_id } => (
            client
                .get_json::<serde_json::Value>(&format!(
                    "/v1/help/{}",
                    encode_uri_component(&help_id)
                ))
                .await?,
            true,
        ),
        HelpCommand::Onboarding => (
            client
                .get_json::<serde_json::Value>("/v1/guides/onboarding")
                .await?,
            false,
        ),
        HelpCommand::RouterCatalog => (
            client
                .get_json::<serde_json::Value>("/v1/guides/router-catalog")
                .await?,
            false,
        ),
        HelpCommand::RouterSearch { query } => (
            client
                .get_json::<serde_json::Value>(&format!(
                    "/v1/guides/router/search?q={}",
                    encode_uri_component(&query)
                ))
                .await?,
            false,
        ),
        HelpCommand::RouterGuide { guide_id } => (
            client
                .get_json::<serde_json::Value>(&format!(
                    "/v1/guides/router/{}",
                    encode_uri_component(&guide_id)
                ))
                .await?,
            false,
        ),
    };
    if common.json {
        print_json(&result)?;
    } else if topic_view {
        if let Some(title) = result.get("title").and_then(serde_json::Value::as_str) {
            println!("{title}");
        }
        if let Some(subtitle) = result.get("subtitle").and_then(serde_json::Value::as_str) {
            println!("{subtitle}\n");
        }
        if let Some(body) = result.get("body").and_then(serde_json::Value::as_str) {
            println!("{body}");
        }
        if let Some(sections) = result.get("sections").and_then(serde_json::Value::as_array) {
            for section in sections {
                print_help_section(section);
            }
        }
    } else {
        print_pretty_json(&result)?;
    }
    Ok(())
}

fn print_help_section(section: &serde_json::Value) {
    match section.get("type").and_then(serde_json::Value::as_str) {
        Some("body") | Some("advanced") => {
            if let Some(markdown) = section.get("markdown").and_then(serde_json::Value::as_str) {
                println!("\n{markdown}");
            }
        }
        Some("bulletList") | Some("inApp") => {
            if let Some(items) = section.get("items").and_then(serde_json::Value::as_array) {
                for item in items.iter().filter_map(serde_json::Value::as_str) {
                    println!("- {item}");
                }
            }
        }
        Some("callout") => {
            if let Some(text) = section.get("text").and_then(serde_json::Value::as_str) {
                println!("\nnote: {text}");
            }
        }
        Some("checklist") => {
            if let Some(steps) = section.get("steps").and_then(serde_json::Value::as_array) {
                for step in steps {
                    let number = step
                        .get("number")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let title = step
                        .get("title")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    let detail = step
                        .get("detail")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    println!("{number}. {title}: {detail}");
                }
            }
        }
        Some("table") => {
            let headers = section.get("headers").and_then(serde_json::Value::as_array);
            let rows = section.get("rows").and_then(serde_json::Value::as_array);
            if let Some(headers) = headers {
                println!(
                    "\n{}",
                    headers
                        .iter()
                        .map(value_text)
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
            }
            if let Some(rows) = rows {
                for row in rows.iter().filter_map(serde_json::Value::as_array) {
                    println!(
                        "{}",
                        row.iter().map(value_text).collect::<Vec<_>>().join(" | ")
                    );
                }
            }
        }
        _ => {}
    }
}

fn value_text(value: &serde_json::Value) -> String {
    value.as_str().unwrap_or("").to_owned()
}

fn print_pretty_json(value: &serde_json::Value) -> Result<(), CliError> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|error| CliError::internal(format!(
            "failed to encode help output: {error}"
        )))?
    );
    Ok(())
}

async fn run_bedrock(common: CommonArgs, command: BedrockCommand) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
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

async fn run_player(common: CommonArgs, command: PlayerCommand) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
    match command {
        PlayerCommand::Online { server } => {
            select_player_server(&client, server.as_deref()).await?;
            let value: serde_json::Value = client.get_json("/v1/players").await?;
            if common.json {
                print_json(&value)?;
            } else if let Some(players) = value["players"].as_array() {
                println!(
                    "players online: {}",
                    value["count"].as_u64().unwrap_or(players.len() as u64)
                );
                for player in players {
                    println!("- {}", player["name"].as_str().unwrap_or("unknown"));
                }
                if let Some(note) = value["note"].as_str() {
                    println!("note: {note}");
                }
            }
        }
        PlayerCommand::Profiles { server } => {
            select_player_server(&client, server.as_deref()).await?;
            let value: serde_json::Value = client.get_json("/v1/players/profiles").await?;
            if common.json {
                let profiles = value["profiles"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|profile| {
                        serde_json::json!({
                            "id": profile["id"],
                            "username": profile["username"],
                            "isOnline": profile["isOnline"],
                            "isOp": profile["isOp"],
                            "lastSeen": profile["lastSeen"],
                            "isBedrockPlayer": profile["isBedrockPlayer"],
                            "isHidden": profile["isHidden"],
                        })
                    })
                    .collect::<Vec<_>>();
                print_json(
                    &serde_json::json!({"profiles": profiles, "isLoadingStats": value["isLoadingStats"]}),
                )?;
            } else if let Some(profiles) = value["profiles"].as_array() {
                for profile in profiles {
                    println!(
                        "{}  {}{}{}",
                        profile["username"].as_str().unwrap_or("unknown"),
                        profile["id"].as_str().unwrap_or("?"),
                        if profile["isOnline"].as_bool() == Some(true) {
                            " [online]"
                        } else {
                            ""
                        },
                        if profile["isOp"].as_bool() == Some(true) {
                            " [operator]"
                        } else {
                            ""
                        }
                    );
                }
                if profiles.is_empty() {
                    println!("No known player profiles.");
                }
            }
        }
        PlayerCommand::Data { command } => run_player_data(&client, common, command).await?,
        PlayerCommand::SkinOverride {
            profile_id,
            lookup_identifier,
            server,
        } => {
            select_player_server(&client, server.as_deref()).await?;
            let value: serde_json::Value = client.post_json("/v1/players/skin-override", &serde_json::json!({"profileId":profile_id,"lookupIdentifier":lookup_identifier})).await?;
            output_value(common.json, &value)?;
        }
        PlayerCommand::Hide {
            profile_id,
            hidden,
            server,
        } => {
            select_player_server(&client, server.as_deref()).await?;
            let value: serde_json::Value = client
                .post_json(
                    "/v1/players/hidden",
                    &serde_json::json!({"profileId":profile_id,"hidden":hidden}),
                )
                .await?;
            output_value(common.json, &value)?;
        }
        PlayerCommand::Identify {
            profile_id,
            gamertag,
            server,
        } => {
            select_player_server(&client, server.as_deref()).await?;
            let value: serde_json::Value = client
                .post_json(
                    "/v1/players/identify",
                    &serde_json::json!({"profileId":profile_id,"gamertag":gamertag}),
                )
                .await?;
            output_value(common.json, &value)?;
        }
        PlayerCommand::Message {
            player,
            message,
            server,
        } => {
            player_action(
                &client,
                common,
                server.as_deref(),
                "message",
                &player,
                Some(&message),
                None,
            )
            .await?
        }
        PlayerCommand::Kick {
            player,
            reason,
            server,
        } => {
            player_action(
                &client,
                common,
                server.as_deref(),
                "kick",
                &player,
                None,
                reason.as_deref(),
            )
            .await?
        }
        PlayerCommand::Ban {
            player,
            reason,
            server,
        } => {
            player_action(
                &client,
                common,
                server.as_deref(),
                "ban",
                &player,
                None,
                reason.as_deref(),
            )
            .await?
        }
        PlayerCommand::Pardon { player, server } => {
            player_action(
                &client,
                common,
                server.as_deref(),
                "pardon",
                &player,
                None,
                None,
            )
            .await?
        }
        PlayerCommand::Op { player, server } => {
            player_action(
                &client,
                common,
                server.as_deref(),
                "op",
                &player,
                None,
                None,
            )
            .await?
        }
        PlayerCommand::Deop { player, server } => {
            player_action(
                &client,
                common,
                server.as_deref(),
                "deop",
                &player,
                None,
                None,
            )
            .await?
        }
        PlayerCommand::Whitelist { command } => match command {
            PlayerWhitelistCommand::Add { player, server } => {
                player_action(
                    &client,
                    common,
                    server.as_deref(),
                    "whitelist-add",
                    &player,
                    None,
                    None,
                )
                .await?
            }
            PlayerWhitelistCommand::Remove { player, server } => {
                player_action(
                    &client,
                    common,
                    server.as_deref(),
                    "whitelist-remove",
                    &player,
                    None,
                    None,
                )
                .await?
            }
        },
    }
    Ok(())
}

async fn run_player_data(
    client: &ApiClient,
    common: CommonArgs,
    command: PlayerDataCommand,
) -> Result<(), CliError> {
    let (player, server, action, target_uuid, confirmed) = match command {
        PlayerDataCommand::Show { player, server } => (player, server, None, None, true),
        PlayerDataCommand::Duplicate { player, server } => {
            (player, server, Some("duplicate"), None, true)
        }
        PlayerDataCommand::Delete {
            player,
            server,
            confirm,
        } => (player, server, Some("delete"), None, confirm),
        PlayerDataCommand::MigrateOffline {
            player,
            server,
            confirm,
        } => (player, server, Some("migrate-offline"), None, confirm),
        PlayerDataCommand::Migrate {
            player,
            target_uuid,
            server,
            confirm,
        } => (player, server, Some("migrate"), Some(target_uuid), confirm),
    };
    let selected = select_player_server(client, server.as_deref()).await?;
    let response: serde_json::Value = client.get_json("/v1/players/profiles").await?;
    let profiles = response["profiles"]
        .as_array()
        .ok_or_else(|| CliError::internal("agent returned an invalid player profile list"))?;
    let profile = resolve_player_profile(profiles, &player)?;
    let profile_id = profile["id"]
        .as_str()
        .ok_or_else(|| CliError::internal("player profile has no id"))?;
    let Some(action) = action else {
        if common.json {
            print_json(profile)?;
        } else {
            print_player_data(profile);
        }
        return Ok(());
    };
    if !confirmed {
        return Err(CliError::usage(format!(
            "{action} changes saved player data; repeat the command with --confirm"
        )));
    }
    let path = match action {
        "duplicate" => "/v1/players/duplicate",
        "delete" => "/v1/players/delete",
        "migrate-offline" => "/v1/players/migrate-offline",
        "migrate" => "/v1/players/migrate",
        _ => unreachable!(),
    };
    let mut body =
        serde_json::json!({"profileId": profile_id, "expectedActiveServerId": selected.id});
    if let Some(target_uuid) = target_uuid {
        body["targetUuid"] = target_uuid.into();
    }
    let result: serde_json::Value = client.post_json(path, &body).await?;
    if common.json {
        print_json(&result)?;
    } else {
        println!(
            "{} player data for {}",
            result["message"].as_str().unwrap_or(action),
            profile["username"].as_str().unwrap_or(&player)
        );
        if let Some(new_profile_id) = result["newProfileId"].as_str() {
            println!("new profile id: {new_profile_id}");
        }
        if selected.server_type == "bedrock" && action == "delete" {
            println!("Bedrock player data changes require a stopped server.");
        }
    }
    Ok(())
}

fn resolve_player_profile<'a>(
    profiles: &'a [serde_json::Value],
    selector: &str,
) -> Result<&'a serde_json::Value, CliError> {
    if let Some(profile) = profiles
        .iter()
        .find(|profile| profile["id"].as_str() == Some(selector))
    {
        return Ok(profile);
    }
    let exact = profiles
        .iter()
        .filter(|profile| profile["username"].as_str() == Some(selector))
        .collect::<Vec<_>>();
    if exact.len() == 1 {
        return Ok(exact[0]);
    }
    if exact.len() > 1 {
        return Err(CliError::usage(format!(
            "multiple player profiles are named {selector:?}; use the profile id"
        )));
    }
    let folded = selector.to_ascii_lowercase();
    let matches = profiles
        .iter()
        .filter(|profile| {
            profile["username"]
                .as_str()
                .is_some_and(|name| name.to_ascii_lowercase() == folded)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [profile] => Ok(profile),
        [] => Err(CliError::usage(format!(
            "no player profile matched {selector:?}"
        ))),
        _ => Err(CliError::usage(format!(
            "multiple player profiles match {selector:?}; use the profile id"
        ))),
    }
}

fn print_player_data(profile: &serde_json::Value) {
    println!(
        "{} ({})",
        profile["username"].as_str().unwrap_or("unknown"),
        profile["id"].as_str().unwrap_or("?")
    );
    match &profile["stats"] {
        serde_json::Value::Object(stats) => {
            for key in [
                "health",
                "maxHealth",
                "foodLevel",
                "xpLevel",
                "xpTotal",
                "gameModeDisplay",
                "dimensionDisplay",
                "posX",
                "posY",
                "posZ",
                "score",
            ] {
                if let Some(value) = stats.get(key) {
                    println!("{}: {}", key, value);
                }
            }
        }
        _ => println!("statistics: unavailable"),
    }
    if let Some(items) = profile["inventory"].as_array() {
        println!("inventory items: {}", items.len());
        for item in items {
            println!(
                "slot {}: {} × {}",
                item["slot"],
                item["count"],
                item["displayName"]
                    .as_str()
                    .or(item["itemID"].as_str())
                    .unwrap_or("unknown item")
            );
        }
    } else {
        println!("inventory: unavailable");
    }
}

async fn select_player_server(
    client: &ApiClient,
    selector: Option<&str>,
) -> Result<ServerDto, CliError> {
    if let Some(selector) = selector {
        return ensure_active_server(client, Some(selector)).await;
    }
    let status: RemoteApiStatus = client.get_json("/v1/status").await?;
    let id = status
        .active_server_id
        .ok_or_else(|| CliError::usage("no active server; pass --server"))?;
    resolve_server(client, &id).await
}

async fn player_action(
    client: &ApiClient,
    common: CommonArgs,
    server: Option<&str>,
    action: &str,
    player: &str,
    message: Option<&str>,
    reason: Option<&str>,
) -> Result<(), CliError> {
    let selected = select_player_server(client, server).await?;
    let result: serde_json::Value = client
        .post_json(
            "/v1/players/action",
            &serde_json::json!({
                "action": action,
                "player": player,
                "message": message,
                "reason": reason,
                "expectedActiveServerId": selected.id,
            }),
        )
        .await?;
    if common.json {
        print_json(&result)?;
    } else {
        println!("{} {} on {}", action, player, selected.name);
    }
    Ok(())
}

async fn run_access(common: CommonArgs, command: AccessCommand) -> Result<(), CliError> {
    use std::io::IsTerminal;

    let client = ApiClient::connect_local().await?;
    match command {
        AccessCommand::Me => {
            let value: serde_json::Value = client.get_json("/v1/me").await?;
            if common.json {
                print_json(&value)?;
            } else {
                println!("role: {}", value["role"].as_str().unwrap_or("unknown"));
                println!("name: {}", value["name"].as_str().unwrap_or("unknown"));
                println!(
                    "named token: {}",
                    value["isNamedToken"].as_bool().unwrap_or(false)
                );
                println!(
                    "permissions: {}",
                    value["permissions"]
                        .as_array()
                        .map(|items| items
                            .iter()
                            .filter_map(serde_json::Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", "))
                        .unwrap_or_default()
                );
            }
        }
        AccessCommand::List => {
            let value: serde_json::Value = client.get_json("/v1/users").await?;
            if common.json {
                print_json(&value)?;
            } else if let Some(users) = value["users"].as_array() {
                for user in users {
                    println!(
                        "{}  {} ({}){}",
                        user["id"].as_str().unwrap_or("?"),
                        user["label"].as_str().unwrap_or("unnamed"),
                        user["role"].as_str().unwrap_or("unknown"),
                        if user["isExpired"].as_bool() == Some(true) {
                            " [expired]"
                        } else {
                            ""
                        }
                    );
                    println!(
                        "  permissions: {}",
                        user["permissions"]
                            .as_array()
                            .map(|items| items
                                .iter()
                                .filter_map(serde_json::Value::as_str)
                                .collect::<Vec<_>>()
                                .join(", "))
                            .unwrap_or_default()
                    );
                    println!(
                        "  expires: {}",
                        user["expiresAtISO8601"].as_str().unwrap_or("never")
                    );
                }
            }
        }
        AccessCommand::Create {
            label,
            role,
            permissions,
            expires_in_days,
        } => {
            if !std::io::stdout().is_terminal() {
                return Err(CliError::usage(
                    "token creation prints the new secret once; run this command in a terminal",
                ));
            }
            let value: serde_json::Value = client.post_json("/v1/users", &serde_json::json!({"label": label, "role": role, "permissions": permissions, "expiresInDays": expires_in_days})).await?;
            if common.json {
                print_json(&value)?;
            } else {
                println!(
                    "created token: {}",
                    value["user"]["label"].as_str().unwrap_or(&label)
                );
                println!("id: {}", value["user"]["id"].as_str().unwrap_or("unknown"));
                println!(
                    "secret (shown once): {}",
                    value["token"].as_str().unwrap_or("missing")
                );
                println!(
                    "permissions: {}",
                    value["user"]["permissions"]
                        .as_array()
                        .map(|items| items
                            .iter()
                            .filter_map(serde_json::Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", "))
                        .unwrap_or_default()
                );
                println!(
                    "expires: {}",
                    value["user"]["expiresAtISO8601"]
                        .as_str()
                        .unwrap_or("never")
                );
            }
        }
        AccessCommand::Update {
            user_id,
            label,
            role,
            permissions,
            clear_permissions,
            expires_in_days,
        } => {
            let mut body = serde_json::json!({"userId": user_id});
            if let Some(label) = label {
                body["label"] = label.into();
            }
            if let Some(role) = role {
                body["role"] = role.into();
            }
            if clear_permissions {
                body["permissions"] = serde_json::json!([]);
            } else if !permissions.is_empty() {
                body["permissions"] = serde_json::json!(permissions);
            }
            if let Some(days) = expires_in_days {
                body["expiresInDays"] = days.into();
            }
            let value: serde_json::Value = client.post_json("/v1/users/update", &body).await?;
            print_admin_result(&common, &value)?;
        }
        AccessCommand::Revoke { user_id } => {
            let value: serde_json::Value = client
                .post_json("/v1/users/revoke", &serde_json::json!({"userId": user_id}))
                .await?;
            print_admin_result(&common, &value)?;
        }
    }
    Ok(())
}

fn print_admin_result(common: &CommonArgs, value: &serde_json::Value) -> Result<(), CliError> {
    if common.json {
        print_json(value)
    } else {
        println!("{}", value["message"].as_str().unwrap_or("ok"));
        Ok(())
    }
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

async fn run_server(common: CommonArgs, command: ServerCommand) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
    match command {
        ServerCommand::List => {
            let servers: Vec<ServerDto> = client.get_json("/v1/servers").await?;
            let status: RemoteApiStatus = client.get_json("/v1/status").await?;
            if common.json {
                print_json(
                    &serde_json::json!({"activeServerId": status.active_server_id, "servers": servers}),
                )?;
            } else {
                for server in &servers {
                    let active = status.active_server_id.as_deref() == Some(server.id.as_str());
                    println!(
                        "{}{}  {} [{}]",
                        if active { "* " } else { "  " },
                        server.name,
                        server.id,
                        server.server_type
                    );
                }
                if servers.is_empty() {
                    println!("No registered servers.");
                }
                println!("* marks the active server.");
            }
            Ok(())
        }
        ServerCommand::Show { server } => {
            let server = resolve_server(&client, &server).await?;
            if common.json {
                print_json(&server)?;
            } else {
                print_server_detail(&server);
            }
            Ok(())
        }
        ServerCommand::Use { server } => {
            let selected = resolve_server(&client, &server).await?;
            let body = ActiveServerRequestDto {
                server_id: Some(selected.id.clone()),
            };
            let _: SimpleResultDto = client.post_json("/v1/active-server", &body).await?;
            if common.json {
                print_json(&selected)?;
            } else {
                println!("active server: {} ({})", selected.name, selected.id);
            }
            Ok(())
        }
        ServerCommand::Size { server } => {
            let server = resolve_server(&client, &server).await?;
            let size: ServerDirectorySizeResponseDto = client
                .get_json(&format!(
                    "/v1/servers/size?serverId={}",
                    encode_uri_component(&server.id)
                ))
                .await?;
            if common.json {
                print_json(&size)?;
            } else {
                println!(
                    "{}: {}",
                    server.name,
                    size.size_bytes
                        .map(format_bytes)
                        .unwrap_or_else(|| "unavailable".to_owned())
                );
            }
            Ok(())
        }
        ServerCommand::Notes { server, text } => {
            let server = resolve_server(&client, &server).await?;
            let Some(text) = text else {
                if common.json {
                    print_json(&serde_json::json!({"serverId": server.id, "notes": server.notes}))?;
                } else {
                    println!(
                        "{}",
                        if server.notes.is_empty() {
                            "(no notes)"
                        } else {
                            &server.notes
                        }
                    );
                }
                return Ok(());
            };
            let body = ServerNotesRequestDto {
                server_id: server.id,
                notes: text,
            };
            let result: ServerNotesResultDto = client.post_json("/v1/servers/notes", &body).await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result.message);
            }
            Ok(())
        }
        ServerCommand::BedrockTransport { server, transport } => {
            let server = resolve_server(&client, &server).await?;
            let value: serde_json::Value = client
                .post_json(
                    "/v1/servers/bedrock-transport",
                    &serde_json::json!({"serverId":server.id,"transport":transport}),
                )
                .await?;
            output_value(common.json, &value)
        }
        ServerCommand::Playit { server, enabled } => {
            let server = resolve_server(&client, &server).await?;
            let value: serde_json::Value = client
                .post_json(
                    "/v1/servers/playit",
                    &serde_json::json!({"serverId":server.id,"enabled":enabled}),
                )
                .await?;
            output_value(common.json, &value)
        }
        ServerCommand::XboxBroadcast { server, enabled } => {
            let server = resolve_server(&client, &server).await?;
            let value: serde_json::Value = client
                .post_json(
                    "/v1/servers/xbox-broadcast",
                    &serde_json::json!({"serverId":server.id,"enabled":enabled}),
                )
                .await?;
            output_value(common.json, &value)
        }
        ServerCommand::Export { output } => {
            let result: ServerTransferExportResultDto = client
                .post_json("/v1/servers/export", &serde_json::json!({}))
                .await?;
            let bytes = client
                .get_raw_bytes(&format!(
                    "/v1/staged-downloads/{}",
                    result.staged_download_id
                ))
                .await?;
            std::fs::write(&output, &bytes).map_err(|error| {
                CliError::internal(format!("could not write {}: {error}", output.display()))
            })?;
            if common.json {
                print_json(
                    &serde_json::json!({"path": output, "fileName": result.file_name, "serverCount": result.server_count, "sizeBytes": bytes.len(), "expiresAt": result.expires_at}),
                )?;
            } else {
                println!(
                    "exported {} servers ({} bytes) to {}",
                    result.server_count,
                    bytes.len(),
                    output.display()
                );
            }
            Ok(())
        }
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
                check_addon_updates: None,
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
                check_addon_updates: None,
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
            let confirm = args.confirm.clone();
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
                check_addon_updates: None,
                enable_voice_chat: None,
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
                world_settings: None,
            };
            let mut request = serde_json::to_value(body).map_err(|error| {
                CliError::internal(format!("failed to encode create request: {error}"))
            })?;
            if let Some(confirm) = confirm {
                request["confirmation"] = serde_json::Value::String(confirm);
            }
            let result: ServerCreateResultDto =
                client.post_json("/v1/servers/create", &request).await?;
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
    client: &ApiClient,
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
                file_name: Some(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                ),
                operation_id,
                file_id,
                expected_bytes: None,
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
    let client = ApiClient::connect_local().await?;
    if args.server.is_some() {
        ensure_active_server(&client, args.server.as_deref()).await?;
    }
    let mut body = serde_json::json!({ "command": args.text });
    if let Some(confirm) = args.confirm {
        body["confirmation"] = serde_json::Value::String(confirm);
    }
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
    let client = ApiClient::connect_local().await?;
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
        ConsoleCommand::Follow { server } => {
            if let Some(server) = server.as_deref() {
                let selected = ensure_active_server(&client, Some(server)).await?;
                if !common.json {
                    eprintln!(
                        "following {} ({}) — Ctrl-C to stop",
                        selected.name, selected.id
                    );
                }
            } else {
                let status: RemoteApiStatus = client.get_json("/v1/status").await?;
                if status.active_server_id.is_none() {
                    return Err(CliError::usage("no active server; pass --server"));
                }
                if !common.json {
                    eprintln!("following active server — Ctrl-C to stop");
                }
            }
            let mut stream = client.connect_console_stream().await?;
            loop {
                tokio::select! {
                    signal = tokio::signal::ctrl_c() => {
                        signal.map_err(|error| CliError::internal(format!("could not listen for Ctrl-C: {error}")))?;
                        break;
                    }
                    message = stream.next() => {
                        match message {
                            Some(Ok(tokio_tungstenite::tungstenite::Message::Text(line))) => print_console_line(common.json, line.as_str()),
                            Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) | None => break,
                            Some(Ok(_)) => {},
                            Some(Err(error)) => return Err(CliError::internal(format!("console stream ended with an error: {error}"))),
                        }
                    }
                }
            }
            Ok(())
        }
    }
}

async fn run_metrics(common: CommonArgs, server: Option<String>) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
    if let Some(server) = server.as_deref() {
        ensure_active_server(&client, Some(server)).await?;
    }
    let snapshot: PerformanceSnapshotDto = client.get_json("/v1/performance").await?;
    if common.json {
        print_json(&snapshot)?;
    } else {
        println!(
            "server: {}",
            snapshot.server_type.as_deref().unwrap_or("unknown")
        );
        println!("sampled: {}", snapshot.ts);
        print_optional_metric("TPS (1 min)", &snapshot.tps_1m);
        print_optional_metric("TPS (5 min)", &snapshot.tps_5m);
        print_optional_metric("TPS (15 min)", &snapshot.tps_15m);
        println!(
            "players online: {}",
            snapshot
                .players_online
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unavailable".to_owned())
        );
        print_optional_metric("server CPU", &snapshot.cpu_percent);
        print_optional_metric("server RAM used (MB)", &snapshot.ram_used_mb);
        print_optional_metric("server RAM maximum (MB)", &snapshot.ram_max_mb);
        print_optional_metric("world size (MB)", &snapshot.world_size_mb);
        println!(
            "world day: {}",
            snapshot
                .world_day
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unavailable".to_owned())
        );
        if snapshot.server_type.as_deref() == Some("bedrock") {
            println!("TPS: unavailable for Bedrock");
        }
        print_runtime(&snapshot.runtime);
    }
    Ok(())
}

async fn run_sessions(common: CommonArgs, server: Option<String>) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
    if let Some(server) = server.as_deref() {
        ensure_active_server(&client, Some(server)).await?;
    }
    let value: serde_json::Value = client.get_json("/v1/session-log").await?;
    if common.json {
        print_json(&value)?;
    } else if let Some(events) = value["events"].as_array() {
        if events.is_empty() {
            println!("No player sessions recorded.");
        }
        for event in events {
            println!(
                "{}  {}  {}",
                event["timestamp"].as_str().unwrap_or("unknown time"),
                event["eventType"].as_str().unwrap_or("event"),
                event["playerName"].as_str().unwrap_or("unknown player")
            );
        }
    }
    Ok(())
}

fn print_console_line(json: bool, line: &str) {
    if json {
        println!("{line}");
        return;
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
        println!(
            "{} {}",
            value["level"].as_str().unwrap_or(""),
            value["text"].as_str().unwrap_or(line)
        );
    } else {
        println!("{line}");
    }
}

fn print_optional_metric(label: &str, metric: &Option<PerformanceMetricNumberDto>) {
    match metric {
        Some(metric) => println!("{label}: {}", metric.value),
        None => println!("{label}: unavailable"),
    }
}

async fn run_capabilities(common: CommonArgs) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
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
    let client = ApiClient::connect_local().await?;
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

async fn run_config(common: CommonArgs, command: ConfigCommand) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
    let (method, path, body) = match command {
        ConfigCommand::Ram { command } => {
            let value: serde_json::Value = match command {
                RamCommand::Get => client.get_json("/v1/config/ram").await?,
                RamCommand::Set { min_gb, max_gb } => {
                    client
                        .post_json(
                            "/v1/config/ram",
                            &serde_json::json!({"minRamGB":min_gb,"maxRamGB":max_gb}),
                        )
                        .await?
                }
            };
            return output_value(common.json, &value);
        }
        ConfigCommand::HostSetup => ("GET", "/v1/config/host-setup", None),
        ConfigCommand::CompleteHostSetup => (
            "POST",
            "/v1/config/host-setup/complete",
            Some(serde_json::json!({})),
        ),
        ConfigCommand::ServersRoot => ("GET", "/v1/config/servers-root", None),
        ConfigCommand::SetServersRoot { path } => (
            "POST",
            "/v1/config/servers-root",
            Some(serde_json::json!({"path": path})),
        ),
        ConfigCommand::Geyser => ("GET", "/v1/config/geyser", None),
        ConfigCommand::SetGeyser { address, port } => (
            "POST",
            "/v1/config/geyser",
            Some(serde_json::json!({"address": address, "port": port})),
        ),
        ConfigCommand::CurseForge => ("GET", "/v1/config/curseforge", None),
        ConfigCommand::SetCurseForgeKey { key_stdin } => {
            let key = read_secret_stdin(key_stdin, "CurseForge API key", "key-stdin")?;
            (
                "POST",
                "/v1/config/curseforge",
                Some(serde_json::json!({"apiKey": key})),
            )
        }
        ConfigCommand::Watchdog => ("GET", "/v1/watchdog/status", None),
        ConfigCommand::WatchdogEnable => {
            ("POST", "/v1/watchdog/enable", Some(serde_json::json!({})))
        }
        ConfigCommand::WatchdogDisable => {
            ("POST", "/v1/watchdog/disable", Some(serde_json::json!({})))
        }
    };
    let result: serde_json::Value = if method == "GET" {
        client.get_json(path).await?
    } else {
        client
            .post_json(path, &body.unwrap_or_else(|| serde_json::json!({})))
            .await?
    };
    if common.json {
        print_json(&result)
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
        Ok(())
    }
}

async fn run_components(common: CommonArgs) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
    let value: serde_json::Value = client.get_json("/v1/components").await?;
    if common.json {
        print_json(&value)
    } else {
        if let Some(items) = value["components"].as_array() {
            for item in items {
                println!(
                    "{}: {}",
                    item["name"].as_str().unwrap_or("component"),
                    item["installedLabel"]
                        .as_str()
                        .or_else(|| item["installedVersion"].as_str())
                        .unwrap_or("not installed")
                );
            }
        }
        Ok(())
    }
}

fn output_value(json: bool, value: &serde_json::Value) -> Result<(), CliError> {
    if json {
        print_json(value)
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_default()
        );
        Ok(())
    }
}

fn read_secret_stdin(enabled: bool, label: &str, flag: &str) -> Result<String, CliError> {
    if !enabled {
        return Err(CliError::usage(format!(
            "provide {label} through stdin with --{flag}; secrets are not accepted as command-line arguments"
        )));
    }
    let mut value = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut value)
        .map_err(|error| CliError::usage(format!("could not read {label} from stdin: {error}")))?;
    let value = value.trim_end_matches(['\r', '\n']).to_string();
    if value.trim().is_empty() {
        return Err(CliError::usage(format!("{label} input was empty")));
    }
    Ok(value)
}

async fn run_playit(common: CommonArgs, command: PlayitCommand) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
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
        PlayitCommand::Setup {
            email,
            password_stdin,
        } => {
            let password = read_secret_stdin(password_stdin, "Playit password", "password-stdin")?;
            let result: serde_json::Value = client
                .post_json(
                    "/v1/playit/setup",
                    &serde_json::json!({"email": email, "password": password}),
                )
                .await?;
            finish_operation(
                &client,
                common.json,
                false,
                result["operationId"].as_str().map(str::to_owned),
                "Playit setup",
            )
            .await
        }
        PlayitCommand::Reset => {
            let result: serde_json::Value = client
                .post_json("/v1/playit/reset", &serde_json::json!({}))
                .await?;
            if common.json {
                print_json(&result)
            } else {
                println!("Playit host setup reset.");
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
    let client = ApiClient::connect_local().await?;
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
            gamertag,
            password_stdin,
        } => {
            let password =
                read_secret_stdin(password_stdin, "Xbox Broadcast password", "password-stdin")?;
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
        BroadcastCommand::ClearCredentials { confirm } => {
            if !confirm {
                return Err(CliError::usage(
                    "clearing Xbox Broadcast credentials requires --confirm",
                ));
            }
            let value: serde_json::Value = client
                .post_json("/v1/broadcast/credentials/clear", &serde_json::json!({}))
                .await?;
            output_value(common.json, &value)
        }
    }
}

async fn run_resource_pack(
    common: CommonArgs,
    command: ResourcePackCommand,
) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
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
        ResourcePackCommand::SetUrl { url, sha1, require } => {
            let result: serde_json::Value = client
                .post_json(
                    "/v1/resourcepacks/seturl",
                    &serde_json::json!({"url": url, "sha1": sha1, "require": require}),
                )
                .await?;
            if common.json {
                print_json(&result)
            } else {
                println!("Java resource-pack URL saved.");
                Ok(())
            }
        }
        ResourcePackCommand::ClearUrl => {
            let result: serde_json::Value = client
                .post_json(
                    "/v1/resourcepacks/activate",
                    &serde_json::json!({"packId": null, "require": false}),
                )
                .await?;
            if common.json {
                print_json(&result)
            } else {
                println!("Java resource-pack URL cleared.");
                Ok(())
            }
        }
        ResourcePackCommand::Remove { pack_id } => {
            let result: serde_json::Value = client
                .post_json(
                    "/v1/resourcepacks/remove",
                    &serde_json::json!({"packId": pack_id, "packKind": "java"}),
                )
                .await?;
            if common.json {
                print_json(&result)
            } else {
                println!("Java resource pack removed.");
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
        ResourcePackCommand::Toggle { pack_id, enabled } => {
            let value: serde_json::Value = client
                .post_json(
                    "/v1/resourcepacks/toggle",
                    &serde_json::json!({"packId":pack_id,"enabled":enabled}),
                )
                .await?;
            output_value(common.json, &value)
        }
    }
}

async fn run_settings(common: CommonArgs, command: SettingsCommand) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
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
        SettingsCommand::Set {
            server,
            changes,
            confirm,
        } => {
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
            let mut body = serde_json::json!({ "changes": parsed });
            if let Some(confirm) = confirm {
                body["confirmation"] = serde_json::Value::String(confirm);
            }
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
    let client = ApiClient::connect_local().await?;
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
        WorldCommand::SaveCurrent => {
            let result: WorldMutationResultDto = client
                .post_json("/v1/worlds/update", &serde_json::json!({}))
                .await?;
            print_world_mutation_result(common.json, &result)
        }
        WorldCommand::Profile { slot_id } => {
            let result: serde_json::Value = client
                .get_json(&format!("/v1/worlds/{slot_id}/profile"))
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!(
                    "world: {}",
                    result["slot"]["name"].as_str().unwrap_or(&slot_id)
                );
                println!(
                    "profile: {}",
                    serde_json::to_string_pretty(&result["profile"]).unwrap_or_default()
                );
            }
            Ok(())
        }
        WorldCommand::Repair { slot_id, no_wait } => {
            let result: serde_json::Value = client
                .post_json("/v1/worlds/repair", &serde_json::json!({"slotId": slot_id}))
                .await?;
            finish_operation(
                &client,
                common.json,
                no_wait,
                result["operationId"].as_str().map(str::to_owned),
                "Bedrock world repair",
            )
            .await
        }
        WorldCommand::RenameActive { name } => {
            let result: serde_json::Value = client
                .post_json(
                    "/v1/worlds/rename-active-world",
                    &serde_json::json!({"name": name}),
                )
                .await?;
            if common.json {
                print_json(&result)
            } else {
                println!("Active world renamed.");
                Ok(())
            }
        }
        WorldCommand::ConvertFormats => {
            let result: serde_json::Value = client.get_json("/v1/worlds/convert/formats").await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!(
                    "Chunker installed: {}",
                    result["installed"].as_bool().unwrap_or(false)
                );
                println!(
                    "Java available: {}",
                    result["javaAvailable"].as_bool().unwrap_or(false)
                );
                if let Some(version) = result["version"].as_str() {
                    println!("Chunker version: {version}");
                }
                for format in result["formats"].as_array().into_iter().flatten() {
                    println!("- {}", format.as_str().unwrap_or("unknown"));
                }
            }
            Ok(())
        }
        WorldCommand::AcquireChunker { no_wait } => {
            let result: serde_json::Value = client
                .post_json("/v1/worlds/convert/chunker", &serde_json::json!({}))
                .await?;
            finish_operation(
                &client,
                common.json,
                no_wait,
                result["operationId"].as_str().map(str::to_owned),
                "Chunker download",
            )
            .await
        }
        WorldCommand::Pack { command } => run_world_pack(&client, common, command).await,
        WorldCommand::Create { name, seed } => {
            let body = WorldCreateRequestDto {
                name,
                seed,
                ..Default::default()
            };
            let result: WorldMutationResultDto =
                client.post_json("/v1/worlds/create", &body).await?;
            print_world_mutation_result(common.json, &result)
        }
        WorldCommand::ProfileSet {
            slot_id,
            changes,
            confirm,
        } => {
            let mut parsed = serde_json::Map::new();
            for change in changes {
                let (key, value) = change.split_once('=').ok_or_else(|| {
                    CliError::usage(format!(
                        "invalid profile change {change:?}; expected key=value"
                    ))
                })?;
                let key = key.trim();
                if key.is_empty() {
                    return Err(CliError::usage(format!(
                        "invalid profile change {change:?}; key cannot be empty"
                    )));
                }
                let value = value.trim();
                let parsed_value = match value {
                    "true" => serde_json::Value::Bool(true),
                    "false" => serde_json::Value::Bool(false),
                    _ => serde_json::Value::String(value.to_string()),
                };
                parsed.insert(key.to_string(), parsed_value);
            }
            let mut body = serde_json::json!({ "changes": parsed });
            if let Some(confirm) = confirm {
                body["confirmation"] = serde_json::Value::String(confirm);
            }
            let result: serde_json::Value = client
                .post_json(&format!("/v1/worlds/{slot_id}/profile"), &body)
                .await?;
            if common.json {
                print_json(&result)?;
            } else {
                println!("{}", result["message"].as_str().unwrap_or("Profile saved."));
            }
            Ok(())
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
                        file_name: None,
                        operation_id: None,
                        file_id: None,
                        expected_bytes: None,
                    },
                )
                .await?;
            let _uploaded: StagedUploadCompleteResultDto = client
                .put_bytes(&begin.upload_path, "application/octet-stream", bytes)
                .await?;
            let body = WorldImportRequestDto {
                name,
                staged_upload_id: begin.staged_upload_id,
                backup_id: None,
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
                                file_name: None,
                                operation_id: None,
                                file_id: None,
                                expected_bytes: None,
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
        WorldCommand::Activate {
            slot_id,
            confirm,
            no_wait,
        } => {
            let mut body = serde_json::json!({ "slotId": slot_id });
            if let Some(confirm) = confirm {
                body["confirmation"] = serde_json::Value::String(confirm);
            }
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

async fn run_world_pack(
    client: &ApiClient,
    common: CommonArgs,
    command: WorldPackCommand,
) -> Result<(), CliError> {
    match command {
        WorldPackCommand::List { slot_id } => {
            let result: serde_json::Value = client
                .get_json(&format!("/v1/worlds/{slot_id}/profile"))
                .await?;
            let packs = result["profile"]["packs"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            print_target_world(&result["slot"], &slot_id, common.json)?;
            if common.json {
                print_json(&serde_json::json!({"world": result["slot"], "packs": packs}))?;
            } else if packs.is_empty() {
                println!("No provider-managed packs are recorded for this world.");
            } else {
                for pack in packs {
                    println!(
                        "{}  {} [{}] {}",
                        pack["id"].as_str().unwrap_or("?"),
                        pack["name"].as_str().unwrap_or("unknown"),
                        pack["kind"].as_str().unwrap_or("unknown"),
                        if pack["enabled"].as_bool() == Some(true) {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    );
                }
            }
            Ok(())
        }
        WorldPackCommand::Search { slot_id, query } => {
            let world: serde_json::Value = client
                .get_json(&format!("/v1/worlds/{slot_id}/profile"))
                .await?;
            let status: RemoteApiStatus = client.get_json("/v1/status").await?;
            let versions: VersionsResponseDto = client.get_json("/v1/versions").await?;
            let game_version = versions.current_version.unwrap_or_default();
            let route = if status.server_type.as_deref() == Some("bedrock") {
                "behaviorpacks"
            } else {
                "datapacks"
            };
            let path = format!(
                "/v1/catalog/{route}?q={}&gameVersion={}",
                encode_uri_component(&query),
                encode_uri_component(&game_version)
            );
            let results: serde_json::Value = client.get_json(&path).await?;
            if common.json {
                print_json(&serde_json::json!({"targetWorld": world["slot"], "catalog": results}))
            } else {
                print_target_world(&world["slot"], &slot_id, false)?;
                print_catalog_value(false, &results)
            }
        }
        WorldPackCommand::Inspect {
            slot_id,
            project_id,
        } => {
            let world: serde_json::Value = client
                .get_json(&format!("/v1/worlds/{slot_id}/profile"))
                .await?;
            let status: RemoteApiStatus = client.get_json("/v1/status").await?;
            let path = if status.server_type.as_deref() == Some("bedrock") {
                format!(
                    "/v1/catalog/behaviorpacks/{}",
                    encode_uri_component(&project_id)
                )
            } else {
                format!("/v1/catalog/projects/{}", encode_uri_component(&project_id))
            };
            let detail: serde_json::Value = client.get_json(&path).await?;
            let compatible = if status.server_type.as_deref() == Some("bedrock") {
                serde_json::json!({"files": detail["files"]})
            } else {
                client
                    .get_json(&format!(
                        "/v1/catalog/projects/{}/versions",
                        encode_uri_component(&project_id)
                    ))
                    .await?
            };
            if common.json {
                print_json(
                    &serde_json::json!({"targetWorld": world["slot"], "project": detail, "compatibleVersionsOrFiles": compatible}),
                )?;
            } else {
                print_target_world(&world["slot"], &slot_id, false)?;
                println!("project: {}", detail["title"].as_str().unwrap_or("unknown"));
                println!(
                    "compatibility and dependencies: {}",
                    serde_json::to_string_pretty(&compatible).unwrap_or_default()
                );
            }
            Ok(())
        }
        WorldPackCommand::Install {
            slot_id,
            project_id,
            version_or_file_id,
            confirm: _,
            no_wait,
        } => {
            let world: serde_json::Value = client
                .get_json(&format!("/v1/worlds/{slot_id}/profile"))
                .await?;
            let status: RemoteApiStatus = client.get_json("/v1/status").await?;
            print_target_world(&world["slot"], &slot_id, common.json)?;
            if !common.json {
                println!(
                    "confirming install of project {project_id} version/file {version_or_file_id}"
                );
            }
            if status.server_type.as_deref() == Some("bedrock") {
                let file_id = version_or_file_id.parse::<i64>().map_err(|_| CliError::usage("Bedrock installation requires the numeric file ID shown by world pack inspect"))?;
                let result: serde_json::Value = client
                    .post_json(
                        &format!("/v1/worlds/{slot_id}/behaviorpacks/install"),
                        &serde_json::json!({"projectId": project_id, "fileId": file_id}),
                    )
                    .await?;
                finish_operation(
                    client,
                    common.json,
                    no_wait,
                    result["operationId"].as_str().map(str::to_owned),
                    "Bedrock behavior-pack install",
                )
                .await
            } else {
                let result: serde_json::Value = client.post_json(&format!("/v1/worlds/{slot_id}/datapacks/install"), &serde_json::json!({"projectId": project_id, "versionId": version_or_file_id})).await?;
                if common.json {
                    print_json(&serde_json::json!({"targetWorld": world["slot"], "result": result}))
                } else {
                    println!("Java data pack installed in world {slot_id}.");
                    Ok(())
                }
            }
        }
        WorldPackCommand::Enable { slot_id, pack_id } => {
            mutate_world_pack(client, common, slot_id, pack_id, "enable").await
        }
        WorldPackCommand::Disable { slot_id, pack_id } => {
            mutate_world_pack(client, common, slot_id, pack_id, "disable").await
        }
        WorldPackCommand::Remove { slot_id, pack_id } => {
            mutate_world_pack(client, common, slot_id, pack_id, "remove").await
        }
    }
}

async fn mutate_world_pack(
    client: &ApiClient,
    common: CommonArgs,
    slot_id: String,
    pack_id: String,
    action: &str,
) -> Result<(), CliError> {
    let mut world: serde_json::Value = client
        .get_json(&format!("/v1/worlds/{slot_id}/profile"))
        .await?;
    let packs = world["profile"]["packs"]
        .as_array_mut()
        .ok_or_else(|| CliError::internal("world profile did not include pack records"))?;
    let Some(index) = packs.iter().position(|pack| pack["id"] == pack_id) else {
        return Err(CliError::usage(format!(
            "pack {pack_id:?} is not installed in world {slot_id:?}"
        )));
    };
    if packs[index]["kind"] != "bedrock_behavior_pack" {
        return Err(CliError::usage(
            "enable, disable, and remove are currently supported for Bedrock behavior packs; Java data packs can be inspected and installed",
        ));
    }
    match action {
        "enable" => packs[index]["enabled"] = serde_json::Value::Bool(true),
        "disable" => packs[index]["enabled"] = serde_json::Value::Bool(false),
        "remove" => {
            packs.remove(index);
        }
        _ => unreachable!(),
    }
    let result: serde_json::Value = client
        .post_json(
            &format!("/v1/worlds/{slot_id}/profile"),
            &serde_json::json!({"changes": {"packs": packs}}),
        )
        .await?;
    if common.json {
        print_json(&result)
    } else {
        println!("World pack {action} complete.");
        Ok(())
    }
}

async fn run_backup(common: CommonArgs, command: BackupCommand) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
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
    client: &ApiClient,
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
    client: &ApiClient,
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
    let client = ApiClient::connect_local().await?;
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

async fn run_java(common: CommonArgs, command: JavaCommand) -> Result<(), CliError> {
    let client = ApiClient::connect_local().await?;
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
                extra_flags: None,
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
    let client = ApiClient::connect_local().await?;
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
    let client = ApiClient::connect_local().await?;
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
        AddonCommand::Inspect { project_id } => {
            let encoded = encode_uri_component(&project_id);
            let detail: serde_json::Value = client
                .get_json(&format!("/v1/catalog/projects/{encoded}"))
                .await?;
            let versions: serde_json::Value = client
                .get_json(&format!("/v1/catalog/projects/{encoded}/versions"))
                .await?;
            if common.json {
                print_json(&serde_json::json!({"project": detail, "versions": versions}))?;
            } else {
                println!(
                    "{} ({})",
                    detail["title"].as_str().unwrap_or("Untitled"),
                    detail["projectId"].as_str().unwrap_or(&project_id)
                );
                println!(
                    "server support: {}",
                    detail["serverSide"].as_str().unwrap_or("unknown")
                );
                if let Some(description) = detail["description"].as_str() {
                    println!("{description}");
                }
                for version in versions["versions"].as_array().into_iter().flatten() {
                    println!(
                        "{}  {}  loaders: {}  Minecraft: {}",
                        version["id"].as_str().unwrap_or("?"),
                        version["versionNumber"].as_str().unwrap_or("?"),
                        value_list(&version["loaders"]),
                        value_list(&version["gameVersions"])
                    );
                    for dependency in version["dependencies"].as_array().into_iter().flatten() {
                        println!(
                            "  dependency: {} ({})",
                            dependency["projectId"]
                                .as_str()
                                .or_else(|| dependency["versionId"].as_str())
                                .unwrap_or("unknown"),
                            dependency["dependencyType"].as_str().unwrap_or("unknown")
                        );
                    }
                }
                println!(
                    "Choose a compatible version id, then run addon install-catalog PROJECT --version-id ID --confirm."
                );
            }
            Ok(())
        }
        AddonCommand::InstallCatalog {
            project_id,
            version_id,
            confirm: _,
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
                        version_id: Some(version_id),
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
                        version_id: None,
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
                        check_addon_updates: None,
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
                        check_addon_updates: None,
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
                        check_addon_updates: None,
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
                        check_addon_updates: None,
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
                        check_addon_updates: None,
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
                        check_addon_updates: None,
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
                        check_addon_updates: None,
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
    let client = ApiClient::connect_local().await?;
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
        ModpackCommand::Import {
            path,
            target_server,
            confirm: _,
            no_wait,
        } => {
            let target = ensure_active_server(&client, Some(&target_server)).await?;
            ensure_modpack_provider_ready(&client, &path).await?;
            if !common.json {
                println!("target server: {} ({})", target.name, target.id);
                println!("the archive will add pack-managed files and metadata to this server");
            }
            let result = import_modpack_command(&client, &path, "import").await?;
            if common.json {
                print_json(
                    &serde_json::json!({"targetServer": {"id": target.id, "name": target.name}, "action": "import", "result": result}),
                )?;
            } else {
                println!("{}", result.message);
                print_manual_files(&result.pending_manual_files);
            }
            finish_operation(
                &client,
                common.json,
                no_wait || !result.pending_manual_files.is_empty(),
                Some(result.operation_id),
                "modpack import",
            )
            .await
        }
        ModpackCommand::Replace {
            path,
            target_server,
            confirm: _,
            no_wait,
        } => {
            let target = ensure_active_server(&client, Some(&target_server)).await?;
            ensure_modpack_provider_ready(&client, &path).await?;
            if !common.json {
                println!("target server: {} ({})", target.name, target.id);
                println!("this replaces the server's current pack-managed content");
            }
            let result = import_modpack_command(&client, &path, "replace").await?;
            if common.json {
                print_json(
                    &serde_json::json!({"targetServer": {"id": target.id, "name": target.name}, "action": "replace", "result": result}),
                )?;
            } else {
                println!("{}", result.message);
                print_manual_files(&result.pending_manual_files);
            }
            finish_operation(
                &client,
                common.json,
                no_wait || !result.pending_manual_files.is_empty(),
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
                        staged_upload_id: Some(staged_upload_id),
                        action: "upload".to_string(),
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
        ModpackCommand::Cancel { operation_id } => {
            let result: serde_json::Value = client
                .post_json(
                    &format!("/v1/operations/{operation_id}/cancel"),
                    &serde_json::json!({}),
                )
                .await?;
            if common.json {
                print_json(&result)
            } else {
                println!("Cancellation requested for modpack operation {operation_id}.");
                Ok(())
            }
        }
    }
}

async fn import_modpack_command(
    client: &ApiClient,
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

fn print_target_world(
    slot: &serde_json::Value,
    fallback_id: &str,
    json_mode: bool,
) -> Result<(), CliError> {
    if !json_mode {
        println!(
            "target world: {} ({})",
            slot["name"].as_str().unwrap_or("unknown"),
            slot["id"].as_str().unwrap_or(fallback_id)
        );
    }
    Ok(())
}

fn print_catalog_value(json_mode: bool, value: &serde_json::Value) -> Result<(), CliError> {
    if json_mode {
        print_json(value)
    } else {
        if let Some(results) = value["results"].as_array() {
            if results.is_empty() {
                println!("No compatible packs found.");
            }
            for item in results {
                println!(
                    "{}  {}",
                    item["projectId"]
                        .as_str()
                        .or_else(|| item["slug"].as_str())
                        .unwrap_or("?"),
                    item["title"].as_str().unwrap_or("Untitled")
                );
                if let Some(description) = item["description"].as_str() {
                    println!("  {description}");
                }
            }
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(value).unwrap_or_default()
            );
        }
        Ok(())
    }
}
fn value_list(value: &serde_json::Value) -> String {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}
fn print_modpack_inspection(result: &ModpackInspectionResultDto) {
    println!("format: {}", result.format);
    if let Some(name) = &result.pack_name {
        println!("pack: {name}");
    }
    if let Some(version) = &result.pack_version {
        println!("version: {version}");
    }
    if let Some(version) = &result.minecraft_version {
        println!("Minecraft: {version}");
    }
    if let Some(loader) = &result.loader_name {
        println!(
            "loader: {} {}",
            loader,
            result.loader_version.as_deref().unwrap_or("")
        );
    }
    println!("files: {}", result.file_count);
    println!("client-only files: {}", result.client_only_file_count);
    println!("override files: {}", result.override_file_count);
    if let Some(available) = result.curseforge_lookup_available {
        println!("CurseForge file lookup available: {available}");
    }
    for warning in &result.warnings {
        println!("warning: {warning}");
    }
    if !result.manual_files.is_empty() {
        println!("manual files:");
        for file in &result.manual_files {
            println!(
                "  {} — {} [{}]",
                file.file_name, file.project_name, file.file_id
            );
            if let Some(reason) = &file.reason {
                println!("    reason: {reason}");
            }
            if let Some(url) = &file.project_url {
                println!("    obtain the exact file: {url}");
            }
        }
    }
}

fn print_manual_files(files: &[msc_api::dto::ModpackManualFileDto]) {
    if files.is_empty() {
        return;
    }
    println!("author-blocked files need a local download; the import operation is waiting:");
    for file in files {
        println!("  expected file: {} (id {})", file.file_name, file.file_id);
        if let Some(reason) = &file.reason {
            println!("    reason: {reason}");
        }
        if let Some(url) = &file.project_url {
            println!("    download page: {url}");
        }
        println!(
            "    resume: msc modpack manual-file OPERATION_ID {} /path/to/{}",
            file.file_id, file.file_name
        );
    }
}

async fn ensure_modpack_provider_ready(client: &ApiClient, path: &Path) -> Result<(), CliError> {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        let status: serde_json::Value = client.get_json("/v1/config/curseforge").await?;
        if status["configured"].as_bool() != Some(true) {
            return Err(CliError::usage(
                "CurseForge import needs an API key. Set it without exposing it in shell history: msc config set-curse-forge-key --key-stdin < protected-input",
            ));
        }
    }
    Ok(())
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

async fn ensure_active_server(
    client: &ApiClient,
    selector: Option<&str>,
) -> Result<ServerDto, CliError> {
    session::ensure_active_server(client, selector).await
}

async fn resolve_server(client: &ApiClient, selector: &str) -> Result<ServerDto, CliError> {
    session::resolve_server(client, selector).await
}

async fn wait_for_stopped(client: &ApiClient) -> Result<(), CliError> {
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

fn print_json<T: Serialize>(value: &T) -> Result<(), CliError> {
    let text = serde_json::to_string(value)
        .map_err(|err| CliError::internal(format!("failed to encode JSON output: {err}")))?;
    println!("{text}");
    Ok(())
}

fn print_status(status: &RemoteApiStatus) {
    let state = if status.running { "RUNNING" } else { "STOPPED" };
    println!("server status: {state}");
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

fn print_server_detail(server: &ServerDto) {
    println!("{} ({})", server.name, server.id);
    println!("type: {}", server.server_type);
    println!("directory: {}", server.directory);
    if let Some(flavor) = &server.java_flavor {
        println!("Java flavor: {flavor}");
    }
    if let Some(port) = server.game_port {
        println!("game port: {port}");
    }
    if let Some(port) = server.bedrock_port {
        println!("Bedrock port: {port}");
    }
    println!(
        "notes: {}",
        if server.notes.is_empty() {
            "(none)"
        } else {
            &server.notes
        }
    );
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
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
