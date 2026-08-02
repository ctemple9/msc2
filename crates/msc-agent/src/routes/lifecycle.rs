//! Shared state and handlers for Phase 4 lifecycle routes.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{ActiveServerRequestDto, ErrorDto, PermissionCategoryDto, SimpleResultDto};
use msc_application::import::ImportedPaperServer;
use msc_application::java_launch::{
    PaperLaunchRequest, StdJavaLaunchFileSystem, ValidatedJavaLaunch, build_paper_launch_command,
};
use msc_application::lifecycle::{
    ConsoleSink, ImportedJavaServer, JavaServerRepository, LifecycleError, LifecycleService,
    ServerId,
};
use msc_application::status::{LifecycleStatusSnapshot, PerformanceSnapshot};
use msc_infrastructure::console_buffer::ConsoleLine;
use msc_infrastructure::metrics::PsProcessMetricsProvider;
use msc_infrastructure::process::{
    OutputLineFramer, OutputStream, ProcessEvent, ProcessId, ProcessSpawnRequest, ProcessSupervisor,
};
use tokio::task::JoinHandle;

use crate::auth::AuthenticatedCredential;
use crate::ws::console::ConsoleState;

#[derive(Clone)]
pub struct LifecycleRoutesState {
    inner: Arc<LifecycleRoutesInner>,
}

struct LifecycleRoutesInner {
    registry: &'static AgentServerRegistry,
    process: &'static (dyn ProcessSupervisor + Send + Sync),
    console: &'static AgentConsoleSink,
    metrics: PsProcessMetricsProvider,
    lifecycle: Mutex<LifecycleService<'static>>,
    pump_tasks: Mutex<Vec<JoinHandle<()>>>,
}

#[derive(Debug, Default)]
pub struct AgentServerRegistry {
    servers: Mutex<BTreeMap<String, RegisteredPaperServer>>,
}

#[derive(Debug, Clone)]
struct RegisteredPaperServer {
    imported: ImportedPaperServer,
}

pub struct AgentConsoleSink {
    console: ConsoleState,
}

impl LifecycleRoutesState {
    pub fn new(console_state: ConsoleState) -> Self {
        let registry = Box::leak(Box::new(AgentServerRegistry::default()));
        let process = Box::leak(default_process_supervisor());
        let console = Box::leak(Box::new(AgentConsoleSink {
            console: console_state,
        }));
        let lifecycle = LifecycleService::new(registry, process, console);

        Self {
            inner: Arc::new(LifecycleRoutesInner {
                registry,
                process,
                console,
                metrics: PsProcessMetricsProvider::default(),
                lifecycle: Mutex::new(lifecycle),
                pump_tasks: Mutex::new(Vec::new()),
            }),
        }
    }

    #[cfg(test)]
    pub fn with_fake_process(console_state: ConsoleState) -> Self {
        let registry = Box::leak(Box::new(AgentServerRegistry::default()));
        let process = Box::leak(Box::new(
            msc_infrastructure::process::FakeProcessSupervisor::new(),
        ));
        let console = Box::leak(Box::new(AgentConsoleSink {
            console: console_state,
        }));
        let lifecycle = LifecycleService::new(registry, process, console);

        Self {
            inner: Arc::new(LifecycleRoutesInner {
                registry,
                process,
                console,
                metrics: PsProcessMetricsProvider::default(),
                lifecycle: Mutex::new(lifecycle),
                pump_tasks: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn register_imported_paper(&self, server: ImportedPaperServer) {
        self.inner.registry.insert(server);
    }

    pub fn servers(&self) -> Vec<RegisteredServerDtoParts> {
        self.inner.registry.list()
    }

    pub fn active_server_id(&self) -> Option<String> {
        self.inner
            .lifecycle
            .lock()
            .unwrap()
            .active_server()
            .map(|id| id.as_str().to_string())
    }

    pub fn select_active_server(&self, server_id: String) -> Result<String, LifecycleError> {
        let id = ServerId::new(server_id);
        let active = id.as_str().to_string();
        self.inner
            .lifecycle
            .lock()
            .unwrap()
            .select_active_server(id)?;
        Ok(active)
    }

    pub fn start_active_server(&self) -> Result<Option<String>, LifecycleError> {
        self.drain_active_process_events();
        let active = self
            .inner
            .lifecycle
            .lock()
            .unwrap()
            .active_server()
            .cloned()
            .ok_or(LifecycleError::NoActiveServer)?;
        let registered = self
            .inner
            .registry
            .get(&active)
            .ok_or_else(|| LifecycleError::ServerNotFound(active.clone()))?;
        let launch = build_launch_request(&registered)?;
        let pid = self
            .inner
            .lifecycle
            .lock()
            .unwrap()
            .start_active_server(launch)?;
        self.spawn_process_pump(pid);
        Ok(Some(active.as_str().to_string()))
    }

    pub fn stop_active_server(&self) -> Result<Option<String>, LifecycleError> {
        self.drain_active_process_events();
        self.inner.lifecycle.lock().unwrap().request_stop()?;
        Ok(self.active_server_id())
    }

    pub fn send_command(&self, command: &str) -> Result<Option<String>, LifecycleError> {
        self.drain_active_process_events();
        self.inner.lifecycle.lock().unwrap().send_command(command)?;
        Ok(self.active_server_id())
    }

    pub fn status_snapshot(&self) -> LifecycleStatusSnapshot {
        self.drain_active_process_events();
        self.inner
            .lifecycle
            .lock()
            .unwrap()
            .status_snapshot()
            .unwrap_or_else(|_| stopped_status())
    }

    pub fn performance_snapshot(&self) -> PerformanceSnapshot {
        self.drain_active_process_events();
        self.inner
            .lifecycle
            .lock()
            .unwrap()
            .performance_snapshot(&self.inner.metrics, unix_timestamp_string())
            .unwrap_or_else(|_| stopped_performance())
    }

    fn spawn_process_pump(&self, pid: ProcessId) {
        let state = self.clone();
        let handle = tokio::spawn(async move {
            let mut framer = OutputLineFramer::new();
            loop {
                state.drain_process_events(pid, &mut framer);
                if state
                    .inner
                    .lifecycle
                    .lock()
                    .unwrap()
                    .active_process()
                    .is_none_or(|active| active != pid)
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        self.inner.pump_tasks.lock().unwrap().push(handle);
    }

    fn drain_active_process_events(&self) {
        let Some(pid) = self.inner.lifecycle.lock().unwrap().active_process() else {
            return;
        };
        let mut framer = OutputLineFramer::new();
        self.drain_process_events(pid, &mut framer);
    }

    fn drain_process_events(&self, pid: ProcessId, framer: &mut OutputLineFramer) {
        let Ok(events) = self.inner.process.drain_events(pid) else {
            return;
        };
        for event in events {
            for line in framer.push_event(&event) {
                self.push_process_line(&event, &line);
                let _ = self
                    .inner
                    .lifecycle
                    .lock()
                    .unwrap()
                    .ingest_console_line(&line);
            }
            let _ = self
                .inner
                .lifecycle
                .lock()
                .unwrap()
                .handle_process_event(pid, &event);
        }
    }

    fn push_process_line(&self, event: &ProcessEvent, text: &str) {
        let source = match event {
            ProcessEvent::Output {
                stream: OutputStream::Stderr,
                ..
            } => "stderr",
            ProcessEvent::Output { .. } | ProcessEvent::Exited(_) => "stdout",
        };
        self.inner
            .console
            .push(ConsoleLine::new(source, None, text.to_string()));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredServerDtoParts {
    pub id: String,
    pub name: String,
    pub directory: String,
    pub server_type: String,
    pub java_flavor: Option<String>,
    pub game_port: Option<i64>,
}

pub async fn start(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::ServerControl) {
        return response;
    }

    match state.start_active_server() {
        Ok(active_server_id) => Json(SimpleResultDto {
            result: "start_requested".to_string(),
            active_server_id,
        })
        .into_response(),
        Err(error) => lifecycle_error_response(error),
    }
}

pub async fn stop(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::ServerControl) {
        return response;
    }

    match state.stop_active_server() {
        Ok(active_server_id) => Json(SimpleResultDto {
            result: "stop_requested".to_string(),
            active_server_id,
        })
        .into_response(),
        Err(error) => lifecycle_error_response(error),
    }
}

pub async fn active_server(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<ActiveServerRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::ServerControl) {
        return response;
    }

    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let Some(server_id) = body.server_id.filter(|value| !value.trim().is_empty()) else {
        return invalid_body("missing_server_id", "serverId is required.");
    };

    match state.select_active_server(server_id) {
        Ok(active_server_id) => Json(SimpleResultDto {
            result: "activated".to_string(),
            active_server_id: Some(active_server_id),
        })
        .into_response(),
        Err(error) => lifecycle_error_response(error),
    }
}

impl AgentServerRegistry {
    fn insert(&self, server: ImportedPaperServer) {
        self.servers.lock().unwrap().insert(
            server.id.as_str().to_string(),
            RegisteredPaperServer { imported: server },
        );
    }

    fn get(&self, id: &ServerId) -> Option<RegisteredPaperServer> {
        self.servers.lock().unwrap().get(id.as_str()).cloned()
    }

    fn list(&self) -> Vec<RegisteredServerDtoParts> {
        self.servers
            .lock()
            .unwrap()
            .values()
            .map(|server| {
                let imported = &server.imported;
                RegisteredServerDtoParts {
                    id: imported.id.as_str().to_string(),
                    name: imported.display_name.clone(),
                    directory: imported.server_dir.display().to_string(),
                    server_type: "java".to_string(),
                    java_flavor: Some("paper".to_string()),
                    game_port: Some(imported.game_port),
                }
            })
            .collect()
    }
}

impl JavaServerRepository for AgentServerRegistry {
    fn load(&self, id: &ServerId) -> Result<Option<ImportedJavaServer>, LifecycleError> {
        Ok(self
            .get(id)
            .map(|server| server.imported.lifecycle_server()))
    }
}

impl ConsoleSink for AgentConsoleSink {
    fn append_system_line(&self, _server_id: &ServerId, line: &str) {
        self.push(ConsoleLine::new("system", None, line.to_string()));
    }
}

impl AgentConsoleSink {
    fn push(&self, line: ConsoleLine) {
        self.console.push(line);
    }
}

pub fn require_permission(
    credential: &AuthenticatedCredential,
    category: PermissionCategoryDto,
) -> Option<Response> {
    if credential.permissions.contains(&category)
        || credential
            .permissions
            .contains(&PermissionCategoryDto::Admin)
    {
        None
    } else {
        Some(error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "This token is not allowed to perform that action.",
        ))
    }
}

pub fn invalid_body(code: &str, message: &str) -> Response {
    error_response(StatusCode::BAD_REQUEST, code, message)
}

pub fn lifecycle_error_response(error: LifecycleError) -> Response {
    match error {
        LifecycleError::NoActiveServer | LifecycleError::ServerNotRunning => {
            error_response(StatusCode::CONFLICT, "conflict", &error.to_string())
        }
        LifecycleError::ServerNotFound(_) => {
            error_response(StatusCode::NOT_FOUND, "not_found", &error.to_string())
        }
        LifecycleError::AlreadyInState(_)
        | LifecycleError::IllegalTransition(_)
        | LifecycleError::WrongActiveServer { .. } => {
            error_response(StatusCode::CONFLICT, "conflict", &error.to_string())
        }
        LifecycleError::Repository(_) | LifecycleError::Process(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        ),
    }
}

pub fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(ErrorDto {
            code: code.to_string(),
            message: message.to_string(),
            help_id: None,
            details: None,
        }),
    )
        .into_response()
}

fn build_launch_request(
    registered: &RegisteredPaperServer,
) -> Result<ProcessSpawnRequest, LifecycleError> {
    let imported = &registered.imported;
    let java_path = std::env::var("MSC2_JAVA_PATH").unwrap_or_else(|_| "java".to_string());
    let request = PaperLaunchRequest::new(
        ValidatedJavaLaunch::new(java_path, Vec::<String>::new()),
        &imported.server_dir,
        &imported.paper_jar_path,
        1.0,
        2.0,
        "",
    );
    let command = build_paper_launch_command(&StdJavaLaunchFileSystem, &request)
        .map_err(|error| LifecycleError::Process(error.to_string()))?;
    Ok(ProcessSpawnRequest {
        executable_path: command.executable_path,
        arguments: command.arguments,
        working_directory: command.working_directory,
        environment: Vec::new(),
    })
}

fn stopped_status() -> LifecycleStatusSnapshot {
    LifecycleStatusSnapshot {
        running: false,
        active_server_id: None,
        pid: None,
        server_type: None,
    }
}

fn stopped_performance() -> PerformanceSnapshot {
    PerformanceSnapshot {
        ts: unix_timestamp_string(),
        tps_1m: None,
        players_online: Some(0),
        cpu_percent: None,
        ram_used_mb: None,
        ram_max_mb: None,
        world_size_mb: None,
        server_type: None,
    }
}

fn unix_timestamp_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn default_process_supervisor() -> Box<dyn ProcessSupervisor + Send + Sync> {
    #[cfg(target_os = "macos")]
    {
        Box::new(msc_platform_macos::process::MacosJavaProcessSupervisor::new())
    }

    #[cfg(target_os = "linux")]
    {
        Box::new(msc_platform_linux::process::LinuxJavaProcessSupervisor::new())
    }

    #[cfg(target_os = "windows")]
    {
        Box::new(msc_platform_windows::process::WindowsJavaProcessSupervisor::new())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Box::new(msc_infrastructure::process::FakeProcessSupervisor::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use msc_api::dto::PermissionCategoryDto;
    use msc_application::import::ImportedPaperServer;
    use msc_domain::properties::ServerPropertiesModel;
    use std::collections::HashMap;

    fn imported_server(server_dir: std::path::PathBuf) -> ImportedPaperServer {
        ImportedPaperServer {
            id: ServerId::new("paper-1"),
            display_name: "Contract Paper".to_string(),
            paper_jar_path: server_dir.join("paper.jar"),
            server_dir,
            eula_accepted: Some(true),
            game_port: 25565,
            max_players: 20,
            world_name: "world".to_string(),
            properties: ServerPropertiesModel::from_dict(&HashMap::new(), None),
        }
    }

    #[tokio::test]
    async fn phase4_lifecycle_routes_state_selects_starts_and_stops_active_server() {
        let state = LifecycleRoutesState::with_fake_process(ConsoleState::default());
        let server_dir = std::env::temp_dir().join(format!(
            "msc2-agent-lifecycle-routes-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&server_dir);
        std::fs::create_dir_all(&server_dir).unwrap();
        let server = imported_server(server_dir.clone());
        std::fs::write(&server.paper_jar_path, b"fake jar").unwrap();

        state.register_imported_paper(server);
        assert_eq!(state.servers().len(), 1);
        assert_eq!(
            state.select_active_server("paper-1".to_string()).unwrap(),
            "paper-1"
        );

        let active = state.start_active_server().unwrap();
        assert_eq!(active.as_deref(), Some("paper-1"));
        let status = state.status_snapshot();
        assert!(status.running);
        assert_eq!(status.active_server_id.as_deref(), Some("paper-1"));
        assert_eq!(status.pid, Some(1000));

        let active = state.stop_active_server().unwrap();
        assert_eq!(active.as_deref(), Some("paper-1"));

        std::fs::remove_dir_all(server_dir).unwrap();
    }

    #[test]
    fn phase4_lifecycle_routes_permission_check_uses_declared_category() {
        let credential = AuthenticatedCredential {
            credential_id: "named".to_string(),
            label: "console".to_string(),
            role: crate::auth::CredentialRole::Named,
            permissions: vec![PermissionCategoryDto::ServerControl],
        };

        assert!(require_permission(&credential, PermissionCategoryDto::ServerControl).is_none());
        assert!(require_permission(&credential, PermissionCategoryDto::Fleet).is_some());
    }
}
