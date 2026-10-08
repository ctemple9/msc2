//! Server folder, archive, and transfer import route orchestration.

use super::*;

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
        return perform_raw_scan(&source_path, body.import_kind.as_deref()).await;
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

pub(super) fn rescan_import(state: &LifecycleRoutesState) -> Response {
    let servers_root = state.servers_root();
    let operation_id = match state.begin_import_operation(&servers_root.to_string_lossy(), None) {
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

pub(super) fn run_rescan_import(
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
pub(super) fn resolve_is_zip(import_kind: Option<&str>, source_path: &str) -> bool {
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
async fn perform_raw_scan(source_path: &str, import_kind: Option<&str>) -> Response {
    let is_zip = resolve_is_zip(import_kind, source_path);
    let path = Path::new(source_path);

    if (is_zip && !path.is_file()) || (!is_zip && !path.is_dir()) {
        return source_not_found_response(source_path);
    }

    let source_path = source_path.to_string();
    let scan_path = path.to_path_buf();
    let result = tokio::task::spawn_blocking(move || {
        if is_zip {
            scan_zip_source(&scan_path)
        } else {
            Ok(scan_server_directory(&StdRawImportFileSystem, &scan_path))
        }
    })
    .await;

    match result {
        Ok(Ok(info)) => Json(scan_response_dto(&source_path, is_zip, &info)).into_response(),
        Ok(Err(error)) => raw_import_error_response(error),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "background_worker_failed",
            &format!("server import scan worker failed: {error}"),
        ),
    }
}

pub(super) fn source_not_found_response(source_path: &str) -> Response {
    error_response(
        StatusCode::NOT_FOUND,
        "not_found",
        &format!("import source not found: {source_path}"),
    )
}

pub(super) fn scan_response_dto(
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
pub(super) fn world_to_dto(world: &DetectedWorld) -> ServerImportWorldDto {
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
pub(super) fn import_raw(
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
    let operation_id = match state.begin_import_operation(&operation_target, server_type) {
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
        check_addon_updates: body.check_addon_updates,
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

pub(super) fn run_raw_import(
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

pub(super) fn infer_import_server_type_from_source(
    source: &RawImportSource,
) -> Result<ServerType, RawImportError> {
    match source {
        RawImportSource::Zip(path) => scan_zip_source(path).map(|info| info.server_type),
        RawImportSource::Folder(path) => {
            Ok(scan_server_directory(&StdRawImportFileSystem, path).server_type)
        }
    }
}

pub(super) fn remove_unregistered_raw_import(request: &RawImportRequest, config: &ConfigServer) {
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

pub(super) fn accepted_import_response(operation_id: &OperationId, message: &str) -> Response {
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

pub(super) fn raw_import_error_code(error: &RawImportError) -> &'static str {
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

pub(super) fn raw_import_error_response(error: RawImportError) -> Response {
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

pub(super) fn default_display_name(source_path: &str) -> String {
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
pub(super) fn agent_home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

// ---------- Transfer-package import (P5.16/P5.17) ----------

pub(super) trait ConfiguredServerStore {
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
pub(super) struct ConfigServerStore {
    servers: Mutex<Vec<ConfigServer>>,
}

#[cfg(test)]
impl ConfigServerStore {
    pub(super) fn new() -> Self {
        ConfigServerStore {
            servers: Mutex::new(Vec::new()),
        }
    }

    pub(super) fn snapshot(&self) -> Vec<ConfigServer> {
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
pub(super) fn test_java_server_port(server: &ConfigServer) -> Option<i64> {
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
pub(super) trait TransferImportPorts {
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

pub(super) struct RealTransferImportPorts;

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
pub(super) enum TransferMode {
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

pub(super) struct TransferImportPlan {
    pub(super) package_path: PathBuf,
    pub(super) mode: TransferMode,
    pub(super) backup_path: Option<String>,
    pub(super) java_port_overrides: HashMap<String, i64>,
    pub(super) bedrock_port_overrides: HashMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TransferImportRouteError {
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
pub(super) fn perform_transfer_import(
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

pub(super) fn import_transfer(
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

    let operation_id = match state.begin_import_operation(source_path, None) {
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

pub(super) fn run_transfer_import(
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

pub(super) fn transfer_error_code(error: &TransferImportRouteError) -> &'static str {
    match error {
        TransferImportRouteError::BackupPathRequired => "backup_path_required",
        TransferImportRouteError::BackupFailed(_) => "backup_failed",
        TransferImportRouteError::InvalidPackage(_) => "invalid_transfer_package",
        TransferImportRouteError::SecretWipeFailed(_) => "secret_wipe_failed",
        TransferImportRouteError::SaveFailed(_) => "internal_error",
    }
}

pub(super) fn transfer_error_message(error: &TransferImportRouteError) -> String {
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
/// durable-by-default; flagged for the owner alongside the registry-split
/// gap above.
pub(super) fn transfer_staging_root() -> PathBuf {
    std::env::temp_dir().join(format!("msc2-transfer-staging-{}", unique_suffix()))
}

pub(super) fn unique_suffix() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{nanos}-{count}", std::process::id())
}

pub(super) fn agent_host_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "MSC 2 Agent".to_string())
}

pub(super) fn iso8601_now() -> String {
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
pub(super) fn civil_from_days(z: i64) -> (i64, u32, u32) {
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
