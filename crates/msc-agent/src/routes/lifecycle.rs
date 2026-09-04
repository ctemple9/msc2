//! Shared state and handlers for Phase 4 lifecycle routes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{ActiveServerRequestDto, ErrorDto, PermissionCategoryDto, SimpleResultDto};
use msc_application::bedrock_runtime::{
    BedrockProvisionRequest, BedrockRuntimeError, BedrockRuntimeEvent, BedrockRuntimeState,
    BedrockStartRequest, BedrockTerminationReason,
};
#[cfg(test)]
use msc_application::import::ImportedPaperServer;
use msc_application::java_launch::{
    PaperLaunchRequest, StdJavaLaunchFileSystem, ValidatedJavaLaunch, build_paper_launch_command,
    find_forge_args_file, find_neoforge_args_file, jvm_flags,
};
use msc_application::lifecycle::{
    ConsoleSink, ImportedJavaServer, JavaServerRepository, LifecycleError, LifecycleService,
    LifecycleState, ServerId,
};
use msc_application::status::{LifecycleStatusSnapshot, PerformanceSnapshot};
use msc_application::transfer::TransferExportServerInput;
use msc_domain::app_config_schema::{AppConfig, ConfigServer};
use msc_domain::helper::{
    FirstRunTransport, FirstStartCoordinator, FirstStartPhase, FirstStartTransportState,
    first_run_safety_cap_reached,
};
use msc_domain::identity::JavaServerFlavor;
use msc_domain::identity::ServerType;
use msc_domain::operation::OperationId;
use msc_domain::world::BackupAssociation;
use msc_infrastructure::audit_log::AuditLog;
use msc_infrastructure::config_repository::{
    AppConfigLoadError, ConfigSaveError, default_app_config_path, default_servers_root,
    load_app_config, load_app_config_migrating_legacy_secrets, save_app_config,
};
use msc_infrastructure::console_buffer::ConsoleLine;
use msc_infrastructure::fs::{FileSystem, StdFileSystem};
use msc_infrastructure::java_runtime_detection;
use msc_infrastructure::metrics::PsProcessMetricsProvider;
use msc_infrastructure::process::{
    OutputLineFramer, OutputStream, ProcessEvent, ProcessId, ProcessSpawnRequest, ProcessSupervisor,
};
use msc_infrastructure::secret_store::SecretStore;
use tokio::task::JoinHandle;

use crate::auth::{AuthState, AuthenticatedCredential};
use crate::routes::bedrock_runtime::BedrockRuntimeSelection;
use crate::routes::operations::{OperationsState, operation_error_response};
use crate::ws::console::ConsoleState;
use crate::ws::notifications::NotificationState;

/// Per-server outcome of world reconciliation and interrupted-mutation
/// recovery. The map is live rather than a startup-only snapshot because
/// imports can add servers after the agent has started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationStatus {
    /// The server has been placed in the registry but its world data has
    /// not yet passed reconciliation. Selection and mutation stay closed.
    Reconciling,
    /// Startup reconciliation completed for this server; world/backup
    /// mutation routes are reachable.
    Ready,
    /// Startup reconciliation (or restart-transaction recovery) failed
    /// for this server. `reason` is the first failure encountered, for
    /// operator diagnosis. Every world/backup mutation route must refuse
    /// this server with one structured error instead of running.
    Degraded { reason: String },
}

/// The idempotent P6.1 world/`world_slots` handoff
/// (`msc_application::worlds::reconcile_imported_worlds`), followed by
/// P6.13/P6.18/P6.33's interrupted activation, restore, and active-world
/// replacement recovery —
/// run once per registered server, in that order, before this registry
/// (and therefore any world/backup mutation route built over it) becomes
/// reachable. **Corrected post-gate-review:** a failure here used to be
/// logged and then silently ignored, leaving every mutation route for
/// that server reachable against unreconciled — possibly unsafe — disk
/// state. Now the first failure for a server (from either stage) is
/// recorded as [`ReconciliationStatus::Degraded`] and returned to the
/// caller, who threads it into every world/backup mutation route's guard
/// (`routes/worlds.rs`'s `active_server_or_response`, `routes/
/// backups.rs`'s `active_server_or_response`). The agent itself still
/// comes up — a damaged server does not block startup, and other,
/// healthy servers are entirely unaffected — matching the "keep the
/// agent available for diagnosis" requirement.
fn reconcile_server(server: &ConfigServer) -> ReconciliationStatus {
    let mut first_failure = None;
    let now = iso8601_now();
    let server_dir = Path::new(&server.server_dir);
    if let Err(err) = msc_application::worlds::reconcile_imported_worlds(
        &StdFileSystem,
        server_dir,
        server.server_type,
        None,
        &now,
    ) {
        eprintln!(
            "[worlds] Warning: could not reconcile imported world data for {}: {err}",
            server.server_dir
        );
        first_failure = Some(format!("world reconciliation failed: {err}"));
    }
    if let Err(err) =
        msc_application::worlds::reconcile_interrupted_activation(&StdFileSystem, server_dir, &now)
    {
        eprintln!(
            "[worlds] Warning: could not reconcile an interrupted activation for {}: {err}",
            server.server_dir
        );
        first_failure
            .get_or_insert_with(|| format!("interrupted activation recovery failed: {err}"));
    }
    if let Err(err) =
        msc_application::backups::reconcile_interrupted_restore(&StdFileSystem, server_dir)
    {
        eprintln!(
            "[worlds] Warning: could not reconcile an interrupted restore for {}: {err}",
            server.server_dir
        );
        first_failure.get_or_insert_with(|| format!("interrupted restore recovery failed: {err}"));
    }
    if let Err(err) = msc_application::worlds::reconcile_interrupted_world_replace(
        &StdFileSystem,
        server_dir,
        server.server_type,
    ) {
        eprintln!(
            "[worlds] Warning: could not reconcile an interrupted active-world replacement for {}: {err}",
            server.server_dir
        );
        first_failure
            .get_or_insert_with(|| format!("interrupted world replacement recovery failed: {err}"));
    }

    first_failure.map_or(ReconciliationStatus::Ready, |reason| {
        ReconciliationStatus::Degraded { reason }
    })
}

fn reconcile_servers_at_startup(
    servers: &[ConfigServer],
) -> BTreeMap<String, ReconciliationStatus> {
    servers
        .iter()
        .map(|server| (server.id.clone(), reconcile_server(server)))
        .collect()
}

/// `MSC2_AUDIT_LOG_DIR`-overridable, mirroring
/// `OperationsState::default_journaled`'s `MSC2_OPERATION_JOURNAL_DIR`
/// pattern — the Phase 6 world/backup mutation audit trail
/// (`routes/worlds.rs`/`routes/backups.rs`) lives alongside the
/// operation journal by default, not inside a server directory.
fn audit_log_dir() -> PathBuf {
    std::env::var_os("MSC2_AUDIT_LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("msc2-audit-log"))
}

/// A plain `yyyy-MM-dd'T'HH:mm:ss'Z'` timestamp, matching `WorldSlot`'s
/// own `.iso8601` encoding strategy (no fractional seconds — unlike
/// `msc-infrastructure::audit_log`'s own ISO-8601 formatter, which
/// deliberately includes milliseconds to match a *different* source
/// formatter). [`civil_from_days`] is duplicated from `audit_log`'s own
/// private copy of the same public-domain algorithm rather than exposed
/// across the crate boundary for this one call site.
fn iso8601_now() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = duration.as_secs() as i64;
    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days` (public domain,
/// <http://howardhinnant.github.io/date_algorithms.html>) — see
/// `msc-infrastructure::audit_log`'s own copy for the full derivation
/// notes.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

#[derive(Clone)]
pub struct LifecycleRoutesState {
    inner: Arc<LifecycleRoutesInner>,
}

/// The lifecycle route owns the Minecraft process, while networking owns the
/// long-lived Playit service instances.  This small callback seam lets the
/// process owner start and stop the helper without duplicating that state in
/// the route layer.
pub trait PlayitLifecycleIntegration: Send + Sync {
    fn start_for_server(&self, server: &ConfigServer);
    fn stop_for_server(&self, server_id: &str);
    fn stop_broadcast_for_server(&self, server_id: &str);
    fn stop_all(&self);
}

struct LifecycleRoutesInner {
    registry: &'static AgentServerRegistry,
    app_config: &'static AgentAppConfigStore,
    process: &'static (dyn ProcessSupervisor + Send + Sync),
    console: &'static AgentConsoleSink,
    metrics: PsProcessMetricsProvider,
    lifecycle: Mutex<LifecycleService<'static>>,
    operations: OperationsState,
    notifications: NotificationState,
    active_lifecycle_operation: Mutex<Option<OperationId>>,
    bedrock_active_server_id: Mutex<Option<String>>,
    pump_tasks: Mutex<Vec<JoinHandle<()>>>,
    auth_state: Option<AuthState>,
    audit_log: &'static AuditLog<'static>,
    reconciliation: Mutex<BTreeMap<String, ReconciliationStatus>>,
    bedrock_runtime: BedrockRuntimeSelection,
    first_start: Mutex<Option<FirstStartCoordinator>>,
    first_start_pass_two_started_at: Mutex<Option<Instant>>,
    playit: Mutex<Option<Arc<dyn PlayitLifecycleIntegration>>>,
}

pub struct AgentServerRegistry {
    app_config: &'static AgentAppConfigStore,
}

pub struct AgentAppConfigStore {
    fs: &'static dyn FileSystem,
    path: PathBuf,
    config: Mutex<AppConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentAppConfigError {
    Load(String),
    Save(String),
}

#[derive(Debug)]
pub enum ActiveServerSelectionError {
    Lifecycle(LifecycleError),
    Reconciliation { reason: String },
}

pub struct AgentConsoleSink {
    console: ConsoleState,
}

impl LifecycleRoutesState {
    #[allow(dead_code)]
    pub fn new(console_state: ConsoleState, operations: OperationsState) -> Self {
        let app_config = Box::leak(Box::new(
            AgentAppConfigStore::production()
                .expect("failed to load durable MSC 2 application config"),
        ));
        Self::with_dependencies(
            console_state,
            operations,
            app_config,
            default_process_supervisor(),
            None,
            BedrockRuntimeSelection::unavailable_for_tests(),
            NotificationState::default(),
        )
    }

    #[allow(dead_code)]
    pub fn new_migrating_legacy_secrets(
        console_state: ConsoleState,
        operations: OperationsState,
        secrets: &dyn SecretStore,
    ) -> Self {
        let app_config = Box::leak(Box::new(
            AgentAppConfigStore::production_migrating_legacy_secrets(secrets)
                .expect("failed to load durable MSC 2 application config"),
        ));
        Self::with_dependencies(
            console_state,
            operations,
            app_config,
            default_process_supervisor(),
            None,
            BedrockRuntimeSelection::unavailable_for_tests(),
            NotificationState::default(),
        )
    }

    #[allow(dead_code)]
    pub fn with_app_config_and_auth(
        console_state: ConsoleState,
        operations: OperationsState,
        app_config: &'static AgentAppConfigStore,
        auth_state: AuthState,
    ) -> Self {
        Self::with_app_config_and_auth_and_bedrock(
            console_state,
            operations,
            app_config,
            auth_state,
            BedrockRuntimeSelection::unavailable_for_tests(),
        )
    }

    pub fn with_app_config_and_auth_and_bedrock(
        console_state: ConsoleState,
        operations: OperationsState,
        app_config: &'static AgentAppConfigStore,
        auth_state: AuthState,
        bedrock_runtime: BedrockRuntimeSelection,
    ) -> Self {
        Self::with_app_config_and_auth_and_bedrock_and_notifications(
            console_state,
            operations,
            app_config,
            auth_state,
            bedrock_runtime,
            NotificationState::default(),
        )
    }

    pub fn with_app_config_and_auth_and_bedrock_and_notifications(
        console_state: ConsoleState,
        operations: OperationsState,
        app_config: &'static AgentAppConfigStore,
        auth_state: AuthState,
        bedrock_runtime: BedrockRuntimeSelection,
        notifications: NotificationState,
    ) -> Self {
        Self::with_dependencies(
            console_state,
            operations,
            app_config,
            default_process_supervisor(),
            Some(auth_state),
            bedrock_runtime,
            notifications,
        )
    }

    fn with_dependencies(
        console_state: ConsoleState,
        operations: OperationsState,
        app_config: &'static AgentAppConfigStore,
        process: Box<dyn ProcessSupervisor + Send + Sync>,
        auth_state: Option<AuthState>,
        bedrock_runtime: BedrockRuntimeSelection,
        notifications: NotificationState,
    ) -> Self {
        let reconciliation = reconcile_servers_at_startup(&app_config.servers());

        let audit_log: &'static AuditLog<'static> = Box::leak(Box::new(AuditLog::new(
            Box::leak(Box::new(StdFileSystem)),
            audit_log_dir(),
        )));
        let _ = std::fs::create_dir_all(audit_log_dir());

        let registry = Box::leak(Box::new(AgentServerRegistry::new(app_config)));
        let process = Box::leak(process);
        let console = Box::leak(Box::new(AgentConsoleSink {
            console: console_state,
        }));
        let fs: &'static dyn FileSystem = Box::leak(Box::new(StdFileSystem));
        let mut lifecycle = LifecycleService::new(registry, process, console, fs);
        let initial_bedrock_active_server_id = app_config.active_server_id().filter(|active| {
            app_config
                .servers()
                .into_iter()
                .any(|server| server.id == *active && server.server_type == ServerType::Bedrock)
        });
        if let Some(active) = app_config.active_server_id()
            && matches!(
                reconciliation.get(&active),
                Some(ReconciliationStatus::Ready)
            )
            && initial_bedrock_active_server_id.is_none()
        {
            let _ = lifecycle.select_active_server(ServerId::new(active));
        }

        Self {
            inner: Arc::new(LifecycleRoutesInner {
                registry,
                app_config,
                process,
                console,
                metrics: PsProcessMetricsProvider::default(),
                lifecycle: Mutex::new(lifecycle),
                operations,
                notifications,
                active_lifecycle_operation: Mutex::new(None),
                bedrock_active_server_id: Mutex::new(initial_bedrock_active_server_id),
                pump_tasks: Mutex::new(Vec::new()),
                auth_state,
                audit_log,
                reconciliation: Mutex::new(reconciliation),
                bedrock_runtime,
                first_start: Mutex::new(None),
                first_start_pass_two_started_at: Mutex::new(None),
                playit: Mutex::new(None),
            }),
        }
    }

    #[cfg(test)]
    pub fn with_fake_process(console_state: ConsoleState, operations: OperationsState) -> Self {
        Self::with_fake_process_capturing_supervisor(console_state, operations).0
    }

    /// [`with_fake_process`]'s own sibling, for a test that needs to keep
    /// driving the fake supervisor after construction -- see
    /// [`with_fake_process_and_app_config_capturing_supervisor`]'s own doc
    /// for why `process_supervisor()` alone can't do this.
    #[cfg(test)]
    pub fn with_fake_process_capturing_supervisor(
        console_state: ConsoleState,
        operations: OperationsState,
    ) -> (
        Self,
        &'static msc_infrastructure::process::FakeProcessSupervisor,
    ) {
        let test_root = std::env::temp_dir().join(format!(
            "msc2-agent-test-state-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let config_parent = test_root.join("data");
        let config_path = config_parent.join("server_config_swift.json");
        let servers_root = test_root.join("servers");
        let fs = Box::leak(Box::new(
            msc_infrastructure::fs::FakeFileSystem::new()
                .with_dir(config_parent.clone())
                .with_dir(servers_root.clone()),
        ));
        let app_config = Box::leak(Box::new(
            AgentAppConfigStore::load(fs, config_path, servers_root).unwrap(),
        ));
        Self::with_fake_process_and_app_config_capturing_supervisor(
            console_state,
            operations,
            app_config,
        )
    }

    #[cfg(test)]
    pub fn with_fake_process_and_app_config(
        console_state: ConsoleState,
        operations: OperationsState,
        app_config: &'static AgentAppConfigStore,
    ) -> Self {
        Self::with_fake_process_and_app_config_capturing_supervisor(
            console_state,
            operations,
            app_config,
        )
        .0
    }

    /// [`with_fake_process_and_app_config`]'s own sibling, for a test that
    /// needs to keep driving the fake supervisor after construction (e.g.
    /// P7.31's own `-version` probe, which has no automatic responder) --
    /// `process_supervisor()` alone can't do this, since it only ever
    /// hands back the type-erased `dyn ProcessSupervisor` trait object,
    /// not `FakeProcessSupervisor`'s own inherent test-double methods.
    #[cfg(test)]
    pub fn with_fake_process_and_app_config_capturing_supervisor(
        console_state: ConsoleState,
        operations: OperationsState,
        app_config: &'static AgentAppConfigStore,
    ) -> (
        Self,
        &'static msc_infrastructure::process::FakeProcessSupervisor,
    ) {
        let supervisor: &'static msc_infrastructure::process::FakeProcessSupervisor = Box::leak(
            Box::new(msc_infrastructure::process::FakeProcessSupervisor::new()),
        );
        let state = Self::with_dependencies(
            console_state,
            operations,
            app_config,
            Box::new(supervisor),
            None,
            BedrockRuntimeSelection::unavailable_for_tests(),
            NotificationState::default(),
        );
        (state, supervisor)
    }

    #[cfg(test)]
    pub fn register_imported_paper(
        &self,
        server: ImportedPaperServer,
    ) -> Result<(), AgentAppConfigError> {
        let server_id = server.id.as_str().to_string();
        self.inner
            .reconciliation
            .lock()
            .unwrap()
            .insert(server_id.clone(), ReconciliationStatus::Reconciling);
        if let Err(error) = self.inner.registry.insert(server) {
            self.inner.reconciliation.lock().unwrap().remove(&server_id);
            return Err(error);
        }
        self.inner
            .reconciliation
            .lock()
            .unwrap()
            .insert(server_id, ReconciliationStatus::Ready);
        Ok(())
    }

    pub fn begin_import_operation(
        &self,
        source_path: &str,
    ) -> Result<OperationId, msc_application::operations::LifecycleOperationError> {
        self.inner.operations.begin_lifecycle(
            "paper-import",
            Some(source_path.to_string()),
            "Importing Paper server.",
        )
    }

    pub fn finish_operation_success(
        &self,
        operation_id: &OperationId,
        status_line: &str,
        result: BTreeMap<String, String>,
    ) -> Result<(), msc_application::operations::LifecycleOperationError> {
        self.inner
            .operations
            .succeed(operation_id, status_line, result)
    }

    pub fn finish_operation_failure(
        &self,
        operation_id: &OperationId,
        code: &str,
        message: String,
    ) -> Result<(), msc_application::operations::LifecycleOperationError> {
        self.inner.operations.fail(operation_id, code, message)
    }

    pub fn servers(&self) -> Vec<RegisteredServerDtoParts> {
        self.inner.registry.list()
    }

    #[cfg(test)]
    pub fn config_servers(&self) -> Vec<ConfigServer> {
        self.inner.app_config.servers()
    }

    pub fn servers_root(&self) -> PathBuf {
        self.inner.app_config.servers_root()
    }

    pub fn app_config_path(&self) -> PathBuf {
        self.inner.app_config.path.clone()
    }

    pub fn reset_after_host_reset(&self) {
        self.stop_all_playit_helpers();
        self.inner.app_config.reset_in_memory();
        self.inner.lifecycle.lock().unwrap().clear_selection();
        *self.inner.bedrock_active_server_id.lock().unwrap() = None;
        self.inner.reconciliation.lock().unwrap().clear();
    }

    #[cfg(test)]
    pub fn merge_config_servers(
        &self,
        new_servers: Vec<ConfigServer>,
    ) -> Result<(), AgentAppConfigError> {
        self.register_imported_config_servers(new_servers, false)
            .map(|_| ())
    }

    /// Persist newly imported servers while their mutation authority is
    /// closed, then run the same reconciliation/recovery sequence used at
    /// startup. A failed reconciliation deliberately does not roll back
    /// registration: the server remains visible for diagnosis as
    /// `Degraded`.
    pub fn register_imported_config_servers(
        &self,
        new_servers: Vec<ConfigServer>,
        replace_all: bool,
    ) -> Result<Vec<(String, ReconciliationStatus)>, AgentAppConfigError> {
        let previous_statuses = self.inner.reconciliation.lock().unwrap().clone();
        {
            let mut statuses = self.inner.reconciliation.lock().unwrap();
            for server in &new_servers {
                statuses.insert(server.id.clone(), ReconciliationStatus::Reconciling);
            }
        }

        let save_result = if replace_all {
            self.inner.app_config.replace_servers(new_servers.clone())
        } else {
            self.inner.app_config.merge_servers(new_servers.clone())
        };
        if let Err(error) = save_result {
            *self.inner.reconciliation.lock().unwrap() = previous_statuses;
            return Err(error);
        }

        let mut outcomes = Vec::with_capacity(new_servers.len());
        if replace_all {
            self.inner
                .reconciliation
                .lock()
                .unwrap()
                .retain(|id, _| new_servers.iter().any(|server| server.id == *id));
        }
        for server in &new_servers {
            let status = reconcile_server(server);
            self.inner
                .reconciliation
                .lock()
                .unwrap()
                .insert(server.id.clone(), status.clone());
            outcomes.push((server.id.clone(), status));
        }
        Ok(outcomes)
    }

    pub fn existing_java_ports(&self) -> Vec<i64> {
        self.inner.app_config.existing_java_ports()
    }

    pub fn existing_bedrock_ports(&self) -> Vec<i64> {
        self.inner.app_config.existing_bedrock_ports()
    }

    pub fn export_inputs(&self) -> Vec<TransferExportServerInput> {
        self.inner.app_config.export_inputs()
    }

    pub fn wipe_replace_all_secrets(&self, previous_server_ids: &[String]) -> Result<(), String> {
        if let Some(auth_state) = &self.inner.auth_state {
            auth_state
                .wipe_replace_all_secrets(previous_server_ids)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn active_server_id(&self) -> Option<String> {
        self.inner
            .bedrock_active_server_id
            .lock()
            .unwrap()
            .clone()
            .or_else(|| {
                self.inner
                    .lifecycle
                    .lock()
                    .unwrap()
                    .active_server()
                    .map(|id| id.as_str().to_string())
            })
    }

    pub fn update_duckdns_hostname(
        &self,
        hostname: Option<String>,
    ) -> Result<(), AgentAppConfigError> {
        self.inner.app_config.update_duckdns_hostname(hostname)
    }

    /// A clone of the shared operation store — P6.21's world/backup
    /// routes journal every mutation through the same
    /// `OperationJournal::admit` per-target exclusivity mechanism
    /// `start_active_server` already uses, so they need their own handle
    /// on it rather than routing every operation call back through this
    /// type.
    pub fn operations(&self) -> OperationsState {
        self.inner.operations.clone()
    }

    /// The real, platform-selected `ProcessSupervisor` this agent starts
    /// Java servers with — needed by Phase 7's provisioning routes
    /// (`routes/servers.rs`) to run a Forge/NeoForge installer as a real
    /// supervised subprocess, the same handle `LifecycleService` itself
    /// already runs against.
    pub fn process_supervisor(&self) -> &'static (dyn ProcessSupervisor + Send + Sync) {
        self.inner.process
    }

    /// Connects the networking-owned Playit services after both route states
    /// have been constructed.  Keeping the registration late avoids a cycle
    /// in the application state while still giving lifecycle the only place
    /// that knows when a Minecraft process has started or ended.
    pub fn register_playit_lifecycle(&self, integration: Arc<dyn PlayitLifecycleIntegration>) {
        *self.inner.playit.lock().unwrap() = Some(integration);
    }

    fn playit_lifecycle(&self) -> Option<Arc<dyn PlayitLifecycleIntegration>> {
        self.inner.playit.lock().unwrap().clone()
    }

    fn start_playit_if_allowed(&self, server: &ConfigServer) {
        let allowed = {
            let first_start = self.inner.first_start.lock().unwrap();
            match first_start.as_ref() {
                Some(run) if run.server_id == server.id => run.phase == FirstStartPhase::PassTwo,
                Some(_) => false,
                None => true,
            }
        };
        if allowed && let Some(integration) = self.playit_lifecycle() {
            integration.start_for_server(server);
        }
    }

    fn stop_helpers_for_server(&self, server_id: &str) {
        if let Some(integration) = self.playit_lifecycle() {
            integration.stop_for_server(server_id);
            integration.stop_broadcast_for_server(server_id);
        }
    }

    /// Stop the backend that actually owns the first-start process. The Java
    /// lifecycle service is still the right owner for Java, but Bedrock lives
    /// in the selected runtime (including the macOS VM sidecar).
    fn stop_bedrock_runtime_if_needed(&self) -> Result<(), BedrockRuntimeError> {
        match self.inner.bedrock_runtime.state() {
            BedrockRuntimeState::Starting | BedrockRuntimeState::Running => {
                self.inner.bedrock_runtime.stop()
            }
            // The first-start pump is already responsible for observing the
            // resulting termination event. A repeated stop must not replace
            // the operation or turn an in-flight shutdown into an error.
            BedrockRuntimeState::Stopping | BedrockRuntimeState::Stopped => Ok(()),
            state => Err(BedrockRuntimeError::InvalidState {
                operation: "stop",
                state,
            }),
        }
    }

    fn stop_first_start_server(&self, server_id: &str) {
        self.stop_helpers_for_server(server_id);
        if self
            .active_bedrock_server()
            .is_some_and(|server| server.id == server_id)
        {
            if let Err(error) = self.stop_bedrock_runtime_if_needed() {
                self.abort_first_start();
                self.finish_active_lifecycle_operation_failure(&error.to_string());
            }
        } else {
            let _ = self.inner.lifecycle.lock().unwrap().request_stop();
        }
    }

    fn stop_all_playit_helpers(&self) {
        if let Some(integration) = self.playit_lifecycle() {
            integration.stop_all();
        }
    }

    /// The full application config, cloned — Phase 7's provisioning/
    /// version/template/java-runtime routes read `paper_template_dir`/
    /// `plugin_template_dir`/`java_path`/`save_downloaded_jars`/
    /// `default_banner_color_hex` off it directly rather than this type
    /// growing one accessor per field.
    pub fn app_config_snapshot(&self) -> AppConfig {
        self.inner.app_config.snapshot()
    }

    pub fn bedrock_runtime_state(&self) -> msc_api::dto::BedrockRuntimeStateDto {
        self.inner.bedrock_runtime.state_dto()
    }

    pub(crate) fn bedrock_runtime_is_busy(&self) -> bool {
        matches!(
            self.inner.bedrock_runtime.state(),
            msc_application::bedrock_runtime::BedrockRuntimeState::Starting
                | msc_application::bedrock_runtime::BedrockRuntimeState::Running
                | msc_application::bedrock_runtime::BedrockRuntimeState::Stopping
        )
    }

    pub(crate) fn bedrock_runtime_is_bound(&self) -> bool {
        self.inner.bedrock_runtime.is_bound()
    }

    /// Applies `update` to the durable `AppConfig` and persists it,
    /// exactly like [`AgentAppConfigStore::mutate`] but for a caller
    /// whose own mutation can itself fail (delete/rename/version-change/
    /// RAM edits — every non-import fleet mutation this phase adds) —
    /// see [`TryMutateError`]'s own doc for why a failed `update` never
    /// reaches the save step at all.
    pub fn try_mutate_config<T, E>(
        &self,
        update: impl FnOnce(&mut AppConfig) -> Result<T, E>,
    ) -> Result<T, TryMutateError<E>> {
        self.inner.app_config.try_mutate(update)
    }

    /// `fleet::delete_server` plus the bookkeeping the domain function
    /// itself can't see: the running-server guard (this type's own
    /// `active_server_id`/`status_snapshot`, not `AppConfig`'s), removing
    /// the deleted id from the live reconciliation map, and re-selecting
    /// whichever server `AppConfig.active_server_id` now names, if any.
    ///
    /// **Known gap, not silently worked around:** when the deleted server
    /// was the *only* server, `fleet::delete_server` correctly leaves
    /// `AppConfig.active_server_id` as `None`, but `LifecycleService`
    /// (Phase 4) has no "deselect" primitive — only `select_active_server`
    /// — so its own in-memory active pointer keeps naming the now-deleted
    /// id until the next real selection. `AgentServerRegistry::get` looks
    /// up fresh from `AppConfig` on every call, so this degrades to a
    /// "not found" style failure rather than a crash, but a real deselect
    /// primitive belongs to whichever later step touches `lifecycle.rs`
    /// (Phase 4's `LifecycleService`) next.
    pub fn delete_fleet_server(
        &self,
        server_id: &str,
    ) -> Result<
        msc_application::fleet::DeletedServer,
        TryMutateError<msc_application::fleet::DeleteServerError>,
    > {
        let is_active_and_running =
            self.active_server_id().as_deref() == Some(server_id) && self.status_snapshot().running;
        let deleted = self.inner.app_config.try_mutate(|config| {
            msc_application::fleet::delete_server(
                &StdFileSystem,
                config,
                server_id,
                is_active_and_running,
            )
        })?;
        self.inner.reconciliation.lock().unwrap().remove(server_id);
        if let Some(new_active) = &deleted.new_active_server_id {
            let _ = self.select_active_server(new_active.clone());
        }
        Ok(deleted)
    }

    pub fn rename_fleet_server(
        &self,
        server_id: &str,
        new_name: &str,
    ) -> Result<(), TryMutateError<msc_application::fleet::RenameServerError>> {
        self.inner
            .app_config
            .try_mutate(|config| msc_application::fleet::rename_server(config, server_id, new_name))
    }

    pub fn update_server_directory(
        &self,
        server_id: &str,
        directory: &str,
    ) -> Result<(), TryMutateError<msc_application::fleet::UpdateServerDirectoryError>> {
        self.inner.app_config.try_mutate(|config| {
            msc_application::fleet::update_server_directory(config, server_id, directory)
        })
    }

    /// See [`AgentConsoleSink`] / `ConsoleState::recent_lines` — the
    /// production `BackupConsole`'s read half.
    pub fn recent_console_lines(&self, count: usize) -> Vec<ConsoleLine> {
        self.inner.console.console.recent_lines(count)
    }

    /// Appends output from a managed helper to the same bounded stream used
    /// by the server console. This keeps helper diagnostics visible in both
    /// the main Console view and any first-start surface reading its tail.
    pub fn append_console_line(&self, source: &str, line: &str) {
        self.inner
            .console
            .push(ConsoleLine::new(source, None, line.to_string()));
    }

    /// The shared Phase 6 mutation audit log — one `AuditLog` instance,
    /// scoped to world/backup mutation routes only (see
    /// `routes/worlds.rs`/`routes/backups.rs`'s own doc comments for why
    /// this doesn't extend to every route in this agent yet).
    pub fn audit_log(&self) -> &'static AuditLog<'static> {
        self.inner.audit_log
    }

    /// Every `ConfigServer` currently on file — the production
    /// counterpart of the existing `#[cfg(test)] config_servers`, needed
    /// by `routes/backups.rs` to read/update a server's auto-backup
    /// settings (`ConfigServer::auto_backup_*`) outside of tests.
    pub fn app_config_servers(&self) -> Vec<ConfigServer> {
        self.inner.app_config.servers()
    }

    /// The currently-active server's full `ConfigServer` record — what
    /// every P6.21 world/backup route needs (`server_dir`, `server_type`,
    /// the auto-backup fields) beyond the narrower
    /// `RegisteredServerDtoParts` [`Self::servers`] already returns.
    /// `None` if no server is active, matching every other
    /// `no_active_server` guard already in this codebase.
    pub fn active_config_server(&self) -> Option<ConfigServer> {
        let active_id = self.active_server_id()?;
        self.app_config_servers()
            .into_iter()
            .find(|server| server.id == active_id)
    }

    /// This server's reconciliation outcome. An absent id fails closed:
    /// callers must never gain mutation authority merely because no state
    /// was recorded for it.
    pub fn reconciliation_status(&self, server_id: &str) -> ReconciliationStatus {
        self.inner
            .reconciliation
            .lock()
            .unwrap()
            .get(server_id)
            .cloned()
            .unwrap_or_else(|| ReconciliationStatus::Degraded {
                reason: "no reconciliation state exists for this server".to_string(),
            })
    }

    #[cfg(test)]
    pub fn set_reconciliation_status(
        &self,
        server_id: impl Into<String>,
        status: ReconciliationStatus,
    ) {
        self.inner
            .reconciliation
            .lock()
            .unwrap()
            .insert(server_id.into(), status);
    }

    /// Updates exactly the three auto-backup fields on the `ConfigServer`
    /// named `server_id`, leaving every other server and every other
    /// field of this one untouched — unlike `replace_config_servers`,
    /// which also resets `active_server_id`, a side effect `POST
    /// /v1/backups/config` has no business triggering.
    pub fn update_backup_config(
        &self,
        server_id: &str,
        enabled: Option<bool>,
        interval_minutes: Option<i64>,
        max_count: Option<i64>,
    ) -> Result<ConfigServer, AgentAppConfigError> {
        self.inner
            .app_config
            .update_backup_config(server_id, enabled, interval_minutes, max_count)
    }

    pub fn select_active_server(
        &self,
        server_id: String,
    ) -> Result<String, ActiveServerSelectionError> {
        match self.reconciliation_status(&server_id) {
            ReconciliationStatus::Ready => {}
            ReconciliationStatus::Reconciling => {
                return Err(ActiveServerSelectionError::Reconciliation {
                    reason: "world reconciliation is still in progress".to_string(),
                });
            }
            ReconciliationStatus::Degraded { reason } => {
                return Err(ActiveServerSelectionError::Reconciliation { reason });
            }
        }
        let server = self
            .inner
            .registry
            .get(&ServerId::new(server_id.clone()))
            .ok_or_else(|| {
                ActiveServerSelectionError::Lifecycle(LifecycleError::ServerNotFound(
                    ServerId::new(server_id.clone()),
                ))
            })?;
        let previous_server_id = self.active_server_id();
        let changing_active_server = previous_server_id.as_deref() != Some(server_id.as_str());
        if changing_active_server
            && (self.status_snapshot().running
                || self
                    .inner
                    .bedrock_runtime
                    .is_bound_to_other_server(&server.server_dir))
        {
            return Err(ActiveServerSelectionError::Lifecycle(
                LifecycleError::AlreadyInState(LifecycleState::Running),
            ));
        }
        if changing_active_server && let Some(previous_server_id) = previous_server_id.as_deref() {
            self.stop_helpers_for_server(previous_server_id);
        }
        if server.server_type == ServerType::Bedrock {
            self.inner
                .bedrock_runtime
                .refresh_for_server(&server.server_dir);
            self.inner
                .app_config
                .set_active_server_id(Some(server_id.clone()))
                .map_err(|error| {
                    ActiveServerSelectionError::Lifecycle(LifecycleError::Repository(
                        error.to_string(),
                    ))
                })?;
            *self.inner.bedrock_active_server_id.lock().unwrap() = Some(server_id.clone());
            return Ok(server_id);
        }
        *self.inner.bedrock_active_server_id.lock().unwrap() = None;
        let id = ServerId::new(server_id);
        let active = id.as_str().to_string();
        self.inner
            .lifecycle
            .lock()
            .unwrap()
            .select_active_server(id)
            .map_err(ActiveServerSelectionError::Lifecycle)?;
        self.inner
            .app_config
            .set_active_server_id(Some(active.clone()))
            .map_err(|error| {
                ActiveServerSelectionError::Lifecycle(LifecycleError::Repository(error.to_string()))
            })?;
        Ok(active)
    }

    #[allow(clippy::result_large_err)]
    pub fn start_active_server(&self) -> Result<LifecycleActionResult, LifecycleRouteError> {
        self.drain_active_process_events();
        if self.active_bedrock_server().is_some() {
            return self.start_active_bedrock_server();
        }
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
        self.prepare_first_start(&registered)?;
        let operation_id = self.inner.operations.begin_lifecycle(
            "java-start",
            Some(active.as_str().to_string()),
            "Starting Java server.",
        )?;
        // P7.31: resolve the effective Java executable and run the
        // required-major guard before spawning it, the start-time
        // counterpart to `create_install_step_server`'s own create-time
        // guard call. Global `cfg.java_path` only -- MSC 1 has no
        // persisted per-server override to resolve here (see
        // `msc_domain::java_runtime::resolve_create_time_java_path`'s own
        // doc); `MSC2_JAVA_PATH` stays a same-priority override on top of
        // it so `tools/phase5/cli-smoke.sh`'s fake-java test hook keeps
        // working unchanged.
        let configured_java_path = self.app_config_snapshot().java_path;
        let java_path = std::env::var("MSC2_JAVA_PATH").unwrap_or(configured_java_path);
        let probe =
            java_runtime_detection::run_java_version_probe(self.process_supervisor(), &java_path);
        if let Err(unusable) = msc_domain::java_runtime::evaluate_java_runtime_guard(
            &java_path,
            registered.minecraft_version.as_deref(),
            &probe,
        ) {
            let message = unusable.to_string();
            msc_application::diagnostics::record_startup_failure(
                &StdFileSystem,
                Path::new(&registered.server_dir),
                &iso8601_now(),
                &message,
                &[],
            );
            let _ = self
                .inner
                .operations
                .fail(&operation_id, "unusable_java_runtime", message);
            return Err(LifecycleRouteError::UnusableJavaRuntime(unusable));
        }

        let launch = build_launch_request(&registered, &java_path).inspect_err(|error| {
            let message = error.to_string();
            msc_application::diagnostics::record_startup_failure(
                &StdFileSystem,
                Path::new(&registered.server_dir),
                &iso8601_now(),
                &message,
                &[],
            );
            let _ = self
                .inner
                .operations
                .fail(&operation_id, "lifecycle_error", message);
        })?;
        let pid = match self
            .inner
            .lifecycle
            .lock()
            .unwrap()
            .start_active_server(launch)
        {
            Ok(pid) => pid,
            Err(error) => {
                let message = error.to_string();
                msc_application::diagnostics::record_startup_failure(
                    &StdFileSystem,
                    Path::new(&registered.server_dir),
                    &iso8601_now(),
                    &message,
                    &[],
                );
                let _ = self
                    .inner
                    .operations
                    .fail(&operation_id, "lifecycle_error", message);
                return Err(error.into());
            }
        };
        let _ = self
            .inner
            .operations
            .progress(&operation_id, 1, 2, "Java process spawned.");
        *self.inner.active_lifecycle_operation.lock().unwrap() = Some(operation_id.clone());
        self.spawn_process_pump(pid);
        self.start_playit_if_allowed(&registered);
        Ok(LifecycleActionResult {
            active_server_id: Some(active.as_str().to_string()),
            operation_id: Some(operation_id.as_str().to_string()),
        })
    }

    /// Arm or advance MSC 1's first-start state before a process is spawned.
    /// A fresh/legacy server enters pass one; the next explicit Start after
    /// pass one enters pass two. The persisted popup flag is deliberately not
    /// used to suppress an in-flight run, so a crash can be retried safely.
    fn prepare_first_start(&self, server: &ConfigServer) -> Result<(), LifecycleError> {
        let mut first_start = self.inner.first_start.lock().unwrap();
        match first_start.as_mut() {
            Some(run) if run.server_id == server.id => {
                if run.phase == FirstStartPhase::WaitingForSetup {
                    run.begin_second_pass();
                    *self.inner.first_start_pass_two_started_at.lock().unwrap() =
                        Some(Instant::now());
                }
            }
            Some(_) => {
                *first_start = None;
            }
            None => {}
        }

        if first_start.is_none() && msc_application::provisioning::first_start_required(server) {
            *first_start = Some(FirstStartCoordinator::new(
                server.id.clone(),
                server.playit_enabled,
                server.xbox_broadcast_enabled,
            ));
            *self.inner.first_start_pass_two_started_at.lock().unwrap() = None;
            self.mark_first_start_started(&server.id)?;
        }
        Ok(())
    }

    fn mark_first_start_started(&self, server_id: &str) -> Result<(), LifecycleError> {
        self.try_mutate_config(|config| {
            let Some(server) = config
                .servers
                .iter_mut()
                .find(|server| server.id == server_id)
            else {
                return Err("server disappeared while starting".to_string());
            };
            server.has_ever_started = true;
            Ok::<_, String>(())
        })
        .map_err(|error| match error {
            TryMutateError::Domain(message) => LifecycleError::Repository(message),
            TryMutateError::Save(error) => LifecycleError::Repository(error.to_string()),
        })
    }

    fn mark_first_start_complete(&self, server_id: &str) -> Result<(), String> {
        self.try_mutate_config(|config| {
            let Some(server) = config
                .servers
                .iter_mut()
                .find(|server| server.id == server_id)
            else {
                return Err("server disappeared before first-start completion".to_string());
            };
            server.has_ever_started = true;
            server.has_shown_first_start_popup = true;
            Ok::<_, String>(())
        })
        .map_err(|error| match error {
            TryMutateError::Domain(message) => message,
            TryMutateError::Save(error) => error.to_string(),
        })
    }

    /// Called by transport integrations when their first-start work resolves.
    /// It is public so the networking route can feed the same agent-owned
    /// state machine when the normal helper lifecycle is connected later.
    #[allow(dead_code)]
    pub fn mark_first_start_transport(
        &self,
        transport: FirstRunTransport,
        state: FirstStartTransportState,
    ) -> bool {
        self.mark_first_start_transport_for_server_inner(None, transport, state)
    }

    /// Records a helper result only when it belongs to the first-start run
    /// currently being coordinated.  The server id matters because a stale
    /// helper from another server must not complete this server's setup.
    pub fn mark_first_start_transport_for_server(
        &self,
        server_id: &str,
        transport: FirstRunTransport,
        state: FirstStartTransportState,
    ) -> bool {
        self.mark_first_start_transport_for_server_inner(Some(server_id), transport, state)
    }

    fn mark_first_start_transport_for_server_inner(
        &self,
        server_id: Option<&str>,
        transport: FirstRunTransport,
        state: FirstStartTransportState,
    ) -> bool {
        let should_stop = {
            let mut first_start = self.inner.first_start.lock().unwrap();
            let Some(run) = first_start.as_mut() else {
                return false;
            };
            if server_id.is_some_and(|server_id| run.server_id != server_id) {
                return false;
            }
            let run_server_id = run.server_id.clone();
            run.mark_transport(transport, state);
            let should_stop = run.ready_to_stop() && run.issue_auto_stop();
            (run_server_id, should_stop)
        };
        if should_stop.1 {
            self.stop_first_start_server(&should_stop.0);
        }
        true
    }

    fn handle_server_ready(&self, status_line: &str) {
        let (should_stop, pass_one) = {
            let mut first_start = self.inner.first_start.lock().unwrap();
            let Some(run) = first_start.as_mut() else {
                self.finish_active_lifecycle_operation_success(status_line);
                return;
            };
            let changed = run.mark_server_ready();
            if !changed {
                return;
            }
            let should_stop = if run.phase == FirstStartPhase::PassTwo {
                run.ready_to_stop() && run.issue_auto_stop()
            } else {
                run.issue_auto_stop()
            };
            (should_stop, run.phase != FirstStartPhase::PassTwo)
        };

        if should_stop {
            if let Some(server_id) = self.active_server_id() {
                self.stop_first_start_server(&server_id);
            }
            if pass_one {
                self.progress_first_start("First-start pass one is ready; stopping the server.");
            } else {
                self.progress_first_start("First-start transports are ready; stopping the server.");
            }
        } else {
            self.progress_first_start(status_line);
        }
    }

    fn progress_first_start(&self, status_line: &str) {
        let Some(operation_id) = self
            .inner
            .active_lifecycle_operation
            .lock()
            .unwrap()
            .clone()
        else {
            return;
        };
        let _ = self
            .inner
            .operations
            .progress(&operation_id, 2, 3, status_line);
    }

    pub fn stop_active_server(&self) -> Result<Option<String>, LifecycleError> {
        self.drain_active_process_events();
        let active_server_id = self.active_server_id();
        if let Some(server_id) = active_server_id.as_deref() {
            self.stop_helpers_for_server(server_id);
        }
        self.inner.lifecycle.lock().unwrap().request_stop()?;
        Ok(active_server_id)
    }

    #[allow(clippy::result_large_err)]
    pub fn stop_active_bedrock_server(&self) -> Result<LifecycleActionResult, LifecycleRouteError> {
        let active = self
            .active_bedrock_server()
            .ok_or(LifecycleRouteError::Lifecycle(
                LifecycleError::NoActiveServer,
            ))?;

        // Pass two already owns a start operation, and FirstStartSheet polls
        // that operation even after it asks this route to stop the server.
        // Reusing it lets the existing Bedrock pump complete the first-start
        // result when the sidecar reports clean termination. Creating a new
        // bedrock-stop operation here would orphan the operation in the sheet.
        let first_start_active = self
            .inner
            .first_start
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|run| run.server_id == active.id);
        if first_start_active {
            self.stop_helpers_for_server(&active.id);
            if let Err(error) = self.stop_bedrock_runtime_if_needed() {
                self.finish_active_lifecycle_operation_failure(&error.to_string());
                return Err(self.bedrock_runtime_error(error));
            }
            self.progress_first_start("Connections are recorded. Stopping the server.");
            let operation_id = self
                .inner
                .active_lifecycle_operation
                .lock()
                .unwrap()
                .clone();
            return Ok(LifecycleActionResult {
                active_server_id: Some(active.id),
                operation_id: operation_id.map(|id| id.as_str().to_string()),
            });
        }

        self.drain_bedrock_events();
        let active = self
            .active_bedrock_server()
            .ok_or(LifecycleRouteError::Lifecycle(
                LifecycleError::NoActiveServer,
            ))?;
        let operation_id = self.inner.operations.begin_lifecycle(
            "bedrock-stop",
            Some(active.id.clone()),
            "Stopping Bedrock server.",
        )?;
        self.stop_helpers_for_server(&active.id);
        if let Err(error) = self.inner.bedrock_runtime.stop() {
            let _ =
                self.inner
                    .operations
                    .fail(&operation_id, "bedrock_stop_failed", error.to_string());
            return Err(self.bedrock_runtime_error(error));
        }
        *self.inner.active_lifecycle_operation.lock().unwrap() = Some(operation_id.clone());
        self.spawn_bedrock_pump();
        Ok(LifecycleActionResult {
            active_server_id: Some(active.id),
            operation_id: Some(operation_id.as_str().to_string()),
        })
    }

    pub fn send_command(&self, command: &str) -> Result<Option<String>, LifecycleError> {
        self.drain_active_process_events();
        self.inner.lifecycle.lock().unwrap().send_command(command)?;
        Ok(self.active_server_id())
    }

    #[allow(clippy::result_large_err)]
    pub fn send_bedrock_command(
        &self,
        command: &str,
    ) -> Result<Option<String>, LifecycleRouteError> {
        self.drain_bedrock_events();
        let active = self
            .active_bedrock_server()
            .ok_or(LifecycleRouteError::Lifecycle(
                LifecycleError::NoActiveServer,
            ))?;
        self.inner
            .bedrock_runtime
            .command(command)
            .map_err(|error| self.bedrock_runtime_error(error))?;
        Ok(Some(active.id))
    }

    pub fn status_snapshot(&self) -> LifecycleStatusSnapshot {
        if self.active_bedrock_server().is_some() {
            self.drain_bedrock_events();
            let runtime_state = self.inner.bedrock_runtime.state();
            return LifecycleStatusSnapshot {
                running: matches!(
                    runtime_state,
                    msc_application::bedrock_runtime::BedrockRuntimeState::Starting
                        | msc_application::bedrock_runtime::BedrockRuntimeState::Running
                        | msc_application::bedrock_runtime::BedrockRuntimeState::Stopping
                ),
                active_server_id: self.active_server_id(),
                pid: self
                    .inner
                    .bedrock_runtime
                    .process_id()
                    .map(|pid| pid.raw() as i64),
                server_type: Some("bedrock".to_owned()),
            };
        }
        self.drain_active_process_events();
        self.inner
            .lifecycle
            .lock()
            .unwrap()
            .status_snapshot()
            .unwrap_or_else(|_| stopped_status())
    }

    pub fn performance_snapshot(&self) -> PerformanceSnapshot {
        if self.active_bedrock_server().is_some() {
            self.drain_bedrock_events();
            let usage = self.inner.bedrock_runtime.process_id().and_then(|pid| {
                msc_infrastructure::metrics::ProcessMetricsProvider::process_usage(
                    &self.inner.metrics,
                    pid,
                )
            });
            return PerformanceSnapshot {
                ts: unix_timestamp_string(),
                tps_1m: None,
                tps_5m: None,
                tps_15m: None,
                players_online: Some(0),
                cpu_percent: usage.as_ref().and_then(|value| value.cpu_percent),
                ram_used_mb: usage.as_ref().and_then(|value| value.ram_used_mb),
                ram_max_mb: None,
                world_size_mb: None,
                server_type: Some("bedrock".to_owned()),
            };
        }
        self.drain_active_process_events();
        self.inner
            .lifecycle
            .lock()
            .unwrap()
            .performance_snapshot(&self.inner.metrics, unix_timestamp_string())
            .unwrap_or_else(|_| stopped_performance())
    }

    pub(crate) fn active_bedrock_server(&self) -> Option<ConfigServer> {
        self.active_server_id().and_then(|id| {
            self.inner
                .app_config
                .servers()
                .into_iter()
                .find(|server| server.id == id && server.server_type == ServerType::Bedrock)
        })
    }

    fn bedrock_runtime_error(&self, error: BedrockRuntimeError) -> LifecycleRouteError {
        LifecycleRouteError::BedrockRuntime {
            state: self.bedrock_runtime_state(),
            error,
        }
    }

    pub fn provision_bedrock_server(
        &self,
        server: &ConfigServer,
    ) -> Result<(), BedrockRuntimeError> {
        let version = server
            .bedrock_version
            .clone()
            .unwrap_or_else(|| "LATEST".to_owned());
        let server_dir = PathBuf::from(&server.server_dir);
        self.inner.bedrock_runtime.provision(
            BedrockProvisionRequest {
                server_dir: server.server_dir.clone(),
                version,
            },
            || bedrock_safety_backup(&server_dir),
        )
    }

    pub(crate) fn provision_imported_bedrock_servers(
        &self,
        servers: &[ConfigServer],
    ) -> Result<(), BedrockRuntimeError> {
        if self.bedrock_runtime_is_bound() {
            return Ok(());
        }
        for server in servers
            .iter()
            .filter(|server| server.server_type == ServerType::Bedrock)
        {
            self.ensure_bedrock_server(server)?;
        }
        Ok(())
    }

    pub(crate) fn ensure_bedrock_server(
        &self,
        server: &ConfigServer,
    ) -> Result<(), BedrockRuntimeError> {
        let version = server
            .bedrock_version
            .clone()
            .unwrap_or_else(|| "LATEST".to_owned());
        let server_dir = PathBuf::from(&server.server_dir);
        self.inner.bedrock_runtime.ensure_distribution(
            BedrockProvisionRequest {
                server_dir: server.server_dir.clone(),
                version,
            },
            || bedrock_safety_backup(&server_dir),
        )
    }

    #[allow(clippy::result_large_err)]
    fn start_active_bedrock_server(&self) -> Result<LifecycleActionResult, LifecycleRouteError> {
        let active = self
            .active_bedrock_server()
            .ok_or(LifecycleRouteError::Lifecycle(
                LifecycleError::NoActiveServer,
            ))?;
        self.prepare_first_start(&active)?;
        let operation_id = self.inner.operations.begin_lifecycle(
            "bedrock-start",
            Some(active.id.clone()),
            "Starting Bedrock server.",
        )?;
        let memory_gb = active.max_ram_gb.max(1.0).ceil() as u32;
        let result = self.provision_bedrock_server(&active).and_then(|()| {
            self.inner.bedrock_runtime.start(BedrockStartRequest {
                memory_gb,
                bedrock_port: active
                    .bedrock_port
                    .unwrap_or(19132)
                    .try_into()
                    .unwrap_or(19132),
            })
        });
        if let Err(error) = result {
            let message = error.to_string();
            msc_application::diagnostics::record_startup_failure(
                &StdFileSystem,
                Path::new(&active.server_dir),
                &iso8601_now(),
                &message,
                &[],
            );
            let code = if matches!(&error, BedrockRuntimeError::Provisioning(_)) {
                "bedrock_provisioning_failed"
            } else if self.bedrock_runtime_state().state != "available" {
                "capability_unavailable"
            } else {
                "bedrock_start_failed"
            };
            let _ = self.inner.operations.fail(&operation_id, code, message);
            return Err(self.bedrock_runtime_error(error));
        }
        let _ = self
            .inner
            .operations
            .progress(&operation_id, 1, 2, "Bedrock process spawned.");
        *self.inner.active_lifecycle_operation.lock().unwrap() = Some(operation_id.clone());
        self.spawn_bedrock_pump();
        self.start_playit_if_allowed(&active);
        Ok(LifecycleActionResult {
            active_server_id: Some(active.id),
            operation_id: Some(operation_id.as_str().to_string()),
        })
    }

    fn spawn_bedrock_pump(&self) {
        let state = self.clone();
        let handle = tokio::spawn(async move {
            loop {
                state.drain_bedrock_events();
                let runtime_state = state.inner.bedrock_runtime.state();
                if state.bedrock_operation_cancel_requested() {
                    match runtime_state {
                        msc_application::bedrock_runtime::BedrockRuntimeState::Starting
                        | msc_application::bedrock_runtime::BedrockRuntimeState::Running => {
                            let _ = state.inner.bedrock_runtime.stop();
                        }
                        msc_application::bedrock_runtime::BedrockRuntimeState::Stopped
                        | msc_application::bedrock_runtime::BedrockRuntimeState::Unavailable => {
                            state.finish_active_lifecycle_operation_cancelled();
                            break;
                        }
                        msc_application::bedrock_runtime::BedrockRuntimeState::New
                        | msc_application::bedrock_runtime::BedrockRuntimeState::Provisioned
                        | msc_application::bedrock_runtime::BedrockRuntimeState::Stopping => {}
                    }
                }
                if matches!(
                    runtime_state,
                    msc_application::bedrock_runtime::BedrockRuntimeState::Stopped
                        | msc_application::bedrock_runtime::BedrockRuntimeState::Unavailable
                ) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        self.inner.pump_tasks.lock().unwrap().push(handle);
    }

    fn drain_bedrock_events(&self) {
        while let Ok(Some(event)) = self.inner.bedrock_runtime.poll_event() {
            match event {
                BedrockRuntimeEvent::ConsoleLine(line) => {
                    self.inner
                        .console
                        .push(ConsoleLine::new("bedrock", None, line));
                }
                BedrockRuntimeEvent::Ready { .. } => {
                    if self.bedrock_operation_cancel_requested() {
                        let _ = self.inner.bedrock_runtime.stop();
                    } else {
                        self.handle_server_ready("Bedrock server is ready.");
                    }
                }
                BedrockRuntimeEvent::Metrics(_) => {}
                BedrockRuntimeEvent::Terminated { reason } => match reason {
                    BedrockTerminationReason::Clean => {
                        if self.bedrock_operation_cancel_requested() {
                            self.finish_active_lifecycle_operation_cancelled();
                        } else {
                            self.handle_process_termination(true);
                        }
                    }
                    BedrockTerminationReason::GuestError(message)
                    | BedrockTerminationReason::StartFailed(message) => {
                        if let Some(server_id) = self.active_server_id() {
                            self.stop_helpers_for_server(&server_id);
                        }
                        self.abort_first_start();
                        self.finish_active_lifecycle_operation_failure(&message);
                    }
                },
            }
        }
    }

    fn spawn_process_pump(&self, pid: ProcessId) {
        let state = self.clone();
        let handle = tokio::spawn(async move {
            let mut framer = OutputLineFramer::new();
            let mut last_metrics_poll = Instant::now();
            loop {
                state.drain_process_events(pid, &mut framer);
                state.enforce_first_start_safety_cap();
                if last_metrics_poll.elapsed() >= Duration::from_secs(5) {
                    state.poll_java_metrics(pid);
                    last_metrics_poll = Instant::now();
                }
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

    /// Requests the same live player/TPS readings MSC 1 asks for every five
    /// seconds. The performance route is intentionally read-only: it reports
    /// the latest values already parsed from the server's console, so the
    /// lifecycle owner must issue these commands while the process is alive.
    fn poll_java_metrics(&self, pid: ProcessId) {
        let Some(server_id) = self.active_server_id() else {
            return;
        };
        let Some(server) = self.inner.registry.get(&ServerId::new(server_id)) else {
            return;
        };
        if server.server_type != ServerType::Java {
            return;
        }

        let tps_command = server
            .java_flavor
            .tps_poll_command(server.minecraft_version.as_deref());
        let mut lifecycle = self.inner.lifecycle.lock().unwrap();
        if lifecycle.active_process() != Some(pid) || lifecycle.state() != LifecycleState::Running {
            return;
        }
        let _ = lifecycle.send_command("list");
        if let Some(command) = tps_command {
            let _ = lifecycle.send_command(command);
        }
    }

    fn enforce_first_start_safety_cap(&self) {
        let should_stop = {
            let mut first_start = self.inner.first_start.lock().unwrap();
            let Some(run) = first_start.as_mut() else {
                return;
            };
            let Some(started_at) = *self.inner.first_start_pass_two_started_at.lock().unwrap()
            else {
                return;
            };
            if run.phase != FirstStartPhase::PassTwo
                || !first_run_safety_cap_reached(started_at.elapsed().as_secs())
            {
                return;
            }
            run.mark_safety_cap_failures();
            run.issue_auto_stop()
        };
        if should_stop {
            if let Some(server_id) = self.active_server_id() {
                self.stop_first_start_server(&server_id);
            }
            self.progress_first_start(
                "First-start safety limit reached; stopping the server and keeping the resolved state.",
            );
        }
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
                let now = iso8601_now();
                let output_events = self
                    .inner
                    .lifecycle
                    .lock()
                    .unwrap()
                    .ingest_console_line(&line, &now)
                    .unwrap_or_default();
                if output_events.iter().any(|event| {
                    matches!(event, msc_application::output_reducer::OutputEvent::Ready)
                }) {
                    self.handle_server_ready("Java server is ready.");
                }
            }
            let exited = matches!(event, ProcessEvent::Exited(_));
            let _ = self.inner.lifecycle.lock().unwrap().handle_process_event(
                pid,
                &event,
                &iso8601_now(),
            );
            if exited {
                self.handle_process_termination(false);
            }
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

    fn handle_process_termination(&self, clean_stop_success: bool) {
        if let Some(server_id) = self.active_server_id() {
            self.stop_helpers_for_server(&server_id);
        }
        let outcome = {
            let mut first_start = self.inner.first_start.lock().unwrap();
            let Some(run) = first_start.as_mut() else {
                if clean_stop_success {
                    self.finish_active_lifecycle_operation_success("Bedrock server stopped.");
                } else {
                    self.finish_active_lifecycle_operation_failure(
                        "Java process exited before readiness.",
                    );
                }
                return;
            };
            let server_id = run.server_id.clone();
            match run.phase {
                FirstStartPhase::WaitingForSetup if run.server_ready => {
                    let needs_second_pass = run.needs_second_pass();
                    run.finish_first_pass();
                    Some((server_id, needs_second_pass, false, false))
                }
                FirstStartPhase::Complete | FirstStartPhase::PassTwo
                    if run.server_ready || run.safety_cap_reached() =>
                {
                    run.complete();
                    Some((server_id, false, true, run.safety_cap_reached()))
                }
                _ => {
                    *first_start = None;
                    None
                }
            }
        };

        match outcome {
            Some((server_id, true, false, _)) => {
                self.progress_first_start(
                    "Pass one is complete. Waiting for the first-start transport choices.",
                );
                self.finish_active_lifecycle_operation_success_with_result(
                    "First-start pass one complete.",
                    BTreeMap::from([
                        ("firstStartPass1Complete".into(), "true".into()),
                        ("firstStartNeedsPass2".into(), "true".into()),
                        ("firstStartServerId".into(), server_id),
                    ]),
                );
            }
            Some((server_id, false, true, safety_cap)) => {
                let completion_saved = self.mark_first_start_complete(&server_id).is_ok();
                self.finish_active_lifecycle_operation_success_with_result(
                    if safety_cap {
                        "First-start safety limit reached; server stopped."
                    } else {
                        "First-start setup complete; server stopped."
                    },
                    BTreeMap::from([
                        ("firstStartComplete".into(), completion_saved.to_string()),
                        ("firstStartServerId".into(), server_id),
                        ("firstStartSafetyCap".into(), safety_cap.to_string()),
                    ]),
                );
                *self.inner.first_start.lock().unwrap() = None;
                *self.inner.first_start_pass_two_started_at.lock().unwrap() = None;
            }
            None => {
                self.finish_active_lifecycle_operation_failure(
                    "The first-start server process exited before completion; initiation remains available on the next Start.",
                );
            }
            Some((_, false, false, _)) | Some((_, true, true, _)) => {
                unreachable!("first-start outcome invariant")
            }
        }
    }

    fn abort_first_start(&self) {
        *self.inner.first_start.lock().unwrap() = None;
        *self.inner.first_start_pass_two_started_at.lock().unwrap() = None;
    }

    fn finish_active_lifecycle_operation_success(&self, status_line: &str) {
        self.finish_active_lifecycle_operation_success_with_result(status_line, BTreeMap::new());
    }

    fn finish_active_lifecycle_operation_success_with_result(
        &self,
        status_line: &str,
        additional_result: BTreeMap<String, String>,
    ) {
        let Some(operation_id) = self.inner.active_lifecycle_operation.lock().unwrap().take()
        else {
            return;
        };
        let mut result = BTreeMap::new();
        if let Some(active) = self.active_server_id() {
            result.insert("activeServerId".to_string(), active);
        }
        result.extend(additional_result);
        let _ = self
            .inner
            .operations
            .succeed(&operation_id, status_line, result);
        if let Some(server_id) = self.active_server_id() {
            let server_name = self
                .inner
                .registry
                .get(&ServerId::new(server_id.clone()))
                .map(|server| server.display_name)
                .unwrap_or_else(|| server_id.clone());
            let started = !status_line.to_ascii_lowercase().contains("stopped");
            self.inner
                .notifications
                .push_lifecycle(&server_id, &server_name, started);
        }
    }

    fn finish_active_lifecycle_operation_failure(&self, message: &str) {
        let Some(operation_id) = self.inner.active_lifecycle_operation.lock().unwrap().take()
        else {
            return;
        };
        let _ = self
            .inner
            .operations
            .fail(&operation_id, "lifecycle_error", message.to_string());
    }

    fn bedrock_operation_cancel_requested(&self) -> bool {
        let Some(operation_id) = self
            .inner
            .active_lifecycle_operation
            .lock()
            .unwrap()
            .clone()
        else {
            return false;
        };
        self.inner.operations.cancellation_check(&operation_id)()
    }

    fn finish_active_lifecycle_operation_cancelled(&self) {
        let Some(operation_id) = self.inner.active_lifecycle_operation.lock().unwrap().take()
        else {
            return;
        };
        let _ = self
            .inner
            .operations
            .cancel(&operation_id, "Bedrock operation cancelled.");
    }
}

fn bedrock_safety_backup(server_dir: &Path) -> bool {
    let association = BackupAssociation {
        slot_id: None,
        slot_name: None,
        world_seed: None,
    };
    msc_application::backups::create_backup(
        &StdFileSystem,
        server_dir,
        ServerType::Bedrock,
        None,
        &association,
        None,
        None,
        false,
        true,
        Some("pre-downgrade"),
        None,
        &iso8601_now(),
        None,
        || false,
        || false,
    )
    .is_ok()
}

#[derive(Debug)]
pub struct LifecycleActionResult {
    pub active_server_id: Option<String>,
    pub operation_id: Option<String>,
}

#[derive(Debug)]
pub enum LifecycleRouteError {
    Lifecycle(LifecycleError),
    Operation(msc_application::operations::LifecycleOperationError),
    BedrockRuntime {
        state: msc_api::dto::BedrockRuntimeStateDto,
        error: BedrockRuntimeError,
    },
    /// P7.31: the required-major Java guard refused the resolved
    /// executable before a process was ever spawned.
    UnusableJavaRuntime(msc_domain::java_runtime::UnusableJavaRuntime),
}

impl From<LifecycleError> for LifecycleRouteError {
    fn from(value: LifecycleError) -> Self {
        Self::Lifecycle(value)
    }
}

impl From<msc_application::operations::LifecycleOperationError> for LifecycleRouteError {
    fn from(value: msc_application::operations::LifecycleOperationError) -> Self {
        Self::Operation(value)
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
    pub bedrock_port: Option<i64>,
    pub first_start_required: bool,
    pub playit_enabled: bool,
    pub xbox_broadcast_enabled: bool,
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
            active_server_id: active_server_id.active_server_id,
            operation_id: active_server_id.operation_id,
            runtime: state
                .active_bedrock_server()
                .map(|_| state.bedrock_runtime_state()),
        })
        .into_response(),
        Err(error) => lifecycle_route_error_response(error),
    }
}

pub async fn stop(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::ServerControl) {
        return response;
    }

    if state.active_bedrock_server().is_some() {
        return match state.stop_active_bedrock_server() {
            Ok(result) => Json(SimpleResultDto {
                result: "stop_requested".to_string(),
                active_server_id: result.active_server_id,
                operation_id: result.operation_id,
                runtime: Some(state.bedrock_runtime_state()),
            })
            .into_response(),
            Err(error) => lifecycle_route_error_response(error),
        };
    }

    match state.stop_active_server() {
        Ok(active_server_id) => Json(SimpleResultDto {
            result: "stop_requested".to_string(),
            active_server_id,
            operation_id: None,
            runtime: None,
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
            operation_id: None,
            runtime: state
                .active_config_server()
                .filter(|server| server.server_type == ServerType::Bedrock)
                .map(|_| state.bedrock_runtime_state()),
        })
        .into_response(),
        Err(error) => active_server_selection_error_response(error),
    }
}

impl AgentAppConfigStore {
    #[allow(dead_code)]
    pub fn production() -> Result<Self, AgentAppConfigError> {
        let path = default_app_config_path();
        let servers_root = default_servers_root();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                AgentAppConfigError::Load(format!(
                    "failed to create app config directory {}: {error}",
                    parent.display()
                ))
            })?;
        }
        std::fs::create_dir_all(&servers_root).map_err(|error| {
            AgentAppConfigError::Load(format!(
                "failed to create servers root {}: {error}",
                servers_root.display()
            ))
        })?;
        Self::load(Box::leak(Box::new(StdFileSystem)), path, servers_root)
    }

    pub fn production_migrating_legacy_secrets(
        secrets: &dyn SecretStore,
    ) -> Result<Self, AgentAppConfigError> {
        let path = default_app_config_path();
        let servers_root = default_servers_root();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                AgentAppConfigError::Load(format!(
                    "failed to create app config directory {}: {error}",
                    parent.display()
                ))
            })?;
        }
        std::fs::create_dir_all(&servers_root).map_err(|error| {
            AgentAppConfigError::Load(format!(
                "failed to create servers root {}: {error}",
                servers_root.display()
            ))
        })?;
        Self::load_migrating_legacy_secrets(
            Box::leak(Box::new(StdFileSystem)),
            path,
            servers_root,
            secrets,
        )
    }

    #[allow(dead_code)]
    pub fn load(
        fs: &'static dyn FileSystem,
        path: PathBuf,
        servers_root: PathBuf,
    ) -> Result<Self, AgentAppConfigError> {
        let defaults = AppConfig::default_config(servers_root.to_string_lossy().into_owned());
        let outcome = load_app_config(fs, &path, &defaults, SystemTime::now())
            .map_err(AgentAppConfigError::from)?;
        Ok(Self {
            fs,
            path,
            config: Mutex::new(outcome.config),
        })
    }

    pub fn load_migrating_legacy_secrets(
        fs: &'static dyn FileSystem,
        path: PathBuf,
        servers_root: PathBuf,
        secrets: &dyn SecretStore,
    ) -> Result<Self, AgentAppConfigError> {
        let defaults = AppConfig::default_config(servers_root.to_string_lossy().into_owned());
        let outcome = load_app_config_migrating_legacy_secrets(
            fs,
            &path,
            &defaults,
            SystemTime::now(),
            secrets,
        )
        .map_err(AgentAppConfigError::from)?;
        Ok(Self {
            fs,
            path,
            config: Mutex::new(outcome.config),
        })
    }

    pub fn snapshot(&self) -> AppConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn reset_in_memory(&self) {
        let servers_root = self.servers_root();
        *self.config.lock().unwrap() =
            AppConfig::default_config(servers_root.to_string_lossy().into_owned());
    }

    pub fn servers(&self) -> Vec<ConfigServer> {
        self.snapshot().servers
    }

    pub fn servers_root(&self) -> PathBuf {
        PathBuf::from(self.config.lock().unwrap().servers_root.clone())
    }

    /// See [`LifecycleRoutesState::update_backup_config`].
    pub fn update_backup_config(
        &self,
        server_id: &str,
        enabled: Option<bool>,
        interval_minutes: Option<i64>,
        max_count: Option<i64>,
    ) -> Result<ConfigServer, AgentAppConfigError> {
        let mut updated = None;
        self.mutate(|config| {
            if let Some(server) = config.servers.iter_mut().find(|s| s.id == server_id) {
                if let Some(enabled) = enabled {
                    server.auto_backup_enabled = enabled;
                }
                if let Some(interval_minutes) = interval_minutes {
                    server.auto_backup_interval_minutes = interval_minutes;
                }
                if let Some(max_count) = max_count {
                    server.auto_backup_max_count = max_count;
                }
                updated = Some(server.clone());
            }
        })?;
        updated.ok_or_else(|| AgentAppConfigError::Save(format!("no server named '{server_id}'")))
    }

    pub fn active_server_id(&self) -> Option<String> {
        self.config.lock().unwrap().active_server_id.clone()
    }

    pub fn update_duckdns_hostname(
        &self,
        hostname: Option<String>,
    ) -> Result<(), AgentAppConfigError> {
        self.mutate(|config| config.duckdns_hostname = hostname)
    }

    #[cfg(test)]
    pub fn upsert_server(&self, server: ConfigServer) -> Result<(), AgentAppConfigError> {
        self.mutate(|config| {
            if let Some(existing) = config.servers.iter_mut().find(|item| item.id == server.id) {
                *existing = server;
            } else {
                config.servers.push(server);
            }
        })
    }

    pub fn merge_servers(&self, new_servers: Vec<ConfigServer>) -> Result<(), AgentAppConfigError> {
        self.mutate(|config| {
            config.servers.extend(new_servers);
        })
    }

    pub fn replace_servers(
        &self,
        new_servers: Vec<ConfigServer>,
    ) -> Result<(), AgentAppConfigError> {
        self.mutate(|config| {
            config.servers = new_servers;
            config.active_server_id = config
                .servers
                .iter()
                .find(|server| server.server_type == ServerType::Java)
                .map(|server| server.id.clone());
        })
    }

    pub fn set_active_server_id(
        &self,
        server_id: Option<String>,
    ) -> Result<(), AgentAppConfigError> {
        self.mutate(|config| {
            config.active_server_id = server_id;
        })
    }

    pub fn existing_java_ports(&self) -> Vec<i64> {
        self.servers()
            .iter()
            .filter(|server| server.server_type == ServerType::Java)
            .filter_map(|server| java_server_port(Path::new(&server.server_dir)))
            .collect()
    }

    pub fn existing_bedrock_ports(&self) -> Vec<i64> {
        self.servers()
            .iter()
            .filter(|server| server.server_type == ServerType::Bedrock)
            .filter_map(|server| server.bedrock_port)
            .collect()
    }

    pub fn export_inputs(&self) -> Vec<TransferExportServerInput> {
        self.servers()
            .into_iter()
            .map(|server| TransferExportServerInput {
                server,
                paper_mc_version: None,
                paper_build: None,
            })
            .collect()
    }

    fn mutate(&self, update: impl FnOnce(&mut AppConfig)) -> Result<(), AgentAppConfigError> {
        let mut guard = self.config.lock().unwrap();
        let previous = guard.clone();
        update(&mut guard);
        if let Err(error) = save_app_config(self.fs, &self.path, &guard) {
            *guard = previous;
            return Err(AgentAppConfigError::from(error));
        }
        Ok(())
    }

    /// [`Self::mutate`]'s fallible sibling: `update` gets a chance to
    /// refuse before anything is saved (or even left mutated) — a
    /// `DeleteServerError::ServerNotFound`, for instance, must never
    /// reach `save_app_config` with a half-applied change sitting in the
    /// guard. On `Ok`, behaves exactly like `mutate` (save, roll back the
    /// in-memory guard on a save failure); on `Err`, the guard is rolled
    /// back to `previous` and nothing is ever written to disk.
    fn try_mutate<T, E>(
        &self,
        update: impl FnOnce(&mut AppConfig) -> Result<T, E>,
    ) -> Result<T, TryMutateError<E>> {
        let mut guard = self.config.lock().unwrap();
        let previous = guard.clone();
        match update(&mut guard) {
            Ok(value) => {
                if let Err(error) = save_app_config(self.fs, &self.path, &guard) {
                    *guard = previous;
                    return Err(TryMutateError::Save(AgentAppConfigError::from(error)));
                }
                Ok(value)
            }
            Err(domain_error) => {
                *guard = previous;
                Err(TryMutateError::Domain(domain_error))
            }
        }
    }
}

/// See [`AgentAppConfigStore::try_mutate`].
#[derive(Debug)]
pub enum TryMutateError<E> {
    Domain(E),
    Save(AgentAppConfigError),
}

impl From<AppConfigLoadError> for AgentAppConfigError {
    fn from(value: AppConfigLoadError) -> Self {
        Self::Load(value.to_string())
    }
}

impl From<ConfigSaveError> for AgentAppConfigError {
    fn from(value: ConfigSaveError) -> Self {
        Self::Save(value.to_string())
    }
}

impl std::fmt::Display for AgentAppConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentAppConfigError::Load(message) | AgentAppConfigError::Save(message) => {
                write!(f, "{message}")
            }
        }
    }
}

impl std::error::Error for AgentAppConfigError {}

impl AgentServerRegistry {
    fn new(app_config: &'static AgentAppConfigStore) -> Self {
        Self { app_config }
    }

    #[cfg(test)]
    fn insert(&self, server: ImportedPaperServer) -> Result<(), AgentAppConfigError> {
        self.app_config
            .upsert_server(config_server_from_paper(server))
    }

    fn get(&self, id: &ServerId) -> Option<ConfigServer> {
        self.app_config
            .servers()
            .into_iter()
            .find(|server| server.id == id.as_str())
    }

    fn list(&self) -> Vec<RegisteredServerDtoParts> {
        self.app_config
            .servers()
            .into_iter()
            .map(|server| RegisteredServerDtoParts {
                id: server.id.clone(),
                name: server.display_name.clone(),
                directory: server.server_dir.clone(),
                server_type: server.server_type.raw_value().to_string(),
                java_flavor: (server.server_type == ServerType::Java)
                    .then(|| server.java_flavor.raw_value().to_string()),
                game_port: if server.server_type == ServerType::Java {
                    java_server_port(Path::new(&server.server_dir))
                } else {
                    server.bedrock_port
                },
                bedrock_port: (server.server_type == ServerType::Java)
                    .then_some(server.bedrock_port)
                    .flatten(),
                first_start_required: msc_application::provisioning::first_start_required(&server),
                playit_enabled: server.playit_enabled,
                xbox_broadcast_enabled: server.xbox_broadcast_enabled,
            })
            .collect()
    }
}

impl JavaServerRepository for AgentServerRegistry {
    fn load(&self, id: &ServerId) -> Result<Option<ImportedJavaServer>, LifecycleError> {
        Ok(self.get(id).and_then(config_server_to_lifecycle_server))
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

pub fn lifecycle_route_error_response(error: LifecycleRouteError) -> Response {
    match error {
        LifecycleRouteError::Lifecycle(error) => lifecycle_error_response(error),
        LifecycleRouteError::Operation(error) => operation_error_response(error),
        LifecycleRouteError::BedrockRuntime { state, error } => {
            if state.state != "available" {
                (
                    StatusCode::CONFLICT,
                    Json(state.capability_unavailable_error()),
                )
                    .into_response()
            } else {
                error_response(
                    StatusCode::CONFLICT,
                    "bedrock_runtime_error",
                    &error.to_string(),
                )
            }
        }
        LifecycleRouteError::UnusableJavaRuntime(error) => error_response(
            StatusCode::CONFLICT,
            "unusable_java_runtime",
            &error.to_string(),
        ),
    }
}

fn active_server_selection_error_response(error: ActiveServerSelectionError) -> Response {
    match error {
        ActiveServerSelectionError::Lifecycle(error) => lifecycle_error_response(error),
        ActiveServerSelectionError::Reconciliation { reason } => {
            reconciliation_degraded_response(&reason)
        }
    }
}

/// The one structured error every world/backup mutation route must
/// return for a server left [`ReconciliationStatus::Degraded`] —
/// `routes/worlds.rs`'s and `routes/backups.rs`'s own
/// `active_server_or_response` gate call this before admitting any
/// mutation. `409 conflict` (not `503`): this is one server's on-disk
/// state, not whole-agent unavailability — the agent, and every other
/// server on it, stays fully usable.
pub fn reconciliation_degraded_response(reason: &str) -> Response {
    error_response(
        StatusCode::CONFLICT,
        "world_reconciliation_degraded",
        &format!(
            "This server's world data could not be safely reconciled, so it is in a \
             read-only diagnostic state: {reason}. Fix the underlying issue and restart the agent."
        ),
    )
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

/// Dispatches on `registered.java_flavor` for the launch shape the port
/// plan's later-audit clause names: Forge/NeoForge run `@<args-file>
/// nogui` against whichever installed-loader directory the installer
/// actually produced (P7.14's `run_loader_installer` already wrote it;
/// this rediscovers it the same way `run_loader_installer` verifies it
/// right after the installer exits), every other flavor keeps the
/// existing `-jar <jar> --nogui` Paper-shaped path unchanged --
/// `registered.paper_jar_path` already holds each download-and-go
/// flavor's real staged jar (always named `paper.jar` on disk per
/// `create_download_and_go_server`, regardless of flavor), so this
/// branch never needed flavor-specific handling in the first place.
fn build_launch_request(
    registered: &ConfigServer,
    java_path: &str,
) -> Result<ProcessSpawnRequest, LifecycleError> {
    let java_path = java_path.to_string();
    let server_dir = PathBuf::from(&registered.server_dir);

    if matches!(
        registered.java_flavor,
        JavaServerFlavor::Forge | JavaServerFlavor::NeoForge
    ) {
        let args_file = match registered.java_flavor {
            JavaServerFlavor::NeoForge => find_neoforge_args_file(
                &StdFileSystem,
                &server_dir,
                registered.loader_version.as_deref(),
            ),
            JavaServerFlavor::Forge => find_forge_args_file(
                &StdFileSystem,
                &server_dir,
                registered.minecraft_version.as_deref(),
                registered.loader_version.as_deref(),
            ),
            _ => unreachable!("matched to Forge|NeoForge above"),
        }
        .ok_or_else(|| {
            LifecycleError::Process(format!(
                "{:?} args file not found. Run the server once inside MSC to complete installation.",
                registered.java_flavor
            ))
        })?;

        let mut arguments = jvm_flags(registered.min_ram_gb, registered.max_ram_gb, "");
        arguments.push(format!("@{args_file}"));
        arguments.push("nogui".to_string());
        return Ok(ProcessSpawnRequest {
            executable_path: PathBuf::from(java_path),
            arguments,
            working_directory: server_dir,
            environment: Vec::new(),
        });
    }

    let request = PaperLaunchRequest::new(
        ValidatedJavaLaunch::new(java_path, Vec::<String>::new()),
        server_dir,
        PathBuf::from(&registered.paper_jar_path),
        registered.min_ram_gb,
        registered.max_ram_gb,
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

#[cfg(test)]
fn config_server_from_paper(server: ImportedPaperServer) -> ConfigServer {
    let mut config = ConfigServer::new(
        server.id.as_str().to_string(),
        server.display_name,
        server.server_dir.to_string_lossy().into_owned(),
        server.paper_jar_path.to_string_lossy().into_owned(),
        1.0,
        2.0,
    );
    config.server_type = ServerType::Java;
    config.java_flavor = JavaServerFlavor::Paper;
    config
}

fn config_server_to_lifecycle_server(server: ConfigServer) -> Option<ImportedJavaServer> {
    if server.server_type != ServerType::Java {
        return None;
    }
    Some(ImportedJavaServer {
        id: ServerId::new(server.id),
        name: server.display_name,
        directory: PathBuf::from(server.server_dir),
        flavor: server.java_flavor,
    })
}

fn java_server_port(server_dir: &Path) -> Option<i64> {
    let contents = std::fs::read_to_string(server_dir.join("server.properties")).ok()?;
    contents
        .lines()
        .find_map(|line| line.strip_prefix("server-port="))
        .and_then(|value| value.trim().parse::<i64>().ok())
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
        tps_5m: None,
        tps_15m: None,
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
    use msc_infrastructure::config_repository::{
        default_app_config_path_from_env, default_servers_root_from_env,
    };
    use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
    use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};
    use std::collections::HashMap;
    use std::ffi::OsString;

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

    fn app_config_store(
        fs: &'static FakeFileSystem,
        config_path: std::path::PathBuf,
        servers_root: std::path::PathBuf,
    ) -> &'static AgentAppConfigStore {
        Box::leak(Box::new(
            AgentAppConfigStore::load(fs, config_path, servers_root).unwrap(),
        ))
    }

    /// P7.31's own required-major guard now spawns `<java> -version`
    /// (through the same `FakeProcessSupervisor` `start_active_server`
    /// itself launches against) before the real server process -- driven
    /// here on a background OS thread since the guard's own call blocks
    /// the calling thread polling for it, the same "the fake has no
    /// automatic responder" shape `provisioning_install_step.rs`'s
    /// `drive_fake_java_version_probe` already established. Answers with
    /// a Java 25 banner, comfortably above every possible
    /// `required_java_major` result, so this never itself trips a
    /// refusal or warning a test here isn't asserting about. Only drives
    /// the *first* spawn (the probe) to completion -- the real launch
    /// spawn that follows it is fire-and-forget, matching how these
    /// tests behaved before this guard existed.
    fn drive_java_version_probe_once(
        supervisor: &'static msc_infrastructure::process::FakeProcessSupervisor,
    ) -> std::thread::JoinHandle<()> {
        drive_java_version_probe_with_banner(supervisor, "openjdk version \"25.0.1\" 2025-01-01\n")
    }

    /// [`drive_java_version_probe_once`]'s own parameterized sibling, for
    /// a test that needs the guard to see a *specific* major version
    /// rather than a comfortably-above-everything one.
    fn drive_java_version_probe_with_banner(
        supervisor: &'static msc_infrastructure::process::FakeProcessSupervisor,
        banner: &'static str,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            loop {
                if let Some((pid, _)) = supervisor.spawned_requests().into_iter().next() {
                    let _ = supervisor.emit_stdout(pid, banner.as_bytes().to_vec());
                    let _ = supervisor.exit_normally(pid);
                    return;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        })
    }

    fn temp_server_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "msc2-durable-server-state-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("paper.jar"), b"fake jar").unwrap();
        std::fs::write(
            dir.join("server.properties"),
            b"server-port=25565\nmax-players=20\nlevel-name=world\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn startup_secret_migration_loader_moves_xbox_password_and_scrubs_config() {
        let fs = Box::leak(Box::new(
            FakeFileSystem::new()
                .with_dir("/cfg")
                .with_dir("/servers")
                .with_dir("/servers/legacy_java"),
        ));
        let config_path = std::path::PathBuf::from("/cfg/server_config_swift.json");
        let servers_root = std::path::PathBuf::from("/servers");
        fs.write(
            &config_path,
            br#"{
  "config_version": 1,
  "servers_root": "/servers",
  "remote_api_token": "legacy-owner-secret-xyz",
  "servers": [
    {
      "id": "11111111-1111-1111-1111-111111111111",
      "display_name": "Legacy Java",
      "server_dir": "/servers/legacy_java",
      "paper_jar_path": "/servers/legacy_java/paper.jar",
      "min_ram_gb": 2,
      "max_ram_gb": 4,
      "server_type": "java",
      "xbox_broadcast_alt_password": "legacy-alt-password"
    }
  ]
}"#,
        )
        .unwrap();
        let secrets = FakeSecretStore::new();

        let store = AgentAppConfigStore::load_migrating_legacy_secrets(
            fs,
            config_path.clone(),
            servers_root,
            &secrets,
        )
        .unwrap();

        assert_eq!(store.servers().len(), 1);
        assert_eq!(
            secrets.get("remote-api.owner-token").unwrap().as_deref(),
            Some("legacy-owner-secret-xyz")
        );
        assert_eq!(
            secrets
                .get("xbox-broadcast.alt-password.11111111-1111-1111-1111-111111111111")
                .unwrap()
                .as_deref(),
            Some("legacy-alt-password")
        );
        let saved = String::from_utf8(fs.read(&config_path).unwrap()).unwrap();
        assert!(!saved.contains("remote_api_token"));
        assert!(!saved.contains("xbox_broadcast_alt_password"));
    }

    #[tokio::test]
    async fn phase4_lifecycle_routes_state_selects_starts_and_stops_active_server() {
        let (state, supervisor) = LifecycleRoutesState::with_fake_process_capturing_supervisor(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );
        let server_dir = std::env::temp_dir().join(format!(
            "msc2-agent-lifecycle-routes-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&server_dir);
        std::fs::create_dir_all(&server_dir).unwrap();
        let server = imported_server(server_dir.clone());
        std::fs::write(&server.paper_jar_path, b"fake jar").unwrap();

        state.register_imported_paper(server).unwrap();
        assert_eq!(state.servers().len(), 1);
        assert_eq!(
            state.select_active_server("paper-1".to_string()).unwrap(),
            "paper-1"
        );

        let probe_driver = drive_java_version_probe_once(supervisor);
        let active = state.start_active_server().unwrap();
        probe_driver.join().unwrap();
        assert_eq!(active.active_server_id.as_deref(), Some("paper-1"));
        assert!(active.operation_id.is_some());
        let status = state.status_snapshot();
        assert!(status.running);
        assert_eq!(status.active_server_id.as_deref(), Some("paper-1"));
        assert_eq!(status.pid, Some(1001));

        let active = state.stop_active_server().unwrap();
        assert_eq!(active.as_deref(), Some("paper-1"));

        std::fs::remove_dir_all(server_dir).unwrap();
    }

    // ---------------------------------------------------------------------
    // P7.31: the required-major guard actually refuses `start`, rather
    // than only proceeding cleanly on a comfortable Java (the case every
    // other `start_active_server` test above now drives past).
    // ---------------------------------------------------------------------

    #[tokio::test]
    async fn java_runtime_guard_start_refuses_below_required_major() {
        let (state, supervisor) = LifecycleRoutesState::with_fake_process_capturing_supervisor(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );
        let server_dir = std::env::temp_dir().join(format!(
            "msc2-agent-lifecycle-routes-java-guard-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&server_dir);
        std::fs::create_dir_all(&server_dir).unwrap();
        std::fs::write(server_dir.join("paper.jar"), b"fake jar").unwrap();

        let mut server = ConfigServer::new(
            "paper-bad-java",
            "Needs Java 21",
            server_dir.to_string_lossy().into_owned(),
            server_dir.join("paper.jar").to_string_lossy().into_owned(),
            1.0,
            2.0,
        );
        server.server_type = ServerType::Java;
        server.java_flavor = JavaServerFlavor::Paper;
        // 1.21.4 needs Java 21 (`required_java_major`); the probe below
        // answers with Java 17.
        server.minecraft_version = Some("1.21.4".to_string());
        state
            .register_imported_config_servers(vec![server], false)
            .unwrap();
        state
            .select_active_server("paper-bad-java".to_string())
            .unwrap();

        let probe_driver = drive_java_version_probe_with_banner(
            supervisor,
            "openjdk version \"17.0.9\" 2023-10-17\n",
        );
        let err = state.start_active_server().unwrap_err();
        probe_driver.join().unwrap();

        assert!(matches!(err, LifecycleRouteError::UnusableJavaRuntime(_)));
        // Refused before the real launch was ever spawned -- only the
        // probe itself shows up.
        assert_eq!(supervisor.spawned_requests().len(), 1);
        assert!(!state.status_snapshot().running);

        std::fs::remove_dir_all(server_dir).unwrap();
    }

    #[tokio::test]
    async fn durable_server_state_persists_import_and_active_selection_across_state_rebuild() {
        let fs = Box::leak(Box::new(
            FakeFileSystem::new()
                .with_dir("/srv/msc2")
                .with_dir("/srv/msc2/servers"),
        ));
        let config_path = std::path::PathBuf::from("/srv/msc2/server_config_swift.json");
        let servers_root = std::path::PathBuf::from("/srv/msc2/servers");
        let first_store = app_config_store(fs, config_path.clone(), servers_root.clone());
        let first = LifecycleRoutesState::with_fake_process_and_app_config(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
            first_store,
        );
        let server_dir = temp_server_dir("restart");

        first
            .register_imported_paper(imported_server(server_dir.clone()))
            .unwrap();
        first.select_active_server("paper-1".to_string()).unwrap();

        let on_disk = String::from_utf8(fs.read(&config_path).unwrap()).unwrap();
        assert!(on_disk.contains("\"servers\""));
        assert!(on_disk.contains("\"active_server_id\""));
        assert!(on_disk.contains("paper-1"));

        let second_store = app_config_store(fs, config_path, servers_root);
        let (second, second_supervisor) =
            LifecycleRoutesState::with_fake_process_and_app_config_capturing_supervisor(
                ConsoleState::default(),
                OperationsState::fake_journaled(),
                second_store,
            );

        assert_eq!(second.servers().len(), 1);
        assert_eq!(second.active_server_id().as_deref(), Some("paper-1"));
        let probe_driver = drive_java_version_probe_once(second_supervisor);
        let active = second.start_active_server().unwrap();
        probe_driver.join().unwrap();
        assert_eq!(active.active_server_id.as_deref(), Some("paper-1"));

        std::fs::remove_dir_all(server_dir).unwrap();
    }

    #[test]
    fn durable_server_state_default_paths_are_app_data_not_temp() {
        let app_config_path = default_app_config_path_from_env(|key| match key {
            "HOME" => Some(OsString::from("/Users/cameron")),
            _ => None,
        });
        let servers_root = default_servers_root_from_env(|key| match key {
            "HOME" => Some(OsString::from("/Users/cameron")),
            _ => None,
        });

        assert!(!app_config_path.starts_with(std::env::temp_dir()));
        assert!(!servers_root.starts_with(std::env::temp_dir()));
        assert!(app_config_path.ends_with("server_config_swift.json"));
        assert!(servers_root.ends_with("servers"));
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
