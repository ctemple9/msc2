//! Native stream staging is isolated from legacy writers. Only verified files
//! enter the importer store; cancellation and redemption compete atomically.
use super::*;
use axum::body::Body;
use axum::http::{HeaderMap, header};
use futures_util::StreamExt;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;

const ADMITTED: u8 = 0;
const RECEIVING: u8 = 1;
const VERIFYING: u8 = 2;
const READY: u8 = 3;
const REDEEMED: u8 = 4;
const CANCELLING: u8 = 5;
const CANCELLED: u8 = 6;
const FAILED: u8 = 7;
const CLEANUP_FAILED: u8 = 8;
// A streamed folder needs room for its TAR and extracted contents. Actual
// reservations still require free disk space before admission.
const RESERVED_BYTES: u64 = 2 * msc_infrastructure::source_tree::MAX_SERVER_SOURCE_BYTES;
const DISK_MARGIN: u64 = 1024 * 1024 * 1024;
const BLOCK_BYTES: usize = 256 * 1024;
const MAX_FRAME_BYTES: usize = 1024 * 1024;

use msc_api::dto::{
    NativeUploadBeginRequestDto as Admission, NativeUploadBeginResultDto, NativeUploadStatusDto,
};

#[derive(serde::Serialize, serde::Deserialize)]
struct Ledger {
    owner: String,
    expected_bytes: u64,
    #[serde(default)]
    reserved_bytes: Option<u64>,
    expires_at: u64,
}

pub(crate) struct Session {
    owner: String,
    reservation: u64,
    adapter: Arc<tokio::sync::Mutex<()>>,
    target: Option<String>,
    upload: StagedUpload,
    expected_sha256: Option<String>,
    pending_identity: Option<PendingIdentity>,
    inspections: AtomicUsize,
    consumer_done: AtomicBool,
    phase: AtomicU8,
    received: AtomicU64,
    expiry: AtomicU64,
    reserved: AtomicBool,
    cancel: watch::Sender<bool>,
    done: watch::Sender<bool>,
    cleanup_error: Mutex<Option<String>>,
    // Rechecks the original target and pending file without trusting request JSON.
    binding_valid: Box<dyn Fn(&AuthenticatedCredential) -> bool + Send + Sync>,
}

impl Session {
    pub(crate) fn can_handoff(
        &self,
        credential: &AuthenticatedCredential,
        state: &ComponentsRoutesState,
    ) -> bool {
        self.phase.load(Ordering::Acquire) == ADMITTED
            && now_unix() <= self.expiry.load(Ordering::Acquire)
            && self.authorized(credential)
            && self.pending_valid(state)
    }
    pub(crate) fn local_progress(&self) -> (u64, u64) {
        (
            self.received.load(Ordering::Acquire),
            self.upload.expected_bytes.unwrap_or(0),
        )
    }

    pub(crate) fn authorized(&self, credential: &AuthenticatedCredential) -> bool {
        self.owner == credential.credential_id && (self.binding_valid)(credential)
    }
    fn visible(&self) -> bool {
        self.phase.load(Ordering::Acquire) == READY
            && now_unix() <= self.expiry.load(Ordering::Acquire)
            && crate::auth::INITIATING_CREDENTIAL
                .try_with(|credential| self.authorized(credential))
                .unwrap_or(false)
    }
    fn pending_valid(&self, state: &ComponentsRoutesState) -> bool {
        self.pending_identity.as_ref().is_none_or(|identity| {
            self.upload
                .operation_id
                .as_ref()
                .zip(self.upload.file_id.as_ref())
                .and_then(|(op, file)| pending_identity(state, op, file))
                .as_ref()
                == Some(identity)
        })
    }
    fn touch(&self) {
        self.expiry
            .store(now_unix() + STAGING_TTL_SECONDS, Ordering::Release);
    }
}

/// The final consumer releases staging even on an early return or worker panic.
/// Drop only signals the reaper; it never performs disk I/O under a map lock.
pub(crate) struct ConsumedInput(Arc<Session>);
impl std::fmt::Debug for ConsumedInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ConsumedInput")
    }
}
impl Drop for ConsumedInput {
    fn drop(&mut self) {
        self.0.consumer_done.store(true, Ordering::Release);
    }
}

/// Enforce native ownership at the shared access boundary, including server
/// creation. Legacy slots keep their previous semantics. Request auth already
/// scopes INITIATING_CREDENTIAL around every management handler.
#[derive(Default)]
pub(crate) struct UploadStore {
    entries: HashMap<String, StagedUpload>,
    native: HashMap<String, Arc<Session>>,
}
impl UploadStore {
    pub(crate) fn insert(&mut self, id: String, upload: StagedUpload) -> Option<StagedUpload> {
        if self.native.contains_key(&id) {
            return None;
        }
        self.entries.insert(id, upload)
    }
    pub(crate) fn get(&self, id: &str) -> Option<&StagedUpload> {
        if let Some(session) = self.native.get(id) {
            if !session.visible() {
                return None;
            }
            session.touch();
        }
        self.entries.get(id)
    }
    pub(crate) fn get_mut(&mut self, id: &str) -> Option<&mut StagedUpload> {
        // Existing PUT/chunk handlers must never append to a verified native file.
        if self.native.contains_key(id) {
            return None;
        }
        self.entries.get_mut(id)
    }
    pub(crate) fn remove(&mut self, id: &str) -> Option<StagedUpload> {
        let session = self.native.get(id).cloned();
        if let Some(session) = &session
            && (!session.visible()
                || session
                    .phase
                    .compare_exchange(READY, REDEEMED, Ordering::AcqRel, Ordering::Acquire)
                    .is_err())
        {
            return None;
        }
        let mut entry = self.entries.remove(id)?;
        entry.native_guard = session.map(|session| Arc::new(ConsumedInput(session)));
        Some(entry)
    }
    #[cfg(test)]
    pub(crate) fn contains_key(&self, id: &str) -> bool {
        self.get(id).is_some()
    }
}

type AvailableSpace = dyn Fn(&Path) -> std::io::Result<u64> + Send + Sync;

#[derive(Clone)]
pub(crate) struct NativeStore {
    pub(crate) local_handoff: Arc<AtomicBool>,
    sessions: Arc<Mutex<HashMap<String, Arc<Session>>>>,
    admission: Arc<tokio::sync::Mutex<()>>,
    initialized: Arc<AtomicBool>,
    recovery_error: Arc<Mutex<Option<String>>>,
    root: Arc<Mutex<Option<PathBuf>>>,
    space: Arc<AvailableSpace>,
}
impl Default for NativeStore {
    fn default() -> Self {
        Self {
            local_handoff: Default::default(),
            sessions: Default::default(),
            admission: Default::default(),
            initialized: Default::default(),
            recovery_error: Default::default(),
            root: Default::default(),
            space: Arc::new(|path| fs2::available_space(path)),
        }
    }
}
impl NativeStore {
    pub(crate) async fn ensure_root(&self, root: PathBuf) -> bool {
        let _admission = self.admission.lock().await;
        if self.root.lock().unwrap().as_ref() != Some(&root) {
            if self
                .sessions
                .lock()
                .unwrap()
                .values()
                .any(|s| s.reserved.load(Ordering::Acquire))
            {
                return false;
            }
            let store = self.clone();
            let new_root = root.clone();
            if tokio::task::spawn_blocking(move || store.initialize(&new_root))
                .await
                .is_err()
            {
                return false;
            }
        }
        self.available() && self.root.lock().unwrap().as_ref() == Some(&root)
    }
    pub(crate) fn contains(&self, id: &str) -> bool {
        self.sessions.lock().unwrap().contains_key(id)
    }
    pub(crate) fn inspection_lease(
        &self,
        id: &str,
        credential: &AuthenticatedCredential,
    ) -> Option<InspectionLease> {
        let session = self.session(id, credential)?;
        let adapter = session.adapter.try_lock().ok()?;
        if !session.authorized(credential) || session.phase.load(Ordering::Acquire) != READY {
            return None;
        }
        session.inspections.fetch_add(1, Ordering::AcqRel);
        session.touch();
        drop(adapter);
        Some(InspectionLease(session))
    }
    pub(crate) fn creation_matches(&self, id: &str) -> bool {
        self.sessions
            .lock()
            .unwrap()
            .get(id)
            .is_none_or(|session| session.target.is_none())
    }
    pub(crate) fn target_matches(&self, id: &str, target: &str) -> bool {
        self.sessions
            .lock()
            .unwrap()
            .get(id)
            .is_none_or(|s| s.target.as_deref() == Some(target))
    }
    pub(crate) fn pending_matches(&self, id: &str, file: &modpacks::UnresolvedModpackFile) -> bool {
        self.sessions.lock().unwrap().get(id).is_none_or(|session| {
            session
                .pending_identity
                .as_ref()
                .is_none_or(|identity| identity == &PendingIdentity::for_file(file))
        })
    }
    pub(crate) fn available(&self) -> bool {
        self.initialized.load(Ordering::Acquire) && self.recovery_error.lock().unwrap().is_none()
    }
    pub(crate) fn session(
        &self,
        id: &str,
        credential: &AuthenticatedCredential,
    ) -> Option<Arc<Session>> {
        self.sessions
            .lock()
            .unwrap()
            .get(id)
            .filter(|s| s.owner == credential.credential_id)
            .cloned()
    }
    /// No bytes survive restart as ready. The on-disk ledger retains ownership
    /// and reservation if cleanup fails; unknown entries disable admission.
    pub(crate) fn initialize(&self, servers_root: &Path) {
        let root = staging_root(servers_root).join("native");
        let result =
            std::fs::create_dir_all(servers_root).and_then(|_| recover(&root, &self.sessions));
        *self.recovery_error.lock().unwrap() = result.err().map(|e| e.to_string());
        *self.root.lock().unwrap() = Some(servers_root.to_path_buf());
        self.initialized.store(true, Ordering::Release);
    }
}

#[cfg(target_os = "windows")]
fn private_dir(path: &Path) -> std::io::Result<()> {
    msc_platform_windows::staging::private_directory(path)
}

#[cfg(not(target_os = "windows"))]
fn private_dir(path: &Path) -> std::io::Result<()> {
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(std::io::Error::other(
                "staging directory is not a private directory",
            ));
        }
    } else {
        std::fs::create_dir(path)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn recover(root: &Path, sessions: &Mutex<HashMap<String, Arc<Session>>>) -> std::io::Result<()> {
    private_dir(
        root.parent()
            .ok_or_else(|| std::io::Error::other("missing staging parent"))?,
    )?;
    private_dir(root)?;
    for (count, item) in std::fs::read_dir(root)?.enumerate() {
        if count >= 8 {
            return Err(std::io::Error::other("too many recovery entries"));
        }
        let item = item?;
        let id = item.file_name().to_string_lossy().to_string();
        if Uuid::parse_str(&id).is_err() || !item.file_type()?.is_dir() {
            return Err(std::io::Error::other("unrecognized native staging entry"));
        }
        let ledger_path = item.path().join("owner.json");
        let meta = match std::fs::symlink_metadata(&ledger_path) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::remove_dir_all(item.path())?;
                continue;
            }
            Err(error) => return Err(error),
        };
        if !meta.is_file() || meta.len() > 4096 {
            return Err(std::io::Error::other("invalid cleanup ledger"));
        }
        let ledger: Ledger = match serde_json::from_slice(&std::fs::read(ledger_path)?) {
            Ok(ledger) => ledger,
            Err(_) => {
                std::fs::remove_dir_all(item.path())?;
                continue;
            }
        };
        if ledger.expected_bytes == 0
            || ledger.expected_bytes > RESERVED_BYTES
            || ledger
                .reserved_bytes
                .is_some_and(|n| n < ledger.expected_bytes || n > RESERVED_BYTES)
        {
            return Err(std::io::Error::other("invalid cleanup reservation"));
        }
        let cleanup = std::fs::remove_dir_all(item.path());
        if let Err(error) = cleanup {
            let (cancel, _) = watch::channel(true);
            let (done, _) = watch::channel(true);
            let session = Arc::new(Session {
                owner: ledger.owner,
                reservation: ledger.reserved_bytes.unwrap_or(ledger.expected_bytes),
                adapter: Default::default(),
                target: None,
                upload: StagedUpload {
                    native_guard: None,
                    purpose: StagedUploadPurposeDto::WorldImport,
                    file_name: None,
                    operation_id: None,
                    file_id: None,
                    expires_at_unix: ledger.expires_at,
                    max_bytes: ledger.expected_bytes,
                    expected_bytes: Some(ledger.expected_bytes),
                    received_bytes: 0,
                    complete: false,
                    path: item.path().join("payload.bin"),
                },
                expected_sha256: None,
                pending_identity: None,
                inspections: AtomicUsize::new(0),
                consumer_done: AtomicBool::new(false),
                phase: AtomicU8::new(CLEANUP_FAILED),
                received: AtomicU64::new(0),
                expiry: AtomicU64::new(ledger.expires_at),
                reserved: AtomicBool::new(true),
                cancel,
                done,
                cleanup_error: Mutex::new(Some(error.to_string())),
                binding_valid: Box::new(|_| false),
            });
            sessions.lock().unwrap().insert(id, session);
        }
    }
    Ok(())
}

fn safe_basename(name: &str) -> bool {
    if name.is_empty()
        || name.len() > 255
        || matches!(name, "." | "..")
        || name.ends_with(['.', ' '])
        || name.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*')
        })
    {
        return false;
    }
    // These basenames address Windows devices even with an ordinary extension.
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                let mut chars = suffix.chars();
                chars.next().is_some_and(|c| c.is_numeric()) && chars.next().is_none()
            })
        })
}

fn file_size(path: &Path) -> std::io::Result<u64> {
    match std::fs::metadata(path) {
        Ok(meta) => Ok(meta.len()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error),
    }
}

fn purpose_allowed(purpose: StagedUploadPurposeDto, credential: &AuthenticatedCredential) -> bool {
    let allowed: &[PermissionCategoryDto] = match purpose {
        StagedUploadPurposeDto::WorldImport
        | StagedUploadPurposeDto::ActiveWorldReplace
        | StagedUploadPurposeDto::WorldThumbnail => &[PermissionCategoryDto::Worlds],
        // This staging purpose serves both add-ons and world packs. The consumer
        // still enforces its own permission before installing anything.
        StagedUploadPurposeDto::AddonLocalFile => {
            &[PermissionCategoryDto::Addons, PermissionCategoryDto::Worlds]
        }
        StagedUploadPurposeDto::RawServerArchive
        | StagedUploadPurposeDto::RawServerFolder
        | StagedUploadPurposeDto::MscTransferArchive => &[PermissionCategoryDto::Fleet],
        _ => &[PermissionCategoryDto::Addons],
    };
    credential
        .permissions
        .contains(&PermissionCategoryDto::Admin)
        || allowed
            .iter()
            .any(|permission| credential.permissions.contains(permission))
}

#[derive(Clone, PartialEq, Eq)]
struct PendingIdentity {
    project: i64,
    file: i64,
    name: String,
    bytes: u64,
    destination: PathBuf,
    sha512: Option<String>,
}
impl PendingIdentity {
    fn for_file(file: &modpacks::UnresolvedModpackFile) -> Self {
        Self {
            project: file.pending.project_id,
            file: file.pending.file_id,
            name: file.pending.expected_file_name.clone(),
            bytes: file.pending.expected_byte_size,
            destination: file.pending.dest.clone(),
            sha512: file.expected_sha512.clone(),
        }
    }
}
fn pending_identity(
    state: &ComponentsRoutesState,
    operation: &str,
    file: &str,
) -> Option<PendingIdentity> {
    let pending = state.pending_modpack_imports.entries.lock().unwrap();
    let import = pending.get(operation)?;
    Some(PendingIdentity::for_file(
        import.remaining_files.iter().find(|f| f.file_id == file)?,
    ))
}

/// An inspector keeps its native input leased until it finishes reading it.
pub(crate) struct InspectionLease(Arc<Session>);
impl Drop for InspectionLease {
    fn drop(&mut self) {
        self.0.inspections.fetch_sub(1, Ordering::AcqRel);
        self.0.touch();
    }
}

pub(crate) async fn admit(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    body: Result<Json<Admission>, JsonRejection>,
) -> Response {
    let Json(body) = match body {
        Ok(body) => body,
        Err(_) => {
            return invalid_body("invalid_json", "Native upload metadata must be valid JSON.");
        }
    };
    tokio::spawn(async move { admit_inner(state, credential, body).await })
        .await
        .unwrap_or_else(|_| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "admission_failed",
                "Native staging admission failed.",
            )
        })
}

async fn admit_inner(
    state: ComponentsRoutesState,
    credential: AuthenticatedCredential,
    body: Admission,
) -> Response {
    let purpose = body.purpose;
    if !purpose_allowed(purpose, &credential) {
        return error_response(
            StatusCode::FORBIDDEN,
            "forbidden",
            "This token is not allowed to perform that action.",
        );
    }
    let servers_root = state.lifecycle.servers_root();
    if !state.staging.native.ensure_root(servers_root.clone()).await {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "staging_unavailable",
            "Native staging recovery is incomplete.",
        );
    }
    let max = match body.purpose {
        StagedUploadPurposeDto::RawServerArchive
        | StagedUploadPurposeDto::RawServerFolder
        | StagedUploadPurposeDto::MscTransferArchive => {
            msc_infrastructure::source_tree::MAX_SERVER_SOURCE_BYTES
        }
        StagedUploadPurposeDto::AddonLocalFile => MAX_LOCAL_ADDON_UPLOAD_BYTES,
        StagedUploadPurposeDto::CurseforgeManualFile
        | StagedUploadPurposeDto::ModpackUnresolvedFile => {
            let pending = state.pending_modpack_imports.entries.lock().unwrap();
            let file = body
                .operation_id
                .as_ref()
                .and_then(|id| pending.get(id))
                .and_then(|p| {
                    p.remaining_files
                        .iter()
                        .find(|f| Some(&f.file_id) == body.file_id.as_ref())
                });
            let Some(file) = file else {
                return invalid_body("invalid_pending_file", "Select an existing pending file.");
            };
            if body.expected_bytes != file.pending.expected_byte_size {
                return invalid_body(
                    "invalid_expected_bytes",
                    "Length must match the pending file exactly.",
                );
            }
            file.pending.expected_byte_size
        }
        _ => MAX_STAGED_UPLOAD_BYTES,
    };
    if body.purpose == StagedUploadPurposeDto::RawServerFolder && body.expected_sha256.is_some() {
        return invalid_body(
            "invalid_digest",
            "Folder handoff verifies its snapshot and produces a copy digest; expectedSha256 is only supported for file archives.",
        );
    }
    if body.expected_bytes == 0 {
        return invalid_body("invalid_expected_bytes", "expectedBytes must be positive.");
    }
    if body.expected_bytes > max {
        return error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "invalid_expected_bytes",
            &format!(
                "Selected input is {:.2} GiB; this purpose supports at most {:.2} GiB.",
                body.expected_bytes as f64 / 1_073_741_824.0,
                max as f64 / 1_073_741_824.0
            ),
        );
    }
    if body
        .expected_sha256
        .as_ref()
        .is_some_and(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return invalid_body(
            "invalid_digest",
            "expectedSha256 must contain 64 hexadecimal characters.",
        );
    }
    if matches!(
        body.purpose,
        StagedUploadPurposeDto::RawServerArchive
            | StagedUploadPurposeDto::RawServerFolder
            | StagedUploadPurposeDto::MscTransferArchive
    ) && (body.target_server_id.is_some()
        || body.operation_id.is_some()
        || body.file_id.is_some())
    {
        return invalid_body(
            "invalid_binding",
            "Server sources cannot target an existing-server or pending-file slot.",
        );
    }
    if body.creation_context
        && (body.target_server_id.is_some()
            || body.operation_id.is_some()
            || body.file_id.is_some()
            || !matches!(
                body.purpose,
                StagedUploadPurposeDto::WorldImport
                    | StagedUploadPurposeDto::AddonLocalFile
                    | StagedUploadPurposeDto::ModpackArchive
            ))
    {
        return invalid_body(
            "invalid_creation_context",
            "Creation inputs cannot target an existing server or pending file.",
        );
    }
    let target = body.target_server_id.clone();
    if let Some(target) = &target
        && state
            .lifecycle
            .app_config_snapshot()
            .active_server_id
            .as_ref()
            != Some(target)
    {
        return error_response(
            StatusCode::CONFLICT,
            "target_mismatch",
            "The selected server changed.",
        );
    }
    if matches!(
        body.purpose,
        StagedUploadPurposeDto::WorldImport
            | StagedUploadPurposeDto::ActiveWorldReplace
            | StagedUploadPurposeDto::WorldThumbnail
            | StagedUploadPurposeDto::AddonLocalFile
    ) && target.is_none()
        && !body.creation_context
    {
        return invalid_body(
            "missing_target",
            "targetServerId is required for this purpose.",
        );
    }
    if body
        .file_name
        .as_ref()
        .is_some_and(|name| !safe_basename(name))
        || (matches!(
            body.purpose,
            StagedUploadPurposeDto::AddonLocalFile
                | StagedUploadPurposeDto::CurseforgeManualFile
                | StagedUploadPurposeDto::ModpackUnresolvedFile
        ) && body.file_name.is_none())
    {
        return invalid_body(
            "invalid_file_name",
            "Provide the original file basename without a path.",
        );
    }
    let pending = body
        .operation_id
        .as_ref()
        .zip(body.file_id.as_ref())
        .and_then(|(op, file)| pending_identity(&state, op, file));
    let _admission = state.staging.native.admission.lock().await;
    if state.lifecycle.servers_root() != servers_root
        || state.staging.native.root.lock().unwrap().as_ref() != Some(&servers_root)
    {
        return error_response(
            StatusCode::CONFLICT,
            "staging_root_changed",
            "The server storage location changed; retry admission.",
        );
    }

    let reserve = if body.purpose == StagedUploadPurposeDto::RawServerArchive
        && body
            .file_name
            .as_deref()
            .is_some_and(|n| n.ends_with(".tar"))
    {
        body.expected_bytes.saturating_mul(2)
    } else {
        body.expected_bytes
    };
    let (count, reserved, outstanding) = {
        let sessions = state.staging.native.sessions.lock().unwrap();
        (
            sessions
                .values()
                .filter(|s| s.reserved.load(Ordering::Acquire))
                .count(),
            sessions
                .values()
                .filter(|s| s.reserved.load(Ordering::Acquire))
                .map(|s| s.reservation)
                .sum::<u64>(),
            sessions
                .values()
                .filter(|s| s.reserved.load(Ordering::Acquire))
                .map(|s| {
                    s.reservation
                        .saturating_sub(s.received.load(Ordering::Acquire))
                })
                .sum::<u64>(),
        )
    };
    // Snapshot paths under the maps, then inspect disk without their locks.
    let (legacy_count, legacy_paths) = {
        let uploads = state.staging.uploads.lock().unwrap();
        let entries: Vec<_> = uploads
            .entries
            .iter()
            .filter(|(id, _)| !uploads.native.contains_key(*id))
            .collect();
        (
            entries.len(),
            entries
                .into_iter()
                .map(|(_, u)| (u.path.clone(), u.expected_bytes.unwrap_or(u.max_bytes)))
                .collect::<Vec<_>>(),
        )
    };
    if count + legacy_count >= 8
        || reserved
            .checked_add(reserve)
            .is_none_or(|n| n > RESERVED_BYTES)
    {
        return error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "staging_capacity",
            "Native staging capacity is full.",
        );
    }
    let mut downloads = state
        .staging
        .downloads
        .lock()
        .unwrap()
        .values()
        .map(|d| d.path.clone())
        .collect::<Vec<_>>();
    let id = Uuid::new_v4().to_string();
    let root = staging_root(&servers_root).join("native");
    let dir = root.join(&id);
    let path = dir.join("payload.bin");
    downloads.extend(
        state
            .staging
            .active_downloads
            .lock()
            .unwrap()
            .values()
            .map(|entry| entry.path.clone()),
    );
    let ledger = Ledger {
        owner: credential.credential_id.clone(),
        expected_bytes: body.expected_bytes,
        reserved_bytes: Some(reserve),
        expires_at: now_unix() + STAGING_TTL_SECONDS,
    };
    let create_dir = dir.clone();
    let space = state.staging.native.space.clone();
    let folder = body.purpose == StagedUploadPurposeDto::RawServerFolder;
    let create = tokio::task::spawn_blocking(move || -> std::io::Result<Option<std::fs::File>> {
        let legacy_reserved = legacy_paths.iter().map(|(_, size)| size).sum::<u64>();
        let downloads_bytes = downloads
            .iter()
            .try_fold(0_u64, |sum, p| file_size(p).map(|n| sum.saturating_add(n)))?;
        let legacy_outstanding = legacy_paths
            .iter()
            .try_fold(0_u64, |sum, (path, expected)| {
                file_size(path).map(|n| sum.saturating_add(expected.saturating_sub(n)))
            })?;
        if reserved
            .saturating_add(legacy_reserved)
            .saturating_add(downloads_bytes)
            .saturating_add(reserve)
            > RESERVED_BYTES
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::QuotaExceeded,
                "staging reservation limit",
            ));
        }
        let free = space(&root)?;
        let required = outstanding
            .saturating_add(legacy_outstanding)
            .saturating_add(reserve)
            .saturating_add(DISK_MARGIN);
        if free < required {
            return Err(std::io::Error::new(
                std::io::ErrorKind::StorageFull,
                format!("Import staging needs {:.2} GiB free including reservations and safety margin; {:.2} GiB is available. Free space or choose another server storage location.",
                    required as f64 / 1_073_741_824.0, free as f64 / 1_073_741_824.0),
            ));
        }
        private_dir(&create_dir)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut journal = options.open(create_dir.join("owner.json"))?;
        std::io::Write::write_all(&mut journal, &serde_json::to_vec(&ledger)?)?;
        journal.sync_all()?;
        if folder {
            private_dir(&create_dir.join("payload.bin"))?;
            Ok(None)
        } else {
            options.open(create_dir.join("payload.bin")).map(Some)
        }
    })
    .await;
    match create {
        Ok(Ok(file)) => drop(file),
        result => {
            let status = if matches!(&result, Ok(Err(error)) if error.kind() == std::io::ErrorKind::QuotaExceeded)
            {
                StatusCode::TOO_MANY_REQUESTS
            } else {
                StatusCode::INSUFFICIENT_STORAGE
            };
            if let Err(error) = tokio::fs::remove_dir_all(&dir).await
                && error.kind() != std::io::ErrorKind::NotFound
            {
                *state.staging.native.recovery_error.lock().unwrap() = Some(
                    "Admission cleanup failed; restart the agent to retry recovery.".to_string(),
                );
            }
            let message = match &result {
                Ok(Err(error)) if error.kind() == std::io::ErrorKind::StorageFull => error.to_string(),
                _ => "Could not reserve private staging; check capacity, available space and staging recovery.".into(),
            };
            return error_response(status, "staging_space", &message);
        }
    }
    let lifecycle = state.lifecycle.clone();
    let (cancel, _) = watch::channel(false);
    let (done, _) = watch::channel(true);
    let session = Arc::new(Session {
        owner: credential.credential_id.clone(),
        reservation: reserve,
        adapter: Default::default(),
        target: target.clone(),
        upload: StagedUpload {
            native_guard: None,
            purpose: body.purpose,
            file_name: body.file_name,
            operation_id: body.operation_id,
            file_id: body.file_id,
            expires_at_unix: now_unix() + STAGING_TTL_SECONDS,
            max_bytes: max,
            expected_bytes: Some(body.expected_bytes),
            received_bytes: 0,
            complete: false,
            path,
        },
        expected_sha256: body.expected_sha256.map(|s| s.to_ascii_lowercase()),
        pending_identity: pending,
        inspections: AtomicUsize::new(0),
        consumer_done: AtomicBool::new(false),
        phase: AtomicU8::new(ADMITTED),
        received: AtomicU64::new(0),
        expiry: AtomicU64::new(now_unix() + STAGING_TTL_SECONDS),
        reserved: AtomicBool::new(true),
        cancel,
        done,
        cleanup_error: Mutex::new(None),
        binding_valid: Box::new(move |credential| {
            purpose_allowed(purpose, credential)
                && lifecycle.servers_root() == servers_root
                && target.as_ref().is_none_or(|id| {
                    lifecycle.app_config_snapshot().active_server_id.as_ref() == Some(id)
                })
        }),
    });
    state
        .staging
        .native
        .sessions
        .lock()
        .unwrap()
        .insert(id.clone(), session.clone());
    // Retain at most 64 terminal records, independently of the eight live slots.
    {
        let mut sessions = state.staging.native.sessions.lock().unwrap();
        let mut terminal = sessions
            .iter()
            .filter(|(_, s)| !s.reserved.load(Ordering::Acquire))
            .map(|(id, s)| (id.clone(), s.expiry.load(Ordering::Acquire)))
            .collect::<Vec<_>>();
        terminal.sort_by_key(|(_, expiry)| *expiry);
        for (id, _) in terminal.iter().take(terminal.len().saturating_sub(64)) {
            sessions.remove(id);
        }
    }
    audit(
        &state.lifecycle,
        &credential,
        "POST",
        "/v1/staged-uploads/stream",
        StatusCode::CREATED,
    );
    (
        StatusCode::CREATED,
        Json(NativeUploadBeginResultDto {
            staged_upload_id: id.clone(),
            upload_path: format!("/v1/staged-uploads/{id}/stream"),
            stream_path: format!("/v1/staged-uploads/{id}/stream"),
            expires_at: unix_to_iso8601(session.expiry.load(Ordering::Acquire)),
            max_bytes: max,
            protocol: 1,
        }),
    )
        .into_response()
}

pub(crate) async fn receive(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    Extension(auth): Extension<crate::auth::AuthState>,
    AxumPath(id): AxumPath<String>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    receive_source(state, credential, auth, id, headers, body, None).await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn receive_source(
    state: ComponentsRoutesState,
    credential: AuthenticatedCredential,
    auth: crate::auth::AuthState,
    id: String,
    headers: HeaderMap,
    body: Body,
    folder: Option<(std::fs::File, msc_infrastructure::source_tree::Snapshot)>,
) -> Response {
    let Some(session) = state.staging.native.session(&id, &credential) else {
        return error_response(StatusCode::NOT_FOUND, "not_found", "Unknown native upload.");
    };
    if (session.upload.purpose == StagedUploadPurposeDto::RawServerFolder) != folder.is_some() {
        return invalid_body(
            "invalid_transport",
            "Folders require the private descriptor endpoint.",
        );
    }
    if !session.authorized(&credential)
        || !session.pending_valid(&state)
        || now_unix() > session.expiry.load(Ordering::Acquire)
    {
        return error_response(
            StatusCode::CONFLICT,
            "upload_binding",
            "Upload binding or lease is no longer valid.",
        );
    }
    let length = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    if length != session.upload.expected_bytes
        || headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            != Some("application/octet-stream")
        || headers
            .get(header::CONTENT_ENCODING)
            .is_some_and(|v| v != "identity")
        || headers.contains_key(header::TRANSFER_ENCODING)
    {
        return invalid_body(
            "invalid_stream_headers",
            "Use an exact Content-Length, octet-stream and identity encoding.",
        );
    }
    let Ok(permit) = state.staging.transfer_slots.clone().try_acquire_owned() else {
        return error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "stream_capacity",
            "Both native transfer slots are active.",
        );
    };
    {
        let sessions = state.staging.native.sessions.lock().unwrap();
        let active = sessions
            .values()
            .filter(|s| matches!(s.phase.load(Ordering::Acquire), RECEIVING | VERIFYING))
            .collect::<Vec<_>>();
        if active.len() >= 2 || active.iter().any(|s| s.owner == session.owner) {
            return error_response(
                StatusCode::TOO_MANY_REQUESTS,
                "stream_capacity",
                "An upload is already active.",
            );
        }
        if session
            .phase
            .compare_exchange(ADMITTED, RECEIVING, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return error_response(
                StatusCode::CONFLICT,
                "upload_state",
                "Upload already started or terminated.",
            );
        }
        session.done.send_replace(false);
    }
    // A detached worker owns cleanup even if the HTTP handler is dropped.
    let mut disconnect = Disconnect {
        session: session.clone(),
        armed: true,
    };
    let worker_session = session.clone();
    let worker_state = state.clone();
    let worker_credential = credential.clone();
    let worker_id = id.clone();
    let result = tokio::spawn(async move {
        let _permit = permit;
        let session = worker_session;
        let mut guard = WorkerGuard {
            state: worker_state.clone(),
            id: worker_id.clone(),
            session: session.clone(),
            armed: true,
        };
        let mut cancelled = session.cancel.subscribe();
        let mut changes = auth.stream_changes();
        let identity = crate::auth::StreamAuthentication {
            credential: worker_credential.clone(),
        };
        let outcome = {
            let is_folder = folder.is_some();
            let drive = async {
                match folder {
                    Some((source, snapshot)) => {
                        write_folder(session.clone(), source, snapshot).await
                    }
                    None => write_stream(&session, body).await,
                }
            };
            tokio::pin!(drive);
            loop {
                if *cancelled.borrow()
                    || !auth.stream_retains_permissions(&identity)
                    || !session.authorized(&worker_credential)
                {
                    session.cancel.send_replace(true);
                    if is_folder {
                        let _ = drive.as_mut().await;
                    }
                    break Err((
                        StatusCode::CONFLICT,
                        "upload_cancelled",
                        "Upload cancelled or authorization changed.",
                    ));
                }
                tokio::select! {
                    result = &mut drive => break result,
                    _ = cancelled.changed() => {},
                    _ = changes.changed() => {},
                }
            }
        }; // The drive and its file are dropped before deletion (Windows too).
        let response = match outcome {
            Ok(digest) => {
                let pending_valid = session.pending_valid(&worker_state);
                let mut uploads = worker_state.staging.uploads.lock().unwrap();
                if pending_valid
                    && session.authorized(&worker_credential)
                    && auth.stream_retains_permissions(&identity)
                    && !*session.cancel.borrow()
                    && session
                        .phase
                        .compare_exchange(VERIFYING, READY, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok()
                {
                    session.touch();
                    let mut upload = session.upload.clone();
                    upload.complete = true;
                    upload.received_bytes = session.received.load(Ordering::Acquire);
                    // The shared access boundary checks the sliding native lease.
                    upload.expires_at_unix = u64::MAX;
                    uploads.entries.insert(worker_id.clone(), upload);
                    uploads.native.insert(worker_id.clone(), session.clone());
                    Json(StagedUploadCompleteResultDto {
                        staged_upload_id: worker_id.clone(),
                        received_bytes: session.received.load(Ordering::Acquire) as i64,
                        sha256: digest,
                    })
                    .into_response()
                } else {
                    error_response(
                        StatusCode::CONFLICT,
                        "upload_cancelled",
                        "Upload cancelled before publication.",
                    )
                }
            }
            Err((status, code, message)) => error_response(status, code, message),
        };
        if session.phase.load(Ordering::Acquire) != READY {
            cleanup(&worker_state, &worker_id, &session, FAILED).await;
        }
        session.done.send_replace(true);
        guard.armed = false;
        response
    })
    .await;
    disconnect.armed = false;
    let response = result.unwrap_or_else(|_| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "stream_worker_failed",
            "Native upload worker failed; cleanup is pending.",
        )
    });
    audit(
        &state.lifecycle,
        &credential,
        "PUT",
        "/v1/staged-uploads/:id/stream",
        response.status(),
    );
    response
}

struct Disconnect {
    session: Arc<Session>,
    armed: bool,
}
impl Drop for Disconnect {
    fn drop(&mut self) {
        if self.armed {
            self.session.cancel.send_replace(true);
        }
    }
}
struct WorkerGuard {
    state: ComponentsRoutesState,
    id: String,
    session: Arc<Session>,
    armed: bool,
}
impl Drop for WorkerGuard {
    fn drop(&mut self) {
        if self.armed {
            let state = self.state.clone();
            let id = self.id.clone();
            let session = self.session.clone();
            session.phase.store(CANCELLING, Ordering::Release);
            tokio::spawn(async move {
                cleanup(&state, &id, &session, FAILED).await;
                session.done.send_replace(true);
            });
        }
    }
}

async fn write_folder(
    session: Arc<Session>,
    source: std::fs::File,
    snapshot: msc_infrastructure::source_tree::Snapshot,
) -> Result<String, StreamFailure> {
    tokio::task::spawn_blocking(move || {
        let mut hash = Sha256::new();
        if snapshot.bytes != session.upload.expected_bytes.unwrap_or(0) {
            return Err((
                StatusCode::CONFLICT,
                "source_changed",
                "Folder size changed.",
            ));
        }
        msc_infrastructure::source_tree::copy(
            &source,
            Path::new(""),
            &session.upload.path,
            &snapshot,
            &|| *session.cancel.borrow(),
            &mut |bytes| {
                let next = session
                    .received
                    .fetch_add(bytes.len() as u64, Ordering::AcqRel)
                    + bytes.len() as u64;
                if next > snapshot.bytes {
                    return Err(std::io::Error::other("folder grew"));
                }
                hash.update(bytes);
                session.touch();
                Ok(())
            },
        )
        .map_err(|_| {
            (
                StatusCode::CONFLICT,
                "source_changed",
                "Folder copy failed, changed, or was cancelled.",
            )
        })?;
        if session.received.load(Ordering::Acquire) != snapshot.bytes
            || session
                .phase
                .compare_exchange(RECEIVING, VERIFYING, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return Err((
                StatusCode::CONFLICT,
                "source_changed",
                "Folder copy did not complete.",
            ));
        }
        Ok(format!("{:x}", hash.finalize()))
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "copy_worker_failed",
            "Folder copy worker failed.",
        )
    })?
}

type StreamFailure = (StatusCode, &'static str, &'static str);
async fn write_stream(session: &Session, body: Body) -> Result<String, StreamFailure> {
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .open(&session.upload.path)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "staging_write",
                "Could not open private staging.",
            )
        })?;
    file.set_max_buf_size(BLOCK_BYTES);
    let mut stream = body.into_data_stream();
    let mut hash = Sha256::new();
    let mut deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(120);
    let expected = session.upload.expected_bytes.unwrap_or(0);
    loop {
        let frame = tokio::time::timeout_at(deadline, stream.next())
            .await
            .map_err(|_| {
                (
                    StatusCode::REQUEST_TIMEOUT,
                    "stream_idle",
                    "No upload progress for 120 seconds.",
                )
            })?;
        let Some(frame) = frame else {
            break;
        };
        let frame = frame.map_err(|_| {
            (
                StatusCode::CONFLICT,
                "stream_truncated",
                "Upload disconnected before completion.",
            )
        })?;
        if frame.len() > MAX_FRAME_BYTES {
            return Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                "stream_frame_limit",
                "Transport frame exceeds the bounded receiver limit.",
            ));
        }
        for block in frame.chunks(BLOCK_BYTES) {
            let next = session
                .received
                .load(Ordering::Acquire)
                .checked_add(block.len() as u64)
                .filter(|n| *n <= expected && *n <= session.upload.max_bytes)
                .ok_or((
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "max_bytes_exceeded",
                    "Upload exceeds the admitted length.",
                ))?;
            tokio::time::timeout_at(deadline, file.write_all(block))
                .await
                .map_err(|_| {
                    (
                        StatusCode::REQUEST_TIMEOUT,
                        "stream_idle",
                        "Staging write made no progress.",
                    )
                })?
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "staging_write",
                        "Could not write private staging.",
                    )
                })?;
            deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(120);
            hash.update(block);
            session.received.store(next, Ordering::Release);
            session.touch();
        }
    }
    if session.received.load(Ordering::Acquire) != expected {
        return Err((
            StatusCode::CONFLICT,
            "stream_truncated",
            "Upload did not match the admitted length.",
        ));
    }
    if session
        .phase
        .compare_exchange(RECEIVING, VERIFYING, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err((
            StatusCode::CONFLICT,
            "upload_cancelled",
            "Upload was cancelled.",
        ));
    }
    let digest = format!("{:x}", hash.finalize());
    if session
        .expected_sha256
        .as_ref()
        .is_some_and(|expected| expected != &digest)
    {
        return Err((
            StatusCode::CONFLICT,
            "digest_mismatch",
            "Upload SHA-256 did not match.",
        ));
    }
    tokio::time::timeout(std::time::Duration::from_secs(120), file.sync_all())
        .await
        .map_err(|_| {
            (
                StatusCode::REQUEST_TIMEOUT,
                "stream_idle",
                "Staging flush timed out.",
            )
        })?
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "staging_sync",
                "Could not flush private staging.",
            )
        })?;
    drop(file);
    Ok(digest)
}

async fn cleanup(state: &ComponentsRoutesState, id: &str, session: &Session, terminal: u8) {
    let _adapter = session.adapter.lock().await;
    if session.inspections.load(Ordering::Acquire) > 0 {
        return;
    }
    cleanup_locked(state, id, session, terminal).await;
}
async fn cleanup_locked(state: &ComponentsRoutesState, id: &str, session: &Session, terminal: u8) {
    {
        let mut uploads = state.staging.uploads.lock().unwrap();
        uploads.entries.remove(id);
        uploads.native.remove(id);
    }
    let dir = session
        .upload
        .path
        .parent()
        .expect("private native slot has a parent");
    match tokio::fs::remove_dir_all(dir).await {
        Ok(()) => {
            *session.cleanup_error.lock().unwrap() = None;
            session.reserved.store(false, Ordering::Release);
            session.phase.store(terminal, Ordering::Release);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            *session.cleanup_error.lock().unwrap() = None;
            session.reserved.store(false, Ordering::Release);
            session.phase.store(terminal, Ordering::Release);
        }
        Err(error) => {
            *session.cleanup_error.lock().unwrap() = Some(error.to_string());
            session.phase.store(CLEANUP_FAILED, Ordering::Release);
        }
    }
}

pub(crate) async fn server_folder(
    staging: &StagingStore,
    credential: &AuthenticatedCredential,
    id: &str,
    auth: Option<crate::auth::AuthState>,
) -> Result<PathBuf, ()> {
    let session = staging.native.session(id, credential).ok_or(())?;
    let identity = crate::auth::StreamAuthentication {
        credential: credential.clone(),
    };
    let guard = session.adapter.clone().lock_owned().await;
    if !session.authorized(credential) || session.phase.load(Ordering::Acquire) != READY {
        return Err(());
    }
    tokio::task::spawn_blocking(move || {
        let _guard = guard;
        let destination = session.upload.path.parent().ok_or(())?.join("folder");
        if !destination.exists() {
            let partial = destination.with_file_name(format!("folder-{}", Uuid::new_v4()));
            std::fs::create_dir(&partial).map_err(|_| ())?;
            let result = msc_infrastructure::source_tree::extract_tar(
                &session.upload.path,
                &partial,
                &|| {
                    *session.cancel.borrow()
                        || auth
                            .as_ref()
                            .is_some_and(|auth| !auth.stream_retains_permissions(&identity))
                },
            );
            if result.is_err() || *session.cancel.borrow() {
                let _ = std::fs::remove_dir_all(&partial);
                return Err(());
            }
            std::fs::rename(&partial, &destination).map_err(|_| ())?;
        }
        Ok(destination)
    })
    .await
    .map_err(|_| ())?
}

pub(crate) async fn cancel(
    state: &ComponentsRoutesState,
    credential: &AuthenticatedCredential,
    id: &str,
) -> Response {
    let Some(session) = state.staging.native.session(id, credential) else {
        return error_response(StatusCode::NOT_FOUND, "not_found", "Unknown native upload.");
    };
    loop {
        let phase = session.phase.load(Ordering::Acquire);
        if phase == REDEEMED {
            return error_response(
                StatusCode::CONFLICT,
                "already_redeemed",
                "Import already owns this file.",
            );
        }
        if matches!(phase, CANCELLED | FAILED) && !session.reserved.load(Ordering::Acquire) {
            return StatusCode::NO_CONTENT.into_response();
        }
        if session
            .phase
            .compare_exchange(phase, CANCELLING, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            break;
        }
    }
    let mut done = session.done.subscribe();
    session.cancel.send_replace(true);
    while !*done.borrow_and_update() {
        if done.changed().await.is_err() {
            break;
        }
    }
    cleanup(state, id, &session, CANCELLED).await;
    let response = if session.phase.load(Ordering::Acquire) == CLEANUP_FAILED {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "cleanup_failed",
            "Could not remove partial staging; reservation retained.",
        )
    } else {
        StatusCode::NO_CONTENT.into_response()
    };
    audit(
        &state.lifecycle,
        credential,
        "DELETE",
        "/v1/staged-uploads/:id",
        response.status(),
    );
    response
}

pub(crate) async fn status(
    State(state): State<ComponentsRoutesState>,
    Extension(credential): Extension<AuthenticatedCredential>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let Some(session) = state.staging.native.session(&id, &credential) else {
        return error_response(StatusCode::NOT_FOUND, "not_found", "Unknown native upload.");
    };
    let phase = match session.phase.load(Ordering::Acquire) {
        ADMITTED => "admitted",
        RECEIVING => "receiving",
        VERIFYING => "verifying",
        READY => "ready",
        REDEEMED => "redeemed",
        CANCELLING => "cancelling",
        CANCELLED => "cancelled",
        FAILED => "failed",
        _ => "cleanup-failed",
    };
    Json(NativeUploadStatusDto {
        staged_upload_id: id,
        state: phase.to_string(),
        received_bytes: session.received.load(Ordering::Acquire),
        expected_bytes: session.upload.expected_bytes.unwrap_or(0),
        expires_at: unix_to_iso8601(session.expiry.load(Ordering::Acquire)),
        cleanup_error: session
            .cleanup_error
            .lock()
            .unwrap()
            .as_ref()
            .map(|_| "Private staging cleanup failed; retry cancellation.".to_string()),
    })
    .into_response()
}

async fn reap_once(state: &ComponentsRoutesState) {
    super::reap_downloads(state).await;
    let sessions = state
        .staging
        .native
        .sessions
        .lock()
        .unwrap()
        .iter()
        .map(|(id, s)| (id.clone(), s.clone()))
        .collect::<Vec<_>>();
    for (id, session) in sessions {
        let phase = session.phase.load(Ordering::Acquire);
        if session.inspections.load(Ordering::Acquire) > 0 {
            session.touch();
            continue;
        }
        if phase == REDEEMED {
            if session.consumer_done.load(Ordering::Acquire) {
                cleanup(state, &id, &session, REDEEMED).await;
            }
        } else if phase == CLEANUP_FAILED {
            cleanup(state, &id, &session, FAILED).await;
        } else if matches!(phase, ADMITTED | READY)
            && now_unix() > session.expiry.load(Ordering::Acquire)
            && session
                .phase
                .compare_exchange(phase, CANCELLING, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        {
            cleanup(state, &id, &session, CANCELLED).await;
        }
    }
}

pub(crate) fn spawn_reaper(state: ComponentsRoutesState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            reap_once(&state).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthState, CredentialRole};
    use crate::routes::operations::OperationsState;
    use axum::body::to_bytes;
    use msc_infrastructure::secret_store::FakeSecretStore;

    struct Fixture {
        state: ComponentsRoutesState,
        auth: AuthState,
        credential: AuthenticatedCredential,
        #[cfg(unix)]
        token: String,
    }
    impl Fixture {
        fn new() -> Self {
            let lifecycle = LifecycleRoutesState::with_fake_process(
                crate::ws::console::ConsoleState::default(),
                OperationsState::fake_journaled(),
            );
            let staging = StagingStore {
                native: NativeStore {
                    space: Arc::new(|_| Ok(RESERVED_BYTES + DISK_MARGIN)),
                    ..Default::default()
                },
                ..Default::default()
            };
            let state =
                ComponentsRoutesState::new(lifecycle, staging, PendingModpackImports::default());
            state
                .staging
                .native
                .initialize(&state.lifecycle.servers_root());
            assert!(state.staging.native.available());
            let auth = AuthState::new(Arc::new(FakeSecretStore::new()));
            let issued = auth
                .issue_credential(
                    "transfer-owner",
                    CredentialRole::Named,
                    vec![PermissionCategoryDto::Addons],
                    None,
                )
                .unwrap();
            let credential = AuthenticatedCredential {
                credential_id: issued.credential_id,
                label: "transfer-owner".into(),
                role: CredentialRole::Named,
                permissions: vec![PermissionCategoryDto::Addons],
            };
            Self {
                state,
                auth,
                credential,
                #[cfg(unix)]
                token: issued.token,
            }
        }
        async fn admit(&self, expected: u64, digest: Option<String>) -> String {
            let response = admit(
                State(self.state.clone()),
                Extension(self.credential.clone()),
                Ok(Json(Admission {
                    creation_context: false,
                    purpose: StagedUploadPurposeDto::ModpackArchive,
                    file_name: Some("pack.zip".into()),
                    expected_bytes: expected,
                    expected_sha256: digest,
                    operation_id: None,
                    file_id: None,
                    target_server_id: None,
                })),
            )
            .await;
            assert_eq!(response.status(), StatusCode::CREATED);
            let result: NativeUploadBeginResultDto =
                serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap())
                    .unwrap();
            result.staged_upload_id
        }
        async fn send(&self, id: &str, length: u64, body: Body) -> Response {
            receive(
                State(self.state.clone()),
                Extension(self.credential.clone()),
                Extension(self.auth.clone()),
                AxumPath(id.to_string()),
                headers(length),
                body,
            )
            .await
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.state.lifecycle.servers_root().parent().unwrap());
        }
    }
    fn headers(length: u64) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            "application/octet-stream".parse().unwrap(),
        );
        headers.insert(header::CONTENT_LENGTH, length.to_string().parse().unwrap());
        headers
    }

    // Essential: a real reported 20 GB server archive must reach admission,
    // while insufficient disk space and individual-world ceilings still refuse.
    // Only metadata/empty staging files are created; no multi-GB input is allocated.
    #[tokio::test]
    async fn large_server_admission_preserves_disk_and_world_limits() {
        let mut fixture = Fixture::new();
        fixture.credential.permissions =
            vec![PermissionCategoryDto::Fleet, PermissionCategoryDto::Worlds];
        let mut body = Admission {
            creation_context: false,
            purpose: StagedUploadPurposeDto::RawServerArchive,
            file_name: Some("tectonic.zip".into()),
            expected_bytes: 20_066_596_805,
            expected_sha256: None,
            operation_id: None,
            file_id: None,
            target_server_id: None,
        };
        let response = admit(
            State(fixture.state.clone()),
            Extension(fixture.credential.clone()),
            Ok(Json(body.clone())),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        fixture.state.staging.native.space = Arc::new(|_| Ok(DISK_MARGIN));
        let response = admit(
            State(fixture.state.clone()),
            Extension(fixture.credential.clone()),
            Ok(Json(body.clone())),
        )
        .await;
        assert_eq!(response.status(), StatusCode::INSUFFICIENT_STORAGE);
        let bytes = to_bytes(response.into_body(), 16 * 1024).await.unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains("GiB free"));
        body.purpose = StagedUploadPurposeDto::WorldImport;
        let response = admit(
            State(fixture.state.clone()),
            Extension(fixture.credential.clone()),
            Ok(Json(body)),
        )
        .await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    // Essential: creation picks must work without weakening the required
    // existing-server binding. Tiny isolated admission requests, no I/O stream.
    #[tokio::test]
    async fn native_creation_context_is_explicit_and_excludes_existing_targets() {
        let fixture = Fixture::new();
        let mut body = Admission {
            creation_context: false,
            purpose: StagedUploadPurposeDto::AddonLocalFile,
            file_name: Some("addon.jar".into()),
            expected_bytes: 6,
            expected_sha256: None,
            operation_id: None,
            file_id: None,
            target_server_id: None,
        };
        assert_eq!(
            admit(
                State(fixture.state.clone()),
                Extension(fixture.credential.clone()),
                Ok(Json(body.clone()))
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
        body.creation_context = true;
        body.target_server_id = Some("existing-server".into());
        assert_eq!(
            admit(
                State(fixture.state.clone()),
                Extension(fixture.credential.clone()),
                Ok(Json(body.clone()))
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
        body.target_server_id = None;
        body.file_id = Some("pending-file".into());
        assert_eq!(
            admit(
                State(fixture.state.clone()),
                Extension(fixture.credential.clone()),
                Ok(Json(body.clone()))
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
        body.file_id = None;
        assert_eq!(
            admit(
                State(fixture.state.clone()),
                Extension(fixture.credential.clone()),
                Ok(Json(body))
            )
            .await
            .status(),
            StatusCode::CREATED
        );
    }

    // Essential: a folder is a descriptor-only source, not an HTTP file body.
    // Its verified files must be usable by the existing raw-server scanner.
    #[cfg(unix)]
    #[tokio::test]
    async fn native_server_folder_requires_fleet_and_publishes_a_verified_tree() {
        let mut fixture = Fixture::new();
        let admission = Admission {
            creation_context: false,
            purpose: StagedUploadPurposeDto::RawServerFolder,
            file_name: Some("server".into()),
            expected_bytes: 6,
            expected_sha256: None,
            operation_id: None,
            file_id: None,
            target_server_id: None,
        };
        assert_eq!(
            admit(
                State(fixture.state.clone()),
                Extension(fixture.credential.clone()),
                Ok(Json(admission.clone()))
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
        let issued = fixture
            .auth
            .issue_credential(
                "fleet",
                CredentialRole::Named,
                vec![PermissionCategoryDto::Fleet],
                None,
            )
            .unwrap();
        fixture.credential = AuthenticatedCredential {
            credential_id: issued.credential_id,
            label: "fleet".into(),
            role: CredentialRole::Named,
            permissions: vec![PermissionCategoryDto::Fleet],
        };
        fixture.token = issued.token;
        let response = admit(
            State(fixture.state.clone()),
            Extension(fixture.credential.clone()),
            Ok(Json(admission)),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let slot: NativeUploadBeginResultDto =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(
            fixture
                .send(&slot.staged_upload_id, 6, Body::from("abcdef"))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
        let source = fixture
            .state
            .lifecycle
            .servers_root()
            .join("selected-server");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join(".hidden"), b"abcdef").unwrap();
        std::fs::create_dir(source.join("empty")).unwrap();
        let file = msc_infrastructure::source_tree::open(&source, true).unwrap();
        let snapshot =
            msc_infrastructure::source_tree::snapshot(&file, &source, &|| false).unwrap();
        let response = receive_source(
            fixture.state.clone(),
            fixture.credential.clone(),
            fixture.auth.clone(),
            slot.staged_upload_id.clone(),
            headers(6),
            Body::empty(),
            Some((file, snapshot)),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let session = fixture
            .state
            .staging
            .native
            .session(&slot.staged_upload_id, &fixture.credential)
            .unwrap();
        assert_eq!(
            std::fs::read(session.upload.path.join(".hidden")).unwrap(),
            b"abcdef"
        );
        assert!(session.upload.path.join("empty").is_dir());
        assert_eq!(std::fs::read(source.join(".hidden")).unwrap(), b"abcdef");
        let body: msc_api::dto::ServerImportRequestDto = serde_json::from_value(
            serde_json::json!({"action":"scan","stagedUploadId":slot.staged_upload_id}),
        )
        .unwrap();
        let response = crate::auth::INITIATING_CREDENTIAL
            .scope(
                fixture.credential.clone(),
                crate::routes::servers::import(
                    State(fixture.state.lifecycle.clone()),
                    Extension(fixture.credential.clone()),
                    Extension(fixture.state.staging.clone()),
                    None,
                    Ok(Json(body)),
                ),
            )
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let scan: msc_api::dto::ServerImportScanResponseDto =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert!(scan.source_path.is_none());
        assert_eq!(session.phase.load(Ordering::Acquire), READY);
    }

    // Essential: the local descriptor path must produce the same verified slot,
    // while wrong-host and replay attempts must never start another copy.
    #[cfg(unix)]
    #[tokio::test]
    async fn local_handoff_verifies_its_host_and_publishes_the_selected_file_once() {
        use msc_infrastructure::local_source::{Identity, send_file};
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt, Interest},
            net::UnixStream,
        };
        let fixture = Fixture::new();
        let path = fixture.state.lifecycle.servers_root().join("selected.zip");
        std::fs::write(&path, b"abcdef").unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let identity = Identity::read(&file).unwrap();
        let host = fixture.auth.agent_host_id().unwrap();
        let id = fixture.admit(6, None).await;
        for (claimed_host, expected_ready) in [
            ("another-host".to_string(), false),
            (host.clone(), true),
            (host.clone(), false),
        ] {
            let (mut client, server) = UnixStream::pair().unwrap();
            let auth = fixture.auth.clone();
            let state = fixture.state.clone();
            let worker = tokio::spawn(super::super::local_source::exchange(auth, state, server));
            let mut request = serde_json::to_vec(&serde_json::json!({
                "protocol": 1, "agentHostId": claimed_host, "token": fixture.token,
                "stagedUploadId": id, "identity": identity,
            }))
            .unwrap();
            request.push(b'\n');
            client.write_all(&request).await.unwrap();
            if !expected_ready {
                assert!(worker.await.unwrap().is_err());
                assert_eq!(
                    fixture
                        .state
                        .staging
                        .native
                        .session(&id, &fixture.credential)
                        .unwrap()
                        .local_progress()
                        .0,
                    if claimed_host == host { 6 } else { 0 }
                );
                continue;
            }
            async fn read_event(client: &mut UnixStream) -> serde_json::Value {
                let mut bytes = Vec::new();
                loop {
                    let byte = client.read_u8().await.unwrap();
                    if byte == b'\n' {
                        return serde_json::from_slice(&bytes).unwrap();
                    }
                    assert!(bytes.len() < 16 * 1024);
                    bytes.push(byte);
                }
            }
            assert_eq!(read_event(&mut client).await["ready"], true);
            loop {
                client.writable().await.unwrap();
                match client.try_io(Interest::WRITABLE, || send_file(&client, &file)) {
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
                    result => {
                        result.unwrap();
                        break;
                    }
                }
            }
            loop {
                let event = read_event(&mut client).await;
                if event.get("status").is_some() {
                    assert_eq!(event["status"], 200);
                    assert_eq!(event["body"]["receivedBytes"], 6);
                    assert_eq!(
                        event["body"]["sha256"],
                        format!("{:x}", Sha256::digest(b"abcdef"))
                    );
                    break;
                }
            }
            worker.await.unwrap().unwrap();
            assert_eq!(std::fs::read(&path).unwrap(), b"abcdef");
            let session = fixture
                .state
                .staging
                .native
                .session(&id, &fixture.credential)
                .unwrap();
            assert_eq!(std::fs::read(&session.upload.path).unwrap(), b"abcdef");
            assert_eq!(session.phase.load(Ordering::Acquire), READY);
        }
    }

    // Essential: descriptor transfer cannot grant write access or be redirected
    // by replacing a pathname; observed source edits must invalidate the copy.
    #[cfg(unix)]
    #[test]
    fn local_descriptor_preserves_identity_and_rejects_writable_sources() {
        use msc_infrastructure::local_source::{Identity, receive_file, send_file};
        let fixture = Fixture::new();
        let path = fixture.state.lifecycle.servers_root().join("source.zip");
        std::fs::write(&path, b"abc").unwrap();
        let original = std::fs::File::open(&path).unwrap();
        let moved = path.with_extension("retained");
        std::fs::rename(&path, &moved).unwrap();
        std::fs::write(&path, b"new").unwrap();
        let identity = Identity::read(&original).unwrap();
        let (sender, receiver) = std::os::unix::net::UnixStream::pair().unwrap();
        send_file(&sender, &original).unwrap();
        let mut received = receive_file(&receiver).unwrap();
        identity.validate(&received).unwrap();
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut received, &mut bytes).unwrap();
        assert_eq!(bytes, b"abc");
        std::fs::write(&moved, b"changed").unwrap();
        assert!(identity.validate(&received).is_err());
        let writable = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        send_file(&sender, &writable).unwrap();
        assert!(receive_file(&receiver).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
    }

    // Protects the cross-credential redemption boundary and legacy overwrite path.
    #[tokio::test]
    async fn native_stream_publishes_exact_bytes_only_to_its_owner_once() {
        let fixture = Fixture::new();
        let digest = format!("{:x}", Sha256::digest(b"abcdef"));
        let id = fixture.admit(6, Some(digest.clone())).await;
        let source = futures_util::stream::iter([
            Ok::<_, std::io::Error>(Bytes::from_static(b"abc")),
            Ok(Bytes::from_static(b"def")),
        ]);
        let response = fixture.send(&id, 6, Body::from_stream(source)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let result: StagedUploadCompleteResultDto =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(result.sha256, digest);
        assert_eq!(result.received_bytes, 6);
        let mut other = fixture.credential.clone();
        other.credential_id = "another-owner".into();
        crate::auth::INITIATING_CREDENTIAL
            .scope(other.clone(), async {
                assert!(
                    fixture
                        .state
                        .staging
                        .uploads
                        .lock()
                        .unwrap()
                        .remove(&id)
                        .is_none()
                );
            })
            .await;
        assert_eq!(
            cancel(&fixture.state, &other, &id).await.status(),
            StatusCode::NOT_FOUND
        );
        let overwritten = super::super::upload_staged_bytes(
            State(fixture.state.clone()),
            Extension(fixture.credential.clone()),
            AxumPath(id.clone()),
            Bytes::from_static(b"bad"),
        )
        .await;
        assert_eq!(overwritten.status(), StatusCode::CONFLICT);
        crate::auth::INITIATING_CREDENTIAL
            .scope(fixture.credential.clone(), async {
                let upload = fixture
                    .state
                    .staging
                    .uploads
                    .lock()
                    .unwrap()
                    .remove(&id)
                    .unwrap();
                assert_eq!(std::fs::read(upload.path).unwrap(), b"abcdef");
                assert!(
                    fixture
                        .state
                        .staging
                        .uploads
                        .lock()
                        .unwrap()
                        .remove(&id)
                        .is_none()
                );
            })
            .await;
        let session = fixture
            .state
            .staging
            .native
            .session(&id, &fixture.credential)
            .unwrap();
        reap_once(&fixture.state).await;
        assert!(!session.upload.path.exists());
        assert!(!session.reserved.load(Ordering::Acquire));
        assert_eq!(
            cancel(&fixture.state, &fixture.credential, &id)
                .await
                .status(),
            StatusCode::CONFLICT
        );
    }

    // Existing chunk tests cannot catch malformed/truncated streaming bodies.
    #[tokio::test]
    async fn native_stream_rejects_short_overlong_and_corrupt_bodies_without_publication() {
        let fixture = Fixture::new();
        for name in ["../plugin.jar", "NUL.jar", "plugin.jar:extra"] {
            let response = admit(
                State(fixture.state.clone()),
                Extension(fixture.credential.clone()),
                Ok(Json(Admission {
                    creation_context: false,
                    purpose: StagedUploadPurposeDto::ModpackArchive,
                    file_name: Some(name.into()),
                    expected_bytes: 3,
                    expected_sha256: None,
                    operation_id: None,
                    file_id: None,
                    target_server_id: None,
                })),
            )
            .await;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
        for (input, digest, status) in [
            ("ab", None, StatusCode::CONFLICT),
            ("abcd", None, StatusCode::PAYLOAD_TOO_LARGE),
            ("abc", Some("0".repeat(64)), StatusCode::CONFLICT),
        ] {
            let id = fixture.admit(3, digest).await;
            let path = fixture
                .state
                .staging
                .native
                .session(&id, &fixture.credential)
                .unwrap()
                .upload
                .path
                .clone();
            assert_eq!(
                fixture.send(&id, 3, Body::from(input)).await.status(),
                status
            );
            assert!(!path.exists());
            assert!(
                !fixture
                    .state
                    .staging
                    .uploads
                    .lock()
                    .unwrap()
                    .entries
                    .contains_key(&id)
            );
            assert!(
                !fixture
                    .state
                    .staging
                    .native
                    .session(&id, &fixture.credential)
                    .unwrap()
                    .reserved
                    .load(Ordering::Acquire)
            );
        }
    }

    // A pending body is controlled by notification, never a sleep or real socket.
    #[tokio::test]
    async fn native_stream_cancellation_and_revocation_close_and_remove_partial_staging() {
        let fixture = Fixture::new();
        for revoke in [false, true] {
            let id = fixture.admit(3, None).await;
            let session = fixture
                .state
                .staging
                .native
                .session(&id, &fixture.credential)
                .unwrap();
            let mut started = session.done.subscribe();
            let state = fixture.state.clone();
            let auth = fixture.auth.clone();
            let credential = fixture.credential.clone();
            let worker_id = id.clone();
            let task = tokio::spawn(async move {
                receive(
                    State(state),
                    Extension(credential),
                    Extension(auth),
                    AxumPath(worker_id),
                    headers(3),
                    Body::from_stream(
                        futures_util::stream::pending::<Result<Bytes, std::io::Error>>(),
                    ),
                )
                .await
            });
            while *started.borrow_and_update() {
                started.changed().await.unwrap();
            }
            if revoke {
                fixture
                    .auth
                    .revoke_credential(&fixture.credential.credential_id, "owner")
                    .unwrap();
            } else {
                assert_eq!(
                    cancel(&fixture.state, &fixture.credential, &id)
                        .await
                        .status(),
                    StatusCode::NO_CONTENT
                );
            }
            assert_eq!(task.await.unwrap().status(), StatusCode::CONFLICT);
            assert!(!session.upload.path.exists());
            assert!(
                !fixture
                    .state
                    .staging
                    .uploads
                    .lock()
                    .unwrap()
                    .entries
                    .contains_key(&id)
            );
        }
    }

    #[tokio::test]
    async fn native_stream_restart_discards_abandoned_slots_without_redeeming_them() {
        let fixture = Fixture::new();
        let id = fixture.admit(3, None).await;
        let path = fixture
            .state
            .staging
            .native
            .session(&id, &fixture.credential)
            .unwrap()
            .upload
            .path
            .clone();
        std::fs::write(&path, b"ab").unwrap();
        let restarted = StagingStore::default();
        restarted
            .native
            .initialize(&fixture.state.lifecycle.servers_root());
        assert!(restarted.native.available());
        assert!(!path.exists());
        assert!(!restarted.native.contains(&id));
        assert!(restarted.uploads.lock().unwrap().get(&id).is_none());
    }
}

fn storage_eligible(session: &Session) -> bool {
    session.reserved.load(Ordering::Acquire)
        && session.inspections.load(Ordering::Acquire) == 0
        && match session.phase.load(Ordering::Acquire) {
            ADMITTED | READY => now_unix() > session.expiry.load(Ordering::Acquire),
            CLEANUP_FAILED => *session.done.borrow(),
            REDEEMED => session.consumer_done.load(Ordering::Acquire),
            _ => false,
        }
}
pub(crate) async fn storage_candidates(
    state: &ComponentsRoutesState,
) -> Vec<super::storage_cleanup::Entry> {
    let sessions = state
        .staging
        .native
        .sessions
        .lock()
        .unwrap()
        .iter()
        .take(256)
        .map(|(id, s)| (id.clone(), s.clone()))
        .collect::<Vec<_>>();
    let mut result = vec![];
    for (id, session) in sessions {
        if !session.reserved.load(Ordering::Acquire) {
            continue;
        }
        let Ok(_adapter) = session.adapter.try_lock() else {
            continue;
        };
        let eligible = storage_eligible(&session);
        let path = session.upload.path.parent().unwrap().to_path_buf();
        let measured = if eligible {
            tokio::task::spawn_blocking(move || {
                msc_infrastructure::storage::observe_tree(&path, &|| false)
            })
            .await
            .ok()
            .and_then(Result::ok)
        } else {
            None
        };
        result.push(super::storage_cleanup::Entry {
            id: format!("upload:{id}"),
            label: "Private upload staging".into(),
            bytes: measured.as_ref().map(|m| m.0),
            removable: eligible && measured.is_some(),
            reason: if eligible {
                "Expired transfer owned by the upload service"
            } else {
                "Active upload, inspection or consumer is protected"
            }
            .into(),
            fingerprint: measured.map(|m| m.1).unwrap_or_default(),
        });
    }
    result
}
pub(crate) async fn storage_remove(
    state: &ComponentsRoutesState,
    entry: &super::storage_cleanup::Entry,
) -> Result<u64, String> {
    let id = entry
        .id
        .strip_prefix("upload:")
        .ok_or("Unknown transfer kind")?;
    let _admission = state.staging.native.admission.lock().await;
    let session = state
        .staging
        .native
        .sessions
        .lock()
        .unwrap()
        .get(id)
        .cloned()
        .ok_or("Upload ownership no longer exists")?;
    let _adapter = session.adapter.try_lock().map_err(|_| "Upload is busy")?;
    if !storage_eligible(&session) {
        return Err("Upload is active or no longer expired".into());
    }
    let path = session.upload.path.parent().unwrap().to_path_buf();
    let observed = tokio::task::spawn_blocking(move || {
        msc_infrastructure::storage::observe_tree(&path, &|| false)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    if Some(observed.0) != entry.bytes || observed.1 != entry.fingerprint {
        return Err("Upload changed since review".into());
    }
    let phase = session.phase.load(Ordering::Acquire);
    session
        .phase
        .compare_exchange(phase, CANCELLING, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| "Upload became active")?;
    cleanup_locked(state, id, &session, CANCELLED).await;
    if session.reserved.load(Ordering::Acquire) {
        Err("Removal unconfirmed; upload service retains its cleanup reservation".into())
    } else {
        Ok(observed.0)
    }
}
