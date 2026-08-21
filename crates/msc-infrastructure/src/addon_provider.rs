//! Add-on metadata provider transport: Modrinth, Hangar, CurseForge, and
//! GitHub Releases, behind one fakeable [`AddonTransport`] boundary with
//! configurable base URLs (the same `provider_base` env-var-override
//! convention `jar_provider.rs` established in P7.13).
//!
//! This step fetches metadata only -- no payload bytes are downloaded or
//! staged here; that's P8.14's job, layered on top of these functions the
//! same way `jar_provider.rs`'s per-family download functions layer on
//! [`crate::download_staging::stage_download`]. Every function composes
//! [`AddonTransport::get`]/[`AddonTransport::post_json`] with
//! `msc_domain::addon_provider`'s pure parsing (P8.10) -- this module's own
//! job is bounding the request (size cap, timeout, batch-chunk cap) and
//! handing the raw status/body to the domain layer, which decides what a
//! given status means (e.g. Modrinth's 404-is-not-an-error on
//! `version_file`). Unlike [`crate::jar_provider::Transport`], which treats
//! any non-2xx as an error itself, [`AddonTransport`] always returns the
//! raw status -- the domain layer's status-code logic (P8.10) needs to see
//! it, not have it pre-judged by the transport.
//!
//! The CurseForge API key is read through [`crate::secret_store::SecretStore`]
//! at [`CURSEFORGE_API_KEY_SECRET`] (`docs/msc2/substrate/secret-storage.md`
//! §9's `curseforge.api-key`), matching `CurseForgeAPI.swift`'s own
//! `KeychainManager.shared.readCurseForgeAPIKey()` call site.

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use msc_domain::addon_provider::{self as domain, AddonProviderError};

use crate::secret_store::SecretStore;

const USER_AGENT: &str = "MinecraftServerController/2.0 (msc2 agent)";

/// Metadata responses (search pages, batch identify/update results,
/// project/version detail) are small JSON -- bounded generously but well
/// below what a runaway/malicious response could exhaust memory with.
pub const RESPONSE_MAX_BYTES: u64 = 10 * 1024 * 1024; // 10 MB

/// Connect + the whole exchange must complete within this long.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// The most ids sent in a single Modrinth/CurseForge batch request; a
/// longer input list is split into this many ids per request rather than
/// growing the request (or the caller's assumed request count) unbounded.
pub const MAX_BATCH_SIZE: usize = 100;

/// `docs/msc2/substrate/secret-storage.md` §9.
pub const CURSEFORGE_API_KEY_SECRET: &str = "curseforge.api-key";

fn provider_base(env_var: &str, default: &str) -> String {
    std::env::var(env_var)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn urlencode(s: &str) -> String {
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

fn bytes_to_utf8(bytes: Vec<u8>, what: &str) -> Result<String, AddonProviderError> {
    String::from_utf8(bytes)
        .map_err(|_| AddonProviderError::Network(format!("{what}: response was not valid UTF-8.")))
}

fn malformed(what: &str, e: serde_json::Error) -> AddonProviderError {
    AddonProviderError::Network(format!("Malformed {what} response: {e}"))
}

fn ensure_ok_generic(provider: &str, status: u16) -> Result<(), AddonProviderError> {
    if !(200..300).contains(&status) {
        return Err(AddonProviderError::Network(format!(
            "{provider} returned status {status}."
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------
// Transport boundary
// ---------------------------------------------------------------------

#[derive(Debug)]
pub struct RawResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

#[derive(Debug)]
pub enum TransportError {
    Network(String),
    Timeout(String),
    ResponseTooLarge { what: String, max_bytes: u64 },
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::Network(m) => write!(f, "{m}"),
            TransportError::Timeout(what) => write!(f, "{what} timed out."),
            TransportError::ResponseTooLarge { what, max_bytes } => {
                write!(f, "{what} exceeded the {max_bytes}-byte size cap.")
            }
        }
    }
}

impl std::error::Error for TransportError {}

fn map_transport_err(e: TransportError) -> AddonProviderError {
    AddonProviderError::Network(e.to_string())
}

/// The boundary every provider fetch goes through: a bare GET, or a POST
/// with a JSON body. Both return the raw status code -- never pre-judged
/// as an error by the transport itself -- bounded at `max_bytes` and
/// [`REQUEST_TIMEOUT`].
pub trait AddonTransport: Send + Sync {
    fn get(
        &self,
        url: &str,
        what: &str,
        headers: &[(&str, &str)],
        max_bytes: u64,
    ) -> Result<RawResponse, TransportError>;

    fn post_json(
        &self,
        url: &str,
        what: &str,
        body: &serde_json::Value,
        headers: &[(&str, &str)],
        max_bytes: u64,
    ) -> Result<RawResponse, TransportError>;
}

/// The real implementation, backed by `ureq`. `http_status_as_error(false)`
/// is the load-bearing config choice here: without it, `ureq` itself would
/// turn a Modrinth 404 or a CurseForge 401 into a transport-level `Err`
/// before the domain layer ever gets to apply its own per-status meaning.
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
            .http_status_as_error(false)
            .build();
        Self {
            agent: ureq::Agent::new_with_config(config),
        }
    }

    fn read_body(
        mut response: ureq::http::Response<ureq::Body>,
        what: &str,
        max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .with_config()
            .limit(max_bytes)
            .read_to_vec()
            .map_err(|e| match e {
                ureq::Error::BodyExceedsLimit(limit) => TransportError::ResponseTooLarge {
                    what: what.to_string(),
                    max_bytes: limit,
                },
                ureq::Error::Timeout(_) => TransportError::Timeout(what.to_string()),
                other => TransportError::Network(format!("{what}: {other}")),
            })?;
        Ok(RawResponse { status, body })
    }
}

impl AddonTransport for HttpTransport {
    fn get(
        &self,
        url: &str,
        what: &str,
        headers: &[(&str, &str)],
        max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        let mut req = self
            .agent
            .get(url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "*/*");
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let response = req.call().map_err(|e| match e {
            ureq::Error::Timeout(_) => TransportError::Timeout(what.to_string()),
            other => TransportError::Network(format!("{what}: {other}")),
        })?;
        Self::read_body(response, what, max_bytes)
    }

    fn post_json(
        &self,
        url: &str,
        what: &str,
        body: &serde_json::Value,
        headers: &[(&str, &str)],
        max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        let payload = serde_json::to_vec(body)
            .map_err(|e| TransportError::Network(format!("{what}: could not encode body: {e}")))?;
        let mut req = self
            .agent
            .post(url)
            .header("User-Agent", USER_AGENT)
            .header("Content-Type", "application/json")
            .header("Accept", "*/*");
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let response = req.send(payload.as_slice()).map_err(|e| match e {
            ureq::Error::Timeout(_) => TransportError::Timeout(what.to_string()),
            other => TransportError::Network(format!("{what}: {other}")),
        })?;
        Self::read_body(response, what, max_bytes)
    }
}

// ---------------------------------------------------------------------
// Modrinth
// ---------------------------------------------------------------------

fn modrinth_base() -> String {
    provider_base("MSC2_PROVIDER_MODRINTH_BASE", "https://api.modrinth.com")
}

#[allow(clippy::too_many_arguments)]
pub fn modrinth_search(
    transport: &dyn AddonTransport,
    query: &str,
    project_type: &str,
    loaders: &[String],
    game_version: Option<&str>,
    limit: u32,
    offset: u32,
) -> Result<domain::ModrinthSearchResult, AddonProviderError> {
    let facets = domain::modrinth_facets(project_type, loaders, game_version);
    let index = domain::modrinth_search_index(query);
    let url = format!(
        "{}/v2/search?query={}&facets={}&index={index}&limit={limit}&offset={offset}",
        modrinth_base(),
        urlencode(query),
        urlencode(&facets),
    );
    let resp = transport
        .get(&url, "Modrinth search", &[], RESPONSE_MAX_BYTES)
        .map_err(map_transport_err)?;
    domain::ensure_modrinth_ok(resp.status)?;
    let text = bytes_to_utf8(resp.body, "Modrinth search")?;
    domain::modrinth_decode_search(&text)
}

pub fn modrinth_version_from_hash(
    transport: &dyn AddonTransport,
    sha512: &str,
) -> Result<Option<domain::ModrinthVersionInfo>, AddonProviderError> {
    let url = format!(
        "{}/v2/version_file/{sha512}?algorithm=sha512",
        modrinth_base()
    );
    let resp = transport
        .get(&url, "Modrinth version_file", &[], RESPONSE_MAX_BYTES)
        .map_err(map_transport_err)?;
    let body = if resp.status == 404 {
        String::new()
    } else {
        bytes_to_utf8(resp.body, "Modrinth version_file")?
    };
    domain::modrinth_version_from_hash(resp.status, &body)
}

/// `ModrinthAPI.versionsFromHashes(_:)`: batches into [`MAX_BATCH_SIZE`]-id
/// chunks (an empty chunk, per [`domain::modrinth_versions_from_hashes_plan`],
/// never reaches the network).
pub fn modrinth_versions_from_hashes(
    transport: &dyn AddonTransport,
    sha512s: &[String],
) -> Result<HashMap<String, domain::ModrinthVersionInfo>, AddonProviderError> {
    let mut result = HashMap::new();
    for chunk in sha512s.chunks(MAX_BATCH_SIZE) {
        let Some(ids) = domain::modrinth_versions_from_hashes_plan(chunk) else {
            continue;
        };
        let body = serde_json::json!({ "hashes": ids, "algorithm": "sha512" });
        let url = format!("{}/v2/version_files", modrinth_base());
        let resp = transport
            .post_json(
                &url,
                "Modrinth version_files",
                &body,
                &[],
                RESPONSE_MAX_BYTES,
            )
            .map_err(map_transport_err)?;
        domain::ensure_modrinth_ok(resp.status)?;
        let text = bytes_to_utf8(resp.body, "Modrinth version_files")?;
        let batch: HashMap<String, domain::ModrinthVersionInfo> =
            serde_json::from_str(&text).map_err(|e| malformed("Modrinth version_files", e))?;
        result.extend(batch);
    }
    Ok(result)
}

/// `ModrinthAPI.latestVersionsForHashes(_:loaders:gameVersions:)`.
pub fn modrinth_latest_versions_for_hashes(
    transport: &dyn AddonTransport,
    sha512s: &[String],
    loaders: &[String],
    game_versions: &[String],
) -> Result<HashMap<String, domain::ModrinthVersionInfo>, AddonProviderError> {
    let mut result = HashMap::new();
    for chunk in sha512s.chunks(MAX_BATCH_SIZE) {
        if chunk.is_empty() {
            continue;
        }
        let body = domain::modrinth_latest_versions_body(chunk, loaders, game_versions);
        let url = format!("{}/v2/version_files/update", modrinth_base());
        let resp = transport
            .post_json(
                &url,
                "Modrinth version_files/update",
                &body,
                &[],
                RESPONSE_MAX_BYTES,
            )
            .map_err(map_transport_err)?;
        domain::ensure_modrinth_ok(resp.status)?;
        let text = bytes_to_utf8(resp.body, "Modrinth version_files/update")?;
        let batch: HashMap<String, domain::ModrinthVersionInfo> = serde_json::from_str(&text)
            .map_err(|e| malformed("Modrinth version_files/update", e))?;
        result.extend(batch);
    }
    Ok(result)
}

/// `ModrinthAPI.projects(ids:)`: batched, each chunk's ids JSON-encoded
/// into the `ids` query param.
pub fn modrinth_projects(
    transport: &dyn AddonTransport,
    ids: &[String],
) -> Result<Vec<serde_json::Value>, AddonProviderError> {
    let mut result = Vec::new();
    for chunk in ids.chunks(MAX_BATCH_SIZE) {
        let Some(chunk_ids) = domain::modrinth_projects_plan(chunk) else {
            continue;
        };
        let ids_json = serde_json::to_string(&chunk_ids).expect("Vec<String> always serializes");
        let url = format!(
            "{}/v2/projects?ids={}",
            modrinth_base(),
            urlencode(&ids_json)
        );
        let resp = transport
            .get(&url, "Modrinth projects", &[], RESPONSE_MAX_BYTES)
            .map_err(map_transport_err)?;
        domain::ensure_modrinth_ok(resp.status)?;
        let text = bytes_to_utf8(resp.body, "Modrinth projects")?;
        let batch: Vec<serde_json::Value> =
            serde_json::from_str(&text).map_err(|e| malformed("Modrinth projects", e))?;
        result.extend(batch);
    }
    Ok(result)
}

/// P8.15 amendment: `ModrinthAPI.project(idOrSlug:)`.
pub fn modrinth_project(
    transport: &dyn AddonTransport,
    id_or_slug: &str,
) -> Result<domain::ModrinthProjectSummary, AddonProviderError> {
    let url = format!("{}/v2/project/{}", modrinth_base(), urlencode(id_or_slug));
    let resp = transport
        .get(&url, "Modrinth project", &[], RESPONSE_MAX_BYTES)
        .map_err(map_transport_err)?;
    domain::ensure_modrinth_ok(resp.status)?;
    let text = bytes_to_utf8(resp.body, "Modrinth project")?;
    domain::modrinth_decode_project(&text)
}

/// P8.15 amendment: `ModrinthAPI.projectVersions(idOrSlug:loaders:gameVersion:)`.
pub fn modrinth_project_versions(
    transport: &dyn AddonTransport,
    id_or_slug: &str,
    loaders: &[String],
    game_version: Option<&str>,
) -> Result<Vec<domain::ModrinthVersionInfo>, AddonProviderError> {
    let params = domain::modrinth_project_versions_query(loaders, game_version);
    let mut url = format!(
        "{}/v2/project/{}/version",
        modrinth_base(),
        urlencode(id_or_slug)
    );
    if !params.is_empty() {
        let query: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{k}={}", urlencode(v)))
            .collect();
        url.push('?');
        url.push_str(&query.join("&"));
    }
    let resp = transport
        .get(&url, "Modrinth project versions", &[], RESPONSE_MAX_BYTES)
        .map_err(map_transport_err)?;
    domain::ensure_modrinth_ok(resp.status)?;
    let text = bytes_to_utf8(resp.body, "Modrinth project versions")?;
    domain::modrinth_decode_project_versions(&text)
}

// ---------------------------------------------------------------------
// Hangar
// ---------------------------------------------------------------------

fn hangar_base() -> String {
    provider_base("MSC2_PROVIDER_HANGAR_BASE", "https://hangar.papermc.io/api")
}

/// `HangarAPI.fetchLatest(author:slug:mcVersion:)`. Returns the selected
/// latest version and its resolved download URL together, since callers
/// never need one without the other.
pub fn hangar_fetch_latest(
    transport: &dyn AddonTransport,
    author: &str,
    slug: &str,
    mc_version: Option<&str>,
) -> Result<(domain::HangarVersion, String), AddonProviderError> {
    let mut url = format!(
        "{}/v1/projects/{author}/{slug}/versions?platform=PAPER&channel=Release",
        hangar_base()
    );
    if let Some(v) = mc_version {
        url.push_str(&format!("&platformVersion={}", urlencode(v)));
    }
    let resp = transport
        .get(&url, "Hangar versions", &[], RESPONSE_MAX_BYTES)
        .map_err(map_transport_err)?;
    ensure_ok_generic("Hangar", resp.status)?;
    let text = bytes_to_utf8(resp.body, "Hangar versions")?;
    let versions = domain::hangar_decode_versions(&text)?;
    let latest = domain::hangar_select_latest(&versions)?;
    let download_url = domain::hangar_download_url(author, slug, latest);
    Ok((latest.clone(), download_url))
}

// ---------------------------------------------------------------------
// CurseForge
// ---------------------------------------------------------------------

fn curseforge_base() -> String {
    provider_base(
        "MSC2_PROVIDER_CURSEFORGE_BASE",
        "https://api.curseforge.com",
    )
}

fn curseforge_api_key(secrets: &dyn SecretStore) -> Result<String, AddonProviderError> {
    let key = secrets
        .get(CURSEFORGE_API_KEY_SECRET)
        .map_err(|e| {
            AddonProviderError::Network(format!("Could not read CurseForge API key: {e}"))
        })?
        .unwrap_or_default();
    domain::curseforge_require_api_key(&key)?;
    Ok(key)
}

/// `CurseForgeAPI.files(fileIds:apiKey:)`: the missing-key guard runs
/// before any request is built, batched into [`MAX_BATCH_SIZE`]-id chunks,
/// each deduped/sorted per [`domain::curseforge_batched_ids`].
pub fn curseforge_files(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    file_ids: &[i64],
) -> Result<Vec<domain::CurseForgeFile>, AddonProviderError> {
    let api_key = curseforge_api_key(secrets)?;
    let mut result = Vec::new();
    for chunk in file_ids.chunks(MAX_BATCH_SIZE) {
        let ids = domain::curseforge_batched_ids(chunk);
        if ids.is_empty() {
            continue;
        }
        let body = serde_json::json!({ "fileIds": ids });
        let url = format!("{}/v1/mods/files", curseforge_base());
        let resp = transport
            .post_json(
                &url,
                "CurseForge files",
                &body,
                &[("x-api-key", api_key.as_str())],
                RESPONSE_MAX_BYTES,
            )
            .map_err(map_transport_err)?;
        domain::ensure_curseforge_ok(resp.status)?;
        let text = bytes_to_utf8(resp.body, "CurseForge files")?;
        result.extend(domain::curseforge_decode_files(&text)?);
    }
    Ok(result)
}

/// `CurseForgeAPI.mods(modIds:apiKey:)`.
pub fn curseforge_mods(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    mod_ids: &[i64],
) -> Result<Vec<domain::CurseForgeMod>, AddonProviderError> {
    let api_key = curseforge_api_key(secrets)?;
    let mut result = Vec::new();
    for chunk in mod_ids.chunks(MAX_BATCH_SIZE) {
        let ids = domain::curseforge_batched_ids(chunk);
        if ids.is_empty() {
            continue;
        }
        let body = serde_json::json!({ "modIds": ids });
        let url = format!("{}/v1/mods", curseforge_base());
        let resp = transport
            .post_json(
                &url,
                "CurseForge mods",
                &body,
                &[("x-api-key", api_key.as_str())],
                RESPONSE_MAX_BYTES,
            )
            .map_err(map_transport_err)?;
        domain::ensure_curseforge_ok(resp.status)?;
        let text = bytes_to_utf8(resp.body, "CurseForge mods")?;
        result.extend(domain::curseforge_decode_mods(&text)?);
    }
    Ok(result)
}

// ---------------------------------------------------------------------
// GitHub Releases
// ---------------------------------------------------------------------

fn github_base() -> String {
    provider_base("MSC2_PROVIDER_GITHUB_BASE", "https://api.github.com")
}

/// `GitHubReleaseChecker.fetchLatestRelease(owner:repo:)`.
pub fn github_latest_release(
    transport: &dyn AddonTransport,
    owner: &str,
    repo: &str,
) -> Result<domain::GitHubRelease, AddonProviderError> {
    let url = format!("{}/repos/{owner}/{repo}/releases/latest", github_base());
    let resp = transport
        .get(
            &url,
            "GitHub release",
            &[("Accept", "application/vnd.github+json")],
            RESPONSE_MAX_BYTES,
        )
        .map_err(map_transport_err)?;
    ensure_ok_generic("GitHub", resp.status)?;
    let text = bytes_to_utf8(resp.body, "GitHub release")?;
    domain::github_decode_release(&text)
}
