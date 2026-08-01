//! `SecretStore` (P3.8) for Windows: Windows Credential Manager
//! (`CredWriteW`/`CredReadW`/`CredDeleteW`, generic-credential type),
//! which wraps DPAPI for at-rest encryption -- confirming which of
//! `msc2-decisions.md` D-025's two named options this targets, per this
//! step's own charge not to assume one. Persisted with
//! `CRED_PERSIST_LOCAL_MACHINE`, the scope `service-identity.md` §50
//! confirmed: DPAPI's *user*-scope mode (despite the constant's name,
//! this only means "survives across this user's logon sessions," not
//! "shared machine-wide" -- Credential Manager entries are always tied
//! to the calling account). That matches D-025 question 4's confirmed
//! answer exactly, because the service already runs as the installing
//! user (P3.1), not `LocalSystem` -- the same account a normal desktop
//! app would use, so no daemon/session mismatch to design around, unlike
//! macOS's `LaunchDaemon` case.
//!
//! One Credential Manager `TargetName` string per `SecretStore` key,
//! namespaced with a fixed prefix so this agent's entries are
//! identifiable in Credential Manager's UI without colliding with
//! anything else on the machine.

use msc_infrastructure::secret_store::{Result, SecretStore, SecretStoreError};
use std::ptr;
use windows_sys::Win32::Foundation::{ERROR_NOT_FOUND, FILETIME, GetLastError};
use windows_sys::Win32::Security::Credentials::{
    CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree, CredReadW,
    CredWriteW,
};

/// Default `TargetName` namespace for production use.
const TARGET_PREFIX: &str = "MSC2:";

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub struct WindowsSecretStore {
    prefix: String,
}

impl WindowsSecretStore {
    pub fn new() -> Self {
        Self {
            prefix: TARGET_PREFIX.to_string(),
        }
    }

    /// Builds a store under a different `TargetName` namespace. Exposed
    /// so tests exercise the real Win32 calls without writing under the
    /// same target names production uses -- Credential Manager, unlike a
    /// macOS keychain file, has no cheap disposable-instance equivalent
    /// to create per test.
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    fn target_name(&self, key: &str) -> Vec<u16> {
        to_wide(&format!("{}{key}", self.prefix))
    }
}

impl Default for WindowsSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for WindowsSecretStore {
    fn get(&self, key: &str) -> Result<Option<String>> {
        let target = self.target_name(key);
        let mut cred_ptr: *mut CREDENTIALW = ptr::null_mut();

        // SAFETY: `target` is a live, NUL-terminated UTF-16 buffer for
        // the duration of the call; `cred_ptr` is an out-param CredReadW
        // fills in on success and leaves untouched on failure.
        let ok = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut cred_ptr) };
        if ok == 0 {
            // SAFETY: plain FFI call, no preconditions.
            let err = unsafe { GetLastError() };
            return if err == ERROR_NOT_FOUND {
                Ok(None)
            } else {
                Err(SecretStoreError(format!(
                    "reading {key}: Win32 error {err}"
                )))
            };
        }

        // SAFETY: `ok != 0` means CredReadW populated `cred_ptr` with a
        // valid, CredFree-owned `CREDENTIALW`; `CredentialBlob` points to
        // `CredentialBlobSize` readable bytes for as long as it's alive.
        let value = unsafe {
            let cred = &*cred_ptr;
            let blob =
                std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize);
            let value = String::from_utf8(blob.to_vec()).map_err(|e| {
                SecretStoreError(format!("stored value for {key} is not valid UTF-8: {e}"))
            });
            CredFree(cred_ptr as *const _);
            value
        }?;
        Ok(Some(value))
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        let mut target = self.target_name(key);
        let mut blob = value.as_bytes().to_vec();
        let credential = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            Comment: ptr::null_mut(),
            LastWritten: FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: ptr::null_mut(),
            TargetAlias: ptr::null_mut(),
            UserName: ptr::null_mut(),
        };

        // SAFETY: `credential` borrows `target`/`blob`, both alive for
        // the duration of this call. `CredWriteW` is an upsert -- it
        // replaces an existing credential under the same `TargetName`
        // rather than erroring, which is exactly `SecretStore::set`'s
        // own upsert contract.
        let ok = unsafe { CredWriteW(&credential, 0) };
        if ok == 0 {
            // SAFETY: plain FFI call, no preconditions.
            let err = unsafe { GetLastError() };
            return Err(SecretStoreError(format!(
                "writing {key}: Win32 error {err}"
            )));
        }
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<()> {
        let target = self.target_name(key);

        // SAFETY: `target` is a live, NUL-terminated UTF-16 buffer for
        // the duration of the call.
        let ok = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if ok != 0 {
            return Ok(());
        }
        // SAFETY: plain FFI call, no preconditions.
        let err = unsafe { GetLastError() };
        if err == ERROR_NOT_FOUND {
            // Already absent -- `delete`'s own contract (fixture
            // `delete-of-unset-key-is-noop`) is `Ok(())`, not an error.
            Ok(())
        } else {
            Err(SecretStoreError(format!(
                "deleting {key}: Win32 error {err}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use msc_infrastructure::secret_store::{contract_fixture_key, run_contract_fixture};
    use std::path::{Path, PathBuf};

    /// A namespace no production key ever uses, so these tests can never
    /// collide with (or be mistaken for) a real stored secret.
    const TEST_PREFIX: &str = "MSC2-contract-test:";

    fn fixtures_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
    }

    /// Deletes the fixture's key before running it (so a previous run
    /// crashing mid-test can't leave a stale value that makes this run's
    /// "key was never set" case fail) and after (so repeated local runs
    /// on a real Windows machine don't accumulate entries in Credential
    /// Manager) -- unlike the macOS crate's throwaway keychain file,
    /// Credential Manager has no cheap disposable-instance equivalent to
    /// create fresh per test.
    fn run_case(case: &str) {
        let store = WindowsSecretStore::with_prefix(TEST_PREFIX);
        let dir = fixtures_dir();
        let key = contract_fixture_key(&dir, case);
        let _ = store.delete(&key);
        run_contract_fixture(&store, &dir, case);
        let _ = store.delete(&key);
    }

    #[test]
    fn secret_store_contract_round_trip_set_then_get() {
        run_case("round-trip-set-then-get");
    }

    #[test]
    fn secret_store_contract_get_of_unset_key_returns_none() {
        run_case("get-of-unset-key-returns-none");
    }

    #[test]
    fn secret_store_contract_set_overwrites_existing_key() {
        run_case("set-overwrites-existing-key");
    }

    #[test]
    fn secret_store_contract_delete_then_get_returns_none() {
        run_case("delete-then-get-returns-none");
    }

    #[test]
    fn secret_store_contract_delete_of_unset_key_is_noop() {
        run_case("delete-of-unset-key-is-noop");
    }
}
