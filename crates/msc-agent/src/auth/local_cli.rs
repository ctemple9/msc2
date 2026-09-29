//! Shared policy for the host-local CLI exchange.
//!
//! Platform listeners must obtain both identities from OS APIs: the peer of
//! the accepted local IPC connection and the account running this agent. No
//! field in an IPC message or TCP request may supply either identity. The
//! listeners in P17.4-P17.6 call `issue_for_local_cli_peer` only after that
//! verification.

use std::collections::hash_map::Entry;
use std::time::{Duration, Instant};

use subtle::ConstantTimeEq;

use super::{
    AuthError, AuthState, AuthenticatedCredential, CredentialRole, IssuedCredential, TOKEN_PREFIX,
    all_permissions, hash_secret, random_hex_id, random_secret, random_secret_salt,
    verifier_salt_bytes,
};

pub(super) const CREDENTIAL_ID_PREFIX: &str = "cli";
const CREDENTIAL_LIFETIME: Duration = Duration::from_secs(5 * 60);
const MAX_LIVE_CREDENTIALS: usize = 128;

/// An OS account identifier obtained by a platform adapter, never by parsing
/// a claimed user name from the client. Windows adapters use canonical SID
/// strings returned by Windows, not display names or environment variables.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum LocalOsAccount {
    UnixUid(u32),
    WindowsSid(String),
}

impl LocalOsAccount {
    fn audit_label(&self) -> String {
        match self {
            Self::UnixUid(uid) => format!("cli:uid:{uid}"),
            Self::WindowsSid(sid) => format!("cli:sid:{sid}"),
        }
    }

    fn is_service_account(&self) -> bool {
        match self {
            Self::UnixUid(uid) => *uid == 0,
            Self::WindowsSid(sid) => sid.eq_ignore_ascii_case("S-1-5-18"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum LocalCliExchangeError {
    Unauthorized,
    Busy,
}

pub(super) struct LocalCredentialRecord {
    pub(super) label: String,
    pub(super) salt: String,
    pub(super) hash: String,
    pub(super) expires_at: Instant,
}

impl AuthState {
    /// Called only by an OS-verified local IPC listener. The listener must
    /// obtain `peer` from the accepted connection and `agent_account` from
    /// the running service process. Equality authorizes the installing user,
    /// including that user's SSH login shell, but rejects other local users.
    #[allow(dead_code)]
    pub(crate) fn issue_for_local_cli_peer(
        &self,
        peer: LocalOsAccount,
        agent_account: LocalOsAccount,
    ) -> Result<IssuedCredential, LocalCliExchangeError> {
        if peer != agent_account || peer.is_service_account() {
            self.record_audit(
                "unknown",
                axum::http::StatusCode::FORBIDDEN,
                "cli_peer_denied",
            );
            return Err(LocalCliExchangeError::Unauthorized);
        }

        let secret = random_secret();
        let salt = random_secret_salt();
        let salt_bytes = verifier_salt_bytes(&salt).expect("generated salt is base64url");
        let mut credentials = self.inner.local_cli_credentials.lock().unwrap();
        let now = Instant::now();
        credentials.retain(|_, record| record.expires_at > now);
        if credentials.len() >= MAX_LIVE_CREDENTIALS {
            return Err(LocalCliExchangeError::Busy);
        }
        let credential_id = loop {
            let id = format!("{CREDENTIAL_ID_PREFIX}{}", random_hex_id());
            if let Entry::Vacant(entry) = credentials.entry(id.clone()) {
                entry.insert(LocalCredentialRecord {
                    label: peer.audit_label(),
                    salt,
                    hash: hash_secret(&secret, &salt_bytes),
                    expires_at: now + CREDENTIAL_LIFETIME,
                });
                break id;
            }
        };
        self.record_audit(
            &peer.audit_label(),
            axum::http::StatusCode::CREATED,
            "cli_token_issued",
        );
        Ok(IssuedCredential {
            token: format!("{TOKEN_PREFIX}_{credential_id}_{secret}"),
            credential_id,
        })
    }

    pub(super) fn authenticate_local_cli_credential(
        &self,
        credential_id: &str,
        secret: &str,
    ) -> Result<AuthenticatedCredential, AuthError> {
        let credentials = self.inner.local_cli_credentials.lock().unwrap();
        let record = credentials.get(credential_id).ok_or(AuthError::Unknown)?;
        if Instant::now() >= record.expires_at {
            return Err(AuthError::Expired);
        }
        let salt = verifier_salt_bytes(&record.salt)
            .map_err(|error| AuthError::SecretStore(error.to_string()))?;
        let presented_hash = hash_secret(secret, &salt);
        if !bool::from(presented_hash.as_bytes().ct_eq(record.hash.as_bytes())) {
            return Err(AuthError::HashMismatch);
        }
        Ok(AuthenticatedCredential {
            credential_id: credential_id.to_string(),
            label: record.label.clone(),
            role: CredentialRole::Admin,
            permissions: all_permissions(),
        })
    }

    pub(super) fn local_cli_identity(
        &self,
        credential_id: &str,
    ) -> Option<AuthenticatedCredential> {
        let credentials = self.inner.local_cli_credentials.lock().unwrap();
        let record = credentials.get(credential_id)?;
        if Instant::now() >= record.expires_at {
            return None;
        }
        Some(AuthenticatedCredential {
            credential_id: credential_id.to_string(),
            label: record.label.clone(),
            role: CredentialRole::Admin,
            permissions: all_permissions(),
        })
    }
}
