//! `SecretStore`: one keyed trait replacing `KeychainManager.swift`'s five
//! hardcoded read/write/delete method pairs (`readRemoteAPIToken`/
//! `writeRemoteAPIToken`, `readXboxBroadcastAltPassword(forServerId:)`,
//! etc., lines 53-132), each sitting on top of its own generic
//! `(service, account)`-keyed primitive (`read`/`write`/`delete`, lines
//! 162-228). Here, a new secret kind is a new key string, documented in
//! `docs/msc2/substrate/secret-storage.md` section 9, not a new method.
//!
//! Behavior is generalized directly from those three primitives, not
//! invented: `get` on a key that was never set returns `Ok(None)`
//! (`read`'s own miss case, line 162); `set` is an upsert (`write`'s own
//! doc comment, line 184); `delete` on a key that was never set is
//! `Ok(())` (`delete`'s own "already absent" comment, line 221). The five
//! fixtures in `fixtures/secret-store-contract/` pin these down, and every
//! platform implementation (P3.9 macOS, P3.10 Windows, P3.11 Linux) is
//! checked against the same five, not a fresh guess per platform.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;

#[derive(Debug)]
pub struct SecretStoreError(pub String);

impl fmt::Display for SecretStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SecretStoreError {}

pub type Result<T> = std::result::Result<T, SecretStoreError>;

/// A durable key-value store for secrets, backed by whatever the host
/// platform offers (Keychain, Credential Manager, `systemd-creds`). `key`
/// is an opaque string per the naming scheme in
/// `docs/msc2/substrate/secret-storage.md` section 9 — this trait itself
/// has no notion of which secret a key names.
pub trait SecretStore {
    /// `Ok(None)` when `key` was never set — never an error for a plain
    /// miss.
    fn get(&self, key: &str) -> Result<Option<String>>;

    /// Upsert: creates `key` if absent, overwrites it if present.
    fn set(&self, key: &str, value: &str) -> Result<()>;

    /// `Ok(())` whether or not `key` was previously set — deleting an
    /// already-absent key is not an error.
    fn delete(&self, key: &str) -> Result<()>;
}

/// In-memory `SecretStore`, for tests. Satisfies the five contract
/// fixtures today, so the contract is checkable before any platform
/// crate (P3.9-P3.11) exists.
#[derive(Debug, Default)]
pub struct FakeSecretStore {
    values: Mutex<BTreeMap<String, String>>,
}

impl FakeSecretStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for FakeSecretStore {
    fn get(&self, key: &str) -> Result<Option<String>> {
        Ok(self.values.lock().unwrap().get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        self.values
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<()> {
        self.values.lock().unwrap().remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_set_then_get() {
        let store = FakeSecretStore::new();
        store.set("remote-api.owner-token", "tok-abc123").unwrap();
        assert_eq!(
            store.get("remote-api.owner-token").unwrap(),
            Some("tok-abc123".to_string())
        );
    }

    #[test]
    fn get_of_unset_key_returns_none() {
        let store = FakeSecretStore::new();
        assert_eq!(store.get("playit.secret-key").unwrap(), None);
    }

    #[test]
    fn set_overwrites_existing_key() {
        let store = FakeSecretStore::new();
        store.set("curseforge.api-key", "first-key").unwrap();
        store.set("curseforge.api-key", "second-key").unwrap();
        assert_eq!(
            store.get("curseforge.api-key").unwrap(),
            Some("second-key".to_string())
        );
    }

    #[test]
    fn delete_then_get_returns_none() {
        let store = FakeSecretStore::new();
        let key = "xbox-broadcast.alt-password.11111111-1111-1111-1111-111111111111";
        store.set(key, "alt-pw").unwrap();
        store.delete(key).unwrap();
        assert_eq!(store.get(key).unwrap(), None);
    }

    #[test]
    fn delete_of_unset_key_is_noop() {
        let store = FakeSecretStore::new();
        assert!(store.delete("remote-api.guest-token").is_ok());
        assert_eq!(store.get("remote-api.guest-token").unwrap(), None);
    }
}
