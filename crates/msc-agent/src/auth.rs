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
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use axum::Json;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use msc_api::dto::{ErrorDto, PermissionCategoryDto};
use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore, SecretStoreError};
use rand::RngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use subtle::ConstantTimeEq;

const TOKEN_PREFIX: &str = "msc2";
const SECRET_STORE_KEY_PREFIX: &str = "remote-api.token.";
const HASH_ALGORITHM: &str = "sha1-salted-v1";
const AUTH_FAILURE_WINDOW: Duration = Duration::from_secs(60);
const AUTH_FAILURE_LIMIT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub fn empty_service_store() -> Self {
        Self::new(Arc::new(FakeSecretStore::new()))
    }

    pub fn new(secret_store: Arc<dyn SecretStore + Send + Sync>) -> Self {
        Self {
            inner: Arc::new(AuthStateInner {
                secret_store,
                registry: Mutex::new(HashMap::new()),
                failures: Mutex::new(HashMap::new()),
                audit_events: Mutex::new(Vec::new()),
            }),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn issue_credential(
        &self,
        label: impl Into<String>,
        role: CredentialRole,
        permissions: Vec<PermissionCategoryDto>,
        expires_at: Option<SystemTime>,
    ) -> Result<IssuedCredential, SecretStoreError> {
        let credential_id = random_hex_id();
        let secret = random_secret();
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

        self.inner.registry.lock().unwrap().insert(
            credential_id.clone(),
            CredentialRecord {
                label: label.into(),
                role,
                permissions,
                expires_at,
                revoked: false,
            },
        );

        self.record_audit("owner-admin", StatusCode::CREATED, "token_created");

        Ok(IssuedCredential {
            token: format!("{TOKEN_PREFIX}_{credential_id}_{secret}"),
            credential_id,
        })
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
}
