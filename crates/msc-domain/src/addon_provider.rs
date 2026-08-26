//! Add-on provider parsing rules: Modrinth (catalog search, exact-hash
//! identity, primary-file selection), Hangar (plugin-source download
//! resolution), CurseForge (batch file/mod metadata for modpack import and
//! the D-027 blocked-file workflow), GitHub Releases (plugin-source jar
//! selection), and the direct-URL dispatch case.
//!
//! Ported from `ModrinthAPI.swift`, `HangarAPI.swift`, `CurseForgeAPI.swift`,
//! `GitHubReleaseChecker.swift`, and `AppViewModel+ComponentsVersions.swift`'s
//! `fetchOnlineVersion` `.direct` case, per `docs/msc2/addons/phase8-scope.md`
//! and `fixtures/addon-providers/` (P8.4). Every function here takes
//! already-fetched status codes/response bodies -- `msc-domain` carries no
//! I/O; the real HTTP transport is `msc-infrastructure`'s job (P8.13).
//!
//! **P8.15 amendment:** P8.10/P8.13 never built `ModrinthAPI.project(idOrSlug:)`
//! or `.projectVersions(idOrSlug:loaders:gameVersion:)` -- both are real
//! gaps `installRequiredDependencies` (`AppViewModel+ModManagement.swift:
//! 271-328`) needs and no earlier Phase 8 step happened to touch, since
//! neither the search flow (P8.4) nor the hash-identity/update flow (P8.5)
//! calls either one. [`ModrinthProjectSummary`]/[`modrinth_decode_project`]
//! and `ModrinthVersionInfo.dependencies` close that gap here rather than
//! inventing a parallel type in `msc-application`.

use crate::addon_dependency::ModrinthDependency;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddonProviderError {
    /// A non-2xx HTTP status, formatted the same way MSC 1's own
    /// per-provider status guards do: `"{Provider} returned status {code}."`.
    Network(String),
    /// `ModrinthAPIError.noJarAsset` (`ModrinthAPI.swift:95-97`): a version
    /// exists but none of its files are a usable jar.
    NoJarAsset,
    /// `ModrinthAPIError.noCompatibleVersion` / `HangarAPIError.noCompatibleVersion`:
    /// the provider's own version list came back empty for this query.
    NoCompatibleVersion { provider: &'static str },
    /// `CurseForgeAPIError.missingAPIKey` (`CurseForgeAPI.swift:116-117`):
    /// no key configured, checked before any request is built.
    MissingApiKey,
    /// `CurseForgeAPIError.unauthorized` (`CurseForgeAPI.swift:136-138`):
    /// a 401/403 is reported distinctly from a generic network error.
    Unauthorized,
    /// The `.direct` dispatch case's `URL(string:)` guard
    /// (`AppViewModel+ComponentsVersions.swift:304`).
    InvalidDirectUrl,
}

impl std::fmt::Display for AddonProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddonProviderError::Network(m) => write!(f, "{m}"),
            AddonProviderError::NoJarAsset => write!(f, "Modrinth release has no JAR download."),
            AddonProviderError::NoCompatibleVersion { provider } => {
                write!(f, "No compatible version found on {provider}.")
            }
            AddonProviderError::MissingApiKey => {
                write!(f, "No CurseForge API key is configured.")
            }
            AddonProviderError::Unauthorized => write!(
                f,
                "CurseForge rejected the API key (401/403). Check the key in Preferences."
            ),
            AddonProviderError::InvalidDirectUrl => write!(f, "Invalid direct download URL."),
        }
    }
}

impl std::error::Error for AddonProviderError {}

fn malformed(what: &str, e: serde_json::Error) -> AddonProviderError {
    AddonProviderError::Network(format!("Malformed {what} response: {e}"))
}

// --- Modrinth ---

/// `ModrinthAPI.facets(projectType:loaders:gameVersion:)` (line 234-246).
/// Returns the compact JSON-array-of-arrays string Modrinth's `facets` query
/// param expects. The `plugin` project type gets a special OR group with
/// `mod` (line 234-236) so cross-platform projects like Geyser/Floodgate,
/// typed "mod" on Modrinth, still surface in a plugin-kind catalog search.
pub fn modrinth_facets(
    project_type: &str,
    loaders: &[String],
    game_version: Option<&str>,
) -> String {
    let mut groups: Vec<Vec<String>> = Vec::new();
    if project_type == "plugin" {
        groups.push(vec![
            "project_type:plugin".to_string(),
            "project_type:mod".to_string(),
        ]);
    } else {
        groups.push(vec![format!("project_type:{project_type}")]);
    }
    if !loaders.is_empty() {
        groups.push(loaders.iter().map(|l| format!("categories:{l}")).collect());
    }
    if let Some(v) = game_version
        && !v.is_empty()
    {
        groups.push(vec![format!("versions:{v}")]);
    }
    serde_json::to_string(&groups).expect("Vec<Vec<String>> always serializes")
}

/// P8.15 amendment: `ModrinthAPI.projectVersions(idOrSlug:loaders:gameVersion:)`'s
/// query-param construction (`ModrinthAPI.swift:280-288`) -- `loaders`
/// becomes a bare JSON string array (`["fabric"]`), `game_version` becomes
/// a single-element one (`["1.21.1"]`); either is omitted entirely from
/// the returned list when empty/absent, matching source's own
/// `items.isEmpty ? nil : items` (an empty query list here means "send no
/// query string at all," not "send an empty one").
pub fn modrinth_project_versions_query(
    loaders: &[String],
    game_version: Option<&str>,
) -> Vec<(String, String)> {
    let mut params = Vec::new();
    if !loaders.is_empty() {
        let json = serde_json::to_string(loaders).expect("Vec<String> always serializes");
        params.push(("loaders".to_string(), json));
    }
    if let Some(v) = game_version
        && !v.is_empty()
    {
        let json = serde_json::to_string(&[v]).expect("[&str; 1] always serializes");
        params.push(("game_versions".to_string(), json));
    }
    params
}

/// `ModrinthAPI.search(query:...)`'s `index` param (line 266): an empty
/// query browses by download count; any real query sorts by relevance.
pub fn modrinth_search_index(query: &str) -> &'static str {
    if query.is_empty() {
        "downloads"
    } else {
        "relevance"
    }
}

/// Computed purely from `server_side` (`ModrinthAPI.swift:195`); `client_side`
/// is irrelevant to this flag.
pub fn modrinth_is_client_only(server_side: &str) -> bool {
    server_side == "unsupported"
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModrinthSearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    /// Modrinth's search response calls this `description` too -- a short
    /// one-line summary, not the project's full Markdown body (that's
    /// `ModrinthProjectSummary`'s job, fetched per-project, not per-hit).
    #[serde(default)]
    pub description: String,
    /// The project owner's username. Modrinth's wire field is `author`,
    /// same as `CatalogItemDTO.author` -- no rename needed here either.
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub downloads: i64,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub server_side: Option<String>,
}

impl ModrinthSearchHit {
    pub fn is_client_only(&self) -> bool {
        self.server_side.as_deref() == Some("unsupported")
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModrinthSearchResult {
    #[serde(default)]
    pub hits: Vec<ModrinthSearchHit>,
    pub total_hits: u64,
}

/// `ModrinthSearchResult` decode (line 124-127), via the shared
/// `.convertFromSnakeCase` decoder -- `serde`'s field names already match
/// Modrinth's wire snake_case, so no rename is needed.
pub fn modrinth_decode_search(body: &str) -> Result<ModrinthSearchResult, AddonProviderError> {
    serde_json::from_str(body).map_err(|e| malformed("Modrinth search", e))
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModrinthVersionFile {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub hashes: HashMap<String, String>,
    #[serde(default)]
    pub size: u64,
}

/// `ModrinthVersionInfo.primaryFile` (line 158-160): the flagged-primary
/// file wins regardless of array order; only when none is flagged primary
/// does this fall back to the array's first element.
pub fn modrinth_primary_file(files: &[ModrinthVersionFile]) -> Option<&ModrinthVersionFile> {
    files.iter().find(|f| f.primary).or_else(|| files.first())
}

/// The older single-plugin fetcher's jar selection (`ModrinthAPI.swift:92-93`),
/// distinct from [`modrinth_primary_file`]: the primary file must ALSO end
/// in `.jar` to be accepted by the first clause.
pub fn modrinth_legacy_jar_file(
    files: &[ModrinthVersionFile],
) -> Result<&ModrinthVersionFile, AddonProviderError> {
    files
        .iter()
        .find(|f| f.primary && f.filename.to_lowercase().ends_with(".jar"))
        .or_else(|| {
            files
                .iter()
                .find(|f| f.filename.to_lowercase().ends_with(".jar"))
        })
        .ok_or(AddonProviderError::NoJarAsset)
}

/// `ModrinthAPI.fetchLatest`'s HTTP status guard (line 59-63): any status
/// outside 200..<300 throws `.networkError`, with the code interpolated.
pub fn ensure_modrinth_ok(status: u16) -> Result<(), AddonProviderError> {
    if !(200..300).contains(&status) {
        return Err(AddonProviderError::Network(format!(
            "Modrinth returned status {status}."
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModrinthVersionSummary {
    #[serde(default)]
    pub files: Vec<ModrinthVersionFile>,
}

/// `ModrinthAPI.fetchLatest(slug:mcVersion:)`: status guard, then
/// `versions.first` (line 87-89), then legacy jar-file selection.
pub fn modrinth_legacy_fetch_latest(
    status: u16,
    versions: &[ModrinthVersionSummary],
) -> Result<&ModrinthVersionFile, AddonProviderError> {
    ensure_modrinth_ok(status)?;
    let latest = versions
        .first()
        .ok_or(AddonProviderError::NoCompatibleVersion {
            provider: "Modrinth",
        })?;
    modrinth_legacy_jar_file(&latest.files)
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModrinthVersionInfo {
    pub id: String,
    pub project_id: String,
    pub version_number: String,
    #[serde(default)]
    pub files: Vec<ModrinthVersionFile>,
    /// P8.15 amendment: `ModrinthVersionInfo.dependencies`, read by
    /// `installRequiredDependencies` off the version that was just
    /// resolved/downloaded (not a separate fetch) -- absent from every
    /// earlier Phase 8 step's own use of this struct (identity/update
    /// checks never needed it), `#[serde(default)]` so those existing
    /// decode sites are unaffected by a response that omits it.
    #[serde(default)]
    pub dependencies: Vec<ModrinthDependency>,
}

/// P8.15 amendment: `ModrinthAPI.project(idOrSlug:)`'s response shape --
/// only the fields `installRequiredDependencies` actually reads (`project.slug`,
/// used for both already-installed checks and the version-list lookup).
#[derive(Debug, Clone, Deserialize)]
pub struct ModrinthProjectSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
}

pub fn modrinth_decode_project(body: &str) -> Result<ModrinthProjectSummary, AddonProviderError> {
    serde_json::from_str(body).map_err(|e| malformed("Modrinth project", e))
}

pub fn modrinth_decode_project_versions(
    body: &str,
) -> Result<Vec<ModrinthVersionInfo>, AddonProviderError> {
    serde_json::from_str(body).map_err(|e| malformed("Modrinth project versions", e))
}

/// `ModrinthAPI.versionFromHash(_:)` (line 375-390): the exact-identity
/// endpoint. A 404 means "not hosted on Modrinth -- expected, not an
/// error" (line 387) and resolves to `Ok(None)`; every other non-2xx
/// status throws.
pub fn modrinth_version_from_hash(
    status: u16,
    body: &str,
) -> Result<Option<ModrinthVersionInfo>, AddonProviderError> {
    if status == 404 {
        return Ok(None);
    }
    ensure_modrinth_ok(status)?;
    let info: ModrinthVersionInfo =
        serde_json::from_str(body).map_err(|e| malformed("Modrinth version_file", e))?;
    Ok(Some(info))
}

/// `ModrinthAPI.versionsFromHashes(_:)`'s empty-input short circuit (line
/// 398): `None` means "skip the request, the result is empty" -- the real
/// batch response decode is the caller's (infrastructure) job.
pub fn modrinth_versions_from_hashes_plan(sha512s: &[String]) -> Option<Vec<String>> {
    if sha512s.is_empty() {
        None
    } else {
        Some(sha512s.to_vec())
    }
}

/// `ModrinthAPI.projects(ids:)`'s empty-input short circuit (line 298).
pub fn modrinth_projects_plan(ids: &[String]) -> Option<Vec<String>> {
    if ids.is_empty() {
        None
    } else {
        Some(ids.to_vec())
    }
}

/// `ModrinthAPI.latestVersionsForHashes(_:loaders:gameVersions:)`'s POST
/// body (line 415-417): `loaders`/`game_versions` keys are only present
/// when non-empty -- an empty filter is omitted, not sent as `[]`.
pub fn modrinth_latest_versions_body(
    sha512s: &[String],
    loaders: &[String],
    game_versions: &[String],
) -> Value {
    let mut body = serde_json::Map::new();
    body.insert("hashes".to_string(), Value::from(sha512s.to_vec()));
    body.insert("algorithm".to_string(), Value::from("sha512"));
    if !loaders.is_empty() {
        body.insert("loaders".to_string(), Value::from(loaders.to_vec()));
    }
    if !game_versions.is_empty() {
        body.insert(
            "game_versions".to_string(),
            Value::from(game_versions.to_vec()),
        );
    }
    Value::Object(body)
}

// --- Hangar ---

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HangarDownload {
    #[serde(rename = "downloadUrl", default)]
    pub download_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HangarVersion {
    pub name: String,
    #[serde(default)]
    pub downloads: HashMap<String, HangarDownload>,
}

#[derive(Debug, Deserialize)]
struct HangarResultEnvelope {
    result: Vec<HangarVersion>,
}

pub fn hangar_decode_versions(body: &str) -> Result<Vec<HangarVersion>, AddonProviderError> {
    let env: HangarResultEnvelope =
        serde_json::from_str(body).map_err(|e| malformed("Hangar versions", e))?;
    Ok(env.result)
}

/// `HangarAPI.fetchLatest`'s empty-result guard (line 83-85).
pub fn hangar_select_latest(
    versions: &[HangarVersion],
) -> Result<&HangarVersion, AddonProviderError> {
    versions
        .first()
        .ok_or(AddonProviderError::NoCompatibleVersion { provider: "Hangar" })
}

/// A path segment percent-encoder matching Foundation's `.urlPathAllowed`
/// closely enough for a version-name path segment: unreserved characters
/// pass through, everything else becomes `%XX`.
fn percent_encode_path_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `HangarAPI.fetchLatest`'s download-URL selection (line 89-95): the
/// API-provided PAPER download URL wins when present and non-empty;
/// otherwise a fallback endpoint is computed from the version name, with
/// that name percent-encoded so an unusual release name can't produce a
/// malformed URL.
pub fn hangar_download_url(author: &str, slug: &str, version: &HangarVersion) -> String {
    if let Some(url) = version
        .downloads
        .get("PAPER")
        .and_then(|d| d.download_url.as_deref())
        && !url.is_empty()
    {
        return url.to_string();
    }
    format!(
        "https://hangar.papermc.io/api/v1/projects/{author}/{slug}/versions/{}/PAPER/download",
        percent_encode_path_segment(&version.name)
    )
}

// --- CurseForge ---

/// `CurseForgeAPI.post`'s trimmed, empty-checked key guard (line 116-117),
/// which runs before the URL is even built.
pub fn curseforge_require_api_key(api_key: &str) -> Result<(), AddonProviderError> {
    if api_key.trim().is_empty() {
        return Err(AddonProviderError::MissingApiKey);
    }
    Ok(())
}

/// `CurseForgeAPI.post`'s status guard (line 136-141): 401/403 are checked
/// (and reported as `.unauthorized`) before the general 200..<300 range
/// check, so a bad key surfaces a specific message rather than a generic
/// "CurseForge returned status N."
pub fn ensure_curseforge_ok(status: u16) -> Result<(), AddonProviderError> {
    if status == 401 || status == 403 {
        return Err(AddonProviderError::Unauthorized);
    }
    if !(200..300).contains(&status) {
        return Err(AddonProviderError::Network(format!(
            "CurseForge returned status {status}."
        )));
    }
    Ok(())
}

/// `CurseForgeAPI.batched(_:_:)` (line 103): duplicate IDs collapse to one
/// request slot, and the resulting batch order is deterministic (ascending)
/// rather than insertion order.
pub fn curseforge_batched_ids(ids: &[i64]) -> Vec<i64> {
    let set: BTreeSet<i64> = ids.iter().copied().collect();
    set.into_iter().collect()
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurseForgeFile {
    pub id: i64,
    #[serde(rename = "modId")]
    pub mod_id: i64,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "downloadUrl", default)]
    pub download_url: Option<String>,
    /// P8.20 amendment: CurseForge's own reported byte length for this
    /// exact file (confirmed present on every real capture in
    /// `corpus/addons/curseforge/`, e.g. `mods-files-blocked-entityculling.json`'s
    /// `fileLength`) — D-027's manual-upload completion path
    /// (`curseforge_manual.rs`) sizes its per-file ceiling to this value
    /// rather than a flat cap, per `phase8-api.md`'s own contract note.
    /// `#[serde(default)]` so every earlier decode site (none of which
    /// needed this field) is unaffected.
    #[serde(rename = "fileLength", default)]
    pub file_length: u64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct CurseForgeModLinks {
    #[serde(rename = "websiteUrl", default)]
    pub website_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurseForgeMod {
    pub id: i64,
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub links: CurseForgeModLinks,
}

impl CurseForgeMod {
    pub fn website_url(&self) -> Option<&str> {
        self.links.website_url.as_deref()
    }
}

#[derive(Debug, Deserialize)]
struct CurseForgeDataEnvelope<T> {
    data: Vec<T>,
}

/// `CurseForgeAPI.files(fileIds:apiKey:)` (line 78's doc comment: "Files
/// the API omits are simply absent from the result" -- a requested ID
/// CurseForge doesn't recognize is neither an error nor a null placeholder,
/// just missing from the decoded array).
pub fn curseforge_decode_files(body: &str) -> Result<Vec<CurseForgeFile>, AddonProviderError> {
    let env: CurseForgeDataEnvelope<CurseForgeFile> =
        serde_json::from_str(body).map_err(|e| malformed("CurseForge files", e))?;
    Ok(env.data)
}

/// `CurseForgeAPI.mods(modIds:apiKey:)` (line 32-41).
pub fn curseforge_decode_mods(body: &str) -> Result<Vec<CurseForgeMod>, AddonProviderError> {
    let env: CurseForgeDataEnvelope<CurseForgeMod> =
        serde_json::from_str(body).map_err(|e| malformed("CurseForge mods", e))?;
    Ok(env.data)
}

// --- GitHub Releases ---

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GitHubRelease {
    #[serde(default)]
    pub assets: Vec<GitHubAsset>,
}

pub fn github_decode_release(body: &str) -> Result<GitHubRelease, AddonProviderError> {
    serde_json::from_str(body).map_err(|e| malformed("GitHub release", e))
}

/// `GitHubReleaseChecker.fetchLatestRelease` (line 86-91): the first asset
/// whose name ends `.jar`, case-insensitively, in API response array order
/// -- no filename-content preference for a base plugin over its addon
/// modules. Returns `None` (not an error) when no asset qualifies; the
/// caller decides whether that's fatal.
pub fn github_select_jar_asset(assets: &[GitHubAsset]) -> Option<&GitHubAsset> {
    assets
        .iter()
        .find(|a| a.name.to_lowercase().ends_with(".jar"))
}

// --- Direct URL ---

/// The `.direct` case's only validation (`AppViewModel+ComponentsVersions.swift:304`):
/// `URL(string:)` succeeding. No scheme/host requirement beyond what
/// Foundation's URL initializer itself rejects -- approximated here as "has
/// a scheme, no whitespace anywhere."
fn is_valid_direct_url(s: &str) -> bool {
    if s.chars().any(|c| c.is_whitespace()) {
        return false;
    }
    match s.split_once("://") {
        Some((scheme, rest)) => {
            !scheme.is_empty()
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
                && !rest.is_empty()
        }
        None => false,
    }
}

/// `fetchOnlineVersion`'s `.direct` case (line 303-309): the literal
/// version string `"(direct)"` stands in for a version identifier since a
/// raw URL carries no version metadata. Returns `(version, download_url)`.
pub fn direct_dispatch(source_url: &str) -> Result<(String, String), AddonProviderError> {
    if !is_valid_direct_url(source_url) {
        return Err(AddonProviderError::InvalidDirectUrl);
    }
    Ok(("(direct)".to_string(), source_url.to_string()))
}
