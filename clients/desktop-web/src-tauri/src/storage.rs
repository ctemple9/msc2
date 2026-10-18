//! Local footprint and bounded saved-host metadata. No generic path-read command.
use crate::{
    credential_key, desktop_secret_store, relative_request_url, StoredDesktopCredential,
    LOCAL_HOST_ID_KEY,
};
use msc_application::storage::{InventoryService, ScanHandle};
use msc_infrastructure::{
    storage::{self, Category, Manifest, Progress, Root, Snapshot},
    uninstall::{self, native, EntryKind, EntryState},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::Duration,
};
use tauri::Manager;
const CACHE_LIMIT: u64 = 8 * 1024 * 1024;
static LOCAL: OnceLock<Mutex<LocalState>> = OnceLock::new();
static REMOTE_SCANS: Mutex<BTreeMap<String, (String, String)>> = Mutex::new(BTreeMap::new());
static REMOTE: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
struct LocalState {
    service: InventoryService,
    scan: Option<ScanHandle>,
    manifest: Option<Manifest>,
    persisted: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalFootprint {
    pub local_id: String,
    pub agent_host_id: Option<String>,
    pub snapshot: Option<Snapshot>,
    pub scan_id: Option<String>,
    pub done: bool,
    pub historical: bool,
    pub progress: Progress,
    pub problem: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct CachedSnapshot {
    credential: String,
    snapshot: Snapshot,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteRequest {
    pub agent_host_id: String,
    pub action: String,
    pub operation_id: Option<String>,
    pub selection: Option<msc_api::dto::StorageReviewRequestDto>,
    pub cleanup: Option<msc_api::dto::StorageReviewedCleanupRequestDto>,
}

fn root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("storage-inventory"))
        .map_err(|e| e.to_string())
}
fn path(cache: &Path, host: &str) -> PathBuf {
    cache.join(format!("{}.json", storage::identity(&[host])))
}
fn credential(host: &str) -> Result<String, String> {
    desktop_secret_store()?
        .get(&credential_key(host))
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No credential for this saved computer".into())
}
fn local_id(cache: &Path) -> String {
    format!("desktop_{}", storage::identity(&[&cache.to_string_lossy()]))
}
fn cached(cache: &Path, host: &str, fingerprint: &str) -> Option<Snapshot> {
    let bytes = storage::read_small(&path(cache, host), CACHE_LIMIT).ok()?;
    let value: CachedSnapshot = serde_json::from_slice(&bytes).ok()?;
    (value.credential == fingerprint
        && value.snapshot.host_id == host
        && value.snapshot.schema_version == 1)
        .then_some(value.snapshot)
}
fn save(cache: &Path, host: &str, fingerprint: &str, snapshot: &Snapshot) -> Result<(), String> {
    std::fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    if std::fs::symlink_metadata(cache)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Storage cache is a link".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(cache, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec(&CachedSnapshot {
        credential: fingerprint.into(),
        snapshot: snapshot.clone(),
    })
    .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > CACHE_LIMIT {
        return Err("Storage metadata exceeds cache budget".into());
    }
    msc_infrastructure::atomic_write::atomic_write(
        &msc_infrastructure::fs::StdFileSystem,
        &path(cache, host),
        &bytes,
    )
    .map_err(|e| e.to_string())
}
fn backend() -> Box<dyn native::LocalServices> {
    #[cfg(target_os = "macos")]
    {
        Box::new(msc_platform_macos::uninstall::MacUninstall)
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(msc_platform_windows::uninstall::WindowsUninstall)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(msc_platform_linux::uninstall::LinuxUninstall::new())
    }
}
fn discover_local(host: &str, client_data: &Path) -> Result<Manifest, String> {
    let origin = std::env::current_exe().map_err(|e| e.to_string())?;
    let discovery = native::discover(
        backend().as_ref(),
        &native::Options {
            origin: origin.clone(),
            desktop_origin: Some(origin),
            installers: vec![],
        },
        option_env!("MSC2_RELEASE_PUBLIC_KEY_HEX"),
    );
    let mut issues = vec![];
    let found = match discovery {
        Ok(found) => found,
        Err(error) => {
            issues.push(format!(
                "Service ownership unavailable; desktop measured independently: {error}"
            ));
            independent_discovery()?
        }
    };
    let mut roots = vec![Root {
        path: client_data.to_path_buf(),
        category: Category::DesktopApp,
        label: "Desktop client data".into(),
        server_id: None,
        evidence: "Tauri-resolved application data directory".into(),
        rules: vec![],
    }];
    let package_roots = match package_roots(&found) {
        Ok(roots) => roots,
        Err(error) => {
            issues.push(error);
            vec![]
        }
    };
    let package_measured = !package_roots.is_empty();
    for path in package_roots {
        roots.push(Root {
            path,
            category: Category::DesktopApp,
            label: "Installed desktop payload".into(),
            server_id: None,
            evidence: "Verified native package installation".into(),
            rules: vec![],
        });
    }
    for entry in found.inventory.entries {
        if entry.state == EntryState::Blocked
            && matches!(entry.kind, EntryKind::Installation | EntryKind::ClientState)
        {
            issues.push(
                entry
                    .problem
                    .unwrap_or_else(|| "Desktop source unavailable".into()),
            );
            continue;
        }
        if entry.state != EntryState::Present {
            continue;
        }
        let category = match entry.kind {
            EntryKind::ClientState if entry.path.as_ref().is_some_and(|path| path.components().any(|part| part.as_os_str().to_string_lossy().starts_with("com.ctemple.msc2"))) => Category::DesktopApp,
            EntryKind::Installation if entry.path.as_ref().is_some_and(|path| found.installations.iter().any(|installation|
                matches!(installation, native::Installation::MacBundle { path: bundle } if bundle == path))) => Category::DesktopApp,
            EntryKind::Installation if entry.path.is_none() => { if !package_measured { issues.push("Installed package file coverage is unknown".into()); } continue; }
            _ => continue,
        };
        if let Some(path) = entry.path {
            // Machine service payload is agent-owned even when uninstall's
            // broad discovery reports it alongside Windows client paths.
            if path.to_string_lossy().contains("ProgramData") {
                continue;
            }
            roots.push(Root {
                path,
                category,
                label: "Desktop app and client data".into(),
                server_id: None,
                evidence: entry.evidence,
                rules: vec![],
            });
        }
    }
    roots.sort_by(|a, b| a.path.cmp(&b.path));
    roots.dedup_by(|a, b| a.path == b.path);
    let generation = storage::identity(&[host, &format!("{roots:?}")]);
    Ok(Manifest {
        host_id: host.into(),
        generation,
        roots,
        excluded_roots: vec![],
        issues,
    })
}
// A stopped/uninspectable agent must not hide this user's desktop footprint.
fn independent_discovery() -> Result<native::Preview, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or("Current home is unavailable")?;
    let platform = if cfg!(target_os = "macos") {
        uninstall::Platform::Macos
    } else if cfg!(windows) {
        uninstall::Platform::Windows
    } else {
        uninstall::Platform::Linux
    };
    let origin = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut request = uninstall::Request {
        computer: "This computer".into(),
        platform,
        home,
        overrides: uninstall::Overrides::default(),
        services: vec![],
        protected_paths: vec![],
        client_paths: vec![],
        headless_roots: vec![],
        installer_candidates: vec![],
        findings: vec![],
    };
    let installations = native::inspect_installations(
        &native::Options {
            origin: origin.clone(),
            desktop_origin: Some(origin),
            installers: vec![],
        },
        &request,
    )
    .unwrap_or_default();
    for installation in &installations {
        let path = match installation {
            native::Installation::MacBundle { path }
            | native::Installation::MarkedArchive { path }
            | native::Installation::LinuxArchive { path } => Some(path.clone()),
            _ => None,
        };
        request.findings.push(uninstall::Entry {
            kind: EntryKind::Installation,
            path,
            identity: format!("{installation:?}"),
            evidence: "Verified local installation identity".into(),
            state: EntryState::Present,
            problem: None,
        });
    }
    let inventory = uninstall::inventory(&msc_infrastructure::fs::StdFileSystem, &request);
    Ok(native::Preview {
        inventory,
        request,
        installations,
        agent_data_dir: crate::agent_data_directory()?,
        agent_running: false,
    })
}

// Package queries happen once per explicit discovery, never once per file.
fn package_roots(found: &native::Preview) -> Result<Vec<PathBuf>, String> {
    #[cfg(target_os = "macos")]
    {
        let _ = found;
        Ok(vec![])
    }
    #[cfg(target_os = "linux")]
    {
        let mut paths = vec![];
        for installation in &found.installations {
            let files = match installation {
                native::Installation::Deb { name } => native::capture("dpkg-query", &["-L", name])?,
                native::Installation::Rpm { name } => native::capture("rpm", &["-ql", name])?,
                _ => continue,
            };
            for path in files.lines().map(PathBuf::from) {
                if path.is_absolute()
                    && std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_file())
                {
                    msc_infrastructure::uninstall::validate_target(
                        &msc_infrastructure::fs::StdFileSystem,
                        &found.request,
                        &path,
                    )?;
                    paths.push(path);
                }
            }
        }
        Ok(paths)
    }
    #[cfg(target_os = "windows")]
    {
        let origin = std::fs::canonicalize(std::env::current_exe().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let mut paths = vec![];
        for installation in &found.installations {
            let native::Installation::Msi { product } = installation else {
                continue;
            };
            if !product
                .chars()
                .all(|c| c.is_ascii_hexdigit() || matches!(c, '{' | '}' | '-'))
            {
                return Err("Invalid installed MSI identity".into());
            }
            let script = format!(
                r"$ErrorActionPreference='Stop'; foreach ($base in @('HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall','HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall')) {{ $p=Get-ItemProperty -LiteralPath ($base+'\{product}') -ErrorAction SilentlyContinue; if ($p.DisplayName -eq 'MSC 2' -and $p.WindowsInstaller -eq 1) {{ $p.InstallLocation }} }}"
            );
            let locations = native::capture(
                "powershell.exe",
                &["-NoProfile", "-NonInteractive", "-Command", &script],
            )?;
            for path in locations
                .lines()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
            {
                msc_infrastructure::uninstall::validate_target(
                    &msc_infrastructure::fs::StdFileSystem,
                    &found.request,
                    &path,
                )?;
                if std::fs::canonicalize(&path).is_ok_and(|path| origin.starts_with(path)) {
                    paths.push(path);
                }
            }
        }
        Ok(paths)
    }
}

fn local_state(id: &str) -> &'static Mutex<LocalState> {
    LOCAL.get_or_init(|| {
        Mutex::new(LocalState {
            service: InventoryService::new(id),
            scan: None,
            manifest: None,
            persisted: String::new(),
        })
    })
}

#[tauri::command]
pub async fn storage_local(
    app: tauri::AppHandle,
    action: String,
) -> Result<LocalFootprint, String> {
    if !matches!(action.as_str(), "cached" | "refresh" | "status" | "cancel") {
        return Err("Unsupported local storage action".into());
    }
    let cache = root(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let id = local_id(&cache);
        let agent = desktop_secret_store()?
            .get(LOCAL_HOST_ID_KEY)
            .map_err(|e| e.to_string())?;
        let mut state = local_state(&id).lock().map_err(|e| e.to_string())?;
        if action == "refresh" {
            let manifest =
                discover_local(&id, cache.parent().ok_or("Missing desktop data directory")?)?;
            let scan = state.service.start(manifest.clone())?;
            state.manifest = Some(manifest);
            state.scan = Some(scan);
        }
        if action == "cancel" {
            if let Some(scan) = &state.scan {
                scan.cancel();
            }
        }
        let current = state.scan.as_ref().and_then(|scan| scan.result());
        let mut problem = state.scan.as_ref().and_then(|scan| scan.failure());
        if let Some(snapshot) = &current {
            if state.persisted != snapshot.id {
                match save(&cache, &id, "local-desktop", snapshot) {
                    Ok(()) => state.persisted = snapshot.id.clone(),
                    Err(error) => problem = Some(error),
                }
            }
        }
        let historical = current.is_none();
        Ok(LocalFootprint {
            local_id: id.clone(),
            agent_host_id: agent,
            snapshot: current
                .map(|snapshot| snapshot.as_ref().clone())
                .or_else(|| cached(&cache, &id, "local-desktop")),
            scan_id: state.scan.as_ref().map(|scan| scan.id.clone()),
            done: state.scan.as_ref().is_none_or(|scan| scan.is_done()),
            historical,
            progress: state
                .scan
                .as_ref()
                .map(|scan| scan.progress())
                .unwrap_or_default(),
            problem,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn storage_cached(
    app: tauri::AppHandle,
    agent_host_id: String,
) -> Result<Option<Snapshot>, String> {
    let cache = root(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let raw = credential(&agent_host_id)?;
        Ok(cached(&cache, &agent_host_id, &storage::identity(&[&raw])))
    })
    .await
    .map_err(|e| e.to_string())?
}

async fn bounded(mut response: reqwest::Response) -> Result<serde_json::Value, String> {
    let status = response.status();
    if response.content_length().is_some_and(|n| n > CACHE_LIMIT) {
        return Err("Storage response too large".into());
    }
    let mut bytes = vec![];
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > CACHE_LIMIT as usize {
            return Err("Storage response exceeds metadata budget".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
        let message = value["message"]
            .as_str()
            .unwrap_or("Computer unavailable or this agent does not support the storage request");
        return Err(format!(
            "Storage request refused ({status}): {}",
            message.chars().take(2000).collect::<String>()
        ));
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn storage_remote(
    app: tauri::AppHandle,
    request: RemoteRequest,
) -> Result<serde_json::Value, String> {
    let _permit = REMOTE
        .try_acquire()
        .map_err(|_| "Two storage host requests are already active")?;
    let raw = credential(&request.agent_host_id)?;
    let record: StoredDesktopCredential = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let id = request.operation_id.as_deref().unwrap_or("");
    if id.len() > 128
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err("Invalid storage operation identity".into());
    }
    let mut payload = serde_json::json!({});
    let (method, path) = match request.action.as_str() {
        "candidates" | "review" => {
            let selection = request
                .selection
                .as_ref()
                .ok_or("Storage selection is required")?;
            if selection.host_id != request.agent_host_id || selection.item_ids.len() > 64 {
                return Err("Storage selection belongs to another computer or is too large".into());
            }
            let allowed = matches!(
                (selection.category.as_str(), selection.action.as_str()),
                ("cache", "map-cache" | "old-downloads")
                    | ("agent", "obsolete-builds")
                    | (
                        "servers",
                        "delete-backups" | "remove-world" | "remove-server"
                    )
            );
            if !allowed {
                return Err("Unsupported storage selection".into());
            }
            if request.action == "candidates" {
                (
                    reqwest::Method::GET,
                    format!(
                        "/v1/host/storage/candidates?category={}&action={}",
                        selection.category, selection.action
                    ),
                )
            } else {
                payload = serde_json::to_value(selection).map_err(|e| e.to_string())?;
                (reqwest::Method::POST, "/v1/host/storage/reviews".into())
            }
        }
        "cleanup" => {
            let cleanup = request
                .cleanup
                .as_ref()
                .ok_or("Reviewed confirmation is required")?;
            if cleanup.host_id != request.agent_host_id {
                return Err("Review belongs to another computer".into());
            }
            payload = serde_json::to_value(cleanup).map_err(|e| e.to_string())?;
            (reqwest::Method::POST, "/v1/host/storage/cleanups".into())
        }
        "result" if !id.is_empty() => (
            reqwest::Method::GET,
            format!("/v1/host/storage/cleanups/{id}"),
        ),
        "inventory" => (
            reqwest::Method::GET,
            "/v1/host/storage/inventory".to_string(),
        ),
        "refresh" => (reqwest::Method::POST, "/v1/host/storage/scans".to_string()),
        "status" if !id.is_empty() => {
            (reqwest::Method::GET, format!("/v1/host/storage/scans/{id}"))
        }
        "cancel" if !id.is_empty() => {
            (reqwest::Method::POST, format!("/v1/operations/{id}/cancel"))
        }
        _ => return Err("Unsupported storage action".into()),
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|e| e.to_string())?;
    let send = |method: reqwest::Method, path: String| {
        let url = relative_request_url(&record.base_url, &path);
        let client = client.clone();
        let token = record.token.clone();
        let payload = payload.clone();
        async move {
            let mut request = client.request(method.clone(), url?).bearer_auth(token);
            if method == reqwest::Method::POST {
                request = request.json(&payload);
            }
            bounded(
                request
                    .send()
                    .await
                    .map_err(|e| format!("Computer unavailable: {e}"))?,
            )
            .await
        }
    };
    if request.action == "refresh" {
        let mut scans = REMOTE_SCANS.lock().map_err(|e| e.to_string())?;
        scans.retain(|host, (_, fingerprint)| {
            credential(host).is_ok_and(|raw| storage::identity(&[&raw]) == *fingerprint)
        });
        if let Some((operation, _)) = scans.get(&request.agent_host_id) {
            if !operation.is_empty() {
                return Ok(
                    serde_json::json!({"hostId":request.agent_host_id,"operationId":operation}),
                );
            }
            return Err("A storage refresh request is already pending".into());
        }
        if scans.len() >= 2 {
            return Err(
                "Two computers are already scanning; check or cancel those scans first".into(),
            );
        }
        scans.insert(
            request.agent_host_id.clone(),
            (String::new(), storage::identity(&[&raw])),
        );
    }
    let mut value = match send(method, path).await {
        Ok(value) => value,
        Err(error) => {
            if request.action == "refresh" {
                REMOTE_SCANS
                    .lock()
                    .map_err(|e| e.to_string())?
                    .remove(&request.agent_host_id);
            }
            return Err(error);
        }
    };
    if credential(&request.agent_host_id)? != raw {
        let mut scans = REMOTE_SCANS.lock().map_err(|e| e.to_string())?;
        if scans
            .get(&request.agent_host_id)
            .is_some_and(|(_, fp)| *fp == storage::identity(&[&raw]))
        {
            scans.remove(&request.agent_host_id);
        }
        return Err("Computer authorization changed during storage request".into());
    }
    if request.action == "refresh" {
        if let Some(operation) = value["operationId"].as_str() {
            REMOTE_SCANS.lock().map_err(|e| e.to_string())?.insert(
                request.agent_host_id.clone(),
                (operation.to_string(), storage::identity(&[&raw])),
            );
        } else {
            REMOTE_SCANS
                .lock()
                .map_err(|e| e.to_string())?
                .remove(&request.agent_host_id);
            return Err("Computer returned no scan identity".into());
        }
    }
    if request.action == "status"
        && matches!(
            value["state"].as_str(),
            Some("complete" | "partial" | "cancelled" | "failed" | "succeeded" | "interrupted")
        )
    {
        REMOTE_SCANS
            .lock()
            .map_err(|e| e.to_string())?
            .remove(&request.agent_host_id);
    }
    if request.action != "cancel" && value["hostId"].as_str() != Some(&request.agent_host_id) {
        return Err("Storage reply belongs to another computer".into());
    }
    if request.action == "inventory" && !value["snapshot"].is_null() {
        let mut snapshot: Snapshot =
            serde_json::from_value(value["snapshot"].clone()).map_err(|e| e.to_string())?;
        let mut cursor = value["nextCursor"].as_str().map(str::to_owned);
        for _ in 0..50 {
            let Some(next) = cursor.take() else {
                break;
            };
            if next.len() > 160
                || !next
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | ':'))
            {
                return Err("Invalid storage page identity".into());
            }
            let page = send(
                reqwest::Method::GET,
                format!("/v1/host/storage/inventory?cursor={next}"),
            )
            .await?;
            let other: Snapshot =
                serde_json::from_value(page["snapshot"].clone()).map_err(|e| e.to_string())?;
            if page["hostId"].as_str() != Some(&request.agent_host_id)
                || other.id != snapshot.id
                || other.host_id != snapshot.host_id
            {
                return Err("Computer or snapshot changed during paging".into());
            }
            if snapshot.items.len() + other.items.len() > 10_000 {
                return Err("Storage item budget exceeded".into());
            }
            snapshot.items.extend(other.items);
            cursor = page["nextCursor"].as_str().map(str::to_owned);
        }
        if cursor.is_some() || snapshot.host_id != request.agent_host_id {
            return Err("Storage paging did not finish on this computer".into());
        }
        if credential(&request.agent_host_id)? != raw {
            return Err("Computer authorization changed during storage request".into());
        }
        let cache = root(&app)?;
        let copy = snapshot.clone();
        let host = request.agent_host_id.clone();
        tauri::async_runtime::spawn_blocking(move || {
            save(&cache, &host, &storage::identity(&[&raw]), &copy)
        })
        .await
        .map_err(|e| e.to_string())??;
        value["snapshot"] = serde_json::to_value(snapshot).map_err(|e| e.to_string())?;
        value["nextCursor"] = serde_json::Value::Null;
    }
    Ok(value)
}

#[tauri::command]
pub async fn storage_reveal(
    app: tauri::AppHandle,
    item_id: String,
    agent_host_id: Option<String>,
) -> Result<(), String> {
    let cache = root(&app)?;
    let path = tauri::async_runtime::spawn_blocking(move || {
        if let Some(host) = agent_host_id {
            if desktop_secret_store()?
                .get(LOCAL_HOST_ID_KEY)
                .map_err(|e| e.to_string())?
                .as_deref()
                != Some(&host)
            {
                return Err("Remote folders cannot be opened on this computer".into());
            }
            let raw = credential(&host)?;
            let snapshot = cached(&cache, &host, &storage::identity(&[&raw]))
                .ok_or("No verified local agent snapshot")?;
            let item = snapshot
                .items
                .iter()
                .find(|item| item.id == item_id)
                .ok_or("Local agent item not found")?;
            let origin = std::env::current_exe().map_err(|e| e.to_string())?;
            let found = native::discover(
                backend().as_ref(),
                &native::Options {
                    origin: origin.clone(),
                    desktop_origin: Some(origin),
                    installers: vec![],
                },
                option_env!("MSC2_RELEASE_PUBLIC_KEY_HEX"),
            )?;
            let path = found
                .inventory
                .entries
                .iter()
                .filter(|entry| entry.state == EntryState::Present)
                .filter_map(|entry| entry.path.as_ref())
                .find(|path| {
                    storage::verified_root_id(&host, path).ok().as_deref() == Some(&item.root_id)
                })
                .cloned();
            if let Some(path) = path.filter(|path| path.is_dir()) {
                return Ok(path);
            }

            // Some inventory roots are discovered inside MSC's owned server
            // tree. The uninstall inventory knows the parent, while the agent
            // snapshot supplies the child path and its independently measured
            // identity. Require both proofs before opening that child.
            let candidate = item
                .display_path
                .as_deref()
                // The scanner joins an empty relative path for root items,
                // which leaves a separator that was absent from the root ID.
                .map(|path| Path::new(path).components().collect::<PathBuf>())
                .ok_or("Local folder identity or ownership changed")?;
            let verified_parent = found.inventory.entries.iter().any(|entry| {
                entry.kind == EntryKind::ServerTree
                    && entry.state == EntryState::Present
                    && entry.path.as_ref().is_some_and(|parent| {
                        candidate.strip_prefix(parent).is_ok_and(|relative| {
                            !relative.as_os_str().is_empty()
                                && relative
                                    .components()
                                    .all(|component| {
                                        matches!(component, std::path::Component::Normal(_))
                                    })
                                && storage::verified_root_id(&host, parent).is_ok()
                                && candidate.is_dir()
                                && storage::verified_root_id(&host, &candidate)
                                    .is_ok_and(|root_id| root_id == item.root_id)
                        })
                    })
            });
            if !verified_parent {
                return Err("Local folder identity or ownership changed".into());
            }
            return Ok(candidate);
        }
        let id = local_id(&cache);
        let state = local_state(&id).lock().map_err(|e| e.to_string())?;
        let snapshot = state
            .scan
            .as_ref()
            .and_then(|scan| scan.result())
            .ok_or("Refresh local storage before opening a folder")?;
        let item = snapshot
            .items
            .iter()
            .find(|item| item.id == item_id)
            .ok_or("Local inventory item was not found")?;
        let manifest = state
            .manifest
            .as_ref()
            .ok_or("Local source information expired")?;
        let root = manifest
            .roots
            .iter()
            .find(|root| {
                storage::verified_root_id(&id, &root.path).ok().as_deref() == Some(&item.root_id)
            })
            .ok_or("Local folder identity changed; refresh storage")?;
        if !root.path.is_dir() {
            return Err("This inventory item is not a local folder".to_string());
        }
        Ok::<_, String>(root.path.clone())
    })
    .await
    .map_err(|e| e.to_string())??;
    crate::reveal_in_file_manager(path.to_string_lossy().into_owned())
}

/// Local and agent measurements share a filesystem identity, not a display
/// name. Unknown identities stay partial rather than being summed twice.
pub fn compose(local: &Snapshot, agent: &Snapshot, bound_host: &str) -> Result<Snapshot, String> {
    if agent.host_id != bound_host {
        return Err("Local agent identity did not match".into());
    }
    let agent_roots: BTreeSet<_> = agent
        .items
        .iter()
        .filter(|i| !i.physical_root_id.is_empty())
        .map(|i| i.physical_root_id.clone())
        .collect();
    let normalize = |p: &str| {
        if cfg!(windows) {
            p.replace('\\', "/").to_lowercase()
        } else {
            p.to_string()
        }
    };
    let overlaps = |a: &str, b: &str| {
        let a = normalize(a);
        let b = normalize(b);
        a == b || a.starts_with(&(b.clone() + "/")) || b.starts_with(&(a + "/"))
    };
    let mut result = agent.clone();
    for item in &local.items {
        // The agent already deduplicated its complete graph, including links
        // charged to another category. Prefer that graph for matching roots.
        if agent_roots.contains(&item.physical_root_id) {
            continue;
        }
        let uncertain = item.physical_root_id.is_empty()
            || item.has_hard_links
            || item.display_path.as_ref().is_some_and(|path| {
                agent
                    .items
                    .iter()
                    .filter_map(|i| i.display_path.as_deref())
                    .any(|other| overlaps(path, other))
            });
        if uncertain {
            result.partial = true;
            result.issues.push(storage::Issue { root_id:Some(item.root_id.clone()), reason:"Desktop source overlaps agent coverage or has shared links; its separate measurement is excluded from the combined total".into() });
        } else {
            result.items.push(item.clone());
        }
    }
    result.logical_bytes = result.items.iter().map(|i| i.logical_bytes).sum();
    result.allocated_bytes = result
        .items
        .iter()
        .try_fold(0u64, |n, i| n.checked_add(i.allocated_bytes?));
    result.partial |= local.partial || agent.items.iter().any(|i| i.physical_root_id.is_empty());
    for volume in &local.volumes {
        if let Some(existing) = result
            .volumes
            .iter_mut()
            .find(|v| !volume.physical_id.is_empty() && v.physical_id == volume.physical_id)
        {
            // macOS reclaimable capacity can be unavailable in a LaunchDaemon
            // session. The verified local desktop samples in the user's session.
            // Preserve the agent's accounting ID while using that dated sample.
            if volume.available_bytes.is_some() && volume.problem.is_none() {
                existing.total_bytes = volume.total_bytes;
                existing.available_bytes = volume.available_bytes;
                existing.observed_at = volume.observed_at;
                existing.problem = None;
            }
            let id = existing.id.clone();
            for item in &mut result.items {
                if item.volume_id.as_deref() == Some(&volume.id) {
                    item.volume_id = Some(id.clone());
                }
            }
        } else {
            result.volumes.push(volume.clone());
        }
    }
    Ok(result)
}

#[tauri::command]
pub async fn storage_composed(
    app: tauri::AppHandle,
    agent_host_id: String,
) -> Result<Option<Snapshot>, String> {
    let cache = root(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let store = desktop_secret_store()?;
        if store
            .get(LOCAL_HOST_ID_KEY)
            .map_err(|e| e.to_string())?
            .as_deref()
            != Some(&agent_host_id)
        {
            return Err("This is not the verified local agent identity".into());
        }
        let raw = credential(&agent_host_id)?;
        let agent = cached(&cache, &agent_host_id, &storage::identity(&[&raw]));
        let id = local_id(&cache);
        let local = local_state(&id)
            .lock()
            .map_err(|e| e.to_string())?
            .scan
            .as_ref()
            .and_then(|scan| scan.result())
            .map(|s| s.as_ref().clone())
            .or_else(|| cached(&cache, &id, "local-desktop"));
        match (local, agent) {
            (Some(local), Some(agent)) => compose(&local, &agent, &agent_host_id).map(Some),
            (Some(mut local), None) => {
                local.partial = true;
                local.issues.push(storage::Issue {
                    root_id: None,
                    reason: "Desktop measured; local agent inventory unavailable".into(),
                });
                Ok(Some(local))
            }
            (None, Some(mut agent)) => {
                agent.partial = true;
                agent.issues.push(storage::Issue {
                    root_id: None,
                    reason: "Local desktop footprint not measured".into(),
                });
                Ok(Some(agent))
            }
            _ => Ok(None),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedSummary {
    pub host_id: String,
    pub snapshot_id: String,
    pub logical_bytes: u64,
    pub partial: bool,
    pub completed_at: u64,
    pub volumes: Vec<storage::Volume>,
}
#[tauri::command]
pub async fn storage_overview(
    app: tauri::AppHandle,
    agent_host_ids: Vec<String>,
) -> Result<Vec<CachedSummary>, String> {
    if agent_host_ids.len() > 64 {
        return Err("Storage overview supports at most 64 saved computers at once".into());
    }
    let cache = root(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut results = vec![];
        for host in agent_host_ids {
            let Ok(raw) = credential(&host) else {
                continue;
            };
            if let Some(snapshot) = cached(&cache, &host, &storage::identity(&[&raw])) {
                results.push(CachedSummary {
                    host_id: host,
                    snapshot_id: snapshot.id,
                    logical_bytes: snapshot.logical_bytes,
                    partial: snapshot.partial,
                    completed_at: snapshot.completed_at,
                    volumes: snapshot.volumes,
                });
            }
        }
        Ok(results)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    // Existing agent accounting tests do not combine native/agent reports.
    // Guards duplicate local app/capacity totals and false locality. No I/O, <1 ms.
    #[test]
    fn matching_local_roots_and_volumes_are_counted_once() {
        let manifest = Manifest {
            host_id: "desktop".into(),
            generation: "one".into(),
            roots: vec![],
            excluded_roots: vec![],
            issues: vec![],
        };
        let mut local = storage::scan(
            &manifest,
            storage::Limits::default(),
            &|| false,
            &mut |_| {},
        );
        local.items.push(serde_json::from_value(serde_json::json!({
            "id":"desktop-item","rootId":"desktop-root","physicalRootId":"same-root","volumeId":"desktop-volume",
            "category":"desktopApp","group":"installation","domainId":null,"serverId":null,"label":"Desktop app",
            "logicalBytes":8,"allocatedBytes":8,"partial":false,"absent":false,"sharedReferences":0,"protection":"Current installation"
        })).unwrap());
        local.volumes.push(storage::Volume {
            id: "desktop-volume".into(),
            physical_id: "same-volume".into(),
            label: "Fixture volume".into(),
            total_bytes: Some(100),
            available_bytes: Some(50),
            observed_at: 0,
            problem: None,
        });
        let mut agent = local.clone();
        agent.host_id = "agent-one".into();
        agent.items[0].id = "agent-item".into();
        agent.items[0].volume_id = Some("agent-volume".into());
        agent.volumes[0].id = "agent-volume".into();
        // Essential: the background capacity API returned a false zero while
        // the user's desktop had a valid available-capacity sample. No I/O.
        agent.volumes[0].available_bytes = Some(0);
        let combined = compose(&local, &agent, "agent-one").unwrap();
        assert_eq!(combined.logical_bytes, 8);
        assert_eq!(combined.items.len(), 1);
        assert_eq!(combined.volumes.len(), 1);
        assert_eq!(combined.volumes[0].available_bytes, Some(50));
        assert_eq!(combined.items[0].volume_id.as_deref(), Some("agent-volume"));
        assert!(compose(&local, &agent, "another-agent").is_err());
        local.items[0].physical_root_id = "another-root".into();
        local.items[0].has_hard_links = true;
        let partial = compose(&local, &agent, "agent-one").unwrap();
        assert!(partial.partial);
        assert_eq!(partial.logical_bytes, 8);
    }
}

#[tauri::command]
pub fn storage_active_scans() -> Result<BTreeMap<String, String>, String> {
    Ok(REMOTE_SCANS
        .lock()
        .map_err(|e| e.to_string())?
        .iter()
        .filter(|(host, (_, fingerprint))| {
            credential(host).is_ok_and(|raw| storage::identity(&[&raw]) == fingerprint.as_str())
        })
        .map(|(host, (operation, _))| (host.clone(), operation.clone()))
        .collect())
}
