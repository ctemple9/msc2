//! P8.22: portable client add-on export —
//! `AppViewModel+ClientExport.swift`'s `buildClientExportItems`/
//! `exportClientModsAsZip`/`copyClientLinksToClipboard`.
//!
//! Builds the client-side mod/plugin export list for a server (a Modrinth
//! link list for Paper-like servers, a deterministic ZIP of JAR files for
//! modded servers), classifying each installed add-on's client-side need
//! from — in order — a persisted `AddonLink.client_side` (Modrinth's own
//! opinion), an unlinked Fabric mod's own embedded `fabric.mod.json`
//! `"environment"`, or an honest `"assumed"` default when neither signal
//! is available.
//!
//! **Rust ZIP, never `/usr/bin/zip -j`**
//! (`docs/msc2/msc2-decisions.md`'s D-006/D-011 headless-tri-platform
//! requirement, already the norm every Phase 6-8 archive site follows —
//! `phase8-scope.md`'s own "zip extraction is macOS-only in the oracle"
//! finding names this exact call site as one of the five that don't
//! survive the port). [`write_client_export_zip`] builds the archive with
//! `zip::ZipWriter` directly, entries in the exact selection order
//! [`build_client_export_items`]'s own deterministic sort already
//! produced — a real improvement over source's own unspecified
//! `Process`/`zip` argument-list order.
//!
//! **No base64, by construction.** [`write_client_export_zip`] writes
//! straight to a caller-supplied destination path and returns nothing but
//! a typed result — never an in-memory blob a route would have to encode.
//! The size ceiling and JSON-response-shape question this step's own
//! `What:` line also raises are the *route* layer's job once one exists
//! (P8.24) — `docs/msc2/addons/phase8-api.md` §5 already settles it
//! (`stagedDownloadId`, `zipBase64` dropped since it was never shipped).
//!
//! **No temp-staging copy, and no subprocess exit status to log.** Source's
//! own `writeZip` copies every selected jar into a throwaway temp
//! directory first (`fixtures/client-addon-export/export-zip-temp-staging-
//! dir-removed-on-both-success-and-failure-paths.json`) purely so
//! `/usr/bin/zip -j`'s own argument list can name flat paths, then logs a
//! non-zero exit status rather than raising an error
//! (`export-zip-nonzero-exit-status-logged-not-silently-swallowed.json`).
//! `ZipWriter::start_file` already writes each entry under its own bare
//! filename directly from the source jar's bytes — there is no
//! intermediate copy to stage or clean up, and no subprocess exit code to
//! observe; a real write failure surfaces as this function's own typed
//! `Err`, not a logged side channel.
//!
//! **"Enforce selection/path bounds" is satisfied by construction, not by
//! an added check.** [`write_client_export_zip`] takes `&[ClientExportItem]`
//! — the only way to produce one is [`build_client_export_items`]'s own
//! directory scan — so there is no call shape that lets a caller point
//! this function at an arbitrary path the way a general file-browser
//! primitive would; the same "no such surface exists" precedent
//! `modpacks.rs`'s own inspection module already established for staged
//! uploads.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use msc_domain::app_config_schema::ConfigServer;
use msc_domain::identity::AddOnKind;
use msc_infrastructure::fs::FileSystem;

use crate::add_on_inventory;

// ---------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------

/// `ClientSideStatus` (`AppViewModel+ClientExport.swift:15-36`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientSideStatus {
    Required,
    Optional,
    ServerOnly,
    Unknown,
}

impl ClientSideStatus {
    /// `isSelectedByDefault` (line 24-28): every status defaults to
    /// selected except `.serverOnly` — an item MSC genuinely doesn't know
    /// about is included rather than excluded, since a missing
    /// client-required mod breaks the connection while an extra
    /// unnecessary one is just clutter (source's own line 21 comment).
    pub fn is_selected_by_default(self) -> bool {
        !matches!(self, Self::ServerOnly)
    }

    pub fn display_label(self) -> &'static str {
        match self {
            Self::Required => "Required",
            Self::Optional => "Optional",
            Self::ServerOnly => "Server-only",
            Self::Unknown => "Unknown",
        }
    }

    fn sort_rank(self) -> u8 {
        match self {
            Self::Required => 0,
            Self::Optional => 1,
            Self::Unknown => 2,
            Self::ServerOnly => 3,
        }
    }
}

/// `clientSideStatus(from:)` (line 225-231): a persisted Modrinth
/// `client_side` value. Distinct from `AddonUpdateResolver`'s own
/// `isClientOnly`/`ModpackClientOnlyClassifier`'s own tiering — this is a
/// separate, local mapper that happens to land on the same status names
/// for the same Modrinth values.
pub fn client_side_status_from_modrinth(value: &str) -> ClientSideStatus {
    match value {
        "required" => ClientSideStatus::Required,
        "optional" => ClientSideStatus::Optional,
        "unsupported" => ClientSideStatus::ServerOnly,
        _ => ClientSideStatus::Unknown,
    }
}

/// `clientSideStatus(fromEnvironment:)` (line 233-237): a Fabric mod's own
/// embedded `fabric.mod.json` `"environment"`. `"client"` maps to
/// `Required`, not excluded — the export list answers "does the client
/// need this present," a different question from whether the same jar
/// should be server-side *disabled* (`ModpackClientOnlyClassifier`'s job).
pub fn client_side_status_from_environment(env: &str) -> ClientSideStatus {
    match env {
        "server" => ClientSideStatus::ServerOnly,
        "client" | "*" => ClientSideStatus::Required,
        _ => ClientSideStatus::Unknown,
    }
}

/// `ClientExportItem` (line 44-58).
#[derive(Debug, Clone)]
pub struct ClientExportItem {
    pub jar_stem: String,
    pub file_name: String,
    pub display_name: String,
    pub icon_url: Option<String>,
    pub project_id: Option<String>,
    pub slug: Option<String>,
    pub client_status: ClientSideStatus,
    /// Human-readable source of the classification: `"Modrinth"`, `"mod
    /// manifest"`, or `"assumed"`.
    pub status_source: String,
    pub is_selected: bool,
    /// Real on-disk path this entry was discovered at — never a
    /// caller-supplied path (see this module's own doc on "enforce
    /// selection/path bounds").
    pub jar_path: PathBuf,
}

impl ClientExportItem {
    /// `modrinthURL` (line 56-58): slug wins over project id when both are
    /// present; `None` when neither is.
    pub fn modrinth_url(&self) -> Option<String> {
        let slug_or_id = self.slug.as_deref().or(self.project_id.as_deref())?;
        Some(format!("https://modrinth.com/project/{slug_or_id}"))
    }
}

// ---------------------------------------------------------------------
// buildClientExportItems
// ---------------------------------------------------------------------

/// `buildClientExportItems(for:)` (line 66-140). Real add-on jars only —
/// `jar_path` fields point at files on disk (`ModJarMetadataParser`'s own
/// zip-entry reads need a real path, the same boundary
/// `add_on_inventory.rs`'s own doc already established), so `fs` is used
/// only for the directory listing itself, matching this crate's existing
/// "list through the trait, read jar bytes for real" split.
pub fn build_client_export_items(fs: &dyn FileSystem, cfg: &ConfigServer) -> Vec<ClientExportItem> {
    let Some(add_on_kind) = cfg.java_flavor.add_on_kind() else {
        return Vec::new();
    };
    let folder = Path::new(&cfg.server_dir).join(add_on_kind.folder_name());
    let Ok(entries) = fs.list(&folder) else {
        return Vec::new();
    };

    let jar_paths: Vec<PathBuf> = entries
        .into_iter()
        .filter(|p| {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_lowercase();
            name.ends_with(".jar") || name.ends_with(".jar.disabled")
        })
        .collect();
    if jar_paths.is_empty() {
        return Vec::new();
    }

    let mut items: Vec<ClientExportItem> = jar_paths
        .into_iter()
        .filter_map(|path| build_one_item(&path, cfg, add_on_kind))
        .collect();

    items.sort_by(|a, b| {
        a.client_status
            .sort_rank()
            .cmp(&b.client_status.sort_rank())
            .then_with(|| {
                a.display_name
                    .to_lowercase()
                    .cmp(&b.display_name.to_lowercase())
            })
    });
    items
}

fn build_one_item(
    path: &Path,
    cfg: &ConfigServer,
    add_on_kind: AddOnKind,
) -> Option<ClientExportItem> {
    let filename = path.file_name()?.to_str()?.to_string();
    let lower = filename.to_lowercase();
    let is_enabled = !lower.ends_with(".jar.disabled");
    let jar_stem = if is_enabled {
        filename
            .strip_suffix(".jar")
            .unwrap_or(&filename)
            .to_string()
    } else {
        filename
            .strip_suffix(".jar.disabled")
            .unwrap_or(&filename)
            .to_string()
    };

    // Managed plugins (Geyser/Floodgate): bedrock compat, not client mods.
    let stem_lower = jar_stem.to_lowercase();
    if stem_lower.contains("geyser") || stem_lower.contains("floodgate") {
        return None;
    }

    let link = cfg.addon_links.as_ref().and_then(|links| {
        links
            .values()
            .find(|l| l.installed_file_name.as_deref() == Some(filename.as_str()))
            .or_else(|| {
                let bare = format!("{jar_stem}.jar");
                links
                    .values()
                    .find(|l| l.installed_file_name.as_deref() == Some(bare.as_str()))
            })
    });

    let (status, source) = match link.and_then(|l| l.client_side.as_deref()) {
        Some(cs) => (client_side_status_from_modrinth(cs), "Modrinth".to_string()),
        None => match add_on_inventory::mod_jar_metadata(path).and_then(|(_, _, _, env)| env) {
            Some(env) => (
                client_side_status_from_environment(&env),
                "mod manifest".to_string(),
            ),
            None => (ClientSideStatus::Unknown, "assumed".to_string()),
        },
    };

    // Paper/Purpur: only items Modrinth explicitly marks as client-needed
    // are shown — unknown/unlinked plugins are almost always server-only.
    if add_on_kind == AddOnKind::Plugin
        && matches!(
            status,
            ClientSideStatus::ServerOnly | ClientSideStatus::Unknown
        )
    {
        return None;
    }

    let display_name = link
        .and_then(|l| l.title.clone())
        .or_else(|| mod_display_name_any(path))
        .unwrap_or_else(|| add_on_inventory::extract_display_name(&jar_stem));

    Some(ClientExportItem {
        jar_stem,
        file_name: filename,
        display_name,
        icon_url: link.and_then(|l| l.icon_url.clone()),
        project_id: link.map(|l| l.project_id.clone()),
        slug: link.and_then(|l| l.slug.clone()),
        client_status: status,
        status_source: source,
        is_selected: status.is_selected_by_default(),
        jar_path: path.to_path_buf(),
    })
}

/// `ModJarMetadataParser.parseAny(jarURL:)?.displayName` (line 129):
/// `parseAny` tries Fabric, then Forge/NeoForge, then `plugin.yml`,
/// returning the FIRST manifest that parses at all — its `displayName` is
/// used even when that manifest itself has none (which does NOT fall
/// through to `plugin.yml`; only "no fabric/forge manifest present at
/// all" does). `add_on_inventory::mod_jar_metadata` already is the exact
/// Fabric-then-Forge half of `parseAny`; `plugin_yml_name` is only
/// consulted when that returns `None` entirely.
fn mod_display_name_any(jar_path: &Path) -> Option<String> {
    match add_on_inventory::mod_jar_metadata(jar_path) {
        Some((_, name, _, _)) => name,
        None => add_on_inventory::plugin_yml_name(jar_path),
    }
}

// ---------------------------------------------------------------------
// copyClientLinksToClipboard
// ---------------------------------------------------------------------

/// `copyClientLinksToClipboard` (line 178-186)'s text-building half — the
/// actual clipboard write is a client concern, not this crate's.
/// `None` when nothing is selected, matching source's own no-op guard.
pub fn client_links_text(items: &[ClientExportItem]) -> Option<String> {
    let lines: Vec<String> = items
        .iter()
        .filter(|i| i.is_selected)
        .map(|item| {
            let url = item
                .modrinth_url()
                .unwrap_or_else(|| "(no link)".to_string());
            format!("\u{2022} {}: {url}", item.display_name)
        })
        .collect();
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

// ---------------------------------------------------------------------
// exportClientModsAsZip / writeZip
// ---------------------------------------------------------------------

/// `exportClientModsAsZip`'s filename half (line 145-149): slashes and
/// colons sanitized so the result is a legal filename on every platform,
/// not just macOS.
pub fn client_export_zip_name(cfg: &ConfigServer) -> String {
    let mc_version = cfg.minecraft_version.as_deref().unwrap_or("mods");
    format!("{}-client-{mc_version}.zip", cfg.display_name)
        .replace('/', "-")
        .replace(':', "")
}

#[derive(Debug)]
pub enum ClientExportZipError {
    /// `guard !selected.isEmpty else { return }` (line 145) — a no-op in
    /// source; a typed refusal here so a caller can report it honestly
    /// rather than silently writing an empty archive.
    NothingSelected,
    Io(std::io::Error),
    Zip(String),
}

impl std::fmt::Display for ClientExportZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NothingSelected => write!(f, "no items are selected for export"),
            Self::Io(e) => write!(f, "{e}"),
            Self::Zip(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for ClientExportZipError {}

/// `writeZip` (line 155-177): flat archive, entries junked to their bare
/// filename (`zip -j`'s own effect, reproduced directly with
/// `ZipWriter::start_file` rather than a nested path) in `items`' own
/// already-deterministic order. Disabled (`.jar.disabled`) entries are
/// included identically to active ones — `ClientExportItem` carries no
/// `is_enabled` field at all, matching source's own gap
/// (`fixtures/client-addon-export/disabled-jar-included-in-export-list-
/// identically-to-active-jar.json`). On any failure, the partially-written
/// destination file is removed rather than left as a corrupt archive —
/// new agent-owned safety source doesn't have (its own `Process`-based
/// `zip` either succeeds or leaves nothing, since it writes to `dest`
/// directly with no intermediate temp file either).
pub fn write_client_export_zip(
    items: &[ClientExportItem],
    dest: &Path,
) -> Result<(), ClientExportZipError> {
    let selected: Vec<&ClientExportItem> = items.iter().filter(|i| i.is_selected).collect();
    if selected.is_empty() {
        return Err(ClientExportZipError::NothingSelected);
    }

    let result = (|| -> Result<(), ClientExportZipError> {
        let file = std::fs::File::create(dest).map_err(ClientExportZipError::Io)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);
        for item in &selected {
            let bytes = std::fs::read(&item.jar_path).map_err(ClientExportZipError::Io)?;
            zip.start_file(&item.file_name, options)
                .map_err(|e| ClientExportZipError::Zip(e.to_string()))?;
            zip.write_all(&bytes).map_err(ClientExportZipError::Io)?;
        }
        zip.finish()
            .map_err(|e| ClientExportZipError::Zip(e.to_string()))?;
        Ok(())
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(dest);
    }
    result
}
