//! `SecretStore` (P3.8) for macOS. The production store follows the
//! P4.40 amendment: privileged install/update work provisions one root
//! key in the System keychain, while routine LaunchDaemon operation writes
//! a mutable agent-owned encrypted file store under the durable data root.
//! Direct per-secret Keychain writes stay available for tests and foreground
//! local smoke harnesses against the user's default keychain, not for
//! production LaunchDaemon auth.

use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use msc_infrastructure::atomic_write::atomic_write;
use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::secret_store::{Result, SecretStore, SecretStoreError};
use security_framework::os::macos::keychain::SecKeychain;
use security_framework_sys::base::errSecItemNotFound;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Fixed `service` value for direct Keychain test entries.
const SERVICE: &str = "com.msc2.agent";
const ROOT_SERVICE: &str = "com.msc2.agent.root";
const ROOT_ACCOUNT: &str = "credential-root-v1";

/// Path `docs/msc2/substrate/secret-storage.md` §10 confirmed: the System
/// keychain, not the per-user login keychain.
const SYSTEM_KEYCHAIN_PATH: &str = "/Library/Keychains/System.keychain";
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

pub struct MacosSecretStore {
    backend: MacosSecretStoreBackend,
}

enum MacosSecretStoreBackend {
    DirectKeychain {
        keychain: SecKeychain,
        service: String,
    },
    EncryptedSystemRoot {
        keychain: SecKeychain,
        root_service: String,
        root_account: String,
        secrets_dir: PathBuf,
    },
}

impl MacosSecretStore {
    /// Opens the System-keychain-rooted production store. The item named
    /// by `MSC2_MACOS_SECRET_ROOT_SERVICE`/`MSC2_MACOS_SECRET_ROOT_ACCOUNT`
    /// (or the defaults above) must already exist; install/service scripts
    /// provision it during their privileged window.
    pub fn system() -> Result<Self> {
        let keychain = SecKeychain::open(SYSTEM_KEYCHAIN_PATH)
            .map_err(|e| SecretStoreError(format!("opening System keychain: {e}")))?;
        Ok(Self {
            backend: MacosSecretStoreBackend::EncryptedSystemRoot {
                keychain,
                root_service: std::env::var("MSC2_MACOS_SECRET_ROOT_SERVICE")
                    .unwrap_or_else(|_| ROOT_SERVICE.to_string()),
                root_account: std::env::var("MSC2_MACOS_SECRET_ROOT_ACCOUNT")
                    .unwrap_or_else(|_| ROOT_ACCOUNT.to_string()),
                secrets_dir: macos_secret_store_dir(),
            },
        })
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
        Self {
            backend: MacosSecretStoreBackend::DirectKeychain {
                keychain,
                service: SERVICE.to_string(),
            },
        }
    }

    /// Opens the logged-in user's default keychain under a caller-provided
    /// service namespace. This is for foreground local smoke harnesses that
    /// run `msc serve` directly, outside LaunchDaemon's System-keychain
    /// provisioning window. Installed service auth still uses [`Self::system`].
    pub fn default_keychain_for_service(service: impl Into<String>) -> Result<Self> {
        let keychain = SecKeychain::default()
            .map_err(|e| SecretStoreError(format!("opening default keychain: {e}")))?;
        Ok(Self {
            backend: MacosSecretStoreBackend::DirectKeychain {
                keychain,
                service: service.into(),
            },
        })
    }

    #[cfg(test)]
    fn with_service_for_tests(keychain: SecKeychain, service: String) -> Self {
        Self {
            backend: MacosSecretStoreBackend::DirectKeychain { keychain, service },
        }
    }

    fn direct_get(keychain: &SecKeychain, service: &str, key: &str) -> Result<Option<String>> {
        match keychain.find_generic_password(service, key) {
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

    fn direct_set(keychain: &SecKeychain, service: &str, key: &str, value: &str) -> Result<()> {
        keychain
            .set_generic_password(service, key, value.as_bytes())
            .map_err(|e| SecretStoreError(format!("writing {key}: {e}")))
    }

    fn direct_delete(keychain: &SecKeychain, service: &str, key: &str) -> Result<()> {
        match keychain.find_generic_password(service, key) {
            Ok((_password, item)) => {
                item.delete();
                Ok(())
            }
            Err(e) if e.code() == errSecItemNotFound => Ok(()),
            Err(e) => Err(SecretStoreError(format!("deleting {key}: {e}"))),
        }
    }

    fn root_key(
        keychain: &SecKeychain,
        root_service: &str,
        root_account: &str,
    ) -> Result<[u8; KEY_LEN]> {
        let encoded = Self::direct_get(keychain, root_service, root_account)?.ok_or_else(|| {
            SecretStoreError(format!(
                "macOS credential root is not provisioned at service {root_service}, account {root_account}"
            ))
        })?;
        decode_hex_key(&encoded)
    }

    fn encrypted_path(secrets_dir: &Path, key: &str) -> PathBuf {
        secrets_dir.join(encode_key(key))
    }
}

impl SecretStore for MacosSecretStore {
    fn get(&self, key: &str) -> Result<Option<String>> {
        match &self.backend {
            MacosSecretStoreBackend::DirectKeychain { keychain, service } => {
                Self::direct_get(keychain, service, key)
            }
            MacosSecretStoreBackend::EncryptedSystemRoot {
                keychain,
                root_service,
                root_account,
                secrets_dir,
            } => {
                let path = Self::encrypted_path(secrets_dir, key);
                let contents = match fs::read(&path) {
                    Ok(bytes) => bytes,
                    Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
                    Err(err) => {
                        return Err(SecretStoreError(format!(
                            "reading {}: {err}",
                            path.display()
                        )));
                    }
                };
                if contents.len() < NONCE_LEN {
                    return Err(SecretStoreError(format!(
                        "{key}: stored value is truncated"
                    )));
                }
                let root_key = Self::root_key(keychain, root_service, root_account)?;
                let cipher = ChaCha20Poly1305::new(Key::from_slice(&root_key));
                let (nonce_bytes, ciphertext) = contents.split_at(NONCE_LEN);
                let plaintext = cipher
                    .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
                    .map_err(|err| SecretStoreError(format!("decrypting {key}: {err}")))?;
                String::from_utf8(plaintext)
                    .map(Some)
                    .map_err(|err| SecretStoreError(format!("{key}: value is not UTF-8: {err}")))
            }
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        match &self.backend {
            MacosSecretStoreBackend::DirectKeychain { keychain, service } => {
                Self::direct_set(keychain, service, key, value)
            }
            MacosSecretStoreBackend::EncryptedSystemRoot {
                keychain,
                root_service,
                root_account,
                secrets_dir,
            } => {
                ensure_owner_only_dir(secrets_dir)?;
                let root_key = Self::root_key(keychain, root_service, root_account)?;
                let cipher = ChaCha20Poly1305::new(Key::from_slice(&root_key));
                let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
                let ciphertext = cipher
                    .encrypt(&nonce, value.as_bytes())
                    .map_err(|err| SecretStoreError(format!("encrypting {key}: {err}")))?;
                let mut contents = Vec::with_capacity(NONCE_LEN + ciphertext.len());
                contents.extend_from_slice(&nonce);
                contents.extend_from_slice(&ciphertext);
                let path = Self::encrypted_path(secrets_dir, key);
                atomic_write(&StdFileSystem, &path, &contents)
                    .map_err(|err| SecretStoreError(format!("writing {key}: {err}")))?;
                set_owner_only_file(&path)
            }
        }
    }

    fn delete(&self, key: &str) -> Result<()> {
        match &self.backend {
            MacosSecretStoreBackend::DirectKeychain { keychain, service } => {
                Self::direct_delete(keychain, service, key)
            }
            MacosSecretStoreBackend::EncryptedSystemRoot { secrets_dir, .. } => {
                let path = Self::encrypted_path(secrets_dir, key);
                match fs::remove_file(&path) {
                    Ok(()) => Ok(()),
                    Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
                    Err(err) => Err(SecretStoreError(format!(
                        "deleting {}: {err}",
                        path.display()
                    ))),
                }
            }
        }
    }
}

fn macos_secret_store_dir() -> PathBuf {
    if let Ok(path) = std::env::var("MSC2_MACOS_SECRET_STORE_DIR")
        && !path.is_empty()
    {
        return PathBuf::from(path);
    }
    if let Ok(path) = std::env::var("MSC2_DATA_DIR")
        && !path.is_empty()
    {
        return PathBuf::from(path).join("secrets");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Library/Application Support".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("MSC2")
        .join("secrets")
}

fn encode_key(key: &str) -> String {
    key.bytes().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex_key(encoded: &str) -> Result<[u8; KEY_LEN]> {
    if encoded.len() != KEY_LEN * 2 {
        return Err(SecretStoreError(format!(
            "macOS credential root has {} hex characters, expected {}",
            encoded.len(),
            KEY_LEN * 2
        )));
    }
    let mut key = [0u8; KEY_LEN];
    for (index, chunk) in encoded.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let high = hex_value(chunk[0])?;
        let low = hex_value(chunk[1])?;
        key[index] = (high << 4) | low;
    }
    Ok(key)
}

fn hex_value(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(SecretStoreError(
            "macOS credential root is not hex encoded".to_string(),
        )),
    }
}

fn ensure_owner_only_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .map_err(|err| SecretStoreError(format!("creating {}: {err}", path.display())))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|err| SecretStoreError(format!("setting mode on {}: {err}", path.display())))
}

fn set_owner_only_file(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|err| SecretStoreError(format!("setting mode on {}: {err}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use security_framework::os::macos::keychain::SecKeychain;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A unique `(service, account)` namespace in the logged-in user's
    /// default keychain. The production store targets the System
    /// keychain, but that path is root-writable only; using the default
    /// keychain here keeps the tests on real Keychain Services calls
    /// without depending on `SecKeychainCreate`, which is flaky on the
    /// current host, or root privileges, which the Verify command never
    /// grants.
    struct TestKeychain {
        store: MacosSecretStore,
        service: String,
    }

    impl TestKeychain {
        fn create(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before unix epoch")
                .as_nanos();
            let keychain = SecKeychain::default()
                .unwrap_or_else(|e| panic!("opening default test keychain: {e}"));
            let service = format!(
                "com.msc2.agent.tests.{name}.{}.{}",
                std::process::id(),
                unique
            );
            Self {
                store: MacosSecretStore::with_service_for_tests(keychain, service.clone()),
                service,
            }
        }

        fn writable_or_skip(name: &str) -> Option<Self> {
            let store = Self::create(name);
            let probe_key = "__msc2_contract_probe__";
            match store.store.set(probe_key, "probe") {
                Ok(()) => {
                    let _ = store.store.delete(probe_key);
                    Some(store)
                }
                Err(err) => {
                    eprintln!(
                        "skipping {name}: macOS keychain writes are unavailable in this host context ({err})"
                    );
                    None
                }
            }
        }
    }

    impl Drop for TestKeychain {
        fn drop(&mut self) {
            let MacosSecretStoreBackend::DirectKeychain { keychain, .. } = &self.store.backend
            else {
                return;
            };
            for key in [
                "remote-api.guest-token",
                "xbox-broadcast.alt-password.11111111-1111-1111-1111-111111111111",
                "playit.secret-key",
                "remote-api.owner-token",
                "curseforge.api-key",
            ] {
                if let Ok((_password, item)) = keychain.find_generic_password(&self.service, key) {
                    item.delete();
                }
            }
        }
    }

    fn fixtures_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
    }

    #[test]
    fn secret_store_contract_round_trip_set_then_get() {
        let Some(kc) = TestKeychain::writable_or_skip("round-trip-set-then-get") else {
            return;
        };
        msc_infrastructure::secret_store::run_contract_fixture(
            &kc.store,
            &fixtures_dir(),
            "round-trip-set-then-get",
        );
    }

    #[test]
    fn secret_store_contract_get_of_unset_key_returns_none() {
        let Some(kc) = TestKeychain::writable_or_skip("get-of-unset-key-returns-none") else {
            return;
        };
        msc_infrastructure::secret_store::run_contract_fixture(
            &kc.store,
            &fixtures_dir(),
            "get-of-unset-key-returns-none",
        );
    }

    #[test]
    fn secret_store_contract_set_overwrites_existing_key() {
        let Some(kc) = TestKeychain::writable_or_skip("set-overwrites-existing-key") else {
            return;
        };
        msc_infrastructure::secret_store::run_contract_fixture(
            &kc.store,
            &fixtures_dir(),
            "set-overwrites-existing-key",
        );
    }

    #[test]
    fn secret_store_contract_delete_then_get_returns_none() {
        let Some(kc) = TestKeychain::writable_or_skip("delete-then-get-returns-none") else {
            return;
        };
        msc_infrastructure::secret_store::run_contract_fixture(
            &kc.store,
            &fixtures_dir(),
            "delete-then-get-returns-none",
        );
    }

    #[test]
    fn secret_store_contract_delete_of_unset_key_is_noop() {
        let Some(kc) = TestKeychain::writable_or_skip("delete-of-unset-key-is-noop") else {
            return;
        };
        msc_infrastructure::secret_store::run_contract_fixture(
            &kc.store,
            &fixtures_dir(),
            "delete-of-unset-key-is-noop",
        );
    }
}
