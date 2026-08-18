//! Server version catalogs: the shared [`ServerVersionEntry`] model, each of
//! the six provisioning families' response-to-entries parse, and the
//! per-family numeric version comparisons/selection rules.
//!
//! Ported from `ServerJarProviders.swift` (the `ServerVersionEntry` model,
//! `compareMCVersions`, and the Purpur/Vanilla/Fabric/NeoForge/Forge
//! providers), `PaperDownloader.swift` (`fetchAllVersionsSorted`,
//! `fetchBestBuild`, `fetchAvailableVersions`'s 20-candidate walk -- the
//! real path `downloadLatestPaper`/`fetchLatestMetadata` and the
//! Components-tab picker both call), and `NeoForgeInstaller.swift`
//! (`NeoForgeInstaller`/`ForgeInstaller`'s `listVersionPairs`/
//! `parseMavenMetadata`). Every function here takes bytes/strings already in
//! memory -- `msc-domain` carries no I/O, per `msc2-engineering.md` §6; the
//! real HTTP fetch is `msc-infrastructure`'s job (P7.13).
//!
//! Deliberately NOT unified across providers, per P7.4's fixture notes: each
//! family's empty-list handling and `isStable` derivation differs in MSC 1
//! and is preserved as-is here rather than factored into one shared
//! implementation that would silently change behavior. The one exception is
//! the numeric dotted-version comparator itself (`compareMCVersions` in
//! `ServerJarProviders.swift`, `compareMinecraftVersions` in
//! `PaperDownloader.swift`, and NeoForge's/Forge's own private `compare`/
//! `compareMCStrings`/`compareForgeVersions`): these are byte-identical
//! algorithms copy-pasted six times in the Swift source, so [`compare_mc_versions`]
//! ports them once -- that removes duplication without changing any
//! observable behavior, unlike the differences this module does preserve.
//!
//! Scope note on Paper: P7.4 characterized `PaperDownloader.swift`'s
//! `fetchBestBuild`/`fetchAvailableVersions` (the real path
//! `downloadLatestPaper`, `fetchLatestMetadata`, and the Components tab all
//! call) with 26/26 fixtures backing it. `ServerJarProviders.swift` also
//! contains two lower-fidelity near-duplicates -- `PaperDownloader.listVersions()`'s
//! own `paperVersionEntryV3` (a list-only picker helper with no
//! download-URL guard, used only to populate the create-flow's on-screen
//! list) and `downloadVersion`'s separate channel-agnostic highest-id scan
//! -- neither of which P7.4 characterized as its own fixture case. This
//! module ports the fixture-backed, functionally load-bearing path only;
//! P7.17's application layer can compose [`paper_flatten_and_sort`] +
//! [`paper_select_build`] directly for the create-flow's version list and
//! download, rather than introducing two more near-duplicate pure functions
//! with no fixture backing.

use std::collections::HashSet;

use serde_json::Value;

/// The sentinel id `ServerVersionEntry.latest` uses (`ServerJarProviders.swift:27`).
pub const LATEST_SENTINEL_ID: &str = "__latest__";

/// The Phase 7 create-flow version floor (D-014): `GET /v1/versions/create`
/// and `GET /v1/versions` drop entries below this line. None of the six
/// providers filter by a version floor themselves -- this is layered
/// uniformly on top of each provider's own raw list by
/// [`filter_to_create_flow_floor`], applied by the caller, not by the
/// per-family functions above.
pub const CREATE_FLOW_FLOOR: &str = "1.20";

/// One item in the version picker (`ServerJarProviders.swift:18-35`). For
/// the four download-and-go flavors (Vanilla/Paper/Purpur/Fabric), `id`
/// equals `mc_version`. For the two install-step flavors (NeoForge/Forge),
/// `id` is the paired `"{mc}—{loader_version}"` string -- the two versions
/// are inseparable, so a version-change route keying off `id` alone must
/// know this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerVersionEntry {
    pub id: String,
    pub display_label: String,
    pub mc_version: String,
    pub loader_version: Option<String>,
    pub build_label: Option<String>,
    pub is_stable: bool,
}

impl ServerVersionEntry {
    /// A sentinel row injected by the version-picker UI, not a real catalog
    /// entry (`ServerJarProviders.swift:26-33`).
    pub fn latest() -> Self {
        ServerVersionEntry {
            id: LATEST_SENTINEL_ID.to_string(),
            display_label: "Latest (recommended)".to_string(),
            mc_version: String::new(),
            loader_version: None,
            build_label: None,
            is_stable: true,
        }
    }

    /// Every call site must check this before trusting `mc_version`/
    /// `loader_version` -- an empty-string `mc_version` is never a real
    /// version to look up (`ServerJarProviders.swift:34`).
    pub fn is_latest(&self) -> bool {
        self.id == LATEST_SENTINEL_ID
    }
}

#[derive(Debug)]
pub enum CatalogError {
    /// A non-2xx HTTP status, formatted the same way `ensureOK`
    /// (`ServerJarProviders.swift:580-584`) and NeoForge's/Forge's own
    /// inline checks do: `"{what} returned status {code}."`.
    Network(String),
    /// The response body was not valid JSON at all -- kept as its own
    /// variant (rather than folded into `InvalidResponse`) because MSC 1
    /// itself treats the two differently: a failed `as?` shape-cast
    /// produces a friendly custom error, while a raw JSON syntax error
    /// propagates unwrapped (`ServerJarProviders.swift` never wraps
    /// `JSONSerialization.jsonObject(with:)` in `try?`/`do`-`catch`). See
    /// `malformed-json-response-rejected-as-invalid-shape`.
    InvalidJson(String),
    /// Valid JSON (or, for NeoForge/Forge, any text at all), but not the
    /// shape this call site expected.
    InvalidResponse(String),
    /// NeoForge's `.noStableVersion` / Forge's `.noVersion`: no candidate
    /// survived the stable filter.
    NoStableVersion,
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::Network(m) => write!(f, "Network error: {m}"),
            CatalogError::InvalidJson(m) => write!(f, "Malformed JSON: {m}"),
            CatalogError::InvalidResponse(m) => write!(f, "Unexpected response: {m}"),
            CatalogError::NoStableVersion => write!(f, "Couldn't find a stable version."),
        }
    }
}

impl std::error::Error for CatalogError {}

/// `ensureOK` (`ServerJarProviders.swift:580-584`): the shared non-2xx
/// guard every download-and-go provider runs before parsing a response
/// body. `what` is a call-site-supplied label baked into the message, the
/// same way every real call site names itself ("Mojang manifest", "Paper
/// builds for 1.21.11", ...).
pub fn ensure_http_ok(status: u16, what: &str) -> Result<(), CatalogError> {
    if status != 200 {
        return Err(CatalogError::Network(format!(
            "{what} returned status {status}."
        )));
    }
    Ok(())
}

fn simple_stable_entry(mc_version: &str) -> ServerVersionEntry {
    ServerVersionEntry {
        id: mc_version.to_string(),
        display_label: mc_version.to_string(),
        mc_version: mc_version.to_string(),
        loader_version: None,
        build_label: None,
        is_stable: true,
    }
}

fn mc_version_components(v: &str) -> Vec<i64> {
    v.split('.').filter_map(|p| p.parse::<i64>().ok()).collect()
}

/// Compares Minecraft version strings numerically, dropping (not
/// zero-padding) any dot-separated component that isn't a bare integer --
/// e.g. `"26.2-rc-2"` compares as `[26]`, not `[26, 2]`. Do not "fix" this
/// by trying to parse pre-release suffixes: nothing downstream depends on
/// pre-release versions sorting sensibly, and MSC 1 never attempted to
/// either. Ported once from the six byte-identical private copies across
/// `ServerJarProviders.swift`, `PaperDownloader.swift`, and
/// `NeoForgeInstaller.swift` (both `NeoForgeInstaller` and `ForgeInstaller`).
pub fn compare_mc_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let ap = mc_version_components(a);
    let bp = mc_version_components(b);
    let n = ap.len().max(bp.len());
    for i in 0..n {
        let av = ap.get(i).copied().unwrap_or(0);
        let bv = bp.get(i).copied().unwrap_or(0);
        match av.cmp(&bv) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// D-014's floor, applied uniformly on top of any provider's raw id list.
/// Not carried in provisioning logic below this line; a below-floor server
/// reached through import still lists, starts, and runs (D-014's own text).
pub fn filter_to_create_flow_floor(ids: &[String]) -> Vec<String> {
    ids.iter()
        .filter(|v| compare_mc_versions(v, CREATE_FLOW_FLOOR) != std::cmp::Ordering::Less)
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------
// Vanilla (Mojang version manifest)
// ---------------------------------------------------------------------

/// `VanillaDownloader.listVersions()` (`ServerJarProviders.swift:377-389`).
/// Filters to `type == "release"` only (real corpus: drops "snapshot" and
/// "old_alpha" entries) and, unlike Purpur/Paper, never sorts -- the
/// returned order is exactly the manifest's own order.
pub fn vanilla_list_versions(
    raw_manifest_body: &str,
) -> Result<Vec<ServerVersionEntry>, CatalogError> {
    let root: Value = serde_json::from_str(raw_manifest_body)
        .map_err(|e| CatalogError::InvalidJson(e.to_string()))?;
    let list = root
        .get("versions")
        .and_then(Value::as_array)
        .ok_or_else(|| CatalogError::InvalidResponse("Mojang manifest malformed.".to_string()))?;
    let mut out = Vec::new();
    for entry in list {
        if entry.get("type").and_then(Value::as_str) != Some("release") {
            continue;
        }
        if let Some(id) = entry.get("id").and_then(Value::as_str) {
            out.push(simple_stable_entry(id));
        }
    }
    Ok(out)
}

/// `VanillaDownloader.downloadLatest`/`downloadVersion`'s first hop
/// (`ServerJarProviders.swift:341-356`, `:391-401`): resolve `latest.release`
/// (or a caller-supplied release id) to its per-version metadata URL.
pub fn vanilla_resolve_metadata_url(
    raw_manifest_body: &str,
    release_id: Option<&str>,
) -> Result<(String, String), CatalogError> {
    let root: Value = serde_json::from_str(raw_manifest_body)
        .map_err(|e| CatalogError::InvalidJson(e.to_string()))?;
    let release_id = match release_id {
        Some(id) => id.to_string(),
        None => root
            .get("latest")
            .and_then(|l| l.get("release"))
            .and_then(Value::as_str)
            .ok_or_else(|| CatalogError::InvalidResponse("Mojang manifest malformed.".to_string()))?
            .to_string(),
    };
    let versions = root
        .get("versions")
        .and_then(Value::as_array)
        .ok_or_else(|| CatalogError::InvalidResponse("Mojang manifest malformed.".to_string()))?;
    let meta_url = versions
        .iter()
        .find(|v| v.get("id").and_then(Value::as_str) == Some(release_id.as_str()))
        .and_then(|v| v.get("url"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CatalogError::InvalidResponse(format!("No manifest entry for {release_id}."))
        })?
        .to_string();
    Ok((release_id, meta_url))
}

/// The second hop: per-version metadata -> `downloads.server.url`
/// (`ServerJarProviders.swift:358-367`, `:402-409`).
pub fn vanilla_server_download_url(
    raw_per_version_metadata_body: &str,
    release_id_for_error: &str,
) -> Result<String, CatalogError> {
    let root: Value = serde_json::from_str(raw_per_version_metadata_body)
        .map_err(|e| CatalogError::InvalidJson(e.to_string()))?;
    root.get("downloads")
        .and_then(|d| d.get("server"))
        .and_then(|s| s.get("url"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            CatalogError::InvalidResponse(format!("No server download for {release_id_for_error}."))
        })
}

// ---------------------------------------------------------------------
// Purpur (api.purpurmc.org)
// ---------------------------------------------------------------------

/// `PurpurDownloader.listVersions()` (`ServerJarProviders.swift:313-325`).
/// Filters to a `"1."` prefix (Purpur has no `type` field like Vanilla's to
/// filter on) and sorts descending. Every entry is `build_label: None`,
/// `is_stable: true` unconditionally -- the version list carries no
/// per-entry build/channel field to derive either from.
pub fn purpur_list_versions(
    raw_project_body: &str,
) -> Result<Vec<ServerVersionEntry>, CatalogError> {
    let root: Value = serde_json::from_str(raw_project_body)
        .map_err(|e| CatalogError::InvalidJson(e.to_string()))?;
    let versions = root
        .get("versions")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CatalogError::InvalidResponse("Purpur versions list malformed.".to_string())
        })?;
    let mut ids: Vec<String> = versions
        .iter()
        .filter_map(Value::as_str)
        .filter(|v| v.starts_with("1."))
        .map(str::to_string)
        .collect();
    ids.sort_by(|a, b| compare_mc_versions(a, b).reverse());
    Ok(ids.iter().map(|v| simple_stable_entry(v)).collect())
}

/// `PurpurDownloader.downloadLatest`'s target-version pick
/// (`ServerJarProviders.swift:262-289`): prefer Paper's own current stable
/// version verbatim if Purpur's raw (unfiltered) list contains it; else the
/// highest `"1."`-prefixed entry; else the highest entry of any shape.
/// `purpur_versions` is the provider's raw list, not `purpur_list_versions`'s
/// already-filtered output -- the containment check runs against the raw
/// list in MSC 1.
pub fn purpur_pick_target_version(
    purpur_versions: &[String],
    papers_stable_version: Option<&str>,
) -> Option<String> {
    if let Some(paper_stable) = papers_stable_version
        && purpur_versions.iter().any(|v| v == paper_stable)
    {
        return Some(paper_stable.to_string());
    }
    if let Some(best) = purpur_versions
        .iter()
        .filter(|v| v.starts_with("1."))
        .max_by(|a, b| compare_mc_versions(a, b))
    {
        return Some(best.clone());
    }
    purpur_versions
        .iter()
        .max_by(|a, b| compare_mc_versions(a, b))
        .cloned()
}

// ---------------------------------------------------------------------
// Fabric (meta.fabricmc.net)
// ---------------------------------------------------------------------

/// `FabricDownloader.listVersions()` (`ServerJarProviders.swift:456-467`).
/// Filters to the game-version entry's own `stable == true`; order is
/// preserved from the response (Fabric's own list happens to already be
/// newest-first). `loader_version`/`build_label` are always `None` on every
/// row -- populated only by a download's result, never by this picker.
pub fn fabric_list_versions(raw_game_body: &str) -> Result<Vec<ServerVersionEntry>, CatalogError> {
    let root: Value = serde_json::from_str(raw_game_body)
        .map_err(|e| CatalogError::InvalidJson(e.to_string()))?;
    let list = root
        .as_array()
        .ok_or_else(|| CatalogError::InvalidResponse("Fabric game list malformed.".to_string()))?;
    let mut out = Vec::new();
    for entry in list {
        if entry.get("stable").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        if let Some(v) = entry.get("version").and_then(Value::as_str) {
            out.push(simple_stable_entry(v));
        }
    }
    Ok(out)
}

/// The nested loader-list scan (`downloadLatest`/`downloadVersion`'s inline
/// loader-entry selection, `ServerJarProviders.swift:438`, `:488`): first
/// entry with `loader.stable == true`, else `list[0]` (first entry in
/// response order -- NOT the highest build number; the two happen to
/// coincide in the common case but the rule is "first stable").
pub fn fabric_select_loader(raw_loader_list_body: &str) -> Result<String, CatalogError> {
    let root: Value = serde_json::from_str(raw_loader_list_body)
        .map_err(|e| CatalogError::InvalidJson(e.to_string()))?;
    let list = root
        .as_array()
        .filter(|l| !l.is_empty())
        .ok_or_else(|| CatalogError::InvalidResponse("No Fabric loaders.".to_string()))?;
    let entry = list
        .iter()
        .find(|e| {
            e.get("loader")
                .and_then(|l| l.get("stable"))
                .and_then(Value::as_bool)
                == Some(true)
        })
        .unwrap_or(&list[0]);
    entry
        .get("loader")
        .and_then(|l| l.get("version"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| CatalogError::InvalidResponse("Malformed Fabric loader entry.".to_string()))
}

/// `firstStableVersion(from:what:)` (`ServerJarProviders.swift:505-517`):
/// the flat-shape sibling of [`fabric_select_loader`], reused identically
/// for the installer list and for the game-version resolution in
/// `downloadLatest`. First entry with `stable == true`, else `list[0]`.
pub fn fabric_first_stable_version(
    raw_list_body: &str,
    what: &str,
) -> Result<String, CatalogError> {
    let root: Value = serde_json::from_str(raw_list_body)
        .map_err(|e| CatalogError::InvalidJson(e.to_string()))?;
    let list = root
        .as_array()
        .filter(|l| !l.is_empty())
        .ok_or_else(|| CatalogError::InvalidResponse(format!("{what} list empty.")))?;
    let entry = list
        .iter()
        .find(|e| e.get("stable").and_then(Value::as_bool) == Some(true))
        .unwrap_or(&list[0]);
    entry
        .get("version")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| CatalogError::InvalidResponse(format!("{what} entry malformed.")))
}

// ---------------------------------------------------------------------
// Paper (fill.papermc.io v3)
// ---------------------------------------------------------------------

/// `fetchAllVersionsSorted` (`PaperDownloader.swift:163-194`) / the
/// flatten+sort half of `PaperDownloader.listVersions()`
/// (`ServerJarProviders.swift:141-163`) -- the two are, per P7.4's fixture
/// notes, textually near-identical: flatten the v3 project response's
/// per-minor-line groups (or a flat array, whichever shape is present) into
/// one list, reject empty, sort descending by [`compare_mc_versions`].
pub fn paper_flatten_and_sort(raw_project_body: &str) -> Result<Vec<String>, CatalogError> {
    let root: Value = serde_json::from_str(raw_project_body)
        .map_err(|e| CatalogError::InvalidJson(e.to_string()))?;
    let mut all_versions: Vec<String> = Vec::new();
    if let Some(by_group) = root.get("versions").and_then(Value::as_object) {
        for list_any in by_group.values() {
            if let Some(list) = list_any.as_array() {
                all_versions.extend(list.iter().filter_map(Value::as_str).map(str::to_string));
            }
        }
    } else if let Some(flat) = root.get("versions").and_then(Value::as_array) {
        all_versions.extend(flat.iter().filter_map(Value::as_str).map(str::to_string));
    }
    if all_versions.is_empty() {
        return Err(CatalogError::InvalidResponse(
            "Paper v3 versions list empty.".to_string(),
        ));
    }
    all_versions.sort_by(|a, b| compare_mc_versions(a, b).reverse());
    Ok(all_versions)
}

/// One qualifying build, as `fetchBestBuild` (`PaperDownloader.swift:201-272`)
/// selects it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaperBuildSelection {
    pub build_id: i64,
    /// Uppercased: `"STABLE"`, `"BETA"`, or `"ALPHA"`.
    pub channel: String,
    pub is_stable: bool,
}

impl PaperBuildSelection {
    /// Mirrors `ServerJarProviders.swift`'s `paperVersionEntryV3` label
    /// convention (lines 206-215), the real string this selection ends up
    /// shown with once it becomes a `ServerVersionEntry`: `"build N"` for a
    /// stable pick, `"build N · beta"` for a beta-or-alpha fallback pick --
    /// yes, alpha too; both channels share one fallback slot there.
    pub fn build_label(&self) -> String {
        if self.is_stable {
            format!("build {}", self.build_id)
        } else {
            format!("build {} · beta", self.build_id)
        }
    }
}

/// `fetchBestBuild(forVersion:includeExperimental:)`
/// (`PaperDownloader.swift:201-272`). Non-experimental: only STABLE-channel
/// entries qualify; highest numeric id wins. Experimental: only BETA/ALPHA
/// entries qualify, and the whole call returns `None` if the version has
/// ANY STABLE build at all (`hasStableBuild` guard) -- a real released
/// version's beta-channel history never lets it appear on the experimental
/// track. A candidate is invisible to the id comparison entirely (not just
/// a worse candidate) unless it carries a `downloads."server:default".url`.
/// Never errors: a malformed/non-array body is `None`, the same as MSC 1's
/// `try?`-wrapped parse.
pub fn paper_select_build(
    raw_builds_body: &str,
    include_experimental: bool,
) -> Option<PaperBuildSelection> {
    let builds: Value = serde_json::from_str(raw_builds_body).ok()?;
    let builds = builds.as_array()?;

    let mut has_stable_build = false;
    let mut best_id: Option<i64> = None;
    let mut best: Option<PaperBuildSelection> = None;

    for entry in builds {
        let Some(channel) = entry.get("channel").and_then(Value::as_str) else {
            continue;
        };
        let channel_upper = channel.to_uppercase();
        if channel_upper == "STABLE" {
            has_stable_build = true;
        }
        let qualifies = if include_experimental {
            channel_upper == "BETA" || channel_upper == "ALPHA"
        } else {
            channel_upper == "STABLE"
        };
        if !qualifies {
            continue;
        }
        let Some(build_id) = entry.get("id").and_then(Value::as_i64) else {
            continue;
        };
        let has_download = entry
            .get("downloads")
            .and_then(|d| d.get("server:default"))
            .and_then(|sd| sd.get("url"))
            .and_then(Value::as_str)
            .is_some();
        if !has_download {
            continue;
        }
        if best_id.is_none_or(|b| build_id > b) {
            best_id = Some(build_id);
            best = Some(PaperBuildSelection {
                build_id,
                is_stable: channel_upper == "STABLE",
                channel: channel_upper,
            });
        }
    }

    if include_experimental && has_stable_build {
        return None;
    }
    best
}

/// The `maxCandidates = 20` walk `fetchAvailableVersions`
/// (`PaperDownloader.swift:69-110`) runs: stop after 20 candidates are
/// *tried*, regardless of how few results have been collected, or once
/// `limit` results are found -- either guard ends the walk. `fetch_best`
/// is the caller's per-candidate lookup (in production, a fetch of that
/// version's builds through [`paper_select_build`]); this function owns
/// only the walk's own termination rule, since the per-candidate fetch is
/// I/O the caller (not `msc-domain`) performs.
pub const PAPER_MAX_CANDIDATES: usize = 20;

pub struct PaperWalkOutcome<T> {
    pub tried: usize,
    pub results: Vec<T>,
}

pub fn paper_walk_candidates<T>(
    candidates: &[String],
    limit: usize,
    mut fetch_best: impl FnMut(&str) -> Option<T>,
) -> PaperWalkOutcome<T> {
    let mut tried = 0usize;
    let mut results = Vec::new();
    for version in candidates {
        if !(tried < PAPER_MAX_CANDIDATES && results.len() < limit) {
            break;
        }
        tried += 1;
        if let Some(v) = fetch_best(version) {
            results.push(v);
        }
    }
    PaperWalkOutcome { tried, results }
}

/// `findStableCeiling` (`PaperDownloader.swift:277-287`): the highest
/// version (among the first 15, newest-first) with at least one STABLE
/// build -- used as the floor above which the experimental track's
/// candidates must sit. No dedicated P7.4 fixture exercises this
/// specifically; ported directly from source since [`paper_walk_candidates`]
/// and [`paper_select_build`] alone don't compose it (it searches
/// non-experimental while the caller wants experimental results).
pub const PAPER_STABLE_CEILING_SEARCH_DEPTH: usize = 15;

pub fn paper_find_stable_ceiling(
    sorted_versions: &[String],
    mut fetch_best_stable: impl FnMut(&str) -> Option<PaperBuildSelection>,
) -> Option<String> {
    for version in sorted_versions
        .iter()
        .take(PAPER_STABLE_CEILING_SEARCH_DEPTH)
    {
        if fetch_best_stable(version).is_some() {
            return Some(version.clone());
        }
    }
    None
}

// ---------------------------------------------------------------------
// NeoForge (maven.neoforged.net)
// ---------------------------------------------------------------------

/// Both NeoForge's and Forge's metadata endpoints are hand-scanned for
/// literal `<version>`/`</version>` substring pairs (`NeoForgeInstaller.swift:52-56`,
/// `:421-425`) -- neither ever uses a real XML parser. Garbled/non-XML
/// input that contains zero complete pairs silently yields an empty list,
/// not a parse error.
fn scrape_xml_version_tags(xml: &str) -> Vec<String> {
    let mut versions = Vec::new();
    let mut rest = xml;
    while let Some(open_idx) = rest.find("<version>") {
        let after_open = &rest[open_idx + "<version>".len()..];
        let Some(close_idx) = after_open.find("</version>") else {
            break;
        };
        versions.push(after_open[..close_idx].to_string());
        rest = &after_open[close_idx + "</version>".len()..];
    }
    versions
}

/// `NeoForgeInstaller.listVersionPairs`'s stable filter
/// (`NeoForgeInstaller.swift:57`): `!version.contains('-')` -- there is no
/// separate channel field like Paper's; a hyphen is the only pre-release
/// signal.
pub fn neoforge_stable_versions(raw_metadata_xml: &str) -> Vec<String> {
    scrape_xml_version_tags(raw_metadata_xml)
        .into_iter()
        .filter(|v| !v.contains('-'))
        .collect()
}

/// `minecraftVersion(forNeoForge:)` (`NeoForgeInstaller.swift:224-231`).
/// Classic scheme (major < 26): `"1.{major}.{minor}"`, or just
/// `"1.{major}"` when minor is 0. New scheme (major >= 26): no `"1."`
/// prefix. A third/fourth build-number component never appears in the
/// derived version.
pub fn neoforge_minecraft_version(neoforge_version: &str) -> String {
    let core = neoforge_version
        .split('-')
        .next()
        .unwrap_or(neoforge_version);
    let comps: Vec<i64> = core
        .split('.')
        .filter_map(|p| p.parse::<i64>().ok())
        .collect();
    if comps.len() < 2 {
        return core.to_string();
    }
    let (major, minor) = (comps[0], comps[1]);
    if major >= 26 {
        if minor == 0 {
            major.to_string()
        } else {
            format!("{major}.{minor}")
        }
    } else if minor == 0 {
        format!("1.{major}")
    } else {
        format!("1.{major}.{minor}")
    }
}

/// `listVersionPairs`'s full pipeline (`NeoForgeInstaller.swift:43-84`):
/// scrape, stable-filter, pair with the derived MC version, dedup by the
/// combined `"{mc}—{nfv}"` id (every stable build is listed, not just the
/// newest per MC version -- modpacks pin exact builds), sort descending by
/// MC version then, within a tie, by NeoForge version.
pub fn neoforge_build_entries(raw_metadata_xml: &str) -> Vec<ServerVersionEntry> {
    let stable = neoforge_stable_versions(raw_metadata_xml);
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    for nfv in &stable {
        let mc = neoforge_minecraft_version(nfv);
        let id = format!("{mc}—{nfv}");
        if !seen.insert(id.clone()) {
            continue;
        }
        entries.push(ServerVersionEntry {
            id,
            display_label: mc.clone(),
            mc_version: mc,
            loader_version: Some(nfv.clone()),
            build_label: Some(format!("NeoForge {nfv}")),
            is_stable: true,
        });
    }
    entries.sort_by(|a, b| {
        let mc_order = compare_mc_versions(&a.mc_version, &b.mc_version).reverse();
        if mc_order != std::cmp::Ordering::Equal {
            return mc_order;
        }
        compare_mc_versions(
            a.loader_version.as_deref().unwrap_or(""),
            b.loader_version.as_deref().unwrap_or(""),
        )
        .reverse()
    });
    entries
}

/// `latestStableVersion`'s pick, split from its XML fetch/scrape so the
/// pure "highest of an already-filtered list" step can be tested directly
/// against a pre-filtered list, not just via `raw_metadata_xml` --
/// `latestStableVersion` (`NeoForgeInstaller.swift:200-220`) has no
/// explicit empty-list guard text; it falls out of `stable.max(by:)`
/// returning `nil` on an empty array, caught by the `guard let`.
pub fn neoforge_pick_latest_stable(stable_versions: &[String]) -> Result<String, CatalogError> {
    stable_versions
        .iter()
        .max_by(|a, b| compare_mc_versions(a, b))
        .cloned()
        .ok_or(CatalogError::NoStableVersion)
}

pub fn neoforge_latest_stable(raw_metadata_xml: &str) -> Result<String, CatalogError> {
    neoforge_pick_latest_stable(&neoforge_stable_versions(raw_metadata_xml))
}

// ---------------------------------------------------------------------
// Forge (files.minecraftforge.net / maven.minecraftforge.net)
// ---------------------------------------------------------------------

/// `parseMavenVersion` (`NeoForgeInstaller.swift:453-459`). Splits at the
/// FIRST `-` only (not the last) -- a Forge build string could in
/// principle contain a further dash itself and still split correctly into
/// `(mc, forge)`.
pub fn forge_parse_maven_version(version: &str) -> Option<(String, String)> {
    let dash = version.find('-')?;
    let mc = &version[..dash];
    let forge = &version[dash + 1..];
    if !mc.chars().next().is_some_and(|c| c.is_ascii_digit()) || forge.is_empty() {
        return None;
    }
    Some((mc.to_string(), forge.to_string()))
}

/// `parseMavenMetadata` (`NeoForgeInstaller.swift:418-449`). Unlike
/// NeoForge, Forge's raw maven list is NOT pre-filtered to stable-only, so
/// `is_stable` is a genuine per-entry computed property
/// (`!forge_version.contains('-')`) that actually varies row to row. Two
/// entries can share the same `mc_version` with different Forge builds --
/// dedup is keyed on the combined `"{mc}—{forge}"` id, never on `mc` alone,
/// because modpacks pin exact non-recommended Forge builds.
pub fn forge_parse_maven_metadata(raw_metadata_xml: &str) -> Vec<ServerVersionEntry> {
    let maven_versions = scrape_xml_version_tags(raw_metadata_xml);
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    for raw in &maven_versions {
        let Some((mc, forge)) = forge_parse_maven_version(raw) else {
            continue;
        };
        let id = format!("{mc}—{forge}");
        if !seen.insert(id.clone()) {
            continue;
        }
        entries.push(ServerVersionEntry {
            id,
            display_label: mc.clone(),
            mc_version: mc,
            is_stable: !forge.contains('-'),
            loader_version: Some(forge.clone()),
            build_label: Some(format!("Forge {forge}")),
        });
    }
    entries.sort_by(|a, b| {
        let mc_order = compare_mc_versions(&a.mc_version, &b.mc_version).reverse();
        if mc_order != std::cmp::Ordering::Equal {
            return mc_order;
        }
        compare_mc_versions(
            a.loader_version.as_deref().unwrap_or(""),
            b.loader_version.as_deref().unwrap_or(""),
        )
        .reverse()
    });
    entries
}

/// `latestRecommendedVersion` (`NeoForgeInstaller.swift:506-528`). Reads
/// `promotions_slim.json` -- a source SEPARATE from the maven-metadata XML
/// `listVersionPairs`/`parseMavenMetadata` read, and known (per P7.3's
/// corpus findings) to be more current than that XML's own stale
/// `<latest>`/`<release>` tags. Filters keys ending `-recommended`, strips
/// the suffix to get each candidate's MC version, and picks the entry whose
/// MC version compares highest -- deliberately NOT the highest Forge build
/// number, which would pick the wrong MC line (the source comment is
/// explicit about this).
pub fn forge_latest_recommended(
    raw_promotions_body: &str,
) -> Result<(String, String), CatalogError> {
    let root: Value = serde_json::from_str(raw_promotions_body)
        .map_err(|e| CatalogError::InvalidJson(e.to_string()))?;
    let promos = root
        .get("promos")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            CatalogError::InvalidResponse("promotions response malformed".to_string())
        })?;
    let suffix = "-recommended";
    let candidates: Vec<(String, String)> = promos
        .iter()
        .filter_map(|(key, value)| {
            let mc = key.strip_suffix(suffix)?;
            let forge = value.as_str()?;
            Some((mc.to_string(), forge.to_string()))
        })
        .collect();
    candidates
        .into_iter()
        .max_by(|a, b| compare_mc_versions(&a.0, &b.0))
        .ok_or(CatalogError::NoStableVersion)
}
