//! Shared state and handlers for Phase 4 lifecycle routes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{ActiveServerRequestDto, ErrorDto, PermissionCategoryDto, SimpleResultDto};
#[cfg(test)]
use msc_application::import::ImportedPaperServer;
use msc_application::java_launch::{
    PaperLaunchRequest, StdJavaLaunchFileSystem, ValidatedJavaLaunch, build_paper_launch_command,
};
use msc_application::lifecycle::{
    ConsoleSink, ImportedJavaServer, JavaServerRepository, LifecycleError, LifecycleService,
    ServerId,
};
use msc_application::status::{LifecycleStatusSnapshot, PerformanceSnapshot};
use msc_application::transfer::TransferExportServerInput;
use msc_domain::app_config_schema::{AppConfig, ConfigServer};
#[cfg(test)]
use msc_domain::identity::JavaServerFlavor;
use msc_domain::identity::ServerType;
use msc_domain::operation::OperationId;
use msc_infrastructure::audit_log::AuditLog;
use msc_infrastructure::config_repository::{
    AppConfigLoadError, ConfigSaveError, default_app_config_path, default_servers_root,
    load_app_config, load_app_config_migrating_legacy_secrets, save_app_config,
};
use msc_infrastructure::console_buffer::ConsoleLine;
use msc_infrastructure::fs::{FileSystem, StdFileSystem};
use msc_infrastructure::metrics::PsProcessMetricsProvider;
use msc_infrastructure::process::{
    OutputLineFramer, OutputStream, ProcessEvent, ProcessId, ProcessSpawnRequest, ProcessSupervisor,
};
use msc_infrastructure::secret_store::SecretStore;
use tokio::task::JoinHandle;

use crate::auth::{AuthState, AuthenticatedCredential};
use crate::routes::operations::{OperationsState, operation_error_response};
use crate::ws::console::ConsoleState;

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

struct LifecycleRoutesInner {
    registry: &'static AgentServerRegistry,
    app_config: &'static AgentAppConfigStore,
    process: &'static (dyn ProcessSupervisor + Send + Sync),
    console: &'static AgentConsoleSink,
    metrics: PsProcessMetricsProvider,
    lifecycle: Mutex<LifecycleService<'static>>,
    operations: OperationsState,
    active_lifecycle_operation: Mutex<Option<OperationId>>,
    pump_tasks: Mutex<Vec<JoinHandle<()>>>,
    auth_state: Option<AuthState>,
    audit_log: &'static AuditLog<'static>,
    reconciliation: Mutex<BTreeMap<String, ReconciliationStatus>>,
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
        )
    }

    pub fn with_app_config_and_auth(
        console_state: ConsoleState,
        operations: OperationsState,
        app_config: &'static AgentAppConfigStore,
        auth_state: AuthState,
    ) -> Self {
        Self::with_dependencies(
            console_state,
            operations,
            app_config,
            default_process_supervisor(),
            Some(auth_state),
        )
    }

    fn with_dependencies(
        console_state: ConsoleState,
        operations: OperationsState,
        app_config: &'static AgentAppConfigStore,
        process: Box<dyn ProcessSupervisor + Send + Sync>,
        auth_state: Option<AuthState>,
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
        let mut lifecycle = LifecycleService::new(registry, process, console);
        if let Some(active) = app_config.active_server_id()
            && matches!(
                reconciliation.get(&active),
                Some(ReconciliationStatus::Ready)
            )
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
                active_lifecycle_operation: Mutex::new(None),
                pump_tasks: Mutex::new(Vec::new()),
                auth_state,
                audit_log,
                reconciliation: Mutex::new(reconciliation),
            }),
        }
    }

    #[cfg(test)]
    pub fn with_fake_process(console_state: ConsoleState, operations: OperationsState) -> Self {
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
        Self::with_fake_process_and_app_config(console_state, operations, app_config)
    }

    #[cfg(test)]
    pub fn with_fake_process_and_app_config(
        console_state: ConsoleState,
        operations: OperationsState,
        app_config: &'static AgentAppConfigStore,
    ) -> Self {
        let process = Box::new(msc_infrastructure::process::FakeProcessSupervisor::new());
        Self::with_dependencies(console_state, operations, app_config, process, None)
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
            .lifecycle
            .lock()
            .unwrap()
            .active_server()
            .map(|id| id.as_str().to_string())
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

    /// The full application config, cloned — Phase 7's provisioning/
    /// version/template/java-runtime routes read `paper_template_dir`/
    /// `plugin_template_dir`/`java_path`/`save_downloaded_jars`/
    /// `default_banner_color_hex` off it directly rather than this type
    /// growing one accessor per field.
    pub fn app_config_snapshot(&self) -> AppConfig {
        self.inner.app_config.snapshot()
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

    /// See [`AgentConsoleSink`] / `ConsoleState::recent_lines` — the
    /// production `BackupConsole`'s read half.
    pub fn recent_console_lines(&self, count: usize) -> Vec<ConsoleLine> {
        self.inner.console.console.recent_lines(count)
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

    pub fn start_active_server(&self) -> Result<LifecycleActionResult, LifecycleRouteError> {
        self.drain_active_process_events();
        let active = self
            .inner
            .lifecycle
            .lock()
            .unwrap()
            .active_server()
            .cloned()
            .ok_or(LifecycleError::NoActiveServer)?;
        let operation_id = self.inner.operations.begin_lifecycle(
            "java-start",
            Some(active.as_str().to_string()),
            "Starting Java server.",
        )?;
        let registered = self
            .inner
            .registry
            .get(&active)
            .ok_or_else(|| LifecycleError::ServerNotFound(active.clone()))
            .inspect_err(|error| {
                let _ =
                    self.inner
                        .operations
                        .fail(&operation_id, "lifecycle_error", error.to_string());
            })?;
        let launch = build_launch_request(&registered).inspect_err(|error| {
            let _ = self
                .inner
                .operations
                .fail(&operation_id, "lifecycle_error", error.to_string());
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
                let _ =
                    self.inner
                        .operations
                        .fail(&operation_id, "lifecycle_error", error.to_string());
                return Err(error.into());
            }
        };
        let _ = self
            .inner
            .operations
            .progress(&operation_id, 1, 2, "Java process spawned.");
        *self.inner.active_lifecycle_operation.lock().unwrap() = Some(operation_id.clone());
        self.spawn_process_pump(pid);
        Ok(LifecycleActionResult {
            active_server_id: Some(active.as_str().to_string()),
            operation_id: Some(operation_id.as_str().to_string()),
        })
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
                let output_events = self
                    .inner
                    .lifecycle
                    .lock()
                    .unwrap()
                    .ingest_console_line(&line)
                    .unwrap_or_default();
                if output_events.iter().any(|event| {
                    matches!(event, msc_application::output_reducer::OutputEvent::Ready)
                }) {
                    self.finish_active_lifecycle_operation_success("Java server is ready.");
                }
            }
            let exited = matches!(event, ProcessEvent::Exited(_));
            let _ = self
                .inner
                .lifecycle
                .lock()
                .unwrap()
                .handle_process_event(pid, &event);
            if exited {
                self.finish_active_lifecycle_operation_failure(
                    "Java process exited before readiness.",
                );
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

    fn finish_active_lifecycle_operation_success(&self, status_line: &str) {
        let Some(operation_id) = self.inner.active_lifecycle_operation.lock().unwrap().take()
        else {
            return;
        };
        let mut result = BTreeMap::new();
        if let Some(active) = self.active_server_id() {
            result.insert("activeServerId".to_string(), active);
        }
        let _ = self
            .inner
            .operations
            .succeed(&operation_id, status_line, result);
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
}

pub struct LifecycleActionResult {
    pub active_server_id: Option<String>,
    pub operation_id: Option<String>,
}

#[derive(Debug)]
pub enum LifecycleRouteError {
    Lifecycle(LifecycleError),
    Operation(msc_application::operations::LifecycleOperationError),
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

    match state.stop_active_server() {
        Ok(active_server_id) => Json(SimpleResultDto {
            result: "stop_requested".to_string(),
            active_server_id,
            operation_id: None,
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

fn build_launch_request(registered: &ConfigServer) -> Result<ProcessSpawnRequest, LifecycleError> {
    let java_path = std::env::var("MSC2_JAVA_PATH").unwrap_or_else(|_| "java".to_string());
    let request = PaperLaunchRequest::new(
        ValidatedJavaLaunch::new(java_path, Vec::<String>::new()),
        PathBuf::from(&registered.server_dir),
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
        let state = LifecycleRoutesState::with_fake_process(
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

        let active = state.start_active_server().unwrap();
        assert_eq!(active.active_server_id.as_deref(), Some("paper-1"));
        assert!(active.operation_id.is_some());
        let status = state.status_snapshot();
        assert!(status.running);
        assert_eq!(status.active_server_id.as_deref(), Some("paper-1"));
        assert_eq!(status.pid, Some(1000));

        let active = state.stop_active_server().unwrap();
        assert_eq!(active.as_deref(), Some("paper-1"));

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
        let second = LifecycleRoutesState::with_fake_process_and_app_config(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
            second_store,
        );

        assert_eq!(second.servers().len(), 1);
        assert_eq!(second.active_server_id().as_deref(), Some("paper-1"));
        let active = second.start_active_server().unwrap();
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
