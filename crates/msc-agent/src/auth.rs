//! Phase 4 bearer-token auth: credentials are identified by the bearer
//! token's credential id and verified against `SecretStore`, not a fixed
//! environment variable.
//!
//! Token shape, from `docs/msc2/lifecycle/pairing-phase4.md`:
//!
//! ```text
//! msc2_<credential-id>_<secret>
//! ```
//!
//! The credential registry is intentionally non-secret. The secret store
//! value at `remote-api.token.<credential-id>` is the authority for whether
//! a token can authenticate.

use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use msc_api::dto::{ErrorDto, PermissionCategoryDto};
use msc_infrastructure::config_repository::LEGACY_OWNER_TOKEN_SECRET_KEY;
use msc_infrastructure::credential_repository::{
    CredentialRegistryEntry, CredentialRepository, CredentialRepositoryError,
};
use msc_infrastructure::fs::{FileSystem, StdFileSystem};
#[cfg(test)]
use msc_infrastructure::secret_store::FakeSecretStore;
use msc_infrastructure::secret_store::{SecretStore, SecretStoreError};
use rand::RngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::{Digest, Sha1};
use subtle::ConstantTimeEq;

const TOKEN_PREFIX: &str = "msc2";
const SECRET_STORE_KEY_PREFIX: &str = "remote-api.token.";
const HASH_ALGORITHM: &str = "sha1-salted-v1";
const AUTH_FAILURE_WINDOW: Duration = Duration::from_secs(60);
const AUTH_FAILURE_LIMIT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(not(test), allow(dead_code))]
pub enum CredentialRole {
    Admin,
    Guest,
    Named,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedCredential {
    pub credential_id: String,
    pub label: String,
    pub role: CredentialRole,
    pub permissions: Vec<PermissionCategoryDto>,
}

#[derive(Debug, Clone)]
struct CredentialRecord {
    label: String,
    role: CredentialRole,
    permissions: Vec<PermissionCategoryDto>,
    expires_at: Option<SystemTime>,
    revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub struct IssuedCredential {
    pub credential_id: String,
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthAuditEvent {
    pub token_label: String,
    pub status_code: u16,
    pub code: String,
}

#[derive(Clone)]
pub struct AuthState {
    inner: Arc<AuthStateInner>,
}

struct AuthStateInner {
    secret_store: Arc<dyn SecretStore + Send + Sync>,
    registry: Mutex<HashMap<String, CredentialRecord>>,
    failures: Mutex<HashMap<String, VecDeque<Instant>>>,
    audit_events: Mutex<Vec<AuthAuditEvent>>,
    /// Where the non-secret registry is durably persisted, if at all --
    /// `None` for the in-memory-only constructors tests use.
    credential_store: Option<CredentialRegistryStore>,
}

/// `fs` must outlive the process: `AuthState` is cloned into axum's
/// `State` extractor, which requires `'static`, the same reason
/// `routes::operations::OperationsState::default_journaled` leaks its
/// `FileSystem` instead of borrowing one with a shorter lifetime.
struct CredentialRegistryStore {
    fs: &'static dyn FileSystem,
    path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenVerifierRecord {
    algorithm: String,
    salt: String,
    hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AuthError {
    Missing,
    Malformed,
    Unknown,
    Expired,
    Revoked,
    SecretStore(String),
    HashMismatch,
    RateLimited,
}

impl AuthState {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new(secret_store: Arc<dyn SecretStore + Send + Sync>) -> Self {
        Self::with_registry_state(secret_store, HashMap::new(), None)
    }

    /// Builds an `AuthState` whose non-secret credential registry is
    /// durable: `registry_path` is read via [`CredentialRepository`] to
    /// reconstruct in-memory state (empty if the file doesn't exist yet --
    /// a fresh install has nothing to reconstruct), and every later
    /// registry mutation ([`Self::issue_credential`],
    /// [`Self::migrate_owner_credential`], the test bootstrap path)
    /// rewrites the same file atomically. Verifier records are not part of
    /// this: they stay in `secret_store`, whatever that is.
    pub fn with_persistent_registry(
        secret_store: Arc<dyn SecretStore + Send + Sync>,
        fs: &'static dyn FileSystem,
        registry_path: impl Into<PathBuf>,
    ) -> Result<Self, CredentialRepositoryError> {
        let path = registry_path.into();
        let entries = CredentialRepository::new(fs, path.clone()).load()?;
        let registry = entries.into_iter().filter_map(entry_to_record).collect();
        Ok(Self::with_registry_state(
            secret_store,
            registry,
            Some(CredentialRegistryStore { fs, path }),
        ))
    }

    /// The real construction path `main.rs` uses: a persistent registry at
    /// [`default_registry_path`], the `MSC2_TEST_BOOTSTRAP_TOKEN` dev
    /// convenience P2.12/P4.5 already relied on, and — the P5.9 addition —
    /// migrating a P5.8 legacy owner token into the Phase 4 credential
    /// model if one is still sitting at `LEGACY_OWNER_TOKEN_SECRET_KEY`.
    /// Idempotent across restarts: migration deletes that legacy key once
    /// it has moved it, so a second call finds nothing left to migrate.
    #[allow(dead_code)]
    pub fn default_persistent_service_store() -> Self {
        let secret_store = production_secret_store().unwrap_or_else(|error| {
            panic!("failed to initialize production secret store: {error}")
        });
        Self::persistent_service_store_with_secret_store(secret_store)
    }

    pub fn persistent_service_store_with_secret_store(
        secret_store: Arc<dyn SecretStore + Send + Sync>,
    ) -> Self {
        let path = default_registry_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("failed to create {}: {error}", parent.display()));
        }
        let fs: &'static dyn FileSystem = Box::leak(Box::new(StdFileSystem));
        let state = Self::with_persistent_registry(secret_store, fs, path)
            .unwrap_or_else(|error| panic!("failed to load credential registry: {error}"));

        if let Ok(token) = std::env::var("MSC2_TEST_BOOTSTRAP_TOKEN") {
            state
                .register_test_bootstrap_token(&token)
                .expect("MSC2_TEST_BOOTSTRAP_TOKEN must have msc2_<id>_<secret> shape");
        }

        if let Some(issued) = state
            .migrate_owner_credential()
            .unwrap_or_else(|error| panic!("failed to migrate legacy owner credential: {error}"))
        {
            println!(
                "msc: migrated the legacy owner API token to credential {} -- new bearer token (shown once): {}",
                issued.credential_id, issued.token
            );
        }

        state
    }

    fn with_registry_state(
        secret_store: Arc<dyn SecretStore + Send + Sync>,
        registry: HashMap<String, CredentialRecord>,
        credential_store: Option<CredentialRegistryStore>,
    ) -> Self {
        Self {
            inner: Arc::new(AuthStateInner {
                secret_store,
                registry: Mutex::new(registry),
                failures: Mutex::new(HashMap::new()),
                audit_events: Mutex::new(Vec::new()),
                credential_store,
            }),
        }
    }

    /// Rewrites the whole persisted registry file from `registry`'s
    /// current contents, or does nothing for an `AuthState` built without
    /// [`Self::with_persistent_registry`]. Called with `registry`'s own
    /// lock already held, so the file on disk and the in-memory map never
    /// observably disagree between two calls.
    fn persist_registry(
        &self,
        registry: &HashMap<String, CredentialRecord>,
    ) -> Result<(), SecretStoreError> {
        let Some(store) = &self.inner.credential_store else {
            return Ok(());
        };
        let entries: Vec<CredentialRegistryEntry> = registry
            .iter()
            .map(|(id, record)| record_to_entry(id, record))
            .collect();
        CredentialRepository::new(store.fs, store.path.clone())
            .save(&entries)
            .map_err(|err| SecretStoreError(err.to_string()))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn issue_credential(
        &self,
        label: impl Into<String>,
        role: CredentialRole,
        permissions: Vec<PermissionCategoryDto>,
        expires_at: Option<SystemTime>,
    ) -> Result<IssuedCredential, SecretStoreError> {
        let issued = self.issue_credential_with_secret(
            label,
            role,
            permissions,
            expires_at,
            random_secret(),
        )?;
        self.record_audit("owner-admin", StatusCode::CREATED, "token_created");
        Ok(issued)
    }

    /// Migrates a P5.8 legacy owner token into the Phase 4 credential
    /// model: `None` if `LEGACY_OWNER_TOKEN_SECRET_KEY` is absent or blank
    /// (nothing to migrate — including the common case where an earlier
    /// call already migrated and deleted it, which is what makes rerunning
    /// this idempotent). Otherwise mints one admin credential whose secret
    /// component *is* the old token — so the value a returning owner
    /// already knows keeps working, just wrapped in the `msc2_<id>_`
    /// envelope every other credential uses — deletes the now-migrated
    /// legacy key, and returns the replacement bearer once.
    ///
    /// `docs/msc2/lifecycle/pairing-phase4.md`'s "do not add a raw-token
    /// parsing fallback" holds structurally: `try_authenticate` never reads
    /// `LEGACY_OWNER_TOKEN_SECRET_KEY` — only the `msc2_<id>_<secret>`
    /// shape authenticates, whether `<secret>` happens to be an old token
    /// or a freshly random one.
    pub fn migrate_owner_credential(&self) -> Result<Option<IssuedCredential>, SecretStoreError> {
        let Some(old_token) = self.inner.secret_store.get(LEGACY_OWNER_TOKEN_SECRET_KEY)? else {
            return Ok(None);
        };
        if old_token.trim().is_empty() {
            return Ok(None);
        }

        let issued = self.issue_credential_with_secret(
            "owner-admin",
            CredentialRole::Admin,
            all_permissions(),
            None,
            old_token,
        )?;
        self.inner
            .secret_store
            .delete(LEGACY_OWNER_TOKEN_SECRET_KEY)?;
        self.record_audit("owner-admin", StatusCode::CREATED, "token_created");
        Ok(Some(issued))
    }

    /// Shared by [`Self::issue_credential`] (fresh random `secret`) and
    /// [`Self::migrate_owner_credential`] (the old legacy token reused as
    /// `secret`): mints `credential_id`, stores its salted verifier hash in
    /// `SecretStore`, records the non-secret registry entry, persists the
    /// registry, and returns the bearer token once.
    fn issue_credential_with_secret(
        &self,
        label: impl Into<String>,
        role: CredentialRole,
        permissions: Vec<PermissionCategoryDto>,
        expires_at: Option<SystemTime>,
        secret: String,
    ) -> Result<IssuedCredential, SecretStoreError> {
        let credential_id = random_hex_id();
        let salt = random_secret_salt();
        let salt_bytes = verifier_salt_bytes(&salt).expect("generated salt is base64url");
        let verifier = TokenVerifierRecord {
            algorithm: HASH_ALGORITHM.to_string(),
            salt,
            hash: hash_secret(&secret, &salt_bytes),
        };

        self.inner.secret_store.set(
            &secret_store_key(&credential_id),
            &serde_json::to_string(&verifier).expect("TokenVerifierRecord serializes"),
        )?;

        {
            let mut registry = self.inner.registry.lock().unwrap();
            registry.insert(
                credential_id.clone(),
                CredentialRecord {
                    label: label.into(),
                    role,
                    permissions,
                    expires_at,
                    revoked: false,
                },
            );
            self.persist_registry(&registry)?;
        }

        Ok(IssuedCredential {
            token: format!("{TOKEN_PREFIX}_{credential_id}_{secret}"),
            credential_id,
        })
    }

    fn register_test_bootstrap_token(&self, token: &str) -> Result<(), SecretStoreError> {
        let (credential_id, secret) = parse_token(token).ok_or_else(|| {
            SecretStoreError("bootstrap token must be msc2_<id>_<secret>".to_string())
        })?;
        let salt = random_secret_salt();
        let salt_bytes = verifier_salt_bytes(&salt).expect("generated salt is base64url");
        let verifier = TokenVerifierRecord {
            algorithm: HASH_ALGORITHM.to_string(),
            salt,
            hash: hash_secret(secret, &salt_bytes),
        };

        self.inner.secret_store.set(
            &secret_store_key(credential_id),
            &serde_json::to_string(&verifier).expect("TokenVerifierRecord serializes"),
        )?;
        {
            let mut registry = self.inner.registry.lock().unwrap();
            registry.insert(
                credential_id.to_string(),
                CredentialRecord {
                    label: "phase4-live-check".to_string(),
                    role: CredentialRole::Admin,
                    permissions: all_permissions(),
                    expires_at: None,
                    revoked: false,
                },
            );
            self.persist_registry(&registry)?;
        }
        Ok(())
    }

    #[cfg(test)]
    fn revoke_for_test(&self, credential_id: &str) {
        if let Some(record) = self.inner.registry.lock().unwrap().get_mut(credential_id) {
            record.revoked = true;
        }
    }

    #[cfg(test)]
    fn audit_events(&self) -> Vec<AuthAuditEvent> {
        self.inner.audit_events.lock().unwrap().clone()
    }

    fn authenticate_headers(
        &self,
        headers: &HeaderMap,
        client_key: &str,
    ) -> Result<AuthenticatedCredential, AuthError> {
        match self.try_authenticate(headers) {
            Ok(credential) => {
                self.inner.failures.lock().unwrap().remove(client_key);
                Ok(credential)
            }
            Err(error) => {
                if matches!(error, AuthError::RateLimited) {
                    self.record_audit("unknown", StatusCode::TOO_MANY_REQUESTS, "rate_limited");
                    return Err(error);
                }
                if self.record_failure_is_limited(client_key) {
                    self.record_audit("unknown", StatusCode::TOO_MANY_REQUESTS, "rate_limited");
                    Err(AuthError::RateLimited)
                } else {
                    let label = match error {
                        AuthError::Missing => "anonymous",
                        _ => "unknown",
                    };
                    self.record_audit(label, StatusCode::UNAUTHORIZED, "unauthorized");
                    Err(error)
                }
            }
        }
    }

    fn try_authenticate(&self, headers: &HeaderMap) -> Result<AuthenticatedCredential, AuthError> {
        let token = bearer_token(headers).ok_or(AuthError::Missing)?;
        let (credential_id, secret) = parse_token(token).ok_or(AuthError::Malformed)?;

        let record = {
            let registry = self.inner.registry.lock().unwrap();
            registry
                .get(credential_id)
                .cloned()
                .ok_or(AuthError::Unknown)?
        };
        if record.revoked {
            return Err(AuthError::Revoked);
        }
        if record
            .expires_at
            .is_some_and(|expires_at| SystemTime::now() >= expires_at)
        {
            return Err(AuthError::Expired);
        }

        let verifier_json = self
            .inner
            .secret_store
            .get(&secret_store_key(credential_id))
            .map_err(|e| AuthError::SecretStore(e.to_string()))?
            .ok_or(AuthError::Unknown)?;
        let verifier: TokenVerifierRecord = serde_json::from_str(&verifier_json)
            .map_err(|e| AuthError::SecretStore(e.to_string()))?;
        if verifier.algorithm != HASH_ALGORITHM {
            return Err(AuthError::SecretStore(format!(
                "unsupported token hash algorithm '{}'",
                verifier.algorithm
            )));
        }
        let salt = verifier_salt_bytes(&verifier.salt)
            .map_err(|e| AuthError::SecretStore(e.to_string()))?;
        let presented_hash = hash_secret(secret, &salt);

        if presented_hash
            .as_bytes()
            .ct_eq(verifier.hash.as_bytes())
            .into()
        {
            Ok(AuthenticatedCredential {
                credential_id: credential_id.to_string(),
                label: record.label,
                role: record.role,
                permissions: record.permissions,
            })
        } else {
            Err(AuthError::HashMismatch)
        }
    }

    fn record_failure_is_limited(&self, client_key: &str) -> bool {
        let now = Instant::now();
        let mut failures = self.inner.failures.lock().unwrap();
        let entries = failures.entry(client_key.to_string()).or_default();
        while entries
            .front()
            .is_some_and(|oldest| now.duration_since(*oldest) > AUTH_FAILURE_WINDOW)
        {
            entries.pop_front();
        }
        entries.push_back(now);
        entries.len() > AUTH_FAILURE_LIMIT
    }

    fn record_audit(&self, token_label: &str, status: StatusCode, code: &str) {
        self.inner
            .audit_events
            .lock()
            .unwrap()
            .push(AuthAuditEvent {
                token_label: token_label.to_string(),
                status_code: status.as_u16(),
                code: code.to_string(),
            });
    }
}

pub async fn require_bearer_token(
    State(auth): State<AuthState>,
    mut request: Request,
    next: Next,
) -> Response {
    match auth.authenticate_headers(request.headers(), "unknown-client") {
        Ok(credential) => {
            request.extensions_mut().insert(credential);
            next.run(request).await
        }
        Err(AuthError::RateLimited) => rate_limited(),
        Err(AuthError::SecretStore(message)) => internal_error(message),
        Err(_) => unauthorized(),
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn parse_token(token: &str) -> Option<(&str, &str)> {
    let mut pieces = token.splitn(3, '_');
    let prefix = pieces.next()?;
    let credential_id = pieces.next()?;
    let secret = pieces.next()?;
    if prefix != TOKEN_PREFIX || credential_id.is_empty() || secret.is_empty() {
        return None;
    }
    Some((credential_id, secret))
}

fn secret_store_key(credential_id: &str) -> String {
    format!("{SECRET_STORE_KEY_PREFIX}{credential_id}")
}

#[cfg_attr(not(test), allow(dead_code))]
fn random_hex_id() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    hex_lower(&bytes)
}

#[cfg_attr(not(test), allow(dead_code))]
fn random_secret() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg_attr(not(test), allow(dead_code))]
fn random_secret_salt() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn verifier_salt_bytes(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    URL_SAFE_NO_PAD.decode(encoded)
}

fn hash_secret(secret: &str, salt: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(salt);
    hasher.update(secret.as_bytes());
    hex_lower(&hasher.finalize())
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProductionSecretStoreKind {
    #[cfg(target_os = "macos")]
    MacosSystemKeychain,
    #[cfg(target_os = "windows")]
    WindowsCredentialManager,
    #[cfg(target_os = "linux")]
    LinuxCredentialHelper,
}

pub(crate) fn production_secret_store()
-> Result<Arc<dyn SecretStore + Send + Sync>, SecretStoreError> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(service) = std::env::var("MSC2_MACOS_USER_KEYCHAIN_SERVICE")
            && !service.is_empty()
        {
            return Ok(Arc::new(
                msc_platform_macos::secret_store::MacosSecretStore::default_keychain_for_service(
                    service,
                )?,
            ));
        }
        Ok(Arc::new(
            msc_platform_macos::secret_store::MacosSecretStore::system()?,
        ))
    }
    #[cfg(target_os = "windows")]
    {
        Ok(Arc::new(
            msc_platform_windows::secret_store::WindowsSecretStore::new(),
        ))
    }
    #[cfg(target_os = "linux")]
    {
        Ok(Arc::new(
            msc_platform_linux::secret_store::LinuxCredentialHelperSecretStore::new(),
        ))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(SecretStoreError(
            "no production SecretStore is available for this target".to_string(),
        ))
    }
}

#[cfg(test)]
fn production_secret_store_kind() -> ProductionSecretStoreKind {
    #[cfg(target_os = "macos")]
    {
        ProductionSecretStoreKind::MacosSystemKeychain
    }
    #[cfg(target_os = "windows")]
    {
        ProductionSecretStoreKind::WindowsCredentialManager
    }
    #[cfg(target_os = "linux")]
    {
        ProductionSecretStoreKind::LinuxCredentialHelper
    }
}

/// Where [`AuthState::default_persistent_service_store`] persists the
/// non-secret credential registry. A deployment may override the whole
/// file path with `MSC2_CREDENTIAL_REGISTRY_PATH`; otherwise the registry
/// lives under the durable app-data root, not the OS temporary directory.
fn default_registry_path() -> PathBuf {
    default_registry_path_from_env(|key| std::env::var_os(key))
}

fn default_registry_path_from_env(mut var: impl FnMut(&str) -> Option<OsString>) -> PathBuf {
    var("MSC2_CREDENTIAL_REGISTRY_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_app_data_dir_from_env(var).join("credential-registry.json"))
}

fn default_app_data_dir_from_env(mut var: impl FnMut(&str) -> Option<OsString>) -> PathBuf {
    if let Some(path) = var("MSC2_DATA_DIR")
        && !path.is_empty()
    {
        return PathBuf::from(path);
    }

    #[cfg(target_os = "macos")]
    {
        let home = var("HOME").unwrap_or_else(|| OsString::from("/Library/Application Support"));
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("MSC2")
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(local_app_data) = var("LOCALAPPDATA") {
            return PathBuf::from(local_app_data).join("MSC2");
        }
        if let Some(app_data) = var("APPDATA") {
            return PathBuf::from(app_data).join("MSC2");
        }
        let profile = var("USERPROFILE").unwrap_or_else(|| OsString::from(r"C:\MSC2"));
        PathBuf::from(profile)
            .join("AppData")
            .join("Local")
            .join("MSC2")
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(xdg) = var("XDG_DATA_HOME")
            && !xdg.is_empty()
        {
            return PathBuf::from(xdg).join("msc2");
        }
        let home = var("HOME").unwrap_or_else(|| OsString::from("/var/lib/msc2"));
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("msc2")
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".msc2")
    }
}

/// `record`'s persisted shape — `role`/`permissions` become plain strings
/// since `msc-infrastructure` (where [`CredentialRegistryEntry`] lives)
/// depends on neither `CredentialRole` nor `PermissionCategoryDto`.
fn record_to_entry(credential_id: &str, record: &CredentialRecord) -> CredentialRegistryEntry {
    CredentialRegistryEntry {
        credential_id: credential_id.to_string(),
        label: record.label.clone(),
        role: role_to_string(record.role),
        permissions: record
            .permissions
            .iter()
            .map(|permission| permission_to_string(*permission))
            .collect(),
        expires_at: record.expires_at.map(system_time_to_unix_secs),
        revoked: record.revoked,
    }
}

/// The inverse of [`record_to_entry`]. `None` for a row whose `role` isn't
/// one this build recognizes — matches
/// `msc_infrastructure::credential_repository`'s own "skip rather than
/// fail the whole load" handling for a malformed row. An unrecognized
/// permission string is dropped from that credential's list instead of
/// invalidating the whole row, since a stricter permission set is a safe
/// direction to fail in.
fn entry_to_record(entry: CredentialRegistryEntry) -> Option<(String, CredentialRecord)> {
    let role = role_from_string(&entry.role)?;
    let permissions = entry
        .permissions
        .into_iter()
        .filter_map(|permission| permission_from_string(&permission))
        .collect();
    Some((
        entry.credential_id,
        CredentialRecord {
            label: entry.label,
            role,
            permissions,
            expires_at: entry.expires_at.map(unix_secs_to_system_time),
            revoked: entry.revoked,
        },
    ))
}

fn role_to_string(role: CredentialRole) -> String {
    serde_json::to_value(role)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .expect("CredentialRole always serializes to a string")
}

fn role_from_string(value: &str) -> Option<CredentialRole> {
    serde_json::from_value(Value::String(value.to_string())).ok()
}

fn permission_to_string(permission: PermissionCategoryDto) -> String {
    serde_json::to_value(permission)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .expect("PermissionCategoryDto always serializes to a string")
}

fn permission_from_string(value: &str) -> Option<PermissionCategoryDto> {
    serde_json::from_value(Value::String(value.to_string())).ok()
}

fn system_time_to_unix_secs(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn unix_secs_to_system_time(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

fn unauthorized() -> Response {
    error_response(
        StatusCode::UNAUTHORIZED,
        "unauthorized",
        "Missing or invalid bearer token.",
    )
}

fn rate_limited() -> Response {
    error_response(
        StatusCode::TOO_MANY_REQUESTS,
        "rate_limited",
        "Too many authentication failures. Try again later.",
    )
}

fn internal_error(message: String) -> Response {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal_error",
        &format!("Authentication store error: {message}"),
    )
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    let body = ErrorDto {
        code: code.to_string(),
        message: message.to_string(),
        help_id: None,
        details: None,
    };
    (status, Json(body)).into_response()
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn all_permissions() -> Vec<PermissionCategoryDto> {
    vec![
        PermissionCategoryDto::ServerControl,
        PermissionCategoryDto::Players,
        PermissionCategoryDto::Settings,
        PermissionCategoryDto::Addons,
        PermissionCategoryDto::Worlds,
        PermissionCategoryDto::Broadcast,
        PermissionCategoryDto::Networking,
        PermissionCategoryDto::Fleet,
        PermissionCategoryDto::Admin,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use std::io;
    use std::path::{Path, PathBuf};

    fn headers_with_bearer(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        headers
    }

    fn test_state() -> AuthState {
        AuthState::new(Arc::new(FakeSecretStore::new()))
    }

    #[test]
    fn auth_real_tokens_accepts_secret_store_backed_token() {
        let state = test_state();
        let issued = state
            .issue_credential(
                "owner-admin",
                CredentialRole::Admin,
                all_permissions(),
                None,
            )
            .unwrap();

        let credential = state
            .authenticate_headers(&headers_with_bearer(&issued.token), "test-client")
            .unwrap();
        assert_eq!(credential.credential_id, issued.credential_id);
        assert_eq!(credential.label, "owner-admin");
        assert_eq!(credential.role, CredentialRole::Admin);
        assert_eq!(credential.permissions, all_permissions());
    }

    #[test]
    fn auth_real_tokens_rejects_dev_token_environment_fallback() {
        unsafe { std::env::set_var("MSC_DEV_TOKEN", "msc2-dev-token") };
        let state = test_state();
        assert!(matches!(
            state.authenticate_headers(&headers_with_bearer("msc2-dev-token"), "test-client"),
            Err(AuthError::Malformed)
        ));
    }

    #[test]
    fn auth_real_tokens_rejects_wrong_secret_for_known_id() {
        let state = test_state();
        let issued = state
            .issue_credential(
                "owner-admin",
                CredentialRole::Admin,
                all_permissions(),
                None,
            )
            .unwrap();
        let wrong_token = format!("msc2_{}_wrong-secret", issued.credential_id);

        assert!(matches!(
            state.authenticate_headers(&headers_with_bearer(&wrong_token), "test-client"),
            Err(AuthError::HashMismatch)
        ));
    }

    #[test]
    fn auth_real_tokens_rejects_missing_secret_store_record() {
        let state = test_state();
        let issued = state
            .issue_credential(
                "owner-admin",
                CredentialRole::Admin,
                all_permissions(),
                None,
            )
            .unwrap();
        state
            .inner
            .secret_store
            .delete(&secret_store_key(&issued.credential_id))
            .unwrap();

        assert!(matches!(
            state.authenticate_headers(&headers_with_bearer(&issued.token), "test-client"),
            Err(AuthError::Unknown)
        ));
    }

    #[test]
    fn auth_real_tokens_rejects_revoked_registry_record() {
        let state = test_state();
        let issued = state
            .issue_credential(
                "owner-admin",
                CredentialRole::Admin,
                all_permissions(),
                None,
            )
            .unwrap();
        state.revoke_for_test(&issued.credential_id);

        assert!(matches!(
            state.authenticate_headers(&headers_with_bearer(&issued.token), "test-client"),
            Err(AuthError::Revoked)
        ));
    }

    #[test]
    fn auth_real_tokens_rate_limits_after_ten_failures() {
        let state = test_state();
        let headers = headers_with_bearer("not-a-real-token");
        for _ in 0..AUTH_FAILURE_LIMIT {
            assert!(matches!(
                state.authenticate_headers(&headers, "same-client"),
                Err(AuthError::Malformed)
            ));
        }

        assert!(matches!(
            state.authenticate_headers(&headers, "same-client"),
            Err(AuthError::RateLimited)
        ));
    }

    #[test]
    fn auth_real_tokens_records_auth_failure_audit_events() {
        let state = test_state();
        let _ = state.authenticate_headers(&HeaderMap::new(), "test-client");

        assert_eq!(
            state.audit_events(),
            vec![AuthAuditEvent {
                token_label: "anonymous".to_string(),
                status_code: 401,
                code: "unauthorized".to_string(),
            }]
        );
    }

    #[test]
    fn auth_real_tokens_supports_guest_and_named_roles() {
        let state = test_state();
        let guest = state
            .issue_credential(
                "guest",
                CredentialRole::Guest,
                vec![PermissionCategoryDto::Players],
                None,
            )
            .unwrap();
        let named = state
            .issue_credential(
                "console-admin",
                CredentialRole::Named,
                vec![PermissionCategoryDto::ServerControl],
                None,
            )
            .unwrap();

        assert_eq!(
            state
                .authenticate_headers(&headers_with_bearer(&guest.token), "guest-client")
                .unwrap()
                .role,
            CredentialRole::Guest
        );
        assert_eq!(
            state
                .authenticate_headers(&headers_with_bearer(&named.token), "named-client")
                .unwrap()
                .role,
            CredentialRole::Named
        );
    }

    #[test]
    fn migrate_owner_credential_is_none_when_nothing_was_extracted() {
        let state = test_state();
        assert_eq!(state.migrate_owner_credential().unwrap(), None);
    }

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "msc2-auth-production-store-{name}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    struct DurableTestSecretStore {
        dir: PathBuf,
    }

    impl DurableTestSecretStore {
        fn new(dir: impl Into<PathBuf>) -> Self {
            Self { dir: dir.into() }
        }

        fn path_for_key(&self, key: &str) -> PathBuf {
            self.dir.join(key)
        }
    }

    impl SecretStore for DurableTestSecretStore {
        fn get(&self, key: &str) -> Result<Option<String>, SecretStoreError> {
            match std::fs::read_to_string(self.path_for_key(key)) {
                Ok(value) => Ok(Some(value)),
                Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
                Err(err) => Err(SecretStoreError(format!("reading {key}: {err}"))),
            }
        }

        fn set(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
            std::fs::create_dir_all(&self.dir)
                .map_err(|err| SecretStoreError(format!("creating secret dir: {err}")))?;
            std::fs::write(self.path_for_key(key), value)
                .map_err(|err| SecretStoreError(format!("writing {key}: {err}")))
        }

        fn delete(&self, key: &str) -> Result<(), SecretStoreError> {
            match std::fs::remove_file(self.path_for_key(key)) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(err) => Err(SecretStoreError(format!("deleting {key}: {err}"))),
            }
        }
    }

    #[test]
    fn auth_production_store_reconstructs_bearer_from_durable_paths() {
        let temp = TempDir::new("reconstructs-bearer");
        let registry_path = temp.path().join("credential-registry.json");
        let secret_dir = temp.path().join("secrets");
        let fs: &'static dyn FileSystem = Box::leak(Box::new(StdFileSystem));

        let (issued_token, issued_id) = {
            let secret_store: Arc<dyn SecretStore + Send + Sync> =
                Arc::new(DurableTestSecretStore::new(&secret_dir));
            let state =
                AuthState::with_persistent_registry(secret_store, fs, &registry_path).unwrap();
            let issued = state
                .issue_credential(
                    "owner-admin",
                    CredentialRole::Admin,
                    all_permissions(),
                    None,
                )
                .unwrap();
            let credential = state
                .authenticate_headers(&headers_with_bearer(&issued.token), "before-restart")
                .unwrap();
            assert_eq!(credential.credential_id, issued.credential_id);
            (issued.token, issued.credential_id)
        };

        let secret_store: Arc<dyn SecretStore + Send + Sync> =
            Arc::new(DurableTestSecretStore::new(&secret_dir));
        let restarted =
            AuthState::with_persistent_registry(secret_store, fs, &registry_path).unwrap();
        let credential = restarted
            .authenticate_headers(&headers_with_bearer(&issued_token), "after-restart")
            .expect("bearer token should authenticate after rebuilding auth state and store");

        assert_eq!(credential.credential_id, issued_id);
        assert_eq!(credential.role, CredentialRole::Admin);
        assert_eq!(credential.permissions, all_permissions());
    }

    #[test]
    fn auth_production_store_factory_is_target_specific_not_fake() {
        let kind = production_secret_store_kind();
        #[cfg(target_os = "macos")]
        assert_eq!(kind, ProductionSecretStoreKind::MacosSystemKeychain);
        #[cfg(target_os = "windows")]
        assert_eq!(kind, ProductionSecretStoreKind::WindowsCredentialManager);
        #[cfg(target_os = "linux")]
        assert_eq!(kind, ProductionSecretStoreKind::LinuxCredentialHelper);
    }

    #[test]
    fn auth_production_store_registry_defaults_to_durable_app_data() {
        let path = default_registry_path_from_env(|key| match key {
            "HOME" => Some(OsString::from("/Users/cameron")),
            "USERPROFILE" => Some(OsString::from(r"C:\Users\cameron")),
            _ => None,
        });

        assert!(path.ends_with("credential-registry.json"));
        assert!(
            !path.starts_with(std::env::temp_dir()),
            "default credential registry must not live under the OS temp dir: {}",
            path.display()
        );
    }

    /// The P5.9 gate: a P5.8-style legacy owner token, sitting in
    /// `SecretStore` at `LEGACY_OWNER_TOKEN_SECRET_KEY`, survives being
    /// migrated into the Phase 4 credential model across a simulated agent
    /// restart, and migrating twice does not mint a second credential.
    #[test]
    fn migrated_owner_credential_survives_restart() {
        use msc_infrastructure::fs::FakeFileSystem;

        let secret_store: Arc<dyn SecretStore + Send + Sync> = Arc::new(FakeSecretStore::new());
        secret_store
            .set(LEGACY_OWNER_TOKEN_SECRET_KEY, "legacy-owner-secret-xyz")
            .unwrap();
        let fs: &'static dyn FileSystem =
            Box::leak(Box::new(FakeFileSystem::new().with_dir("/srv/agent")));
        let registry_path = "/srv/agent/credentials.json";

        // "Before restart": migrate the legacy token into a real credential.
        let state =
            AuthState::with_persistent_registry(secret_store.clone(), fs, registry_path).unwrap();
        let issued = state
            .migrate_owner_credential()
            .unwrap()
            .expect("a legacy owner token was present to migrate");
        assert_eq!(
            issued.token,
            "msc2_".to_string() + &issued.credential_id + "_legacy-owner-secret-xyz"
        );
        assert!(
            secret_store
                .get(LEGACY_OWNER_TOKEN_SECRET_KEY)
                .unwrap()
                .is_none(),
            "the legacy plaintext key must be gone once migrated"
        );

        let credential = state
            .authenticate_headers(&headers_with_bearer(&issued.token), "cli-client")
            .expect("replacement bearer authenticates before restart");
        assert_eq!(credential.role, CredentialRole::Admin);

        // "After restart": a fresh `AuthState` reconstructed from the same
        // registry file and the same `SecretStore`.
        let restarted =
            AuthState::with_persistent_registry(secret_store.clone(), fs, registry_path).unwrap();
        let credential = restarted
            .authenticate_headers(&headers_with_bearer(&issued.token), "cli-client")
            .expect("replacement bearer authenticates after restart");
        assert_eq!(credential.credential_id, issued.credential_id);
        assert_eq!(credential.role, CredentialRole::Admin);
        assert_eq!(credential.permissions, all_permissions());

        // Rerunning migration after restart must be a no-op: the legacy
        // key is already gone, so nothing new is minted.
        assert_eq!(restarted.migrate_owner_credential().unwrap(), None);
        let entries = CredentialRepository::new(fs, registry_path).load().unwrap();
        assert_eq!(
            entries.len(),
            1,
            "rerunning migration must not duplicate the credential"
        );
    }
}
