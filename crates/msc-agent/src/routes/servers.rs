//! `GET /v1/servers` and `POST /v1/servers/import`.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use msc_api::dto::{
    ErrorDto, PermissionCategoryDto, ServerCreateRequestDto, ServerCreateResultDto,
    ServerDeleteRequestDto, ServerDeleteResultDto, ServerDto, ServerEulaRequestDto,
    ServerEulaResultDto, ServerImportRequestDto, ServerImportResultDto,
    ServerImportScanResponseDto, ServerImportWorldDto, ServerRenameRequestDto,
    ServerRenameResultDto,
};
use msc_application::fleet::{self, AcceptEulaError, DeleteServerError, RenameServerError};
use msc_application::import::{
    DetectedWorld, RawImportError, RawImportOverrides, RawImportRequest, RawImportSource,
    ScannedServerInfo, StdRawImportFileSystem, import_raw_server, rescan_and_import_servers,
    scan_server_directory, scan_zip_source,
};
use msc_application::modpacks;
use msc_application::provisioning::{
    self, CreateFromPackError, CreateServerError, NewServerRequest, PackServerRequest, WorldSource,
};
use msc_application::transfer::{
    TransferApplyRequest, TransferApplyResult, TransferExportRequest, TransferExportServerInput,
    TransferInspection, apply_transfer_import, export_server_transfer, inspect_transfer_package,
};
use msc_application::world_safety::{self, SafetyConfirmation};
use msc_domain::app_config_schema::{AppConfig, ConfigServer};
use msc_domain::identity::{JavaServerFlavor, ServerProvisioningKind, ServerType};
use msc_domain::operation::OperationId;
use msc_infrastructure::addon_provider::HttpTransport as AddonHttpTransport;
use msc_infrastructure::fs::{FileSystem, StdFileSystem};
use msc_infrastructure::jar_provider::HttpTransport;
use msc_infrastructure::java_runtime_detection;
use msc_infrastructure::process::ProcessSupervisor;

use crate::auth::{AuthenticatedCredential, production_secret_store};
use crate::routes::lifecycle::{
    LifecycleRoutesState, TryMutateError, error_response, invalid_body, require_permission,
};
use crate::routes::operations::operation_error_response;
use crate::routes::worlds::{StagedUpload, StagingStore, now_unix, staging_root};

pub async fn list(State(state): State<LifecycleRoutesState>) -> Json<Vec<ServerDto>> {
    let active_server_id = state.active_server_id();
    let servers: Vec<ServerDto> = state
        .servers()
        .into_iter()
        .map(|server| {
            let runtime = (server.server_type == ServerType::Bedrock.raw_value()
                && active_server_id.as_deref() == Some(server.id.as_str()))
            .then(|| state.bedrock_runtime_state());
            ServerDto {
                id: server.id,
                name: server.name,
                directory: server.directory,
                server_type: server.server_type,
                java_flavor: server.java_flavor,
                game_port: server.game_port,
                host_address: None,
                runtime,
            }
        })
        .collect();
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

    if action == "rescan" {
        return rescan_import(&state);
    }

    let Some(source_path) = body
        .source_path
        .clone()
        .filter(|value| !value.trim().is_empty())
    else {
        return invalid_body("missing_source_path", "sourcePath is required.");
    };

    if action == "scan" {
        return perform_raw_scan(&source_path, body.import_kind.as_deref());
    }

    // Transfer matching is evaluated only for non-scan requests (the scan
    // branch above already returned) — `phase5-scope.md` "Transfer
    // behavior": gated on `action == "importTransfer" || importKind ==
    // "transfer" || <ext> == .msctransfer`, not on the scan route.
    let is_transfer_request = action == "importTransfer"
        || body.import_kind.as_deref() == Some("transfer")
        || source_path.to_ascii_lowercase().ends_with(".msctransfer");

    if is_transfer_request {
        return import_transfer(&state, &source_path, &body);
    }

    if action != "importExisting" {
        return invalid_body(
            "invalid_action",
            "action must be scan, importExisting, importTransfer, or rescan.",
        );
    }

    import_raw(&state, &source_path, &body)
}

fn rescan_import(state: &LifecycleRoutesState) -> Response {
    let servers_root = state.servers_root();
    let operation_id = match state.begin_import_operation(&servers_root.to_string_lossy()) {
        Ok(operation_id) => operation_id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let worker_state = state.clone();
    let worker_operation_id = operation_id.clone();
    tokio::spawn(async move {
        let failure_state = worker_state.clone();
        let failure_operation_id = worker_operation_id.clone();
        if let Err(error) = tokio::task::spawn_blocking(move || {
            run_rescan_import(worker_state, worker_operation_id, servers_root)
        })
        .await
        {
            let _ = failure_state.finish_operation_failure(
                &failure_operation_id,
                "background_worker_failed",
                error.to_string(),
            );
        }
    });

    accepted_import_response(&operation_id, "Recovery rescan accepted.")
}

fn run_rescan_import(
    state: LifecycleRoutesState,
    operation_id: OperationId,
    servers_root: PathBuf,
) {
    let should_cancel = state.operations().cancellation_check(&operation_id);
    if should_cancel() {
        let _ = state
            .operations()
            .cancel(&operation_id, "Recovery rescan cancelled before scanning.");
        return;
    }
    let existing_server_dirs = state
        .export_inputs()
        .into_iter()
        .map(|input| input.server.server_dir)
        .collect::<Vec<_>>();
    let result = rescan_and_import_servers(
        &StdRawImportFileSystem,
        &servers_root,
        &existing_server_dirs,
    );
    if should_cancel() {
        let _ = state.operations().cancel(
            &operation_id,
            "Recovery rescan cancelled before registration.",
        );
        return;
    }
    let first_lifecycle_server_id = result.added.first().map(|server| server.id.clone());
    let first_added = result.added.first().cloned();
    let added_servers = result.added.clone();
    let imported = result.added.len() as i64;
    let skipped = result.skipped as i64;
    match state.register_imported_config_servers(result.added, false) {
        Ok(statuses) => {
            if let Err(error) = state.provision_imported_bedrock_servers(&added_servers) {
                let _ = state.finish_operation_failure(
                    &operation_id,
                    "bedrock_provisioning_failed",
                    error.to_string(),
                );
                return;
            }
            if let Some(server_id) = first_lifecycle_server_id
                && statuses.iter().any(|(id, status)| {
                    id == &server_id
                        && matches!(
                            status,
                            crate::routes::lifecycle::ReconciliationStatus::Ready
                        )
                })
            {
                let _ = state.select_active_server(server_id);
            }
            let message = format!("Recovery rescan complete: {imported} added, {skipped} skipped.");
            let mut result_map = BTreeMap::new();
            result_map.insert("imported".to_string(), imported.to_string());
            result_map.insert("skipped".to_string(), skipped.to_string());
            result_map.insert("replaced".to_string(), "false".to_string());
            if let Some(server) = first_added {
                result_map.insert("serverId".to_string(), server.id);
                result_map.insert("serverName".to_string(), server.display_name);
            }
            let _ = state.finish_operation_success(&operation_id, &message, result_map);
        }
        Err(error) => {
            let _ = state.finish_operation_failure(
                &operation_id,
                "rescan_save_failed",
                error.to_string(),
            );
        }
    }
}

// ---------- Raw folder/ZIP scan and import (P5.19-P5.21) ----------
//
// Wires P5.19's read-only scanner and P5.20's mutating importer to the
// route — the broad `folder|zip|auto` half MSC 1 actually ships, replacing
// this route's earlier Paper-only stand-in (`import_existing_paper_server`,
// still ported and unit-tested in `msc-application` for Phase 4's own
// lifecycle slice, just no longer this route's `importExisting` target).

/// `folder`/`zip`/`auto` (or an absent `importKind`) resolve to `false`
/// (folder) or a `.zip`-extension check — mirroring `handleImportDrop`'s
/// own `ext == "zip"` inference (`AddServerWizardView.swift:2231-2246`),
/// the only place MSC 1 itself decides "is this a zip".
fn resolve_is_zip(import_kind: Option<&str>, source_path: &str) -> bool {
    match import_kind {
        Some("zip") => true,
        Some("folder") => false,
        _ => source_path.to_ascii_lowercase().ends_with(".zip"),
    }
}

/// Route-level boundary validation, not a port of `scanServerDirectory`
/// itself (which never rejects a missing path — P5.19's own doc comment).
/// MSC 1 only ever scans a path an `NSOpenPanel` guaranteed exists; this
/// route accepts a raw string over HTTP, so it checks existence itself
/// rather than silently returning a defaulted, low-information scan
/// result for a typo'd path. Reuses this endpoint's own documented 404
/// `source_not_found` code (`openapi.json`), previously unwired for scan.
fn perform_raw_scan(source_path: &str, import_kind: Option<&str>) -> Response {
    let is_zip = resolve_is_zip(import_kind, source_path);
    let path = Path::new(source_path);

    if is_zip {
        if !path.is_file() {
            return source_not_found_response(source_path);
        }
        match scan_zip_source(path) {
            Ok(info) => Json(scan_response_dto(source_path, true, &info)).into_response(),
            Err(error) => raw_import_error_response(error),
        }
    } else {
        if !path.is_dir() {
            return source_not_found_response(source_path);
        }
        let info = scan_server_directory(&StdRawImportFileSystem, path);
        Json(scan_response_dto(source_path, false, &info)).into_response()
    }
}

fn source_not_found_response(source_path: &str) -> Response {
    error_response(
        StatusCode::NOT_FOUND,
        "not_found",
        &format!("import source not found: {source_path}"),
    )
}

fn scan_response_dto(
    source_path: &str,
    is_zip: bool,
    info: &ScannedServerInfo,
) -> ServerImportScanResponseDto {
    ServerImportScanResponseDto {
        success: true,
        message: "Server directory scan completed.".to_string(),
        source_path: Some(source_path.to_string()),
        is_zip: Some(is_zip),
        server_type: Some(info.server_type.raw_value().to_string()),
        port: Some(info.port),
        max_players: Some(info.max_players),
        eula_accepted: Some(info.eula_accepted),
        worlds: info.worlds.iter().map(world_to_dto).collect(),
        default_world_name: Some(info.default_world_name.clone()),
        java_flavor: info
            .java_flavor
            .map(|flavor| flavor.raw_value().to_string()),
        detected_mc_version: info.detected_mc_version.clone(),
        detected_loader_version: info.detected_loader_version.clone(),
    }
}

/// `id` mirrors MSC 1's own `DetectedWorld: Identifiable` (`var id: String
/// { name }`); `dimensionsLabel` mirrors its computed property exactly
/// (`AppViewModel+ServerImport.swift:36-49`).
fn world_to_dto(world: &DetectedWorld) -> ServerImportWorldDto {
    let mut dims = vec!["Overworld"];
    if world.has_nether {
        dims.push("Nether");
    }
    if world.has_end {
        dims.push("End");
    }
    ServerImportWorldDto {
        id: world.name.clone(),
        name: world.name.clone(),
        size_bytes: world.size_bytes as i64,
        dimensions_label: dims.join(" + "),
    }
}

/// Ports `importExistingServer`'s registration step for this route: builds
/// the copied/extracted server via P5.20's `import_raw_server`, then
/// persists it through P5.27's single `AppConfig` state. When the request
/// omits `serverType`, the route scans the source and infers Java vs.
/// Bedrock instead of falling back to the old Phase 4 Paper-only stand-in.
/// Imported Java servers are selected as active immediately, which makes
/// settings/start/stop use the same persisted record.
fn import_raw(
    state: &LifecycleRoutesState,
    source_path: &str,
    body: &ServerImportRequestDto,
) -> Response {
    let display_name = body
        .display_name
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_display_name(source_path));
    let server_type = match body.server_type.as_deref() {
        Some(raw) => match ServerType::from_raw_value(raw) {
            Some(server_type) => Some(server_type),
            None => {
                return invalid_body("invalid_server_type", "serverType must be java or bedrock.");
            }
        },
        None => None,
    };
    let is_zip = resolve_is_zip(body.import_kind.as_deref(), source_path);
    let source_path = PathBuf::from(source_path);
    if (is_zip && !source_path.is_file()) || (!is_zip && !source_path.is_dir()) {
        return source_not_found_response(&source_path.to_string_lossy());
    }
    let source = if is_zip {
        RawImportSource::Zip(source_path)
    } else {
        RawImportSource::Folder(source_path)
    };

    let operation_target = match &source {
        RawImportSource::Folder(path) | RawImportSource::Zip(path) => path.to_string_lossy(),
    };
    let operation_id = match state.begin_import_operation(&operation_target) {
        Ok(operation_id) => operation_id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };
    let servers_root = state.servers_root();
    let overrides = RawImportOverrides {
        port: body.port,
        max_players: body.max_players,
        active_world_name: body.active_world_name.clone(),
        eula_accepted: body.accept_eula,
        enable_playit: body.enable_playit,
    };

    let worker_state = state.clone();
    let worker_operation_id = operation_id.clone();
    tokio::spawn(async move {
        let failure_state = worker_state.clone();
        let failure_operation_id = worker_operation_id.clone();
        if let Err(error) = tokio::task::spawn_blocking(move || {
            run_raw_import(
                worker_state,
                worker_operation_id,
                display_name,
                server_type,
                source,
                servers_root,
                overrides,
            )
        })
        .await
        {
            let _ = failure_state.finish_operation_failure(
                &failure_operation_id,
                "background_worker_failed",
                error.to_string(),
            );
        }
    });

    accepted_import_response(&operation_id, "Server import accepted.")
}

fn run_raw_import(
    state: LifecycleRoutesState,
    operation_id: OperationId,
    display_name: String,
    server_type: Option<ServerType>,
    source: RawImportSource,
    servers_root: PathBuf,
    overrides: RawImportOverrides,
) {
    let should_cancel = state.operations().cancellation_check(&operation_id);
    if should_cancel() {
        let _ = state
            .operations()
            .cancel(&operation_id, "Server import cancelled before copying.");
        return;
    }
    let server_type = match server_type {
        Some(server_type) => server_type,
        None => match infer_import_server_type_from_source(&source) {
            Ok(server_type) => server_type,
            Err(error) => {
                let _ = state.finish_operation_failure(
                    &operation_id,
                    raw_import_error_code(&error),
                    error.to_string(),
                );
                return;
            }
        },
    };
    let request = RawImportRequest {
        display_name,
        server_type,
        source,
        servers_root,
        overrides,
    };
    match import_raw_server(&request, &agent_home_dir()) {
        Ok(imported) => {
            let config = imported.config;
            let imported_server = config.clone();
            if should_cancel() {
                remove_unregistered_raw_import(&request, &config);
                let _ = state.operations().cancel(
                    &operation_id,
                    "Server import cancelled before registration.",
                );
                return;
            }
            let message = format!("Imported {} server.", server_type.raw_value());
            let mut result = BTreeMap::new();
            result.insert("serverId".to_string(), config.id.clone());
            result.insert("serverName".to_string(), config.display_name.clone());
            result.insert("imported".to_string(), "1".to_string());
            result.insert("skipped".to_string(), "0".to_string());
            result.insert("replaced".to_string(), "false".to_string());
            let imported_server_id = config.id.clone();
            match state.register_imported_config_servers(vec![config], false) {
                Ok(statuses) => {
                    if let Err(error) = state
                        .provision_imported_bedrock_servers(std::slice::from_ref(&imported_server))
                    {
                        let _ = state.finish_operation_failure(
                            &operation_id,
                            "bedrock_provisioning_failed",
                            error.to_string(),
                        );
                        return;
                    }
                    let reconciled = statuses.iter().any(|(id, status)| {
                        id == &imported_server_id
                            && matches!(
                                status,
                                crate::routes::lifecycle::ReconciliationStatus::Ready
                            )
                    });
                    let runtime_ready = imported_server.server_type != ServerType::Bedrock
                        || (!state.bedrock_runtime_is_bound()
                            && !state.bedrock_runtime_is_busy()
                            && state.bedrock_runtime_state().state == "available");
                    let ready = reconciled && runtime_ready;
                    result.insert("ready".to_string(), ready.to_string());
                    if ready {
                        let _ = state.select_active_server(imported_server_id);
                    }
                    let _ = state.finish_operation_success(&operation_id, &message, result);
                }
                Err(error) => {
                    let _ = state.finish_operation_failure(
                        &operation_id,
                        "internal_error",
                        error.to_string(),
                    );
                }
            }
        }
        Err(error) => {
            let _ = state.finish_operation_failure(
                &operation_id,
                raw_import_error_code(&error),
                error.to_string(),
            );
        }
    }
}

fn infer_import_server_type_from_source(
    source: &RawImportSource,
) -> Result<ServerType, RawImportError> {
    match source {
        RawImportSource::Zip(path) => scan_zip_source(path).map(|info| info.server_type),
        RawImportSource::Folder(path) => {
            Ok(scan_server_directory(&StdRawImportFileSystem, path).server_type)
        }
    }
}

fn remove_unregistered_raw_import(request: &RawImportRequest, config: &ConfigServer) {
    let type_root = request
        .servers_root
        .join(if request.server_type == ServerType::Java {
            "java"
        } else {
            "bedrock"
        });
    let configured = Path::new(&config.server_dir);
    let Some(first_component) = configured
        .strip_prefix(&type_root)
        .ok()
        .and_then(|relative| relative.components().next())
    else {
        return;
    };
    let _ = std::fs::remove_dir_all(type_root.join(first_component.as_os_str()));
}

fn accepted_import_response(operation_id: &OperationId, message: &str) -> Response {
    (
        StatusCode::ACCEPTED,
        Json(ServerImportResultDto {
            success: true,
            message: message.to_string(),
            operation_id: Some(operation_id.as_str().to_string()),
            server_id: None,
            server_name: None,
            imported: None,
            skipped: None,
            replaced: None,
            runtime: None,
        }),
    )
        .into_response()
}

fn raw_import_error_code(error: &RawImportError) -> &'static str {
    match error {
        RawImportError::EmptyDisplayName => "display_name_required",
        RawImportError::EmptyDestinationName => "invalid_display_name",
        RawImportError::PathSafety(_) => "invalid_path",
        RawImportError::DestinationExists { .. } => "conflict",
        RawImportError::SourceNotFound { .. } => "not_found",
        RawImportError::OpenZip(_) => "invalid_path",
        RawImportError::UnsafeZipEntry { .. } => "invalid_path",
        RawImportError::UnsafeSymlink { .. } => "invalid_path",
        RawImportError::Io(_) => "internal_error",
    }
}

fn raw_import_error_response(error: RawImportError) -> Response {
    let code = raw_import_error_code(&error);
    let message = error.to_string();
    match &error {
        RawImportError::EmptyDisplayName
        | RawImportError::EmptyDestinationName
        | RawImportError::PathSafety(_)
        | RawImportError::OpenZip(_)
        | RawImportError::UnsafeZipEntry { .. }
        | RawImportError::UnsafeSymlink { .. } => invalid_body(code, &message),
        RawImportError::DestinationExists { .. } => {
            error_response(StatusCode::CONFLICT, code, &message)
        }
        RawImportError::SourceNotFound { .. } => {
            error_response(StatusCode::NOT_FOUND, code, &message)
        }
        RawImportError::Io(_) => error_response(StatusCode::INTERNAL_SERVER_ERROR, code, &message),
    }
}

fn default_display_name(source_path: &str) -> String {
    PathBuf::from(source_path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Imported Server")
        .to_string()
}

/// `safe_path`'s own required `home_dir` parameter (used only for its
/// `ForbiddenRoot` check — see `import.rs`'s own note on `import_raw_server`
/// calling it "defense-in-depth... rather than load-bearing" here). No
/// shared HOME resolver exists elsewhere in this crate yet.
fn agent_home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

// ---------- Transfer-package import (P5.16/P5.17) ----------

trait ConfiguredServerStore {
    fn export_inputs(&self) -> Vec<TransferExportServerInput>;
    fn existing_java_ports(&self) -> Vec<i64>;
    fn existing_bedrock_ports(&self) -> Vec<i64>;
    fn wipe_replace_all_secrets(&self, previous_server_ids: &[String]) -> Result<(), String>;
    fn merge(&self, new_servers: Vec<ConfigServer>) -> Result<(), String>;
    fn replace_all(&self, new_servers: Vec<ConfigServer>) -> Result<(), String>;
}

impl ConfiguredServerStore for LifecycleRoutesState {
    fn export_inputs(&self) -> Vec<TransferExportServerInput> {
        self.export_inputs()
    }

    fn existing_java_ports(&self) -> Vec<i64> {
        self.existing_java_ports()
    }

    fn existing_bedrock_ports(&self) -> Vec<i64> {
        self.existing_bedrock_ports()
    }

    fn wipe_replace_all_secrets(&self, previous_server_ids: &[String]) -> Result<(), String> {
        self.wipe_replace_all_secrets(previous_server_ids)
    }

    fn merge(&self, new_servers: Vec<ConfigServer>) -> Result<(), String> {
        self.register_imported_config_servers(new_servers, false)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn replace_all(&self, new_servers: Vec<ConfigServer>) -> Result<(), String> {
        self.register_imported_config_servers(new_servers, true)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
struct ConfigServerStore {
    servers: Mutex<Vec<ConfigServer>>,
}

#[cfg(test)]
impl ConfigServerStore {
    fn new() -> Self {
        ConfigServerStore {
            servers: Mutex::new(Vec::new()),
        }
    }

    fn snapshot(&self) -> Vec<ConfigServer> {
        self.servers.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl ConfiguredServerStore for ConfigServerStore {
    fn export_inputs(&self) -> Vec<TransferExportServerInput> {
        self.snapshot()
            .into_iter()
            .map(|server| TransferExportServerInput {
                server,
                paper_mc_version: None,
                paper_build: None,
            })
            .collect()
    }

    fn existing_java_ports(&self) -> Vec<i64> {
        self.snapshot()
            .iter()
            .filter(|server| server.server_type == ServerType::Java)
            .filter_map(test_java_server_port)
            .collect()
    }

    fn existing_bedrock_ports(&self) -> Vec<i64> {
        self.snapshot()
            .iter()
            .filter(|server| server.server_type == ServerType::Bedrock)
            .filter_map(|server| server.bedrock_port)
            .collect()
    }

    fn wipe_replace_all_secrets(&self, _previous_server_ids: &[String]) -> Result<(), String> {
        Ok(())
    }

    fn merge(&self, new_servers: Vec<ConfigServer>) -> Result<(), String> {
        self.servers.lock().unwrap().extend(new_servers);
        Ok(())
    }

    fn replace_all(&self, new_servers: Vec<ConfigServer>) -> Result<(), String> {
        *self.servers.lock().unwrap() = new_servers;
        Ok(())
    }
}

/// A Java server's live port, read from its own `server.properties` —
/// `ConfigServer` itself carries no port field for Java (only
/// `bedrock_port` for Bedrock); the transfer format tracks it out-of-band
/// on `TransferServerEntry.java_port` for the same reason.
#[cfg(test)]
fn test_java_server_port(server: &ConfigServer) -> Option<i64> {
    let contents =
        std::fs::read_to_string(Path::new(&server.server_dir).join("server.properties")).ok()?;
    contents
        .lines()
        .find_map(|line| line.strip_prefix("server-port="))
        .and_then(|value| value.trim().parse::<i64>().ok())
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
    SecretWipeFailed(String),
    SaveFailed(String),
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
    store: &dyn ConfiguredServerStore,
    servers_root: &Path,
    staging_root: &Path,
    plan: &TransferImportPlan,
) -> Result<TransferApplyResult, TransferImportRouteError> {
    let previous_servers = if plan.mode == TransferMode::ReplaceAll {
        store.export_inputs()
    } else {
        Vec::new()
    };
    if plan.mode == TransferMode::ReplaceAll {
        let backup_path = plan
            .backup_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(TransferImportRouteError::BackupPathRequired)?;
        ports
            .backup(&previous_servers, Path::new(backup_path))
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
        let previous_server_ids = previous_servers
            .iter()
            .map(|input| input.server.id.clone())
            .collect::<Vec<_>>();
        store
            .wipe_replace_all_secrets(&previous_server_ids)
            .map_err(TransferImportRouteError::SecretWipeFailed)?;
        store
            .replace_all(result.servers.clone())
            .map_err(TransferImportRouteError::SaveFailed)?;
    } else {
        store
            .merge(result.servers.clone())
            .map_err(TransferImportRouteError::SaveFailed)?;
    }

    let _ = std::fs::remove_dir_all(staging_root);
    Ok(result)
}

fn import_transfer(
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
    if !plan.package_path.is_file() {
        return source_not_found_response(source_path);
    }
    if replace_all
        && plan
            .backup_path
            .as_deref()
            .map(str::trim)
            .is_none_or(str::is_empty)
    {
        return invalid_body(
            "backup_path_required",
            "backupPath is required for a replaceAll transfer import.",
        );
    }

    let operation_id = match state.begin_import_operation(source_path) {
        Ok(operation_id) => operation_id,
        Err(error) => return crate::routes::operations::operation_error_response(error),
    };

    let worker_state = state.clone();
    let worker_operation_id = operation_id.clone();
    tokio::spawn(async move {
        let failure_state = worker_state.clone();
        let failure_operation_id = worker_operation_id.clone();
        if let Err(error) = tokio::task::spawn_blocking(move || {
            run_transfer_import(worker_state, worker_operation_id, plan, replace_all)
        })
        .await
        {
            let _ = failure_state.finish_operation_failure(
                &failure_operation_id,
                "background_worker_failed",
                error.to_string(),
            );
        }
    });

    accepted_import_response(&operation_id, "Transfer import accepted.")
}

fn run_transfer_import(
    state: LifecycleRoutesState,
    operation_id: OperationId,
    plan: TransferImportPlan,
    replace_all: bool,
) {
    let should_cancel = state.operations().cancellation_check(&operation_id);
    if should_cancel() {
        let _ = state
            .operations()
            .cancel(&operation_id, "Transfer import cancelled before staging.");
        return;
    }
    let staging_root = transfer_staging_root();
    let result = perform_transfer_import(
        &RealTransferImportPorts,
        &state,
        &state.servers_root(),
        &staging_root,
        &plan,
    );

    match result {
        Ok(applied) => {
            let lifecycle_server_id = applied.servers.first().map(|server| server.id.clone());
            if let Err(error) = state.provision_imported_bedrock_servers(&applied.servers) {
                let _ = state.finish_operation_failure(
                    &operation_id,
                    "bedrock_provisioning_failed",
                    error.to_string(),
                );
                return;
            }
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
            if let Some(server_id) = lifecycle_server_id
                && matches!(
                    state.reconciliation_status(&server_id),
                    crate::routes::lifecycle::ReconciliationStatus::Ready
                )
            {
                let _ = state.select_active_server(server_id);
            }
            result_map.insert("replaced".to_string(), replace_all.to_string());
            if let Some(server) = applied.servers.first() {
                result_map.insert("serverId".to_string(), server.id.clone());
                result_map.insert("serverName".to_string(), server.display_name.clone());
            }
            let _ = state.finish_operation_success(&operation_id, &message, result_map);
        }
        Err(error) => {
            let _ = state.finish_operation_failure(
                &operation_id,
                transfer_error_code(&error),
                transfer_error_message(&error),
            );
        }
    }
}

fn transfer_error_code(error: &TransferImportRouteError) -> &'static str {
    match error {
        TransferImportRouteError::BackupPathRequired => "backup_path_required",
        TransferImportRouteError::BackupFailed(_) => "backup_failed",
        TransferImportRouteError::InvalidPackage(_) => "invalid_transfer_package",
        TransferImportRouteError::SecretWipeFailed(_) => "secret_wipe_failed",
        TransferImportRouteError::SaveFailed(_) => "internal_error",
    }
}

fn transfer_error_message(error: &TransferImportRouteError) -> String {
    match error {
        TransferImportRouteError::BackupPathRequired => {
            "backupPath is required for a replaceAll transfer import.".to_string()
        }
        TransferImportRouteError::BackupFailed(message) => format!("backup_failed: {message}"),
        TransferImportRouteError::InvalidPackage(message) => message.clone(),
        TransferImportRouteError::SecretWipeFailed(message) => message.clone(),
        TransferImportRouteError::SaveFailed(message) => message.clone(),
    }
}

/// `configManager.serversRootURL` has no Rust equivalent yet (no
/// `AppConfig` is loaded in `msc-agent` — see this section's header
/// comment), so this resolves the same way `auth.rs`'s
/// `default_persistent_service_store` resolves the credential registry
/// path: an env var override, falling back to the OS temp dir. Not
/// durable-by-default; flagged for Cameron alongside the registry-split
/// gap above.
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

// ---------- P7.23: POST /v1/servers/create ----------
//
// MSC 1's own `handleCreateServer` blocks the HTTP connection until
// `createServerProvider` fully returns — real minutes for a Forge/
// NeoForge install-step create (P7.3's captured installer runs).
// `openapi.json`'s own P7.9 `x-notes` on this route corrects that under
// D-006's "correction" clause: this handler validates synchronously
// (name/type/flavor, the Bedrock refusal, and a cheap pre-admission
// folder-collision check), admits a `"server-create"` operation, and
// returns 200 immediately with that `operationId` — the real jar
// download/install/world-slot work runs in the background, and only the
// operation's own terminal result carries the real `serverId`.
//
// **Correction to `openapi.json`'s own `x-notes`**, recorded rather than
// silently applied: that note describes `serverId` as "known
// synchronously... folder derivation is a pure function of the trimmed
// name." The *folder name* genuinely is (`folder_name_from_safe_name`,
// used below for the pre-admission collision check and the operation's
// own exclusivity target) — but the *id* on `ConfigServer` is a fresh
// `Uuid::new_v4()` minted deep inside `finish_server_creation`
// (`msc_domain::provisioning::new_server_config_fields`), not a
// deterministic function of anything the route has before the operation
// runs. This handler follows the same "id arrives on the terminal
// result" shape `POST /v1/servers/import` already established (P5.17)
// rather than inventing a pre-assigned id scheme no other creation path
// in this codebase uses.
const INSTALLER_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// The six create-flow flavors this route accepts — `families/
/// phase7-scope.md`'s own boundary, already enforced upstream by
/// `create_flow_choices`/`filter_to_create_flow_floor` for version
/// listing; this is the same filter applied to the flavor a create
/// *request* names, since Pufferfish/Spigot/Quilt parse fine as a
/// `JavaServerFlavor` but were never offered by MSC 1's own create flow
/// (`isAvailableInCreateFlow`).
pub(crate) fn is_create_flow_flavor(flavor: JavaServerFlavor) -> bool {
    !matches!(
        flavor,
        JavaServerFlavor::Pufferfish | JavaServerFlavor::Spigot | JavaServerFlavor::Quilt
    )
}

pub async fn create(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(staging): Extension<StagingStore>,
    body: Result<Json<serde_json::Value>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Fleet) {
        return response;
    }
    let Json(raw_body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let confirmation = raw_body
        .get("confirmation")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let body = match serde_json::from_value::<ServerCreateRequestDto>(raw_body) {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_body", "Request body must be a server object."),
    };

    let Some(safe_name) = msc_domain::provisioning::trimmed_server_name(&body.name) else {
        return invalid_body("name_required", "name is required.");
    };

    let server_type = match body.server_type.as_deref().map(str::trim) {
        None | Some("") => ServerType::Java,
        Some(raw) => match ServerType::from_raw_value(raw) {
            Some(server_type) => server_type,
            None => {
                return invalid_body("invalid_server_type", "serverType must be java or bedrock.");
            }
        },
    };
    if let Some(required) =
        world_safety::confirmation_for_server_creation(server_type, body.gamemode.as_deref())
        && !world_safety::is_confirmed(required, confirmation.as_deref())
    {
        return confirmation_required_response(required);
    }
    if server_type == ServerType::Bedrock {
        let port = match body.port.unwrap_or(19132).try_into() {
            Ok(port) if port > 0 => port,
            _ => return invalid_body("invalid_body", "port must be between 1 and 65535."),
        };
        let max_players = body.max_players.unwrap_or(10);
        if !(1..=10_000).contains(&max_players) {
            return invalid_body("invalid_body", "maxPlayers must be between 1 and 10000.");
        }
        let operation_id = match state.operations().begin_lifecycle(
            "server-create",
            Some(safe_name.clone()),
            "Creating Bedrock server.",
        ) {
            Ok(id) => id,
            Err(error) => return operation_error_response(error),
        };
        let runtime = state.bedrock_runtime_state();
        let worker_state = state.clone();
        let worker_operation_id = operation_id.clone();
        let request_name = safe_name.clone();
        let request_world_name = body.world_name.clone();
        let request_version = body.bedrock_version.clone();
        let request_enable_playit = body.enable_playit.unwrap_or(false);
        let request_enable_xbox = body.enable_xbox_broadcast.unwrap_or(false);
        let request_difficulty = body
            .difficulty
            .clone()
            .unwrap_or_else(|| "normal".to_string());
        let request_gamemode = body
            .gamemode
            .clone()
            .unwrap_or_else(|| "survival".to_string());
        let request_seed = body.world_seed.clone();
        tokio::spawn(async move {
            let failure_state = worker_state.clone();
            let failure_operation_id = worker_operation_id.clone();
            let result = tokio::task::spawn_blocking(move || {
                run_create_bedrock_server(
                    worker_state,
                    worker_operation_id,
                    request_name,
                    request_world_name,
                    request_version,
                    port,
                    max_players,
                    request_enable_playit,
                    request_enable_xbox,
                    request_difficulty,
                    request_gamemode,
                    request_seed,
                )
            })
            .await;
            if let Err(error) = result {
                let _ = failure_state.finish_operation_failure(
                    &failure_operation_id,
                    "background_worker_failed",
                    error.to_string(),
                );
            }
        });
        return Json(ServerCreateResultDto {
            success: true,
            message: "Bedrock server creation started.".to_string(),
            operation_id: Some(operation_id.as_str().to_string()),
            server_id: None,
            server_name: Some(safe_name),
            warnings: None,
            runtime: Some(runtime),
        })
        .into_response();
    }

    let flavor = match body.java_flavor.as_deref().map(str::trim) {
        None | Some("") => JavaServerFlavor::Paper,
        Some(raw) => {
            match JavaServerFlavor::from_raw_value(raw).filter(|f| is_create_flow_flavor(*f)) {
                Some(flavor) => flavor,
                None => {
                    return invalid_body(
                        "invalid_java_flavor",
                        "javaFlavor must be one of paper, purpur, vanilla, fabric, neoforge, forge.",
                    );
                }
            }
        }
    };

    let port: u16 = match body.port.unwrap_or(25565).try_into() {
        Ok(port) => port,
        Err(_) => return invalid_body("invalid_body", "port must be between 0 and 65535."),
    };
    let cross_play_bedrock_port = body
        .cross_play_bedrock_port
        .and_then(|port| u16::try_from(port).ok());

    let cfg = state.app_config_snapshot();
    let servers_root = state.servers_root();
    let folder_name = msc_domain::provisioning::folder_name_from_safe_name(&safe_name);
    let new_dir = servers_root.join("java").join(&folder_name);
    if StdFileSystem.stat(&new_dir).is_ok() {
        return error_response(
            StatusCode::CONFLICT,
            "create_failed",
            &format!("A server folder named \"{folder_name}\" already exists."),
        );
    }

    // The CLI has already uploaded the archive through the shared staging
    // route. Redeem it exactly once here, and only when it carries the
    // purpose this create path owns.
    let staged_modpack =
        match redeem_modpack_upload(&staging, body.staged_modpack_upload_id.as_deref()) {
            Ok(upload) => upload,
            Err(response) => return *response,
        };

    let operation_id = match state.operations().begin_lifecycle(
        "server-create",
        Some(folder_name),
        "Creating server.",
    ) {
        Ok(id) => id,
        Err(error) => return operation_error_response(error),
    };

    let difficulty = body.difficulty.clone().unwrap_or_default();
    let gamemode = body.gamemode.clone().unwrap_or_default();
    let world_seed = body.world_seed.clone();
    let initial_world_name = body.world_name.clone();
    let enable_cross_play = body.enable_cross_play.unwrap_or(false);
    let enable_playit = body.enable_playit.unwrap_or(false);
    let enable_xbox_broadcast = body.enable_xbox_broadcast.unwrap_or(false);
    let java_path = body
        .java_path
        .clone()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| cfg.java_path.clone());
    let accept_eula = body.accept_eula.unwrap_or(false);
    let save_downloaded_jars = cfg.save_downloaded_jars;
    let default_banner_color_hex = cfg.default_banner_color_hex.clone().unwrap_or_default();
    let home_dir = agent_home_dir();
    let paper_template_dir = PathBuf::from(&cfg.paper_template_dir);
    let plugin_template_dir = PathBuf::from(&cfg.plugin_template_dir);
    let response_server_name = safe_name.clone();

    let worker_state = state.clone();
    let worker_operation_id = operation_id.clone();
    tokio::spawn(async move {
        let failure_state = worker_state.clone();
        let failure_operation_id = worker_operation_id.clone();
        if let Err(error) = tokio::task::spawn_blocking(move || {
            run_create_server(
                worker_state,
                worker_operation_id,
                safe_name,
                flavor,
                port,
                enable_cross_play,
                cross_play_bedrock_port,
                enable_playit,
                enable_xbox_broadcast,
                difficulty,
                gamemode,
                world_seed,
                initial_world_name,
                save_downloaded_jars,
                default_banner_color_hex,
                java_path,
                accept_eula,
                home_dir,
                servers_root,
                paper_template_dir,
                plugin_template_dir,
                staged_modpack,
            )
        })
        .await
        {
            let _ = failure_state.finish_operation_failure(
                &failure_operation_id,
                "background_worker_failed",
                error.to_string(),
            );
        }
    });

    Json(ServerCreateResultDto {
        success: true,
        message: "Server creation started.".to_string(),
        operation_id: Some(operation_id.as_str().to_string()),
        server_id: None,
        server_name: Some(response_server_name),
        warnings: None,
        runtime: None,
    })
    .into_response()
}

fn confirmation_required_response(required: SafetyConfirmation) -> Response {
    (
        StatusCode::CONFLICT,
        Json(ErrorDto {
            code: "confirmation_required".to_string(),
            message: required.message().to_string(),
            help_id: None,
            details: Some(required.details()),
        }),
    )
        .into_response()
}

#[allow(clippy::too_many_arguments)]
fn run_create_bedrock_server(
    state: LifecycleRoutesState,
    operation_id: OperationId,
    name: String,
    initial_world_name: Option<String>,
    bedrock_version: Option<String>,
    port: u16,
    max_players: i64,
    enable_playit: bool,
    enable_xbox_broadcast: bool,
    difficulty: String,
    gamemode: String,
    world_seed: Option<String>,
) {
    let should_cancel = state.operations().cancellation_check(&operation_id);
    if should_cancel() {
        let _ = state.operations().cancel(
            &operation_id,
            "Bedrock server creation cancelled before it started.",
        );
        return;
    }
    let request = msc_application::provisioning::BedrockCreateRequest {
        name: &name,
        initial_world_name: initial_world_name.as_deref(),
        bedrock_version: bedrock_version.as_deref(),
        port,
        max_players,
        enable_playit,
        enable_xbox_broadcast,
        difficulty: &difficulty,
        gamemode: &gamemode,
        world_seed: world_seed.as_deref(),
        world_source: msc_application::provisioning::BedrockWorldSource::Fresh,
    };
    let created = msc_application::provisioning::create_bedrock_server(
        &StdFileSystem,
        &state.servers_root(),
        &request,
        &iso8601_now(),
    );
    match created {
        Ok(created) => {
            match state.register_imported_config_servers(vec![created.config.clone()], false) {
                Ok(statuses) => {
                    let first_start_required =
                        msc_application::provisioning::first_start_required(&created.config);
                    let reconciled = statuses.iter().any(|(id, status)| {
                        id == &created.config.id
                            && matches!(
                                status,
                                crate::routes::lifecycle::ReconciliationStatus::Ready
                            )
                    });
                    if let Err(error) = state.provision_bedrock_server(&created.config) {
                        let _ = state.finish_operation_failure(
                            &operation_id,
                            "bedrock_provisioning_failed",
                            error.to_string(),
                        );
                        return;
                    }
                    let ready = reconciled
                        && !state.bedrock_runtime_is_busy()
                        && state.bedrock_runtime_state().state == "available";
                    if ready {
                        let _ = state.select_active_server(created.config.id.clone());
                    }
                    let mut result = BTreeMap::new();
                    result.insert("serverId".to_string(), created.config.id);
                    result.insert("serverName".to_string(), created.config.display_name);
                    result.insert("ready".to_string(), ready.to_string());
                    result.insert(
                        "firstStartRequired".to_string(),
                        first_start_required.to_string(),
                    );
                    let _ = state.finish_operation_success(
                        &operation_id,
                        "Bedrock server created.",
                        result,
                    );
                }
                Err(error) => {
                    let _ = state.finish_operation_failure(
                        &operation_id,
                        "internal_error",
                        error.to_string(),
                    );
                }
            }
        }
        Err(error) => {
            let _ =
                state.finish_operation_failure(&operation_id, "create_failed", error.to_string());
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_create_server(
    state: LifecycleRoutesState,
    operation_id: OperationId,
    safe_name: String,
    flavor: JavaServerFlavor,
    port: u16,
    enable_cross_play: bool,
    cross_play_bedrock_port: Option<u16>,
    enable_playit: bool,
    enable_xbox_broadcast: bool,
    difficulty: String,
    gamemode: String,
    world_seed: Option<String>,
    initial_world_name: Option<String>,
    save_downloaded_jars: bool,
    default_banner_color_hex: String,
    java_path: String,
    accept_eula: bool,
    home_dir: PathBuf,
    servers_root: PathBuf,
    paper_template_dir: PathBuf,
    plugin_template_dir: PathBuf,
    staged_modpack: Option<StagedUpload>,
) {
    let should_cancel = state.operations().cancellation_check(&operation_id);
    if should_cancel() {
        let _ = state.operations().cancel(
            &operation_id,
            "Server creation cancelled before it started.",
        );
        return;
    }
    let now = iso8601_now();
    let request = NewServerRequest {
        name: &safe_name,
        initial_world_name: initial_world_name.as_deref(),
        flavor,
        port,
        enable_cross_play,
        cross_play_bedrock_port,
        enable_playit,
        enable_xbox_broadcast,
        difficulty: &difficulty,
        gamemode: &gamemode,
        world_seed: world_seed.as_deref(),
        world_source: WorldSource::Fresh,
        save_downloaded_jars,
        default_banner_color_hex: &default_banner_color_hex,
    };
    let transport = HttpTransport::new();

    if let Some(staged_modpack) = staged_modpack {
        let addon_transport = AddonHttpTransport::new();
        let secrets = match production_secret_store() {
            Ok(secrets) => secrets,
            Err(error) => {
                let _ = state.finish_operation_failure(
                    &operation_id,
                    "internal_error",
                    error.to_string(),
                );
                return;
            }
        };
        let inspection = match modpacks::inspect_staged_archive(
            &StdFileSystem,
            &addon_transport,
            secrets.as_ref(),
            &staged_modpack.path,
            &staging_root(&servers_root).join("modpacks"),
            operation_id.as_str(),
        ) {
            Ok(inspection) => inspection,
            Err(error) => {
                let _ = StdFileSystem.remove(&staged_modpack.path);
                let _ = state.finish_operation_failure(
                    &operation_id,
                    "invalid_body",
                    error.to_string(),
                );
                return;
            }
        };
        let pack_request = PackServerRequest {
            name: &safe_name,
            initial_world_name: initial_world_name.as_deref(),
            port,
            enable_cross_play,
            cross_play_bedrock_port,
            enable_playit,
            enable_xbox_broadcast,
            difficulty: &difficulty,
            gamemode: &gamemode,
            world_seed: world_seed.as_deref(),
            world_source: WorldSource::Fresh,
            default_banner_color_hex: &default_banner_color_hex,
        };
        let result = provisioning::create_server_from_pack(
            &StdFileSystem,
            &transport,
            &addon_transport,
            secrets.as_ref(),
            state.process_supervisor(),
            &home_dir,
            &servers_root,
            &plugin_template_dir,
            &pack_request,
            &inspection,
            &java_path,
            INSTALLER_TIMEOUT,
            &now,
            &should_cancel,
            |_stream, _bytes| {},
            provisioning::real_unzip_world_backup,
            provisioning::real_copy_existing_world_folder,
        );
        let _ = StdFileSystem.remove(&staged_modpack.path);
        match result {
            Ok(created) => {
                let created = created.created;
                let flavor = created.config.java_flavor;
                finish_created_server(&state, &operation_id, created, flavor, accept_eula, None);
            }
            Err(error) => {
                let code = if matches!(error, CreateFromPackError::Cancelled) {
                    "cancelled"
                } else {
                    "create_failed"
                };
                let _ = state.finish_operation_failure(&operation_id, code, error.to_string());
            }
        }
        return;
    }

    let result = if flavor.provisioning_kind() == ServerProvisioningKind::InstallStep {
        provisioning::create_install_step_server(
            &StdFileSystem,
            &transport,
            state.process_supervisor(),
            &home_dir,
            &servers_root,
            &plugin_template_dir,
            &request,
            &java_path,
            INSTALLER_TIMEOUT,
            &now,
            &should_cancel,
            |_stream, _bytes| {},
            provisioning::real_unzip_world_backup,
            provisioning::real_copy_existing_world_folder,
        )
    } else {
        provisioning::create_download_and_go_server(
            &StdFileSystem,
            &transport,
            &home_dir,
            &servers_root,
            &paper_template_dir,
            &plugin_template_dir,
            &request,
            &now,
            provisioning::real_unzip_world_backup,
            provisioning::real_copy_existing_world_folder,
        )
    };

    match result {
        Ok(mut created) => {
            // P7.31: `create_install_step_server` already ran this guard
            // itself (before its own installer subprocess), so this only
            // gates the four download-and-go families, which never spawn
            // Java at create time at all -- this is their one chance to
            // refuse before the server is registered into the fleet and
            // becomes something a caller could try to start.
            if flavor.provisioning_kind() != ServerProvisioningKind::InstallStep {
                match evaluate_download_and_go_java_guard(
                    state.process_supervisor(),
                    &java_path,
                    created.config.minecraft_version.as_deref(),
                ) {
                    Ok(warning) => created.java_compatibility_warning = warning,
                    Err(unusable) => {
                        let _ = StdFileSystem.remove(Path::new(&created.config.server_dir));
                        let _ = state.finish_operation_failure(
                            &operation_id,
                            "unusable_java_runtime",
                            unusable.to_string(),
                        );
                        return;
                    }
                }
            }

            let java_compatibility_warning = created.java_compatibility_warning.clone();
            finish_created_server(
                &state,
                &operation_id,
                created,
                flavor,
                accept_eula,
                java_compatibility_warning,
            );
        }
        Err(error) => {
            let _ = state.finish_operation_failure(
                &operation_id,
                create_server_error_code(&error),
                error.to_string(),
            );
        }
    }
}

fn redeem_modpack_upload(
    staging: &StagingStore,
    staged_upload_id: Option<&str>,
) -> Result<Option<StagedUpload>, Box<Response>> {
    let Some(staged_upload_id) = staged_upload_id.filter(|id| !id.trim().is_empty()) else {
        return Ok(None);
    };
    let entry = staging.uploads.lock().unwrap().remove(staged_upload_id);
    let Some(entry) = entry else {
        return Err(Box::new(error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged upload.",
        )));
    };
    if now_unix() > entry.expires_at_unix
        || !matches!(
            entry.purpose,
            msc_api::dto::StagedUploadPurposeDto::ModpackArchive
        )
    {
        return Err(Box::new(error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Unknown or already-redeemed staged upload.",
        )));
    }
    Ok(Some(entry))
}

fn finish_created_server(
    state: &LifecycleRoutesState,
    operation_id: &OperationId,
    created: provisioning::CreatedServer,
    flavor: JavaServerFlavor,
    accept_eula: bool,
    java_compatibility_warning: Option<String>,
) {
    let server_id = created.config.id.clone();
    let server_name = created.config.display_name.clone();
    let first_start_required = provisioning::first_start_required(&created.config);
    match state.register_imported_config_servers(vec![created.config], false) {
        Ok(statuses) => {
            let ready = statuses.iter().any(|(id, status)| {
                id == &server_id
                    && matches!(
                        status,
                        crate::routes::lifecycle::ReconciliationStatus::Ready
                    )
            });
            if accept_eula {
                let _ =
                    fleet::accept_eula(&StdFileSystem, &state.app_config_snapshot(), &server_id);
            }
            if ready {
                let _ = state.select_active_server(server_id.clone());
            }
            let mut result_map = BTreeMap::new();
            result_map.insert("serverId".to_string(), server_id);
            result_map.insert("serverName".to_string(), server_name.clone());
            result_map.insert("ready".to_string(), ready.to_string());
            result_map.insert(
                "firstStartRequired".to_string(),
                first_start_required.to_string(),
            );
            if let Some(warning) = java_compatibility_warning {
                result_map.insert("javaCompatibilityWarning".to_string(), warning);
            }
            let _ = state.finish_operation_success(
                operation_id,
                &format!("Created {} server \"{server_name}\".", flavor.raw_value()),
                result_map,
            );
        }
        Err(error) => {
            let _ =
                state.finish_operation_failure(operation_id, "internal_error", error.to_string());
        }
    }
}

/// P7.31's download-and-go half of the required-major guard.
/// `create_install_step_server` already runs its own copy of this same
/// composition (`msc_infrastructure::java_runtime_detection::
/// run_java_version_probe` + `msc_domain::java_runtime::
/// evaluate_java_runtime_guard`) against Forge/NeoForge's own installer-
/// spawning Java before this route ever sees a result
/// (`provisioning::check_java_runtime_guard`, `pub(crate)` to that
/// crate); this is the four download-and-go families' own gate, since
/// they never spawn Java at create time at all. Split out from
/// `run_create_server`'s own match arm so it's directly testable against
/// a `FakeProcessSupervisor` — the guard itself never touches the
/// network; only *reaching* a `Created` server via this route's own
/// hardcoded `HttpTransport` does, which is why P7.31's own report
/// flagged this as untested and this follow-up closes that.
fn evaluate_download_and_go_java_guard(
    supervisor: &dyn ProcessSupervisor,
    java_path: &str,
    minecraft_version: Option<&str>,
) -> Result<Option<String>, msc_domain::java_runtime::UnusableJavaRuntime> {
    let probe = java_runtime_detection::run_java_version_probe(supervisor, java_path);
    msc_domain::java_runtime::evaluate_java_runtime_guard(java_path, minecraft_version, &probe)
}

fn create_server_error_code(error: &CreateServerError) -> &'static str {
    match error {
        CreateServerError::EmptyName => "name_required",
        CreateServerError::Cancelled => "cancelled",
        CreateServerError::UnusableJavaRuntime(_) => "unusable_java_runtime",
        _ => "create_failed",
    }
}

// ---------- P7.23: POST /v1/servers/delete ----------

pub async fn delete(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<ServerDeleteRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Fleet) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let server_id = body.server_id.trim().to_string();
    if server_id.is_empty() {
        return invalid_body("missing_server_id", "serverId is required.");
    }

    match state.delete_fleet_server(&server_id) {
        Ok(deleted) => Json(ServerDeleteResultDto {
            success: true,
            message: format!("Deleted server \"{}\".", deleted.removed_display_name),
            server_id: Some(server_id),
        })
        .into_response(),
        Err(TryMutateError::Domain(error)) => delete_server_error_response(error),
        Err(TryMutateError::Save(error)) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "delete_failed",
            &error.to_string(),
        ),
    }
}

fn delete_server_error_response(error: DeleteServerError) -> Response {
    match error {
        DeleteServerError::EmptyServerId => {
            invalid_body("missing_server_id", "serverId is required.")
        }
        DeleteServerError::ServerNotFound => error_response(
            StatusCode::NOT_FOUND,
            "server_not_found",
            "Server not found.",
        ),
        DeleteServerError::ServerRunning => {
            error_response(StatusCode::CONFLICT, "server_running", "Server is running.")
        }
        DeleteServerError::DeleteFailed(io_error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "delete_failed",
            &io_error.to_string(),
        ),
    }
}

// ---------- P7.23: POST /v1/servers/rename ----------

pub async fn rename(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<ServerRenameRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Fleet) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let server_id = body.server_id.trim().to_string();
    if server_id.is_empty() {
        return invalid_body("missing_server_id", "serverId is required.");
    }
    if body.name.trim().is_empty() {
        return invalid_body("name_required", "name is required.");
    }

    match state.rename_fleet_server(&server_id, &body.name) {
        Ok(()) => Json(ServerRenameResultDto {
            success: true,
            message: "Server renamed.".to_string(),
            server_id: Some(server_id),
            name: Some(body.name.trim().to_string()),
        })
        .into_response(),
        Err(TryMutateError::Domain(error)) => rename_server_error_response(error),
        Err(TryMutateError::Save(error)) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &error.to_string(),
        ),
    }
}

fn rename_server_error_response(error: RenameServerError) -> Response {
    match error {
        RenameServerError::EmptyServerId => {
            invalid_body("missing_server_id", "serverId is required.")
        }
        RenameServerError::EmptyName => invalid_body("name_required", "name is required."),
        RenameServerError::ServerNotFound => error_response(
            StatusCode::NOT_FOUND,
            "server_not_found",
            "Server not found.",
        ),
    }
}

// ---------- P7.23: POST /v1/servers/eula ----------

pub async fn eula(
    State(state): State<LifecycleRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<ServerEulaRequestDto>, JsonRejection>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Fleet) {
        return response;
    }
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => return invalid_body("invalid_json", "Request body must be valid JSON."),
    };
    let server_id = body
        .server_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty());
    let Some(server_id) = server_id else {
        return invalid_body("missing_server_id", "serverId is required.");
    };

    let cfg = state.app_config_snapshot();
    match fleet::accept_eula(&StdFileSystem, &cfg, server_id) {
        Ok(()) => Json(ServerEulaResultDto {
            success: true,
            message: "EULA accepted.".to_string(),
            server_id: Some(server_id.to_string()),
            accepted: Some(true),
        })
        .into_response(),
        Err(AcceptEulaError::ServerNotFound) => error_response(
            StatusCode::NOT_FOUND,
            "server_not_found",
            "Server not found.",
        ),
        Err(AcceptEulaError::UnsupportedServerType) => invalid_body(
            "missing_server_id",
            "EULA acceptance only applies to Java servers.",
        ),
        Err(AcceptEulaError::WriteFailed(io_error)) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "eula_write_failed",
            &io_error.to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthState, CredentialRole};
    use crate::routes::lifecycle::AgentAppConfigStore;
    use crate::routes::operations::OperationsState;
    use crate::ws::console::ConsoleState;
    use msc_api::dto::{OperationDto, OperationStateDto};
    use msc_application::transfer::{TransferManifest, TransferServerConflict};
    use msc_infrastructure::fs::{FileSystem, StdFileSystem};
    use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};
    use std::sync::Arc;

    /// P7.31's own required-major guard now spawns `<java> -version`
    /// before any real server process (create's own download-and-go
    /// guard, and `start_active_server`'s own start-time guard) --
    /// `FakeProcessSupervisor` has no automatic responder, so a test that
    /// exercises either path has to drive it from a background thread the
    /// same way `routes::lifecycle`'s own test module's identically-named
    /// helper does. Answers with a Java 25 banner, comfortably above
    /// every possible `required_java_major` result.
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

    // ---------------------------------------------------------------------
    // P7.31 follow-up: `evaluate_download_and_go_java_guard`, tested
    // directly against a `FakeProcessSupervisor` rather than through a
    // real, network-backed `create_download_and_go_server` call — the
    // guard itself never touches the network, only *reaching* a
    // `Created` server via this route's own hardcoded `HttpTransport`
    // does. Closes the gap P7.31's own "Actual result" flagged.
    // ---------------------------------------------------------------------

    #[test]
    fn java_runtime_guard_download_and_go_refuses_below_required_major() {
        let supervisor: &'static msc_infrastructure::process::FakeProcessSupervisor = Box::leak(
            Box::new(msc_infrastructure::process::FakeProcessSupervisor::new()),
        );
        let driver = drive_java_version_probe_with_banner(
            supervisor,
            "openjdk version \"17.0.9\" 2023-10-17\n",
        );

        // 1.21.4 needs Java 21; the probe above answers with Java 17.
        let err = evaluate_download_and_go_java_guard(supervisor, "/usr/bin/java", Some("1.21.4"))
            .unwrap_err();
        driver.join().unwrap();

        assert_eq!(
            err.reason,
            msc_domain::java_runtime::UnusableJavaRuntimeReason::BelowRequiredMajor
        );
        assert_eq!(err.required_major, 21);
        assert_eq!(err.detected_major, Some(17));
        // Only the probe was ever spawned.
        assert_eq!(supervisor.spawned_requests().len(), 1);
    }

    #[test]
    fn java_runtime_guard_download_and_go_proceeds_when_java_is_sufficient() {
        let supervisor: &'static msc_infrastructure::process::FakeProcessSupervisor = Box::leak(
            Box::new(msc_infrastructure::process::FakeProcessSupervisor::new()),
        );
        let driver = drive_java_version_probe_once(supervisor);

        let warning =
            evaluate_download_and_go_java_guard(supervisor, "/usr/bin/java", Some("1.21.4"))
                .unwrap();
        driver.join().unwrap();

        assert_eq!(warning, None);
    }

    #[test]
    fn java_runtime_guard_download_and_go_refuses_when_java_not_found() {
        let supervisor: &'static msc_infrastructure::process::FakeProcessSupervisor = Box::leak(
            Box::new(msc_infrastructure::process::FakeProcessSupervisor::new()),
        );
        supervisor.fail_next_spawn("no such executable");

        let err =
            evaluate_download_and_go_java_guard(supervisor, "/nonexistent/java", Some("1.21.4"))
                .unwrap_err();

        assert_eq!(
            err.reason,
            msc_domain::java_runtime::UnusableJavaRuntimeReason::NotFound
        );
    }

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

    fn sample_config_server_with_dir(id: &str, dir: &Path) -> ConfigServer {
        let mut server = sample_config_server(id);
        server.server_dir = dir.to_string_lossy().into_owned();
        server
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
        let store = ConfigServerStore::new();
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
        let store = ConfigServerStore::new();
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
        let store = ConfigServerStore::new();
        store
            .replace_all(vec![sample_config_server("OLD-1")])
            .unwrap();
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
        let store = ConfigServerStore::new();
        store.merge(vec![sample_config_server("OLD-1")]).unwrap();
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

    fn persistent_route_state(config_path: &Path, servers_root: &Path) -> LifecycleRoutesState {
        let fs: &'static dyn FileSystem = Box::leak(Box::new(StdFileSystem));
        let app_config = Box::leak(Box::new(
            AgentAppConfigStore::load(fs, config_path.to_path_buf(), servers_root.to_path_buf())
                .unwrap(),
        ));
        LifecycleRoutesState::with_fake_process_and_app_config(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
            app_config,
        )
    }

    fn persistent_route_state_with_auth(
        config_path: &Path,
        servers_root: &Path,
        auth_state: AuthState,
    ) -> LifecycleRoutesState {
        let fs: &'static dyn FileSystem = Box::leak(Box::new(StdFileSystem));
        let app_config = Box::leak(Box::new(
            AgentAppConfigStore::load(fs, config_path.to_path_buf(), servers_root.to_path_buf())
                .unwrap(),
        ));
        LifecycleRoutesState::with_app_config_and_auth(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
            app_config,
            auth_state,
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

    async fn await_import(
        state: &LifecycleRoutesState,
        response: Response,
    ) -> (StatusCode, ServerImportResultDto, OperationDto) {
        let (status, accepted): (StatusCode, ServerImportResultDto) = response_json(response).await;
        let operation_id = accepted
            .operation_id
            .as_deref()
            .expect("accepted import carries operationId");
        for _ in 0..200 {
            let operation = state
                .operations()
                .snapshot(operation_id)
                .expect("accepted operation remains readable");
            if matches!(
                operation.state,
                OperationStateDto::Succeeded
                    | OperationStateDto::Failed
                    | OperationStateDto::Cancelled
            ) {
                return (status, accepted, operation);
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("import operation {operation_id} did not become terminal");
    }

    fn operation_result_string<'a>(operation: &'a OperationDto, key: &str) -> &'a str {
        operation.result.as_ref().unwrap()[key]
            .as_str()
            .unwrap_or_else(|| panic!("operation result has no string field {key}"))
    }

    #[tokio::test]
    async fn transfer_import_route_merge_appends_across_two_imports() {
        let state = route_state();
        let package_one = build_transfer_package("ROUTE-A", 25601);
        let package_two = build_transfer_package("ROUTE-B", 25602);

        let (status_one, _, operation_one) = await_import(
            &state,
            call_import(&state, import_request(&package_one, None, None)).await,
        )
        .await;
        assert_eq!(status_one, StatusCode::ACCEPTED);
        assert_eq!(operation_one.state, OperationStateDto::Succeeded);
        assert_eq!(operation_result_string(&operation_one, "imported"), "1");
        assert_eq!(operation_result_string(&operation_one, "replaced"), "false");

        let (status_two, _, operation_two) = await_import(
            &state,
            call_import(&state, import_request(&package_two, None, None)).await,
        )
        .await;
        assert_eq!(status_two, StatusCode::ACCEPTED);
        assert_eq!(operation_result_string(&operation_two, "imported"), "1");

        let ids: Vec<String> = state
            .config_servers()
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
        let (status, _, operation) = await_import(
            &state,
            call_import(&state, import_request(&first_package, None, None)).await,
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(operation.state, OperationStateDto::Succeeded);

        let second_package = build_transfer_package("ROUTE-E", 25605);
        let backup_path = temp_dir("backups").join("before-replace-all.msctransfer");

        let (status, _, operation) = await_import(
            &state,
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

        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(operation_result_string(&operation, "replaced"), "true");
        assert!(
            backup_path.is_file(),
            "backup file was not written before replaceAll"
        );

        let names: Vec<String> = state
            .config_servers()
            .iter()
            .map(|s| s.display_name.clone())
            .collect();
        assert_eq!(names, vec!["Route Test ROUTE-E".to_string()]);
    }

    #[tokio::test]
    async fn replace_all_deletes_real_remote_token_and_previous_server_secrets() {
        let root = temp_dir("replace-all-root");
        let config_dir = temp_dir("replace-all-config");
        let config_path = config_dir.join("server_config_swift.json");
        let old_dir = temp_dir("replace-all-old-server");
        std::fs::write(old_dir.join("server.properties"), "server-port=25631\n").unwrap();
        let secret_store: Arc<dyn SecretStore + Send + Sync> = Arc::new(FakeSecretStore::new());
        let auth_state = AuthState::new(secret_store.clone());
        let issued = auth_state
            .issue_credential(
                "owner-admin",
                CredentialRole::Admin,
                vec![PermissionCategoryDto::Fleet],
                None,
            )
            .unwrap();
        secret_store
            .set(
                "xbox-broadcast.alt-password.OLD-REPLACE",
                "old-xbox-password",
            )
            .unwrap();
        let state = persistent_route_state_with_auth(&config_path, &root, auth_state);
        state
            .merge_config_servers(vec![sample_config_server_with_dir("OLD-REPLACE", &old_dir)])
            .unwrap();

        let package = build_transfer_package("ROUTE-F", 25606);
        let backup_path = temp_dir("replace-all-real-secret-backup").join("backup.msctransfer");
        let (status, _, operation) = await_import(
            &state,
            call_import(
                &state,
                import_request(
                    &package,
                    Some("replaceAll"),
                    Some(&backup_path.to_string_lossy()),
                ),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(operation_result_string(&operation, "replaced"), "true");
        assert!(
            secret_store
                .get(&format!("remote-api.token.{}", issued.credential_id))
                .unwrap()
                .is_none(),
            "replaceAll should invalidate existing bearer token verifiers"
        );
        assert_eq!(
            secret_store
                .get("xbox-broadcast.alt-password.OLD-REPLACE")
                .unwrap(),
            None,
            "replaceAll should delete Xbox secrets for the pre-replace server set"
        );
        let names = state
            .config_servers()
            .into_iter()
            .map(|server| server.display_name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["Route Test ROUTE-F".to_string()]);
    }

    // ---------- P5.21/P5.28: raw folder/ZIP scan and import route wiring ----------
    //
    // ZIP-source coverage (extraction, single-root unwrap, traversal-entry
    // rejection) lives in `msc-application`'s own `raw_server_import.rs`/
    // `raw_server_scan.rs` (P5.19/P5.20) and in `tools/phase5/cli-smoke.sh
    // --raw` end to end; `msc-agent` carries no `zip`-writing dependency of
    // its own, so these route tests cover folder sources only — proving
    // request/response wiring, override persistence, error-code mapping,
    // and P5.28's no-`serverType` source inference.

    use msc_domain::identity::JavaServerFlavor;

    fn raw_import_request(
        action: &str,
        source_path: &Path,
        import_kind: Option<&str>,
        server_type: Option<&str>,
    ) -> ServerImportRequestDto {
        ServerImportRequestDto {
            action: Some(action.to_string()),
            source_path: Some(source_path.to_string_lossy().into_owned()),
            import_kind: import_kind.map(str::to_string),
            display_name: None,
            server_type: server_type.map(str::to_string),
            active_world_name: None,
            port: None,
            max_players: None,
            accept_eula: None,
            enable_playit: None,
            transfer_mode: None,
            backup_path: None,
            java_port_overrides: HashMap::new(),
            bedrock_port_overrides: HashMap::new(),
        }
    }

    fn rescan_request() -> ServerImportRequestDto {
        ServerImportRequestDto {
            action: Some("rescan".to_string()),
            source_path: None,
            import_kind: None,
            display_name: None,
            server_type: None,
            active_world_name: None,
            port: None,
            max_players: None,
            accept_eula: None,
            enable_playit: None,
            transfer_mode: None,
            backup_path: None,
            java_port_overrides: HashMap::new(),
            bedrock_port_overrides: HashMap::new(),
        }
    }

    fn write_paper_source(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("paper-1.21.1-131.jar"), "").unwrap();
        std::fs::write(
            dir.join("server.properties"),
            "server-port=25565\nmax-players=20\nlevel-name=world\n",
        )
        .unwrap();
        std::fs::write(dir.join("eula.txt"), "eula=true\n").unwrap();
    }

    fn write_bedrock_source(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("bedrock_server"), "").unwrap();
        std::fs::write(
            dir.join("server.properties"),
            "server-port=19132\nmax-players=10\n",
        )
        .unwrap();
    }

    #[tokio::test]
    async fn raw_import_route_scans_java_folder() {
        let state = route_state();
        let source = temp_dir("scan-java-folder");
        write_paper_source(&source);

        let (status, result): (StatusCode, ServerImportScanResponseDto) = response_json(
            call_import(
                &state,
                raw_import_request("scan", &source, Some("folder"), None),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(result.success);
        assert_eq!(result.is_zip, Some(false));
        assert_eq!(result.server_type.as_deref(), Some("java"));
        assert_eq!(result.java_flavor.as_deref(), Some("paper"));
        assert_eq!(result.port, Some(25565));
        assert_eq!(result.max_players, Some(20));
        assert_eq!(result.eula_accepted, Some(true));
    }

    #[tokio::test]
    async fn raw_import_route_scans_bedrock_folder() {
        let state = route_state();
        let source = temp_dir("scan-bedrock-folder");
        write_bedrock_source(&source);

        let (status, result): (StatusCode, ServerImportScanResponseDto) = response_json(
            call_import(
                &state,
                raw_import_request("scan", &source, Some("folder"), None),
            )
            .await,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(result.server_type.as_deref(), Some("bedrock"));
        assert_eq!(result.java_flavor, None);
        assert_eq!(result.port, Some(19132));
        assert_eq!(result.max_players, Some(10));
    }

    #[tokio::test]
    async fn raw_import_route_scan_of_missing_source_is_not_found() {
        let state = route_state();
        let missing = temp_dir("scan-missing-parent").join("does-not-exist");

        let response = call_import(
            &state,
            raw_import_request("scan", &missing, Some("folder"), None),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn raw_import_route_imports_java_folder_with_overrides_and_registers_server() {
        let state = route_state();
        let source = temp_dir("import-java-folder-source");
        write_paper_source(&source);

        let mut request =
            raw_import_request("importExisting", &source, Some("folder"), Some("java"));
        request.display_name = Some("Raw Route Java".to_string());
        request.port = Some(25599);
        request.max_players = Some(7);
        request.active_world_name = Some("survival".to_string());
        request.accept_eula = Some(true);
        request.enable_playit = Some(true);

        let (status, _, operation) = await_import(&state, call_import(&state, request).await).await;

        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(operation.state, OperationStateDto::Succeeded);
        assert_eq!(
            operation_result_string(&operation, "serverName"),
            "Raw Route Java"
        );
        let server_id = operation_result_string(&operation, "serverId").to_string();

        let snapshot = state.config_servers();
        let registered = snapshot
            .iter()
            .find(|s| s.id == server_id)
            .expect("imported server should be registered");
        assert_eq!(registered.server_type, ServerType::Java);
        assert_eq!(registered.java_flavor, JavaServerFlavor::Paper);
        assert!(registered.playit_enabled);

        let properties =
            std::fs::read_to_string(Path::new(&registered.server_dir).join("server.properties"))
                .unwrap();
        assert!(properties.contains("server-port=25599"));
        assert!(properties.contains("max-players=7"));
        assert!(properties.contains("level-name=survival"));

        // The imported server is copied, not moved: the original source is
        // untouched.
        assert!(source.join("paper-1.21.1-131.jar").is_file());
    }

    #[tokio::test]
    async fn raw_import_route_imports_bedrock_folder() {
        let state = route_state();
        let source = temp_dir("import-bedrock-folder-source");
        write_bedrock_source(&source);

        let mut request =
            raw_import_request("importExisting", &source, Some("folder"), Some("bedrock"));
        request.display_name = Some("Raw Route Bedrock".to_string());
        request.port = Some(19199);

        let (status, _, operation) = await_import(&state, call_import(&state, request).await).await;

        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(operation.state, OperationStateDto::Succeeded);
        let server_id = operation_result_string(&operation, "serverId").to_string();

        let snapshot = state.config_servers();
        let registered = snapshot
            .iter()
            .find(|s| s.id == server_id)
            .expect("imported server should be registered");
        assert_eq!(registered.server_type, ServerType::Bedrock);
    }

    #[test]
    fn raw_import_worker_cancels_before_copying_at_its_first_safe_boundary() {
        let state = route_state();
        let source = temp_dir("cancelled-raw-import-source");
        write_paper_source(&source);
        let operation_id = state.begin_import_operation("cancelled-source").unwrap();
        state
            .operations()
            .request_cancel(&operation_id, "Cancelling…")
            .unwrap();

        run_raw_import(
            state.clone(),
            operation_id.clone(),
            "Cancelled Import".to_string(),
            Some(ServerType::Java),
            RawImportSource::Folder(source),
            state.servers_root(),
            RawImportOverrides::default(),
        );

        let snapshot = state
            .operations()
            .snapshot(operation_id.as_str())
            .expect("cancelled operation remains readable");
        assert_eq!(snapshot.state, OperationStateDto::Cancelled);
        assert!(state.config_servers().is_empty());
    }

    #[tokio::test]
    async fn raw_import_route_refuses_existing_destination_as_conflict() {
        let state = route_state();
        let source = temp_dir("import-conflict-source");
        write_paper_source(&source);

        let mut request =
            raw_import_request("importExisting", &source, Some("folder"), Some("java"));
        request.display_name = Some("Conflict Server".to_string());

        let first = call_import(&state, request.clone()).await;
        let (first_status, _, first_operation) = await_import(&state, first).await;
        assert_eq!(first_status, StatusCode::ACCEPTED);
        assert_eq!(first_operation.state, OperationStateDto::Succeeded);

        let second = call_import(&state, request).await;
        let (second_status, _, second_operation) = await_import(&state, second).await;
        assert_eq!(second_status, StatusCode::ACCEPTED);
        assert_eq!(second_operation.state, OperationStateDto::Failed);
        assert_eq!(
            second_operation.error.unwrap().code,
            "conflict",
            "filesystem conflicts after admission are recorded durably"
        );
    }

    #[tokio::test]
    async fn import_lifecycle_without_server_type_infers_java_and_selects_it() {
        let state = route_state();
        let source = temp_dir("inferred-paper-source");
        write_paper_source(&source);

        let request = raw_import_request("importExisting", &source, Some("folder"), None);
        let (status, _, operation) = await_import(&state, call_import(&state, request).await).await;

        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(operation.state, OperationStateDto::Succeeded);
        let server_id = operation_result_string(&operation, "serverId").to_string();

        assert!(
            state.servers().iter().any(|s| s.id == server_id),
            "expected inferred import to register through the lifecycle registry"
        );
        assert!(
            state.config_servers().iter().any(|s| s.id == server_id),
            "expected inferred import to persist into AppConfig"
        );
        assert_eq!(
            state.active_server_id().as_deref(),
            Some(server_id.as_str())
        );

        let registered = state
            .config_servers()
            .into_iter()
            .find(|s| s.id == server_id)
            .expect("expected saved server");
        assert_ne!(
            registered.server_dir,
            source.to_string_lossy(),
            "importExisting should now copy into the configured servers root"
        );
    }

    #[tokio::test]
    async fn rescan_route_requires_fleet_permission() {
        let state = route_state();
        let credential = AuthenticatedCredential {
            credential_id: "guest".to_string(),
            label: "guest".to_string(),
            role: CredentialRole::Guest,
            permissions: vec![],
        };

        let response = import(
            State(state),
            Extension(credential),
            Ok(Json(rescan_request())),
        )
        .await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn rescan_route_registers_untracked_servers_and_survives_state_rebuild() {
        let root = temp_dir("rescan-root");
        let config_dir = temp_dir("rescan-config");
        let config_path = config_dir.join("server_config_swift.json");
        let source = root.join("java").join("rescan_smoke_java");
        write_paper_source(&source);

        let state = persistent_route_state(&config_path, &root);
        let (status, _, operation) =
            await_import(&state, call_import(&state, rescan_request()).await).await;

        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(operation.state, OperationStateDto::Succeeded);
        assert_eq!(operation_result_string(&operation, "imported"), "1");
        let server_id = operation_result_string(&operation, "serverId").to_string();
        assert_eq!(
            operation_result_string(&operation, "serverName"),
            "rescan smoke java"
        );
        assert_eq!(
            state.active_server_id().as_deref(),
            Some(server_id.as_str())
        );

        let saved = state.config_servers();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].id, server_id);
        assert_eq!(saved[0].server_dir, source.to_string_lossy());
        assert_eq!(saved[0].server_type, ServerType::Java);
        assert_eq!(saved[0].java_flavor, JavaServerFlavor::Paper);
        assert!(saved[0].has_ever_started);

        let restarted = persistent_route_state(&config_path, &root);
        assert_eq!(
            restarted.active_server_id().as_deref(),
            Some(server_id.as_str())
        );
        assert!(
            restarted
                .servers()
                .iter()
                .any(|server| server.id == server_id),
            "restarted route state should reconstruct lifecycle registry from saved config"
        );

        let (status, _, second) =
            await_import(&restarted, call_import(&restarted, rescan_request()).await).await;
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(operation_result_string(&second, "imported"), "0");
    }

    // ---------- P7.23: POST /v1/servers/create (synchronous validation) ----------
    //
    // The real jar-download/install path needs a fake `Transport` this
    // route doesn't accept as an injectable parameter (it always builds a
    // real `HttpTransport`, matching this phase's own "provisioning tests
    // never touch the network... served by a fake provider" design that
    // lives at the `msc-application`/`msc-infrastructure` layer instead —
    // `crates/msc-application/tests/provisioning.rs` already covers that
    // ground). These tests cover only the route's own synchronous
    // validation, which runs and returns before any network call.

    fn create_request(name: &str) -> ServerCreateRequestDto {
        ServerCreateRequestDto {
            name: name.to_string(),
            ..Default::default()
        }
    }

    async fn call_create(state: &LifecycleRoutesState, body: ServerCreateRequestDto) -> Response {
        call_create_with_staging(state, StagingStore::default(), body).await
    }

    async fn call_create_with_staging(
        state: &LifecycleRoutesState,
        staging: StagingStore,
        body: ServerCreateRequestDto,
    ) -> Response {
        create(
            State(state.clone()),
            Extension(transfer_credential()),
            Extension(staging),
            Ok(Json(serde_json::to_value(body).unwrap())),
        )
        .await
    }

    #[tokio::test]
    async fn create_route_rejects_a_blank_name() {
        let state = route_state();
        let response = call_create(&state, create_request("   ")).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(
            String::from_utf8(bytes.to_vec())
                .unwrap()
                .contains("name_required")
        );
    }

    #[tokio::test]
    async fn create_route_accepts_bedrock_creation_operation() {
        let state = route_state();
        let mut body = create_request("Bedrock Realm");
        body.server_type = Some("bedrock".to_string());
        let response = call_create(&state, body).await;
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: ServerCreateResultDto = serde_json::from_slice(&bytes).unwrap();
        assert!(body.success);
        assert_eq!(body.message, "Bedrock server creation started.");
        assert!(body.operation_id.is_some());
    }

    #[tokio::test]
    async fn create_route_rejects_a_flavor_outside_the_create_flow() {
        let state = route_state();
        let mut body = create_request("Spigot Realm");
        body.java_flavor = Some("spigot".to_string());
        let response = call_create(&state, body).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(
            String::from_utf8(bytes.to_vec())
                .unwrap()
                .contains("invalid_java_flavor")
        );
    }

    #[tokio::test]
    async fn create_route_rejects_an_unrecognized_server_type() {
        let state = route_state();
        let mut body = create_request("Odd Realm");
        body.server_type = Some("atari".to_string());
        let response = call_create(&state, body).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(
            String::from_utf8(bytes.to_vec())
                .unwrap()
                .contains("invalid_server_type")
        );
    }

    #[test]
    fn create_route_redeems_a_modpack_upload_instead_of_falling_back_to_paper() {
        let staging = StagingStore::default();
        let staged_path = temp_dir("create-modpack-upload").join("pack.bin");
        std::fs::write(&staged_path, b"not a zip archive").unwrap();
        staging.uploads.lock().unwrap().insert(
            "pack-upload".to_string(),
            StagedUpload {
                purpose: msc_api::dto::StagedUploadPurposeDto::ModpackArchive,
                expires_at_unix: now_unix() + 60,
                max_bytes: 1024,
                path: staged_path.clone(),
            },
        );
        let redeemed = redeem_modpack_upload(&staging, Some("pack-upload"))
            .expect("a modpack-purpose upload is redeemable by server creation")
            .expect("a supplied upload produces a staged file");
        assert_eq!(redeemed.path, staged_path);
        assert!(staging.uploads.lock().unwrap().get("pack-upload").is_none());
    }

    // ---------- P7.23: POST /v1/servers/delete ----------

    fn seeded_server(id: &str, dir: &Path) -> ConfigServer {
        ConfigServer::new(
            id,
            format!("Server {id}"),
            dir.to_string_lossy().into_owned(),
            "",
            2.0,
            4.0,
        )
    }

    async fn call_delete(state: &LifecycleRoutesState, server_id: &str) -> Response {
        delete(
            State(state.clone()),
            Extension(transfer_credential()),
            Ok(Json(ServerDeleteRequestDto {
                server_id: server_id.to_string(),
            })),
        )
        .await
    }

    #[tokio::test]
    async fn delete_route_removes_a_registered_server() {
        let state = route_state();
        let dir = temp_dir("delete-route");
        state
            .register_imported_config_servers(vec![seeded_server("DEL-1", &dir)], false)
            .unwrap();

        let response = call_delete(&state, "DEL-1").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            !state.servers().iter().any(|s| s.id == "DEL-1"),
            "deleted server should no longer be registered"
        );
    }

    #[tokio::test]
    async fn delete_route_refuses_the_running_active_server() {
        let (state, supervisor) = LifecycleRoutesState::with_fake_process_capturing_supervisor(
            ConsoleState::default(),
            OperationsState::fake_journaled(),
        );
        let dir = temp_dir("delete-route-running");
        std::fs::write(dir.join("paper.jar"), b"fake jar").unwrap();
        let mut server = seeded_server("DEL-2", &dir);
        server.paper_jar_path = dir.join("paper.jar").to_string_lossy().into_owned();
        state
            .register_imported_config_servers(vec![server], false)
            .unwrap();
        state.select_active_server("DEL-2".to_string()).unwrap();
        let probe_driver = drive_java_version_probe_once(supervisor);
        state.start_active_server().unwrap();
        probe_driver.join().unwrap();

        let response = call_delete(&state, "DEL-2").await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(
            state.servers().iter().any(|s| s.id == "DEL-2"),
            "a running server must not be deleted"
        );
    }

    #[tokio::test]
    async fn delete_route_reports_missing_server_id() {
        let state = route_state();
        let response = call_delete(&state, "").await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn delete_route_reports_server_not_found() {
        let state = route_state();
        let response = call_delete(&state, "does-not-exist").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // ---------- P7.23: POST /v1/servers/rename ----------

    async fn call_rename(state: &LifecycleRoutesState, server_id: &str, name: &str) -> Response {
        rename(
            State(state.clone()),
            Extension(transfer_credential()),
            Ok(Json(ServerRenameRequestDto {
                server_id: server_id.to_string(),
                name: name.to_string(),
            })),
        )
        .await
    }

    #[tokio::test]
    async fn rename_route_updates_the_display_name() {
        let state = route_state();
        let dir = temp_dir("rename-route");
        state
            .register_imported_config_servers(vec![seeded_server("REN-1", &dir)], false)
            .unwrap();

        let response = call_rename(&state, "REN-1", "New Display Name").await;
        assert_eq!(response.status(), StatusCode::OK);
        let renamed = state
            .servers()
            .into_iter()
            .find(|s| s.id == "REN-1")
            .unwrap();
        assert_eq!(renamed.name, "New Display Name");
    }

    #[tokio::test]
    async fn rename_route_rejects_a_blank_name() {
        let state = route_state();
        let dir = temp_dir("rename-route-blank");
        state
            .register_imported_config_servers(vec![seeded_server("REN-2", &dir)], false)
            .unwrap();

        let response = call_rename(&state, "REN-2", "   ").await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // ---------- P7.23: POST /v1/servers/eula ----------

    async fn call_eula(state: &LifecycleRoutesState, server_id: Option<&str>) -> Response {
        eula(
            State(state.clone()),
            Extension(transfer_credential()),
            Ok(Json(ServerEulaRequestDto {
                server_id: server_id.map(str::to_string),
            })),
        )
        .await
    }

    #[tokio::test]
    async fn eula_route_writes_the_accepted_eula_file() {
        let config_path = temp_dir("eula-route-config").join("server_config_swift.json");
        let servers_root = temp_dir("eula-route-servers");
        let server_dir = servers_root.join("java").join("eula-server");
        std::fs::create_dir_all(&server_dir).unwrap();
        let state = persistent_route_state(&config_path, &servers_root);
        state
            .register_imported_config_servers(vec![seeded_server("EULA-1", &server_dir)], false)
            .unwrap();

        let response = call_eula(&state, Some("EULA-1")).await;
        assert_eq!(response.status(), StatusCode::OK);
        let written = std::fs::read_to_string(server_dir.join("eula.txt")).unwrap();
        assert!(written.contains("eula=true"));
    }

    #[tokio::test]
    async fn eula_route_reports_missing_server_id() {
        let state = route_state();
        let response = call_eula(&state, None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn eula_route_reports_server_not_found() {
        let state = route_state();
        let response = call_eula(&state, Some("does-not-exist")).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
