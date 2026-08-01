//! `SecretStore` (P3.8) for macOS: Keychain Services, at the System
//! keychain scope `docs/msc2/substrate/secret-storage.md` §10 confirmed
//! -- a `LaunchDaemon` (P3.1, `service-identity.md`) has no login session
//! to unlock a login keychain against, so this targets the System
//! keychain instead, which any locally-running process (daemon or not)
//! can reach.
//!
//! One `(service, account)` pair per `SecretStore` key, mirroring
//! `KeychainManager.swift`'s own generic-password primitive: `service` is
//! fixed to this agent's identifier, `account` is the `SecretStore` key
//! string, so every secret this trait ever stores lands under one
//! service name instead of MSC 1's five bespoke ones.

use msc_infrastructure::secret_store::{Result, SecretStore, SecretStoreError};
use security_framework::os::macos::keychain::SecKeychain;
use security_framework_sys::base::errSecItemNotFound;

/// Fixed `service` value for every item this store writes. `account`
/// carries the actual `SecretStore` key.
const SERVICE: &str = "com.msc2.agent";

/// Path `docs/msc2/substrate/secret-storage.md` §10 confirmed: the System
/// keychain, not the per-user login keychain.
const SYSTEM_KEYCHAIN_PATH: &str = "/Library/Keychains/System.keychain";

pub struct MacosSecretStore {
    keychain: SecKeychain,
}

impl MacosSecretStore {
    /// Opens the System keychain at the confirmed production scope.
    pub fn system() -> Result<Self> {
        let keychain = SecKeychain::open(SYSTEM_KEYCHAIN_PATH)
            .map_err(|e| SecretStoreError(format!("opening System keychain: {e}")))?;
        Ok(Self { keychain })
    }

    /// Wraps an already-open keychain. Production always goes through
    /// [`Self::system`]; this constructor exists so tests can point the
    /// same read/write/delete logic at a throwaway keychain instead --
    /// see the crate's test module for why: an ordinary, unprivileged
    /// process cannot write to the System keychain at all
    /// (`errSecWrPerm`), so exercising the contract fixtures against it
    /// would require running the test binary as root, which the plan's
    /// own Verify line (a plain `cargo nextest run`) does not do.
    pub fn with_keychain(keychain: SecKeychain) -> Self {
        Self { keychain }
    }
}

impl SecretStore for MacosSecretStore {
    fn get(&self, key: &str) -> Result<Option<String>> {
        match self.keychain.find_generic_password(SERVICE, key) {
            Ok((password, _item)) => {
                let value = String::from_utf8(password.as_ref().to_vec()).map_err(|e| {
                    SecretStoreError(format!("stored value for {key} is not valid UTF-8: {e}"))
                })?;
                Ok(Some(value))
            }
            Err(e) if e.code() == errSecItemNotFound => Ok(None),
            Err(e) => Err(SecretStoreError(format!("reading {key}: {e}"))),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        // `SecKeychain::set_generic_password` is already the upsert
        // primitive `SecretStore::set` needs: it looks up the item first
        // and updates it in place if found, adds it if not.
        self.keychain
            .set_generic_password(SERVICE, key, value.as_bytes())
            .map_err(|e| SecretStoreError(format!("writing {key}: {e}")))
    }

    fn delete(&self, key: &str) -> Result<()> {
        match self.keychain.find_generic_password(SERVICE, key) {
            Ok((_password, item)) => {
                item.delete();
                Ok(())
            }
            // Already absent -- `delete`'s own contract (fixture
            // `delete-of-unset-key-is-noop`) is `Ok(())`, not an error.
            Err(e) if e.code() == errSecItemNotFound => Ok(()),
            Err(e) => Err(SecretStoreError(format!("deleting {key}: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use security_framework::os::macos::keychain::CreateOptions;
    use std::path::{Path, PathBuf};

    /// A password-less keychain file in the OS temp directory, deleted
    /// when the guard drops. Lets the contract fixtures exercise the
    /// real `SecKeychainAddGenericPassword`/`SecKeychainFindGenericPassword`/
    /// `SecKeychainItemDelete` calls without needing root or an admin
    /// authorization prompt, which writing to the real System keychain
    /// requires (confirmed empirically: an unprivileged
    /// `set_generic_password` against `/Library/Keychains/System.keychain`
    /// returns `errSecWrPerm`, code -61).
    struct TempKeychain {
        path: PathBuf,
        store: MacosSecretStore,
    }

    impl TempKeychain {
        fn create(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "msc2-secret-store-contract-{name}-{}.keychain",
                std::process::id()
            ));
            let keychain = CreateOptions::new()
                .password("")
                .create(&path)
                .unwrap_or_else(|e| panic!("creating temp keychain at {path:?}: {e}"));
            Self {
                path,
                store: MacosSecretStore::with_keychain(keychain),
            }
        }
    }

    impl Drop for TempKeychain {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn fixtures_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
    }

    #[test]
    fn secret_store_contract_round_trip_set_then_get() {
        let kc = TempKeychain::create("round-trip-set-then-get");
        msc_infrastructure::secret_store::run_contract_fixture(
            &kc.store,
            &fixtures_dir(),
            "round-trip-set-then-get",
        );
    }

    #[test]
    fn secret_store_contract_get_of_unset_key_returns_none() {
        let kc = TempKeychain::create("get-of-unset-key-returns-none");
        msc_infrastructure::secret_store::run_contract_fixture(
            &kc.store,
            &fixtures_dir(),
            "get-of-unset-key-returns-none",
        );
    }

    #[test]
    fn secret_store_contract_set_overwrites_existing_key() {
        let kc = TempKeychain::create("set-overwrites-existing-key");
        msc_infrastructure::secret_store::run_contract_fixture(
            &kc.store,
            &fixtures_dir(),
            "set-overwrites-existing-key",
        );
    }

    #[test]
    fn secret_store_contract_delete_then_get_returns_none() {
        let kc = TempKeychain::create("delete-then-get-returns-none");
        msc_infrastructure::secret_store::run_contract_fixture(
            &kc.store,
            &fixtures_dir(),
            "delete-then-get-returns-none",
        );
    }

    #[test]
    fn secret_store_contract_delete_of_unset_key_is_noop() {
        let kc = TempKeychain::create("delete-of-unset-key-is-noop");
        msc_infrastructure::secret_store::run_contract_fixture(
            &kc.store,
            &fixtures_dir(),
            "delete-of-unset-key-is-noop",
        );
    }
}
