//! `CredentialRepository`: the durable, non-secret half of the Phase 4/P5.9
//! credential registry `docs/msc2/lifecycle/pairing-phase4.md` describes --
//! "the non-secret registry stores the ... credential id, label, role,
//! permission categories, ... optional expiry, revoked state". Verifier
//! records (the salted hash Phase 4's bearer-token auth checks a presented
//! secret against) stay in `SecretStore`; this module never touches that
//! trait or reads a raw token.
//!
//! Greenfield MSC 2 construction, not a port -- D-019's admin/guest/named
//! multi-credential model has no MSC 1 oracle (MSC 1 keeps exactly two
//! global tokens in Keychain, no registry at all). `role`/`permissions` are
//! stored as plain strings here rather than `msc-agent`'s `CredentialRole`
//! or `msc-api`'s `PermissionCategoryDto`: this crate depends on neither,
//! the same crate-boundary [`crate::operation_journal`] draws around its
//! own `operation_type` field.
//!
//! One JSON file holds the whole registry, unlike
//! [`crate::operation_journal::OperationJournal`]'s one-file-per-entry
//! layout -- the registry is small (one row per issued credential, not one
//! per lifecycle operation) and every reconstruction on startup wants the
//! full set at once, so there is nothing to gain from splitting it.

use crate::atomic_write::{AtomicWriteError, atomic_write};
use crate::fs::FileSystem;
use serde_json::Value;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// One row of the non-secret credential registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialRegistryEntry {
    pub credential_id: String,
    pub label: String,
    pub role: String,
    pub permissions: Vec<String>,
    /// Unix seconds; `None` means "never expires".
    pub expires_at: Option<u64>,
    pub revoked: bool,
}

#[derive(Debug)]
pub enum CredentialRepositoryError {
    Io(io::Error),
    Parse(serde_json::Error),
    Write(AtomicWriteError),
}

impl fmt::Display for CredentialRepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CredentialRepositoryError::Io(err) => write!(f, "{err}"),
            CredentialRepositoryError::Parse(err) => write!(f, "{err}"),
            CredentialRepositoryError::Write(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for CredentialRepositoryError {}

/// Reads and writes one registry file. Like every other Phase 3/4/5
/// primitive built on [`FileSystem`], `path`'s parent directory must
/// already exist -- this type does not create it.
pub struct CredentialRepository<'fs> {
    fs: &'fs dyn FileSystem,
    path: PathBuf,
}

impl<'fs> CredentialRepository<'fs> {
    pub fn new(fs: &'fs dyn FileSystem, path: impl Into<PathBuf>) -> Self {
        Self {
            fs,
            path: path.into(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Every registry entry, or an empty list if `path` doesn't exist yet --
    /// a fresh install has no registry to reconstruct.
    pub fn load(&self) -> Result<Vec<CredentialRegistryEntry>, CredentialRepositoryError> {
        let bytes = match self.fs.read(&self.path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(CredentialRepositoryError::Io(err)),
        };
        let value: Value =
            serde_json::from_slice(&bytes).map_err(CredentialRepositoryError::Parse)?;
        Ok(entries_from_value(&value))
    }

    /// Overwrites the whole registry file via [`atomic_write`] -- a reader
    /// (including a concurrent restart reload) never observes a
    /// half-written registry.
    pub fn save(
        &self,
        entries: &[CredentialRegistryEntry],
    ) -> Result<(), CredentialRepositoryError> {
        let bytes = serde_json::to_vec_pretty(&entries_to_value(entries))
            .expect("registry entries always serialize");
        atomic_write(self.fs, &self.path, &bytes).map_err(CredentialRepositoryError::Write)
    }
}

fn entries_to_value(entries: &[CredentialRegistryEntry]) -> Value {
    Value::Array(entries.iter().map(entry_to_value).collect())
}

fn entry_to_value(entry: &CredentialRegistryEntry) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert(
        "credentialId".to_string(),
        Value::String(entry.credential_id.clone()),
    );
    obj.insert("label".to_string(), Value::String(entry.label.clone()));
    obj.insert("role".to_string(), Value::String(entry.role.clone()));
    obj.insert(
        "permissions".to_string(),
        Value::Array(
            entry
                .permissions
                .iter()
                .map(|p| Value::String(p.clone()))
                .collect(),
        ),
    );
    obj.insert(
        "expiresAt".to_string(),
        entry.expires_at.map(Value::from).unwrap_or(Value::Null),
    );
    obj.insert("revoked".to_string(), Value::Bool(entry.revoked));
    Value::Object(obj)
}

/// Skips any row missing a required field rather than failing the whole
/// load -- matches [`crate::operation_journal`]'s "not a file this module
/// wrote; leave it alone" handling for a corrupt entry.
fn entries_from_value(value: &Value) -> Vec<CredentialRegistryEntry> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(entry_from_value)
        .collect()
}

fn entry_from_value(value: &Value) -> Option<CredentialRegistryEntry> {
    let obj = value.as_object()?;
    let credential_id = obj.get("credentialId")?.as_str()?.to_string();
    let label = obj.get("label")?.as_str()?.to_string();
    let role = obj.get("role")?.as_str()?.to_string();
    let permissions = obj
        .get("permissions")?
        .as_array()?
        .iter()
        .filter_map(|p| p.as_str().map(str::to_string))
        .collect();
    let expires_at = obj
        .get("expiresAt")
        .and_then(|v| if v.is_null() { None } else { v.as_u64() });
    let revoked = obj.get("revoked")?.as_bool()?;
    Some(CredentialRegistryEntry {
        credential_id,
        label,
        role,
        permissions,
        expires_at,
        revoked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::FakeFileSystem;

    fn entry(id: &str) -> CredentialRegistryEntry {
        CredentialRegistryEntry {
            credential_id: id.to_string(),
            label: "owner-admin".to_string(),
            role: "admin".to_string(),
            permissions: vec!["admin".to_string(), "serverControl".to_string()],
            expires_at: None,
            revoked: false,
        }
    }

    #[test]
    fn load_of_missing_file_returns_empty() {
        let fs = FakeFileSystem::new().with_dir("/srv/agent");
        let repo = CredentialRepository::new(&fs, "/srv/agent/credentials.json");
        assert_eq!(repo.load().expect("load"), Vec::new());
    }

    #[test]
    fn save_then_load_round_trips() {
        let fs = FakeFileSystem::new().with_dir("/srv/agent");
        let repo = CredentialRepository::new(&fs, "/srv/agent/credentials.json");
        let entries = vec![entry("cred-1"), entry("cred-2")];

        repo.save(&entries).expect("save");

        assert_eq!(repo.load().expect("load"), entries);
    }

    #[test]
    fn save_overwrites_rather_than_appends() {
        let fs = FakeFileSystem::new().with_dir("/srv/agent");
        let repo = CredentialRepository::new(&fs, "/srv/agent/credentials.json");

        repo.save(&[entry("cred-1")]).expect("save");
        repo.save(&[entry("cred-2")]).expect("save");

        assert_eq!(repo.load().expect("load"), vec![entry("cred-2")]);
    }
}
