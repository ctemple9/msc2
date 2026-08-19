//! The server-jar provider boundary: the first place MSC 2 makes an
//! outbound network request on a user's behalf.
//!
//! Architecture call, decided for you rather than asked (per this phase's
//! own "decided without asking" precedent): a synchronous/blocking HTTP
//! client (`ureq`, rustls TLS backend via its default feature set) rather
//! than pulling `reqwest`+`tokio` into this crate. Every existing
//! `msc-infrastructure` trait (`FileSystem`, `process`) is synchronous;
//! this stays consistent. The async agent layer (`msc-agent`, which
//! already depends on `tokio`) can wrap a blocking call in
//! `spawn_blocking` when it gets there in a later step — that's not this
//! step's job.
//!
//! [`Transport`] is the boundary itself: fetch bytes from a URL, capped at
//! a caller-given size, with a request timeout. [`HttpTransport`] is the
//! real implementation; `tests/jar_provider.rs`'s own `FakeTransport`
//! serves canned bytes fed from `corpus/providers/`, per this phase's own
//! decided-for-you note: "Provisioning tests never touch the network."
//! Every successful jar/installer download routes through
//! [`crate::download_staging::stage_download`].
//!
//! Per-family functions compose [`Transport::get`] with
//! `msc_domain::server_versions`'s pure parsing (P7.10): fetch bytes, hand
//! them to the parser, follow whatever URL it resolves. Ported from
//! `ServerJarProviders.swift` (Vanilla/Paper/Purpur/Fabric's `downloadLatest`/
//! `listVersions`) and `NeoForgeInstaller.swift` (NeoForge's/Forge's
//! `listVersionPairs` metadata fetch and their installer-jar download step
//! — *running* the installer is P7.14's `loader_installer`, not this step's
//! job; this step only fetches the installer jar itself, the same as it
//! fetches any other family's jar).

use std::fmt;
use std::path::Path;
use std::time::Duration;

use msc_domain::server_versions::{self, CatalogError};

use crate::download_staging::{self, CachedFile, DownloadStagingError};
use crate::fs::FileSystem;

const USER_AGENT: &str = "MinecraftServerController/2.0 (msc2 agent)";

/// Reads a provider host override for P7.27's portable smoke -- a local
/// fake HTTP server serving `corpus/providers/` responses, with no real
/// network reachable. Defaults to the real host every existing caller
/// already hardcoded, so nothing about real provisioning changes unless
/// the env var is actually set. An empty value is treated the same as
/// unset (a smoke harness that sets `FOO=""` by accident should still
/// get the real host, not an empty base that turns every URL relative).
fn provider_base(env_var: &str, default: &str) -> String {
    std::env::var(env_var)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Catalog/metadata responses (JSON or XML) are small — bound generously
/// but well below what a runaway/malicious response could exhaust memory
/// with.
pub const CATALOG_MAX_BYTES: u64 = 20 * 1024 * 1024; // 20 MB

/// Real server jars run 40-65 MB in the P7.3 corpus evidence; installer
/// jars are smaller. 300 MB leaves headroom for a large modpack-adjacent
/// jar without accepting an unbounded stream.
pub const JAR_MAX_BYTES: u64 = 300 * 1024 * 1024; // 300 MB

/// Connect + the whole exchange must complete within this long — long
/// enough for a slow real download, short enough that a hung provider
/// degrades honestly (per this phase's "honest degradation" requirement)
/// instead of blocking a create/version-change operation forever.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub enum JarProviderError {
    /// Connection failure, non-2xx status, or any other transport-level
    /// problem. `what` is a call-site label, the same convention
    /// `ensure_http_ok`/`ensureOK` already established in P7.10.
    Network(String),
    Timeout(String),
    ResponseTooLarge {
        what: String,
        max_bytes: u64,
    },
    /// A parse/shape failure from `msc_domain::server_versions`.
    Catalog(CatalogError),
    /// Response bytes weren't valid UTF-8 where a caller needed text
    /// (`server_versions`'s functions all take `&str`).
    InvalidUtf8 {
        what: String,
    },
    Staging(DownloadStagingError),
}

impl fmt::Display for JarProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JarProviderError::Network(m) => write!(f, "{m}"),
            JarProviderError::Timeout(what) => write!(f, "{what} timed out."),
            JarProviderError::ResponseTooLarge { what, max_bytes } => {
                write!(f, "{what} exceeded the {max_bytes}-byte size cap.")
            }
            JarProviderError::Catalog(e) => write!(f, "{e}"),
            JarProviderError::InvalidUtf8 { what } => {
                write!(f, "{what}: response was not valid UTF-8.")
            }
            JarProviderError::Staging(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for JarProviderError {}

impl From<CatalogError> for JarProviderError {
    fn from(e: CatalogError) -> Self {
        JarProviderError::Catalog(e)
    }
}

/// The boundary every family's catalog fetch and jar download goes
/// through. `get` returns the response body on a 200 status, bounded at
/// `max_bytes`; anything else (non-2xx, connection failure, timeout, a
/// body that exceeds the cap) is a typed [`JarProviderError`], never a
/// panic or an unbounded read.
pub trait Transport: Send + Sync {
    fn get(&self, url: &str, what: &str, max_bytes: u64) -> Result<Vec<u8>, JarProviderError>;
}

fn bytes_to_utf8(bytes: Vec<u8>, what: &str) -> Result<String, JarProviderError> {
    String::from_utf8(bytes).map_err(|_| JarProviderError::InvalidUtf8 {
        what: what.to_string(),
    })
}

/// The real implementation, backed by `ureq`.
pub struct HttpTransport {
    agent: ureq::Agent,
}

impl Default for HttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTransport {
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build();
        Self {
            agent: ureq::Agent::new_with_config(config),
        }
    }
}

impl Transport for HttpTransport {
    fn get(&self, url: &str, what: &str, max_bytes: u64) -> Result<Vec<u8>, JarProviderError> {
        let response = self
            .agent
            .get(url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "*/*")
            .call();
        let mut response = match response {
            Ok(r) => r,
            Err(ureq::Error::StatusCode(code)) => {
                return Err(JarProviderError::Network(format!(
                    "{what} returned status {code}."
                )));
            }
            Err(ureq::Error::Timeout(_)) => {
                return Err(JarProviderError::Timeout(what.to_string()));
            }
            Err(e) => return Err(JarProviderError::Network(format!("{what}: {e}"))),
        };
        response
            .body_mut()
            .with_config()
            .limit(max_bytes)
            .read_to_vec()
            .map_err(|e| match e {
                ureq::Error::BodyExceedsLimit(limit) => JarProviderError::ResponseTooLarge {
                    what: what.to_string(),
                    max_bytes: limit,
                },
                ureq::Error::Timeout(_) => JarProviderError::Timeout(what.to_string()),
                other => JarProviderError::Network(format!("{what}: {other}")),
            })
    }
}

// ---------------------------------------------------------------------
// Vanilla
// ---------------------------------------------------------------------

fn vanilla_manifest_url() -> String {
    format!(
        "{}/mc/game/version_manifest_v2.json",
        provider_base(
            "MSC2_PROVIDER_VANILLA_BASE",
            "https://launchermeta.mojang.com"
        )
    )
}

pub fn vanilla_list_versions(
    transport: &dyn Transport,
) -> Result<Vec<server_versions::ServerVersionEntry>, JarProviderError> {
    let bytes = transport.get(
        &vanilla_manifest_url(),
        "Mojang manifest",
        CATALOG_MAX_BYTES,
    )?;
    let manifest = bytes_to_utf8(bytes, "Mojang manifest")?;
    Ok(server_versions::vanilla_list_versions(&manifest)?)
}

pub fn vanilla_download_latest(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    dest: &Path,
) -> Result<CachedFile, JarProviderError> {
    vanilla_download(transport, fs, None, dest)
}

/// `VanillaDownloader.downloadVersion(_:to:)` (`ServerJarProviders.swift:
/// 391-414`): identical to [`vanilla_download_latest`] except the release
/// id is pinned rather than resolved from `latest.release` — P7.19's
/// version-change is this variant's first caller (P7.13/P7.17 only ever
/// needed "latest"). `vanilla_resolve_metadata_url` already accepted an
/// optional pin (`Some(pinned_release_id)` errors if the manifest has no
/// matching entry, the same `No manifest entry for ...` shape source's own
/// `guard let entry = versionList.first(where: ...)` produces), so this is
/// the same composition, not a new algorithm.
pub fn vanilla_download_version(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    release_id: &str,
    dest: &Path,
) -> Result<CachedFile, JarProviderError> {
    vanilla_download(transport, fs, Some(release_id), dest)
}

fn vanilla_download(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    pinned_release_id: Option<&str>,
    dest: &Path,
) -> Result<CachedFile, JarProviderError> {
    let manifest_bytes = transport.get(
        &vanilla_manifest_url(),
        "Mojang manifest",
        CATALOG_MAX_BYTES,
    )?;
    let manifest = bytes_to_utf8(manifest_bytes, "Mojang manifest")?;
    let (release_id, meta_url) =
        server_versions::vanilla_resolve_metadata_url(&manifest, pinned_release_id)?;

    let meta_bytes = transport.get(&meta_url, "Mojang version metadata", CATALOG_MAX_BYTES)?;
    let meta_text = bytes_to_utf8(meta_bytes, "Mojang version metadata")?;
    let jar_url = server_versions::vanilla_server_download_url(&meta_text, &release_id)?;

    let jar_bytes = transport.get(&jar_url, "Vanilla download", JAR_MAX_BYTES)?;
    download_staging::stage_download(fs, dest, &jar_bytes, &jar_url, &release_id, None)
        .map_err(JarProviderError::Staging)
}

// ---------------------------------------------------------------------
// Purpur
// ---------------------------------------------------------------------

fn purpur_base() -> String {
    provider_base("MSC2_PROVIDER_PURPUR_BASE", "https://api.purpurmc.org")
}

fn purpur_project_url() -> String {
    format!("{}/v2/purpur", purpur_base())
}

pub fn purpur_list_versions(
    transport: &dyn Transport,
) -> Result<Vec<server_versions::ServerVersionEntry>, JarProviderError> {
    let bytes = transport.get(&purpur_project_url(), "Purpur project", CATALOG_MAX_BYTES)?;
    let body = bytes_to_utf8(bytes, "Purpur project")?;
    Ok(server_versions::purpur_list_versions(&body)?)
}

pub fn purpur_download_version(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    version: &str,
    dest: &Path,
) -> Result<CachedFile, JarProviderError> {
    let dl_url = format!("{}/v2/purpur/{version}/latest/download", purpur_base());
    let jar_bytes = transport.get(&dl_url, "Purpur download", JAR_MAX_BYTES)?;
    download_staging::stage_download(fs, dest, &jar_bytes, &dl_url, version, None)
        .map_err(JarProviderError::Staging)
}

/// `PurpurDownloader.downloadLatest`'s own raw fetch (`ServerJarProviders
/// .swift:268-274`): the **unfiltered** `versions` string array, unlike
/// [`purpur_list_versions`]'s already-`"1."`-filtered, sorted,
/// `ServerVersionEntry` output. [`msc_domain::server_versions::
/// purpur_pick_target_version`]'s Paper-alignment containment check runs
/// against this raw list in MSC 1 (P7.17 needs this half of "download
/// latest," which P7.13 didn't build — only Vanilla got a complete
/// latest-composite there).
pub fn purpur_raw_version_list(transport: &dyn Transport) -> Result<Vec<String>, JarProviderError> {
    let bytes = transport.get(&purpur_project_url(), "Purpur project", CATALOG_MAX_BYTES)?;
    let body = bytes_to_utf8(bytes, "Purpur project")?;
    let root: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        JarProviderError::Network(format!("Purpur project response was not valid JSON: {e}"))
    })?;
    let versions = root
        .get("versions")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| JarProviderError::Network("Purpur versions list missing.".to_string()))?;
    Ok(versions
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect())
}

/// `PurpurDownloader.downloadLatest`'s build-label lookup
/// (`ServerJarProviders.swift:291-302`): `GET .../purpur/{version}`, then
/// `builds.latest` — a present-but-malformed/missing field falls back to
/// the literal `"latest"` (source's own `else` branch), but a transport
/// failure still propagates, matching source's `try ensureOK` before the
/// soft field read.
pub fn purpur_latest_build_label(
    transport: &dyn Transport,
    version: &str,
) -> Result<String, JarProviderError> {
    let url = format!("{}/v2/purpur/{version}", purpur_base());
    let bytes = transport.get(&url, "Purpur version", CATALOG_MAX_BYTES)?;
    let body = bytes_to_utf8(bytes, "Purpur version")?;
    let value: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
    Ok(value
        .get("builds")
        .and_then(|b| b.get("latest"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| "latest".to_string()))
}

// ---------------------------------------------------------------------
// Paper
// ---------------------------------------------------------------------

fn paper_project_url() -> String {
    format!(
        "{}/v3/projects/paper",
        provider_base("MSC2_PROVIDER_PAPER_BASE", "https://fill.papermc.io")
    )
}

pub fn paper_flatten_and_sort_versions(
    transport: &dyn Transport,
) -> Result<Vec<String>, JarProviderError> {
    let bytes = transport.get(&paper_project_url(), "Paper project v3", CATALOG_MAX_BYTES)?;
    let body = bytes_to_utf8(bytes, "Paper project v3")?;
    Ok(server_versions::paper_flatten_and_sort(&body)?)
}

/// `fetchBestBuild(forVersion:includeExperimental:)`'s real HTTP hop:
/// fetch that one version's builds, then delegate the pure selection to
/// `msc_domain::server_versions::paper_select_build`.
pub fn paper_select_build(
    transport: &dyn Transport,
    version: &str,
    include_experimental: bool,
) -> Option<server_versions::PaperBuildSelection> {
    let url = format!("{}/versions/{version}/builds", paper_project_url());
    let bytes = transport
        .get(&url, "Paper builds", CATALOG_MAX_BYTES)
        .ok()?;
    let body = String::from_utf8(bytes).ok()?;
    server_versions::paper_select_build(&body, include_experimental)
}

/// `fetchAvailableVersions(includeExperimental:limit:)`: the flatten+sort
/// fetch, then [`server_versions::paper_walk_candidates`]'s 20-candidate
/// walk, fetching each candidate's builds through [`paper_select_build`].
pub fn paper_fetch_available_versions(
    transport: &dyn Transport,
    include_experimental: bool,
    limit: usize,
) -> Result<Vec<server_versions::PaperBuildSelection>, JarProviderError> {
    let candidates = paper_flatten_and_sort_versions(transport)?;
    let outcome = server_versions::paper_walk_candidates(&candidates, limit, |v| {
        paper_select_build(transport, v, include_experimental)
    });
    Ok(outcome.results)
}

/// `PaperDownloader.downloadLatestPaper`/`fetchLatestMetadata`'s shared
/// first step (`PaperDownloader.swift:114-120,151-157`):
/// `fetchAvailableVersions(includeExperimental: false, limit: 1).first`,
/// kept paired with the version string [`paper_fetch_available_versions`]
/// itself discards (see that function's own doc) since both the create
/// flow's Paper download and Purpur's Paper-alignment check need it.
/// `Ok(None)` mirrors source's own "no stable Paper builds found" case,
/// not a transport failure.
pub fn paper_resolve_latest_stable(
    transport: &dyn Transport,
) -> Result<Option<(String, server_versions::PaperBuildSelection)>, JarProviderError> {
    let candidates = paper_flatten_and_sort_versions(transport)?;
    let outcome = server_versions::paper_walk_candidates(&candidates, 1, |v| {
        paper_select_build(transport, v, false).map(|sel| (v.to_string(), sel))
    });
    Ok(outcome.results.into_iter().next())
}

/// Downloads the given build's jar for `version`. `paper_select_build`
/// only records `build_id`/`channel`/`is_stable`, not the download URL
/// (MSC 1's own `PaperVersionOption` carries a `downloadURL` field that
/// selection doesn't expose in this port) — the URL is re-read from the
/// same builds response here rather than plumbed through
/// `PaperBuildSelection`, keeping that struct a pure "which build" answer.
pub fn paper_download_build(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    version: &str,
    build_id: i64,
    dest: &Path,
) -> Result<CachedFile, JarProviderError> {
    let builds_url = format!("{}/versions/{version}/builds", paper_project_url());
    let bytes = transport.get(&builds_url, "Paper builds", CATALOG_MAX_BYTES)?;
    let body = bytes_to_utf8(bytes, "Paper builds")?;
    let jar_url = paper_build_download_url(&body, build_id).ok_or_else(|| {
        JarProviderError::Network(format!(
            "No download found for Paper {version} build {build_id}."
        ))
    })?;

    let jar_bytes = transport.get(&jar_url, "Paper download", JAR_MAX_BYTES)?;
    download_staging::stage_download(
        fs,
        dest,
        &jar_bytes,
        &jar_url,
        &format!("{version}-{build_id}"),
        None,
    )
    .map_err(JarProviderError::Staging)
}

fn paper_build_download_url(raw_builds_body: &str, build_id: i64) -> Option<String> {
    let builds: serde_json::Value = serde_json::from_str(raw_builds_body).ok()?;
    let builds = builds.as_array()?;
    builds
        .iter()
        .find(|entry| entry.get("id").and_then(serde_json::Value::as_i64) == Some(build_id))
        .and_then(|entry| entry.get("downloads"))
        .and_then(|d| d.get("server:default"))
        .and_then(|sd| sd.get("url"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// `PaperDownloader.downloadVersion(_:to:)` (`ServerJarProviders.swift:
/// 218-254`): picks the **highest build id of any channel** for a pinned
/// version, no STABLE/BETA/ALPHA preference at all — genuinely different
/// from [`paper_download_build`] (needs a build id already known) and
/// from [`paper_select_build`]/[`paper_version_entry_from_builds`] (both
/// channel-aware). P7.19's version-change is this function's first
/// caller; nothing in the create flow ever pins a specific Paper version
/// (see `provisioning.rs`'s own "download latest only" scope note).
/// Returns the staged jar alongside the resolved build id directly —
/// P7.19's caller (`msc-application/src/server_versions.rs`) needs it
/// separately for `ConfigServer.serverBuild`/the Paper sidecar, and
/// re-deriving it from `CachedFile.version`'s combined `"{version}-
/// {build_id}"` label (the same convention [`paper_download_build`]
/// already uses) would mean assuming build ids never contain a `-`,
/// true today but not a contract worth leaning on.
pub fn paper_download_pinned_version(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    version: &str,
    dest: &Path,
) -> Result<(CachedFile, i64), JarProviderError> {
    let builds_url = format!("{}/versions/{version}/builds", paper_project_url());
    let bytes = transport.get(&builds_url, "Paper builds", CATALOG_MAX_BYTES)?;
    let body = bytes_to_utf8(bytes, "Paper builds")?;
    let (build_id, jar_url) = paper_highest_build_any_channel(&body).ok_or_else(|| {
        JarProviderError::Network(format!("No builds found for Paper {version}."))
    })?;

    let jar_bytes = transport.get(&jar_url, "Paper download", JAR_MAX_BYTES)?;
    let cached = download_staging::stage_download(
        fs,
        dest,
        &jar_bytes,
        &jar_url,
        &format!("{version}-{build_id}"),
        None,
    )
    .map_err(JarProviderError::Staging)?;
    Ok((cached, build_id))
}

fn paper_highest_build_any_channel(raw_builds_body: &str) -> Option<(i64, String)> {
    let builds: serde_json::Value = serde_json::from_str(raw_builds_body).ok()?;
    let builds = builds.as_array()?;
    builds
        .iter()
        .filter_map(|entry| {
            let id = entry.get("id").and_then(serde_json::Value::as_i64)?;
            let url = entry
                .get("downloads")
                .and_then(|d| d.get("server:default"))
                .and_then(|sd| sd.get("url"))
                .and_then(serde_json::Value::as_str)?;
            Some((id, url.to_string()))
        })
        .max_by_key(|(id, _)| *id)
}

/// `PaperDownloader.listVersions()` (`ServerJarProviders.swift:141-174`):
/// the version-picker/version-change listing — every Paper version,
/// newest first, no 20-candidate cap (that cap belongs to the "download
/// latest" walk `paper_select_build`/[`msc_domain::server_versions::
/// paper_walk_candidates`] compose, a genuinely different, capped walk).
/// Source fetches every version's builds concurrently (`withTaskGroup`);
/// this crate's `Transport` is a blocking, synchronous boundary (see this
/// module's own doc), so the walk here is sequential — a real behavior
/// difference in *latency*, not in the *result*, since source's task
/// group already discards ordering and this function re-sorts by
/// `compare_mc_versions` regardless (`paper_flatten_and_sort` already
/// sorted its input the same way). An entry with no recognized build at
/// all (`build_label: None`) is dropped, matching source's own
/// `if entry.buildLabel != nil` filter.
pub fn paper_list_versions_for_picker(
    transport: &dyn Transport,
) -> Result<Vec<server_versions::ServerVersionEntry>, JarProviderError> {
    let versions = paper_flatten_and_sort_versions(transport)?;
    let mut entries = Vec::with_capacity(versions.len());
    for version in &versions {
        let builds_url = format!("{}/versions/{version}/builds", paper_project_url());
        let entry = match transport.get(&builds_url, "Paper builds", CATALOG_MAX_BYTES) {
            Ok(bytes) => match bytes_to_utf8(bytes, "Paper builds") {
                Ok(body) => server_versions::paper_version_entry_from_builds(version, &body),
                Err(_) => server_versions::paper_version_entry_from_builds(version, ""),
            },
            // A per-version fetch failure degrades to "no recognized build,"
            // matching source's own `guard let (data, _) = try? await ...`
            // — one bad version doesn't fail the whole listing.
            Err(_) => server_versions::paper_version_entry_from_builds(version, ""),
        };
        if entry.build_label.is_some() {
            entries.push(entry);
        }
    }
    Ok(entries)
}

// ---------------------------------------------------------------------
// Fabric
// ---------------------------------------------------------------------

fn fabric_base() -> String {
    provider_base("MSC2_PROVIDER_FABRIC_BASE", "https://meta.fabricmc.net")
}

fn fabric_game_url() -> String {
    format!("{}/v2/versions/game", fabric_base())
}

fn fabric_installer_url() -> String {
    format!("{}/v2/versions/installer", fabric_base())
}

pub fn fabric_list_versions(
    transport: &dyn Transport,
) -> Result<Vec<server_versions::ServerVersionEntry>, JarProviderError> {
    let bytes = transport.get(
        &fabric_game_url(),
        "Fabric game versions",
        CATALOG_MAX_BYTES,
    )?;
    let body = bytes_to_utf8(bytes, "Fabric game versions")?;
    Ok(server_versions::fabric_list_versions(&body)?)
}

/// `FabricDownloader.downloadLatest`'s first hop
/// (`ServerJarProviders.swift:427-428`): `firstStableVersion(from:
/// ".../versions/game", ...)` — genuinely different from
/// [`fabric_list_versions`]'s picker output (see that function's own doc):
/// this reads the **raw**, unfiltered game list and falls back to index 0
/// when nothing is marked stable, rather than returning an empty result.
/// P7.13 didn't build this half of "download latest" — only Vanilla got a
/// complete latest-composite there.
pub fn fabric_latest_stable_game_version(
    transport: &dyn Transport,
) -> Result<String, JarProviderError> {
    let bytes = transport.get(
        &fabric_game_url(),
        "Fabric game versions",
        CATALOG_MAX_BYTES,
    )?;
    let body = bytes_to_utf8(bytes, "Fabric game versions")?;
    Ok(server_versions::fabric_first_stable_version(
        &body,
        "Fabric game",
    )?)
}

/// The no-pinned-loader half of [`fabric_download_version`]
/// (`ServerJarProviders.swift:481-492`), split out so a caller resolving
/// "download latest" can learn the loader version it resolved to (needed
/// for `ConfigServer.loaderVersion`/`recordLoaderVersion` — Fabric is a
/// modded-category flavor) without a second, redundant loader-list fetch:
/// pass the result back into [`fabric_download_version`] as its own
/// `pinned_loader_version`.
pub fn fabric_resolve_loader(
    transport: &dyn Transport,
    mc_version: &str,
) -> Result<String, JarProviderError> {
    let loader_url = format!("{}/v2/versions/loader/{mc_version}", fabric_base());
    let bytes = transport.get(&loader_url, "Fabric loader", CATALOG_MAX_BYTES)?;
    let body = bytes_to_utf8(bytes, "Fabric loader")?;
    Ok(server_versions::fabric_select_loader(&body)?)
}

pub fn fabric_download_version(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    mc_version: &str,
    pinned_loader_version: Option<&str>,
    dest: &Path,
) -> Result<CachedFile, JarProviderError> {
    let loader = match pinned_loader_version
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        Some(pinned) => pinned.to_string(),
        None => {
            let loader_url = format!("{}/v2/versions/loader/{mc_version}", fabric_base());
            let bytes = transport.get(&loader_url, "Fabric loader", CATALOG_MAX_BYTES)?;
            let body = bytes_to_utf8(bytes, "Fabric loader")?;
            server_versions::fabric_select_loader(&body)?
        }
    };

    let installer_bytes = transport.get(
        &fabric_installer_url(),
        "Fabric installer",
        CATALOG_MAX_BYTES,
    )?;
    let installer_body = bytes_to_utf8(installer_bytes, "Fabric installer")?;
    let installer =
        server_versions::fabric_first_stable_version(&installer_body, "Fabric installer")?;

    let dl_url = format!(
        "{}/v2/versions/loader/{mc_version}/{loader}/{installer}/server/jar",
        fabric_base()
    );
    let jar_bytes = transport.get(&dl_url, "Fabric server jar", JAR_MAX_BYTES)?;
    download_staging::stage_download(fs, dest, &jar_bytes, &dl_url, mc_version, None)
        .map_err(JarProviderError::Staging)
}

// ---------------------------------------------------------------------
// NeoForge
// ---------------------------------------------------------------------

fn neoforge_maven_base() -> String {
    provider_base(
        "MSC2_PROVIDER_NEOFORGE_MAVEN_BASE",
        "https://maven.neoforged.net",
    )
}

fn neoforge_metadata_url() -> String {
    format!(
        "{}/releases/net/neoforged/neoforge/maven-metadata.xml",
        neoforge_maven_base()
    )
}

pub fn neoforge_list_version_pairs(
    transport: &dyn Transport,
) -> Result<Vec<server_versions::ServerVersionEntry>, JarProviderError> {
    let bytes = transport.get(
        &neoforge_metadata_url(),
        "NeoForge metadata",
        CATALOG_MAX_BYTES,
    )?;
    let xml = bytes_to_utf8(bytes, "NeoForge metadata")?;
    Ok(server_versions::neoforge_build_entries(&xml))
}

/// `NeoForgeInstaller.latestStableVersion`'s network hop
/// (`NeoForgeInstaller.swift:200-220`), delegating selection to the
/// already-ported `msc_domain::server_versions::neoforge_latest_stable`
/// (P7.10). P7.13 built the raw metadata fetch/parse
/// ([`neoforge_list_version_pairs`]) but not this single-"latest stable
/// version" composite — P7.18 (install-step creation) is this
/// function's first caller.
pub fn neoforge_latest_stable(transport: &dyn Transport) -> Result<String, JarProviderError> {
    let bytes = transport.get(
        &neoforge_metadata_url(),
        "NeoForge metadata",
        CATALOG_MAX_BYTES,
    )?;
    let xml = bytes_to_utf8(bytes, "NeoForge metadata")?;
    Ok(server_versions::neoforge_latest_stable(&xml)?)
}

/// Downloads (not runs — that's P7.14's `loader_installer`) the NeoForge
/// installer jar for `version` into `dest`.
pub fn neoforge_download_installer(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    version: &str,
    dest: &Path,
) -> Result<CachedFile, JarProviderError> {
    let url = format!(
        "{}/releases/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar",
        neoforge_maven_base()
    );
    let bytes = transport.get(&url, "NeoForge installer download", JAR_MAX_BYTES)?;
    download_staging::stage_download(fs, dest, &bytes, &url, version, None)
        .map_err(JarProviderError::Staging)
}

// ---------------------------------------------------------------------
// Forge
// ---------------------------------------------------------------------

fn forge_metadata_url() -> String {
    format!(
        "{}/net/minecraftforge/forge/maven-metadata.xml",
        provider_base(
            "MSC2_PROVIDER_FORGE_MAVEN_BASE",
            "https://maven.minecraftforge.net"
        )
    )
}

fn forge_promotions_url() -> String {
    format!(
        "{}/net/minecraftforge/forge/promotions_slim.json",
        provider_base(
            "MSC2_PROVIDER_FORGE_FILES_BASE",
            "https://files.minecraftforge.net"
        )
    )
}

pub fn forge_list_version_pairs(
    transport: &dyn Transport,
) -> Result<Vec<server_versions::ServerVersionEntry>, JarProviderError> {
    let bytes = transport.get(&forge_metadata_url(), "Forge metadata", CATALOG_MAX_BYTES)?;
    let xml = bytes_to_utf8(bytes, "Forge metadata")?;
    Ok(server_versions::forge_parse_maven_metadata(&xml))
}

pub fn forge_latest_recommended(
    transport: &dyn Transport,
) -> Result<(String, String), JarProviderError> {
    let bytes = transport.get(
        &forge_promotions_url(),
        "Forge promotions",
        CATALOG_MAX_BYTES,
    )?;
    let body = bytes_to_utf8(bytes, "Forge promotions")?;
    Ok(server_versions::forge_latest_recommended(&body)?)
}

/// Downloads (not runs) the Forge installer jar for the `{mc}-{forge}`
/// pair into `dest`.
pub fn forge_download_installer(
    transport: &dyn Transport,
    fs: &dyn FileSystem,
    mc_version: &str,
    forge_version: &str,
    dest: &Path,
) -> Result<CachedFile, JarProviderError> {
    let url = format!(
        "{}/net/minecraftforge/forge/{mc_version}-{forge_version}/forge-{mc_version}-{forge_version}-installer.jar",
        provider_base(
            "MSC2_PROVIDER_FORGE_MAVEN_BASE",
            "https://maven.minecraftforge.net"
        )
    );
    let version_label = format!("{mc_version}-{forge_version}");
    let bytes = transport.get(&url, "Forge installer download", JAR_MAX_BYTES)?;
    download_staging::stage_download(fs, dest, &bytes, &url, &version_label, None)
        .map_err(JarProviderError::Staging)
}
