//! `SecretStore` (P3.8) for Linux.
//!
//! **Design history, not guessed scope creep:** `docs/msc2/substrate/secret-storage.md`
//! (P3.2) picked `systemd-creds` as the intended backend, per
//! `msc2-engineering.md` §8's own stated preference. Building this step
//! against that choice surfaced that `systemd-creds` does not fit
//! `SecretStore`'s live `get`/`set`/`delete` shape at all, on any machine
//! without a TPM chip -- which includes plain cloud/VM Linux hosts and
//! this project's own CI runners. Confirmed against the `systemd-creds`
//! manpage and several still-open upstream bug reports of others hitting
//! exactly this (`systemd/systemd#30191`, `#33318`, `#36895`): both
//! `systemd-creds encrypt` *and* `... decrypt`, called directly outside a
//! unit's own startup, fail with a permission error unless the caller is
//! root -- there is no unprivileged mode short of `--with-key=null`,
//! which stores the value with no encryption at all. `systemd-creds`'s
//! real design is "systemd itself, running as root, decrypts a fixed
//! list once when a unit starts" -- not "a running service calls this on
//! demand," which is what this trait needs.
//!
//! **Cameron Temple confirmed, 2026-08-01** (see
//! `docs/msc2/substrate/secret-storage.md` §12 for the full record): the
//! real target design is a small privileged helper the installer sets up
//! once, at the same elevated moment it already writes the `systemd`
//! unit file (P3.1) -- the agent talks to it locally whenever it needs a
//! secret, and only the helper ever touches `systemd-creds`. That
//! helper needs its own service registration, which is Phase 4's job
//! (the same line `phase3-scope.md` already draws for the *agent's* own
//! registration applies to a second privileged component) -- so it is
//! not built here.
//!
//! **This module ships the explicitly-labeled v1 stand-in instead:** a
//! plain file per secret, encrypted with a key that belongs to the
//! agent's own installing-user account (P3.1), not root -- so the agent
//! reads and writes it with no elevation at any point, unlike
//! `systemd-creds`. Threat model, stated plainly per `secret-storage.md`
//! §8's own requirement: anything running as that same OS user account
//! can read the key file (`<base>/key`, mode 0600) and decrypt every
//! secret -- weaker than `systemd-creds`' TPM2 mode, but not a new
//! category of exposure, since it is the same "recoverable by anything
//! with this account's access" shape `service-identity.md` already
//! accepts for the installing-user design as a whole. It needs no root
//! at any point, which the confirmed helper design will still have to
//! earn later without weakening this.

use msc_infrastructure::atomic_write::atomic_write;
use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::secret_store::{Result, SecretStore, SecretStoreError};

use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

pub struct LinuxSecretStore {
    base_dir: PathBuf,
}

impl LinuxSecretStore {
    /// Production default: the installing user's own data directory --
    /// `$XDG_DATA_HOME/msc2/secrets`, falling back to
    /// `$HOME/.local/share/msc2/secrets` -- readable and writable by
    /// that account alone, with no elevation, matching P3.1's "agent
    /// runs as the installing user" design.
    pub fn new() -> Self {
        Self::at(default_base_dir())
    }

    /// Builds a store rooted at an arbitrary directory. Exposed so tests
    /// don't share state with (or risk being mistaken for) a real
    /// installation's own secrets -- unlike `systemd-creds`, this
    /// backend has a cheap disposable-instance equivalent: just a
    /// throwaway directory, no root needed to create one.
    pub fn at(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    fn secrets_dir(&self) -> PathBuf {
        self.base_dir.join("secrets")
    }

    fn key_path(&self) -> PathBuf {
        self.base_dir.join("key")
    }

    fn secret_path(&self, key: &str) -> PathBuf {
        self.secrets_dir().join(encode_key(key))
    }

    fn cipher(&self) -> Result<ChaCha20Poly1305> {
        let key_bytes = self.load_or_create_key()?;
        Ok(ChaCha20Poly1305::new(Key::from_slice(&key_bytes)))
    }

    fn load_or_create_key(&self) -> Result<[u8; KEY_LEN]> {
        let path = self.key_path();
        match fs::read(&path) {
            Ok(bytes) if bytes.len() == KEY_LEN => {
                let mut key = [0u8; KEY_LEN];
                key.copy_from_slice(&bytes);
                Ok(key)
            }
            Ok(bytes) => Err(SecretStoreError(format!(
                "{}: key file has {} bytes, expected {KEY_LEN}",
                path.display(),
                bytes.len()
            ))),
            Err(e) if e.kind() == ErrorKind::NotFound => self.create_key(&path),
            Err(e) => Err(SecretStoreError(format!("reading {}: {e}", path.display()))),
        }
    }

    fn create_key(&self, path: &Path) -> Result<[u8; KEY_LEN]> {
        ensure_owner_only_dir(&self.base_dir)?;
        let key = ChaCha20Poly1305::generate_key(&mut OsRng);
        fs::write(path, key.as_slice())
            .map_err(|e| SecretStoreError(format!("writing {}: {e}", path.display())))?;
        set_owner_only_file(path)?;
        let mut out = [0u8; KEY_LEN];
        out.copy_from_slice(key.as_slice());
        Ok(out)
    }
}

impl Default for LinuxSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for LinuxSecretStore {
    fn get(&self, key: &str) -> Result<Option<String>> {
        let path = self.secret_path(key);
        let contents = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(SecretStoreError(format!("reading {key}: {e}"))),
        };
        if contents.len() < NONCE_LEN {
            return Err(SecretStoreError(format!(
                "{key}: stored value is truncated"
            )));
        }
        let (nonce_bytes, ciphertext) = contents.split_at(NONCE_LEN);
        let cipher = self.cipher()?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
            .map_err(|e| SecretStoreError(format!("decrypting {key}: {e}")))?;
        let value = String::from_utf8(plaintext).map_err(|e| {
            SecretStoreError(format!("stored value for {key} is not valid UTF-8: {e}"))
        })?;
        Ok(Some(value))
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        ensure_owner_only_dir(&self.secrets_dir())?;

        let cipher = self.cipher()?;
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, value.as_bytes())
            .map_err(|e| SecretStoreError(format!("encrypting {key}: {e}")))?;

        let mut contents = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        contents.extend_from_slice(&nonce);
        contents.extend_from_slice(&ciphertext);

        let path = self.secret_path(key);
        // set/get/delete is an upsert-shaped API (SecretStore::set's own
        // contract), but atomic_write's rename step overwrites the
        // destination unconditionally either way -- no separate
        // exists-check needed before writing.
        atomic_write(&StdFileSystem, &path, &contents)
            .map_err(|e| SecretStoreError(format!("writing {key}: {e}")))?;
        set_owner_only_file(&path)?;
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<()> {
        let path = self.secret_path(key);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            // Already absent -- `delete`'s own contract (fixture
            // `delete-of-unset-key-is-noop`) is `Ok(())`, not an error.
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(SecretStoreError(format!("deleting {key}: {e}"))),
        }
    }
}

/// `SecretStore` keys are dot-delimited identifiers by convention
/// (`secret-storage.md` §9), but this encodes defensively rather than
/// trust that: hex-encoding every key byte rules out path traversal or
/// any other filesystem-meaningful character reaching the real path,
/// with no new dependency.
fn encode_key(key: &str) -> String {
    key.bytes().map(|b| format!("{b:02x}")).collect()
}

fn default_base_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME")
        && !xdg.is_empty()
    {
        return PathBuf::from(xdg).join("msc2").join("secrets");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(".local/share/msc2/secrets")
}

fn ensure_owner_only_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .map_err(|e| SecretStoreError(format!("creating {}: {e}", path.display())))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|e| SecretStoreError(format!("setting permissions on {}: {e}", path.display())))
}

fn set_owner_only_file(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|e| SecretStoreError(format!("setting permissions on {}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use msc_infrastructure::secret_store::run_contract_fixture;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
    }

    /// A throwaway directory under the OS temp dir, removed when the
    /// guard drops -- this backend's disposable-instance equivalent to
    /// P3.9's throwaway keychain, cheaper here since it needs no special
    /// API to create (just a directory).
    struct TempStore {
        dir: PathBuf,
        store: LinuxSecretStore,
    }

    impl TempStore {
        fn create(name: &str) -> Self {
            let mut dir = std::env::temp_dir();
            dir.push(format!(
                "msc2-secret-store-contract-{name}-{}",
                std::process::id()
            ));
            Self {
                store: LinuxSecretStore::at(dir.clone()),
                dir,
            }
        }
    }

    impl Drop for TempStore {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn secret_store_contract_round_trip_set_then_get() {
        let store = TempStore::create("round-trip-set-then-get");
        run_contract_fixture(&store.store, &fixtures_dir(), "round-trip-set-then-get");
    }

    #[test]
    fn secret_store_contract_get_of_unset_key_returns_none() {
        let store = TempStore::create("get-of-unset-key-returns-none");
        run_contract_fixture(
            &store.store,
            &fixtures_dir(),
            "get-of-unset-key-returns-none",
        );
    }

    #[test]
    fn secret_store_contract_set_overwrites_existing_key() {
        let store = TempStore::create("set-overwrites-existing-key");
        run_contract_fixture(&store.store, &fixtures_dir(), "set-overwrites-existing-key");
    }

    #[test]
    fn secret_store_contract_delete_then_get_returns_none() {
        let store = TempStore::create("delete-then-get-returns-none");
        run_contract_fixture(
            &store.store,
            &fixtures_dir(),
            "delete-then-get-returns-none",
        );
    }

    #[test]
    fn secret_store_contract_delete_of_unset_key_is_noop() {
        let store = TempStore::create("delete-of-unset-key-is-noop");
        run_contract_fixture(&store.store, &fixtures_dir(), "delete-of-unset-key-is-noop");
    }

    #[test]
    fn key_file_and_secrets_dir_are_owner_only() {
        let store = TempStore::create("permissions");
        store.store.set("some.key", "value").unwrap();

        let key_mode = fs::metadata(store.store.key_path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(key_mode, 0o600);

        let dir_mode = fs::metadata(store.store.secrets_dir())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700);
    }
}
