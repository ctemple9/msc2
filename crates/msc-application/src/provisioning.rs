//! P7.17: provision the four download-and-go Java server families
//! (Vanilla, Paper, Purpur, Fabric) end to end. P7.18: provision the two
//! install-step families (Forge, NeoForge) as a cancellable operation.
//!
//! Both port `AppViewModel+ServerCreation.swift::createNewServer`
//! (`create_download_and_go_server` the `else` branch starting source
//! line 240, `create_install_step_server` the `if
//! flavor.provisioningKind == .installStep` branch starting line 194),
//! sharing everything from `eula.txt` onward through
//! [`finish_server_creation`] — see that function's own doc.
//! `createInitialPersistentWorldSlot` (line 65-123) and the world-source
//! dispatch (`unzipWorldBackup`/`copyExistingWorldFolder`,
//! `AppViewModel+WorldManagement.swift:276-312`) are shared the same way,
//! against all 24 `fixtures/server-creation/` cases.
//!
//! **One deliberate strengthening over the oracle**, per this phase's own
//! working exit criteria ("every failed create rolls its directory back
//! completely, leaving no half-provisioned server behind") and per
//! P7.1/P7.6's own flag that this gap was "left for P7.17/P7.18 to
//! close": source's two `WorldSource` copy-failure paths
//! (`fixtures/server-creation/world-source-backup-zip-failure-aborts-
//! returns-false.json`, `.../world-source-existing-folder-failure-aborts-
//! returns-false.json`) return `false` with **no** directory cleanup and
//! **no** error message. This port instead rolls the whole `new_dir` back
//! on *every* failure path once it exists — the same single `catch`-style
//! wrapper source's own top-level `catch` already applies to every other
//! failure in the function, just applied without the source's one
//! documented gap.
//!
//! **Registering the new server into the fleet is not this module's
//! job.** Source's own `upsertServer(cfgServer)` / `setActiveServer` /
//! `replayCreationConsole` calls (line 382, 388, 391) are cross-cutting
//! `AppConfig` and client-console concerns this module has no access to —
//! every other `msc-application` module in this phase operates on one
//! server's own directory, never the whole fleet registry. This module
//! returns the fully-built [`msc_domain::app_config_schema::ConfigServer`]
//! and its initial [`WorldSlot`]; inserting them into `AppConfig.servers`
//! and saving is the route layer's job (P7.23).
//!
//! **`recordLoaderVersion`'s actual persistence is not built anywhere in
//! this codebase yet** (no P7 step's `Files:` list names a "loader
//! version history" store) — [`CreatedServer::should_record_loader_version`]
//! exposes the already-ported P7.12 condition so a future caller can act
//! on it, but this step does not invent the write target itself.

use msc_domain::app_config_schema::ConfigServer;
use msc_domain::bedrock::render_raw_properties;
use msc_domain::identity::{JavaServerFlavor, ServerType};
use msc_domain::modpack_manifest;
use msc_domain::provisioning::{self, ImportedWorldMetadata};
use msc_domain::world::{self, WorldSlot};
use msc_domain::{nbt, server_versions};
use msc_infrastructure::addon_provider::AddonTransport;
use msc_infrastructure::archive::{self, ArchiveError};
use msc_infrastructure::atomic_write::{AtomicWriteError, atomic_write};
use msc_infrastructure::fs::{FileSystem, join_forward_slash};
use msc_infrastructure::jar_provider::{self, JarProviderError, Transport};
use msc_infrastructure::java_runtime_detection;
use msc_infrastructure::loader_installer::{
    self, LoaderInstallRequest, LoaderInstallerError, LoaderTarget,
};
use msc_infrastructure::path_safety::{self, PathSafetyError};
use msc_infrastructure::process::{OutputStream, ProcessSupervisor};
use msc_infrastructure::secret_store::SecretStore;
use msc_infrastructure::template_store::{self, TemplateStoreError};
use msc_infrastructure::world_store;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::modpacks::{self, ModpackInspection};
use crate::worlds::{self, WorldError};

/// `AppViewModel.WorldSource` (`AppViewModel+ServerCreation.swift:12-16`).
#[derive(Debug, Clone)]
pub enum WorldSource<'a> {
    Fresh,
    BackupZip(&'a Path),
    ExistingFolder(&'a Path),
}

#[derive(Debug)]
pub enum CreateServerError {
    /// `guard !safeName.isEmpty else { return false }` (line 147).
    EmptyName,
    /// `fm.fileExists(atPath: newDir.path)` (line 178) — checked before
    /// `newDir` is created, so nothing is rolled back.
    FolderAlreadyExists {
        folder_name: String,
        path: PathBuf,
    },
    /// This module only provisions the four download-and-go families.
    UnsupportedFlavor(JavaServerFlavor),
    Download(JarProviderError),
    TemplateStore(TemplateStoreError),
    PathSafety(PathSafetyError),
    Io(std::io::Error),
    Archive(ArchiveError),
    /// One of `unzip_world_backup`/`copy_existing_world_folder` returned
    /// `false` — see this module's own doc for the deliberate rollback
    /// strengthening applied here that source itself does not perform.
    WorldSourceFailed,
    /// `createInitialPersistentWorldSlot` returned `nil` (line 356-367).
    InitialWorldSlotFailed,
    WorldSlot(WorldError),
    /// `NeoForgeError.installerFailed`/`.argsFileMissing`, or a process
    /// spawn/timeout failure — every non-`NonZeroExit`/`ArgsFileNotProduced`
    /// `LoaderInstallerError` variant collapses source's own three
    /// distinct thrown errors into this one wrapper, matching the same
    /// "one typed error, not three" precedent [`WorldSlot`] already sets.
    LoaderInstaller(LoaderInstallerError),
    /// `should_cancel` reported true before the installer's own polling
    /// loop had a chance to observe it — checked at entry and again
    /// immediately before the installer subprocess starts, the same two
    /// "nothing long-running touched yet" boundaries
    /// `world_conversion::convert_world` already uses.
    Cancelled,
    /// P7.31: the required-major Java guard refused the resolved
    /// executable before the loader installer (Forge/NeoForge only —
    /// download-and-go never spawns Java at create time, so this variant
    /// is only ever raised by [`create_install_step_server`]) got a
    /// chance to run against it and produce an opaque process failure
    /// instead.
    UnusableJavaRuntime(msc_domain::java_runtime::UnusableJavaRuntime),
}

impl fmt::Display for CreateServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CreateServerError::EmptyName => write!(f, "server name is empty"),
            CreateServerError::FolderAlreadyExists { folder_name, path } => write!(
                f,
                "A server folder named \"{folder_name}\" already exists at {}. Choose a different name, or remove that folder.",
                path.display()
            ),
            CreateServerError::UnsupportedFlavor(flavor) => {
                write!(
                    f,
                    "{} is not provisioned by this workflow",
                    flavor.raw_value()
                )
            }
            CreateServerError::Download(e) => write!(f, "{e}"),
            CreateServerError::TemplateStore(e) => write!(f, "{e}"),
            CreateServerError::PathSafety(e) => write!(f, "{e}"),
            CreateServerError::Io(e) => write!(f, "{e}"),
            CreateServerError::Archive(e) => write!(f, "{e}"),
            CreateServerError::WorldSourceFailed => {
                write!(f, "failed to install the requested world data")
            }
            CreateServerError::InitialWorldSlotFailed => {
                write!(f, "Couldn't create the initial world slot.")
            }
            CreateServerError::WorldSlot(e) => write!(f, "{e}"),
            CreateServerError::LoaderInstaller(e) => write!(f, "{e}"),
            CreateServerError::Cancelled => write!(f, "creation was cancelled"),
            CreateServerError::UnusableJavaRuntime(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CreateServerError {}

impl From<JarProviderError> for CreateServerError {
    fn from(e: JarProviderError) -> Self {
        CreateServerError::Download(e)
    }
}

impl From<TemplateStoreError> for CreateServerError {
    fn from(e: TemplateStoreError) -> Self {
        CreateServerError::TemplateStore(e)
    }
}

impl From<PathSafetyError> for CreateServerError {
    fn from(e: PathSafetyError) -> Self {
        CreateServerError::PathSafety(e)
    }
}

impl From<std::io::Error> for CreateServerError {
    fn from(e: std::io::Error) -> Self {
        CreateServerError::Io(e)
    }
}

impl From<ArchiveError> for CreateServerError {
    fn from(e: ArchiveError) -> Self {
        CreateServerError::Archive(e)
    }
}

impl From<WorldError> for CreateServerError {
    fn from(e: WorldError) -> Self {
        CreateServerError::WorldSlot(e)
    }
}

impl From<LoaderInstallerError> for CreateServerError {
    fn from(e: LoaderInstallerError) -> Self {
        CreateServerError::LoaderInstaller(e)
    }
}

impl From<msc_domain::java_runtime::UnusableJavaRuntime> for CreateServerError {
    fn from(e: msc_domain::java_runtime::UnusableJavaRuntime) -> Self {
        CreateServerError::UnusableJavaRuntime(e)
    }
}

/// Runs P7.31's required-major guard against `java_executable_path`,
/// probing it for real through `supervisor` (the same handle the loader
/// installer itself spawns against) rather than trusting it unchecked.
/// `Ok(Some(warning))` is the above-required-but-Java-17-era case;
/// callers thread it into [`CreatedServer::java_compatibility_warning`]
/// rather than blocking on it.
pub(crate) fn check_java_runtime_guard(
    supervisor: &dyn ProcessSupervisor,
    java_executable_path: &str,
    minecraft_version: Option<&str>,
) -> Result<Option<String>, CreateServerError> {
    let probe = java_runtime_detection::run_java_version_probe(supervisor, java_executable_path);
    msc_domain::java_runtime::evaluate_java_runtime_guard(
        java_executable_path,
        minecraft_version,
        &probe,
    )
    .map_err(CreateServerError::from)
}

/// The operation-journal `operation_type` every real server-create
/// operation is journaled under (`routes/servers.rs`'s own
/// `begin_lifecycle("server-create", ...)` call). Shared here, rather
/// than left as a bare string literal on both sides, so
/// `msc_application::operations::LifecycleOperations::reconcile_on_startup`'s
/// P7.33 orphaned-directory sweep recognizes a reconciled entry as this
/// domain's without the two call sites being able to drift apart.
pub const CREATE_OPERATION_TYPE: &str = "server-create";

/// Atomically claims `new_dir` for a brand-new server, closing the
/// check-then-create race `docs/msc2/families/phase7-scope.md`'s P7.1
/// note flagged and P7.30's gate audit confirmed was still open: the
/// previous `fs.stat` (check) then `fs.create_dir_all` (create) were two
/// separate filesystem calls, so two concurrent creates of the same
/// server name could both observe "nothing there yet" before either had
/// created anything. `fs.create_dir_exclusive` is the single call that
/// actually makes the claim — only one caller can ever see it succeed
/// for a given `new_dir`; the loser gets the same
/// [`CreateServerError::FolderAlreadyExists`] this function always
/// returned for the non-racing case, not a new error shape.
fn claim_new_server_directory(
    fs: &dyn FileSystem,
    new_dir: &Path,
    folder_name: &str,
) -> Result<(), CreateServerError> {
    if let Some(parent) = new_dir.parent() {
        fs.create_dir_all(parent)?;
    }
    fs.create_dir_exclusive(new_dir).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            CreateServerError::FolderAlreadyExists {
                folder_name: folder_name.to_string(),
                path: new_dir.to_path_buf(),
            }
        } else {
            CreateServerError::Io(error)
        }
    })
}

/// Best-effort removal of a `"server-create"` operation's half-provisioned
/// server directory, called by
/// [`crate::operations::LifecycleOperations::reconcile_on_startup`] once
/// it reconciles that operation to `Failed` after an interrupted install
/// (agent killed or crashed mid-create, so the operation journal was left
/// `running`). Before P7.33 this case was reconciled at the *operation*
/// level only — `<servers_root>/java/<folder_name>` itself was never
/// swept, the second of the two gaps this step's own scope note named. A
/// create that fails in-process already rolls its own directory back
/// (`create_download_and_go_server`/`create_install_step_server`'s own
/// `let _ = fs.remove(&new_dir);`) before this could ever run; this sweep
/// only ever finds something to remove when the process died before that
/// cleanup had a chance to. Silently does nothing if the directory is
/// already gone, for exactly that reason — not a symptom worth
/// surfacing.
pub fn sweep_orphaned_server_directory(
    fs: &dyn FileSystem,
    servers_root: &Path,
    folder_name: &str,
) {
    let dir = join_forward_slash(
        &join_forward_slash(servers_root, std::ffi::OsStr::new("java")),
        std::ffi::OsStr::new(folder_name),
    );
    let _ = fs.remove(&dir);
}

/// `AppViewModel.createNewServer`'s parameters, minus `specificVersion`
/// (no `fixtures/server-creation` case exercises pinning a non-latest
/// version at create time — this module only builds "download latest,"
/// matching the `Not in this phase`/scope notes this phase's own
/// preamble draws elsewhere rather than adding untested surface) and
/// `stagedAddOns` (Phase 8, per this phase's own "Not in this phase"
/// list).
#[derive(Debug, Clone)]
pub struct NewServerRequest<'a> {
    pub name: &'a str,
    pub initial_world_name: Option<&'a str>,
    /// [`create_download_and_go_server`] refuses any flavor whose
    /// `provisioning_kind()` isn't `DownloadAndGo`; the install-step
    /// families (Forge, NeoForge) go through [`create_install_step_server`]
    /// instead, which refuses the reverse.
    pub flavor: JavaServerFlavor,
    pub port: u16,
    pub enable_cross_play: bool,
    pub cross_play_bedrock_port: Option<u16>,
    pub enable_playit: bool,
    pub enable_xbox_broadcast: bool,
    pub difficulty: &'a str,
    pub gamemode: &'a str,
    pub world_seed: Option<&'a str>,
    pub world_source: WorldSource<'a>,
    pub save_downloaded_jars: bool,
    pub default_banner_color_hex: &'a str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatedServer {
    pub config: ConfigServer,
    pub world_slot: WorldSlot,
    /// The already-ported P7.12 `should_record_loader_version` condition
    /// — see this module's own doc for why the write itself isn't built
    /// here.
    pub should_record_loader_version: bool,
    /// P7.31's required-major guard's own above-required-but-Java-17-era
    /// warning (`msc_domain::java_runtime::compatibility_warning_text`),
    /// when the resolved executable triggered one. `None` on a clean bill
    /// of health — set by each caller after [`finish_server_creation`]
    /// returns, not by that shared tail itself (it has no java-executable
    /// argument to run the guard's own probe against).
    pub java_compatibility_warning: Option<String>,
}

/// The three world-source choices shared by MSC 1's Bedrock creation flow.
#[derive(Debug, Clone)]
pub enum BedrockWorldSource<'a> {
    Fresh,
    BackupZip(&'a Path),
    ExistingFolder(&'a Path),
}

#[derive(Debug, Clone)]
pub struct BedrockCreateRequest<'a> {
    pub name: &'a str,
    pub initial_world_name: Option<&'a str>,
    pub bedrock_version: Option<&'a str>,
    pub port: u16,
    pub max_players: i64,
    pub enable_playit: bool,
    pub enable_xbox_broadcast: bool,
    pub difficulty: &'a str,
    pub gamemode: &'a str,
    pub world_seed: Option<&'a str>,
    pub world_source: BedrockWorldSource<'a>,
}

#[derive(Debug)]
pub enum BedrockCreateError {
    EmptyName,
    EmptyDestinationName,
    FolderAlreadyExists { path: PathBuf },
    InvalidWorldSource { path: PathBuf, reason: String },
    Archive(ArchiveError),
    WorldSlot(WorldError),
    AtomicWrite(AtomicWriteError),
    Io(std::io::Error),
}

impl fmt::Display for BedrockCreateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => f.write_str("server name is empty"),
            Self::EmptyDestinationName => f.write_str("server name produces an empty folder name"),
            Self::FolderAlreadyExists { path } => {
                write!(
                    f,
                    "a Bedrock server folder already exists at {}",
                    path.display()
                )
            }
            Self::InvalidWorldSource { path, reason } => {
                write!(
                    f,
                    "invalid Bedrock world source {}: {reason}",
                    path.display()
                )
            }
            Self::Archive(error) => write!(f, "{error}"),
            Self::WorldSlot(error) => write!(f, "{error}"),
            Self::AtomicWrite(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for BedrockCreateError {}

impl From<ArchiveError> for BedrockCreateError {
    fn from(error: ArchiveError) -> Self {
        Self::Archive(error)
    }
}

impl From<WorldError> for BedrockCreateError {
    fn from(error: WorldError) -> Self {
        Self::WorldSlot(error)
    }
}

impl From<AtomicWriteError> for BedrockCreateError {
    fn from(error: AtomicWriteError) -> Self {
        Self::AtomicWrite(error)
    }
}

impl From<std::io::Error> for BedrockCreateError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatedBedrockServer {
    pub config: ConfigServer,
    pub world_slot: WorldSlot,
}

/// What a jar acquisition resolved to — mirrors `ServerJarDownloadResult`
/// (`ServerJarProviders.swift:42-46`). `pub(crate)`: [`download_flavor_jar`]
/// is now a same-crate `pub(crate)` boundary too (P7.19's `server_versions`
/// module is its second caller), and a `pub(crate)` function can't return
/// a private type.
pub(crate) struct ResolvedJar {
    pub(crate) version: String,
    pub(crate) build: String,
    pub(crate) loader_version: Option<String>,
}

/// `initialWorldSlotName(forServerName:requestedWorldName:)`
/// (`AppViewModel+ServerCreation.swift:18-24`). `pub(crate)`: P7.21's
/// `templates::create_server_from_template` needs the identical
/// slot-name derivation for its own, jar-source-swapped call into
/// [`finish_server_creation`].
pub(crate) fn initial_world_slot_name(
    server_name: &str,
    requested_world_name: Option<&str>,
) -> String {
    let requested_trimmed = requested_world_name.unwrap_or("").trim();
    if !requested_trimmed.is_empty() {
        return requested_trimmed.to_string();
    }
    let server_trimmed = server_name.trim();
    if server_trimmed.is_empty() {
        "World 1".to_string()
    } else {
        server_trimmed.to_string()
    }
}

/// `normalizedInitialWorldSeed(_:worldSource:)` (line 26-30): `None` for
/// every source but `.fresh`. `pub(crate)` for the same P7.21 reuse
/// reason as [`initial_world_slot_name`].
pub(crate) fn normalized_initial_world_seed(
    seed: Option<&str>,
    world_source: &WorldSource,
) -> Option<String> {
    if !matches!(world_source, WorldSource::Fresh) {
        return None;
    }
    let trimmed = seed.unwrap_or("").trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// `importedWorldMetadata(fromFolder:serverType:)` (`WorldSlotManager
/// .swift:1271-1275`), against a real, already-selected folder on disk —
/// matching `world_conversion.rs`'s established precedent that a
/// caller-selected filesystem location this specific (not something a
/// `FakeFileSystem` fixture tree stands in for) is read via real
/// `std::fs`, not the injectable [`FileSystem`] trait.
fn imported_metadata_from_folder(folder: &Path, server_type: ServerType) -> ImportedWorldMetadata {
    let parsed = std::fs::read(folder.join("level.dat"))
        .ok()
        .map(|bytes| nbt::imported_world_metadata_from_level_dat(&bytes, server_type))
        .unwrap_or_default();
    ImportedWorldMetadata {
        difficulty: parsed.difficulty,
        gamemode: parsed.gamemode,
        seed: parsed.seed,
    }
}

/// `importedWorldMetadata(fromZIP:serverType:)` (`WorldSlotManager.swift:
/// 1260-1269`): a non-blank sidecar seed wins over a parsed `level.dat`
/// seed; difficulty/gamemode always come from the parse (the sidecar
/// this codebase writes, `BackupMeta`, never carries either — see
/// [`nbt::merge_sidecar_metadata`]'s own doc).
fn imported_metadata_from_zip(zip_path: &Path, server_type: ServerType) -> ImportedWorldMetadata {
    let sidecar_seed = worlds::read_sidecar_world_seed(zip_path);
    let parsed = archive::list_entry_names(zip_path)
        .ok()
        .and_then(|listing| {
            let refs: Vec<&str> = listing.iter().map(String::as_str).collect();
            nbt::first_level_dat_path(&refs)
        })
        .and_then(|member| archive::read_entry_bytes(zip_path, &member).ok().flatten())
        .map(|bytes| nbt::imported_world_metadata_from_level_dat(&bytes, server_type))
        .unwrap_or_default();
    let merged = nbt::merge_sidecar_metadata(sidecar_seed, parsed);
    ImportedWorldMetadata {
        difficulty: merged.difficulty,
        gamemode: merged.gamemode,
        seed: merged.seed,
    }
}

/// `PaperVersionSidecarManager.write` (`PaperVersionSidecar.swift:35-49`):
/// intentionally best-effort, matching source's own empty `catch` — a
/// failure here must never break server creation. `now` is an already-
/// formatted timestamp (the ISO-8601-`now`-threading convention every
/// other function in this phase already uses), not computed internally.
/// `pub(crate)`: P7.21's `templates::create_server_from_template` writes
/// this same sidecar for its own `.template(url)` jar-source branch
/// (`AppViewModel+ServerCreation.swift:249-253`).
pub(crate) fn write_paper_version_sidecar(
    fs: &dyn FileSystem,
    server_dir: &Path,
    mc_version: &str,
    build: i64,
    now: &str,
) {
    let value = serde_json::json!({
        "build": build,
        "mcVersion": mc_version,
        "timestamp": now,
    });
    if let Ok(bytes) = serde_json::to_vec_pretty(&value) {
        let _ = fs.write(&server_dir.join(".msc_paper_version.json"), &bytes);
    }
}

/// The Paper archive-first shortcut's hit path
/// (`AppViewModel+ServerCreation.swift:258-271`). `None` on a metadata-
/// fetch failure (swallowed by source's own `try?`) or an archive miss —
/// either way the caller falls through to a real download, matching
/// source's `if !usedArchive` continuation exactly.
fn try_paper_archive_hit(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    home_dir: &Path,
    paper_template_dir: &Path,
    jar_dest: &Path,
    now: &str,
) -> Option<ResolvedJar> {
    let (version, selection) = jar_provider::paper_resolve_latest_stable(transport)
        .ok()
        .flatten()?;
    let archive_filename = format!("paper-{version}-build{}.jar", selection.build_id);
    let archive_path =
        path_safety::safe_path(fs, paper_template_dir, Some(&archive_filename), home_dir).ok()?;
    if fs.stat(&archive_path).is_err() {
        return None;
    }
    let bytes = fs.read(&archive_path).ok()?;
    fs.write(jar_dest, &bytes).ok()?;
    let server_dir = jar_dest.parent().unwrap_or(jar_dest);
    write_paper_version_sidecar(fs, server_dir, &version, selection.build_id, now);
    Some(ResolvedJar {
        version,
        build: selection.build_id.to_string(),
        loader_version: None,
    })
}

/// The real (non-archived) `ServerJarProvider.downloadLatest` dispatch
/// (`ServerJarProviders.swift:98-117`), for the four download-and-go
/// families this module provisions. `pub(crate)` (not `pub`) because
/// P7.19's `server_versions::change_version` is a second, same-crate
/// caller — source's own `changeVersionProvider` and `createNewServer`
/// call the identical `ServerJarProvider.downloadLatest(flavor:to:)`
/// dispatcher, so this port shares the one Rust composition rather than
/// duplicating the Purpur-alignment/Fabric-loader-resolution logic a
/// second time.
pub(crate) fn download_flavor_jar(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    flavor: JavaServerFlavor,
    jar_dest: &Path,
) -> Result<ResolvedJar, CreateServerError> {
    match flavor {
        JavaServerFlavor::Vanilla => {
            let cached = jar_provider::vanilla_download_latest(transport, fs, jar_dest)?;
            Ok(ResolvedJar {
                version: cached.version,
                build: "release".to_string(),
                loader_version: None,
            })
        }
        JavaServerFlavor::Purpur => {
            let raw_versions = jar_provider::purpur_raw_version_list(transport)?;
            let paper_stable = jar_provider::paper_resolve_latest_stable(transport)
                .ok()
                .flatten()
                .map(|(version, _)| version);
            let target =
                server_versions::purpur_pick_target_version(&raw_versions, paper_stable.as_deref())
                    .ok_or_else(|| {
                        CreateServerError::Download(JarProviderError::Network(
                            "No Purpur versions found.".to_string(),
                        ))
                    })?;
            let build = jar_provider::purpur_latest_build_label(transport, &target)?;
            jar_provider::purpur_download_version(transport, fs, &target, jar_dest)?;
            Ok(ResolvedJar {
                version: target,
                build,
                loader_version: None,
            })
        }
        JavaServerFlavor::Paper => {
            let (version, selection) = jar_provider::paper_resolve_latest_stable(transport)?
                .ok_or_else(|| {
                    CreateServerError::Download(JarProviderError::Network(
                        "No stable Paper builds found.".to_string(),
                    ))
                })?;
            jar_provider::paper_download_build(
                transport,
                fs,
                &version,
                selection.build_id,
                jar_dest,
            )?;
            Ok(ResolvedJar {
                version,
                build: selection.build_id.to_string(),
                loader_version: None,
            })
        }
        JavaServerFlavor::Fabric => {
            let game = jar_provider::fabric_latest_stable_game_version(transport)?;
            let loader = jar_provider::fabric_resolve_loader(transport, &game)?;
            jar_provider::fabric_download_version(transport, fs, &game, Some(&loader), jar_dest)?;
            Ok(ResolvedJar {
                version: game,
                build: format!("fabric {loader}"),
                loader_version: Some(loader),
            })
        }
        other => Err(CreateServerError::UnsupportedFlavor(other)),
    }
}

/// The whole non-install-step jar acquisition step (source lines 240-
/// 293): the Paper archive-first shortcut, then a real download,
/// archiving the freshly-downloaded jar afterward when
/// `save_downloaded_jars` is set (source line 289-291) — matching
/// `archiveServerJar`'s own silent-skip behavior for every flavor this
/// module doesn't handle, which never applies here since this function
/// only ever calls [`download_flavor_jar`] for the four it does.
#[allow(clippy::too_many_arguments)]
fn acquire_jar(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    home_dir: &Path,
    paper_template_dir: &Path,
    flavor: JavaServerFlavor,
    jar_dest: &Path,
    save_downloaded_jars: bool,
    now: &str,
) -> Result<ResolvedJar, CreateServerError> {
    if flavor == JavaServerFlavor::Paper
        && save_downloaded_jars
        && let Some(hit) =
            try_paper_archive_hit(transport, fs, home_dir, paper_template_dir, jar_dest, now)
    {
        return Ok(hit);
    }

    let resolved = download_flavor_jar(transport, fs, flavor, jar_dest)?;

    if flavor == JavaServerFlavor::Paper
        && let Ok(build_int) = resolved.build.parse::<i64>()
    {
        let server_dir = jar_dest.parent().unwrap_or(jar_dest);
        write_paper_version_sidecar(fs, server_dir, &resolved.version, build_int, now);
    }

    if save_downloaded_jars {
        let _ = template_store::archive_jar(
            fs,
            paper_template_dir,
            home_dir,
            flavor,
            &resolved.version,
            &resolved.build,
            jar_dest,
        )?;
    }

    Ok(resolved)
}

/// The initial persistent world-slot creation dispatch
/// (`createInitialPersistentWorldSlot`, line 65-123), reusing Phase 6's
/// already-built slot constructors per this step's own `What:` line.
/// Returns the built [`WorldSlot`] plus the fresh-world seed retained for
/// callers that still need to distinguish a generated slot from imported
/// data. The actual runtime projection is applied through
/// [`worlds::apply_world_profile`] after this dispatch completes.
#[allow(clippy::too_many_arguments)]
fn create_initial_world_slot(
    fs: &dyn FileSystem,
    server_dir: &Path,
    slot_name: &str,
    initial_level_name: &str,
    world_source: &WorldSource,
    effective_world_seed: Option<&str>,
    imported_seed: Option<&str>,
    now: &str,
) -> Result<(WorldSlot, Option<String>), CreateServerError> {
    match world_source {
        WorldSource::Fresh => {
            let id = uuid::Uuid::new_v4().to_string().to_uppercase();
            let slot = world::build_fresh_slot(
                id,
                slot_name,
                effective_world_seed,
                ServerType::Java,
                now.to_string(),
            );
            world_store::save_metadata(fs, server_dir, &slot).map_err(|e| match e {
                msc_infrastructure::atomic_write::AtomicWriteError::Io(e) => {
                    CreateServerError::Io(e)
                }
                other => CreateServerError::Io(std::io::Error::other(other.to_string())),
            })?;
            let seed = slot.world_seed.clone();
            Ok((slot, seed))
        }
        WorldSource::BackupZip(zip_path) => {
            // `raw_level_name` only feeds `import_zip_as_new_slot`'s
            // Bedrock branch (Java infers its level-name from the zip's
            // own contents instead) — passed anyway for consistency with
            // the `ExistingFolder` arm below, where it is load-bearing.
            let slot = worlds::import_zip_as_new_slot(
                fs,
                server_dir,
                ServerType::Java,
                Some(initial_level_name),
                zip_path,
                slot_name,
                now,
            )?;
            Ok((slot, None))
        }
        WorldSource::ExistingFolder(_) => {
            // `raw_level_name` here must be the exact level-name
            // `copy_existing_world_folder` just copied the source folder
            // into (`initial_level_name`) — `create_slot_from_current_world`
            // zips whatever live folder is named after it, and a wrong
            // guess (e.g. the `current_level_name` fallback of `"world"`)
            // would zip nothing at all.
            let slot = worlds::create_slot_from_current_world(
                fs,
                server_dir,
                ServerType::Java,
                Some(initial_level_name),
                slot_name,
                imported_seed,
                now,
            )?;
            Ok((slot, None))
        }
    }
}

/// Provisions a new Vanilla, Paper, Purpur, or Fabric server end to end.
///
/// `unzip_world_backup`/`copy_existing_world_folder` are the fakeable
/// boundary over `AppViewModel+WorldManagement.swift`'s two real-disk
/// operations (`unzipWorldBackup`/`copyExistingWorldFolder`, line 276-
/// 312) — the same "closure over a risky/external mechanism" shape
/// `world_conversion::convert_world`'s `pre_conversion_backup` parameter
/// already established. [`real_unzip_world_backup`]/
/// [`real_copy_existing_world_folder`] are this module's own production
/// implementations for callers (the P7.23 route layer) that want the
/// real behavior rather than a test double.
#[allow(clippy::too_many_arguments)]
pub fn create_download_and_go_server(
    fs: &dyn FileSystem,
    transport: &dyn Transport,
    home_dir: &Path,
    servers_root: &Path,
    paper_template_dir: &Path,
    plugin_template_dir: &Path,
    request: &NewServerRequest,
    now: &str,
    unzip_world_backup: impl FnOnce(&Path, &Path) -> bool,
    copy_existing_world_folder: impl FnOnce(&Path, &Path, &str) -> bool,
) -> Result<CreatedServer, CreateServerError> {
    let safe_name =
        provisioning::trimmed_server_name(request.name).ok_or(CreateServerError::EmptyName)?;

    let initial_slot_name = initial_world_slot_name(&safe_name, request.initial_world_name);
    let normalized_world_seed =
        normalized_initial_world_seed(request.world_seed, &request.world_source);
    let imported_metadata = match &request.world_source {
        WorldSource::Fresh => ImportedWorldMetadata::default(),
        WorldSource::BackupZip(zip_path) => imported_metadata_from_zip(zip_path, ServerType::Java),
        WorldSource::ExistingFolder(folder) => {
            imported_metadata_from_folder(folder, ServerType::Java)
        }
    };
    let effective = provisioning::effective_world_settings(
        request.difficulty,
        request.gamemode,
        normalized_world_seed.as_deref(),
        &imported_metadata,
    );
    let initial_level_name = world::sanitized_world_level_name(&initial_slot_name, "world");

    let folder_name = provisioning::folder_name_from_safe_name(&safe_name);
    // `join_forward_slash` rather than `Path::join`: `servers_root` is
    // written in this codebase's forward-slash fixture convention, but
    // `Path::join` inserts `MAIN_SEPARATOR` (a backslash on Windows) for
    // each new component, so a bare `.join("java").join(&folder_name)`
    // produces a path that's forward-slash for its root and backslash for
    // everything appended -- fine for file I/O (Windows accepts both
    // interchangeably) but wrong once `.to_string_lossy()`'d into the
    // durable `server_directory`/`paper_jar_path` config fields below,
    // found by P7.29's own Windows CI leg. Same fix `java_launch.rs`
    // already applies to the launch-command jar path.
    let new_dir = join_forward_slash(
        &join_forward_slash(servers_root, std::ffi::OsStr::new("java")),
        folder_name.as_ref(),
    );

    claim_new_server_directory(fs, &new_dir, &folder_name)?;

    let outcome = (|| -> Result<CreatedServer, CreateServerError> {
        let jar_dest = join_forward_slash(&new_dir, std::ffi::OsStr::new("paper.jar"));
        let resolved = acquire_jar(
            transport,
            fs,
            home_dir,
            paper_template_dir,
            request.flavor,
            &jar_dest,
            request.save_downloaded_jars,
            now,
        )?;
        let primary_jar_path = jar_dest.to_string_lossy().into_owned();

        finish_server_creation(
            fs,
            home_dir,
            plugin_template_dir,
            &new_dir,
            request,
            &safe_name,
            &initial_slot_name,
            &initial_level_name,
            &effective,
            &imported_metadata,
            &primary_jar_path,
            Some(resolved.version.as_str()),
            Some(resolved.build.as_str()),
            resolved.loader_version.as_deref(),
            now,
            unzip_world_backup,
            copy_existing_world_folder,
        )
    })();

    if outcome.is_err() {
        let _ = fs.remove(&new_dir);
    }
    outcome
}

/// The tail shared by [`create_download_and_go_server`] (source lines
/// 296-393) and [`create_install_step_server`]: `eula.txt`,
/// `server.properties`, the add-on folder and cross-play copy, the
/// world-source dispatch, the `ConfigServer` field set, and the initial
/// persistent world slot. Everything before this point (jar acquisition
/// vs. installer run) is genuinely different between the two
/// provisioning kinds and stays in each caller.
///
/// `resolved_version`/`resolved_build` are `Option<&str>`, not `&str`:
/// both download-and-go and install-step callers always have concrete
/// values, but P7.21's `templates::create_server_from_template` doesn't
/// — `ComponentVersionParsing.parsePaperJarFilename` only recognizes a
/// `paper-*` filename (`ComponentVersionParsing.swift:29`), so a
/// template whose name doesn't match (e.g. a Purpur template) leaves
/// `resolvedVersion`/`resolvedBuild` `nil` in source too
/// (`AppViewModel+ServerCreation.swift:249-253`'s `if let parsed = ...`
/// has no `else`). `pub(crate)` for the same P7.21 reuse reason as
/// [`initial_world_slot_name`].
#[allow(clippy::too_many_arguments)]
pub(crate) fn finish_server_creation(
    fs: &dyn FileSystem,
    home_dir: &Path,
    plugin_template_dir: &Path,
    new_dir: &Path,
    request: &NewServerRequest,
    safe_name: &str,
    initial_slot_name: &str,
    initial_level_name: &str,
    effective: &provisioning::EffectiveWorldSettings,
    imported_metadata: &ImportedWorldMetadata,
    primary_jar_path: &str,
    resolved_version: Option<&str>,
    resolved_build: Option<&str>,
    resolved_loader: Option<&str>,
    now: &str,
    unzip_world_backup: impl FnOnce(&Path, &Path) -> bool,
    copy_existing_world_folder: impl FnOnce(&Path, &Path, &str) -> bool,
) -> Result<CreatedServer, CreateServerError> {
    fs.write(&new_dir.join("eula.txt"), b"eula=false\n")?;

    let props = provisioning::fresh_server_properties(
        request.port,
        safe_name,
        &effective.difficulty,
        &effective.gamemode,
        initial_level_name,
        effective.world_seed.as_deref(),
    );
    let props_path = new_dir.join("server.properties");
    worlds::write_properties_map(fs, &props_path, &props)?;

    if let Some(add_on) = provisioning::add_on_folder_name(request.flavor) {
        let add_on_dir = new_dir.join(add_on);
        fs.create_dir_all(&add_on_dir)?;
        if request.enable_cross_play && add_on == "plugins" {
            apply_cross_play_templates_if_available(fs, home_dir, plugin_template_dir, &add_on_dir);
        }
    }

    match &request.world_source {
        WorldSource::Fresh => {}
        WorldSource::BackupZip(zip_path) => {
            if !unzip_world_backup(zip_path, new_dir) {
                return Err(CreateServerError::WorldSourceFailed);
            }
        }
        WorldSource::ExistingFolder(src_folder) => {
            if !copy_existing_world_folder(src_folder, new_dir, initial_level_name) {
                return Err(CreateServerError::WorldSourceFailed);
            }
        }
    }

    let (min_ram_gb, max_ram_gb) = provisioning::default_ram_gb(request.flavor);
    let fields = provisioning::new_server_config_fields(
        safe_name,
        &new_dir.to_string_lossy(),
        primary_jar_path,
        min_ram_gb,
        max_ram_gb,
        request.flavor,
        resolved_version,
        resolved_build,
        resolved_loader,
        request.default_banner_color_hex,
        request.enable_playit,
        request.enable_xbox_broadcast,
        if request.enable_cross_play {
            request.cross_play_bedrock_port
        } else {
            None
        },
    );

    let (slot, _) = create_initial_world_slot(
        fs,
        new_dir,
        initial_slot_name,
        initial_level_name,
        &request.world_source,
        effective.world_seed.as_deref(),
        imported_metadata.seed.as_deref(),
        now,
    )
    .map_err(|_| CreateServerError::InitialWorldSlotFailed)?;

    if matches!(request.world_source, WorldSource::Fresh) {
        let profile =
            worlds::fresh_world_profile(&slot, Some(request.difficulty), Some(request.gamemode));
        world_store::save_profile(fs, new_dir, &slot, &profile)
            .map_err(|error| CreateServerError::Io(std::io::Error::other(error.to_string())))?;
    }
    let profile = world_store::load_profile(fs, new_dir, &slot);
    worlds::apply_world_profile(
        fs,
        new_dir,
        ServerType::Java,
        &profile,
        worlds::WorldProfileApplyContext::Creation,
        false,
    )?;
    world_store::set_active_slot_id(fs, new_dir, Some(&slot.id))?;

    let should_record = provisioning::should_record_loader_version(
        request.flavor,
        resolved_version,
        resolved_loader,
    );

    Ok(CreatedServer {
        config: apply_fields(fields),
        world_slot: slot,
        should_record_loader_version: should_record,
        java_compatibility_warning: None,
    })
}

/// P7.18: provisions a new Forge or NeoForge server end to end —
/// `createNewServer`'s install-step branch (`AppViewModel+ServerCreation
/// .swift:194-239`), composing P7.10's version resolution
/// (`server_versions::neoforge_minecraft_version`), P7.13's installer-jar
/// download (`jar_provider::neoforge_download_installer`/
/// `forge_download_installer`), and P7.14's supervised installer run
/// (`loader_installer::run_loader_installer`, which already confirms the
/// generated args file exists before returning success — "confirm the
/// args file exists before the server is registered as usable" is
/// satisfied by that function's own `ArgsFileNotProduced` guard, not
/// re-checked here).
///
/// **Journaling the operation, streaming progress through
/// `LifecycleOperations`, and restart reconciliation are not this
/// function's job.** `LifecycleOperations::begin_running`/`succeed`/
/// `fail`/`cancellation_check` (`crates/msc-application/src/
/// operations.rs`) is already fully generic — `operation_type`/`target`/
/// `status_line` are plain strings with no lifecycle-specific coupling —
/// and no other long-running `msc-application` service in this phase
/// (`world_conversion::convert_world`, `backups::create_backup`) calls
/// into it directly either; `backups.rs`'s own module doc names this
/// split explicitly. This function takes a caller-supplied
/// `should_cancel`/`on_output` instead, the identical shape
/// `convert_world` already established, and returns synchronously —
/// wrapping it in `begin_running`/`spawn_blocking`/`succeed`/`fail`, and
/// polling `reconcile_on_startup` at agent boot, are the P7.23 route
/// layer's job. `operations.rs` itself needed no changes for this,
/// despite being named in this step's own `Files:` list.
///
/// `should_cancel` is checked at entry and again immediately before the
/// installer subprocess starts — the same two "nothing long-running
/// touched yet" boundaries `world_conversion::convert_world` already
/// uses. Once the installer is running, `run_loader_installer`'s own
/// poll loop (P7.14) checks it on every 50ms tick and kills the process
/// tree on a positive result, rather than leaving it orphaned. On any
/// failure or cancellation, the whole `new_dir` is removed — a Forge/
/// NeoForge install writes a large `libraries/` tree, so a partial one
/// is both large and unusable, matching this step's own `What:` line.
#[allow(clippy::too_many_arguments)]
pub fn create_install_step_server(
    fs: &dyn FileSystem,
    transport: &dyn Transport,
    supervisor: &dyn ProcessSupervisor,
    home_dir: &Path,
    servers_root: &Path,
    plugin_template_dir: &Path,
    request: &NewServerRequest,
    java_executable_path: &str,
    installer_timeout: Duration,
    now: &str,
    should_cancel: &dyn Fn() -> bool,
    on_output: impl FnMut(OutputStream, &[u8]),
    unzip_world_backup: impl FnOnce(&Path, &Path) -> bool,
    copy_existing_world_folder: impl FnOnce(&Path, &Path, &str) -> bool,
) -> Result<CreatedServer, CreateServerError> {
    if !matches!(
        request.flavor,
        JavaServerFlavor::NeoForge | JavaServerFlavor::Forge
    ) {
        return Err(CreateServerError::UnsupportedFlavor(request.flavor));
    }

    let safe_name =
        provisioning::trimmed_server_name(request.name).ok_or(CreateServerError::EmptyName)?;
    if should_cancel() {
        return Err(CreateServerError::Cancelled);
    }

    let initial_slot_name = initial_world_slot_name(&safe_name, request.initial_world_name);
    let normalized_world_seed =
        normalized_initial_world_seed(request.world_seed, &request.world_source);
    let imported_metadata = match &request.world_source {
        WorldSource::Fresh => ImportedWorldMetadata::default(),
        WorldSource::BackupZip(zip_path) => imported_metadata_from_zip(zip_path, ServerType::Java),
        WorldSource::ExistingFolder(folder) => {
            imported_metadata_from_folder(folder, ServerType::Java)
        }
    };
    let effective = provisioning::effective_world_settings(
        request.difficulty,
        request.gamemode,
        normalized_world_seed.as_deref(),
        &imported_metadata,
    );
    let initial_level_name = world::sanitized_world_level_name(&initial_slot_name, "world");

    let folder_name = provisioning::folder_name_from_safe_name(&safe_name);
    // Same Windows mixed-separator gap as `create_download_and_go_server`'s
    // own `new_dir` above -- this function has its own separate copy of
    // the same construction, found needing the same fix while tracing
    // this file's own `provisioning_install_step` CI race (P7.29).
    let new_dir = join_forward_slash(
        &join_forward_slash(servers_root, std::ffi::OsStr::new("java")),
        folder_name.as_ref(),
    );

    claim_new_server_directory(fs, &new_dir, &folder_name)?;

    let mut java_compatibility_warning: Option<String> = None;
    let outcome = (|| -> Result<CreatedServer, CreateServerError> {
        let (resolved_version, resolved_loader) = match request.flavor {
            JavaServerFlavor::NeoForge => {
                let version = jar_provider::neoforge_latest_stable(transport)?;
                let mc_version = server_versions::neoforge_minecraft_version(&version);
                // P7.31: refuse an unusable Java executable before the
                // installer -- which itself needs to spawn
                // `java_executable_path` -- gets a chance to fail with an
                // opaque process error instead. Checked as soon as
                // `mc_version` is known, before the (larger) installer
                // download, so a bad runtime is never worth the network
                // cost of finding out.
                java_compatibility_warning =
                    check_java_runtime_guard(supervisor, java_executable_path, Some(&mc_version))?;
                let installer_jar_name = "neoforge-installer.jar".to_string();
                jar_provider::neoforge_download_installer(
                    transport,
                    fs,
                    &version,
                    &new_dir.join(&installer_jar_name),
                )?;
                if should_cancel() {
                    return Err(CreateServerError::Cancelled);
                }
                loader_installer::run_loader_installer(
                    supervisor,
                    fs,
                    &LoaderInstallRequest {
                        java_executable_path: java_executable_path.to_string(),
                        installer_jar_name: installer_jar_name.clone(),
                        server_dir: new_dir.clone(),
                        timeout: installer_timeout,
                        target: LoaderTarget::NeoForge {
                            specific_version: Some(version.clone()),
                        },
                    },
                    should_cancel,
                    on_output,
                )?;
                // `NeoForgeInstaller.install`'s own tidy-up (source line
                // 129-131): both the installer jar and its log, unlike
                // Forge below — a real asymmetry P7.5 already flagged,
                // preserved rather than unified.
                let _ = fs.remove(&new_dir.join(&installer_jar_name));
                let _ = fs.remove(&new_dir.join("installer.log"));
                (mc_version, version)
            }
            JavaServerFlavor::Forge => {
                let (mc_version, forge_version) =
                    jar_provider::forge_latest_recommended(transport)?;
                // P7.31: same guard, same reasoning as the NeoForge arm
                // above.
                java_compatibility_warning =
                    check_java_runtime_guard(supervisor, java_executable_path, Some(&mc_version))?;
                let installer_jar_name = "forge-installer.jar".to_string();
                jar_provider::forge_download_installer(
                    transport,
                    fs,
                    &mc_version,
                    &forge_version,
                    &new_dir.join(&installer_jar_name),
                )?;
                if should_cancel() {
                    return Err(CreateServerError::Cancelled);
                }
                loader_installer::run_loader_installer(
                    supervisor,
                    fs,
                    &LoaderInstallRequest {
                        java_executable_path: java_executable_path.to_string(),
                        installer_jar_name: installer_jar_name.clone(),
                        server_dir: new_dir.clone(),
                        timeout: installer_timeout,
                        target: LoaderTarget::Forge {
                            mc_version: Some(mc_version.clone()),
                            forge_version: Some(forge_version.clone()),
                        },
                    },
                    should_cancel,
                    on_output,
                )?;
                // `ForgeInstaller.install`'s own tidy-up (source line
                // 358): the installer jar only.
                let _ = fs.remove(&new_dir.join(&installer_jar_name));
                (mc_version, forge_version)
            }
            other => return Err(CreateServerError::UnsupportedFlavor(other)),
        };

        finish_server_creation(
            fs,
            home_dir,
            plugin_template_dir,
            &new_dir,
            request,
            &safe_name,
            &initial_slot_name,
            &initial_level_name,
            &effective,
            &imported_metadata,
            "",
            Some(resolved_version.as_str()),
            Some(resolved_loader.as_str()),
            Some(&resolved_loader),
            now,
            unzip_world_backup,
            copy_existing_world_folder,
        )
        .map(|mut created| {
            created.java_compatibility_warning = java_compatibility_warning.take();
            created
        })
    })();

    if outcome.is_err() {
        let _ = fs.remove(&new_dir);
    }
    outcome
}

// ---------------------------------------------------------------------
// P8.21: create a server from a staged, already-inspected modpack
// ---------------------------------------------------------------------

/// `applyStagedAddOn`'s `.mrpackFile`/`.curseForgeFile` cases
/// (`AppViewModel+ServerCreation.swift:700-707`) call into the same
/// `importModpack` used for an already-existing server — source has no
/// dedicated "create from pack" primitive at all
/// (`docs/msc2/addons/phase8-scope.md`'s own "Modpack create/import
/// boundary" finding). This section's own working exit criterion goes
/// further than that oracle behavior on purpose (`rolling-plan.md`'s own
/// gate text: "create a correctly pinned Fabric/Forge/NeoForge server as
/// a durable, cancellable operation"): rather than requiring the caller
/// to guess a matching flavor up front the way MSC 1's wizard UI does,
/// the loader flavor, Minecraft version, and loader build are all derived
/// from the pack's own pin — [`PackServerRequest`] has no flavor field at
/// all.
#[derive(Debug)]
pub enum CreateFromPackError {
    Create(CreateServerError),
    /// The pack pins no loader at all, or pins one this codebase has no
    /// installer for (Quilt — confirmed by grep that no Quilt install
    /// path exists anywhere in this crate; Phase 7 never provisioned it
    /// either).
    UnsupportedLoader,
    /// A `.mrpack` manifest with no `"minecraft"` dependency entry —
    /// legal per `mrpack_metadata`'s own doc, but nothing this function
    /// can provision a server from.
    MissingMinecraftVersion,
    /// The pinned Forge `{mc}-{build}` pair isn't a real entry in Forge's
    /// own Maven metadata — checked before spending a download on a
    /// build that would just 404, using the same parser
    /// `fixtures/modpack-pinning/forge-maven-*` already characterizes.
    PinnedForgeBuildNotFound {
        mc_version: String,
        forge_version: String,
    },
    /// [`modpacks::import_mrpack`]/[`modpacks::import_curseforge`] itself
    /// refused before writing anything (never `PackManaged` here — a
    /// brand-new server is never already pack-managed — but
    /// `NoAddOnKind`/`MissingApiKey`/`Provider` all apply). The whole
    /// `new_dir` is rolled back the same as every other failure in this
    /// function.
    Import(String),
    Cancelled,
}

impl fmt::Display for CreateFromPackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Create(e) => write!(f, "{e}"),
            Self::UnsupportedLoader => {
                write!(f, "this pack's loader isn't supported for server creation")
            }
            Self::MissingMinecraftVersion => {
                write!(f, "this pack doesn't pin a Minecraft version")
            }
            Self::PinnedForgeBuildNotFound {
                mc_version,
                forge_version,
            } => write!(
                f,
                "Forge build {forge_version} for Minecraft {mc_version} was not found"
            ),
            Self::Import(m) => write!(f, "{m}"),
            Self::Cancelled => write!(f, "creation was cancelled"),
        }
    }
}

impl std::error::Error for CreateFromPackError {}

impl From<CreateServerError> for CreateFromPackError {
    fn from(e: CreateServerError) -> Self {
        CreateFromPackError::Create(e)
    }
}

/// [`NewServerRequest`] minus `flavor` (derived from the pack — see this
/// section's own doc) and `save_downloaded_jars` (a pack-driven create has
/// no single downloaded "the jar" to archive as a reusable template the
/// way a download-and-go create does).
#[derive(Debug, Clone)]
pub struct PackServerRequest<'a> {
    pub name: &'a str,
    pub initial_world_name: Option<&'a str>,
    pub port: u16,
    pub enable_cross_play: bool,
    pub cross_play_bedrock_port: Option<u16>,
    pub enable_playit: bool,
    pub enable_xbox_broadcast: bool,
    pub difficulty: &'a str,
    pub gamemode: &'a str,
    pub world_seed: Option<&'a str>,
    pub world_source: WorldSource<'a>,
    pub default_banner_color_hex: &'a str,
}

/// Either pack-apply report [`modpacks::import_mrpack`]/
/// [`modpacks::import_curseforge`] can produce — kept as the real,
/// detailed type rather than flattened, so a caller can report
/// `failed_files`/`blocked_files` exactly as those functions already
/// characterize them.
#[derive(Debug)]
pub enum PackApplyReport {
    Mrpack(crate::modpacks::MrpackImportReport),
    CurseForge(crate::modpacks::CurseForgeImportReport),
}

#[derive(Debug)]
pub struct CreatedFromPack {
    pub created: CreatedServer,
    pub pack_report: PackApplyReport,
}

/// The pinned `(loader flavor, Minecraft version, loader version)` triple
/// a manifest declares — `MissingMinecraftVersion`/`UnsupportedLoader`
/// cover every case where that pin doesn't fully identify a provisionable
/// server (see each variant's own doc).
fn pack_loader_pin(
    format: &modpacks::InspectedFormat,
) -> Result<(modpack_manifest::LoaderFlavor, String, String), CreateFromPackError> {
    match format {
        modpacks::InspectedFormat::Mrpack(manifest) => {
            let meta = modpack_manifest::mrpack_metadata(manifest);
            let mc_version = meta
                .minecraft_version
                .ok_or(CreateFromPackError::MissingMinecraftVersion)?;
            let flavor = meta
                .loader_flavor
                .ok_or(CreateFromPackError::UnsupportedLoader)?;
            let loader_version = meta
                .loader_version
                .ok_or(CreateFromPackError::UnsupportedLoader)?;
            Ok((flavor, mc_version, loader_version))
        }
        modpacks::InspectedFormat::CurseForge(metadata) => {
            let flavor = metadata
                .loader_flavor
                .ok_or(CreateFromPackError::UnsupportedLoader)?;
            let loader_version = metadata
                .loader_version
                .clone()
                .ok_or(CreateFromPackError::UnsupportedLoader)?;
            Ok((flavor, metadata.minecraft_version.clone(), loader_version))
        }
        modpacks::InspectedFormat::PlainJarZip { .. } => {
            Err(CreateFromPackError::UnsupportedLoader)
        }
    }
}

fn loader_flavor_to_java_flavor(
    flavor: modpack_manifest::LoaderFlavor,
) -> Result<JavaServerFlavor, CreateFromPackError> {
    match flavor {
        modpack_manifest::LoaderFlavor::Forge => Ok(JavaServerFlavor::Forge),
        modpack_manifest::LoaderFlavor::NeoForge => Ok(JavaServerFlavor::NeoForge),
        modpack_manifest::LoaderFlavor::Fabric => Ok(JavaServerFlavor::Fabric),
        modpack_manifest::LoaderFlavor::Quilt => Err(CreateFromPackError::UnsupportedLoader),
    }
}

/// Provisions a brand-new Fabric/Forge/NeoForge server pinned exactly to
/// `inspection`'s own manifest, then applies that pack's mod list —
/// composing this module's existing [`finish_server_creation`] tail with
/// P8.19/P8.20's [`modpacks::import_mrpack`]/[`modpacks::import_curseforge`].
/// Nothing is published (the caller's registry insert — the same boundary
/// this module's own doc already draws for every other create function)
/// until loader provisioning, the shared tail, AND the pack apply itself
/// all return successfully — on any of those three failing, or on
/// `should_cancel` firing at any of this function's checkpoints (entry,
/// after loader provisioning, before the shared tail, before pack apply),
/// the whole `new_dir` is removed, the same single rollback every other
/// function in this module already uses.
/// `inspection.staged_dir` is removed unconditionally before returning —
/// once the pack's files are either merged into `new_dir` or already
/// downloaded independently via the manifest's own URLs, nothing further
/// reads it, so no staging residue survives either a successful or a
/// failed create.
#[allow(clippy::too_many_arguments)]
pub fn create_server_from_pack(
    fs: &dyn FileSystem,
    jar_transport: &dyn Transport,
    addon_transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    supervisor: &dyn ProcessSupervisor,
    home_dir: &Path,
    servers_root: &Path,
    plugin_template_dir: &Path,
    request: &PackServerRequest,
    inspection: &ModpackInspection,
    java_executable_path: &str,
    installer_timeout: Duration,
    now: &str,
    should_cancel: &dyn Fn() -> bool,
    on_output: impl FnMut(OutputStream, &[u8]),
    unzip_world_backup: impl FnOnce(&Path, &Path) -> bool,
    copy_existing_world_folder: impl FnOnce(&Path, &Path, &str) -> bool,
) -> Result<CreatedFromPack, CreateFromPackError> {
    let (loader_flavor, mc_version, loader_version) = pack_loader_pin(&inspection.format)?;
    let flavor = loader_flavor_to_java_flavor(loader_flavor)?;

    let safe_name =
        provisioning::trimmed_server_name(request.name).ok_or(CreateServerError::EmptyName)?;
    if should_cancel() {
        return Err(CreateFromPackError::Cancelled);
    }

    let initial_slot_name = initial_world_slot_name(&safe_name, request.initial_world_name);
    let normalized_world_seed =
        normalized_initial_world_seed(request.world_seed, &request.world_source);
    let imported_metadata = match &request.world_source {
        WorldSource::Fresh => ImportedWorldMetadata::default(),
        WorldSource::BackupZip(zip_path) => imported_metadata_from_zip(zip_path, ServerType::Java),
        WorldSource::ExistingFolder(folder) => {
            imported_metadata_from_folder(folder, ServerType::Java)
        }
    };
    let effective = provisioning::effective_world_settings(
        request.difficulty,
        request.gamemode,
        normalized_world_seed.as_deref(),
        &imported_metadata,
    );
    let initial_level_name = world::sanitized_world_level_name(&initial_slot_name, "world");

    let folder_name = provisioning::folder_name_from_safe_name(&safe_name);
    let new_dir = join_forward_slash(
        &join_forward_slash(servers_root, std::ffi::OsStr::new("java")),
        folder_name.as_ref(),
    );

    claim_new_server_directory(fs, &new_dir, &folder_name)?;

    let mut java_compatibility_warning: Option<String> = None;
    let outcome = (|| -> Result<CreatedFromPack, CreateFromPackError> {
        let (resolved_build, resolved_loader, primary_jar_path) = match flavor {
            JavaServerFlavor::Fabric => {
                let jar_dest = join_forward_slash(&new_dir, std::ffi::OsStr::new("paper.jar"));
                jar_provider::fabric_download_version(
                    jar_transport,
                    fs,
                    &mc_version,
                    Some(&loader_version),
                    &jar_dest,
                )
                .map_err(CreateServerError::from)?;
                (
                    format!("fabric {loader_version}"),
                    loader_version.clone(),
                    jar_dest.to_string_lossy().into_owned(),
                )
            }
            JavaServerFlavor::Forge => {
                let entries = jar_provider::forge_list_version_pairs(jar_transport)
                    .map_err(CreateServerError::from)?;
                let id = format!("{mc_version}\u{2014}{loader_version}");
                if !entries.iter().any(|e| e.id == id) {
                    return Err(CreateFromPackError::PinnedForgeBuildNotFound {
                        mc_version: mc_version.clone(),
                        forge_version: loader_version.clone(),
                    });
                }
                java_compatibility_warning =
                    check_java_runtime_guard(supervisor, java_executable_path, Some(&mc_version))
                        .map_err(CreateFromPackError::from)?;
                let installer_jar_name = "forge-installer.jar".to_string();
                jar_provider::forge_download_installer(
                    jar_transport,
                    fs,
                    &mc_version,
                    &loader_version,
                    &new_dir.join(&installer_jar_name),
                )
                .map_err(CreateServerError::from)?;
                if should_cancel() {
                    return Err(CreateFromPackError::Cancelled);
                }
                loader_installer::run_loader_installer(
                    supervisor,
                    fs,
                    &LoaderInstallRequest {
                        java_executable_path: java_executable_path.to_string(),
                        installer_jar_name: installer_jar_name.clone(),
                        server_dir: new_dir.clone(),
                        timeout: installer_timeout,
                        target: LoaderTarget::Forge {
                            mc_version: Some(mc_version.clone()),
                            forge_version: Some(loader_version.clone()),
                        },
                    },
                    should_cancel,
                    on_output,
                )
                .map_err(CreateServerError::from)?;
                let _ = fs.remove(&new_dir.join(&installer_jar_name));
                (
                    loader_version.clone(),
                    loader_version.clone(),
                    String::new(),
                )
            }
            JavaServerFlavor::NeoForge => {
                java_compatibility_warning =
                    check_java_runtime_guard(supervisor, java_executable_path, Some(&mc_version))
                        .map_err(CreateFromPackError::from)?;
                let installer_jar_name = "neoforge-installer.jar".to_string();
                jar_provider::neoforge_download_installer(
                    jar_transport,
                    fs,
                    &loader_version,
                    &new_dir.join(&installer_jar_name),
                )
                .map_err(CreateServerError::from)?;
                if should_cancel() {
                    return Err(CreateFromPackError::Cancelled);
                }
                loader_installer::run_loader_installer(
                    supervisor,
                    fs,
                    &LoaderInstallRequest {
                        java_executable_path: java_executable_path.to_string(),
                        installer_jar_name: installer_jar_name.clone(),
                        server_dir: new_dir.clone(),
                        timeout: installer_timeout,
                        target: LoaderTarget::NeoForge {
                            specific_version: Some(loader_version.clone()),
                        },
                    },
                    should_cancel,
                    on_output,
                )
                .map_err(CreateServerError::from)?;
                let _ = fs.remove(&new_dir.join(&installer_jar_name));
                let _ = fs.remove(&new_dir.join("installer.log"));
                (
                    loader_version.clone(),
                    loader_version.clone(),
                    String::new(),
                )
            }
            other => {
                return Err(CreateFromPackError::Create(
                    CreateServerError::UnsupportedFlavor(other),
                ));
            }
        };

        if should_cancel() {
            return Err(CreateFromPackError::Cancelled);
        }

        let created = finish_server_creation(
            fs,
            home_dir,
            plugin_template_dir,
            &new_dir,
            &NewServerRequest {
                name: request.name,
                initial_world_name: request.initial_world_name,
                flavor,
                port: request.port,
                enable_cross_play: request.enable_cross_play,
                cross_play_bedrock_port: request.cross_play_bedrock_port,
                enable_playit: request.enable_playit,
                enable_xbox_broadcast: request.enable_xbox_broadcast,
                difficulty: request.difficulty,
                gamemode: request.gamemode,
                world_seed: request.world_seed,
                world_source: request.world_source.clone(),
                save_downloaded_jars: false,
                default_banner_color_hex: request.default_banner_color_hex,
            },
            &safe_name,
            &initial_slot_name,
            &initial_level_name,
            &effective,
            &imported_metadata,
            &primary_jar_path,
            Some(mc_version.as_str()),
            Some(resolved_build.as_str()),
            Some(resolved_loader.as_str()),
            now,
            unzip_world_backup,
            copy_existing_world_folder,
        )
        .map_err(CreateFromPackError::from)?;

        if should_cancel() {
            return Err(CreateFromPackError::Cancelled);
        }

        let pack_report = match &inspection.format {
            modpacks::InspectedFormat::Mrpack(manifest) => {
                let report = modpacks::import_mrpack(
                    addon_transport,
                    fs,
                    &new_dir,
                    flavor,
                    manifest,
                    &inspection.staged_dir,
                    home_dir,
                    false,
                    false,
                    should_cancel,
                )
                .map_err(|e| CreateFromPackError::Import(e.to_string()))?;
                if report.cancelled {
                    return Err(CreateFromPackError::Cancelled);
                }
                PackApplyReport::Mrpack(report)
            }
            modpacks::InspectedFormat::CurseForge(metadata) => {
                let report = modpacks::import_curseforge(
                    addon_transport,
                    secrets,
                    fs,
                    &new_dir,
                    flavor,
                    metadata,
                    &inspection.staged_dir,
                    false,
                    false,
                    should_cancel,
                )
                .map_err(|e| CreateFromPackError::Import(e.to_string()))?;
                if report.cancelled {
                    return Err(CreateFromPackError::Cancelled);
                }
                PackApplyReport::CurseForge(report)
            }
            modpacks::InspectedFormat::PlainJarZip { .. } => {
                unreachable!("pack_loader_pin already rejected PlainJarZip")
            }
        };

        let (pack_name, pack_version) = match &pack_report {
            PackApplyReport::Mrpack(r) => (r.pack_name.clone(), r.pack_version.clone()),
            PackApplyReport::CurseForge(r) => (r.pack_name.clone(), r.pack_version.clone()),
        };
        let mut created = created;
        created.config.pack_managed = true;
        created.config.pack_name = Some(pack_name);
        created.config.pack_version = Some(pack_version);

        Ok(CreatedFromPack {
            created,
            pack_report,
        })
    })();

    let _ = fs.remove(&inspection.staged_dir);
    if outcome.is_err() {
        let _ = fs.remove(&new_dir);
    }
    outcome.map(|mut result| {
        result.created.java_compatibility_warning = java_compatibility_warning.take();
        result
    })
}

/// `ConfigServer(id:...)` plus its per-field assignments (source lines
/// 338-354), applied to the base six-argument initializer.
fn apply_fields(fields: provisioning::NewServerConfigFields) -> ConfigServer {
    let mut config = ConfigServer::new(
        fields.id,
        fields.display_name,
        fields.server_dir,
        fields.paper_jar_path,
        fields.min_ram_gb,
        fields.max_ram_gb,
    );
    config.java_flavor = fields.java_flavor;
    config.minecraft_version = fields.minecraft_version;
    config.server_build = fields.server_build;
    config.loader_version = fields.loader_version;
    config.banner_color_hex = Some(fields.banner_color_hex);
    config.playit_enabled = fields.playit_enabled;
    config.xbox_broadcast_enabled = fields.xbox_broadcast_enabled;
    config.bedrock_port = fields.bedrock_port.map(i64::from);
    config
}

/// `applyCrossPlayTemplatesIfAvailable(to:)`
/// (`AppViewModel+ServerCreation.swift:547-580`): copies the newest
/// Geyser and Floodgate jars found in the plugin template directory into
/// the new server's `plugins/` — silently does nothing if either is
/// missing, matching source's own log-and-return (no error propagates
/// to the caller either way).
fn apply_cross_play_templates_if_available(
    fs: &dyn FileSystem,
    home_dir: &Path,
    plugin_template_dir: &Path,
    plugins_dir: &Path,
) {
    let Ok(entries) = template_store::list_templates(fs, plugin_template_dir, home_dir) else {
        return;
    };
    let geyser = entries
        .iter()
        .find(|e| e.filename.to_lowercase().contains("geyser"));
    let floodgate = entries
        .iter()
        .find(|e| e.filename.to_lowercase().contains("floodgate"));
    let (Some(geyser), Some(floodgate)) = (geyser, floodgate) else {
        return;
    };
    let _ = template_store::copy_into_server_dir(
        fs,
        &geyser.path,
        plugins_dir,
        home_dir,
        &geyser.filename,
    );
    let _ = template_store::copy_into_server_dir(
        fs,
        &floodgate.path,
        plugins_dir,
        home_dir,
        &floodgate.filename,
    );
}

/// Resolve MSC 1's two accepted Bedrock import shapes: the selected folder
/// itself, or one direct child containing `level.dat`.  A symlink is rejected
/// instead of followed so an import cannot copy a live world outside the
/// user-selected source tree.
pub fn resolve_bedrock_world_folder(folder: &Path) -> Result<PathBuf, BedrockCreateError> {
    let folder_type = std::fs::symlink_metadata(folder)?;
    if !folder_type.is_dir() {
        return Err(BedrockCreateError::InvalidWorldSource {
            path: folder.to_path_buf(),
            reason: "selected path is not a directory".to_owned(),
        });
    }

    let level_dat = folder.join("level.dat");
    if std::fs::symlink_metadata(&level_dat)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
    {
        return Ok(folder.to_path_buf());
    }
    if std::fs::symlink_metadata(&level_dat).is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(BedrockCreateError::InvalidWorldSource {
            path: level_dat,
            reason: "level.dat is a symbolic link".to_owned(),
        });
    }

    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        let metadata = std::fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() {
            return Err(BedrockCreateError::InvalidWorldSource {
                path: entry.path(),
                reason: "symbolic links are not allowed in a Bedrock world import".to_owned(),
            });
        }
        if !metadata.is_dir() {
            continue;
        }
        let child_level_dat = entry.path().join("level.dat");
        if std::fs::symlink_metadata(&child_level_dat)
            .map(|metadata| metadata.is_file())
            .unwrap_or(false)
        {
            candidates.push(entry.path());
        } else if std::fs::symlink_metadata(&child_level_dat)
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(BedrockCreateError::InvalidWorldSource {
                path: child_level_dat,
                reason: "level.dat is a symbolic link".to_owned(),
            });
        }
    }

    if candidates.len() == 1 {
        Ok(candidates.remove(0))
    } else {
        // This fallback is observable MSC 1 behavior. The creation caller
        // separately requires level.dat, so an ambiguous wrapper is reported
        // as unusable rather than registered as a ready server.
        Ok(folder.to_path_buf())
    }
}

fn sanitized_bedrock_level_name(raw: &str, fallback: &str) -> String {
    msc_domain::world::sanitized_world_level_name(raw, fallback)
}

fn bedrock_folder_name(name: &str) -> String {
    name.to_lowercase()
        .replace(' ', "_")
        .chars()
        .filter(|character| character.is_alphanumeric() || *character == '_' || *character == '-')
        .take(40)
        .collect()
}

fn copy_bedrock_world_tree(
    fs: &dyn FileSystem,
    source: &Path,
    destination: &Path,
) -> Result<(), BedrockCreateError> {
    let metadata = std::fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(BedrockCreateError::InvalidWorldSource {
            path: source.to_path_buf(),
            reason: "world source must be a real directory".to_owned(),
        });
    }
    fs.create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_child = entry.path();
        let destination_child = destination.join(entry.file_name());
        let metadata = std::fs::symlink_metadata(&source_child)?;
        if metadata.file_type().is_symlink() {
            return Err(BedrockCreateError::InvalidWorldSource {
                path: source_child,
                reason: "symbolic links are not allowed in a Bedrock world import".to_owned(),
            });
        }
        if metadata.is_dir() {
            copy_bedrock_world_tree(fs, &source_child, &destination_child)?;
        } else if metadata.is_file() {
            fs.write(&destination_child, &std::fs::read(&source_child)?)?;
        }
    }
    Ok(())
}

/// Create a native Bedrock server directory and its first persistent world
/// slot. The directory is claimed before any file is written, and every
/// later error removes the whole candidate, so callers never receive a
/// partially-created server record.
#[allow(clippy::too_many_arguments)]
pub fn create_bedrock_server(
    fs: &dyn FileSystem,
    servers_root: &Path,
    request: &BedrockCreateRequest<'_>,
    now: &str,
) -> Result<CreatedBedrockServer, BedrockCreateError> {
    let safe_name = request.name.trim();
    if safe_name.is_empty() {
        return Err(BedrockCreateError::EmptyName);
    }
    let folder_name = bedrock_folder_name(safe_name);
    if folder_name.is_empty() {
        return Err(BedrockCreateError::EmptyDestinationName);
    }
    let bedrock_root = join_forward_slash(servers_root, std::ffi::OsStr::new("bedrock"));
    let new_dir = join_forward_slash(&bedrock_root, folder_name.as_ref());
    if let Some(parent) = new_dir.parent() {
        fs.create_dir_all(parent)?;
    }
    if let Err(error) = fs.create_dir_exclusive(&new_dir) {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            return Err(BedrockCreateError::FolderAlreadyExists { path: new_dir });
        }
        return Err(error.into());
    }

    let result = (|| -> Result<CreatedBedrockServer, BedrockCreateError> {
        let slot_name = initial_world_slot_name(safe_name, request.initial_world_name);
        let imported_metadata = match &request.world_source {
            BedrockWorldSource::Fresh => ImportedWorldMetadata::default(),
            BedrockWorldSource::BackupZip(path) => {
                if !std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file()) {
                    return Err(BedrockCreateError::InvalidWorldSource {
                        path: (*path).to_path_buf(),
                        reason: "backup ZIP does not exist".to_owned(),
                    });
                }
                imported_metadata_from_zip(path, ServerType::Bedrock)
            }
            BedrockWorldSource::ExistingFolder(path) => {
                let resolved = resolve_bedrock_world_folder(path)?;
                let level_dat = resolved.join("level.dat");
                if !std::fs::metadata(&level_dat).is_ok_and(|metadata| metadata.is_file()) {
                    return Err(BedrockCreateError::InvalidWorldSource {
                        path: resolved,
                        reason: "no level.dat was found at the selected world root".to_owned(),
                    });
                }
                imported_metadata_from_folder(&resolved, ServerType::Bedrock)
            }
        };
        let world_seed = if matches!(request.world_source, BedrockWorldSource::Fresh) {
            request
                .world_seed
                .map(str::trim)
                .filter(|seed| !seed.is_empty())
                .map(str::to_owned)
        } else {
            imported_metadata.seed.clone()
        };
        let effective_difficulty = imported_metadata
            .difficulty
            .as_deref()
            .unwrap_or(request.difficulty);
        let effective_gamemode = imported_metadata
            .gamemode
            .as_deref()
            .unwrap_or(request.gamemode);
        let initial_level_name = match &request.world_source {
            BedrockWorldSource::ExistingFolder(path) => {
                let resolved = resolve_bedrock_world_folder(path)?;
                sanitized_bedrock_level_name(
                    resolved
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(""),
                    &slot_name,
                )
            }
            BedrockWorldSource::Fresh | BedrockWorldSource::BackupZip(_) => {
                sanitized_bedrock_level_name(&slot_name, "Bedrock level")
            }
        };
        let mut properties = std::collections::BTreeMap::new();
        properties.insert("server-name".to_owned(), safe_name.to_owned());
        properties.insert("level-name".to_owned(), initial_level_name.clone());
        properties.insert("gamemode".to_owned(), effective_gamemode.to_owned());
        properties.insert("difficulty".to_owned(), effective_difficulty.to_owned());
        properties.insert("max-players".to_owned(), request.max_players.to_string());
        properties.insert("server-port".to_owned(), request.port.to_string());
        properties.insert("server-portv6".to_owned(), "19133".to_owned());
        properties.insert("online-mode".to_owned(), "true".to_owned());
        properties.insert("allow-cheats".to_owned(), "false".to_owned());
        if let Some(seed) = world_seed.as_deref() {
            properties.insert("level-seed".to_owned(), seed.to_owned());
        }

        atomic_write(
            fs,
            &new_dir.join("server.properties"),
            render_raw_properties(&properties).as_bytes(),
        )?;
        atomic_write(fs, &new_dir.join("allowlist.json"), b"[]\n")?;
        atomic_write(fs, &new_dir.join("permissions.json"), b"[]\n")?;

        match &request.world_source {
            BedrockWorldSource::Fresh => {}
            BedrockWorldSource::BackupZip(path) => {
                fs.create_dir_all(&new_dir.join("worlds"))?;
                archive::extract_zip(path, &new_dir)?;
            }
            BedrockWorldSource::ExistingFolder(path) => {
                let resolved = resolve_bedrock_world_folder(path)?;
                copy_bedrock_world_tree(
                    fs,
                    &resolved,
                    &new_dir.join("worlds").join(&initial_level_name),
                )?;
            }
        }

        let slot = match &request.world_source {
            BedrockWorldSource::Fresh => {
                let slot = world::build_fresh_slot(
                    uuid::Uuid::new_v4().to_string().to_uppercase(),
                    &slot_name,
                    world_seed.as_deref(),
                    ServerType::Bedrock,
                    now.to_owned(),
                );
                world_store::save_metadata(fs, &new_dir, &slot)?;
                let profile = worlds::fresh_world_profile(
                    &slot,
                    Some(request.difficulty),
                    Some(request.gamemode),
                );
                world_store::save_profile(fs, &new_dir, &slot, &profile)
                    .map_err(BedrockCreateError::AtomicWrite)?;
                slot
            }
            BedrockWorldSource::BackupZip(path) => worlds::import_zip_as_new_slot(
                fs,
                &new_dir,
                ServerType::Bedrock,
                Some(&initial_level_name),
                path,
                &slot_name,
                now,
            )?,
            BedrockWorldSource::ExistingFolder(_) => worlds::create_slot_from_current_world(
                fs,
                &new_dir,
                ServerType::Bedrock,
                Some(&initial_level_name),
                &slot_name,
                imported_metadata.seed.as_deref(),
                now,
            )?,
        };
        let profile = world_store::load_profile(fs, &new_dir, &slot);
        worlds::apply_world_profile(
            fs,
            &new_dir,
            ServerType::Bedrock,
            &profile,
            worlds::WorldProfileApplyContext::Creation,
            false,
        )?;
        world_store::set_active_slot_id(fs, &new_dir, Some(&slot.id))?;

        let mut config = ConfigServer::new(
            uuid::Uuid::new_v4().to_string().to_uppercase(),
            safe_name,
            new_dir.to_string_lossy(),
            "",
            0.0,
            0.0,
        );
        config.server_type = ServerType::Bedrock;
        config.bedrock_port = Some(i64::from(request.port));
        config.bedrock_version = request
            .bedrock_version
            .map(str::trim)
            .filter(|version| !version.is_empty() && *version != "LATEST")
            .map(str::to_owned);
        config.playit_enabled = request.enable_playit;
        config.xbox_broadcast_enabled = request.enable_xbox_broadcast;

        Ok(CreatedBedrockServer {
            config,
            world_slot: slot,
        })
    })();

    if result.is_err() {
        let _ = fs.remove(&new_dir);
    }
    result
}

/// Production `unzip_world_backup`: extracts a backup zip's own top-level
/// folders directly into the new server directory
/// (`AppViewModel+WorldManagement.swift:276-296`'s `unzip -o`).
pub fn real_unzip_world_backup(zip_path: &Path, into_server_dir: &Path) -> bool {
    archive::extract_zip(zip_path, into_server_dir).is_ok()
}

/// Production `copy_existing_world_folder`: replaces
/// `server_dir/level_name` with a full recursive copy of `src_folder`
/// (`AppViewModel+WorldManagement.swift:298-312`).
pub fn real_copy_existing_world_folder(
    src_folder: &Path,
    server_dir: &Path,
    level_name: &str,
) -> bool {
    let dest = server_dir.join(level_name);
    let _ = std::fs::remove_dir_all(&dest);
    copy_dir_recursive_real(src_folder, &dest).is_ok()
}

fn copy_dir_recursive_real(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive_real(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}
