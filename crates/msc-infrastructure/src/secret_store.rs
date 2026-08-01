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
use std::path::Path;
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

/// All five `fixtures/secret-store-contract/*.json` case names, in the
/// order every platform implementation (P3.9 macOS, P3.10 Windows, P3.11
/// Linux) runs them.
pub const CONTRACT_CASES: [&str; 5] = [
    "round-trip-set-then-get",
    "get-of-unset-key-returns-none",
    "set-overwrites-existing-key",
    "delete-then-get-returns-none",
    "delete-of-unset-key-is-noop",
];

/// Runs one `fixtures/secret-store-contract/<case>.json` fixture against
/// `store`, replaying its `input.operations` in order and asserting each
/// against the matching entry in `expected.results`. Shared so every
/// platform implementation is checked against the exact same fixture
/// files P3.8 wrote, not a fresh re-guess of the contract per platform.
///
/// `fixtures_dir` is the repo-root `fixtures/` directory; each caller
/// passes `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")`
/// from its own crate so the path resolves regardless of which crate's
/// `tests/` directory this runs from.
fn load_contract_fixture(fixtures_dir: &Path, case: &str) -> serde_json::Value {
    let path = fixtures_dir
        .join("secret-store-contract")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()))
}

/// Returns the `input.key` a contract fixture exercises, without running
/// it. A store implementation backed by a real, persistent, shared OS
/// facility (Windows Credential Manager, a Linux `systemd-creds` file --
/// unlike macOS's throwaway-keychain-per-test approach, neither has a
/// cheap disposable-instance equivalent) can use this to clear that exact
/// key before and after its own test, so repeated local runs don't leave
/// residue behind or get tripped up by a previous run's leftovers.
pub fn contract_fixture_key(fixtures_dir: &Path, case: &str) -> String {
    let json = load_contract_fixture(fixtures_dir, case);
    json["input"]["key"]
        .as_str()
        .unwrap_or_else(|| panic!("{case}: input.key missing"))
        .to_string()
}

pub fn run_contract_fixture(store: &dyn SecretStore, fixtures_dir: &Path, case: &str) {
    let json = load_contract_fixture(fixtures_dir, case);

    let key = json["input"]["key"]
        .as_str()
        .unwrap_or_else(|| panic!("{case}: input.key missing"));
    let operations = json["input"]["operations"]
        .as_array()
        .unwrap_or_else(|| panic!("{case}: input.operations missing"));
    let results = json["expected"]["results"]
        .as_array()
        .unwrap_or_else(|| panic!("{case}: expected.results missing"));
    assert_eq!(
        operations.len(),
        results.len(),
        "{case}: operations/results length mismatch"
    );

    for (op, expected) in operations.iter().zip(results) {
        match op["op"].as_str() {
            Some("set") => {
                let value = op["value"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{case}: set op missing value"));
                let ok = store.set(key, value).is_ok();
                assert_eq!(
                    ok,
                    expected["ok"].as_bool().unwrap_or(false),
                    "{case}: set op result mismatch"
                );
            }
            Some("get") => {
                let actual = store
                    .get(key)
                    .unwrap_or_else(|e| panic!("{case}: get op returned Err: {e}"));
                let expected_value = expected["value"].as_str().map(str::to_string);
                assert_eq!(actual, expected_value, "{case}: get op result mismatch");
            }
            Some("delete") => {
                let ok = store.delete(key).is_ok();
                assert_eq!(
                    ok,
                    expected["ok"].as_bool().unwrap_or(false),
                    "{case}: delete op result mismatch"
                );
            }
            other => panic!("{case}: unknown op {other:?}"),
        }
    }
}

/// Runs all five contract fixtures against `store`. See
/// [`run_contract_fixture`] for the path convention `fixtures_dir` follows.
pub fn run_contract_fixtures(store: &dyn SecretStore, fixtures_dir: &Path) {
    for case in CONTRACT_CASES {
        run_contract_fixture(store, fixtures_dir, case);
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

    #[test]
    fn shared_contract_runner_passes_against_fake_store() {
        let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
        let store = FakeSecretStore::new();
        run_contract_fixtures(&store, &fixtures_dir);
    }
}
