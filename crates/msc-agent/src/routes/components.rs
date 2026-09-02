//! P8.24: add-on, catalog, client-export, modpack, and shared staging
//! routes wired to the real Phase 8 application services.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use axum::body::Bytes;
use axum::extract::rejection::JsonRejection;
use axum::extract::{DefaultBodyLimit, Extension, Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use msc_api::dto::{
    AddonItemDto, AddonRemoveRequestDto, AddonRemoveResultDto, AddonUpdateResultDto,
    AddonsResponseDto, CatalogGalleryImageDto, CatalogInstallRequestDto, CatalogInstallResultDto,
    CatalogItemDto, CatalogProjectDetailDto, CatalogSearchResponseDto, CatalogVersionDependencyDto,
    CatalogVersionDto, CatalogVersionFileDto, CatalogVersionsResponseDto, ClientExportItemDto,
    ClientExportResponseDto, ComponentStatusDto, ComponentUpdateRequestDto, ComponentsStatusDto,
    ModpackImportRequestDto, ModpackImportResultDto, ModpackInspectionRequestDto,
    ModpackInspectionResultDto, ModpackManualFileDto, ModpackManualFileRequestDto,
    ModpackManualFileResultDto, PermissionCategoryDto, StagedUploadBeginRequestDto,
    StagedUploadBeginResultDto, StagedUploadCompleteResultDto, StagedUploadPurposeDto,
};
use msc_application::addon_updates;
use msc_application::addons::{self, AddonMutationError};
use msc_application::client_export::{self, ClientSideStatus};
use msc_application::curseforge_manual::{self, PendingManualFile};
use msc_application::geyser;
use msc_application::modpacks;
use msc_domain::addon_provider::{self, AddonProviderError};
use msc_domain::addon_update::AddonUpdateBucket;
use msc_domain::app_config_schema::{AddonLink, AddonLinkProvenance, PluginSourceConfig};
use msc_domain::identity::{AddOnKind, JavaServerFlavor, ServerType};
use msc_infrastructure::addon_provider::{self as provider, AddonTransport, HttpTransport};
use msc_infrastructure::audit_log::Entry as AuditEntry;
use msc_infrastructure::download_staging::sha512_hex;
use msc_infrastructure::fs::StdFileSystem;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::auth::{AuthenticatedCredential, production_secret_store};
use crate::routes::lifecycle::{
    LifecycleRoutesState, TryMutateError, error_response, invalid_body, require_permission,
};
use crate::routes::worlds::{
    StagedDownload, StagedUpload, StagingStore, now_unix, staging_root, unix_to_iso8601,
};

const MAX_STAGED_UPLOAD_BYTES: u64 = 10 * 1024 * 1024 * 1024;
const MAX_LOCAL_ADDON_UPLOAD_BYTES: u64 = 512 * 1024 * 1024;
const STAGING_TTL_SECONDS: u64 = 30 * 60;

#[derive(Clone)]
pub struct ComponentsRoutesState {
    pub lifecycle: LifecycleRoutesState,
    staging: StagingStore,
    pending_modpack_imports: Arc<Mutex<HashMap<String, PendingModpackImport>>>,
}

#[derive(Debug, Clone)]
struct PendingModpackImport {
    remaining_manual_files: Vec<PendingManualFile>,
}

impl ComponentsRoutesState {
    pub fn new(lifecycle: LifecycleRoutesState, staging: StagingStore) -> Self {
        Self {
            lifecycle,
            staging,
            pending_modpack_imports: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogQuery {
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    offset: Option<usize>,
    /// Search against this flavor instead of the active server's -- lets a
    /// caller with no active server yet (the Add Server wizard's Add-ons
    /// step, before the server it's configuring exists) search for real.
    /// When present, `minecraft_version` is used too if given; the active
    /// server is not consulted at all. Absent, this route's behavior is
    /// unchanged from before this field existed.
    #[serde(default)]
    java_flavor: Option<String>,
    #[serde(default)]
    minecraft_version: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientExportQuery {
    #[serde(default)]
    selected: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct ModrinthCatalogVersion {
    id: String,
    project_id: String,
    name: String,
    version_number: String,
    version_type: String,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    #[serde(default)]
    date_published: Option<String>,
    #[serde(default)]
    dependencies: Vec<ModrinthCatalogDependency>,
    #[serde(default)]
    files: Vec<ModrinthCatalogFile>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
struct ModrinthCatalogDependency {
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    version_id: Option<String>,
    dependency_type: String,
}

#[derive(Debug, serde::Deserialize)]
struct ModrinthCatalogFile {
    url: String,
    filename: String,
    primary: bool,
    #[serde(default)]
    size: Option<i64>,
}

pub fn router(state: ComponentsRoutesState) -> Router {
    Router::new()
        .route("/staged-uploads", post(begin_staged_upload))
        .route(
            "/staged-uploads/:id",
            put(upload_staged_bytes)
                .route_layer(DefaultBodyLimit::max(MAX_STAGED_UPLOAD_BYTES as usize)),
        )
        .route("/staged-downloads/:id", get(download_staged_bytes))
        .route("/addons", get(get_addons))
        .route("/catalog/search", get(search_catalog))
        .route("/catalog/projects/:project_id", get(get_catalog_project))
        .route(
            "/catalog/projects/:project_id/versions",
            get(get_catalog_project_versions),
        )
        .route("/components", get(get_components))
        .route("/components/client-export", get(get_client_export))
        .route("/components/install", post(install_component))
        .route("/components/remove", post(remove_component))
        .route("/components/update", post(update_component))
        .route("/modpacks/inspect", post(inspect_modpack))
        .route("/modpacks/import", post(import_modpack))
        .route(
            "/modpacks/:operation_id/manual-file",
            post(complete_modpack_manual_file),
        )
        .with_state(state)
}

fn audit(
    state: &LifecycleRoutesState,
    credential: &AuthenticatedCredential,
    method: &str,
    path: &str,
    status: StatusCode,
) {
    let _ = state.audit_log().log(&AuditEntry {
        timestamp: SystemTime::now(),
        client_ip: String::new(),
        token_label: credential.label.clone(),
        method: method.to_string(),
        path: path.to_string(),
        status_code: status.as_u16(),
    });
}

fn no_active_server() -> Response {
    error_response(
        StatusCode::CONFLICT,
        "no_active_server",
        "No active server.",
    )
}

fn addon_bucket_name(bucket: AddonUpdateBucket) -> &'static str {
    match bucket {
        AddonUpdateBucket::UpdateAvailable => "updateAvailable",
        AddonUpdateBucket::NoCompatibleVersion => "noCompatibleVersion",
        AddonUpdateBucket::UpToDate => "upToDate",
        AddonUpdateBucket::Unlinked => "unlinked",
    }
}

fn client_status_name(status: ClientSideStatus) -> &'static str {
    match status {
        ClientSideStatus::Required => "required",
        ClientSideStatus::Optional => "optional",
        ClientSideStatus::ServerOnly => "server_only",
        ClientSideStatus::Unknown => "unknown",
    }
}

fn parse_selected_ids(query: &ClientExportQuery) -> Option<Vec<String>> {
    query.selected.as_ref().map(|raw| {
        raw.split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect()
    })
}

fn modrinth_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn modrinth_get(path: &str, what: &str) -> Result<String, AddonProviderError> {
    let base = std::env::var("MSC2_PROVIDER_MODRINTH_BASE")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://api.modrinth.com".to_string());
    let transport = HttpTransport::new();
    let response = transport
        .get(
            &format!("{}/v2/{path}", base.trim_end_matches('/')),
            what,
            &[],
            provider::RESPONSE_MAX_BYTES,
        )
        .map_err(|error| AddonProviderError::Network(error.to_string()))?;
    addon_provider::ensure_modrinth_ok(response.status)?;
    String::from_utf8(response.body)
        .map_err(|_| AddonProviderError::Network(format!("{what}: response was not valid UTF-8.")))
}

fn provider_error(error: AddonProviderError) -> Response {
    error_response(
        StatusCode::BAD_GATEWAY,
        "provider_unavailable",
        &error.to_string(),
    )
}

fn is_paper_like(flavor: JavaServerFlavor) -> bool {
    matches!(
        flavor,
        JavaServerFlavor::Paper
            | JavaServerFlavor::Purpur
            | JavaServerFlavor::Spigot
            | JavaServerFlavor::Pufferfish
    )
}

fn component_rows(server: &msc_domain::app_config_schema::ConfigServer) -> Vec<ComponentStatusDto> {
    let mut rows = Vec::new();
    if server.server_type == ServerType::Java {
        let installed_label = match (&server.minecraft_version, &server.server_build) {
            (Some(version), Some(build)) => Some(format!("{version} · {build}")),
            (Some(version), None) => Some(version.clone()),
            (None, Some(build)) => Some(build.clone()),
            (None, None) => None,
        };
        rows.push(ComponentStatusDto {
            name: server.java_flavor.raw_value().to_string(),
            installed_build: None,
            latest_build: None,
            installed_version: server.minecraft_version.clone(),
            latest_version: server.minecraft_version.clone(),
            is_up_to_date: true,
            installed_label,
            updatable: Some(true),
            note: None,
        });
        // Geyser and Floodgate are managed compatibility helpers, not
        // catalog add-ons.  Their builds have no provider-backed update check
        // yet, so expose the installed fact and say that honestly.
        let installation = geyser::installation(&StdFileSystem, Path::new(&server.server_dir));
        for (name, installed, plugin_path) in [
            (
                "Geyser",
                installation.geyser_installed,
                installation.geyser_path.as_deref(),
            ),
            (
                "Floodgate",
                installation.floodgate_installed,
                installation.floodgate_path.as_deref(),
            ),
        ] {
            if installed {
                let installed_metadata = plugin_path
                    .and_then(|path| geyser::installed_plugin_version(&StdFileSystem, path));
                let installed_label = installed_metadata.as_ref().map(|metadata| {
                    metadata
                        .build
                        .map(|build| build.to_string())
                        .unwrap_or_else(|| metadata.version.clone())
                });
                rows.push(ComponentStatusDto {
                    name: name.to_string(),
                    installed_build: installed_metadata
                        .as_ref()
                        .and_then(|metadata| metadata.build),
                    latest_build: None,
                    installed_version: installed_metadata.map(|metadata| metadata.version),
                    latest_version: None,
                    is_up_to_date: false,
                    installed_label: installed_label.or_else(|| Some("installed".to_string())),
                    updatable: Some(false),
                    note: Some("update_information_unavailable".to_string()),
                });
            }
        }
    }
    rows
}

fn add_on_dir(server: &msc_domain::app_config_schema::ConfigServer) -> Option<PathBuf> {
    server
        .java_flavor
        .add_on_kind()
        .map(|kind| Path::new(&server.server_dir).join(kind.folder_name()))
}

fn add_on_kind_name(kind: AddOnKind) -> &'static str {
    match kind {
        AddOnKind::Mod => "mod",
        AddOnKind::Plugin => "plugin",
    }
}

fn loaders_for(flavor: JavaServerFlavor) -> Vec<String> {
    flavor
        .modrinth_loader_facets()
        .iter()
        .map(|loader| loader.to_string())
        .collect()
}

fn install_result(project_id: Option<String>, operation_id: &str, message: &str) -> Response {
    (
        StatusCode::ACCEPTED,
        Json(CatalogInstallResultDto {
            success: true,
            message: message.to_string(),
            project_id,
            operation_id: Some(operation_id.to_string()),
            installed_dependencies: Vec::new(),
        }),
    )
        .into_response()
}

fn update_result(
    result: &str,
    jar_stem: Option<String>,
    count: i64,
    operation_id: Option<String>,
) -> Response {
    (
        if operation_id.is_some() {
            StatusCode::ACCEPTED
        } else {
            StatusCode::OK
        },
        Json(AddonUpdateResultDto {
            result: result.to_string(),
            jar_stem,
            count,
            operation_id,
        }),
    )
        .into_response()
}

fn add_or_update_link(
    state: &LifecycleRoutesState,
    server_id: &str,
    project_id: &str,
    title: Option<String>,
    slug: Option<String>,
    installed_path: Option<&Path>,
) -> Result<(), AddonMutationError> {
    state
        .try_mutate_config(|config| {
            let server = config
                .servers
                .iter_mut()
                .find(|server| server.id == server_id)
                .ok_or(())?;
            let links = server.addon_links.get_or_insert_with(HashMap::new);
            let mut link = links.get(project_id).cloned().unwrap_or(AddonLink {
                project_id: project_id.to_string(),
                title: None,
                slug: None,
                icon_url: None,
                provenance: AddonLinkProvenance::UserLinked,
                installed_version_id: None,
                installed_file_name: None,
                installed_hash: None,
                client_side: None,
                server_side: None,
                extra: Default::default(),
            });
            if let Some(title) = title {
                link.title = Some(title);
            }
            if let Some(slug) = slug {
                link.slug = Some(slug);
            }
            if let Some(path) = installed_path {
                link.installed_file_name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_string);
                if let Ok(bytes) = std::fs::read(path) {
                    link.installed_hash = Some(sha512_hex(&bytes));
                }
            }
            links.insert(project_id.to_string(), link);
            Ok(())
        })
        .map_err(|error| match error {
            TryMutateError::Domain(()) | TryMutateError::Save(_) => {
                AddonMutationError::Io("Could not persist add-on metadata.".to_string())
            }
        })
}

fn set_pack_metadata(
    state: &LifecycleRoutesState,
    server_id: &str,
    pack_name: &str,
    pack_version: &str,
) -> Result<(), String> {
    state
        .try_mutate_config(|config| {
            let server = config
                .servers
                .iter_mut()
                .find(|server| server.id == server_id)
                .ok_or(())?;
            server.pack_managed = true;
            server.pack_name = Some(pack_name.to_string());
            server.pack_version = Some(pack_version.to_string());
            Ok::<(), ()>(())
        })
        .map_err(|_: TryMutateError<()>| "Could not persist pack metadata.".to_string())
}

pub async fn begin_staged_upload(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<StagedUploadBeginRequestDto>, JsonRejection>,
) -> Response {
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };

    let max_bytes = match body.purpose {
        StagedUploadPurposeDto::WorldImport
        | StagedUploadPurposeDto::ActiveWorldReplace
        | StagedUploadPurposeDto::WorldThumbnail => MAX_STAGED_UPLOAD_BYTES,
        StagedUploadPurposeDto::ModpackArchive => MAX_STAGED_UPLOAD_BYTES,
        StagedUploadPurposeDto::AddonLocalFile => MAX_LOCAL_ADDON_UPLOAD_BYTES,
        StagedUploadPurposeDto::CurseforgeManualFile => {
            let operation_id = body
                .operation_id
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| invalid_body("missing_operation_id", "operationId is required."));
            let file_id = body
                .file_id
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| invalid_body("missing_file_id", "fileId is required."));
            let (operation_id, file_id) = match (operation_id, file_id) {
                (Ok(operation_id), Ok(file_id)) => (operation_id, file_id),
                (Err(response), _) | (_, Err(response)) => return response,
            };
            let pending = state.pending_modpack_imports.lock().unwrap();
            let Some(entry) = pending.get(operation_id) else {
                return error_response(
                    StatusCode::NOT_FOUND,
                    "not_found",
                    "No pending modpack import matched that operation.",
                );
            };
            let Some(file) = entry
                .remaining_manual_files
                .iter()
                .find(|file| file.file_id.to_string() == file_id)
            else {
                return error_response(
                    StatusCode::NOT_FOUND,
                    "not_found",
                    "No pending manual file matched that fileId.",
                );
            };
            file.expected_byte_size
        }
    };

    let id = Uuid::new_v4().to_string();
    let expires_at_unix = now_unix() + STAGING_TTL_SECONDS;
    let uploads_dir = staging_root(&state.lifecycle.servers_root()).join("uploads");
    let path = uploads_dir.join(format!("{id}.bin"));
    state.staging.uploads.lock().unwrap().insert(
        id.clone(),
        StagedUpload {
            purpose: body.purpose,
            expires_at_unix,
            max_bytes,
            path,
        },
    );
    let response = Json(StagedUploadBeginResultDto {
        staged_upload_id: id.clone(),
        upload_path: format!("/v1/staged-uploads/{id}"),
        expires_at: unix_to_iso8601(expires_at_unix),
        max_bytes: i64::try_from(max_bytes).unwrap_or(i64::MAX),
    })
    .into_response();
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/staged-uploads",
        response.status(),
    );
    response
}

pub async fn upload_staged_bytes(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(id): AxumPath<String>,
    body: Bytes,
) -> Response {
    let entry = { state.staging.uploads.lock().unwrap().get(&id).cloned() };
    let Some(entry) = entry else {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged upload.",
        );
    };
    if now_unix() > entry.expires_at_unix {
        state.staging.uploads.lock().unwrap().remove(&id);
        return error_response(
            StatusCode::CONFLICT,
            "staged_upload_expired",
            "This staged upload has expired.",
        );
    }
    if body.len() as u64 > entry.max_bytes {
        return error_response(
            StatusCode::CONFLICT,
            "max_bytes_exceeded",
            "Upload exceeds the staged upload's byte ceiling.",
        );
    }
    if let Some(parent) = entry.path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&entry.path, &body).is_err() {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Could not write staged upload.",
        );
    }
    let mut hasher = Sha256::new();
    hasher.update(&body);
    let response = Json(StagedUploadCompleteResultDto {
        staged_upload_id: id.clone(),
        received_bytes: body.len() as i64,
        sha256: format!("{:x}", hasher.finalize()),
    })
    .into_response();
    audit(
        &state.lifecycle,
        &credential,
        "PUT",
        "/v1/staged-uploads/:id",
        response.status(),
    );
    response
}

pub async fn download_staged_bytes(
    State(state): State<ComponentsRoutesState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let entry = { state.staging.downloads.lock().unwrap().remove(&id) };
    let Some(entry) = entry else {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown staged download.",
        );
    };
    if now_unix() > entry.expires_at_unix {
        let _ = std::fs::remove_file(&entry.path);
        return error_response(
            StatusCode::CONFLICT,
            "staged_download_expired",
            "This staged download has expired.",
        );
    }
    match std::fs::read(&entry.path) {
        Ok(bytes) => {
            let _ = std::fs::remove_file(&entry.path);
            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
                bytes,
            )
                .into_response()
        }
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Could not read staged download.",
        ),
    }
}

pub async fn get_components(
    State(state): State<ComponentsRoutesState>,
    Extension(_credential): Extension<AuthenticatedCredential>,
) -> Response {
    let Some(server) = state.lifecycle.active_config_server() else {
        return Json(ComponentsStatusDto {
            components: Vec::new(),
            restart_required_to_apply: false,
        })
        .into_response();
    };
    Json(ComponentsStatusDto {
        components: component_rows(&server),
        restart_required_to_apply: false,
    })
    .into_response()
}

pub async fn get_addons(
    State(state): State<ComponentsRoutesState>,
    Extension(_credential): Extension<AuthenticatedCredential>,
) -> Response {
    let Some(server) = state.lifecycle.active_config_server() else {
        return Json(AddonsResponseDto {
            addons: Vec::new(),
            is_resolving: false,
            server_supports_addons: false,
            pack_managed: None,
            pack_name: None,
            note: Some("No active server.".to_string()),
        })
        .into_response();
    };
    let Some(add_on_dir) = add_on_dir(&server) else {
        return Json(AddonsResponseDto {
            addons: Vec::new(),
            is_resolving: false,
            server_supports_addons: false,
            pack_managed: Some(server.pack_managed),
            pack_name: server.pack_name.clone(),
            note: Some("This server flavor has no add-ons.".to_string()),
        })
        .into_response();
    };
    let transport = HttpTransport::new();
    let plan = addon_updates::resolve_addon_updates(
        &transport,
        &StdFileSystem,
        &add_on_dir,
        server.java_flavor,
        server.minecraft_version.as_deref(),
        &server.addon_links.clone().unwrap_or_default(),
        &server.plugin_sources.clone().unwrap_or_default(),
    );
    Json(AddonsResponseDto {
        addons: plan
            .items
            .into_iter()
            .map(|item| AddonItemDto {
                jar_stem: item.jar_stem,
                display_name: item.display_name,
                is_enabled: item.is_enabled,
                project_id: item.project_id,
                current_version: item.current_version,
                available_version: item.available_version_label,
                bucket: addon_bucket_name(item.bucket).to_string(),
                icon_url: None,
            })
            .collect(),
        is_resolving: false,
        server_supports_addons: true,
        pack_managed: Some(server.pack_managed),
        pack_name: server.pack_name,
        note: None,
    })
    .into_response()
}

pub async fn search_catalog(
    State(state): State<ComponentsRoutesState>,
    Extension(_credential): Extension<AuthenticatedCredential>,
    Query(query): Query<CatalogQuery>,
) -> Response {
    // A `javaFlavor` query param searches against that flavor directly,
    // without any active server at all -- the Add Server wizard's Add-ons
    // step uses this to search Modrinth for the flavor it's configuring,
    // before that server exists. Absent, this falls back to the active
    // server exactly as before this param existed.
    let (java_flavor, minecraft_version) = if let Some(requested) = query
        .java_flavor
        .as_deref()
        .and_then(JavaServerFlavor::from_raw_value)
    {
        (requested, query.minecraft_version.clone())
    } else {
        let Some(server) = state.lifecycle.active_config_server() else {
            return Json(CatalogSearchResponseDto {
                supports_addons: false,
                addon_kind: None,
                loader_name: None,
                game_version: None,
                results: Vec::new(),
                note: Some("No active server.".to_string()),
            })
            .into_response();
        };
        (server.java_flavor, server.minecraft_version.clone())
    };
    let Some(add_on_kind) = java_flavor.add_on_kind() else {
        return Json(CatalogSearchResponseDto {
            supports_addons: false,
            addon_kind: None,
            loader_name: None,
            game_version: minecraft_version,
            results: Vec::new(),
            note: Some("This server flavor has no add-ons.".to_string()),
        })
        .into_response();
    };
    let transport = HttpTransport::new();
    let loaders = loaders_for(java_flavor);
    let search = match provider::modrinth_search(
        &transport,
        query.q.as_deref().unwrap_or_default(),
        add_on_kind_name(add_on_kind),
        &loaders,
        minecraft_version.as_deref(),
        20,
        query.offset.unwrap_or(0).try_into().unwrap_or(0),
    ) {
        Ok(search) => search,
        Err(error) => {
            return Json(CatalogSearchResponseDto {
                supports_addons: true,
                addon_kind: Some(add_on_kind_name(add_on_kind).to_string()),
                loader_name: Some(java_flavor.raw_value().to_string()),
                game_version: minecraft_version,
                results: Vec::new(),
                note: Some(error.to_string()),
            })
            .into_response();
        }
    };
    Json(CatalogSearchResponseDto {
        supports_addons: true,
        addon_kind: Some(add_on_kind_name(add_on_kind).to_string()),
        loader_name: Some(java_flavor.raw_value().to_string()),
        game_version: minecraft_version,
        results: search
            .hits
            .into_iter()
            .map(|hit| {
                let is_client_only = hit.is_client_only();
                CatalogItemDto {
                    project_id: hit.project_id,
                    slug: hit.slug,
                    title: hit.title,
                    description: hit.description,
                    author: hit.author,
                    downloads: hit.downloads,
                    icon_url: hit.icon_url,
                    is_client_only,
                    project_type: add_on_kind_name(add_on_kind).to_string(),
                }
            })
            .collect(),
        note: None,
    })
    .into_response()
}

pub async fn get_catalog_project(
    State(_state): State<ComponentsRoutesState>,
    Extension(_credential): Extension<AuthenticatedCredential>,
    AxumPath(project_id): AxumPath<String>,
) -> Response {
    let body = match modrinth_get(
        &format!("project/{}", modrinth_path_segment(&project_id)),
        "Modrinth project",
    ) {
        Ok(body) => body,
        Err(error) => return provider_error(error),
    };
    let project = match addon_provider::modrinth_decode_project_detail(&body) {
        Ok(project) => project,
        Err(error) => return provider_error(error),
    };
    Json(CatalogProjectDetailDto {
        project_id: project.id,
        slug: project.slug,
        title: project.title,
        description: project.description,
        body: project.body,
        icon_url: project.icon_url,
        downloads: project.downloads,
        followers: project.followers,
        server_side: project.server_side,
        gallery: project
            .gallery
            .into_iter()
            .map(|image| CatalogGalleryImageDto {
                url: image.url,
                title: image.title,
                description: image.description,
                featured: image.featured,
            })
            .collect(),
        source_url: project.source_url,
        issues_url: project.issues_url,
        wiki_url: project.wiki_url,
        discord_url: project.discord_url,
    })
    .into_response()
}

pub async fn get_catalog_project_versions(
    State(_state): State<ComponentsRoutesState>,
    Extension(_credential): Extension<AuthenticatedCredential>,
    AxumPath(project_id): AxumPath<String>,
) -> Response {
    let body = match modrinth_get(
        &format!("project/{}/version", modrinth_path_segment(&project_id)),
        "Modrinth project versions",
    ) {
        Ok(body) => body,
        Err(error) => return provider_error(error),
    };
    let versions: Vec<ModrinthCatalogVersion> = match serde_json::from_str(&body) {
        Ok(versions) => versions,
        Err(error) => {
            return provider_error(AddonProviderError::Network(format!(
                "Malformed Modrinth project versions response: {error}"
            )));
        }
    };
    Json(CatalogVersionsResponseDto {
        versions: versions
            .into_iter()
            .map(|version| CatalogVersionDto {
                id: version.id,
                project_id: version.project_id,
                name: version.name,
                version_number: version.version_number,
                version_type: version.version_type,
                game_versions: version.game_versions,
                loaders: version.loaders,
                date_published: version.date_published,
                dependencies: version
                    .dependencies
                    .into_iter()
                    .map(|dependency| CatalogVersionDependencyDto {
                        project_id: dependency.project_id,
                        version_id: dependency.version_id,
                        dependency_type: dependency.dependency_type,
                    })
                    .collect(),
                files: version
                    .files
                    .into_iter()
                    .map(|file| CatalogVersionFileDto {
                        url: file.url,
                        filename: file.filename,
                        primary: file.primary,
                        size: file.size,
                    })
                    .collect(),
            })
            .collect(),
    })
    .into_response()
}

pub async fn get_client_export(
    State(state): State<ComponentsRoutesState>,
    Extension(_credential): Extension<AuthenticatedCredential>,
    Query(query): Query<ClientExportQuery>,
) -> Response {
    let Some(server) = state.lifecycle.active_config_server() else {
        return Json(ClientExportResponseDto {
            server_name: None,
            server_type: "unknown".to_string(),
            export_kind: "links".to_string(),
            is_paper_like: false,
            items: Vec::new(),
            selected_count: 0,
            share_text: None,
            zip_file_name: None,
            staged_download_id: None,
            note: Some("java_addons_only".to_string()),
        })
        .into_response();
    };
    if server.server_type != ServerType::Java || server.java_flavor.add_on_kind().is_none() {
        return Json(ClientExportResponseDto {
            server_name: Some(server.display_name),
            server_type: server.server_type.raw_value().to_string(),
            export_kind: "links".to_string(),
            is_paper_like: false,
            items: Vec::new(),
            selected_count: 0,
            share_text: None,
            zip_file_name: None,
            staged_download_id: None,
            note: Some("java_addons_only".to_string()),
        })
        .into_response();
    }

    let mut items = client_export::build_client_export_items(&StdFileSystem, &server);
    let requested = parse_selected_ids(&query);
    if let Some(selected_ids) = &requested {
        for item in &mut items {
            item.is_selected = selected_ids.contains(&item.jar_stem);
        }
    }

    if items.is_empty() {
        return Json(ClientExportResponseDto {
            server_name: Some(server.display_name),
            server_type: server.server_type.raw_value().to_string(),
            export_kind: if is_paper_like(server.java_flavor) {
                "links".to_string()
            } else {
                "zip".to_string()
            },
            is_paper_like: is_paper_like(server.java_flavor),
            items: Vec::new(),
            selected_count: 0,
            share_text: None,
            zip_file_name: None,
            staged_download_id: None,
            note: Some("empty".to_string()),
        })
        .into_response();
    }

    let response_items: Vec<ClientExportItemDto> = items
        .iter()
        .map(|item| ClientExportItemDto {
            id: item.jar_stem.clone(),
            file_name: item.file_name.clone(),
            display_name: item.display_name.clone(),
            icon_url: item.icon_url.clone(),
            project_url: item.modrinth_url(),
            client_status: client_status_name(item.client_status).to_string(),
            status_source: item.status_source.clone(),
            selected_by_default: item.client_status.is_selected_by_default(),
        })
        .collect();
    let selected_count = items.iter().filter(|item| item.is_selected).count() as i64;
    let paper_like = is_paper_like(server.java_flavor);
    if paper_like {
        return Json(ClientExportResponseDto {
            server_name: Some(server.display_name),
            server_type: server.server_type.raw_value().to_string(),
            export_kind: "links".to_string(),
            is_paper_like: true,
            items: response_items,
            selected_count,
            share_text: client_export::client_links_text(&items),
            zip_file_name: None,
            staged_download_id: None,
            note: if selected_count == 0 {
                Some("nothing_selected".to_string())
            } else {
                None
            },
        })
        .into_response();
    }

    let staged_download_id = if selected_count == 0 {
        None
    } else {
        let id = Uuid::new_v4().to_string();
        let downloads_dir = staging_root(&state.lifecycle.servers_root()).join("downloads");
        let path = downloads_dir.join(format!("{id}.zip"));
        let _ = std::fs::create_dir_all(&downloads_dir);
        if client_export::write_client_export_zip(&items, &path).is_ok() {
            state.staging.downloads.lock().unwrap().insert(
                id.clone(),
                StagedDownload {
                    expires_at_unix: now_unix() + STAGING_TTL_SECONDS,
                    path,
                },
            );
            Some(id)
        } else {
            None
        }
    };

    Json(ClientExportResponseDto {
        server_name: Some(server.display_name.clone()),
        server_type: server.server_type.raw_value().to_string(),
        export_kind: "zip".to_string(),
        is_paper_like: false,
        items: response_items,
        selected_count,
        share_text: None,
        zip_file_name: Some(format!("{}-client-export.zip", server.id)),
        staged_download_id,
        note: if selected_count == 0 {
            Some("nothing_selected".to_string())
        } else {
            None
        },
    })
    .into_response()
}

pub async fn install_component(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<CatalogInstallRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let Some(server) = state.lifecycle.active_config_server() else {
        return no_active_server();
    };
    let Some(add_on_dir) = add_on_dir(&server) else {
        return error_response(
            StatusCode::CONFLICT,
            "not_supported",
            "This server's flavor has no add-ons.",
        );
    };
    let operation_id = match state.lifecycle.operations().begin_lifecycle(
        "addon-install",
        Some(server.id.clone()),
        "Installing add-on.",
    ) {
        Ok(id) => id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };

    let result = if let Some(staged_upload_id) = body.staged_upload_id.as_deref() {
        let entry = state
            .staging
            .uploads
            .lock()
            .unwrap()
            .remove(staged_upload_id);
        let Some(entry) = entry else {
            return error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                "Unknown or already-redeemed staged upload.",
            );
        };
        if now_unix() > entry.expires_at_unix
            || !matches!(entry.purpose, StagedUploadPurposeDto::AddonLocalFile)
        {
            return error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                "Unknown or already-redeemed staged upload.",
            );
        }
        let filename = entry
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("uploaded.jar")
            .to_string();
        let installed = addons::install_from_staged_local_jar(
            &StdFileSystem,
            Path::new(&server.server_dir),
            server.java_flavor,
            &entry.path,
            &filename,
            server.pack_managed,
        );
        let _ = std::fs::remove_file(&entry.path);
        installed.map(|_| None::<(String, String)>)
    } else {
        let project_id = body
            .project_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid_body("missing_project_id", "projectId is required."));
        let project_id = match project_id {
            Ok(project_id) => project_id,
            Err(response) => return response,
        };
        let transport = HttpTransport::new();
        let versions = if let Some(version_id) = body
            .version_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            modrinth_get(
                &format!("version/{}", modrinth_path_segment(version_id)),
                "Modrinth version",
            )
            .and_then(|body| addon_provider::modrinth_decode_version(&body))
            .map(|version| vec![version])
        } else {
            provider::modrinth_project_versions(
                &transport,
                project_id,
                &loaders_for(server.java_flavor),
                server.minecraft_version.as_deref(),
            )
        };
        let versions = match versions {
            Ok(versions) => versions,
            Err(error) => {
                let _ = state.lifecycle.finish_operation_failure(
                    &operation_id,
                    "provider_unavailable",
                    error.to_string(),
                );
                return install_result(
                    body.project_id.clone(),
                    operation_id.as_str(),
                    "Install started.",
                );
            }
        };
        let Some(version) = versions.first() else {
            let _ = state.lifecycle.finish_operation_failure(
                &operation_id,
                "no_compatible_version",
                "No compatible version was found.".to_string(),
            );
            return install_result(
                body.project_id.clone(),
                operation_id.as_str(),
                "Install started.",
            );
        };
        let installed_mod_ids: Vec<String> =
            if server.java_flavor.add_on_kind() == Some(AddOnKind::Mod) {
                addon_updates::resolve_addon_updates(
                    &transport,
                    &StdFileSystem,
                    &add_on_dir,
                    server.java_flavor,
                    server.minecraft_version.as_deref(),
                    &server.addon_links.clone().unwrap_or_default(),
                    &server.plugin_sources.clone().unwrap_or_default(),
                )
                .items
                .into_iter()
                .filter_map(|item| item.project_id)
                .collect()
            } else {
                Vec::new()
            };
        addons::install_from_catalog(
            &transport,
            &StdFileSystem,
            Path::new(&server.server_dir),
            server.java_flavor,
            version,
            server.minecraft_version.as_deref(),
            &installed_mod_ids,
            server.pack_managed,
            &|| false,
        )
        .and_then(|outcome| {
            if let Some(project_id) = body.project_id.as_deref() {
                add_or_update_link(
                    &state.lifecycle,
                    &server.id,
                    project_id,
                    body.title.clone(),
                    body.slug.clone(),
                    Some(&outcome.installed_path),
                )?;
            }
            Ok(body.project_id.clone().zip(body.slug.clone()))
        })
    };

    match result {
        Ok(_) => {
            let _ = state.lifecycle.finish_operation_success(
                &operation_id,
                "Installed add-on.",
                BTreeMap::new(),
            );
        }
        Err(error) => {
            let _ = state.lifecycle.finish_operation_failure(
                &operation_id,
                if matches!(error, AddonMutationError::PackManaged) {
                    "conflict"
                } else {
                    "internal_error"
                },
                error.to_string(),
            );
        }
    }

    let response = install_result(
        body.project_id.clone(),
        operation_id.as_str(),
        "Install started.",
    );
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/components/install",
        response.status(),
    );
    response
}

pub async fn remove_component(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<AddonRemoveRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let Some(server) = state.lifecycle.active_config_server() else {
        return no_active_server();
    };
    let Some(add_on_dir) = add_on_dir(&server) else {
        return error_response(
            StatusCode::CONFLICT,
            "not_supported",
            "This server's flavor has no add-ons.",
        );
    };
    let existing = std::fs::read_dir(&add_on_dir)
        .ok()
        .into_iter()
        .flatten()
        .find_map(|entry| {
            let path = entry.ok()?.path();
            let file_name = path.file_name()?.to_str()?;
            if file_name == format!("{}.jar", body.jar_stem)
                || file_name == format!("{}.jar.disabled", body.jar_stem)
            {
                Some(path)
            } else {
                None
            }
        });
    let Some(path) = existing else {
        return error_response(StatusCode::NOT_FOUND, "not_found", "Add-on not found.");
    };
    match addons::remove(&StdFileSystem, &path, server.pack_managed) {
        Ok(()) => Json(AddonRemoveResultDto {
            success: true,
            message: "Removed add-on.".to_string(),
            jar_stem: body.jar_stem,
        })
        .into_response(),
        Err(AddonMutationError::PackManaged) => error_response(
            StatusCode::CONFLICT,
            "conflict",
            "This server is managed by a modpack.",
        ),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        ),
    }
}

pub async fn update_component(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<ComponentUpdateRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let Some(server) = state.lifecycle.active_config_server() else {
        return no_active_server();
    };
    let Some(add_on_dir) = add_on_dir(&server) else {
        return error_response(
            StatusCode::CONFLICT,
            "not_supported",
            "This server's flavor has no add-ons.",
        );
    };

    if let Some(enabled) = body.enabled {
        let jar_stem = body
            .jar_stem
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid_body("missing_jar_stem", "jarStem is required."));
        let jar_stem = match jar_stem {
            Ok(value) => value,
            Err(response) => return response,
        };
        let current_name = if enabled {
            format!("{jar_stem}.jar.disabled")
        } else {
            format!("{jar_stem}.jar")
        };
        let path = add_on_dir.join(current_name);
        let target = if enabled {
            add_on_dir.join(format!("{jar_stem}.jar"))
        } else {
            add_on_dir.join(format!("{jar_stem}.jar.disabled"))
        };
        if !server.pack_managed && !path.is_file() && target.is_file() {
            return update_result(
                if enabled { "enabled" } else { "disabled" },
                Some(jar_stem.to_string()),
                1,
                None,
            );
        }
        return match addons::toggle(&StdFileSystem, &path, server.pack_managed) {
            Ok(_) => update_result(
                if enabled { "enabled" } else { "disabled" },
                Some(jar_stem.to_string()),
                1,
                None,
            ),
            Err(AddonMutationError::PackManaged) => error_response(
                StatusCode::CONFLICT,
                "conflict",
                "This server is managed by a modpack.",
            ),
            Err(error) => error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &error.to_string(),
            ),
        };
    }

    if let Some(project_id) = body.link_project_id.as_deref() {
        let jar_stem = body
            .jar_stem
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid_body("missing_jar_stem", "jarStem is required."));
        let jar_stem = match jar_stem {
            Ok(value) => value,
            Err(response) => return response,
        };
        let result = state.lifecycle.try_mutate_config(|config| {
            let server_cfg = config
                .servers
                .iter_mut()
                .find(|candidate| candidate.id == server.id)
                .ok_or(())?;
            let links = server_cfg.addon_links.get_or_insert_with(HashMap::new);
            addons::set_manual_addon_link(links, project_id, None, Some(project_id.to_string()));
            if let Some(link) = links.get_mut(project_id) {
                link.installed_file_name = Some(format!("{jar_stem}.jar"));
            }
            Ok::<(), ()>(())
        });
        return match result {
            Ok(()) => update_result("linked", Some(jar_stem.to_string()), 1, None),
            Err(_) => error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Could not persist add-on link.",
            ),
        };
    }

    if body.source_url.is_some() || body.remove_source.unwrap_or(false) {
        let jar_stem = body
            .jar_stem
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid_body("missing_jar_stem", "jarStem is required."));
        let jar_stem = match jar_stem {
            Ok(value) => value,
            Err(response) => return response,
        };
        let source_url = body.source_url.clone();
        let remove_source = body.remove_source.unwrap_or(false);
        let result = state.lifecycle.try_mutate_config(|config| {
            let server_cfg = config
                .servers
                .iter_mut()
                .find(|candidate| candidate.id == server.id)
                .ok_or(())?;
            let sources = server_cfg.plugin_sources.get_or_insert_with(HashMap::new);
            if remove_source {
                if let Some(next) = addons::remove_plugin_source(sources.clone(), jar_stem) {
                    *sources = next;
                } else {
                    sources.clear();
                }
            } else if let Some(url) = source_url.clone() {
                let Some(source_type) = msc_domain::plugin_source::detect(&url) else {
                    return Err(());
                };
                addons::set_plugin_source(
                    sources,
                    jar_stem,
                    PluginSourceConfig {
                        url,
                        source_type: match source_type {
                            msc_domain::plugin_source::PluginSourceType::Github => {
                                msc_domain::app_config_schema::PluginSourceKind::Github
                            }
                            msc_domain::plugin_source::PluginSourceType::Modrinth => {
                                msc_domain::app_config_schema::PluginSourceKind::Modrinth
                            }
                            msc_domain::plugin_source::PluginSourceType::Hangar => {
                                msc_domain::app_config_schema::PluginSourceKind::Hangar
                            }
                            msc_domain::plugin_source::PluginSourceType::Direct => {
                                msc_domain::app_config_schema::PluginSourceKind::Direct
                            }
                        },
                        extra: Default::default(),
                    },
                );
            }
            Ok(())
        });
        return match result {
            Ok(()) => update_result(
                if remove_source {
                    "source_removed"
                } else {
                    "source_set"
                },
                Some(jar_stem.to_string()),
                1,
                None,
            ),
            Err(_) => error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Could not persist plugin source.",
            ),
        };
    }

    let transport = HttpTransport::new();
    let plan = addon_updates::resolve_addon_updates(
        &transport,
        &StdFileSystem,
        &add_on_dir,
        server.java_flavor,
        server.minecraft_version.as_deref(),
        &server.addon_links.clone().unwrap_or_default(),
        &server.plugin_sources.clone().unwrap_or_default(),
    );
    let installed_mod_ids: Vec<String> = plan
        .items
        .iter()
        .filter_map(|item| item.project_id.clone())
        .collect();

    if body.update_all.unwrap_or(false) {
        let items: Vec<_> = plan
            .items
            .iter()
            .filter(|item| item.available_version.is_some())
            .cloned()
            .collect();
        if items.is_empty() {
            return update_result("no_updates_available", None, 0, None);
        }
        let operation_id = match state.lifecycle.operations().begin_lifecycle(
            "addon-update",
            Some(server.id.clone()),
            "Updating add-ons.",
        ) {
            Ok(id) => id,
            Err(error) => return crate::routes::operations::operation_error_response(error),
        };
        let results = addons::update_all(
            &transport,
            &StdFileSystem,
            Path::new(&server.server_dir),
            server.java_flavor,
            &items,
            server.minecraft_version.as_deref(),
            &installed_mod_ids,
            server.pack_managed,
            &|| false,
        );
        let updated = results
            .iter()
            .filter(|result| result.outcome.is_ok())
            .count();
        if let Some(error) = results
            .iter()
            .find_map(|result| result.outcome.as_ref().err())
        {
            let _ = state.lifecycle.finish_operation_failure(
                &operation_id,
                if matches!(error, AddonMutationError::PackManaged) {
                    "conflict"
                } else {
                    "internal_error"
                },
                error.to_string(),
            );
        } else {
            let _ = state.lifecycle.finish_operation_success(
                &operation_id,
                "Updated add-ons.",
                BTreeMap::new(),
            );
        }
        return update_result(
            "update_started",
            None,
            i64::try_from(updated).unwrap_or(0),
            Some(operation_id.as_str().to_string()),
        );
    }

    let jar_stem = body
        .jar_stem
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid_body("missing_jar_stem", "jarStem is required."));
    let jar_stem = match jar_stem {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(item) = plan.items.iter().find(|item| item.jar_stem == jar_stem) else {
        return error_response(StatusCode::NOT_FOUND, "not_found", "Add-on not found.");
    };
    if item.available_version.is_none() {
        return update_result("no_updates_available", Some(jar_stem.to_string()), 0, None);
    }
    let operation_id = match state.lifecycle.operations().begin_lifecycle(
        "addon-update",
        Some(server.id.clone()),
        "Updating add-on.",
    ) {
        Ok(id) => id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let update_outcome = if let Some(source) = server
        .plugin_sources
        .clone()
        .unwrap_or_default()
        .get(jar_stem)
        .cloned()
    {
        let mut sources = server.plugin_sources.clone().unwrap_or_default();
        addons::update_plugin_from_source(
            &transport,
            &StdFileSystem,
            &add_on_dir,
            jar_stem,
            &item.display_name,
            item.is_enabled,
            &source,
            server.minecraft_version.as_deref(),
            &loaders_for(server.java_flavor),
            server.pack_managed,
            &mut sources,
        )
        .map(|_| ())
    } else {
        addons::update_one(
            &transport,
            &StdFileSystem,
            Path::new(&server.server_dir),
            server.java_flavor,
            item,
            server.minecraft_version.as_deref(),
            &installed_mod_ids,
            server.pack_managed,
            &|| false,
        )
        .map(|_| ())
    };
    match update_outcome {
        Ok(()) => {
            let _ = state.lifecycle.finish_operation_success(
                &operation_id,
                "Updated add-on.",
                BTreeMap::new(),
            );
        }
        Err(error) => {
            let _ = state.lifecycle.finish_operation_failure(
                &operation_id,
                if matches!(error, AddonMutationError::PackManaged) {
                    "conflict"
                } else {
                    "internal_error"
                },
                error.to_string(),
            );
        }
    }
    update_result(
        "update_started",
        Some(jar_stem.to_string()),
        1,
        Some(operation_id.as_str().to_string()),
    )
}

pub async fn inspect_modpack(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<ModpackInspectionRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let entry = {
        state
            .staging
            .uploads
            .lock()
            .unwrap()
            .get(&body.staged_upload_id)
            .cloned()
    };
    let Some(entry) = entry else {
        return error_response(StatusCode::NOT_FOUND, "not_found", "Unknown staged upload.");
    };
    if now_unix() > entry.expires_at_unix
        || !matches!(entry.purpose, StagedUploadPurposeDto::ModpackArchive)
    {
        return error_response(StatusCode::NOT_FOUND, "not_found", "Unknown staged upload.");
    }
    let transport = HttpTransport::new();
    let secrets = match production_secret_store() {
        Ok(secrets) => secrets,
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &error.to_string(),
            );
        }
    };
    let operation_id = format!("inspect-{}", Uuid::new_v4());
    let inspection = match modpacks::inspect_staged_archive(
        &StdFileSystem,
        &transport,
        secrets.as_ref(),
        &entry.path,
        &staging_root(&state.lifecycle.servers_root()).join("modpacks"),
        &operation_id,
    ) {
        Ok(inspection) => inspection,
        Err(error) => return invalid_body("invalid_body", &error.to_string()),
    };
    let response = modpack_inspection_response(&inspection);
    let _ = std::fs::remove_dir_all(&inspection.staged_dir);
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/modpacks/inspect",
        StatusCode::OK,
    );
    Json(response).into_response()
}

fn modpack_inspection_response(
    inspection: &modpacks::ModpackInspection,
) -> ModpackInspectionResultDto {
    let (format, pack_name, pack_version, file_count) = match &inspection.format {
        modpacks::InspectedFormat::Mrpack(manifest) => {
            let _meta = msc_domain::modpack_manifest::mrpack_metadata(manifest);
            (
                "mrpack".to_string(),
                Some(manifest.name.clone()),
                Some(manifest.version_id.clone()),
                i64::try_from(manifest.files.len()).unwrap_or(0),
            )
        }
        modpacks::InspectedFormat::CurseForge(metadata) => (
            "curseforge".to_string(),
            Some(metadata.name.clone()),
            Some(metadata.version_id.clone()),
            i64::try_from(metadata.files.len()).unwrap_or(0),
        ),
        modpacks::InspectedFormat::PlainJarZip { jar_entries } => (
            "plain-jar-zip".to_string(),
            None,
            None,
            i64::try_from(jar_entries.len()).unwrap_or(0),
        ),
    };
    let pinned = inspection.pinned_version.clone();
    ModpackInspectionResultDto {
        success: true,
        message: "Modpack inspected.".to_string(),
        format,
        pack_name,
        pack_version,
        minecraft_version: pinned.as_ref().map(|entry| entry.mc_version.clone()),
        loader_name: pinned
            .as_ref()
            .and_then(|entry| entry.build_label.as_ref())
            .and_then(|label| label.split_whitespace().next().map(str::to_string)),
        loader_version: pinned.and_then(|entry| entry.loader_version),
        file_count,
        client_only_file_count: 0,
        manual_files: inspection
            .manual_downloads
            .iter()
            .map(|file| ModpackManualFileDto {
                file_id: file.file_name.clone(),
                file_name: file.file_name.clone(),
                project_name: file.mod_name.clone(),
            })
            .collect(),
        warnings: if inspection.manual_downloads.is_empty() {
            Vec::new()
        } else {
            vec![format!(
                "{} file(s) require manual completion before import can finish.",
                inspection.manual_downloads.len()
            )]
        },
    }
}

pub async fn import_modpack(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<ModpackImportRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let Some(server) = state.lifecycle.active_config_server() else {
        return no_active_server();
    };
    let explicit_replace = match body.action.as_str() {
        "import" => false,
        "replace" => true,
        _ => return invalid_body("invalid_action", "action must be import or replace."),
    };
    let entry = state
        .staging
        .uploads
        .lock()
        .unwrap()
        .get(&body.staged_upload_id)
        .cloned();
    let Some(entry) = entry else {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged upload.",
        );
    };
    if now_unix() > entry.expires_at_unix
        || !matches!(entry.purpose, StagedUploadPurposeDto::ModpackArchive)
    {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged upload.",
        );
    }
    let transport = HttpTransport::new();
    let secrets = match production_secret_store() {
        Ok(secrets) => secrets,
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &error.to_string(),
            );
        }
    };
    let inspection_id = format!("inspect-{}", Uuid::new_v4());
    let inspection = match modpacks::inspect_staged_archive(
        &StdFileSystem,
        &transport,
        secrets.as_ref(),
        &entry.path,
        &staging_root(&state.lifecycle.servers_root()).join("modpacks"),
        &inspection_id,
    ) {
        Ok(inspection) => inspection,
        Err(error) => return invalid_body("invalid_body", &error.to_string()),
    };

    if matches!(inspection.format, modpacks::InspectedFormat::CurseForge(_)) {
        let configured = match secrets.get(provider::CURSEFORGE_API_KEY_SECRET) {
            Ok(value) => value.is_some_and(|value| !value.trim().is_empty()),
            Err(error) => {
                let _ = std::fs::remove_dir_all(&inspection.staged_dir);
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    &error.to_string(),
                );
            }
        };
        if !configured {
            let _ = std::fs::remove_dir_all(&inspection.staged_dir);
            return error_response(
                StatusCode::BAD_REQUEST,
                "missing_curseforge_api_key",
                "This CurseForge modpack needs an API key. Save one in MSC Settings, then retry the import.",
            );
        }
    }

    if state
        .staging
        .uploads
        .lock()
        .unwrap()
        .remove(&body.staged_upload_id)
        .is_none()
    {
        let _ = std::fs::remove_dir_all(&inspection.staged_dir);
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged upload.",
        );
    }
    let operation_id = match state.lifecycle.operations().begin_lifecycle(
        "modpack-import",
        Some(server.id.clone()),
        "Importing modpack.",
    ) {
        Ok(id) => id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };

    let result = match &inspection.format {
        modpacks::InspectedFormat::Mrpack(manifest) => modpacks::import_mrpack(
            &transport,
            &StdFileSystem,
            Path::new(&server.server_dir),
            server.java_flavor,
            manifest,
            &inspection.staged_dir,
            &state.lifecycle.servers_root(),
            server.pack_managed,
            explicit_replace,
            &|| false,
        )
        .map_err(|error| error.to_string())
        .and_then(|report| {
            set_pack_metadata(
                &state.lifecycle,
                &server.id,
                &report.pack_name,
                &report.pack_version,
            )?;
            Ok(report)
        })
        .map(|report| {
            (
                Vec::new(),
                report.pack_name,
                report.pack_version,
                report.cancelled,
            )
        }),
        modpacks::InspectedFormat::CurseForge(metadata) => modpacks::import_curseforge(
            &transport,
            secrets.as_ref(),
            &StdFileSystem,
            Path::new(&server.server_dir),
            server.java_flavor,
            metadata,
            &inspection.staged_dir,
            server.pack_managed,
            explicit_replace,
            &|| false,
        )
        .map_err(|error| error.to_string())
        .and_then(|report| {
            set_pack_metadata(
                &state.lifecycle,
                &server.id,
                &report.pack_name,
                &report.pack_version,
            )?;
            Ok((
                report.blocked_files,
                report.pack_name,
                report.pack_version,
                report.cancelled,
            ))
        }),
        modpacks::InspectedFormat::PlainJarZip { .. } => {
            Err("Plain JAR ZIP import is not supported here.".to_string())
        }
    };
    let _ = std::fs::remove_file(&entry.path);

    let response = match result {
        Ok((blocked_files, pack_name, pack_version, cancelled)) => {
            if cancelled {
                let _ = state
                    .lifecycle
                    .operations()
                    .cancel(&operation_id, "Modpack import cancelled.");
            } else if blocked_files.is_empty() {
                let mut result_map = BTreeMap::new();
                result_map.insert("packName".to_string(), pack_name);
                result_map.insert("packVersion".to_string(), pack_version);
                let _ = state.lifecycle.finish_operation_success(
                    &operation_id,
                    "Imported modpack.",
                    result_map,
                );
            } else {
                state.pending_modpack_imports.lock().unwrap().insert(
                    operation_id.as_str().to_string(),
                    PendingModpackImport {
                        remaining_manual_files: blocked_files.clone(),
                    },
                );
            }
            (
                StatusCode::ACCEPTED,
                Json(ModpackImportResultDto {
                    success: true,
                    message: if blocked_files.is_empty() {
                        "Import started.".to_string()
                    } else {
                        format!(
                            "Import paused: {} file(s) need manual completion.",
                            blocked_files.len()
                        )
                    },
                    operation_id: operation_id.as_str().to_string(),
                    pending_manual_files: blocked_files
                        .iter()
                        .map(|file| ModpackManualFileDto {
                            file_id: file.file_id.to_string(),
                            file_name: file.expected_file_name.clone(),
                            project_name: file.expected_file_name.clone(),
                        })
                        .collect(),
                }),
            )
                .into_response()
        }
        Err(message) => {
            let _ =
                state
                    .lifecycle
                    .finish_operation_failure(&operation_id, "internal_error", message);
            (
                StatusCode::ACCEPTED,
                Json(ModpackImportResultDto {
                    success: true,
                    message: "Import started.".to_string(),
                    operation_id: operation_id.as_str().to_string(),
                    pending_manual_files: Vec::new(),
                }),
            )
                .into_response()
        }
    };
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/modpacks/import",
        response.status(),
    );
    response
}

pub async fn complete_modpack_manual_file(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(operation_id): AxumPath<String>,
    body: Result<Json<ModpackManualFileRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let entry = state
        .staging
        .uploads
        .lock()
        .unwrap()
        .remove(&body.staged_upload_id);
    let Some(entry) = entry else {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged upload.",
        );
    };
    if now_unix() > entry.expires_at_unix
        || !matches!(entry.purpose, StagedUploadPurposeDto::CurseforgeManualFile)
    {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged upload.",
        );
    }
    let mut pending = state.pending_modpack_imports.lock().unwrap();
    let Some(import) = pending.get_mut(&operation_id) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "No pending modpack import matched that operation.",
        );
    };
    let Some(index) = import
        .remaining_manual_files
        .iter()
        .position(|file| file.file_id.to_string() == body.file_id)
    else {
        return error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "No pending manual file matched that fileId.",
        );
    };
    let pending_file = import.remaining_manual_files[index].clone();
    let staged_filename = entry
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("uploaded.jar")
        .to_string();
    let completion = curseforge_manual::complete_pending_file(
        &StdFileSystem,
        &entry.path,
        &staged_filename,
        &pending_file,
        import.remaining_manual_files.len() == 1,
    );
    let _ = std::fs::remove_file(&entry.path);
    match completion {
        Ok(_) => {
            import.remaining_manual_files.remove(index);
            let all_files_resolved = import.remaining_manual_files.is_empty();
            let remaining = import
                .remaining_manual_files
                .iter()
                .map(|file| ModpackManualFileDto {
                    file_id: file.file_id.to_string(),
                    file_name: file.expected_file_name.clone(),
                    project_name: file.expected_file_name.clone(),
                })
                .collect::<Vec<_>>();
            if all_files_resolved {
                pending.remove(&operation_id);
                let _ = state.lifecycle.finish_operation_success(
                    &msc_domain::operation::OperationId::new(operation_id.clone()),
                    "Imported modpack.",
                    BTreeMap::new(),
                );
            }
            let response = Json(ModpackManualFileResultDto {
                success: true,
                message: if all_files_resolved {
                    "File accepted; import resumed.".to_string()
                } else {
                    "File accepted; waiting for the remaining manual files.".to_string()
                },
                operation_id: operation_id.clone(),
                remaining_manual_files: remaining,
                all_files_resolved,
            })
            .into_response();
            audit(
                &state.lifecycle,
                &credential,
                "POST",
                "/v1/modpacks/:operation_id/manual-file",
                response.status(),
            );
            response
        }
        Err(error) => invalid_body("invalid_body", &error.to_string()),
    }
}
