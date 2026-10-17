//! Explicit, journaled inventory scans; no recurring traversal or cleanup.
use super::*;
use axum::extract::{Path as RoutePath, Query};
use msc_api::dto::{StorageInventoryDto, StorageProgressDto, StorageScanDto, StorageSnapshotDto};
use msc_application::storage::{Discovery, InventoryService, ScanHandle};
use msc_domain::operation::OperationId;
use msc_infrastructure::{
    storage::{self, Manifest, Snapshot},
    uninstall::{self, native},
};
use std::path::PathBuf;

pub(super) struct InventoryState {
    host_id: String,
    service: InventoryService,
    active: Option<(OperationId, ScanHandle)>,
    historical: Option<Arc<Snapshot>>,
}
impl InventoryState {
    pub(super) fn new() -> Self {
        Self {
            host_id: String::new(),
            service: InventoryService::new("uninitialized"),
            active: None,
            historical: None,
        }
    }
}

pub(super) fn backend() -> Box<dyn native::LocalServices> {
    #[cfg(target_os = "macos")]
    {
        Box::new(msc_platform_macos::uninstall::MacUninstall)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(msc_platform_linux::uninstall::LinuxUninstall::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(msc_platform_windows::uninstall::WindowsUninstall)
    }
}

pub(super) fn discover(lifecycle: &LifecycleRoutesState, host: &str) -> Result<Manifest, String> {
    let data = msc_infrastructure::config_repository::default_app_data_dir();
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or("Cannot determine the service account's home")?;
    let platform = if cfg!(target_os = "macos") {
        uninstall::Platform::Macos
    } else if cfg!(windows) {
        uninstall::Platform::Windows
    } else {
        uninstall::Platform::Linux
    };
    let services = backend();
    let mut problems = vec![];
    let reports = services.inspect().unwrap_or_else(|error| {
        problems.push(error);
        vec![]
    });
    let request = uninstall::Request {
        computer: host.into(),
        platform,
        home,
        overrides: uninstall::Overrides {
            data_dir: Some(data),
            config_path: Some(lifecycle.app_config_path()),
            servers_root: Some(lifecycle.servers_root()),
        },
        services: reports,
        protected_paths: vec![],
        client_paths: vec![],
        headless_roots: vec![],
        installer_candidates: vec![],
        findings: vec![],
    };
    let mut ownership = uninstall::inventory(&msc_infrastructure::fs::StdFileSystem, &request);
    let mut desktop = vec![];
    if let Ok(origin) = std::env::current_exe() {
        match native::discover(
            services.as_ref(),
            &native::Options {
                origin,
                desktop_origin: None,
                installers: vec![],
            },
            option_env!("MSC2_RELEASE_PUBLIC_KEY_HEX"),
        ) {
            Ok(found) => {
                for installation in found.installations {
                    if let native::Installation::MacBundle { path } = installation {
                        desktop.push(path);
                    }
                }
                ownership.entries.extend(found.inventory.entries);
            }
            Err(error) => problems.push(error),
        }
    }
    let config = lifecycle.app_config_snapshot();
    let root = lifecycle.servers_root();
    let mut manifest = msc_application::storage::discover(&Discovery {
        host_id: host,
        config: &config,
        servers_root: &root,
        ownership: &ownership,
        desktop_installations: &desktop,
    });
    manifest.issues.extend(problems);
    Ok(manifest)
}

fn cache_path(host: &str) -> PathBuf {
    msc_infrastructure::config_repository::default_app_data_dir()
        .join("storage-inventory")
        .join(format!("{}.json", storage::identity(&[host])))
}
fn load(host: &str) -> Option<Arc<Snapshot>> {
    let bytes = storage::read_small(&cache_path(host), 8 * 1024 * 1024).ok()?;
    decode_cache(host, &bytes)
}
fn decode_cache(host: &str, bytes: &[u8]) -> Option<Arc<Snapshot>> {
    let snapshot: Snapshot = serde_json::from_slice(bytes).ok()?;
    (snapshot.host_id == host && snapshot.schema_version == 1).then(|| Arc::new(snapshot))
}

fn save(snapshot: &Snapshot) -> Result<(), String> {
    let path = cache_path(&snapshot.host_id);
    let parent = path.parent().ok_or("Missing cache parent")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    // Fixed private metadata location, never a server path or execution target.
    if std::fs::symlink_metadata(parent)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Storage cache directory is a link".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec(snapshot).map_err(|e| e.to_string())?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Storage snapshot exceeds the cache budget".into());
    }
    msc_infrastructure::atomic_write::atomic_write(
        &msc_infrastructure::fs::StdFileSystem,
        &path,
        &bytes,
    )
    .map_err(|e| e.to_string())
}

fn ensure_host(state: &mut InventoryState, host: &str) {
    if state.host_id == host {
        return;
    }
    if let Some((_, scan)) = &state.active {
        scan.cancel();
    }
    state.service.clear();
    state.host_id = host.into();
    state.service = InventoryService::new(host);
    state.active = None;
    state.historical = load(host);
}

#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct InventoryQuery {
    cursor: Option<String>,
}

pub(super) async fn inventory(
    State(state): State<StorageState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Query(query): Query<InventoryQuery>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Admin) {
        return response;
    }
    let host = match state.lifecycle.storage_host_id() {
        Ok(host) => host,
        Err(error) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "storage_identity_unavailable",
                &error,
            );
        }
    };
    let inventory = state.inventory.clone();
    let host_copy = host.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut inventory = inventory.lock().unwrap();
        ensure_host(&mut inventory, &host_copy);
        let snapshot = inventory
            .active
            .as_ref()
            .and_then(|(_, scan)| scan.result())
            .or_else(|| inventory.historical.clone());
        let historical = inventory
            .active
            .as_ref()
            .and_then(|(_, scan)| scan.result())
            .is_none();
        (snapshot, historical)
    })
    .await;
    let (snapshot, historical) = match result {
        Ok(result) => result,
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage_cache_failed",
                &error.to_string(),
            );
        }
    };
    let total_items = snapshot.as_ref().map_or(0, |s| s.items.len());
    let mut dto: Option<StorageSnapshotDto> = snapshot
        .as_ref()
        .and_then(|s| serde_json::to_value(s.as_ref()).ok())
        .and_then(|v| serde_json::from_value(v).ok());
    let offset = match (&query.cursor, &dto) {
        (None, _) => 0,
        (Some(cursor), Some(snapshot)) => match cursor.rsplit_once(':') {
            Some((id, offset)) if id == snapshot.id => match offset.parse::<usize>() {
                Ok(n) if n <= total_items => n,
                _ => {
                    return error_response(
                        StatusCode::CONFLICT,
                        "storage_cursor_changed",
                        "Refresh the inventory page",
                    );
                }
            },
            _ => {
                return error_response(
                    StatusCode::CONFLICT,
                    "storage_cursor_changed",
                    "The snapshot changed; refresh the inventory page",
                );
            }
        },
        _ => {
            return error_response(
                StatusCode::CONFLICT,
                "storage_cursor_changed",
                "No current snapshot",
            );
        }
    };
    let next_cursor = dto.as_mut().and_then(|snapshot| {
        let end = (offset + 200).min(total_items);
        snapshot.items = snapshot.items.get(offset..end).unwrap_or_default().to_vec();
        (end < total_items).then(|| format!("{}:{end}", snapshot.id))
    });
    Json(StorageInventoryDto {
        protocol: 1,
        host_id: host,
        historical: historical
            || snapshot
                .as_ref()
                .is_some_and(|s| InventoryService::is_stale(s)),
        snapshot: dto,
        total_items,
        next_cursor,
    })
    .into_response()
}

pub(super) async fn refresh(
    State(state): State<StorageState>,
    Extension(credential): Extension<AuthenticatedCredential>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Admin) {
        return response;
    }
    let host = match state.lifecycle.storage_host_id() {
        Ok(host) => host,
        Err(error) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "storage_identity_unavailable",
                &error,
            );
        }
    };
    let operation = match state.lifecycle.operations().begin_lifecycle(
        "host-storage-scan",
        None,
        "Discovering owned storage",
    ) {
        Ok(operation) => operation,
        Err(error) => return super::super::operations::operation_error_response(error),
    };
    let state_worker = state.clone();
    let operation_worker = operation.clone();
    let host_worker = host.clone();
    tokio::spawn(async move {
        let lifecycle = state_worker.lifecycle.clone();
        let discovery_host = host_worker.clone();
        let manifest =
            tokio::task::spawn_blocking(move || discover(&lifecycle, &discovery_host)).await;
        let manifest = match manifest {
            Ok(Ok(manifest)) => manifest,
            result => {
                let _ = state_worker.lifecycle.operations().fail(
                    &operation_worker,
                    "storage_discovery_failed",
                    format!("Storage discovery failed: {result:?}"),
                );
                return;
            }
        };
        let cancel_before_start = state_worker
            .lifecycle
            .operations()
            .cancellation_check(&operation_worker);
        if cancel_before_start() {
            let _ = state_worker
                .lifecycle
                .operations()
                .cancel(&operation_worker, "Storage discovery cancelled");
            return;
        }
        let admission_operation = operation_worker.clone();
        let scan_state = state_worker.inventory.clone();
        let scan_host = host_worker.clone();
        let started = tokio::task::spawn_blocking(move || {
            let mut inventory = scan_state.lock().unwrap();
            ensure_host(&mut inventory, &scan_host);
            let scan = inventory.service.start(manifest)?;
            inventory.active = Some((operation_worker.clone(), scan.clone()));
            Ok::<_, String>((operation_worker, scan))
        })
        .await;
        let (operation_worker, scan) = match started {
            Ok(Ok(value)) => value,
            result => {
                let error = match result {
                    Ok(Err(error)) => error,
                    Err(error) => error.to_string(),
                    Ok(Ok(_)) => unreachable!(),
                };
                let _ = state_worker.lifecycle.operations().fail(
                    &admission_operation,
                    "storage_scan_busy",
                    error,
                );
                return;
            }
        };
        let cancelled = state_worker
            .lifecycle
            .operations()
            .cancellation_check(&operation_worker);
        while !scan.is_done() {
            if cancelled() {
                scan.cancel();
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        let operations = state_worker.lifecycle.operations();
        if state_worker.lifecycle.storage_host_id().ok().as_deref() != Some(&host_worker) {
            let _ = operations.fail(
                &operation_worker,
                "storage_host_changed",
                "Computer identity changed; result was discarded".into(),
            );
            return;
        }
        if let Some(snapshot) = scan.result() {
            let copy = snapshot.clone();
            let persisted = tokio::task::spawn_blocking(move || save(&copy)).await;
            if !matches!(persisted, Ok(Ok(()))) {
                let _ = operations.fail(
                    &operation_worker,
                    "storage_cache_failed",
                    format!("Scan finished; snapshot persistence failed: {persisted:?}"),
                );
                return;
            }
            state_worker.inventory.lock().unwrap().historical = Some(snapshot.clone());
            if snapshot.cancelled {
                let _ = operations.cancel(
                    &operation_worker,
                    "Storage scan cancelled; partial results retained",
                );
            } else {
                let _ = operations.succeed(
                    &operation_worker,
                    if snapshot.partial {
                        "Storage scan partial"
                    } else {
                        "Storage scan complete"
                    },
                    std::collections::BTreeMap::from([
                        ("snapshotId".into(), snapshot.id.clone()),
                        ("partial".into(), snapshot.partial.to_string()),
                    ]),
                );
            }
        } else {
            let _ = operations.fail(
                &operation_worker,
                "storage_scan_failed",
                scan.failure()
                    .unwrap_or_else(|| "Storage scan failed".into()),
            );
        }
    });
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"hostId":host,"operationId":operation.as_str()})),
    )
        .into_response()
}

pub(super) async fn scan_status(
    State(state): State<StorageState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    RoutePath(id): RoutePath<String>,
) -> Response {
    if let Some(response) = require_permission(&credential, PermissionCategoryDto::Admin) {
        return response;
    }
    let inventory = state.inventory.lock().unwrap();
    let Some((operation, scan)) = inventory
        .active
        .as_ref()
        .filter(|(op, _)| op.as_str() == id)
    else {
        if let Some(operation) = state
            .lifecycle
            .operations()
            .snapshot(&id)
            .filter(|op| op.r#type == "host-storage-scan")
        {
            return Json(StorageScanDto {
                host_id: state.lifecycle.storage_host_id().unwrap_or_default(),
                operation_id: id,
                state: format!("{:?}", operation.state).to_lowercase(),
                progress: StorageProgressDto::default(),
                snapshot_id: operation
                    .result
                    .as_ref()
                    .and_then(|r| r.get("snapshotId"))
                    .and_then(|v| v.as_str())
                    .map(str::to_owned),
                message: operation.status_line,
            })
            .into_response();
        }
        return error_response(
            StatusCode::NOT_FOUND,
            "storage_scan_unavailable",
            "Storage scan was not found",
        );
    };
    let result = scan.result();
    Json(StorageScanDto {
        host_id: state.lifecycle.storage_host_id().unwrap_or_default(),
        operation_id: operation.as_str().into(),
        state: if scan.is_done() {
            if result.as_ref().is_some_and(|s| s.cancelled) {
                "cancelled"
            } else if result.as_ref().is_some_and(|s| s.partial) {
                "partial"
            } else if result.is_some() {
                "complete"
            } else {
                "failed"
            }
        } else {
            "running"
        }
        .into(),
        progress: serde_json::from_value::<StorageProgressDto>(
            serde_json::to_value(scan.progress()).unwrap(),
        )
        .unwrap(),
        snapshot_id: result.map(|s| s.id.clone()),
        message: scan.failure(),
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    // Persisted host metadata did not exist in P21.2. This guards stale/tampered
    // snapshots being attached to a different authenticated agent. No I/O, <1 ms.
    #[test]
    fn persisted_inventory_requires_the_same_host_and_known_schema() {
        let manifest = Manifest {
            host_id: "agent-one".into(),
            generation: "one".into(),
            roots: vec![],
            excluded_roots: vec![],
            issues: vec![],
        };
        let mut snapshot = storage::scan(
            &manifest,
            storage::Limits::default(),
            &|| false,
            &mut |_| {},
        );
        let bytes = serde_json::to_vec(&snapshot).unwrap();
        assert!(decode_cache("agent-one", &bytes).is_some());
        assert!(decode_cache("agent-two", &bytes).is_none());
        snapshot.schema_version = 99;
        assert!(decode_cache("agent-one", &serde_json::to_vec(&snapshot).unwrap()).is_none());
    }
}
