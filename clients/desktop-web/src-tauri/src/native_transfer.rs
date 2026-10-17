//! A picker grants one host/purpose-bound source. File bytes stay in Rust,
//! including when speaking the older agent's chunk protocol.
use crate::{
    canonical_base_url, credential_key, desktop_secret_store, ensure_current_local_agent_service,
    relative_request_url, StoredDesktopCredential, LOCAL_AGENT_ORIGIN,
};
use futures_util::{stream, FutureExt, StreamExt};
use reqwest::{header, Client, Method, Response};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{File, Metadata, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};
use tauri::ipc::Channel;
use tauri_plugin_dialog::DialogExt;
use tokio::io::AsyncReadExt;
use tokio::sync::watch;
use uuid::Uuid;

const BLOCK: usize = 256 * 1024;
const LEGACY_CHUNK: usize = 1024 * 1024;
const METADATA_LIMIT: usize = 256 * 1024;
const SOURCE_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_SOURCE_BYTES: u64 = 20 * 1024 * 1024 * 1024;
static MANAGER: OnceLock<Manager> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Purpose {
    WorldImport,
    ActiveWorldReplace,
    WorldThumbnail,
    ModpackArchive,
    AddonLocalFile,
    CurseforgeManualFile,
    ModpackUnresolvedFile,
}
impl Purpose {
    fn max_source_bytes(self) -> u64 {
        match self {
            Self::AddonLocalFile => 512 * 1024 * 1024,
            Self::CurseforgeManualFile | Self::ModpackUnresolvedFile => MAX_SOURCE_BYTES,
            _ => 10 * 1024 * 1024 * 1024,
        }
    }
    fn chunks(self) -> bool {
        matches!(
            self,
            Self::WorldImport | Self::ActiveWorldReplace | Self::ModpackArchive
        )
    }
    fn allows_permission(self, permission: &str) -> bool {
        permission == "admin"
            || match self {
                Self::WorldImport | Self::ActiveWorldReplace | Self::WorldThumbnail => {
                    permission == "worlds"
                }
                Self::AddonLocalFile => matches!(permission, "worlds" | "addons"),
                _ => permission == "addons",
            }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Binding {
    agent_host_id: String,
    purpose: Purpose,
    target_server_id: Option<String>,
    operation_id: Option<String>,
    file_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PickerRequest {
    binding: Binding,
    label: String,
    #[serde(default)]
    extensions: Vec<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SelectedSource {
    source_id: String,
    binding: Binding,
    name: String,
    size: u64,
    expires_in_seconds: u64,
}
struct Source {
    selected: SelectedSource,
    path: PathBuf,
    file: File,
    identity: Identity,
    expires: Instant,
}

#[derive(PartialEq, Eq)]
struct Identity {
    size: u64,
    modified: SystemTime,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(windows)]
    volume: u32,
    #[cfg(windows)]
    index: u64,
}
impl Identity {
    fn read(file: &File) -> Result<Self, String> {
        let metadata = file
            .metadata()
            .map_err(|_| "Could not inspect the selected file.")?;
        if !metadata.is_file() {
            return Err("Choose a regular file, not a folder or device.".into());
        }
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        #[cfg(windows)]
        let info = {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::Storage::FileSystem::{
                GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
            };
            let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
            // SAFETY: the file owns a live Windows handle and info is writable.
            if unsafe { GetFileInformationByHandle(file.as_raw_handle().cast(), &mut info) } == 0 {
                return Err("Could not identify the selected file.".into());
            }
            info
        };
        Ok(Self {
            size: metadata.len(),
            modified: metadata
                .modified()
                .map_err(|_| "The selected file has no modification identity.")?,
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
            #[cfg(windows)]
            volume: info.dwVolumeSerialNumber,
            #[cfg(windows)]
            index: ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
        })
    }
}
fn regular_metadata(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return false;
        }
    }
    metadata.is_file() && !metadata.file_type().is_symlink()
}
fn open_source(path: &Path) -> Result<File, String> {
    if !regular_metadata(
        &std::fs::symlink_metadata(path).map_err(|_| "The selected file is unavailable.")?,
    ) {
        return Err("Choose a regular file without a symbolic link or reparse point.".into());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options
        .open(path)
        .map_err(|_| "Could not open the selected file safely.")?;
    if !regular_metadata(
        &file
            .metadata()
            .map_err(|_| "Could not inspect the opened file.")?,
    ) {
        return Err("The selected file was replaced by a link or device.".into());
    }
    Ok(file)
}
impl Source {
    fn open(path: PathBuf, binding: Binding) -> Result<Self, String> {
        let file = open_source(&path)?;
        let identity = Identity::read(&file)?;
        if identity.size == 0 || identity.size > binding.purpose.max_source_bytes() {
            return Err("The selected file is empty or exceeds native staging capacity.".into());
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("The selected file name is not valid text.")?
            .to_string();
        let source = Self {
            selected: SelectedSource {
                source_id: Uuid::new_v4().to_string(),
                binding,
                name,
                size: identity.size,
                expires_in_seconds: SOURCE_TTL.as_secs(),
            },
            path,
            file,
            identity,
            expires: Instant::now() + SOURCE_TTL,
        };
        source.validate()?;
        Ok(source)
    }
    fn validate(&self) -> Result<(), String> {
        // The retained handle fixes what is read; reopening only checks whether
        // the original selection was substituted or edited, never supplies bytes.
        if Identity::read(&self.file)? != self.identity
            || Identity::read(&open_source(&self.path)?)? != self.identity
        {
            return Err("The selected source changed. Choose the file again.".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Transport {
    Stream,
    LegacyChunks,
    LegacyBufferedAgent,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Progress {
    operation_id: String,
    agent_host_id: String,
    phase: &'static str,
    bytes_transferred: u64,
    total_bytes: u64,
    bytes_per_second: f64,
    transport: Option<Transport>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UploadResult {
    operation_id: String,
    agent_host_id: String,
    staged_upload_id: String,
    received_bytes: u64,
    sha256: String,
    transport: Transport,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UploadError {
    code: String,
    message: String,
    cleanup: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_host_id: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_id: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    staged_upload_id: Option<Box<str>>,
}
impl UploadError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            cleanup: "not-admitted",
            agent_host_id: None,
            operation_id: None,
            staged_upload_id: None,
        }
    }
}
type Outcome = Result<UploadResult, UploadError>;
type ProgressSink = dyn Fn(Progress) + Send + Sync;
struct Operation {
    id: String,
    host: String,
    cancel: watch::Sender<bool>,
    done: watch::Sender<Option<Outcome>>,
    progress: Mutex<Progress>,
    sink: Arc<ProgressSink>,
    finish: Mutex<()>,
    started: Instant,
    last_emit: Mutex<Instant>,
}
impl Operation {
    fn signal_cancel(&self) {
        let _finish = self.finish.lock().unwrap();
        if self.done.borrow().is_none() {
            self.cancel.send_replace(true);
        }
    }
    fn progress(&self, phase: &'static str, bytes: u64, transport: Option<Transport>, force: bool) {
        let progress = Progress {
            operation_id: self.id.clone(),
            agent_host_id: self.host.clone(),
            phase,
            bytes_transferred: bytes,
            total_bytes: self.progress.lock().unwrap().total_bytes,
            bytes_per_second: bytes as f64 / self.started.elapsed().as_secs_f64().max(0.001),
            transport,
        };
        *self.progress.lock().unwrap() = progress.clone();
        let mut last = self.last_emit.lock().unwrap();
        if force || last.elapsed() >= Duration::from_millis(250) {
            *last = Instant::now();
            (self.sink)(progress);
        }
    }
    async fn wait(&self) -> Outcome {
        let mut done = self.done.subscribe();
        loop {
            if let Some(result) = done.borrow_and_update().clone() {
                return result;
            }
            done.changed().await.map_err(|_| {
                UploadError::new(
                    "operation_lost",
                    "The native operation ended without a result.",
                )
            })?;
        }
    }
}
#[derive(Default)]
struct Registry {
    sources: HashMap<String, Source>,
    operations: HashMap<String, Arc<Operation>>,
    generations: HashMap<String, u64>,
}
#[derive(Default)]
struct Manager {
    registry: Mutex<Registry>,
    client: OnceLock<Result<Client, String>>,
    reaper_started: AtomicBool,
}
impl Manager {
    fn client(&self) -> Result<Client, String> {
        self.client
            .get_or_init(|| {
                Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .connect_timeout(Duration::from_secs(10))
                    .build()
                    .map_err(|_| "Could not create native transfer transport.".to_string())
            })
            .clone()
    }
    fn generation(&self, host: &str) -> u64 {
        *self
            .registry
            .lock()
            .unwrap()
            .generations
            .get(host)
            .unwrap_or(&0)
    }
    fn insert(&self, source: Source, generation: u64) -> Result<SelectedSource, String> {
        let mut registry = self.registry.lock().unwrap();
        registry.sources.retain(|_, s| Instant::now() < s.expires);
        if *registry
            .generations
            .get(&source.selected.binding.agent_host_id)
            .unwrap_or(&0)
            != generation
        {
            return Err("The selected host authorization changed while choosing the file.".into());
        }
        if registry.sources.len() >= 16 {
            return Err("Close an unused file selection before choosing another.".into());
        }
        let selected = source.selected.clone();
        registry.sources.insert(selected.source_id.clone(), source);
        Ok(selected)
    }
    fn claim(
        &self,
        host: &str,
        source_id: &str,
        sink: Arc<ProgressSink>,
    ) -> Result<(Source, Arc<Operation>), String> {
        let mut registry = self.registry.lock().unwrap();
        let source = registry
            .sources
            .get(source_id)
            .ok_or("The file selection expired or was already used.")?;
        if source.selected.binding.agent_host_id != host {
            return Err("The selected file belongs to another host.".into());
        }
        if Instant::now() >= source.expires {
            registry.sources.remove(source_id);
            return Err("Choose the file again; its selection expired.".into());
        }
        let active = registry
            .operations
            .values()
            .filter(|op| op.done.borrow().is_none())
            .collect::<Vec<_>>();
        if active.len() >= 2 || active.iter().any(|op| op.host == host) {
            return Err("A file transfer is already active for this host, or both desktop transfer slots are busy.".into());
        }
        let source = registry
            .sources
            .remove(source_id)
            .expect("selection checked under this lock");
        let id = source_id.to_string();
        let (cancel, _) = watch::channel(false);
        let (done, _) = watch::channel(None);
        let operation = Arc::new(Operation {
            id: id.clone(),
            host: host.into(),
            cancel,
            done,
            sink,
            finish: Mutex::new(()),
            progress: Mutex::new(Progress {
                operation_id: id.clone(),
                agent_host_id: host.into(),
                phase: "preparing",
                bytes_transferred: 0,
                total_bytes: source.selected.size,
                bytes_per_second: 0.0,
                transport: None,
            }),
            started: Instant::now(),
            last_emit: Mutex::new(Instant::now()),
        });
        let mut finished = registry
            .operations
            .iter()
            .filter(|(_, op)| op.done.borrow().is_some())
            .map(|(id, op)| (id.clone(), op.started))
            .collect::<Vec<_>>();
        finished.sort_by_key(|(_, time)| *time);
        for (id, _) in finished.iter().take(finished.len().saturating_sub(63)) {
            registry.operations.remove(id);
        }
        registry.operations.insert(id, operation.clone());
        Ok((source, operation))
    }
    fn operation(&self, host: &str, id: &str) -> Result<Arc<Operation>, UploadError> {
        self.registry
            .lock()
            .unwrap()
            .operations
            .get(id)
            .filter(|op| op.host == host)
            .cloned()
            .ok_or_else(|| {
                UploadError::new(
                    "operation_unavailable",
                    "Native operation is unavailable for this host.",
                )
            })
    }
    fn cancel_target(&self, host: &str, id: &str) -> Result<Arc<Operation>, UploadError> {
        let mut registry = self.registry.lock().unwrap();
        if let Some(operation) = registry.operations.get(id).filter(|op| op.host == host) {
            return Ok(operation.clone());
        }
        // An abort can beat start's IPC delivery. Retire the selection under
        // the same lock that claims it, so a late start cannot escape cancellation.
        if registry
            .sources
            .get(id)
            .is_some_and(|s| s.selected.binding.agent_host_id == host)
        {
            registry.sources.remove(id);
            return Err(cancelled());
        }
        Err(UploadError::new(
            "operation_unavailable",
            "The native operation or unused selection is unavailable for this host.",
        ))
    }
    fn invalidate(&self, host: &str) {
        let mut registry = self.registry.lock().unwrap();
        *registry.generations.entry(host.into()).or_default() += 1;
        registry
            .sources
            .retain(|_, s| s.selected.binding.agent_host_id != host);
        for op in registry.operations.values().filter(|op| op.host == host) {
            op.signal_cancel();
        }
    }
}
fn manager() -> &'static Manager {
    let manager = MANAGER.get_or_init(Manager::default);
    if !manager.reaper_started.swap(true, Ordering::AcqRel) {
        tauri::async_runtime::spawn(async {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                if let Some(manager) = MANAGER.get() {
                    manager
                        .registry
                        .lock()
                        .unwrap()
                        .sources
                        .retain(|_, source| Instant::now() < source.expires);
                }
            }
        });
    }
    manager
}
pub(crate) struct AuthorizationChange(String);
impl Drop for AuthorizationChange {
    fn drop(&mut self) {
        invalidate_host(&self.0);
    }
}
/// Cancel on both sides of a store mutation: an operation cannot slip into
/// the interval between the first invalidation and the credential write.
pub(crate) fn changing_authorization(host: &str) -> AuthorizationChange {
    invalidate_host(host);
    AuthorizationChange(host.to_string())
}
pub(crate) fn invalidate_host(host: &str) {
    if let Some(manager) = MANAGER.get() {
        manager.invalidate(host);
    }
}
pub(crate) fn invalidate_all() {
    if let Some(manager) = MANAGER.get() {
        let hosts = {
            let registry = manager.registry.lock().unwrap();
            registry
                .sources
                .values()
                .map(|s| s.selected.binding.agent_host_id.clone())
                .chain(registry.operations.values().map(|op| op.host.clone()))
                .collect::<Vec<_>>()
        };
        for host in hosts {
            manager.invalidate(&host);
        }
    }
}
fn main_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("Native file operations are restricted to the main desktop window.".into())
    }
}

#[tauri::command]
pub(crate) async fn pick_upload_source(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    request: PickerRequest,
) -> Result<Option<SelectedSource>, String> {
    main_window(&window)?;
    if request.binding.agent_host_id.is_empty()
        || request.binding.agent_host_id.len() > 256
        || [
            &request.binding.target_server_id,
            &request.binding.operation_id,
            &request.binding.file_id,
        ]
        .iter()
        .any(|value| {
            value
                .as_ref()
                .is_some_and(|value| value.is_empty() || value.len() > 256)
        })
        || request.label.len() > 200
        || request.extensions.len() > 16
        || request.extensions.iter().any(|extension| {
            extension.is_empty()
                || extension.len() > 32
                || !extension
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '*')
        })
    {
        return Err("Invalid native file selection request.".into());
    }
    let generation = manager().generation(&request.binding.agent_host_id);
    let selected = tauri::async_runtime::spawn_blocking(move || {
        let mut picker = app.dialog().file().set_title(request.label);
        let extensions = request
            .extensions
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if !extensions.is_empty() {
            picker = picker.add_filter("Supported files", &extensions);
        }
        picker
            .blocking_pick_file()
            .map(|file| {
                let path = file
                    .into_path()
                    .map_err(|_| "The selected source is not a filesystem file.".to_string())?;
                Source::open(path, request.binding)
            })
            .transpose()
    })
    .await
    .map_err(|_| "The native file picker failed.".to_string())??;
    selected
        .map(|source| manager().insert(source, generation))
        .transpose()
}
#[tauri::command]
pub(crate) fn release_upload_source(
    window: tauri::WebviewWindow,
    agent_host_id: String,
    source_id: String,
) -> Result<(), String> {
    main_window(&window)?;
    let mut registry = manager().registry.lock().unwrap();
    if registry
        .sources
        .get(&source_id)
        .is_some_and(|s| s.selected.binding.agent_host_id != agent_host_id)
    {
        return Err("The file selection belongs to another host.".into());
    }
    registry.sources.remove(&source_id);
    Ok(())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StartRequest {
    agent_host_id: String,
    source_id: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Started {
    operation_id: String,
    agent_host_id: String,
}
#[tauri::command]
pub(crate) async fn start_native_upload(
    window: tauri::WebviewWindow,
    request: StartRequest,
    progress: Channel<Progress>,
) -> Result<Started, UploadError> {
    main_window(&window).map_err(|e| UploadError::new("start_refused", e))?;
    let (source, operation) = manager()
        .claim(
            &request.agent_host_id,
            &request.source_id,
            Arc::new(move |event| {
                let _ = progress.send(event);
            }),
        )
        .map_err(|e| UploadError::new("start_refused", e))?;
    let started = Started {
        operation_id: operation.id.clone(),
        agent_host_id: operation.host.clone(),
    };
    tauri::async_runtime::spawn(async move {
        run_operation(source, operation).await;
    });
    Ok(started)
}
#[tauri::command]
pub(crate) async fn wait_native_upload(
    window: tauri::WebviewWindow,
    agent_host_id: String,
    operation_id: String,
) -> Outcome {
    main_window(&window).map_err(|e| UploadError::new("wrong_window", e))?;
    manager()
        .operation(&agent_host_id, &operation_id)?
        .wait()
        .await
}
#[tauri::command]
pub(crate) async fn cancel_native_upload(
    window: tauri::WebviewWindow,
    agent_host_id: String,
    operation_id: String,
) -> Outcome {
    main_window(&window).map_err(|e| UploadError::new("wrong_window", e))?;
    let operation = manager().cancel_target(&agent_host_id, &operation_id)?;
    operation.signal_cancel();
    operation.wait().await
}

struct Context {
    client: Client,
    record: StoredDesktopCredential,
    generation: u64,
    host: String,
}
impl Context {
    fn load(host: String) -> Result<Self, UploadError> {
        let generation = manager().generation(&host);
        let store = desktop_secret_store().map_err(|_| {
            UploadError::new(
                "authentication",
                "Could not open the desktop credential store.",
            )
        })?;
        let raw = store
            .get(&credential_key(&host))
            .map_err(|_| {
                UploadError::new(
                    "authentication",
                    "Could not resolve this host's credential.",
                )
            })?
            .ok_or_else(|| {
                UploadError::new(
                    "authentication",
                    "This desktop has no credential for the selected host.",
                )
            })?;
        let mut record: StoredDesktopCredential = serde_json::from_str(&raw).map_err(|_| {
            UploadError::new("authentication", "Stored host authorization is invalid.")
        })?;
        record.base_url = canonical_base_url(&record.base_url)
            .map_err(|e| UploadError::new("authentication", e))?;
        if !record.token.starts_with("msc2_") {
            return Err(UploadError::new(
                "authentication",
                "Stored host authorization is invalid.",
            ));
        }
        if record.base_url == LOCAL_AGENT_ORIGIN {
            ensure_current_local_agent_service()
                .map_err(|e| UploadError::new("service_unavailable", e))?;
        }
        if manager().generation(&host) != generation {
            return Err(cancelled());
        }
        Ok(Self {
            client: manager()
                .client()
                .map_err(|e| UploadError::new("network", e))?,
            record,
            generation,
            host,
        })
    }
    fn request(&self, method: Method, path: &str) -> Result<reqwest::RequestBuilder, UploadError> {
        if manager().generation(&self.host) != self.generation {
            return Err(cancelled());
        }
        self.original_request(method, path)
    }
    fn original_request(
        &self,
        method: Method,
        path: &str,
    ) -> Result<reqwest::RequestBuilder, UploadError> {
        let url = relative_request_url(&self.record.base_url, path)
            .map_err(|e| UploadError::new("invalid_route", e))?;
        let metadata = method == Method::GET || method == Method::POST;
        let request = self
            .client
            .request(method, url)
            .bearer_auth(&self.record.token)
            .header("X-MSC-Api-Version", "1.0");
        Ok(if metadata {
            request.timeout(Duration::from_secs(30))
        } else {
            request
        })
    }
    async fn json<T: serde::de::DeserializeOwned>(
        &self,
        response: Response,
    ) -> Result<T, UploadError> {
        let status = response.status();
        let bytes = bounded_response(response).await?;
        if !status.is_success() {
            if status.as_u16() == 401 {
                invalidate_host(&self.host);
            }
            let message = serde_json::from_slice::<serde_json::Value>(&bytes)
                .ok()
                .and_then(|v| {
                    v.get("message")
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_else(|| {
                    format!("The host refused the transfer (HTTP {}).", status.as_u16())
                });
            return Err(UploadError::new(
                if matches!(status.as_u16(), 401 | 403) {
                    "authentication"
                } else {
                    "agent_refused"
                },
                message.replace(&self.record.token, "[redacted]"),
            ));
        }
        serde_json::from_slice(&bytes).map_err(|_| {
            UploadError::new(
                "invalid_response",
                "The host returned invalid transfer metadata.",
            )
        })
    }
}
async fn bounded_response(response: Response) -> Result<Vec<u8>, UploadError> {
    tokio::time::timeout(Duration::from_secs(30), read_metadata(response))
        .await
        .map_err(|_| network_error())?
}
async fn read_metadata(response: Response) -> Result<Vec<u8>, UploadError> {
    if response
        .content_length()
        .is_some_and(|n| n > METADATA_LIMIT as u64)
    {
        return Err(UploadError::new(
            "invalid_response",
            "Transfer metadata exceeded its size limit.",
        ));
    }
    let mut stream = response.bytes_stream();
    let mut result = Vec::new();
    while let Some(bytes) = stream.next().await {
        let bytes = bytes.map_err(|_| network_error())?;
        if bytes.len() > METADATA_LIMIT - result.len() {
            return Err(UploadError::new(
                "invalid_response",
                "Transfer metadata exceeded its size limit.",
            ));
        }
        result.extend_from_slice(&bytes);
    }
    Ok(result)
}
fn network_error() -> UploadError {
    UploadError::new(
        "network",
        "The native connection was interrupted. Retry with a new file selection.",
    )
}
fn cancelled() -> UploadError {
    UploadError::new(
        "cancelled",
        "The transfer was cancelled or its host authorization changed.",
    )
}
async fn cancellable<T>(
    operation: &Operation,
    future: impl std::future::Future<Output = Result<T, UploadError>>,
) -> Result<T, UploadError> {
    let mut cancel = operation.cancel.subscribe();
    if *cancel.borrow() {
        return Err(cancelled());
    }
    tokio::select! {
        biased;
        _ = cancel.changed() => Err(cancelled()),
        result = future => if result.is_ok() && *cancel.borrow() { Err(cancelled()) } else { result },
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Capabilities {
    api_major: u32,
    permissions: Vec<String>,
    native_transfers: Option<NativeProtocol>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeProtocol {
    protocol: u32,
    stream_upload: bool,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Slot {
    staged_upload_id: String,
    upload_path: String,
    max_bytes: u64,
    max_chunk_bytes: Option<usize>,
    protocol: Option<u32>,
    stream_path: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Completion {
    staged_upload_id: String,
    received_bytes: u64,
    sha256: String,
}
#[derive(Default)]
struct Attempt {
    admission_started: bool,
    slot: Option<Slot>,
    transport: Option<Transport>,
}

async fn validate_source(source: Arc<Source>) -> Result<(), UploadError> {
    tauri::async_runtime::spawn_blocking(move || source.validate())
        .await
        .map_err(|_| UploadError::new("source_changed", "Could not inspect the selected source."))?
        .map_err(|e| UploadError::new("source_changed", e))
}
async fn perform(
    context: &Context,
    source: Arc<Source>,
    operation: Arc<Operation>,
    attempt: &mut Attempt,
) -> Outcome {
    validate_source(source.clone()).await?;
    let caps: Capabilities = context
        .json(
            context
                .request(Method::GET, "/v1/capabilities")?
                .send()
                .await
                .map_err(|_| network_error())?,
        )
        .await?;
    if caps.api_major != 1 {
        return Err(UploadError::new(
            "incompatible_agent",
            "The host's API version does not support this client.",
        ));
    }
    if !caps
        .permissions
        .iter()
        .any(|p| source.selected.binding.purpose.allows_permission(p))
    {
        return Err(UploadError::new(
            "permission_denied",
            "This host credential cannot upload files for that purpose.",
        ));
    }
    let transport = match caps.native_transfers {
        Some(protocol) if protocol.protocol == 1 && protocol.stream_upload => Transport::Stream,
        Some(_) => {
            return Err(UploadError::new(
                "incompatible_agent",
                "Native uploads are unavailable or use an unsupported protocol on this host.",
            ))
        }
        None if source.selected.binding.purpose.chunks() => Transport::LegacyChunks,
        None => Transport::LegacyBufferedAgent,
    };
    attempt.transport = Some(transport);
    operation.progress("preparing", 0, Some(transport), true);
    let binding = &source.selected.binding;
    let metadata = serde_json::json!({"purpose": binding.purpose, "fileName": source.selected.name,
        "expectedBytes": source.selected.size, "operationId": binding.operation_id, "fileId": binding.file_id});
    let mut metadata = metadata;
    if matches!(transport, Transport::Stream) {
        metadata["targetServerId"] =
            serde_json::to_value(&binding.target_server_id).expect("optional string serializes");
    }
    let begin_path = if matches!(transport, Transport::Stream) {
        "/v1/staged-uploads/stream"
    } else {
        "/v1/staged-uploads"
    };
    attempt.admission_started = true;
    let admission_response = context
        .request(Method::POST, begin_path)?
        .json(&metadata)
        .send()
        .await
        .map_err(|_| network_error())?;
    let admission_status = admission_response.status();
    if admission_status.is_client_error() {
        attempt.admission_started = false;
    }
    let slot: Slot = context.json(admission_response).await?;
    if Uuid::parse_str(&slot.staged_upload_id).is_err() {
        return Err(UploadError::new(
            "invalid_response",
            "The host returned an invalid staging identity.",
        ));
    }
    attempt.slot = Some(slot.clone());
    if admission_status.as_u16()
        != if matches!(transport, Transport::Stream) {
            201
        } else {
            200
        }
    {
        return Err(UploadError::new(
            "invalid_admission",
            "The host returned an unsupported staging admission response.",
        ));
    }

    let expected_path = format!(
        "/v1/staged-uploads/{}{}",
        slot.staged_upload_id,
        if matches!(transport, Transport::Stream) {
            "/stream"
        } else {
            ""
        }
    );
    if slot.upload_path != expected_path
        || source.selected.size > slot.max_bytes
        || (matches!(transport, Transport::Stream)
            && (slot.protocol != Some(1)
                || slot.stream_path.as_deref() != Some(expected_path.as_str())))
    {
        return Err(UploadError::new(
            "invalid_admission",
            "The host's staging route or byte ceiling is incompatible with this file.",
        ));
    }
    if matches!(transport, Transport::LegacyChunks)
        && binding.purpose != Purpose::ModpackArchive
        && slot.max_chunk_bytes.is_none()
    {
        return Err(UploadError::new("incompatible_agent", "This older agent cannot confirm chunked world support. Update the agent before importing this file."));
    }
    let hash = Arc::new(Mutex::new(Sha256::new()));
    let mut file = tokio::fs::File::from_std(source.file.try_clone().map_err(|_| {
        UploadError::new(
            "source_unavailable",
            "Could not retain the selected file handle.",
        )
    })?);
    file.set_max_buf_size(BLOCK);
    let completion = if matches!(transport, Transport::LegacyChunks) {
        let limit = slot
            .max_chunk_bytes
            .unwrap_or(2 * 1024 * 1024)
            .min(LEGACY_CHUNK);
        if limit == 0 {
            return Err(UploadError::new(
                "invalid_response",
                "The host returned an invalid chunk ceiling.",
            ));
        }
        let mut offset = 0;
        loop {
            let count = (source.selected.size - offset).min(limit as u64) as usize;
            let mut bytes = vec![0; count];
            for block in bytes.chunks_mut(BLOCK) {
                tokio::time::timeout(Duration::from_secs(120), file.read_exact(block))
                    .await
                    .map_err(|_| {
                        UploadError::new("source_idle", "Reading the selected file timed out.")
                    })?
                    .map_err(|_| {
                        UploadError::new(
                            "source_changed",
                            "Could not read the complete selected file.",
                        )
                    })?;
            }
            hash.lock().unwrap().update(&bytes);
            let next = offset + count as u64;
            let final_chunk = next == source.selected.size;
            let response = context
                .request(
                    Method::PUT,
                    &format!("{expected_path}/chunks?offset={offset}&complete={final_chunk}"),
                )?
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .body(bytes)
                .send()
                .await
                .map_err(|_| network_error())?;
            if final_chunk {
                operation.progress("verifying", next, Some(transport), true);
                break context.json::<Completion>(response).await?;
            }
            if response.status().as_u16() != 204 {
                let _: serde_json::Value = context.json(response).await?;
                return Err(UploadError::new(
                    "invalid_response",
                    "The host did not acknowledge the file chunk.",
                ));
            }
            // Drain bounded metadata to allow the same pooled connection to be reused.
            bounded_response(response).await?;
            offset = next;
            operation.progress("transferring", offset, Some(transport), false);
        }
    } else {
        let stream_operation = operation.clone();
        let stream_hash = hash.clone();
        let total = source.selected.size;
        let body = stream::try_unfold((file, 0_u64), move |(mut file, offset)| {
            let operation = stream_operation.clone();
            let hash = stream_hash.clone();
            async move {
                if *operation.cancel.borrow() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "native transfer cancelled",
                    ));
                }
                if offset == total {
                    return Ok::<_, std::io::Error>(None);
                }
                let mut bytes = vec![0; (total - offset).min(BLOCK as u64) as usize];
                tokio::time::timeout(Duration::from_secs(120), file.read_exact(&mut bytes))
                    .await
                    .map_err(|_| {
                        std::io::Error::new(std::io::ErrorKind::TimedOut, "source read stalled")
                    })??;
                if *operation.cancel.borrow() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "native transfer cancelled",
                    ));
                }
                hash.lock().unwrap().update(&bytes);
                let next = offset + bytes.len() as u64;
                operation.progress(
                    if next == total {
                        "verifying"
                    } else {
                        "transferring"
                    },
                    next,
                    Some(transport),
                    next == total,
                );
                Ok(Some((bytes, (file, next))))
            }
        });
        let response = context
            .request(Method::PUT, &expected_path)?
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(header::CONTENT_LENGTH, source.selected.size)
            .body(reqwest::Body::wrap_stream(body))
            .send()
            .await
            .map_err(|_| network_error())?;
        context.json::<Completion>(response).await?
    };
    validate_source(source.clone()).await?;
    let digest = format!("{:x}", hash.lock().unwrap().clone().finalize());
    if completion.staged_upload_id != slot.staged_upload_id
        || completion.received_bytes != source.selected.size
        || completion.sha256.to_ascii_lowercase() != digest
    {
        return Err(UploadError::new(
            "integrity_mismatch",
            "The host's byte count or SHA-256 did not match the selected file.",
        ));
    }
    Ok(UploadResult {
        operation_id: operation.id.clone(),
        agent_host_id: operation.host.clone(),
        staged_upload_id: slot.staged_upload_id,
        received_bytes: completion.received_bytes,
        sha256: digest,
        transport,
    })
}
async fn remove_staging(context: &Context, attempt: &Attempt) -> bool {
    let Some(slot) = &attempt.slot else {
        return !attempt.admission_started;
    };
    // Cleanup uses the original origin/token even after a saved route changes.
    let request = match context.original_request(
        Method::DELETE,
        &format!("/v1/staged-uploads/{}", slot.staged_upload_id),
    ) {
        Ok(request) => request,
        Err(_) => return false,
    };
    match request.timeout(Duration::from_secs(20)).send().await {
        Ok(response) if response.status().as_u16() == 204 => {
            bounded_response(response).await.is_ok()
        }
        _ => false,
    }
}
async fn run_operation(source: Source, operation: Arc<Operation>) {
    operation.progress("preparing", 0, None, true);
    let host = operation.host.clone();
    let context = cancellable(&operation, async {
        tauri::async_runtime::spawn_blocking(move || Context::load(host))
            .await
            .map_err(|_| {
                UploadError::new(
                    "authentication",
                    "Could not prepare the selected host's transport.",
                )
            })?
    })
    .await;
    let mut attempt = Attempt::default();
    let (outcome, context) = match context {
        Ok(context) => {
            let outcome = std::panic::AssertUnwindSafe(cancellable(
                &operation,
                perform(&context, Arc::new(source), operation.clone(), &mut attempt),
            ))
            .catch_unwind()
            .await
            .unwrap_or_else(|_| {
                Err(UploadError::new(
                    "native_worker_failed",
                    "The native transfer stopped unexpectedly.",
                ))
            });
            (outcome, Some(context))
        }
        Err(error) => (Err(error), None),
    };
    let mut error = match outcome {
        Ok(result) => {
            let finish = operation.finish.lock().unwrap();
            if !*operation.cancel.borrow() {
                operation.done.send_replace(Some(Ok(result.clone())));
                drop(finish);
                operation.progress(
                    "complete",
                    result.received_bytes,
                    Some(result.transport),
                    true,
                );
                return;
            }
            cancelled()
        }
        Err(error) => error,
    };
    operation.signal_cancel(); // Stop any body task still owned by the HTTP connection.
    let progress = operation.progress.lock().unwrap().clone();
    if error.code == "cancelled" {
        operation.progress(
            "cancelling",
            progress.bytes_transferred,
            attempt.transport,
            true,
        );
    }
    let removed = match context {
        Some(context) => remove_staging(&context, &attempt).await,
        None => true,
    };
    error.cleanup = if removed {
        if attempt.admission_started {
            "confirmed"
        } else {
            "not-admitted"
        }
    } else {
        "unconfirmed"
    };
    if !removed {
        error.message.push_str(" The host could not confirm staging cleanup; reconnect to inspect it. No automatic retry was made.");
    }
    error.agent_host_id = Some(operation.host.clone().into_boxed_str());
    error.operation_id = Some(operation.id.clone().into_boxed_str());
    if !removed {
        error.staged_upload_id = attempt
            .slot
            .as_ref()
            .map(|slot| slot.staged_upload_id.clone().into_boxed_str());
    }
    let phase = if error.code == "cancelled" && removed {
        "cancelled"
    } else {
        "failed"
    };
    operation.progress(phase, progress.bytes_transferred, attempt.transport, true);
    let _finish = operation.finish.lock().unwrap();
    operation.done.send_replace(Some(Err(error)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("msc-native-transfer-{}", Uuid::new_v4()));
            std::fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn source(&self, name: &str, host: &str) -> Source {
            let path = self.0.join(name);
            std::fs::write(&path, b"abc").unwrap();
            Source::open(
                path,
                Binding {
                    agent_host_id: host.into(),
                    purpose: Purpose::ModpackArchive,
                    target_server_id: None,
                    operation_id: None,
                    file_id: None,
                },
            )
            .unwrap()
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn sink() -> Arc<ProgressSink> {
        Arc::new(|_| {})
    }

    // Existing webview-chunk tests never exercise native picker capabilities.
    #[test]
    fn native_selection_is_single_use_host_bound_and_concurrency_bounded() {
        let directory = Directory::new();
        let manager = Manager::default();
        let selected = manager
            .insert(directory.source("world.zip", "alpha"), 0)
            .unwrap();
        assert!(manager.claim("beta", &selected.source_id, sink()).is_err());
        let (mut source, alpha) = manager.claim("alpha", &selected.source_id, sink()).unwrap();
        let mut bytes = Vec::new();
        source.file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"abc");
        assert_eq!(alpha.id, selected.source_id);
        assert!(manager.claim("alpha", &selected.source_id, sink()).is_err());
        assert!(!*alpha.cancel.borrow());
        let second_alpha = manager
            .insert(directory.source("second.zip", "alpha"), 0)
            .unwrap();
        assert!(manager
            .claim("alpha", &second_alpha.source_id, sink())
            .is_err());
        let beta = manager
            .insert(directory.source("pack.zip", "beta"), 0)
            .unwrap();
        assert!(manager.claim("beta", &beta.source_id, sink()).is_ok());
        let gamma = manager
            .insert(directory.source("third.zip", "gamma"), 0)
            .unwrap();
        assert!(manager.claim("gamma", &gamma.source_id, sink()).is_err());
    }

    #[test]
    fn native_source_substitution_and_edits_fail_without_redirecting_the_handle() {
        let directory = Directory::new();
        let mut source = directory.source("world.zip", "alpha");
        std::fs::rename(&source.path, directory.0.join("original.zip")).unwrap();
        std::fs::write(&source.path, b"xyz").unwrap();
        assert!(source.validate().is_err());
        let mut bytes = Vec::new();
        source.file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"abc");
        let edited = directory.source("pack.zip", "alpha");
        std::fs::write(&edited.path, b"longer").unwrap();
        assert!(edited.validate().is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                directory.0.join("original.zip"),
                directory.0.join("linked.zip"),
            )
            .unwrap();
            assert!(Source::open(
                directory.0.join("linked.zip"),
                source.selected.binding.clone()
            )
            .is_err());
        }
    }

    #[test]
    fn native_abort_before_start_and_host_changes_retire_only_their_own_work() {
        let directory = Directory::new();
        let manager = Manager::default();
        let alpha = manager
            .insert(directory.source("world.zip", "alpha"), 0)
            .unwrap();
        assert!(manager.cancel_target("beta", &alpha.source_id).is_err());
        let stopped = manager
            .cancel_target("alpha", &alpha.source_id)
            .err()
            .unwrap();
        assert_eq!(stopped.code, "cancelled");
        assert_eq!(stopped.cleanup, "not-admitted");
        assert!(manager.claim("alpha", &alpha.source_id, sink()).is_err());
        let beta = manager
            .insert(directory.source("pack.zip", "beta"), 0)
            .unwrap();
        let (_, operation) = manager.claim("beta", &beta.source_id, sink()).unwrap();
        manager.invalidate("alpha");
        assert!(!*operation.cancel.borrow());
        manager.invalidate("beta");
        assert!(*operation.cancel.borrow());
        assert!(manager
            .insert(directory.source("late.zip", "beta"), 0)
            .is_err());
        assert_eq!(std::fs::read(directory.0.join("pack.zip")).unwrap(), b"abc");
    }
}
