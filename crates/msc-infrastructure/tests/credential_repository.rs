//! Hand-written coverage of `CredentialRepository`, in the same style
//! P4.9's `crates/msc-infrastructure/tests/operation_journal.rs` used: no
//! MSC 1 fixture to port from (greenfield MSC 2 construction -- see the
//! module's own doc comment), so these exercise the implementation
//! directly against its own written contract.
//!
//! Test functions are prefixed `credential_repository_` so the plan's
//! Verify command (a plain nextest substring filter) selects all of them.

use msc_infrastructure::credential_repository::{CredentialRegistryEntry, CredentialRepository};
use msc_infrastructure::fs::FakeFileSystem;

const PATH: &str = "/srv/agent/credentials.json";

fn owner_entry() -> CredentialRegistryEntry {
    CredentialRegistryEntry {
        credential_id: "cred-owner".to_string(),
        label: "owner-admin".to_string(),
        role: "admin".to_string(),
        permissions: vec!["admin".to_string(), "serverControl".to_string()],
        expires_at: None,
        revoked: false,
    }
}

fn guest_entry() -> CredentialRegistryEntry {
    CredentialRegistryEntry {
        credential_id: "cred-guest".to_string(),
        label: "guest".to_string(),
        role: "guest".to_string(),
        permissions: vec!["players".to_string()],
        expires_at: Some(1_800_000_000),
        revoked: true,
    }
}

#[test]
fn credential_repository_load_of_missing_file_returns_empty() {
    let fs = FakeFileSystem::new().with_dir("/srv/agent");
    let repo = CredentialRepository::new(&fs, PATH);

    assert_eq!(repo.load().expect("load"), Vec::new());
}

#[test]
fn credential_repository_save_then_load_round_trips_every_field() {
    let fs = FakeFileSystem::new().with_dir("/srv/agent");
    let repo = CredentialRepository::new(&fs, PATH);
    let entries = vec![owner_entry(), guest_entry()];

    repo.save(&entries).expect("save");

    assert_eq!(repo.load().expect("load"), entries);
}

#[test]
fn credential_repository_save_is_a_full_overwrite_not_an_append() {
    let fs = FakeFileSystem::new().with_dir("/srv/agent");
    let repo = CredentialRepository::new(&fs, PATH);

    repo.save(&[owner_entry()]).expect("first save");
    repo.save(&[guest_entry()]).expect("second save");

    assert_eq!(repo.load().expect("load"), vec![guest_entry()]);
}

#[test]
fn credential_repository_survives_a_second_repository_over_the_same_file() {
    // Simulates the msc-agent restart scenario one layer down: a fresh
    // `CredentialRepository` value, constructed after the first one is
    // dropped but pointed at the same `fs`/`path`, sees exactly what was
    // last saved.
    let fs = FakeFileSystem::new().with_dir("/srv/agent");
    {
        let repo = CredentialRepository::new(&fs, PATH);
        repo.save(&[owner_entry()]).expect("save");
    }

    let reloaded = CredentialRepository::new(&fs, PATH);
    assert_eq!(reloaded.load().expect("load"), vec![owner_entry()]);
}
