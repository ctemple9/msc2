//! Add-on install, update, toggle, remove, and source linking — the real
//! mutation half of `AddonUpdateResolver`/`AppViewModel+ModManagement.swift`/
//! `AppViewModel+PluginManagement.swift`, wiring P8.14's verified storage
//! primitives and P8.15's dependency installer to real add-on/plugin
//! mutations. Pack-managed refusal (`msc_domain::modpack::
//! pack_mutation_refused`, P8.12) gates every mutation here, including the
//! two `health/repair` actions (P8.23) once those route through the same
//! functions — not a second, parallel gate.
//!
//! **No `LifecycleOperations`/audit-log wiring here**, the same deferral
//! `addon_updates.rs`'s own module doc already explains for the read path:
//! operation-lifecycle and audit-log wiring are the *route* layer's job
//! (P8.24), once a route exists to own a mutation across an async boundary
//! and to supply the client-IP/token-label the audit primitive needs —
//! neither is available to a synchronous application function. Every
//! function here returns a plain, typed `Result` a route can translate
//! into an operation outcome and an audit entry.
//!
//! **No stopped-server requirement.** Checked directly against the
//! oracle (`AppViewModel+ModManagement.swift`, `AppViewModel+
//! PluginManagement.swift`): neither file checks `isRunning` before
//! mutating a mod/plugin folder — MSC 1 lets you install/update/toggle/
//! remove an add-on while the server is running (it just won't take
//! effect until the next restart). This port preserves that; "enforce
//! stopped-server... rules where required" (`rolling-plan.md`'s own P8.17
//! text) is satisfied by there being no such requirement to enforce,
//! decided rather than silently assumed.
//!
//! **Disabled-state preservation across replacement.** Both update paths
//! below (Modrinth-catalog and plugin-source) compute their final
//! destination with the entry's disabled suffix preserved when the
//! existing file was disabled — an item you disabled stays disabled after
//! an update, matching `rolling-plan.md`'s own explicit requirement for
//! this step.

use std::fmt;
use std::path::{Path, PathBuf};

use msc_domain::addon_provider::{self as domain, ModrinthVersionInfo};
use msc_domain::addon_update::{self, PluginVersionDispatch};
use msc_domain::app_config_schema::{
    AddonLink, AddonLinkProvenance, PluginSourceConfig, PluginSourceKind,
};
use msc_domain::identity::JavaServerFlavor;
use msc_domain::modpack::{self, AddonMutationKind};
use msc_domain::plugin_source;

use msc_infrastructure::addon_provider::AddonTransport;
use msc_infrastructure::addon_store::{self, AddonStoreError, DISABLED_SUFFIX};
use msc_infrastructure::download_staging::ExpectedChecksum;
use msc_infrastructure::fs::FileSystem;

use crate::addon_dependencies::{self, DependencyInstallReport};
use crate::addon_updates::AddonUpdateItem;

use std::collections::HashMap;

#[derive(Debug)]
pub enum AddonMutationError {
    /// The target server is pack-managed and this mutation isn't the
    /// sanctioned whole-pack-replace escape hatch (`msc_domain::modpack`,
    /// P8.12).
    PackManaged,
    /// `flavor.add_on_kind()` is `None` (Vanilla has no add-on folder).
    NoAddOnKind,
    /// The chosen Modrinth version has no installable primary file.
    NoPrimaryFile,
    /// `update_one` was asked to update an item whose bucket isn't
    /// `UpdateAvailable`, or whose `available_version` is missing.
    NoUpdateAvailable,
    /// A plugin-source URL this crate could not classify or parse at all.
    UnrecognizedSource,
    Provider(String),
    Store(String),
    Io(String),
}

impl fmt::Display for AddonMutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PackManaged => write!(f, "this server is managed by a modpack"),
            Self::NoAddOnKind => write!(f, "this server flavor has no add-on folder"),
            Self::NoPrimaryFile => write!(f, "the selected version has no installable file"),
            Self::NoUpdateAvailable => write!(f, "no update is available for this add-on"),
            Self::UnrecognizedSource => write!(f, "this plugin source URL could not be resolved"),
            Self::Provider(m) => write!(f, "{m}"),
            Self::Store(m) => write!(f, "{m}"),
            Self::Io(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for AddonMutationError {}

impl From<AddonStoreError> for AddonMutationError {
    fn from(e: AddonStoreError) -> Self {
        Self::Store(e.to_string())
    }
}

impl From<domain::AddonProviderError> for AddonMutationError {
    fn from(e: domain::AddonProviderError) -> Self {
        Self::Provider(e.to_string())
    }
}

fn ensure_not_pack_managed(
    pack_managed: bool,
    kind: AddonMutationKind,
) -> Result<(), AddonMutationError> {
    if modpack::pack_mutation_refused(pack_managed, kind) {
        return Err(AddonMutationError::PackManaged);
    }
    Ok(())
}

fn add_on_folder(
    server_dir: &Path,
    flavor: JavaServerFlavor,
) -> Result<PathBuf, AddonMutationError> {
    let kind = flavor
        .add_on_kind()
        .ok_or(AddonMutationError::NoAddOnKind)?;
    Ok(server_dir.join(kind.folder_name()))
}

/// A `.jar` destination path, with `DISABLED_SUFFIX` appended when the
/// entry being replaced was disabled — "preserve disabled suffixes...
/// across replacement" (this module's own doc).
fn dest_preserving_disabled_state(folder: &Path, filename: &str, was_enabled: bool) -> PathBuf {
    if was_enabled {
        folder.join(filename)
    } else {
        folder.join(format!("{filename}{DISABLED_SUFFIX}"))
    }
}

// ---------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------

#[derive(Debug)]
pub struct InstallOutcome {
    pub installed_path: PathBuf,
    pub dependencies: DependencyInstallReport,
}

/// Installs `version`'s primary file into the server's add-on folder, then
/// chases its required dependencies (P8.15). A catalog install always
/// lands enabled — MSC 1's own catalog/search install flow has no
/// disabled-on-install option either.
#[allow(clippy::too_many_arguments)]
pub fn install_from_catalog(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    server_dir: &Path,
    flavor: JavaServerFlavor,
    version: &ModrinthVersionInfo,
    minecraft_version: Option<&str>,
    installed_mod_ids: &[String],
    pack_managed: bool,
    should_cancel: &dyn Fn() -> bool,
) -> Result<InstallOutcome, AddonMutationError> {
    ensure_not_pack_managed(pack_managed, AddonMutationKind::Install)?;
    let folder = add_on_folder(server_dir, flavor)?;
    let primary =
        domain::modrinth_primary_file(&version.files).ok_or(AddonMutationError::NoPrimaryFile)?;

    fs.create_dir_all(&folder)
        .map_err(|e| AddonMutationError::Io(e.to_string()))?;
    let dest = folder.join(&primary.filename);
    let expected_checksum = primary
        .hashes
        .get("sha1")
        .map(|hex| ExpectedChecksum::sha1(hex.clone()));

    addon_store::install_verified_file(
        transport,
        fs,
        &primary.url,
        &version.version_number,
        expected_checksum.as_ref(),
        &dest,
    )?;

    let dependencies = addon_dependencies::install_required_dependencies(
        transport,
        fs,
        version,
        flavor,
        minecraft_version,
        server_dir,
        installed_mod_ids,
        should_cancel,
    );

    Ok(InstallOutcome {
        installed_path: dest,
        dependencies,
    })
}

/// Installs an already-staged local jar (a client-uploaded file redeemed
/// through `POST /v1/staged-uploads` by the caller — P8.24's job, this
/// function only ever sees a real path on disk already) verbatim into the
/// server's add-on folder. No provider, no checksum to verify against
/// (there is no publisher digest for a locally-supplied file), matching
/// `AppViewModel+ModManagement`'s own manual-file-add path, which performs
/// no structural validation beyond the copy itself.
pub fn install_from_staged_local_jar(
    fs: &dyn FileSystem,
    server_dir: &Path,
    flavor: JavaServerFlavor,
    staged_jar_path: &Path,
    filename: &str,
    pack_managed: bool,
) -> Result<PathBuf, AddonMutationError> {
    ensure_not_pack_managed(pack_managed, AddonMutationKind::Install)?;
    let folder = add_on_folder(server_dir, flavor)?;
    fs.create_dir_all(&folder)
        .map_err(|e| AddonMutationError::Io(e.to_string()))?;
    let bytes = fs
        .read(staged_jar_path)
        .map_err(|e| AddonMutationError::Io(e.to_string()))?;
    let dest = folder.join(filename);
    fs.write(&dest, &bytes)
        .map_err(|e| AddonMutationError::Io(e.to_string()))?;
    Ok(dest)
}

// ---------------------------------------------------------------------
// Update (Modrinth-catalog-linked add-ons — mods and Modrinth-linked
// plugins alike, the `AddonUpdateResolver`/`addon_updates.rs` path)
// ---------------------------------------------------------------------

/// Updates one item to its already-resolved `available_version`
/// (`addon_updates::resolve_addon_updates`'s own output — no second
/// Modrinth request here, see that module's own P8.17 amendment note on
/// `AddonUpdateItem::available_version`). A filename change (a version
/// bump that renames the jar) removes the stale file only after the new
/// one is verified and staged.
#[allow(clippy::too_many_arguments)]
pub fn update_one(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    server_dir: &Path,
    flavor: JavaServerFlavor,
    item: &AddonUpdateItem,
    minecraft_version: Option<&str>,
    installed_mod_ids: &[String],
    pack_managed: bool,
    should_cancel: &dyn Fn() -> bool,
) -> Result<InstallOutcome, AddonMutationError> {
    ensure_not_pack_managed(pack_managed, AddonMutationKind::Update)?;
    let version = item
        .available_version
        .as_ref()
        .ok_or(AddonMutationError::NoUpdateAvailable)?;
    let folder = add_on_folder(server_dir, flavor)?;
    let primary =
        domain::modrinth_primary_file(&version.files).ok_or(AddonMutationError::NoPrimaryFile)?;

    fs.create_dir_all(&folder)
        .map_err(|e| AddonMutationError::Io(e.to_string()))?;
    let dest = dest_preserving_disabled_state(&folder, &primary.filename, item.is_enabled);
    let expected_checksum = primary
        .hashes
        .get("sha1")
        .map(|hex| ExpectedChecksum::sha1(hex.clone()));

    addon_store::install_verified_file(
        transport,
        fs,
        &primary.url,
        &version.version_number,
        expected_checksum.as_ref(),
        &dest,
    )?;

    if dest.file_name() != Some(std::ffi::OsStr::new(&item.filename)) {
        let old_path = folder.join(&item.filename);
        let _ = addon_store::remove_addon_jar(fs, &old_path);
    }

    let dependencies = addon_dependencies::install_required_dependencies(
        transport,
        fs,
        version,
        flavor,
        minecraft_version,
        server_dir,
        installed_mod_ids,
        should_cancel,
    );

    Ok(InstallOutcome {
        installed_path: dest,
        dependencies,
    })
}

/// One outcome per item attempted, in `items`' own order. A per-item
/// failure never aborts the batch — matching every other "update all" /
/// "install all dependencies" loop this phase has already built
/// (`addon_dependencies.rs`'s own per-dependency non-fatal loop).
pub struct BatchUpdateResult {
    pub filename: String,
    pub outcome: Result<InstallOutcome, AddonMutationError>,
}

#[allow(clippy::too_many_arguments)]
pub fn update_all(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    server_dir: &Path,
    flavor: JavaServerFlavor,
    items: &[AddonUpdateItem],
    minecraft_version: Option<&str>,
    installed_mod_ids: &[String],
    pack_managed: bool,
    should_cancel: &dyn Fn() -> bool,
) -> Vec<BatchUpdateResult> {
    items
        .iter()
        .filter(|item| item.available_version.is_some())
        .map(|item| {
            let outcome = update_one(
                transport,
                fs,
                server_dir,
                flavor,
                item,
                minecraft_version,
                installed_mod_ids,
                pack_managed,
                should_cancel,
            );
            BatchUpdateResult {
                filename: item.filename.clone(),
                outcome,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------
// Toggle / remove
// ---------------------------------------------------------------------

pub fn toggle(
    fs: &dyn FileSystem,
    path: &Path,
    pack_managed: bool,
) -> Result<PathBuf, AddonMutationError> {
    ensure_not_pack_managed(pack_managed, AddonMutationKind::Toggle)?;
    Ok(addon_store::toggle_addon_jar(fs, path)?)
}

pub fn remove(
    fs: &dyn FileSystem,
    path: &Path,
    pack_managed: bool,
) -> Result<(), AddonMutationError> {
    ensure_not_pack_managed(pack_managed, AddonMutationKind::Remove)?;
    Ok(addon_store::remove_addon_jar(fs, path)?)
}

// ---------------------------------------------------------------------
// Manual Modrinth link (`ConfigServer.addon_links`)
// ---------------------------------------------------------------------

/// A user-supplied manual link between an on-disk jar and a Modrinth
/// project — no oracle mutation gate exists for this (it only ever writes
/// config), so no pack-managed check here; linking metadata, unlike an
/// install/remove/toggle/update, changes nothing on disk.
pub fn set_manual_addon_link(
    links: &mut HashMap<String, AddonLink>,
    project_id: &str,
    title: Option<String>,
    slug: Option<String>,
) {
    links.insert(
        project_id.to_string(),
        AddonLink {
            project_id: project_id.to_string(),
            title,
            slug,
            icon_url: None,
            provenance: AddonLinkProvenance::UserLinked,
            installed_version_id: None,
            installed_file_name: None,
            installed_hash: None,
            client_side: None,
            server_side: None,
            extra: Default::default(),
        },
    );
}

pub fn remove_addon_link(links: &mut HashMap<String, AddonLink>, project_id: &str) {
    links.remove(project_id);
}

// ---------------------------------------------------------------------
// Plugin-source set/remove (`ConfigServer.plugin_sources`)
// ---------------------------------------------------------------------

pub fn set_plugin_source(
    sources: &mut HashMap<String, PluginSourceConfig>,
    jar_stem: &str,
    config: PluginSourceConfig,
) {
    addon_update::set_plugin_source(sources, jar_stem, config);
}

pub fn remove_plugin_source(
    sources: HashMap<String, PluginSourceConfig>,
    jar_stem: &str,
) -> Option<HashMap<String, PluginSourceConfig>> {
    addon_update::remove_plugin_source(sources, jar_stem)
}

// ---------------------------------------------------------------------
// Plugin-source update (GitHub / Modrinth / Hangar / Direct — the
// `downloadPluginWithSourceCheck`/`downloadLatestForPlugin` path,
// distinct from the Modrinth-hash-based `update_one` above: this path
// updates a plugin the user manually pointed at a URL, not one
// Modrinth's own hash-identity resolved.)
// ---------------------------------------------------------------------

pub struct PluginSourceUpdateOutcome {
    pub installed_path: PathBuf,
    /// `Some(new_stem)` when the download's own final filename changed the
    /// jar's stem (`msc_domain::addon_update::plugin_source_rekey`) —
    /// `sources` has already been rekeyed to match by the time this
    /// returns; carried here only so a caller also holding
    /// `ConfigServer.addon_links`/other jar-stem-keyed state knows to
    /// follow the rename too.
    pub rekeyed_to: Option<String>,
}

/// Updates one plugin-source-linked plugin (`ConfigServer.plugin_sources`,
/// not `addon_links`) to its provider's current release, mirroring
/// `downloadPluginWithSourceCheck`/`downloadLatestForPlugin`
/// (`AppViewModel+PluginManagement.swift`). `sources` is rekeyed in place
/// when the download changes the jar's stem — the same "strip stale
/// prefix entries, write the new key" operation `set_plugin_source`
/// itself performs (P8.11's `plugin_source_rekey`/`set_plugin_source`).
#[allow(clippy::too_many_arguments)]
pub fn update_plugin_from_source(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    plugins_dir: &Path,
    jar_stem: &str,
    display_name: &str,
    is_enabled: bool,
    source: &PluginSourceConfig,
    minecraft_version: Option<&str>,
    loaders: &[String],
    pack_managed: bool,
    sources: &mut HashMap<String, PluginSourceConfig>,
) -> Result<PluginSourceUpdateOutcome, AddonMutationError> {
    ensure_not_pack_managed(pack_managed, AddonMutationKind::Update)?;

    let (online_version, download_url, checksum) =
        resolve_source_download(transport, source, minecraft_version, loaders)?;

    let final_filename = addon_update::plugin_final_filename(
        &download_url,
        display_name,
        Some(online_version.as_str()),
    );

    fs.create_dir_all(plugins_dir)
        .map_err(|e| AddonMutationError::Io(e.to_string()))?;

    // Remove stale prior copies (both enabled and disabled) BEFORE writing
    // the new file, matching `downloadLatestForPlugin`'s own ordering
    // (`msc_domain::addon_update::stale_jars_to_remove`'s own doc).
    let existing_files: Vec<String> = fs
        .list(plugins_dir)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string))
        .collect();
    for stale in addon_update::stale_jars_to_remove(&existing_files, display_name) {
        let _ = addon_store::remove_addon_jar(fs, &plugins_dir.join(stale));
    }

    let dest = dest_preserving_disabled_state(plugins_dir, &final_filename, is_enabled);
    addon_store::install_verified_file(
        transport,
        fs,
        &download_url,
        &online_version,
        checksum.as_ref(),
        &dest,
    )?;

    let rekeyed_to = addon_update::plugin_source_rekey(jar_stem, &final_filename);
    if let Some(new_stem) = &rekeyed_to
        && let Some(config) = sources.remove(jar_stem).or_else(|| Some(source.clone()))
    {
        addon_update::set_plugin_source(sources, new_stem, config);
    }
    Ok(PluginSourceUpdateOutcome {
        installed_path: dest,
        rekeyed_to,
    })
}

fn resolve_source_download(
    transport: &dyn AddonTransport,
    source: &PluginSourceConfig,
    minecraft_version: Option<&str>,
    loaders: &[String],
) -> Result<(String, String, Option<ExpectedChecksum>), AddonMutationError> {
    match addon_update::plugin_version_dispatch(source.source_type) {
        PluginVersionDispatch::DirectImmediate { .. } => {
            let (version, url) = domain::direct_dispatch(&source.url)?;
            Ok((version, url, None))
        }
        PluginVersionDispatch::FetchOnlineFirst => match source.source_type {
            PluginSourceKind::Github => {
                let (owner, repo) = plugin_source::parse_github(&source.url)
                    .ok_or(AddonMutationError::UnrecognizedSource)?;
                let release = msc_infrastructure::addon_provider::github_latest_release(
                    transport, &owner, &repo,
                )?;
                let asset = domain::github_select_jar_asset(&release.assets)
                    .ok_or(AddonMutationError::NoPrimaryFile)?;
                Ok((asset.name.clone(), asset.browser_download_url.clone(), None))
            }
            PluginSourceKind::Hangar => {
                let (author, slug) = plugin_source::parse_hangar(&source.url)
                    .ok_or(AddonMutationError::UnrecognizedSource)?;
                let (version, url) = msc_infrastructure::addon_provider::hangar_fetch_latest(
                    transport,
                    &author,
                    &slug,
                    minecraft_version,
                )?;
                Ok((version.name, url, None))
            }
            PluginSourceKind::Modrinth => {
                let slug = plugin_source::parse_modrinth(&source.url)
                    .ok_or(AddonMutationError::UnrecognizedSource)?;
                let project =
                    msc_infrastructure::addon_provider::modrinth_project(transport, &slug)?;
                let versions = msc_infrastructure::addon_provider::modrinth_project_versions(
                    transport,
                    &project.slug,
                    loaders,
                    minecraft_version,
                )?;
                let best = versions
                    .first()
                    .ok_or(AddonMutationError::NoUpdateAvailable)?;
                let primary = domain::modrinth_primary_file(&best.files)
                    .ok_or(AddonMutationError::NoPrimaryFile)?;
                let checksum = primary
                    .hashes
                    .get("sha1")
                    .map(|hex| ExpectedChecksum::sha1(hex.clone()));
                Ok((best.version_number.clone(), primary.url.clone(), checksum))
            }
            PluginSourceKind::Direct => unreachable!("Direct is DirectImmediate above"),
        },
    }
}
