//! P7.19: list catalog versions for an existing server and change its
//! jar version. Ports `changeVersionProvider`
//! (`AppViewModel+APIWiringAddons.swift:358-573`) — the real wire-route
//! oracle, **not** `AppViewModel+ComponentsVersions.swift`'s
//! `downloadAndApplyJarVersion`, a separate Mac-local-UI-only code path
//! the remote route reimplements independently rather than calling (a
//! fork-agent research pass confirmed this by reading both functions
//! end to end). Also composes `ServerJarProvider.listVersions(for:)`
//! (`ServerJarProviders.swift:68-77`) for the version-listing half this
//! step's own "What" line asks for.
//!
//! **Correction to this step's own plan text**: `changeVersionProvider`
//! never archives a jar at all — no `archiveServerJar`/
//! `saveDownloadedJars` call anywhere in it (confirmed by reading the
//! whole function, source line 358-573). That archiving behavior exists
//! only in the Mac-local `downloadAndApplyJarVersion`, and even there it
//! archives the jar it just downloaded (the new one — `destURL` is
//! already overwritten before that archive call runs), not "the
//! outgoing jar" as this step's plan text says. This port follows the
//! real wire oracle and does not archive; noted here rather than
//! silently invented, the same "What"-line correction pattern P7.1/
//! P7.6/P7.8/P7.15 already established for other steps.
//!
//! **Registering the resolved version into the fleet is not this
//! module's job**, matching `provisioning.rs`'s own precedent: this
//! module returns the resolved [`ChangedVersion`]; writing it into
//! `AppConfig.servers[idx]` and saving, toggling `isDownloadingJar`
//! around the call, and `recordLoaderVersion`'s actual persistence are
//! all the route layer's (P7.23) — the same split `provisioning.rs`
//! already draws for `should_record_loader_version`.
//!
//! **Bedrock is out of scope.** `changeVersionProvider`'s Bedrock branch
//! (`BedrockProvisioner`/`updateBedrockVMFiles`) belongs to Phase 10 per
//! this phase's own "Not in this phase" list — the same refusal
//! precedent P6.8/P7.17 already set for Bedrock creation. A caller
//! refuses Bedrock with `capability_unavailable` before ever reaching
//! this module, which only takes a [`msc_domain::identity::JavaServerFlavor`].
//!
//! **Pufferfish/Spigot/Quilt are refused, not partially built.** No P7
//! step built a Pufferfish downloader (Jenkins CI, no HTTP catalog) —
//! `ServerJarProvider.downloadVersion`'s own dispatcher already throws
//! `unsupportedFlavor` for a *pinned* Pufferfish/Spigot/Quilt version
//! change; this port additionally refuses the *latest* case too (source
//! would actually succeed there, via `PufferfishDownloader.downloadLatest`)
//! rather than build a Jenkins CI client under this phase's time budget.
//! A decided-without-asking scope narrowing, flagged rather than
//! silently matched to source's own inconsistency.
//!
//! **A real oracle finding, ported faithfully rather than "fixed":**
//! Fabric's pinned-version-change path can never honor a pinned loader
//! version, even though the wire request carries one. Source's Fabric
//! case always builds its `ServerVersionEntry` with `loaderVersion: nil`
//! (`AppViewModel+APIWiringAddons.swift:516-518`), so the `loaderVersion`
//! closure parameter is only ever consumed by the NeoForge/Forge
//! branches. [`change_fabric`] always resolves the loader fresh for
//! whichever Minecraft version it lands on, matching this real (if
//! surprising) limitation exactly.

use msc_domain::identity::JavaServerFlavor;
use msc_domain::server_versions::{self, ServerVersionEntry};
use msc_domain::version::is_downgrade;
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::jar_provider::{self, JarProviderError, Transport};
use msc_infrastructure::loader_installer::{
    self, LoaderInstallRequest, LoaderInstallerError, LoaderTarget,
};
use msc_infrastructure::process::{OutputStream, ProcessSupervisor};
use std::fmt;
use std::path::Path;
use std::time::Duration;

/// `ServerVersionEntry.latest.id` (`ServerJarProviders.swift:26-33`) —
/// the version picker's own sentinel for "resolve the latest stable
/// version," which `changeVersionProvider` checks by string comparison
/// (`versionId == "__latest__" || versionId.isEmpty`) rather than a typed
/// `Option`, since the wire request carries a plain string.
pub const LATEST: &str = "__latest__";

#[derive(Debug)]
pub enum ChangeVersionError {
    /// `"server_running"` (source line 373).
    ServerRunning,
    /// `"download_in_progress"` (source line 376).
    DownloadInProgress,
    /// `"not_supported"`/the pinned-download dispatcher's own
    /// `unsupportedFlavor` throw — collapsed into one variant since both
    /// end in the same "this flavor can't be version-changed" outcome.
    UnsupportedFlavor(JavaServerFlavor),
    /// `"backup_failed"` (source line 430-431).
    BackupFailed,
    Download(JarProviderError),
    LoaderInstaller(LoaderInstallerError),
    /// `should_cancel` reported true before the installer's own polling
    /// loop had a chance to observe it — checked at entry and again
    /// immediately before the installer subprocess starts, the same
    /// boundary `provisioning::create_install_step_server` already uses.
    Cancelled,
}

impl fmt::Display for ChangeVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChangeVersionError::ServerRunning => write!(f, "server is running"),
            ChangeVersionError::DownloadInProgress => {
                write!(f, "a jar download is already in progress")
            }
            ChangeVersionError::UnsupportedFlavor(flavor) => {
                write!(f, "{} is not version-changeable", flavor.raw_value())
            }
            ChangeVersionError::BackupFailed => write!(f, "pre-downgrade backup failed"),
            ChangeVersionError::Download(e) => write!(f, "{e}"),
            ChangeVersionError::LoaderInstaller(e) => write!(f, "{e}"),
            ChangeVersionError::Cancelled => write!(f, "version change was cancelled"),
        }
    }
}

impl std::error::Error for ChangeVersionError {}

impl From<JarProviderError> for ChangeVersionError {
    fn from(e: JarProviderError) -> Self {
        ChangeVersionError::Download(e)
    }
}

impl From<LoaderInstallerError> for ChangeVersionError {
    fn from(e: LoaderInstallerError) -> Self {
        ChangeVersionError::LoaderInstaller(e)
    }
}

/// What a successful change resolved to — the caller applies these onto
/// `ConfigServer.minecraftVersion`/`.serverBuild`/`.loaderVersion` (Forge/
/// NeoForge/Fabric only) itself, per this module's own doc.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangedVersion {
    pub minecraft_version: String,
    pub build: String,
    pub loader_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChangeVersionRequest<'a> {
    pub flavor: JavaServerFlavor,
    /// `"__latest__"`/empty means latest, matching source's own string
    /// sentinel rather than a typed `Option` — see [`LATEST`]'s own doc.
    pub version_id: &'a str,
    /// Only ever consulted for NeoForge/Forge — see this module's own
    /// doc for why Fabric can't use it despite the wire request carrying
    /// it too.
    pub loader_version: Option<&'a str>,
    pub current_minecraft_version: Option<&'a str>,
    pub server_dir: &'a Path,
    /// `cfg.paperJarPath`, already trimmed — empty means `<serverDir>/
    /// paper.jar` (source line 438-441), the same fallback
    /// `launch_shape::jar_basename` already encodes.
    pub paper_jar_path: &'a str,
}

/// `changeVersionProvider`'s downgrade-guard target extraction (source
/// line 416-426): NeoForge/Forge ids are `"MC\u{2014}Loader"` (em-dash);
/// every other flavor uses `versionId` directly. `None` for "latest" —
/// a move to latest is never treated as a downgrade.
fn target_mc_for_downgrade_check(flavor: JavaServerFlavor, version_id: &str) -> Option<String> {
    let mc = match flavor {
        JavaServerFlavor::NeoForge | JavaServerFlavor::Forge => {
            version_id.split('\u{2014}').next().unwrap_or("")
        }
        _ => version_id,
    };
    if mc.is_empty() || mc == LATEST {
        None
    } else {
        Some(mc.to_string())
    }
}

/// Ports `changeVersionProvider` end to end (source line 358-573), minus
/// its Bedrock branch (out of scope, see this module's own doc) and
/// minus everything the caller owns instead (fleet registry, the
/// `isDownloadingJar` flag, `recordLoaderVersion`'s write target).
///
/// `pre_downgrade_backup` is the fakeable boundary over `self.createBackup`
/// (source line 428) — the same "closure over a risky/external mechanism"
/// shape `world_conversion::convert_world`'s `pre_conversion_backup`
/// parameter and `provisioning.rs`'s `unzip_world_backup` already
/// establish, called only when [`is_downgrade`] says the target is
/// actually older than `current_minecraft_version`.
#[allow(clippy::too_many_arguments)]
pub fn change_version(
    fs: &dyn FileSystem,
    transport: &dyn Transport,
    supervisor: &dyn ProcessSupervisor,
    java_executable_path: &str,
    installer_timeout: Duration,
    request: &ChangeVersionRequest,
    is_running: bool,
    is_downloading: bool,
    pre_downgrade_backup: impl FnOnce() -> bool,
    should_cancel: &dyn Fn() -> bool,
    on_output: impl FnMut(OutputStream, &[u8]),
) -> Result<ChangedVersion, ChangeVersionError> {
    if is_running {
        return Err(ChangeVersionError::ServerRunning);
    }
    if is_downloading {
        return Err(ChangeVersionError::DownloadInProgress);
    }
    if matches!(
        request.flavor,
        JavaServerFlavor::Pufferfish | JavaServerFlavor::Spigot | JavaServerFlavor::Quilt
    ) {
        return Err(ChangeVersionError::UnsupportedFlavor(request.flavor));
    }
    if should_cancel() {
        return Err(ChangeVersionError::Cancelled);
    }

    if let Some(target) = target_mc_for_downgrade_check(request.flavor, request.version_id)
        && is_downgrade(request.current_minecraft_version, &target)
        && !pre_downgrade_backup()
    {
        return Err(ChangeVersionError::BackupFailed);
    }

    match request.flavor {
        JavaServerFlavor::NeoForge => change_neoforge(
            fs,
            transport,
            supervisor,
            java_executable_path,
            installer_timeout,
            request,
            should_cancel,
            on_output,
        ),
        JavaServerFlavor::Forge => change_forge(
            fs,
            transport,
            supervisor,
            java_executable_path,
            installer_timeout,
            request,
            should_cancel,
            on_output,
        ),
        JavaServerFlavor::Fabric => {
            let jar_dest = jar_destination(request);
            change_fabric(transport, fs, &jar_dest, request.version_id)
        }
        _ => {
            let jar_dest = jar_destination(request);
            change_download_and_go(transport, fs, request.flavor, request.version_id, &jar_dest)
        }
    }
}

/// `let trimmed = cfg.paperJarPath...; let destURL = trimmed.isEmpty ?
/// serverDir/paper.jar : URL(fileURLWithPath: trimmed)` (source line
/// 438-441).
fn jar_destination(request: &ChangeVersionRequest) -> std::path::PathBuf {
    let trimmed = request.paper_jar_path.trim();
    if trimmed.is_empty() {
        request.server_dir.join("paper.jar")
    } else {
        std::path::PathBuf::from(trimmed)
    }
}

fn is_latest(version_id: &str) -> bool {
    version_id.is_empty() || version_id == LATEST
}

/// Paper/Purpur/Vanilla (`default:` branch, source line 529-551).
fn change_download_and_go(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    flavor: JavaServerFlavor,
    version_id: &str,
    jar_dest: &Path,
) -> Result<ChangedVersion, ChangeVersionError> {
    let (minecraft_version, build) = match flavor {
        JavaServerFlavor::Paper => {
            if is_latest(version_id) {
                let (version, selection) = jar_provider::paper_resolve_latest_stable(transport)?
                    .ok_or_else(|| {
                        ChangeVersionError::Download(JarProviderError::Network(
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
                (version, selection.build_id.to_string())
            } else {
                let (_cached, build_id) = jar_provider::paper_download_pinned_version(
                    transport, fs, version_id, jar_dest,
                )?;
                (version_id.to_string(), build_id.to_string())
            }
        }
        JavaServerFlavor::Purpur => {
            if is_latest(version_id) {
                let raw_versions = jar_provider::purpur_raw_version_list(transport)?;
                let paper_stable = jar_provider::paper_resolve_latest_stable(transport)
                    .ok()
                    .flatten()
                    .map(|(version, _)| version);
                let target = server_versions::purpur_pick_target_version(
                    &raw_versions,
                    paper_stable.as_deref(),
                )
                .ok_or_else(|| {
                    ChangeVersionError::Download(JarProviderError::Network(
                        "No Purpur versions found.".to_string(),
                    ))
                })?;
                let build = jar_provider::purpur_latest_build_label(transport, &target)?;
                jar_provider::purpur_download_version(transport, fs, &target, jar_dest)?;
                (target, build)
            } else {
                // `PurpurDownloader.downloadVersion(_:to:)` reports the
                // literal string `"latest"` as its build label — it never
                // resolves the real build number the way `downloadLatest`
                // does (`ServerJarProviders.swift:327-333`), a genuine
                // asymmetry preserved here rather than "improved."
                jar_provider::purpur_download_version(transport, fs, version_id, jar_dest)?;
                (version_id.to_string(), "latest".to_string())
            }
        }
        JavaServerFlavor::Vanilla => {
            if is_latest(version_id) {
                let cached = jar_provider::vanilla_download_latest(transport, fs, jar_dest)?;
                (cached.version, "release".to_string())
            } else {
                jar_provider::vanilla_download_version(transport, fs, version_id, jar_dest)?;
                (version_id.to_string(), "release".to_string())
            }
        }
        other => return Err(ChangeVersionError::UnsupportedFlavor(other)),
    };
    Ok(ChangedVersion {
        minecraft_version,
        build,
        loader_version: None,
    })
}

/// Fabric (source line 511-527) — see this module's own doc for why
/// `request_loader_version` is never consulted here.
fn change_fabric(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    jar_dest: &Path,
    version_id: &str,
) -> Result<ChangedVersion, ChangeVersionError> {
    let game_version = if is_latest(version_id) {
        jar_provider::fabric_latest_stable_game_version(transport)?
    } else {
        version_id.to_string()
    };
    let loader = jar_provider::fabric_resolve_loader(transport, &game_version)?;
    jar_provider::fabric_download_version(transport, fs, &game_version, Some(&loader), jar_dest)?;
    Ok(ChangedVersion {
        minecraft_version: game_version,
        build: format!("fabric {loader}"),
        loader_version: Some(loader),
    })
}

/// NeoForge (source line 456-479): re-runs the installer directly into
/// the existing `server_dir` — no new directory, matching
/// `create_install_step_server`'s installer composition but without any
/// of that function's directory-creation/rollback machinery, since
/// there's an existing server here to preserve on failure, not a fresh
/// one to discard.
#[allow(clippy::too_many_arguments)]
fn change_neoforge(
    fs: &dyn FileSystem,
    transport: &dyn Transport,
    supervisor: &dyn ProcessSupervisor,
    java_executable_path: &str,
    installer_timeout: Duration,
    request: &ChangeVersionRequest,
    should_cancel: &dyn Fn() -> bool,
    on_output: impl FnMut(OutputStream, &[u8]),
) -> Result<ChangedVersion, ChangeVersionError> {
    let pinned = request
        .loader_version
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let version = match pinned {
        Some(v) => v.to_string(),
        None => jar_provider::neoforge_latest_stable(transport)?,
    };
    let minecraft_version = server_versions::neoforge_minecraft_version(&version);
    let installer_jar_name = "neoforge-installer.jar".to_string();
    jar_provider::neoforge_download_installer(
        transport,
        fs,
        &version,
        &request.server_dir.join(&installer_jar_name),
    )?;
    if should_cancel() {
        return Err(ChangeVersionError::Cancelled);
    }
    loader_installer::run_loader_installer(
        supervisor,
        fs,
        &LoaderInstallRequest {
            java_executable_path: java_executable_path.to_string(),
            installer_jar_name: installer_jar_name.clone(),
            server_dir: request.server_dir.to_path_buf(),
            timeout: installer_timeout,
            target: LoaderTarget::NeoForge {
                specific_version: Some(version.clone()),
            },
        },
        should_cancel,
        on_output,
    )?;
    // `NeoForgeInstaller.install`'s own tidy-up (source line 129-131),
    // matching `create_install_step_server`'s identical cleanup exactly.
    let _ = fs.remove(&request.server_dir.join(&installer_jar_name));
    let _ = fs.remove(&request.server_dir.join("installer.log"));
    Ok(ChangedVersion {
        minecraft_version,
        build: version.clone(),
        loader_version: Some(version),
    })
}

/// Forge (source line 481-509).
#[allow(clippy::too_many_arguments)]
fn change_forge(
    fs: &dyn FileSystem,
    transport: &dyn Transport,
    supervisor: &dyn ProcessSupervisor,
    java_executable_path: &str,
    installer_timeout: Duration,
    request: &ChangeVersionRequest,
    should_cancel: &dyn Fn() -> bool,
    on_output: impl FnMut(OutputStream, &[u8]),
) -> Result<ChangedVersion, ChangeVersionError> {
    let mc_ver_from_id = request
        .version_id
        .split('\u{2014}')
        .next()
        .unwrap_or(request.version_id)
        .trim();
    let pinned_forge = request
        .loader_version
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let (minecraft_version, forge_version) = if let Some(fv) = pinned_forge
        && !mc_ver_from_id.is_empty()
        && request.version_id != LATEST
    {
        (mc_ver_from_id.to_string(), fv.to_string())
    } else {
        jar_provider::forge_latest_recommended(transport)?
    };

    let installer_jar_name = "forge-installer.jar".to_string();
    jar_provider::forge_download_installer(
        transport,
        fs,
        &minecraft_version,
        &forge_version,
        &request.server_dir.join(&installer_jar_name),
    )?;
    if should_cancel() {
        return Err(ChangeVersionError::Cancelled);
    }
    loader_installer::run_loader_installer(
        supervisor,
        fs,
        &LoaderInstallRequest {
            java_executable_path: java_executable_path.to_string(),
            installer_jar_name: installer_jar_name.clone(),
            server_dir: request.server_dir.to_path_buf(),
            timeout: installer_timeout,
            target: LoaderTarget::Forge {
                mc_version: Some(minecraft_version.clone()),
                forge_version: Some(forge_version.clone()),
            },
        },
        should_cancel,
        on_output,
    )?;
    // `ForgeInstaller.install`'s own tidy-up: the installer jar only,
    // matching `create_install_step_server`'s identical cleanup exactly.
    let _ = fs.remove(&request.server_dir.join(&installer_jar_name));
    Ok(ChangedVersion {
        minecraft_version,
        build: forge_version.clone(),
        loader_version: Some(forge_version),
    })
}

/// `ServerJarProvider.listVersions(for:)` (`ServerJarProviders.swift:
/// 68-77`), plus D-014's 1.20 floor (per this rolling-plan's own
/// "decided without asking" note: the floor applies to `GET /v1/versions`
/// too, not just `/versions/create`) and a per-entry `is_current` flag
/// against `current_minecraft_version` — the "current version marked"
/// half of this step's own "What" line, which has no source equivalent
/// (MSC 1's version picker marks the current row in its own SwiftUI list
/// view, not in the fetched data).
pub fn list_versions_for_server(
    transport: &dyn Transport,
    flavor: JavaServerFlavor,
    current_minecraft_version: Option<&str>,
) -> Result<Vec<VersionListEntry>, JarProviderError> {
    let raw: Vec<ServerVersionEntry> = match flavor {
        JavaServerFlavor::Paper => jar_provider::paper_list_versions_for_picker(transport)?,
        JavaServerFlavor::Purpur => jar_provider::purpur_list_versions(transport)?,
        JavaServerFlavor::Vanilla => jar_provider::vanilla_list_versions(transport)?,
        JavaServerFlavor::Fabric => jar_provider::fabric_list_versions(transport)?,
        JavaServerFlavor::NeoForge => jar_provider::neoforge_list_version_pairs(transport)?,
        JavaServerFlavor::Forge => jar_provider::forge_list_version_pairs(transport)?,
        // `default: return []` (source line 76) for Pufferfish/Spigot/
        // Quilt — MSC 1 itself has no version picker for any of the
        // three either, matching this module's own doc on the
        // pinned/latest download refusal above.
        JavaServerFlavor::Pufferfish | JavaServerFlavor::Spigot | JavaServerFlavor::Quilt => {
            Vec::new()
        }
    };
    // Filtered on `mc_version`, not `id`: NeoForge/Forge ids are
    // `"MC\u{2014}Loader"` pairs (`neoforge_build_entries`/
    // `forge_parse_maven_metadata`), and `filter_to_create_flow_floor`'s
    // `compare_mc_versions` expects a plain dotted-decimal string — an
    // id-keyed filter would feed it the combined pair and mis-parse.
    let mc_versions: Vec<String> = raw.iter().map(|e| e.mc_version.clone()).collect();
    let floored_mc_versions: std::collections::HashSet<String> =
        server_versions::filter_to_create_flow_floor(&mc_versions)
            .into_iter()
            .collect();
    Ok(raw
        .into_iter()
        .filter(|e| floored_mc_versions.contains(&e.mc_version))
        .map(|e| {
            let is_current = current_minecraft_version.is_some_and(|cur| cur == e.mc_version);
            VersionListEntry {
                entry: e,
                is_current,
            }
        })
        .collect())
}

#[derive(Debug, Clone, PartialEq)]
pub struct VersionListEntry {
    pub entry: ServerVersionEntry,
    pub is_current: bool,
}
