//! Desktop pairing records share the normal credential registry rather than
//! creating a second desktop-only token authority.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::{
    AuthState, CredentialRole, IssuedCredential, TokenVerifierRecord, constant_time_eq,
    hash_secret, random_hex_id, random_secret, random_secret_salt, verifier_salt_bytes,
};
use msc_api::dto::PermissionCategoryDto;
use msc_infrastructure::secret_store::SecretStoreError;

const PAIRING_TTL: Duration = Duration::from_secs(10 * 60);
const PAIRING_KEY_PREFIX: &str = "remote-api.desktop-pairing.";
const AGENT_HOST_ID_KEY: &str = "remote-api.agent-host-id";

#[derive(Debug, Clone)]
pub(crate) struct CreateDesktopPairing {
    pub label: String,
    pub role: CredentialRole,
    pub permissions: Vec<PermissionCategoryDto>,
    pub expires_at: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub(crate) struct CreatedDesktopPairing {
    pub pairing_code: String,
    pub agent_host_id: String,
    pub expires_at: SystemTime,
}

#[derive(Debug, Clone)]
pub(crate) struct DesktopCredential {
    pub agent_host_id: String,
    pub issued: IssuedCredential,
    pub expires_at: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DesktopPairingError {
    Unauthorized,
    Consumed,
    Expired,
    Store(String),
}

impl From<SecretStoreError> for DesktopPairingError {
    fn from(error: SecretStoreError) -> Self {
        Self::Store(error.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DesktopPairingRecord {
    verifier: TokenVerifierRecord,
    label: String,
    role: CredentialRole,
    permissions: Vec<PermissionCategoryDto>,
    credential_expires_at: Option<u64>,
    expires_at: u64,
}

impl AuthState {
    /// Returns an opaque installation identity. It is intentionally neither a
    /// network address nor a display label: a desktop secret follows this
    /// value when an administrator changes an agent's address.
    pub(crate) fn agent_host_id(&self) -> Result<String, DesktopPairingError> {
        if let Some(host_id) = self.inner.secret_store.get(AGENT_HOST_ID_KEY)?
            && !host_id.trim().is_empty()
        {
            return Ok(host_id);
        }
        let host_id = format!("agent_{}", random_hex_id());
        self.inner.secret_store.set(AGENT_HOST_ID_KEY, &host_id)?;
        Ok(host_id)
    }

    pub(crate) fn create_desktop_pairing(
        &self,
        request: CreateDesktopPairing,
    ) -> Result<CreatedDesktopPairing, DesktopPairingError> {
        let id = random_hex_id();
        let secret = random_secret();
        let salt = random_secret_salt();
        let salt_bytes = verifier_salt_bytes(&salt).expect("generated salt is base64url");
        let expires_at = SystemTime::now() + PAIRING_TTL;
        let record = DesktopPairingRecord {
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
            &serde_json::to_string(&record).expect("desktop pairing serializes"),
        )?;
        Ok(CreatedDesktopPairing {
            pairing_code: format!("pair_{id}_{secret}"),
            agent_host_id: self.agent_host_id()?,
            expires_at,
        })
    }

    /// Consumes a desktop-only code before issuing the ordinary bearer
    /// credential. A retry therefore cannot receive a second token.
    pub(crate) fn exchange_desktop_pairing(
        &self,
        code: &str,
    ) -> Result<DesktopCredential, DesktopPairingError> {
        let (id, secret) = parse_pairing_code(code).ok_or(DesktopPairingError::Unauthorized)?;
        let key = pairing_key(id);
        let record_json = self
            .inner
            .secret_store
            .get(&key)?
            .ok_or(DesktopPairingError::Consumed)?;
        let record: DesktopPairingRecord = serde_json::from_str(&record_json)
            .map_err(|error| DesktopPairingError::Store(error.to_string()))?;
        if SystemTime::now() >= from_unix_secs(record.expires_at) {
            self.inner.secret_store.delete(&key)?;
            return Err(DesktopPairingError::Expired);
        }
        if record.verifier.algorithm != super::HASH_ALGORITHM {
            return Err(DesktopPairingError::Store(
                "unsupported desktop pairing verifier".into(),
            ));
        }
        let salt = verifier_salt_bytes(&record.verifier.salt)
            .map_err(|error| DesktopPairingError::Store(error.to_string()))?;
        if !constant_time_eq(&hash_secret(secret, &salt), &record.verifier.hash) {
            return Err(DesktopPairingError::Unauthorized);
        }

        self.inner.secret_store.delete(&key)?;
        let expires_at = record.credential_expires_at.map(from_unix_secs);
        let issued = self
            .issue_credential(record.label, record.role, record.permissions, expires_at)
            .map_err(DesktopPairingError::from)?;
        Ok(DesktopCredential {
            agent_host_id: self.agent_host_id()?,
            issued,
            expires_at,
        })
    }
}

fn pairing_key(id: &str) -> String {
    format!("{PAIRING_KEY_PREFIX}{id}")
}

fn parse_pairing_code(code: &str) -> Option<(&str, &str)> {
    let mut parts = code.splitn(3, '_');
    (parts.next()? == "pair").then_some(())?;
    Some((parts.next()?, parts.next()?))
}

fn unix_secs(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn from_unix_secs(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}
