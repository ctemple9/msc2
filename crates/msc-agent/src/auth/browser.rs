//! Durable, browser-only pairing and session records.
//!
//! These records live in `SecretStore` rather than the public credential
//! registry because they contain verifiers and CSRF material.  The cookie
//! carries only a random session id and secret; it can never be used as a
//! bearer credential or inspected by Svelte.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::http::{HeaderMap, header};
use serde::{Deserialize, Serialize};

use super::{
    AuthState, BrowserSessionAuthentication, CredentialRole, TokenVerifierRecord, constant_time_eq,
    hash_secret, random_hex_id, random_secret, random_secret_salt, verifier_salt_bytes,
};
use msc_api::dto::PermissionCategoryDto;
use msc_infrastructure::secret_store::SecretStoreError;

const PAIRING_TTL: Duration = Duration::from_secs(10 * 60);
const SESSION_IDLE_TTL: Duration = Duration::from_secs(8 * 60 * 60);
const SESSION_ABSOLUTE_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);
const PAIRING_KEY_PREFIX: &str = "remote-api.browser-pairing.";
const SESSION_KEY_PREFIX: &str = "remote-api.browser-session.";
const COOKIE_NAME: &str = "msc2_session";

#[derive(Debug, Clone)]
pub(crate) struct CreateBrowserPairing {
    pub label: String,
    pub role: CredentialRole,
    pub permissions: Vec<PermissionCategoryDto>,
    pub expires_at: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub(crate) struct CreatedBrowserPairing {
    pub pairing_code: String,
    pub expires_at: SystemTime,
}

#[derive(Debug, Clone)]
pub(crate) struct BrowserSession {
    pub session_id: String,
    pub credential_id: String,
    pub cookie_secret: String,
    pub csrf_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BrowserSessionError {
    Unauthorized,
    Consumed,
    Expired,
    Store(String),
}

impl From<SecretStoreError> for BrowserSessionError {
    fn from(error: SecretStoreError) -> Self {
        Self::Store(error.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrowserPairingRecord {
    verifier: TokenVerifierRecord,
    label: String,
    role: CredentialRole,
    permissions: Vec<PermissionCategoryDto>,
    credential_expires_at: Option<u64>,
    expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BrowserSessionRecord {
    verifier: TokenVerifierRecord,
    credential_id: String,
    csrf_token: String,
    idle_expires_at: u64,
    absolute_expires_at: u64,
}

impl AuthState {
    pub(crate) fn create_browser_pairing(
        &self,
        request: CreateBrowserPairing,
    ) -> Result<CreatedBrowserPairing, BrowserSessionError> {
        let id = random_hex_id();
        let secret = random_secret();
        let salt = random_secret_salt();
        let salt_bytes = verifier_salt_bytes(&salt).expect("generated salt is base64url");
        let expires_at = SystemTime::now() + PAIRING_TTL;
        let record = BrowserPairingRecord {
            verifier: TokenVerifierRecord {
                algorithm: super::HASH_ALGORITHM.to_string(),
                salt,
                hash: hash_secret(&secret, &salt_bytes),
            },
            label: request.label,
            role: request.role,
            permissions: request.permissions,
            credential_expires_at: request.expires_at.map(unix_secs),
            expires_at: unix_secs(expires_at),
        };
        self.inner.secret_store.set(
            &pairing_key(&id),
            &serde_json::to_string(&record).expect("browser pairing serializes"),
        )?;
        Ok(CreatedBrowserPairing {
            pairing_code: format!("pair_{id}_{secret}"),
            expires_at,
        })
    }

    pub(crate) fn exchange_browser_pairing(
        &self,
        code: &str,
    ) -> Result<BrowserSession, BrowserSessionError> {
        let (id, secret) = parse_pairing_code(code).ok_or(BrowserSessionError::Unauthorized)?;
        let key = pairing_key(id);
        let record_json = self
            .inner
            .secret_store
            .get(&key)?
            .ok_or(BrowserSessionError::Consumed)?;
        let record: BrowserPairingRecord = serde_json::from_str(&record_json)
            .map_err(|error| BrowserSessionError::Store(error.to_string()))?;
        if SystemTime::now() >= from_unix_secs(record.expires_at) {
            self.inner.secret_store.delete(&key)?;
            return Err(BrowserSessionError::Expired);
        }
        if record.verifier.algorithm != super::HASH_ALGORITHM {
            return Err(BrowserSessionError::Store(
                "unsupported browser pairing verifier".into(),
            ));
        }
        let salt = verifier_salt_bytes(&record.verifier.salt)
            .map_err(|error| BrowserSessionError::Store(error.to_string()))?;
        if !constant_time_eq(&hash_secret(secret, &salt), &record.verifier.hash) {
            return Err(BrowserSessionError::Unauthorized);
        }

        // Delete before issuing the credential. A concurrent or retried
        // exchange therefore observes `pairing_consumed`, never two sessions.
        self.inner.secret_store.delete(&key)?;
        let issued = self
            .issue_credential(
                record.label,
                record.role,
                record.permissions,
                record.credential_expires_at.map(from_unix_secs),
            )
            .map_err(BrowserSessionError::from)?;
        self.create_browser_session(&issued.credential_id)
    }

    fn create_browser_session(
        &self,
        credential_id: &str,
    ) -> Result<BrowserSession, BrowserSessionError> {
        let session_id = random_hex_id();
        let secret = random_secret();
        let salt = random_secret_salt();
        let salt_bytes = verifier_salt_bytes(&salt).expect("generated salt is base64url");
        let now = SystemTime::now();
        let absolute_expires_at = now + SESSION_ABSOLUTE_TTL;
        let idle_expires_at = now + SESSION_IDLE_TTL;
        let csrf_token = random_secret();
        let record = BrowserSessionRecord {
            verifier: TokenVerifierRecord {
                algorithm: super::HASH_ALGORITHM.to_string(),
                salt,
                hash: hash_secret(&secret, &salt_bytes),
            },
            credential_id: credential_id.to_string(),
            csrf_token: csrf_token.clone(),
            idle_expires_at: unix_secs(idle_expires_at),
            absolute_expires_at: unix_secs(absolute_expires_at),
        };
        self.inner.secret_store.set(
            &session_key(&session_id),
            &serde_json::to_string(&record).expect("browser session serializes"),
        )?;
        Ok(BrowserSession {
            session_id,
            credential_id: credential_id.to_string(),
            cookie_secret: secret,
            csrf_token,
        })
    }

    pub(crate) fn authenticate_browser_session(
        &self,
        headers: &HeaderMap,
    ) -> Result<BrowserSession, BrowserSessionError> {
        let cookie = cookie_value(headers, COOKIE_NAME).ok_or(BrowserSessionError::Unauthorized)?;
        let (session_id, secret) =
            parse_session_cookie(cookie).ok_or(BrowserSessionError::Unauthorized)?;
        let key = session_key(session_id);
        let record_json = self
            .inner
            .secret_store
            .get(&key)?
            .ok_or(BrowserSessionError::Unauthorized)?;
        let mut record: BrowserSessionRecord = serde_json::from_str(&record_json)
            .map_err(|error| BrowserSessionError::Store(error.to_string()))?;
        let now = SystemTime::now();
        let absolute = from_unix_secs(record.absolute_expires_at);
        if now >= absolute || now >= from_unix_secs(record.idle_expires_at) {
            self.inner.secret_store.delete(&key)?;
            return Err(BrowserSessionError::Expired);
        }
        if record.verifier.algorithm != super::HASH_ALGORITHM {
            return Err(BrowserSessionError::Store(
                "unsupported browser session verifier".into(),
            ));
        }
        let salt = verifier_salt_bytes(&record.verifier.salt)
            .map_err(|error| BrowserSessionError::Store(error.to_string()))?;
        if !constant_time_eq(&hash_secret(secret, &salt), &record.verifier.hash) {
            return Err(BrowserSessionError::Unauthorized);
        }

        let renewed = std::cmp::min(now + SESSION_IDLE_TTL, absolute);
        record.idle_expires_at = unix_secs(renewed);
        self.inner.secret_store.set(
            &key,
            &serde_json::to_string(&record).expect("browser session serializes"),
        )?;
        Ok(BrowserSession {
            session_id: session_id.to_string(),
            credential_id: record.credential_id,
            cookie_secret: secret.to_string(),
            csrf_token: record.csrf_token,
        })
    }

    pub(crate) fn csrf_for_browser_session(
        &self,
        authentication: &BrowserSessionAuthentication,
    ) -> Result<(String, SystemTime), BrowserSessionError> {
        let record_json = self
            .inner
            .secret_store
            .get(&session_key(&authentication.session_id))?
            .ok_or(BrowserSessionError::Unauthorized)?;
        let record: BrowserSessionRecord = serde_json::from_str(&record_json)
            .map_err(|error| BrowserSessionError::Store(error.to_string()))?;
        if !constant_time_eq(&authentication.csrf_token, &record.csrf_token) {
            return Err(BrowserSessionError::Unauthorized);
        }
        Ok((record.csrf_token, from_unix_secs(record.idle_expires_at)))
    }

    pub(crate) fn revoke_browser_session(
        &self,
        session_id: &str,
    ) -> Result<(), BrowserSessionError> {
        self.inner.secret_store.delete(&session_key(session_id))?;
        Ok(())
    }
}

pub(crate) fn session_cookie(session: &BrowserSession, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!(
        "{COOKIE_NAME}={}_{}; HttpOnly; SameSite=Strict; Path=/v1; Max-Age={}{}",
        session.session_id,
        session.cookie_secret,
        SESSION_IDLE_TTL.as_secs(),
        secure
    )
}

pub(crate) fn cleared_session_cookie(secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!("{COOKIE_NAME}=; HttpOnly; SameSite=Strict; Path=/v1; Max-Age=0{secure}")
}

pub(crate) fn request_has_exact_origin(headers: &HeaderMap) -> bool {
    let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let Some(host) = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    origin == format!("http://{host}") || origin == format!("https://{host}")
}

pub(crate) fn request_uses_https(headers: &HeaderMap) -> bool {
    headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|origin| origin.starts_with("https://"))
}

fn pairing_key(id: &str) -> String {
    format!("{PAIRING_KEY_PREFIX}{id}")
}
fn session_key(id: &str) -> String {
    format!("{SESSION_KEY_PREFIX}{id}")
}

fn parse_pairing_code(code: &str) -> Option<(&str, &str)> {
    let mut parts = code.splitn(3, '_');
    (parts.next()? == "pair").then_some(())?;
    Some((parts.next()?, parts.next()?))
}

fn parse_session_cookie(cookie: &str) -> Option<(&str, &str)> {
    let mut parts = cookie.splitn(2, '_');
    Some((parts.next()?, parts.next()?))
}

fn cookie_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            let (key, value) = part.trim().split_once('=')?;
            (key == name).then_some(value)
        })
}

fn unix_secs(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn from_unix_secs(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}
