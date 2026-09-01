//! Phase 9's player-facing helper and resource-pack routes.
//!
//! The route layer owns the long-lived service instances because Playit and
//! MCXboxBroadcast have state that spans requests (running process, readiness,
//! and the current operation).  They share the same operation journal and
//! process supervisor as the rest of the agent; a second in-memory operation
//! map here would make polling and cancellation lie.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router, routing::post};
use msc_api::dto::{
    BroadcastAuthPromptDto, BroadcastAutoStartDto, BroadcastCredentialsDto,
    BroadcastCredentialsStatusDto, BroadcastJarDownloadResultDto, BroadcastJarStatusDto,
    BroadcastSimpleResultDto, BroadcastStatusDto, PermissionCategoryDto, PlayitActionResultDto,
    PlayitResetResultDto, PlayitSetupAcceptedDto, PlayitSetupRequestDto, PlayitStatusDto,
    ResourcePackActivateRequestDto, ResourcePackItemDto, ResourcePackMutationResultDto,
    ResourcePackRemoveRequestDto, ResourcePackSetUrlRequestDto, ResourcePackToggleRequestDto,
    ResourcePacksResponseDto,
};
use msc_application::operations::LifecycleOperations;
use msc_application::playit::{
    PLAYIT_SETUP_OPERATION_TARGET, PLAYIT_SETUP_OPERATION_TYPE, PlayitAccountSetup, PlayitError,
    PlayitLifecycleStatus, PlayitService, PlayitSetupError, PlayitSetupStage,
};
use msc_application::resource_packs::ResourcePackService;
use msc_application::xbox_broadcast::{
    BroadcastOutputLine, XboxBroadcastError, XboxBroadcastService,
};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::helper::{FirstRunTransport, FirstStartTransportState, HelperStatus};
use msc_domain::identity::ServerType;
use msc_domain::networking::{PlayitTunnelSpec, patch_voice_chat_properties, playit_tunnel_specs};
use msc_infrastructure::addon_provider::HttpTransport as ProviderHttpTransport;
use msc_infrastructure::fs::{FileSystem, StdFileSystem};
use msc_infrastructure::helper_acquisition::HelperAcquisitionError;
use msc_infrastructure::jar_provider::HttpTransport;
use msc_infrastructure::playit::{PLAYIT_SECRET_KEY, PlayitSecretBridge};
use msc_infrastructure::playit_api::PlayitHttpTransport;
use msc_infrastructure::process::ProcessSupervisor;
use msc_infrastructure::secret_store::SecretStore;
use msc_infrastructure::{config_repository, xbox_broadcast};

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{
    LifecycleRoutesState, PlayitLifecycleIntegration, error_response, invalid_body,
    require_permission,
};
use crate::routes::operations::OperationsState;

type SharedPlayitService = PlayitService<'static>;
type SharedBroadcastService = XboxBroadcastService<'static>;

struct PlayitLifecycleController {
    services: Arc<Mutex<BTreeMap<String, SharedPlayitService>>>,
    broadcast: Arc<Mutex<BTreeMap<String, SharedBroadcastService>>>,
    process: &'static (dyn ProcessSupervisor + Send + Sync),
    secrets: &'static (dyn SecretStore + Send + Sync),
    operations: &'static LifecycleOperations<'static>,
    transport: &'static HttpTransport,
    fs: &'static dyn FileSystem,
    helper_cache: &'static Path,
    pending_starts: Mutex<BTreeSet<String>>,
}

impl PlayitLifecycleController {
    fn start_for_server(&self, server: &ConfigServer) {
        if !server.playit_enabled {
            return;
        }
        let mut services = self.services.lock().expect("Playit service lock poisoned");
        let service = services.entry(server.id.clone()).or_insert_with(|| {
            PlayitService::new(
                server.id.clone(),
                server.playit_enabled,
                self.process,
                self.secrets,
                self.operations,
            )
        });
        if service.is_active() {
            if service.lifecycle_status() == PlayitLifecycleStatus::Stopping {
                self.pending_starts
                    .lock()
                    .expect("Playit pending-start lock poisoned")
                    .insert(server.id.clone());
            }
            return;
        }
        self.pending_starts
            .lock()
            .expect("Playit pending-start lock poisoned")
            .remove(&server.id);
        match service.has_secret() {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                service.record_start_failure(error.to_string());
                return;
            }
        }
        let acquisition =
            match msc_infrastructure::playit::PlayitBinaryAcquisition::for_current_platform(
                self.transport,
                self.fs,
                self.helper_cache,
            ) {
                Ok(acquisition) => acquisition,
                Err(error) => {
                    service
                        .record_start_failure(format!("Playit helper acquisition failed: {error}"));
                    return;
                }
            };
        let working_directory = PathBuf::from(&server.server_dir).join(".msc2-playit");
        if let Err(error) = std::fs::create_dir_all(&working_directory) {
            service.record_start_failure(format!(
                "Playit helper working directory could not be created: {error}"
            ));
            return;
        }
        let launch = msc_infrastructure::playit::PlayitLaunch::managed(working_directory);
        let _ = service.start(launch, &acquisition);
    }

    fn stop_for_server(&self, server_id: &str) {
        self.pending_starts
            .lock()
            .expect("Playit pending-start lock poisoned")
            .remove(server_id);
        let mut services = self.services.lock().expect("Playit service lock poisoned");
        let Some(service) = services.get_mut(server_id) else {
            return;
        };
        let _ = service.stop();
    }

    fn stop_broadcast_for_server(&self, server_id: &str) {
        let mut services = self
            .broadcast
            .lock()
            .expect("Broadcast service lock poisoned");
        if let Some(service) = services.get_mut(server_id) {
            let _ = service.stop();
        }
    }

    fn stop_all(&self) {
        self.pending_starts
            .lock()
            .expect("Playit pending-start lock poisoned")
            .clear();
        let mut services = self.services.lock().expect("Playit service lock poisoned");
        for service in services.values_mut() {
            // Host reset is destructive to approved local paths, so it must
            // use the same bounded reset as the explicit Playit reset route.
            // A graceful request alone could leave playitd alive while its
            // bridge and working directory are being removed.
            if let Err(error) = service.reset() {
                service.record_start_failure(error.to_string());
            }
        }
        let mut broadcast = self
            .broadcast
            .lock()
            .expect("Broadcast service lock poisoned");
        for service in broadcast.values_mut() {
            let _ = service.stop();
        }
    }

    fn start_pending(&self, lifecycle: &LifecycleRoutesState) {
        let Some(server) = lifecycle.active_config_server() else {
            self.pending_starts
                .lock()
                .expect("Playit pending-start lock poisoned")
                .clear();
            return;
        };
        if !lifecycle.status_snapshot().running {
            return;
        }
        let should_start = self
            .pending_starts
            .lock()
            .expect("Playit pending-start lock poisoned")
            .remove(&server.id);
        if should_start {
            self.start_for_server(&server);
        }
    }
}

impl PlayitLifecycleIntegration for PlayitLifecycleController {
    fn start_for_server(&self, server: &ConfigServer) {
        Self::start_for_server(self, server);
    }

    fn stop_for_server(&self, server_id: &str) {
        Self::stop_for_server(self, server_id);
    }

    fn stop_broadcast_for_server(&self, server_id: &str) {
        Self::stop_broadcast_for_server(self, server_id);
    }

    fn stop_all(&self) {
        Self::stop_all(self);
    }
}

#[derive(Clone)]
pub struct NetworkingState {
    pub(crate) lifecycle: LifecycleRoutesState,
    operations: OperationsState,
    playit: Arc<Mutex<BTreeMap<String, SharedPlayitService>>>,
    playit_mutation: Arc<tokio::sync::Semaphore>,
    broadcast: Arc<Mutex<BTreeMap<String, SharedBroadcastService>>>,
    process: &'static (dyn ProcessSupervisor + Send + Sync),
    secrets: &'static (dyn SecretStore + Send + Sync),
    operations_ref: &'static LifecycleOperations<'static>,
    transport: &'static HttpTransport,
    playit_transport: &'static (dyn PlayitHttpTransport + Send + Sync),
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
        let playit_transport: &'static (dyn PlayitHttpTransport + Send + Sync) =
            Box::leak(Box::new(ProviderHttpTransport::new()));
        let fs: &'static dyn FileSystem = Box::leak(Box::new(StdFileSystem));
        let helper_cache: &'static Path = Box::leak(Box::new(
            config_repository::default_app_data_dir().join("helpers"),
        ));
        let _ = std::fs::create_dir_all(helper_cache);
        let playit = Arc::new(Mutex::new(BTreeMap::new()));
        let broadcast = Arc::new(Mutex::new(BTreeMap::new()));
        let playit_mutation = Arc::new(tokio::sync::Semaphore::new(1));
        for server in lifecycle.app_config_snapshot().servers {
            let bridge = Path::new(&server.server_dir)
                .join(".msc2-playit")
                .join("secret-bridge");
            let _ = PlayitSecretBridge::remove_path(&bridge);
        }
        let state = Self {
            lifecycle,
            operations,
            playit: playit.clone(),
            playit_mutation,
            broadcast: broadcast.clone(),
            process,
            secrets,
            operations_ref,
            transport,
            playit_transport,
            fs,
            helper_cache,
        };
        let controller = Arc::new(PlayitLifecycleController {
            services: playit.clone(),
            broadcast,
            process,
            secrets,
            operations: operations_ref,
            transport,
            fs,
            helper_cache,
            pending_starts: Mutex::new(BTreeSet::new()),
        });
        state
            .lifecycle
            .register_playit_lifecycle(controller.clone());
        spawn_playit_output_pump(playit, state.lifecycle.clone(), Arc::clone(&controller));
        spawn_broadcast_output_pump(Arc::clone(&state.broadcast), state.lifecycle.clone());
        state
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

    /// Start the managed agent during native account setup, before the
    /// application service asks Playit to inventory or create tunnels.
    /// Returns true only when this setup call started a new helper, so a
    /// failed setup can clean up its own process without stopping a helper
    /// that was already serving the server.
    fn start_playit_for_setup(
        &self,
        server: &ConfigServer,
        expected_agent_id: &str,
        should_cancel: &impl Fn() -> bool,
    ) -> Result<bool, String> {
        if !server.playit_enabled {
            return Ok(false);
        }

        let mut services = self.playit_service(server);
        let service = services.get_mut(&server.id).expect("service was inserted");
        if service.lifecycle_status() == PlayitLifecycleStatus::Stopping {
            return Err("The existing Playit helper is still stopping.".into());
        }
        let mut started = false;
        if !service.is_active() {
            let acquisition =
                msc_infrastructure::playit::PlayitBinaryAcquisition::for_current_platform(
                    self.transport,
                    self.fs,
                    self.helper_cache,
                )
                .map_err(|error| format!("Playit helper acquisition failed: {error}"))?;
            let working_directory = PathBuf::from(&server.server_dir).join(".msc2-playit");
            std::fs::create_dir_all(&working_directory).map_err(|error| {
                format!("Playit helper working directory could not be created: {error}")
            })?;
            let launch = msc_infrastructure::playit::PlayitLaunch::managed(working_directory);
            service
                .start(launch, &acquisition)
                .map_err(|error| error.to_string())?;
            started = true;
        }

        // The provider cannot create a tunnel for a claimed-but-offline agent.
        // Wait for playitd's own matching connection line instead of treating a
        // later player address as the first readiness signal (there is no such
        // address until after a tunnel exists).
        service
            .wait_for_agent_connection(expected_agent_id, should_cancel)
            .map_err(|error| error.to_string())?;
        Ok(started)
    }

    fn stop_playit_after_setup_failure(&self, server_id: &str) {
        let mut services = self.playit.lock().expect("Playit service lock poisoned");
        if let Some(service) = services.get_mut(server_id) {
            let _ = service.reset();
        }
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
        .route("/playit/setup", post(playit_setup))
        .route("/playit/reset", post(playit_reset))
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
        .route(
            "/broadcast/credentials",
            axum::routing::get(broadcast_credentials).post(set_broadcast_credentials),
        )
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

fn spawn_playit_output_pump(
    services: Arc<Mutex<BTreeMap<String, SharedPlayitService>>>,
    lifecycle: LifecycleRoutesState,
    controller: Arc<PlayitLifecycleController>,
) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    handle.spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            interval.tick().await;
            let mut transport_updates = Vec::new();
            {
                let Ok(mut services) = services.lock() else {
                    continue;
                };
                for (server_id, service) in services.iter_mut() {
                    if let Err(error) = service.poll() {
                        service.record_start_failure(error.to_string());
                    }
                    let status = service.lifecycle_status();
                    if service.first_start_ready() {
                        transport_updates
                            .push((server_id.clone(), FirstStartTransportState::Ready));
                    } else if matches!(status, PlayitLifecycleStatus::Failed { .. }) {
                        transport_updates
                            .push((server_id.clone(), FirstStartTransportState::Failed));
                    }
                }
            }
            for (server_id, status) in transport_updates {
                let _ = lifecycle.mark_first_start_transport_for_server(
                    &server_id,
                    FirstRunTransport::Playit,
                    status,
                );
            }
            controller.start_pending(&lifecycle);
        }
    });
}

/// Keeps the managed Xbox Broadcast process moving independently of status
/// requests. Its output contains both the Microsoft device-code prompt and
/// the readiness line that completes the first-start operation.
fn spawn_broadcast_output_pump(
    services: Arc<Mutex<BTreeMap<String, SharedBroadcastService>>>,
    lifecycle: LifecycleRoutesState,
) {
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    handle.spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            interval.tick().await;
            let Ok(mut services) = services.lock() else {
                continue;
            };
            for service in services.values_mut() {
                match service.poll() {
                    Ok(lines) => {
                        for line in lines {
                            append_broadcast_console_line(&lifecycle, line);
                        }
                    }
                    Err(error) => {
                        lifecycle.append_console_line("xbox-broadcast", &error.to_string());
                    }
                }
            }
        }
    });
}

fn append_broadcast_console_line(lifecycle: &LifecycleRoutesState, line: BroadcastOutputLine) {
    let text = match line.stream {
        msc_infrastructure::process::OutputStream::Stdout => line.line,
        msc_infrastructure::process::OutputStream::Stderr => {
            format!("[Xbox Broadcast stderr] {}", line.line)
        }
    };
    lifecycle.append_console_line("xbox-broadcast", &text);
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
    let (has_secret, lifecycle_status) = {
        let mut services = state.playit_service(&server);
        let service = services.get_mut(&server.id).expect("service was inserted");
        (
            service.has_secret().unwrap_or(false),
            service.lifecycle_status(),
        )
    };
    let status_note = match &lifecycle_status {
        PlayitLifecycleStatus::SetupRequired => Some("Playit setup is required.".into()),
        PlayitLifecycleStatus::Starting => Some("Playit is starting.".into()),
        PlayitLifecycleStatus::WaitingForTunnels => Some("Waiting for Playit tunnels.".into()),
        PlayitLifecycleStatus::Running => None,
        PlayitLifecycleStatus::Stopping => Some("Playit is stopping.".into()),
        PlayitLifecycleStatus::Stopped => None,
        PlayitLifecycleStatus::TimedOut => {
            Some("Playit timed out while waiting for tunnels.".into())
        }
        PlayitLifecycleStatus::Failed { message } => Some(message.clone()),
    };
    let mut config = state.lifecycle.app_config_snapshot();
    let tunnel_specs = server_playit_tunnel_specs(&server);
    let configured_agent_id = config.playit_agent_id.clone();
    let has_missing_address = tunnel_specs.iter().any(|spec| match spec.kind {
        msc_domain::networking::PlayitTunnelKind::Java => config.playit_java_address.is_none(),
        msc_domain::networking::PlayitTunnelKind::Bedrock => {
            config.playit_bedrock_address.is_none()
        }
        msc_domain::networking::PlayitTunnelKind::Voice => config.playit_voice_address.is_none(),
    });
    if server.playit_enabled
        && has_secret
        && has_missing_address
        && !tunnel_specs.is_empty()
        && let Some(agent_id) = configured_agent_id.filter(|agent_id| !agent_id.trim().is_empty())
    {
        let transport = state.playit_transport;
        let secrets = state.secrets;
        let agent_id_for_refresh = agent_id.clone();
        let refresh = tokio::task::spawn_blocking(move || {
            PlayitAccountSetup::new(transport, secrets)
                .refresh_tunnel_addresses(&agent_id_for_refresh, &tunnel_specs)
        })
        .await;
        if let Ok(Ok(addresses)) = refresh {
            let server_id = server.id.clone();
            let refreshed_agent_id = agent_id;
            let _ = state.lifecycle.try_mutate_config(|saved| {
                // A reset or a new setup may have happened while the one
                // read-only inventory request was in flight. Never restore
                // addresses for an agent that is no longer the saved one.
                if saved.playit_agent_id.as_deref() != Some(refreshed_agent_id.as_str()) {
                    return Ok::<_, std::convert::Infallible>(());
                }
                if addresses.java.is_some() {
                    saved.playit_java_address = addresses.java;
                }
                if addresses.bedrock.is_some() {
                    saved.playit_bedrock_address = addresses.bedrock;
                }
                if addresses.voice.is_some() {
                    saved.playit_voice_address = addresses.voice;
                    if let Some(saved_server) = saved
                        .servers
                        .iter_mut()
                        .find(|saved_server| saved_server.id == server_id)
                    {
                        saved_server.playit_voice_chat_enabled = true;
                    }
                }
                Ok::<_, std::convert::Infallible>(())
            });
            config = state.lifecycle.app_config_snapshot();
        }
    }
    let voice_address = config.playit_voice_address;
    if let Some(voice_host) = voice_address.as_deref() {
        // Reading status after a later SVC install is the lightweight sync
        // path: an existing tunnel needs no re-authentication, only the
        // provider address written into the loader-specific config file.
        let _ = patch_voice_chat_config(&server.server_dir, voice_host);
    }
    let svc_installed = voice_chat_installed(Path::new(&server.server_dir));
    let voice_chat_enabled = server.playit_enabled
        && svc_installed
        && (server.playit_voice_chat_enabled || voice_address.is_some());
    Json(PlayitStatusDto {
        server_name: server.display_name,
        server_type: server.server_type.raw_value().into(),
        playit_enabled: server.playit_enabled,
        is_running: lifecycle_status.is_active(),
        has_secret_key: has_secret,
        java_address: config.playit_java_address,
        bedrock_address: config.playit_bedrock_address,
        voice_address,
        voice_chat_enabled,
        note: status_note,
    })
    .into_response()
}

/// `POST /v1/playit/setup` is the authenticated boundary for native account
/// provisioning. The worker keeps credentials and the temporary Playit
/// session inside the setup call while the shared operation journal reports
/// progress, cancellation, and the final safe outcome.
pub async fn playit_setup(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<PlayitSetupRequestDto>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Networking) {
        return response;
    }
    let Json(request) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    if request.email.trim().is_empty() {
        return invalid_body("missing_email", "email is required.");
    }
    if request.password.is_empty() {
        return invalid_body("missing_password", "password is required.");
    }

    // Reserve the single Playit mutation slot before admitting the operation.
    // A worker that has been accepted but not scheduled yet must still be
    // ordered before a reset that arrives immediately afterward.
    let playit_mutation_permit = match Arc::clone(&state.playit_mutation).try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return error_response(
                StatusCode::CONFLICT,
                "setup_in_progress",
                "A Playit setup or reset is already in progress.",
            );
        }
    };

    let operation_id = match state.operations.begin_lifecycle(
        PLAYIT_SETUP_OPERATION_TYPE,
        Some(PLAYIT_SETUP_OPERATION_TARGET.to_string()),
        "Starting native Playit setup.",
    ) {
        Ok(id) => id,
        Err(msc_application::operations::LifecycleOperationError::Conflict(_)) => {
            return error_response(
                StatusCode::CONFLICT,
                "setup_in_progress",
                "A Playit setup is already in progress.",
            );
        }
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let response = PlayitSetupAcceptedDto {
        result: "setup_accepted".into(),
        operation_id: operation_id.as_str().to_string(),
        message: Some("Playit setup has started.".into()),
    };
    let operation_id_for_worker = operation_id.clone();
    let operations = state.operations.clone();
    let lifecycle = state.lifecycle.clone();
    let networking = state.clone();
    let transport = state.playit_transport;
    let secrets = state.secrets;
    let existing_agent_id = state.lifecycle.app_config_snapshot().playit_agent_id;
    let active_server = state.lifecycle.active_config_server();
    let tunnel_specs = active_server
        .as_ref()
        .filter(|server| server.playit_enabled)
        .map(server_playit_tunnel_specs)
        .unwrap_or_default();
    let voice_server_dir = active_server
        .as_ref()
        .filter(|server| server.playit_enabled)
        .map(|server| server.server_dir.clone());
    let voice_server_id = active_server.as_ref().map(|server| server.id.clone());
    let setup_server = active_server.clone();
    let setup_started = Arc::new(AtomicBool::new(false));
    let setup_started_for_worker = Arc::clone(&setup_started);
    let provisions_voice_tunnel = tunnel_specs
        .iter()
        .any(|spec| spec.kind == msc_domain::networking::PlayitTunnelKind::Voice);
    let provisions_tunnels = !tunnel_specs.is_empty();
    let email = request.email.trim().to_owned();
    let password = request.password;

    // The provider transport is synchronous like the rest of the infrastructure
    // traits. Run it off the async executor, while the operation journal remains
    // the single source of progress and cancellation truth.
    tokio::task::spawn_blocking(move || {
        // Hold the admission permit through the setup worker's final
        // config/key write. Reset acquires the same permit before clearing
        // local state, so the two operations cannot overwrite one another.
        let _playit_mutation_permit = playit_mutation_permit;
        let should_cancel = operations.cancellation_check(&operation_id_for_worker);
        let operation_for_progress = operation_id_for_worker.clone();
        let operations_for_progress = operations.clone();
        let report = move |stage: PlayitSetupStage| {
            let (current, total) = stage.progress();
            let _ = operations_for_progress.progress(
                &operation_for_progress,
                current,
                total,
                stage.status_line(),
            );
        };
        let should_cancel_for_agent_start = should_cancel.clone();
        let ensure_agent = |agent_id: &str| {
            let server = setup_server
                .as_ref()
                .ok_or_else(|| "No active server is selected for Playit tunnels.".to_string())?;
            let started = networking.start_playit_for_setup(
                server,
                agent_id,
                &should_cancel_for_agent_start,
            )?;
            if started {
                setup_started_for_worker.store(true, Ordering::Release);
            }
            Ok(())
        };
        let lifecycle_for_agent_configuration = lifecycle.clone();
        let save_agent_configuration = |agent_id: &str| {
            lifecycle_for_agent_configuration
                .try_mutate_config(|config| {
                    config.playit_agent_id = Some(agent_id.to_owned());
                    Ok::<_, std::convert::Infallible>(())
                })
                .map_err(|_| {
                    "MSC could not save the Playit agent configuration on this host.".to_owned()
                })
        };
        let setup = PlayitAccountSetup::with_agent_lifecycle(
            transport,
            secrets,
            &save_agent_configuration,
            &ensure_agent,
        );
        let result = setup.run_with_tunnels(
            &email,
            &password,
            existing_agent_id.as_deref(),
            &tunnel_specs,
            should_cancel,
            report,
        );

        match result {
            Ok(result) => {
                let reused_existing_agent = result.reused_existing_agent;
                let tunnel_addresses = result.tunnel_addresses.clone();
                let config_saved = lifecycle
                    .try_mutate_config(|config| {
                        config.playit_agent_id = Some(result.agent_id);
                        if provisions_tunnels {
                            config.playit_java_address = tunnel_addresses.java.clone();
                            config.playit_bedrock_address = tunnel_addresses.bedrock.clone();
                            config.playit_voice_address = tunnel_addresses.voice.clone();
                        }
                        if provisions_voice_tunnel
                            && let Some(server_id) = voice_server_id.as_deref()
                            && let Some(server) = config
                                .servers
                                .iter_mut()
                                .find(|server| server.id == server_id)
                        {
                            server.playit_voice_chat_enabled = tunnel_addresses.voice.is_some();
                            server.svc_tunnel_prompt_dismissed = false;
                        }
                        Ok::<_, std::convert::Infallible>(())
                    })
                    .is_ok();
                if !config_saved {
                    if setup_started.load(Ordering::Acquire)
                        && let Some(server) = setup_server.as_ref()
                    {
                        networking.stop_playit_after_setup_failure(&server.id);
                    }
                    // A newly claimed key is useless without its matching
                    // agent ID. Remove it so a later attempt cannot create a
                    // second cloud agent because of a half-written local state.
                    if !reused_existing_agent {
                        let _ = secrets.delete(PLAYIT_SECRET_KEY);
                    }
                    let _ = operations.fail(
                        &operation_id_for_worker,
                        "credential_store_failed",
                        "MSC could not save the Playit agent configuration on this host.".into(),
                    );
                } else {
                    if let (Some(server_dir), Some(voice_host)) = (
                        voice_server_dir.as_deref(),
                        tunnel_addresses.voice.as_deref(),
                    ) {
                        let _ = patch_voice_chat_config(server_dir, voice_host);
                    }
                    let _ = operations.succeed(
                        &operation_id_for_worker,
                        "Playit agent and tunnels are configured.",
                        BTreeMap::from([
                            ("agentConfigured".to_string(), "true".to_string()),
                            (
                                "reusedExistingAgent".to_string(),
                                reused_existing_agent.to_string(),
                            ),
                        ]),
                    );
                }
            }
            Err(PlayitSetupError::Cancelled) => {
                if setup_started.load(Ordering::Acquire)
                    && let Some(server) = setup_server.as_ref()
                {
                    networking.stop_playit_after_setup_failure(&server.id);
                }
                let _ = operations.cancel(
                    &operation_id_for_worker,
                    "Playit setup cancelled before credentials were saved.",
                );
            }
            Err(error) => {
                if setup_started.load(Ordering::Acquire)
                    && let Some(server) = setup_server.as_ref()
                {
                    networking.stop_playit_after_setup_failure(&server.id);
                }
                let _ = operations.fail(
                    &operation_id_for_worker,
                    error.stable_code(),
                    error.to_string(),
                );
            }
        }
    });

    (StatusCode::ACCEPTED, Json(response)).into_response()
}

/// Clear host-local Playit state after every currently managed helper has
/// stopped. The mutation runs off the async executor because reset waits for
/// process reconciliation, and it shares a lock with setup so the worker
/// cannot write credentials or configuration over a reset that is in flight.
pub async fn playit_reset(
    State(state): State<NetworkingState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Networking) {
        return response;
    }

    let result = tokio::task::spawn_blocking(move || reset_playit_local_state(&state)).await;
    match result {
        Ok(Ok(result)) => Json(result).into_response(),
        Ok(Err((status, code, message))) => error_response(status, &code, &message),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "playit_reset_failed",
            &format!("Playit reset worker failed: {error}"),
        ),
    }
}

fn reset_playit_local_state(
    state: &NetworkingState,
) -> Result<PlayitResetResultDto, (StatusCode, String, String)> {
    let _playit_mutation_permit = state.playit_mutation.clone().acquire_owned();
    let _playit_mutation_permit = tokio::runtime::Handle::current()
        .block_on(_playit_mutation_permit)
        .expect("Playit mutation semaphore closed");
    let mut services = state.playit.lock().expect("Playit service lock poisoned");
    for service in services.values_mut() {
        if let Err(error) = service.reset() {
            return Err((
                StatusCode::CONFLICT,
                "playit_reset_failed".into(),
                error.to_string(),
            ));
        }
    }
    drop(services);

    for server in state.lifecycle.app_config_snapshot().servers {
        let bridge = Path::new(&server.server_dir)
            .join(".msc2-playit")
            .join("secret-bridge");
        if let Err(error) = PlayitSecretBridge::remove_path(&bridge) {
            return Err((
                StatusCode::CONFLICT,
                "playit_reset_failed".into(),
                error.to_string(),
            ));
        }
    }

    let had_secret = match state.secrets.get(PLAYIT_SECRET_KEY) {
        Ok(value) => value.is_some_and(|secret| !secret.trim().is_empty()),
        Err(error) => {
            return Err((
                StatusCode::CONFLICT,
                "playit_reset_failed".into(),
                error.to_string(),
            ));
        }
    };
    if let Err(error) = state.secrets.delete(PLAYIT_SECRET_KEY) {
        return Err((
            StatusCode::CONFLICT,
            "playit_reset_failed".into(),
            error.to_string(),
        ));
    }
    if state
        .lifecycle
        .try_mutate_config(|config| {
            config.playit_agent_id = None;
            config.playit_java_address = None;
            config.playit_bedrock_address = None;
            config.playit_voice_address = None;
            for server in &mut config.servers {
                server.svc_tunnel_prompt_dismissed = false;
            }
            Ok::<_, std::convert::Infallible>(())
        })
        .is_err()
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "playit_reset_failed".into(),
            "Could not clear the saved Playit state.".into(),
        ));
    }

    Ok(PlayitResetResultDto {
        result: if had_secret {
            "cleared"
        } else {
            "already_clear"
        }
        .into(),
        message: Some("Host-local Playit credentials and addresses were cleared.".into()),
        operation_id: None,
    })
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
    if service.is_active() {
        return Json(PlayitActionResultDto {
            result: "already_running".into(),
            message: None,
            operation_id: None,
        })
        .into_response();
    }
    let working_directory = PathBuf::from(&server.server_dir).join(".msc2-playit");
    let _ = std::fs::create_dir_all(&working_directory);
    let launch = msc_infrastructure::playit::PlayitLaunch::managed(working_directory);
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
    if !service.is_active() {
        return Json(PlayitActionResultDto {
            result: "not_running".into(),
            message: None,
            operation_id: None,
        })
        .into_response();
    }
    match service.stop() {
        Ok(operation_id) => (
            if operation_id.is_some() {
                StatusCode::ACCEPTED
            } else {
                StatusCode::OK
            },
            Json(PlayitActionResultDto {
                result: "stopped".into(),
                message: Some("Playit tunnel stop requested.".into()),
                operation_id,
            }),
        )
            .into_response(),
        Err(error) => helper_error_response(error.to_string(), "playit_stop_failed"),
    }
}

pub async fn broadcast_status(State(state): State<NetworkingState>) -> Response {
    let Ok(server) = state.active_server() else {
        return Json(BroadcastStatusDto {
            xbox_broadcast_running: false,
            bedrock_broadcast_running: false,
            gamertag: None,
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
            gamertag: status.gamertag,
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

pub async fn broadcast_credentials(State(state): State<NetworkingState>) -> Response {
    let config = state.lifecycle.app_config_snapshot();
    let mut has_password = state
        .secrets
        .get(msc_infrastructure::xbox_broadcast::global_alt_password_secret_key())
        .map(|value| value.is_some_and(|value| !value.trim().is_empty()));
    if matches!(has_password, Ok(false)) {
        for server in &config.servers {
            has_password = state
                .secrets
                .get(&msc_infrastructure::xbox_broadcast::alt_password_secret_key(&server.id))
                .map(|value| value.is_some_and(|value| !value.trim().is_empty()));
            if matches!(has_password, Ok(true)) {
                break;
            }
        }
    }
    match has_password {
        Ok(has_password) => Json(BroadcastCredentialsStatusDto {
            email: config.xbox_broadcast_alt_email,
            gamertag: config.xbox_broadcast_alt_gamertag,
            has_password,
        })
        .into_response(),
        Err(error) => helper_error_response(error.to_string(), "credential_status_failed"),
    }
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
    if let Err(error) = state.secrets.set(
        msc_infrastructure::xbox_broadcast::global_alt_password_secret_key(),
        body.password.trim(),
    ) {
        return helper_error_response(error.to_string(), "credential_store_failed");
    }
    let user_update = state.lifecycle.try_mutate_config(|config| {
        config.xbox_broadcast_alt_email = Some(body.email.trim().to_string());
        config.xbox_broadcast_alt_gamertag = Some(body.gamertag.trim().to_string());
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
    let operation = match state.operations.begin_lifecycle(
        "broadcast-jar-download",
        None,
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

fn server_playit_tunnel_specs(server: &ConfigServer) -> Vec<PlayitTunnelSpec> {
    let java_port = (server.server_type == ServerType::Java)
        .then(|| {
            read_properties(Path::new(&server.server_dir).join("server.properties"))
                .get("server-port")
                .and_then(|value| value.parse::<u16>().ok())
                .filter(|port| *port > 0)
        })
        .flatten();
    let bedrock_port = server
        .bedrock_port
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port > 0);
    // Setup is also the re-authentication path after a user installs SVC on
    // an already configured Playit server. The filesystem check remains the
    // guard that prevents a voice tunnel for a server without SVC.
    let voice_enabled = voice_chat_installed(Path::new(&server.server_dir));
    playit_tunnel_specs(
        server.server_type,
        java_port,
        server.bedrock_enabled,
        bedrock_port,
        voice_enabled,
    )
}

fn voice_chat_installed(server_dir: &Path) -> bool {
    ["plugins", "mods"].iter().any(|folder| {
        std::fs::read_dir(server_dir.join(folder))
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .any(|entry| {
                let path = entry.path();
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))
                    && (name.contains("voicechat") || name.contains("voice-chat"))
            })
    })
}

fn patch_voice_chat_config(server_dir: &str, voice_host: &str) -> Result<(), std::io::Error> {
    let root = Path::new(server_dir);
    if !voice_chat_installed(root) {
        return Ok(());
    }
    let plugins_path = root.join("plugins/voicechat/voicechat-server.properties");
    let config_path = root.join("config/voicechat/voicechat-server.properties");
    let path = if plugins_path.exists() {
        plugins_path
    } else if config_path.exists() {
        config_path
    } else if root.join("plugins").exists() && !root.join("mods").exists() {
        plugins_path
    } else {
        config_path
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let existing = if path.exists() {
        std::fs::read_to_string(&path)?
    } else {
        String::new()
    };
    std::fs::write(path, patch_voice_chat_properties(&existing, voice_host))
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
