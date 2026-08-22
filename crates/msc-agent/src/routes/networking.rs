//! Phase 9's player-facing helper and resource-pack routes.
//!
//! The route layer owns the long-lived service instances because Playit and
//! MCXboxBroadcast have state that spans requests (running process, readiness,
//! and the current operation).  They share the same operation journal and
//! process supervisor as the rest of the agent; a second in-memory operation
//! map here would make polling and cancellation lie.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::post};
use msc_api::dto::{
    BroadcastAuthPromptDto, BroadcastAutoStartDto, BroadcastCredentialsDto,
    BroadcastJarDownloadResultDto, BroadcastJarStatusDto, BroadcastSimpleResultDto,
    BroadcastStatusDto, PermissionCategoryDto, PlayitActionResultDto, PlayitStatusDto,
    ResourcePackActivateRequestDto, ResourcePackItemDto, ResourcePackMutationResultDto,
    ResourcePackRemoveRequestDto, ResourcePackSetUrlRequestDto, ResourcePackToggleRequestDto,
    ResourcePacksResponseDto,
};
use msc_application::operations::LifecycleOperations;
use msc_application::playit::{PlayitError, PlayitService};
use msc_application::resource_packs::ResourcePackService;
use msc_application::xbox_broadcast::{XboxBroadcastError, XboxBroadcastService};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::helper::HelperStatus;
use msc_domain::identity::ServerType;
use msc_infrastructure::fs::{FileSystem, StdFileSystem};
use msc_infrastructure::helper_acquisition::HelperAcquisitionError;
use msc_infrastructure::jar_provider::HttpTransport;
use msc_infrastructure::process::ProcessSupervisor;
use msc_infrastructure::secret_store::SecretStore;
use msc_infrastructure::{config_repository, xbox_broadcast};

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{
    LifecycleRoutesState, error_response, invalid_body, require_permission,
};
use crate::routes::operations::OperationsState;

type SharedPlayitService = PlayitService<'static>;
type SharedBroadcastService = XboxBroadcastService<'static>;

#[derive(Clone)]
pub struct NetworkingState {
    pub(crate) lifecycle: LifecycleRoutesState,
    operations: OperationsState,
    playit: Arc<Mutex<BTreeMap<String, SharedPlayitService>>>,
    broadcast: Arc<Mutex<BTreeMap<String, SharedBroadcastService>>>,
    process: &'static (dyn ProcessSupervisor + Send + Sync),
    secrets: &'static (dyn SecretStore + Send + Sync),
    operations_ref: &'static LifecycleOperations<'static>,
    transport: &'static HttpTransport,
    fs: &'static dyn FileSystem,
    helper_cache: &'static Path,
}

impl NetworkingState {
    pub fn new(
        lifecycle: LifecycleRoutesState,
        operations: OperationsState,
        secret_store: Arc<dyn SecretStore + Send + Sync>,
    ) -> Self {
        let process = lifecycle.process_supervisor();
        let leaked_store: &'static Arc<dyn SecretStore + Send + Sync> =
            Box::leak(Box::new(secret_store));
        let secrets: &'static (dyn SecretStore + Send + Sync) = leaked_store.as_ref();
        let operations_ref = operations.application_operations();
        let transport = Box::leak(Box::new(HttpTransport::new()));
        let fs: &'static dyn FileSystem = Box::leak(Box::new(StdFileSystem));
        let helper_cache: &'static Path = Box::leak(Box::new(
            config_repository::default_app_data_dir().join("helpers"),
        ));
        let _ = std::fs::create_dir_all(helper_cache);
        Self {
            lifecycle,
            operations,
            playit: Arc::new(Mutex::new(BTreeMap::new())),
            broadcast: Arc::new(Mutex::new(BTreeMap::new())),
            process,
            secrets,
            operations_ref,
            transport,
            fs,
            helper_cache,
        }
    }

    #[allow(clippy::result_large_err)]
    fn active_server(&self) -> Result<ConfigServer, Response> {
        self.lifecycle.active_config_server().ok_or_else(|| {
            error_response(
                StatusCode::CONFLICT,
                "no_active_server",
                "No active server is selected.",
            )
        })
    }

    fn playit_service(
        &self,
        server: &ConfigServer,
    ) -> std::sync::MutexGuard<'_, BTreeMap<String, SharedPlayitService>> {
        let mut services = self.playit.lock().expect("Playit service lock poisoned");
        services.entry(server.id.clone()).or_insert_with(|| {
            PlayitService::new(
                server.id.clone(),
                server.playit_enabled,
                self.process,
                self.secrets,
                self.operations_ref,
            )
        });
        services
    }

    fn broadcast_service(
        &self,
        server: &ConfigServer,
    ) -> std::sync::MutexGuard<'_, BTreeMap<String, SharedBroadcastService>> {
        let mut services = self
            .broadcast
            .lock()
            .expect("Broadcast service lock poisoned");
        services.entry(server.id.clone()).or_insert_with(|| {
            XboxBroadcastService::new(
                server.id.clone(),
                server.xbox_broadcast_enabled,
                self.process,
                self.secrets,
                self.operations_ref,
            )
        });
        services
    }

    #[allow(clippy::result_large_err)]
    fn playit_acquisition(
        &self,
    ) -> Result<msc_infrastructure::playit::PlayitBinaryAcquisition<'static>, Response> {
        msc_infrastructure::playit::PlayitBinaryAcquisition::for_current_platform(
            self.transport,
            self.fs,
            self.helper_cache,
        )
        .map_err(|error| acquisition_error_response("playit", error))
    }

    #[allow(clippy::result_large_err)]
    fn broadcast_acquisition(
        &self,
    ) -> Result<xbox_broadcast::XboxBroadcastJarAcquisition<'static>, Response> {
        xbox_broadcast::XboxBroadcastJarAcquisition::for_current_platform(
            self.transport,
            self.fs,
            self.helper_cache,
        )
        .map_err(|error| acquisition_error_response("Xbox Broadcast", error))
    }
}

pub fn router(state: NetworkingState) -> Router {
    Router::new()
        .route("/playit", axum::routing::get(playit_status))
        .route("/playit/start", post(playit_start))
        .route("/playit/stop", post(playit_stop))
        .route("/broadcast/status", axum::routing::get(broadcast_status))
        .route(
            "/broadcast/autostart",
            axum::routing::get(broadcast_autostart).post(set_broadcast_autostart),
        )
        .route(
            "/broadcast/auth-prompt",
            axum::routing::get(broadcast_auth_prompt),
        )
        .route(
            "/broadcast/auth-prompt/dismiss",
            post(dismiss_broadcast_prompt),
        )
        .route("/broadcast/credentials", post(set_broadcast_credentials))
        .route(
            "/broadcast/jar-status",
            axum::routing::get(broadcast_jar_status),
        )
        .route("/broadcast/download-jar", post(download_broadcast_jar))
        .route("/broadcast/start", post(start_broadcast))
        .route("/broadcast/stop", post(stop_broadcast))
        .route("/broadcast/restart", post(restart_broadcast))
        .route("/resourcepacks", axum::routing::get(resource_packs))
        .route("/resourcepacks/activate", post(activate_resource_pack))
        .route("/resourcepacks/seturl", post(set_resource_pack_url))
        .route("/resourcepacks/toggle", post(toggle_resource_pack))
        .route("/resourcepacks/remove", post(remove_resource_pack))
        .with_state(state)
}

pub async fn playit_status(State(state): State<NetworkingState>) -> Response {
    let Ok(server) = state.active_server() else {
        return Json(PlayitStatusDto {
            server_name: String::new(),
            server_type: "java".into(),
            playit_enabled: false,
            is_running: false,
            has_secret_key: false,
            java_address: None,
            bedrock_address: None,
            voice_address: None,
            voice_chat_enabled: false,
            note: Some("no_active_server".into()),
        })
        .into_response();
    };
    let mut services = state.playit_service(&server);
    let service = services.get_mut(&server.id).expect("service was inserted");
    let has_secret = service.has_secret().unwrap_or(false);
    let snapshot = service.status().clone();
    Json(PlayitStatusDto {
        server_name: server.display_name,
        server_type: server.server_type.raw_value().into(),
        playit_enabled: server.playit_enabled,
        is_running: matches!(
            snapshot.status,
            HelperStatus::Running | HelperStatus::Starting
        ),
        has_secret_key: has_secret,
        java_address: state.lifecycle.app_config_snapshot().playit_java_address,
        bedrock_address: state.lifecycle.app_config_snapshot().playit_bedrock_address,
        voice_address: state.lifecycle.app_config_snapshot().playit_voice_address,
        voice_chat_enabled: server.playit_voice_chat_enabled,
        note: None,
    })
    .into_response()
}

pub async fn playit_start(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Networking) {
        return response;
    }
    let server = match state.active_server() {
        Ok(server) => server,
        Err(response) => return response,
    };
    let acquisition = match state.playit_acquisition() {
        Ok(acquisition) => acquisition,
        Err(response) => return response,
    };
    let mut services = state.playit_service(&server);
    let service = services.get_mut(&server.id).expect("service was inserted");
    if matches!(
        service.status().status,
        HelperStatus::Running | HelperStatus::Starting
    ) {
        return Json(PlayitActionResultDto {
            result: "already_running".into(),
            message: None,
            operation_id: None,
        })
        .into_response();
    }
    let working_directory = PathBuf::from(&server.server_dir).join(".msc2-playit");
    let _ = std::fs::create_dir_all(&working_directory);
    let launch = msc_infrastructure::playit::PlayitLaunch {
        working_directory: working_directory.clone(),
        secret_path: working_directory.join("secret-bridge"),
    };
    match service.start(launch, &acquisition) {
        Ok(result) => (
            StatusCode::ACCEPTED,
            Json(PlayitActionResultDto {
                result: "started".into(),
                message: Some("Playit tunnel start accepted.".into()),
                operation_id: result.operation_id,
            }),
        )
            .into_response(),
        Err(error) => helper_error_response(error.to_string(), "playit_start_failed"),
    }
}

pub async fn playit_stop(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Networking) {
        return response;
    }
    let server = match state.active_server() {
        Ok(server) => server,
        Err(response) => return response,
    };
    let mut services = state.playit_service(&server);
    let service = services.get_mut(&server.id).expect("service was inserted");
    if matches!(
        service.status().status,
        HelperStatus::Stopped | HelperStatus::TimedOut
    ) {
        return Json(PlayitActionResultDto {
            result: "not_running".into(),
            message: None,
            operation_id: None,
        })
        .into_response();
    }
    match service.stop() {
        Ok(()) => Json(PlayitActionResultDto {
            result: "stopped".into(),
            message: Some("Playit tunnel stop requested.".into()),
            operation_id: None,
        })
        .into_response(),
        Err(error) => helper_error_response(error.to_string(), "playit_stop_failed"),
    }
}

pub async fn broadcast_status(State(state): State<NetworkingState>) -> Response {
    let Ok(server) = state.active_server() else {
        return Json(BroadcastStatusDto {
            xbox_broadcast_running: false,
            bedrock_broadcast_running: false,
        })
        .into_response();
    };
    let mut services = state.broadcast_service(&server);
    let service = services.get_mut(&server.id).expect("service was inserted");
    match service.status() {
        Ok(status) => Json(BroadcastStatusDto {
            xbox_broadcast_running: matches!(
                status.snapshot.status,
                HelperStatus::Running | HelperStatus::Starting
            ),
            bedrock_broadcast_running: false,
        })
        .into_response(),
        Err(error) => helper_error_response(error.to_string(), "broadcast_status_failed"),
    }
}

pub async fn broadcast_autostart(
    State(state): State<NetworkingState>,
) -> Json<BroadcastAutoStartDto> {
    Json(BroadcastAutoStartDto {
        enabled: state
            .lifecycle
            .app_config_snapshot()
            .xbox_broadcast_auto_start_enabled,
    })
}

pub async fn set_broadcast_autostart(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<BroadcastAutoStartDto>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Broadcast) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    match state.lifecycle.try_mutate_config(|config| {
        config.xbox_broadcast_auto_start_enabled = body.enabled;
        Ok::<_, std::convert::Infallible>(())
    }) {
        Ok(()) => Json(BroadcastAutoStartDto {
            enabled: body.enabled,
        })
        .into_response(),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Could not save the Xbox Broadcast autostart setting.",
        ),
    }
}

pub async fn broadcast_auth_prompt(State(state): State<NetworkingState>) -> Response {
    let Ok(server) = state.active_server() else {
        return Json(BroadcastAuthPromptDto {
            is_present: false,
            code: None,
            link_url: None,
        })
        .into_response();
    };
    let mut services = state.broadcast_service(&server);
    let service = services.get_mut(&server.id).expect("service was inserted");
    match service.status() {
        Ok(status) => Json(BroadcastAuthPromptDto {
            is_present: status.auth_prompt.is_some(),
            code: status
                .auth_prompt
                .as_ref()
                .map(|prompt| prompt.code.clone()),
            link_url: status
                .auth_prompt
                .as_ref()
                .map(|prompt| prompt.link_url.clone()),
        })
        .into_response(),
        Err(error) => helper_error_response(error.to_string(), "broadcast_status_failed"),
    }
}

pub async fn dismiss_broadcast_prompt(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Broadcast) {
        return response;
    }
    let server = match state.active_server() {
        Ok(server) => server,
        Err(response) => return response,
    };
    let mut services = state.broadcast_service(&server);
    services
        .get_mut(&server.id)
        .expect("service was inserted")
        .dismiss_auth_prompt();
    Json(BroadcastSimpleResultDto {
        result: "dismissed".into(),
        operation_id: None,
    })
    .into_response()
}

pub async fn set_broadcast_credentials(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<BroadcastCredentialsDto>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Broadcast) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let server = match state.active_server() {
        Ok(server) => server,
        Err(response) => return response,
    };
    let mut services = state.broadcast_service(&server);
    let service = services.get_mut(&server.id).expect("service was inserted");
    if let Err(error) = service.save_password(&body.password) {
        return helper_error_response(error.to_string(), "credential_store_failed");
    }
    let user_update = state.lifecycle.try_mutate_config(|config| {
        let entry = config
            .servers
            .iter_mut()
            .find(|item| item.id == server.id)
            .ok_or("server disappeared")?;
        entry.xbox_broadcast_alt_email = Some(body.email.trim().to_string());
        entry.xbox_broadcast_alt_gamertag = Some(body.gamertag.trim().to_string());
        Ok::<_, &str>(())
    });
    match user_update {
        Ok(()) => Json(BroadcastSimpleResultDto {
            result: "credentials_saved".into(),
            operation_id: None,
        })
        .into_response(),
        _ => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Could not save broadcast credentials.",
        ),
    }
}

pub async fn broadcast_jar_status(
    State(state): State<NetworkingState>,
) -> Json<BroadcastJarStatusDto> {
    let directory = state.helper_cache.join("xbox-broadcast");
    let filename = std::fs::read_dir(directory)
        .ok()
        .into_iter()
        .flatten()
        .find_map(|version| {
            let version = version.ok()?.path();
            std::fs::read_dir(version)
                .ok()?
                .flatten()
                .find_map(|entry| {
                    let path = entry.path();
                    if path.extension().and_then(|ext| ext.to_str()) == Some("jar") {
                        Some(path.file_name()?.to_string_lossy().into_owned())
                    } else {
                        None
                    }
                })
        });
    Json(BroadcastJarStatusDto {
        installed: filename.is_some(),
        downloading: false,
        filename,
    })
}

pub async fn download_broadcast_jar(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Broadcast) {
        return response;
    }
    let server = match state.active_server() {
        Ok(server) => server,
        Err(response) => return response,
    };
    let operation = match state.operations.begin_lifecycle(
        "broadcast-jar-download",
        Some(server.id.clone()),
        "Downloading Xbox Broadcast JAR.",
    ) {
        Ok(id) => id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let acquisition = match state.broadcast_acquisition() {
        Ok(acquisition) => acquisition,
        Err(response) => return response,
    };
    match acquisition.acquire() {
        Ok(acquired) => {
            let filename = acquired
                .artifact
                .path
                .file_name()
                .map(|value| value.to_string_lossy().into_owned());
            let _ = state.operations.succeed(
                &operation,
                "Xbox Broadcast JAR is ready.",
                BTreeMap::new(),
            );
            (
                StatusCode::ACCEPTED,
                Json(BroadcastJarDownloadResultDto {
                    success: true,
                    message: "downloaded".into(),
                    filename,
                    operation_id: Some(operation.as_str().into()),
                }),
            )
                .into_response()
        }
        Err(error) => {
            let _ =
                state
                    .operations
                    .fail(&operation, "broadcast_download_failed", error.to_string());
            error_response(
                StatusCode::CONFLICT,
                "broadcast_download_failed",
                &error.to_string(),
            )
        }
    }
}

pub async fn start_broadcast(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Broadcast) {
        return response;
    }
    let server = match state.active_server() {
        Ok(server) => server,
        Err(response) => return response,
    };
    let acquisition = match state.broadcast_acquisition() {
        Ok(acquisition) => acquisition,
        Err(response) => return response,
    };
    let mut services = state.broadcast_service(&server);
    let service = services.get_mut(&server.id).expect("service was inserted");
    let workdir = PathBuf::from(&server.server_dir).join(".msc2-broadcast");
    let _ = std::fs::create_dir_all(&workdir);
    let launch = xbox_broadcast::XboxBroadcastLaunch {
        java_path: PathBuf::from(state.lifecycle.app_config_snapshot().java_path),
        working_directory: workdir,
    };
    match service.start(launch, &acquisition) {
        Ok(operation_id) => (
            StatusCode::ACCEPTED,
            Json(BroadcastSimpleResultDto {
                result: "broadcast_start_requested".into(),
                operation_id: Some(operation_id),
            }),
        )
            .into_response(),
        Err(error) => helper_error_response(error.to_string(), "broadcast_start_failed"),
    }
}

pub async fn stop_broadcast(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Broadcast) {
        return response;
    }
    let server = match state.active_server() {
        Ok(server) => server,
        Err(response) => return response,
    };
    let mut services = state.broadcast_service(&server);
    let service = services.get_mut(&server.id).expect("service was inserted");
    match service.stop() {
        Ok(()) => Json(BroadcastSimpleResultDto {
            result: "broadcast_stop_requested".into(),
            operation_id: None,
        })
        .into_response(),
        Err(XboxBroadcastError::Process(message)) if message.contains("not managed") => {
            Json(BroadcastSimpleResultDto {
                result: "not_running".into(),
                operation_id: None,
            })
            .into_response()
        }
        Err(error) => helper_error_response(error.to_string(), "broadcast_stop_failed"),
    }
}

pub async fn restart_broadcast(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Broadcast) {
        return response;
    }
    let server = match state.active_server() {
        Ok(server) => server,
        Err(response) => return response,
    };
    let acquisition = match state.broadcast_acquisition() {
        Ok(acquisition) => acquisition,
        Err(response) => return response,
    };
    let mut services = state.broadcast_service(&server);
    let service = services.get_mut(&server.id).expect("service was inserted");
    let _ = service.stop();
    let workdir = PathBuf::from(&server.server_dir).join(".msc2-broadcast");
    let _ = std::fs::create_dir_all(&workdir);
    let launch = xbox_broadcast::XboxBroadcastLaunch {
        java_path: PathBuf::from(state.lifecycle.app_config_snapshot().java_path),
        working_directory: workdir,
    };
    match service.start(launch, &acquisition) {
        Ok(operation_id) => (
            StatusCode::ACCEPTED,
            Json(BroadcastSimpleResultDto {
                result: "broadcast_restart_requested".into(),
                operation_id: Some(operation_id),
            }),
        )
            .into_response(),
        Err(error) => helper_error_response(error.to_string(), "broadcast_restart_failed"),
    }
}

pub async fn resource_packs(
    State(state): State<NetworkingState>,
) -> Json<ResourcePacksResponseDto> {
    Json(resource_pack_response(
        &state,
        state.lifecycle.active_config_server().as_ref(),
    ))
}

pub async fn activate_resource_pack(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<ResourcePackActivateRequestDto>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let server = match state.active_server() {
        Ok(server) => server,
        Err(response) => return response,
    };
    if server.server_type != ServerType::Java {
        return error_response(
            StatusCode::CONFLICT,
            "java_only",
            "Resource packs are available only for Java servers.",
        );
    }
    let service = ResourcePackService::new(&server.server_dir, state.fs);
    let result = match body.pack_id {
        None => service.disable().map(|_| "cleared".to_string()),
        Some(pack_id) => {
            let bytes = match service.approved_bytes(&pack_id) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return error_response(StatusCode::NOT_FOUND, "not_found", &error.to_string());
                }
            };
            let host = state
                .lifecycle
                .app_config_snapshot()
                .duckdns_hostname
                .unwrap_or_else(|| "127.0.0.1".into());
            let port = u16::try_from(server.resource_pack_host_port).unwrap_or(8123);
            service
                .publish_and_activate(&pack_id, &bytes, &host, port, body.require.unwrap_or(false))
                .map(|_| "activated".to_string())
        }
    };
    match result {
        Ok(message) => Json(ResourcePackMutationResultDto {
            success: true,
            message,
            updated: Some(resource_pack_response(&state, Some(&server))),
        })
        .into_response(),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        ),
    }
}

pub async fn set_resource_pack_url(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<ResourcePackSetUrlRequestDto>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let server = match state.active_server() {
        Ok(server) => server,
        Err(response) => return response,
    };
    let service = ResourcePackService::new(&server.server_dir, state.fs);
    match service.set_external_url(
        &body.url,
        body.sha1.as_deref(),
        body.require.unwrap_or(false),
    ) {
        Ok(()) => Json(ResourcePackMutationResultDto {
            success: true,
            message: "url_saved".into(),
            updated: Some(resource_pack_response(&state, Some(&server))),
        })
        .into_response(),
        Err(error) => error_response(StatusCode::BAD_REQUEST, "invalid_url", &error.to_string()),
    }
}

pub async fn toggle_resource_pack(
    State(_state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<ResourcePackToggleRequestDto>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let _ = body;
    error_response(
        StatusCode::CONFLICT,
        "not_supported",
        "Geyser resource-pack toggling is not available until the Bedrock client-pack store is implemented.",
    )
}

pub async fn remove_resource_pack(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<ResourcePackRemoveRequestDto>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Addons) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    if body.pack_kind != "java" {
        return error_response(
            StatusCode::CONFLICT,
            "not_supported",
            "Only Java resource packs are managed by this route.",
        );
    }
    let server = match state.active_server() {
        Ok(server) => server,
        Err(response) => return response,
    };
    let service = ResourcePackService::new(&server.server_dir, state.fs);
    match service.remove(&body.pack_id) {
        Ok(()) => Json(ResourcePackMutationResultDto {
            success: true,
            message: "removed".into(),
            updated: Some(resource_pack_response(&state, Some(&server))),
        })
        .into_response(),
        Err(error) => error_response(StatusCode::NOT_FOUND, "not_found", &error.to_string()),
    }
}

fn resource_pack_response(
    state: &NetworkingState,
    server: Option<&ConfigServer>,
) -> ResourcePacksResponseDto {
    let Some(server) = server else {
        return ResourcePacksResponseDto {
            server_type: "java".into(),
            is_java: true,
            packs: Vec::new(),
            geyser_packs: Vec::new(),
            is_geyser_available: false,
            active_pack_url: None,
            require_pack: false,
            note: Some("no_active_server".into()),
        };
    };
    let is_java = server.server_type == ServerType::Java;
    let properties = read_properties(Path::new(&server.server_dir).join("server.properties"));
    let active_url = properties
        .get("resource-pack")
        .filter(|url| !url.is_empty())
        .cloned();
    let require = properties
        .get("require-resource-pack")
        .is_some_and(|value| value == "true");
    let packs = if is_java {
        std::fs::read_dir(Path::new(&server.server_dir).join("resource-packs"))
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                let name = path.file_name()?.to_string_lossy().into_owned();
                if path.extension().and_then(|ext| ext.to_str()) != Some("zip") {
                    return None;
                }
                let size = std::fs::metadata(&path).ok()?.len();
                Some(ResourcePackItemDto {
                    id: name.clone(),
                    name: name.clone(),
                    file_name: name,
                    file_size_display: format!("{size} bytes"),
                    pack_kind: "java".into(),
                    is_active: active_url.as_deref().is_some_and(|url| {
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| url.contains(name))
                    }),
                    type_label: "Java resource pack".into(),
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    let geyser_available = is_java
        && msc_application::geyser::installation(state.fs, Path::new(&server.server_dir))
            .geyser_installed;
    ResourcePacksResponseDto {
        server_type: server.server_type.raw_value().into(),
        is_java,
        packs,
        geyser_packs: Vec::new(),
        is_geyser_available: geyser_available,
        active_pack_url: active_url,
        require_pack: require,
        note: None,
    }
}

fn read_properties(path: PathBuf) -> BTreeMap<String, String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn helper_error_response(message: String, code: &str) -> Response {
    error_response(StatusCode::CONFLICT, code, &message)
}

fn acquisition_error_response(helper: &str, error: HelperAcquisitionError) -> Response {
    error_response(
        StatusCode::CONFLICT,
        "helper_unavailable",
        &format!("{helper} is unavailable: {error}"),
    )
}

#[allow(dead_code)]
fn map_playit_error(error: PlayitError) -> Response {
    helper_error_response(error.to_string(), "playit_failed")
}
