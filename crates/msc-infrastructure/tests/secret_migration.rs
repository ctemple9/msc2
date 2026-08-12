//! Port of the 5 fixtures in `fixtures/secret-migration/`:
//! `ConfigManager.swift`'s one-time legacy plaintext-to-Keychain migration
//! (`init`, lines 73-99, P5.8). No MSC 1 test file exercises this path
//! directly — like `config-recovery` (P5.7) before it, every fixture here
//! was characterized straight from source, and each fixture's `source.test`
//! names the migration step itself rather than an XCTest function.

use msc_infrastructure::config_repository::migrate_legacy_secrets;
use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};
use serde_json::Value;
use std::path::Path;

fn load_fixture(case: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/secret-migration")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()))
}

/// Runs `migrate_legacy_secrets` against a fixture's `input.config`,
/// asserting the rewritten config matches `expected.config` exactly and
/// that every `expected.secretWrites` entry landed in the store — no more,
/// no less relevant to the case, since each fixture's own assertions cover
/// the "nothing else was written" side via specific key lookups below.
fn run_fixture(case: &str) -> (Value, FakeSecretStore, Value) {
    let fixture = load_fixture(case);
    let input_config = fixture["input"]["config"].clone();
    let store = FakeSecretStore::new();

    let (rewritten, _outcome) = migrate_legacy_secrets(&store, &input_config)
        .unwrap_or_else(|e| panic!("{case}: migrate_legacy_secrets returned Err: {e}"));

    assert_eq!(
        rewritten, fixture["expected"]["config"],
        "{case}: rewritten config mismatch"
    );

    for write in fixture["expected"]["secretWrites"].as_array().unwrap() {
        let key = write["key"].as_str().unwrap();
        let expected_value = write["value"].as_str().unwrap();
        assert_eq!(
            store.get(key).unwrap().as_deref(),
            Some(expected_value),
            "{case}: expected secret write at {key}"
        );
    }

    (rewritten, store, fixture)
}

#[test]
fn owner_token_and_server_password_migrate() {
    let (_rewritten, store, fixture) = run_fixture("owner-token-and-server-password-migrate");
    let writes = fixture["expected"]["secretWrites"].as_array().unwrap();
    assert_eq!(writes.len(), 2, "both legacy secrets should have migrated");
    let _ = store;
}

#[test]
fn blank_owner_token_is_not_migrated() {
    let (_rewritten, store, _fixture) = run_fixture("blank-owner-token-is-not-migrated");
    assert_eq!(
        store.get("remote-api.owner-token").unwrap(),
        None,
        "a whitespace-only token must not reach SecretStore"
    );
}

#[test]
fn blank_server_password_is_not_migrated() {
    let (_rewritten, store, _fixture) = run_fixture("blank-server-password-is-not-migrated");
    assert_eq!(
        store
            .get("xbox-broadcast.alt-password.22222222-2222-2222-2222-222222222222")
            .unwrap(),
        None,
        "an empty password must not reach SecretStore"
    );
}

#[test]
fn password_migrates_without_owner_token() {
    let (_rewritten, store, _fixture) = run_fixture("password-migrates-without-owner-token");
    assert_eq!(
        store.get("remote-api.owner-token").unwrap(),
        None,
        "no remote_api_token key was present to migrate"
    );
    assert_eq!(
        store
            .get("xbox-broadcast.alt-password.33333333-3333-3333-3333-333333333333")
            .unwrap(),
        Some("another-alt-pw".to_string()),
        "the server password must still migrate independently"
    );
}

#[test]
fn rerunning_cleaned_input_is_a_no_op() {
    let fixture = load_fixture("rerunning-cleaned-input-is-a-no-op");
    let input_config = fixture["input"]["config"].clone();
    let store = FakeSecretStore::new();

    let (rewritten, outcome) =
        migrate_legacy_secrets(&store, &input_config).expect("migrate_legacy_secrets returned Err");

    assert_eq!(rewritten, fixture["expected"]["config"]);
    assert_eq!(
        rewritten, input_config,
        "clean input must come back unchanged"
    );
    assert!(!outcome.owner_token_migrated);
    assert!(outcome.server_passwords_migrated.is_empty());

    // Run it a second time on its own output to prove idempotency directly,
    // not just via a fixture whose input already happens to be clean.
    let (rewritten_again, outcome_again) = migrate_legacy_secrets(&store, &rewritten)
        .expect("second migrate_legacy_secrets call returned Err");
    assert_eq!(rewritten_again, rewritten);
    assert!(!outcome_again.owner_token_migrated);
    assert!(outcome_again.server_passwords_migrated.is_empty());
}
