//! `GET /v1/servers` and `POST /v1/servers/import`.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{
    PermissionCategoryDto, ServerDto, ServerImportRequestDto, ServerImportResultDto,
    ServerImportScanResponseDto,
};
use msc_application::import::{
    PaperImportError, PaperImportRequest, StdPaperImportFileSystem, import_existing_paper_server,
};
use msc_application::transfer::{
    TransferApplyRequest, TransferApplyResult, TransferExportRequest, TransferExportServerInput,
    TransferInspection, apply_transfer_import, export_server_transfer, inspect_transfer_package,
};
use msc_domain::app_config_schema::{AppConfig, ConfigServer};
use msc_domain::identity::ServerType;

use crate::auth::AuthenticatedCredential;
use crate::routes::lifecycle::{
    LifecycleRoutesState, error_response, invalid_body, require_permission,
};

pub async fn list(State(state): State<LifecycleRoutesState>) -> Json<Vec<ServerDto>> {
    let mut servers: Vec<ServerDto> = state
        .servers()
        .into_iter()
        .map(|server| ServerDto {
            id: server.id,
            name: server.name,
            directory: server.directory,
            server_type: server.server_type,
            java_flavor: server.java_flavor,
            game_port: server.game_port,
            host_address: None,
        })
        .collect();
    servers.extend(
        TransferServerStore::global()
            .snapshot()
            .iter()
            .map(config_server_to_dto),
    );
    Json(servers)
}

pub async fn import(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<ServerImportRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Fleet) {
        return response;
    }

    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let Some(action) = body.action.clone().filter(|value| !value.trim().is_empty()) else {
        return invalid_body("missing_action", "action is required.");
    };
    let Some(source_path) = body
        .source_path
        .clone()
        .filter(|value| !value.trim().is_empty())
    else {
        return invalid_body("missing_source_path", "sourcePath is required.");
    };

    if action == "scan" {
        return Json(ServerImportScanResponseDto {
            success: true,
            message: "Paper server directory scan completed.".to_string(),
            source_path: Some(source_path),
            is_zip: Some(false),
            server_type: Some("java".to_string()),
            port: None,
            max_players: None,
            eula_accepted: None,
            default_world_name: None,
            java_flavor: Some("paper".to_string()),
        })
        .into_response();
    }

    // Transfer matching is evaluated only for non-scan requests (the scan
    // branch above already returned) — `phase5-scope.md` "Transfer
    // behavior": gated on `action == "importTransfer" || importKind ==
    // "transfer" || <ext> == .msctransfer`, not on the scan route.
    let is_transfer_request = action == "importTransfer"
        || body.import_kind.as_deref() == Some("transfer")
        || source_path.to_ascii_lowercase().ends_with(".msctransfer");

    if is_transfer_request {
        return import_transfer(&state, &source_path, &body).await;
    }

    if action != "importExisting" {
        return invalid_body(
            "invalid_action",
            "action must be scan, importExisting, or importTransfer.",
        );
    }

    let display_name = body
        .display_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_display_name(&source_path));
    let operation_id = match state.begin_import_operation(&source_path) {
        Ok(operation_id) => operation_id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let request = PaperImportRequest::new(display_name, PathBuf::from(&source_path));
    let fs = StdPaperImportFileSystem;
    let mut registry = RouteRegistry { state: &state };

    match import_existing_paper_server(&fs, &mut registry, &request) {
        Ok(server) => {
            let mut result = BTreeMap::new();
            result.insert("serverId".to_string(), server.id.as_str().to_string());
            let _ = state.finish_operation_success(&operation_id, "Imported Paper server.", result);
            Json(ServerImportResultDto {
                success: true,
                message: "Imported Paper server.".to_string(),
                operation_id: Some(operation_id.as_str().to_string()),
                server_id: Some(server.id.as_str().to_string()),
                server_name: Some(server.display_name),
                imported: Some(1),
                skipped: Some(0),
                replaced: Some(false),
            })
            .into_response()
        }
        Err(error) => {
            let _ =
                state.finish_operation_failure(&operation_id, "import_error", error.to_string());
            import_error_response(error)
        }
    }
}

struct RouteRegistry<'state> {
    state: &'state LifecycleRoutesState,
}

impl msc_application::import::PaperServerRegistry for RouteRegistry<'_> {
    fn register(
        &mut self,
        server: msc_application::import::ImportedPaperServer,
    ) -> Result<(), PaperImportError> {
        self.state.register_imported_paper(server);
        Ok(())
    }
}

fn import_error_response(error: PaperImportError) -> Response {
    match error {
        PaperImportError::EmptyDisplayName => {
            invalid_body("invalid_body", "displayName cannot be empty.")
        }
        PaperImportError::NoJavaServerJar { .. } => error_response(
            axum::http::StatusCode::CONFLICT,
            "conflict",
            &error.to_string(),
        ),
        PaperImportError::ReadDirectory { .. } | PaperImportError::Registry(_) => error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        ),
    }
}

fn default_display_name(source_path: &str) -> String {
    PathBuf::from(source_path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Imported Paper Server")
        .to_string()
}

// ---------- Transfer-package import (P5.16/P5.17) ----------
//
// `msc-agent` has no unified, persisted `AppConfig`/`ConfigServer` list yet
// (Phase 4's `AgentServerRegistry` in `crates/msc-agent/src/routes/lifecycle.rs`
// only tracks Paper-folder imports, and neither this step nor P5.16 lists
// that file). Rather than extend that Phase 4 registry — which Cameron
// confirmed staying out of during P5.16/17's Read move — this keeps a
// second, independent list of transfer-imported servers, scoped entirely
// to this file. **Known, flagged gap:** a `replaceAll` transfer import
// therefore only backs up and replaces *this* list, never a
// Paper-folder-imported server. MSC 1's own `replaceAll` backs up and
// wipes every configured server, with no such split. See
// `docs/msc2/config-migration/phase5-scope.md` "Transfer behavior" for the
// note this step adds documenting the gap, and P5.16/17's own rolling-plan
// entries for the question raised about unifying the two registries later.

/// The transfer-imported server list this route owns. A bare `'static`
/// (rather than a `LifecycleRoutesState` field) for the same reason
/// `AgentServerRegistry` itself is `Box::leak`'d in `lifecycle.rs`: it
/// needs to outlive and be shared across every request handled by this
/// process, and this module can't add a field to `LifecycleRoutesState`
/// without editing `lifecycle.rs`. `cargo nextest run` gives every test
/// its own process, so this doesn't leak state between tests.
struct TransferServerStore {
    servers: Mutex<Vec<ConfigServer>>,
}

impl TransferServerStore {
    fn new() -> Self {
        TransferServerStore {
            servers: Mutex::new(Vec::new()),
        }
    }

    fn global() -> &'static TransferServerStore {
        static STORE: OnceLock<TransferServerStore> = OnceLock::new();
        STORE.get_or_init(TransferServerStore::new)
    }

    fn snapshot(&self) -> Vec<ConfigServer> {
        self.servers.lock().unwrap().clone()
    }

    fn merge(&self, new_servers: Vec<ConfigServer>) {
        self.servers.lock().unwrap().extend(new_servers);
    }

    fn replace_all(&self, new_servers: Vec<ConfigServer>) {
        *self.servers.lock().unwrap() = new_servers;
    }

    fn existing_java_ports(&self) -> Vec<i64> {
        self.snapshot()
            .iter()
            .filter(|server| server.server_type == ServerType::Java)
            .filter_map(java_server_port)
            .collect()
    }

    fn existing_bedrock_ports(&self) -> Vec<i64> {
        self.snapshot()
            .iter()
            .filter(|server| server.server_type == ServerType::Bedrock)
            .filter_map(|server| server.bedrock_port)
            .collect()
    }

    fn export_inputs(&self) -> Vec<TransferExportServerInput> {
        self.snapshot()
            .into_iter()
            .map(|server| TransferExportServerInput {
                server,
                // `PaperVersionSidecarManager` isn't ported (Phase 7
                // territory, per `transfer.rs`'s own doc comment) — this
                // route has no sidecar to read these from either.
                paper_mc_version: None,
                paper_build: None,
            })
            .collect()
    }
}

/// A Java server's live port, read from its own `server.properties` —
/// `ConfigServer` itself carries no port field for Java (only
/// `bedrock_port` for Bedrock); the transfer format tracks it out-of-band
/// on `TransferServerEntry.java_port` for the same reason.
fn java_server_port(server: &ConfigServer) -> Option<i64> {
    let contents =
        std::fs::read_to_string(Path::new(&server.server_dir).join("server.properties")).ok()?;
    contents
        .lines()
        .find_map(|line| line.strip_prefix("server-port="))
        .and_then(|value| value.trim().parse::<i64>().ok())
}

fn config_server_to_dto(server: &ConfigServer) -> ServerDto {
    let is_java = server.server_type == ServerType::Java;
    ServerDto {
        id: server.id.clone(),
        name: server.display_name.clone(),
        directory: server.server_dir.clone(),
        server_type: server.server_type.raw_value().to_string(),
        java_flavor: is_java.then(|| server.java_flavor.raw_value().to_string()),
        game_port: if is_java {
            java_server_port(server)
        } else {
            server.bedrock_port
        },
        host_address: None,
    }
}

/// The seam the plan's "event-recording fakes" hang off — everything
/// `import_transfer` needs from the outside world, injectable so
/// P5.16/P5.17's ordering tests can prove call order and short-circuiting
/// without a real `.msctransfer` file for every case. The production
/// implementation ([`RealTransferImportPorts`]) is real, disk-backed I/O,
/// matching this crate's own precedent (`transfer.rs`'s tests use real
/// temp-directory trees, not fakes, for genuinely disk-shaped work) —
/// only [`wipe_all_secrets`](TransferImportPorts::wipe_all_secrets) is
/// necessarily a stand-in, see its doc comment.
trait TransferImportPorts {
    fn backup(&self, servers: &[TransferExportServerInput], dest_path: &Path)
    -> Result<(), String>;
    fn inspect(
        &self,
        package_path: &Path,
        staging_root: &Path,
        existing_java_ports: &[i64],
        existing_bedrock_ports: &[i64],
    ) -> Result<TransferInspection, String>;
    fn apply(
        &self,
        inspection: &TransferInspection,
        request: &TransferApplyRequest,
    ) -> TransferApplyResult;
    /// Ports `KeychainManager.deleteAllMSCSecrets` — MSC 1 wipes the
    /// owner's own Remote API token, guest token, playit key, CurseForge
    /// key, and every per-server Xbox broadcast password on a successful
    /// `replaceAll` (`KeychainManager.swift:132-152`).
    ///
    /// **Flagged gap, not a silent no-op:** `LifecycleRoutesState` (this
    /// route's only state) doesn't hold a `SecretStore` — the owner
    /// credential lives in a separate `AuthState` wired up in
    /// `auth.rs`/`main.rs`, neither of which is in P5.16/P5.17's file
    /// list. This step proves the *ordering* contract (never called
    /// before a successful backup, never called on `merge`) with a
    /// recording fake in tests; wiring a real wipe through needs
    /// `AuthState`'s `SecretStore` threaded into this route, which is
    /// follow-up work outside this step's scope.
    fn wipe_all_secrets(&self);
}

struct RealTransferImportPorts;

impl TransferImportPorts for RealTransferImportPorts {
    fn backup(
        &self,
        servers: &[TransferExportServerInput],
        dest_path: &Path,
    ) -> Result<(), String> {
        if let Some(parent) = dest_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = std::fs::File::create(dest_path).map_err(|error| error.to_string())?;
        let request = TransferExportRequest {
            servers: servers.to_vec(),
            created_at: iso8601_now(),
            source_machine_name: agent_host_name(),
            app_config_version: AppConfig::LATEST_CONFIG_VERSION,
        };
        export_server_transfer(&request, file)
            .map(|_manifest| ())
            .map_err(|error| error.to_string())
    }

    fn inspect(
        &self,
        package_path: &Path,
        staging_root: &Path,
        existing_java_ports: &[i64],
        existing_bedrock_ports: &[i64],
    ) -> Result<TransferInspection, String> {
        inspect_transfer_package(
            package_path,
            staging_root,
            existing_java_ports,
            existing_bedrock_ports,
        )
        .map_err(|error| error.to_string())
    }

    fn apply(
        &self,
        inspection: &TransferInspection,
        request: &TransferApplyRequest,
    ) -> TransferApplyResult {
        apply_transfer_import(inspection, request)
    }

    fn wipe_all_secrets(&self) {
        // See `TransferImportPorts::wipe_all_secrets`'s doc comment: no
        // `SecretStore` is reachable from this route today.
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransferMode {
    Merge,
    ReplaceAll,
}

impl TransferMode {
    /// Anything other than `replaceAll` — including absent or
    /// unrecognized — defaults to merge (`phase5-scope.md` "Transfer
    /// behavior", pinned against source line 501).
    fn from_dto(value: Option<&str>) -> Self {
        if value == Some("replaceAll") {
            Self::ReplaceAll
        } else {
            Self::Merge
        }
    }
}

struct TransferImportPlan {
    package_path: PathBuf,
    mode: TransferMode,
    backup_path: Option<String>,
    java_port_overrides: HashMap<String, i64>,
    bedrock_port_overrides: HashMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TransferImportRouteError {
    BackupPathRequired,
    BackupFailed(String),
    InvalidPackage(String),
}

/// Ports `serverImportProvider`'s orchestration (`phase5-scope.md`
/// "Transfer behavior"): for `replaceAll`, back up the current server set
/// *before* inspecting or applying anything, and fail the whole request
/// if that backup fails — `merge` skips the backup precondition entirely.
/// Only on success does this register the imported servers into `store`
/// (merge appends; `replaceAll` also wipes secrets and replaces the list
/// wholesale).
fn perform_transfer_import(
    ports: &dyn TransferImportPorts,
    store: &TransferServerStore,
    servers_root: &Path,
    staging_root: &Path,
    plan: &TransferImportPlan,
) -> Result<TransferApplyResult, TransferImportRouteError> {
    if plan.mode == TransferMode::ReplaceAll {
        let backup_path = plan
            .backup_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(TransferImportRouteError::BackupPathRequired)?;
        ports
            .backup(&store.export_inputs(), Path::new(backup_path))
            .map_err(TransferImportRouteError::BackupFailed)?;
    }

    let inspection = ports
        .inspect(
            &plan.package_path,
            staging_root,
            &store.existing_java_ports(),
            &store.existing_bedrock_ports(),
        )
        .map_err(TransferImportRouteError::InvalidPackage)?;

    let apply_request = TransferApplyRequest {
        servers_root: servers_root.to_path_buf(),
        java_port_overrides: plan.java_port_overrides.clone(),
        bedrock_port_overrides: plan.bedrock_port_overrides.clone(),
    };
    let result = ports.apply(&inspection, &apply_request);

    if plan.mode == TransferMode::ReplaceAll {
        ports.wipe_all_secrets();
        store.replace_all(result.servers.clone());
    } else {
        store.merge(result.servers.clone());
    }

    let _ = std::fs::remove_dir_all(staging_root);
    Ok(result)
}

async fn import_transfer(
    state: &LifecycleRoutesState,
    source_path: &str,
    body: &ServerImportRequestDto,
) -> Response {
    let plan = TransferImportPlan {
        package_path: PathBuf::from(source_path),
        mode: TransferMode::from_dto(body.transfer_mode.as_deref()),
        backup_path: body.backup_path.clone(),
        java_port_overrides: body.java_port_overrides.clone(),
        bedrock_port_overrides: body.bedrock_port_overrides.clone(),
    };
    let replace_all = plan.mode == TransferMode::ReplaceAll;

    let operation_id = match state.begin_import_operation(source_path) {
        Ok(operation_id) => operation_id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };

    let staging_root = transfer_staging_root();
    let result = perform_transfer_import(
        &RealTransferImportPorts,
        TransferServerStore::global(),
        &transfer_servers_root(),
        &staging_root,
        &plan,
    );

    match result {
        Ok(applied) => {
            let mode_note = if replace_all {
                " (replaced existing set)"
            } else {
                ""
            };
            let message = format!(
                "Transfer import complete: {} added, {} skipped{mode_note}.",
                applied.imported, applied.skipped
            );
            let mut result_map = BTreeMap::new();
            result_map.insert("imported".to_string(), applied.imported.to_string());
            result_map.insert("skipped".to_string(), applied.skipped.to_string());
            let _ = state.finish_operation_success(&operation_id, &message, result_map);
            Json(ServerImportResultDto {
                success: true,
                message,
                operation_id: Some(operation_id.as_str().to_string()),
                server_id: applied.servers.first().map(|server| server.id.clone()),
                server_name: applied
                    .servers
                    .first()
                    .map(|server| server.display_name.clone()),
                imported: Some(applied.imported as i64),
                skipped: Some(applied.skipped as i64),
                replaced: Some(replace_all),
            })
            .into_response()
        }
        Err(error) => {
            let _ = state.finish_operation_failure(
                &operation_id,
                transfer_error_code(&error),
                transfer_error_message(&error),
            );
            transfer_import_error_response(error)
        }
    }
}

fn transfer_error_code(error: &TransferImportRouteError) -> &'static str {
    match error {
        TransferImportRouteError::BackupPathRequired => "backup_path_required",
        TransferImportRouteError::BackupFailed(_) => "backup_failed",
        TransferImportRouteError::InvalidPackage(_) => "invalid_transfer_package",
    }
}

fn transfer_error_message(error: &TransferImportRouteError) -> String {
    match error {
        TransferImportRouteError::BackupPathRequired => {
            "backupPath is required for a replaceAll transfer import.".to_string()
        }
        TransferImportRouteError::BackupFailed(message) => format!("backup_failed: {message}"),
        TransferImportRouteError::InvalidPackage(message) => message.clone(),
    }
}

fn transfer_import_error_response(error: TransferImportRouteError) -> Response {
    match &error {
        TransferImportRouteError::BackupPathRequired => {
            invalid_body(transfer_error_code(&error), &transfer_error_message(&error))
        }
        TransferImportRouteError::BackupFailed(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            transfer_error_code(&error),
            &transfer_error_message(&error),
        ),
        TransferImportRouteError::InvalidPackage(_) => {
            invalid_body(transfer_error_code(&error), &transfer_error_message(&error))
        }
    }
}

/// `configManager.serversRootURL` has no Rust equivalent yet (no
/// `AppConfig` is loaded in `msc-agent` — see this section's header
/// comment), so this resolves the same way `auth.rs`'s
/// `default_persistent_service_store` resolves the credential registry
/// path: an env var override, falling back to the OS temp dir. Not
/// durable-by-default; flagged for Cameron alongside the registry-split
/// gap above.
fn transfer_servers_root() -> PathBuf {
    std::env::var_os("MSC2_TRANSFER_SERVERS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("msc2-transfer-servers"))
}

fn transfer_staging_root() -> PathBuf {
    std::env::temp_dir().join(format!("msc2-transfer-staging-{}", unique_suffix()))
}

fn unique_suffix() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{nanos}-{count}", std::process::id())
}

fn agent_host_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "MSC 2 Agent".to_string())
}

fn iso8601_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86_400;
    let remainder = secs % 86_400;
    let (hour, minute, second) = (remainder / 3600, (remainder % 3600) / 60, remainder % 60);
    let (year, month, day) = civil_from_days(days as i64);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days` — days-since-epoch to a proleptic
/// Gregorian (year, month, day), used instead of adding a date/time crate
/// dependency for one formatted timestamp.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::CredentialRole;
    use crate::routes::operations::OperationsState;
    use crate::ws::console::ConsoleState;
    use msc_application::transfer::{TransferManifest, TransferServerConflict};

    // ---------- P5.16: replace-all backup ordering ----------

    #[derive(Default)]
    struct RecordingPorts {
        events: Mutex<Vec<&'static str>>,
        backup_result: Mutex<Option<Result<(), String>>>,
        inspect_result: Mutex<Option<Result<TransferInspection, String>>>,
        apply_result: Mutex<Option<TransferApplyResult>>,
    }

    impl TransferImportPorts for RecordingPorts {
        fn backup(
            &self,
            _servers: &[TransferExportServerInput],
            _dest_path: &Path,
        ) -> Result<(), String> {
            self.events.lock().unwrap().push("backup");
            self.backup_result.lock().unwrap().take().unwrap_or(Ok(()))
        }

        fn inspect(
            &self,
            _package_path: &Path,
            _staging_root: &Path,
            _existing_java_ports: &[i64],
            _existing_bedrock_ports: &[i64],
        ) -> Result<TransferInspection, String> {
            self.events.lock().unwrap().push("inspect");
            self.inspect_result
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(|| Err("no inspect result configured".to_string()))
        }

        fn apply(
            &self,
            _inspection: &TransferInspection,
            _request: &TransferApplyRequest,
        ) -> TransferApplyResult {
            self.events.lock().unwrap().push("apply");
            self.apply_result
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(empty_apply_result)
        }

        fn wipe_all_secrets(&self) {
            self.events.lock().unwrap().push("wipe_all_secrets");
        }
    }

    fn empty_apply_result() -> TransferApplyResult {
        TransferApplyResult {
            servers: Vec::new(),
            imported: 0,
            skipped: 0,
        }
    }

    fn sample_config_server(id: &str) -> ConfigServer {
        ConfigServer::new(
            id,
            format!("Server {id}"),
            "/tmp/does-not-matter",
            "",
            2.0,
            4.0,
        )
    }

    fn empty_inspection() -> TransferInspection {
        TransferInspection {
            staging_root: PathBuf::from("/tmp/staging"),
            manifest: TransferManifest {
                format_version: 2,
                app_config_version: 1,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                source_machine_name: "Test".to_string(),
                servers: Vec::new(),
            },
            conflicts: Vec::<TransferServerConflict>::new(),
        }
    }

    fn plan(mode: TransferMode, backup_path: Option<&str>) -> TransferImportPlan {
        TransferImportPlan {
            package_path: PathBuf::from("/tmp/does-not-exist.msctransfer"),
            mode,
            backup_path: backup_path.map(str::to_string),
            java_port_overrides: HashMap::new(),
            bedrock_port_overrides: HashMap::new(),
        }
    }

    #[test]
    fn transfer_replace_all_missing_backup_path_rejects_before_any_port_call() {
        let ports = RecordingPorts::default();
        let store = TransferServerStore::new();
        let plan = plan(TransferMode::ReplaceAll, None);

        let result = perform_transfer_import(
            &ports,
            &store,
            Path::new("/tmp/servers-root"),
            Path::new("/tmp/staging-root"),
            &plan,
        );

        assert_eq!(result, Err(TransferImportRouteError::BackupPathRequired));
        assert!(ports.events.lock().unwrap().is_empty());
        assert!(store.snapshot().is_empty());
    }

    #[test]
    fn transfer_replace_all_backup_failure_stops_before_inspect_and_apply() {
        let ports = RecordingPorts::default();
        *ports.backup_result.lock().unwrap() = Some(Err("disk full".to_string()));
        let store = TransferServerStore::new();
        let plan = plan(TransferMode::ReplaceAll, Some("/tmp/backup.msctransfer"));

        let result = perform_transfer_import(
            &ports,
            &store,
            Path::new("/tmp/servers-root"),
            Path::new("/tmp/staging-root"),
            &plan,
        );

        assert_eq!(
            result,
            Err(TransferImportRouteError::BackupFailed(
                "disk full".to_string()
            ))
        );
        assert_eq!(*ports.events.lock().unwrap(), vec!["backup"]);
        assert!(store.snapshot().is_empty());
    }

    #[test]
    fn transfer_replace_all_success_calls_backup_then_inspect_then_apply_then_wipe() {
        let ports = RecordingPorts::default();
        *ports.inspect_result.lock().unwrap() = Some(Ok(empty_inspection()));
        *ports.apply_result.lock().unwrap() = Some(TransferApplyResult {
            servers: vec![sample_config_server("NEW-1")],
            imported: 1,
            skipped: 0,
        });
        let store = TransferServerStore::new();
        store.replace_all(vec![sample_config_server("OLD-1")]);
        let plan = plan(TransferMode::ReplaceAll, Some("/tmp/backup.msctransfer"));

        let result = perform_transfer_import(
            &ports,
            &store,
            Path::new("/tmp/servers-root"),
            Path::new("/tmp/staging-root-that-does-not-exist"),
            &plan,
        );

        assert!(result.is_ok());
        assert_eq!(
            *ports.events.lock().unwrap(),
            vec!["backup", "inspect", "apply", "wipe_all_secrets"]
        );
        let snapshot = store.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].id, "NEW-1");
    }

    #[test]
    fn transfer_merge_never_calls_backup_or_wipe_and_appends() {
        let ports = RecordingPorts::default();
        *ports.inspect_result.lock().unwrap() = Some(Ok(empty_inspection()));
        *ports.apply_result.lock().unwrap() = Some(TransferApplyResult {
            servers: vec![sample_config_server("NEW-1")],
            imported: 1,
            skipped: 0,
        });
        let store = TransferServerStore::new();
        store.merge(vec![sample_config_server("OLD-1")]);
        let plan = plan(TransferMode::Merge, None);

        let result = perform_transfer_import(
            &ports,
            &store,
            Path::new("/tmp/servers-root"),
            Path::new("/tmp/staging-root-that-does-not-exist"),
            &plan,
        );

        assert!(result.is_ok());
        assert_eq!(*ports.events.lock().unwrap(), vec!["inspect", "apply"]);
        let snapshot = store.snapshot();
        assert_eq!(snapshot.len(), 2);
        assert!(snapshot.iter().any(|s| s.id == "OLD-1"));
        assert!(snapshot.iter().any(|s| s.id == "NEW-1"));
    }

    // ---------- P5.17: real route wiring ----------

    fn transfer_credential() -> AuthenticatedCredential {
        AuthenticatedCredential {
            credential_id: "named".to_string(),
            label: "console".to_string(),
            role: CredentialRole::Named,
            permissions: vec![PermissionCategoryDto::Fleet],
        }
    }

    fn route_state() -> LifecycleRoutesState {
        LifecycleRoutesState::with_fake_process(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        )
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "msc2-transfer-import-route-{tag}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Builds a real, on-disk `.msctransfer` package (matching the
    /// approach `transfer_export.rs`'s own tests use — genuinely
    /// disk-shaped work, not faked) so this module's route tests exercise
    /// real inspect/apply, not just the ordering ports.
    fn build_transfer_package(source_id: &str, java_port: i64) -> PathBuf {
        let source_dir = temp_dir(&format!("source-{source_id}"));
        std::fs::write(
            source_dir.join("server.properties"),
            format!("server-port={java_port}\nmax-players=5\n"),
        )
        .unwrap();
        let server = ConfigServer::new(
            source_id,
            format!("Route Test {source_id}"),
            source_dir.to_string_lossy().into_owned(),
            "",
            2.0,
            4.0,
        );
        let package_dir = temp_dir("packages");
        let package_path = package_dir.join(format!("{source_id}.msctransfer"));
        let file = std::fs::File::create(&package_path).unwrap();
        let request = TransferExportRequest {
            servers: vec![TransferExportServerInput {
                server,
                paper_mc_version: None,
                paper_build: None,
            }],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            source_machine_name: "Test".to_string(),
            app_config_version: 1,
        };
        export_server_transfer(&request, file).unwrap();
        package_path
    }

    fn import_request(
        source_path: &Path,
        transfer_mode: Option<&str>,
        backup_path: Option<&str>,
    ) -> ServerImportRequestDto {
        ServerImportRequestDto {
            action: Some("importTransfer".to_string()),
            source_path: Some(source_path.to_string_lossy().into_owned()),
            import_kind: Some("transfer".to_string()),
            display_name: None,
            server_type: None,
            active_world_name: None,
            port: None,
            max_players: None,
            accept_eula: None,
            enable_playit: None,
            transfer_mode: transfer_mode.map(str::to_string),
            backup_path: backup_path.map(str::to_string),
            java_port_overrides: HashMap::new(),
            bedrock_port_overrides: HashMap::new(),
        }
    }

    async fn call_import(state: &LifecycleRoutesState, body: ServerImportRequestDto) -> Response {
        import(
            State(state.clone()),
            Extension(transfer_credential()),
            Ok(Json(body)),
        )
        .await
    }

    async fn response_json<T: serde::de::DeserializeOwned>(response: Response) -> (StatusCode, T) {
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn transfer_import_route_merge_appends_across_two_imports() {
        let state = route_state();
        let package_one = build_transfer_package("ROUTE-A", 25601);
        let package_two = build_transfer_package("ROUTE-B", 25602);

        let (status_one, result_one): (StatusCode, ServerImportResultDto) =
            response_json(call_import(&state, import_request(&package_one, None, None)).await)
                .await;
        assert_eq!(status_one, StatusCode::OK);
        assert_eq!(result_one.imported, Some(1));
        assert_eq!(result_one.replaced, Some(false));

        let (status_two, result_two): (StatusCode, ServerImportResultDto) =
            response_json(call_import(&state, import_request(&package_two, None, None)).await)
                .await;
        assert_eq!(status_two, StatusCode::OK);
        assert_eq!(result_two.imported, Some(1));

        let ids: Vec<String> = TransferServerStore::global()
            .snapshot()
            .iter()
            .map(|s| s.display_name.clone())
            .collect();
        assert!(ids.contains(&"Route Test ROUTE-A".to_string()));
        assert!(ids.contains(&"Route Test ROUTE-B".to_string()));
    }

    #[tokio::test]
    async fn transfer_import_route_replace_all_without_backup_path_is_rejected() {
        let state = route_state();
        let package = build_transfer_package("ROUTE-C", 25603);

        let response =
            call_import(&state, import_request(&package, Some("replaceAll"), None)).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(text.contains("backup_path_required"));
    }

    #[tokio::test]
    async fn transfer_import_route_replace_all_backs_up_before_replacing() {
        let state = route_state();
        let first_package = build_transfer_package("ROUTE-D", 25604);
        let (status, _result): (StatusCode, ServerImportResultDto) =
            response_json(call_import(&state, import_request(&first_package, None, None)).await)
                .await;
        assert_eq!(status, StatusCode::OK);

        let second_package = build_transfer_package("ROUTE-E", 25605);
        let backup_path = temp_dir("backups").join("before-replace-all.msctransfer");

        let (status, result): (StatusCode, ServerImportResultDto) = response_json(
            call_import(
                &state,
                import_request(
                    &second_package,
                    Some("replaceAll"),
                    Some(backup_path.to_str().unwrap()),
                ),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(result.replaced, Some(true));
        assert!(
            backup_path.is_file(),
            "backup file was not written before replaceAll"
        );

        let names: Vec<String> = TransferServerStore::global()
            .snapshot()
            .iter()
            .map(|s| s.display_name.clone())
            .collect();
        assert_eq!(names, vec!["Route Test ROUTE-E".to_string()]);
    }
}
