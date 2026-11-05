//! Java CurseForge catalog primitives shared by creation and installed servers.
//! This layer returns provider metadata; callers own compatibility review and
//! staged installation, never trusting a project title as file identity.
use super::*;
use serde::Deserialize;
use serde::de::DeserializeOwned;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: i64,
    pub game_id: i64,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub download_count: f64,
    #[serde(default)]
    pub authors: Vec<Author>,
    pub class_id: Option<i64>,
    pub allow_mod_distribution: Option<bool>,
    pub links: domain::CurseForgeModLinks,
    pub logo: Option<Logo>,
}

#[derive(Debug, Deserialize)]
pub struct Logo {
    pub url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub id: i64,
    pub mod_id: i64,
    pub is_available: bool,
    pub display_name: String,
    pub file_name: String,
    pub file_date: String,
    pub release_type: u32,
    pub file_length: u64,
    pub download_url: Option<String>,
    pub hashes: Vec<domain::CurseForgeFileHash>,
    pub game_versions: Vec<String>,
    pub dependencies: Vec<Dependency>,
    pub is_server_pack: Option<bool>,
    pub server_pack_file_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub mod_id: i64,
    pub relation_type: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub index: u32,
    pub page_size: u32,
    pub result_count: u32,
    pub total_count: u32,
}

#[derive(Debug, Deserialize)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub pagination: Pagination,
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    data: T,
}

/// Both search and release enumeration use provider pagination; never pretend
/// the first page is the entire pack catalog.
pub struct Query<'a> {
    pub text: &'a str,
    pub class_id: Option<u32>,
    pub game_version: Option<&'a str>,
    pub loader_type: Option<u32>,
    pub offset: u32,
    pub limit: u32,
}

impl Query<'_> {
    fn parameters(&self) -> Result<String, AddonProviderError> {
        if self.limit == 0 || self.limit > 50 || self.offset > 10_000 - self.limit {
            return Err(AddonProviderError::Network(
                "Invalid CurseForge page range.".into(),
            ));
        }
        if self.loader_type.is_some_and(|loader| loader > 6) {
            return Err(AddonProviderError::Network(
                "Unknown CurseForge loader type.".into(),
            ));
        }
        let mut params = format!("index={}&pageSize={}", self.offset, self.limit);
        if let Some(version) = self.game_version {
            params.push_str(&format!("&gameVersion={}", urlencode(version)));
        }
        if let Some(loader) = self.loader_type {
            params.push_str(&format!("&modLoaderType={loader}"));
        }
        Ok(params)
    }
}

fn get<T: DeserializeOwned>(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    path: &str,
) -> Result<T, AddonProviderError> {
    let key = curseforge_api_key(secrets)?;
    let response = transport
        .get(
            &format!("{}{path}", curseforge_base()),
            "CurseForge Java catalog",
            &[("x-api-key", key.as_str())],
            RESPONSE_MAX_BYTES,
        )
        .map_err(map_transport_err)?;
    ensure_curseforge_ok_for("Java catalog", response.status)?;
    serde_json::from_slice(&response.body)
        .map_err(|error| malformed("CurseForge Java catalog", error))
}

pub fn search(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    query: &Query<'_>,
) -> Result<Page<Project>, AddonProviderError> {
    let params = query.parameters()?;
    let class = query
        .class_id
        .map(|id| format!("&classId={id}"))
        .unwrap_or_default();
    get(
        transport,
        secrets,
        &format!(
            "/v1/mods/search?gameId=432&searchFilter={}&sortField=2&sortOrder=desc&{params}{class}",
            urlencode(query.text)
        ),
    )
}

pub fn project(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    project_id: u64,
) -> Result<Project, AddonProviderError> {
    let envelope: Envelope<Project> = get(transport, secrets, &format!("/v1/mods/{project_id}"))?;
    if envelope.data.game_id != 432 {
        return Err(AddonProviderError::Network(
            "Project is not Minecraft Java content.".into(),
        ));
    }
    Ok(envelope.data)
}

pub fn description(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    project_id: u64,
) -> Result<String, AddonProviderError> {
    let envelope: Envelope<String> = get(
        transport,
        secrets,
        &format!("/v1/mods/{project_id}/description"),
    )?;
    Ok(envelope.data)
}

pub fn releases(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    project_id: u64,
    query: &Query<'_>,
) -> Result<Page<Release>, AddonProviderError> {
    get(
        transport,
        secrets,
        &format!("/v1/mods/{project_id}/files?{}", query.parameters()?),
    )
}

pub fn release(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    project_id: u64,
    file_id: u64,
) -> Result<Release, AddonProviderError> {
    let envelope: Envelope<Release> = get(
        transport,
        secrets,
        &format!("/v1/mods/{project_id}/files/{file_id}"),
    )?;
    if envelope.data.mod_id != project_id as i64 || envelope.data.id != file_id as i64 {
        return Err(AddonProviderError::Network(
            "CurseForge returned a different file identity.".into(),
        ));
    }
    Ok(envelope.data)
}

pub fn classes(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
) -> Result<Vec<Category>, AddonProviderError> {
    let envelope: Envelope<Vec<Category>> = get(
        transport,
        secrets,
        "/v1/categories?gameId=432&classesOnly=true",
    )?;
    Ok(envelope.data)
}

#[derive(Debug, Deserialize)]
pub struct Category {
    pub id: u32,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Deserialize)]
pub struct Author {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FingerprintMatches {
    pub is_cache_built: bool,
    #[serde(default)]
    pub exact_matches: Vec<FingerprintMatch>,
}

#[derive(Debug, Deserialize)]
pub struct FingerprintMatch {
    pub id: i64,
    pub file: Release,
}

pub fn fingerprint_matches(
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    fingerprints: &[u32],
) -> Result<Vec<FingerprintMatch>, AddonProviderError> {
    let key = curseforge_api_key(secrets)?;
    let mut matches = Vec::new();
    for chunk in fingerprints.chunks(MAX_BATCH_SIZE) {
        let response = transport
            .post_json(
                &format!("{}/v1/fingerprints/432", curseforge_base()),
                "CurseForge fingerprints",
                &serde_json::json!({"fingerprints": chunk}),
                &[("x-api-key", key.as_str())],
                RESPONSE_MAX_BYTES,
            )
            .map_err(map_transport_err)?;
        ensure_curseforge_ok_for("fingerprint lookup", response.status)?;
        let envelope: Envelope<FingerprintMatches> = serde_json::from_slice(&response.body)
            .map_err(|error| malformed("CurseForge fingerprints", error))?;
        if !envelope.data.is_cache_built {
            return Err(AddonProviderError::Network(
                "CurseForge fingerprint index is unavailable".into(),
            ));
        }
        matches.extend(envelope.data.exact_matches);
    }
    Ok(matches)
}
