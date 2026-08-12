//! Proves `load_app_config`/`save_app_config` (P5.6) compose P5.4/P5.5's
//! typed `AppConfig` schema with the generic `load_config`/`save_config`
//! primitive (P3.7): normal load, atomic save, the full malformed-JSON
//! recovery (byte-for-byte `.corrupt-*` copy, then the original path
//! replaced with defaults), and the port-range clamp `ConfigManager.init`
//! applies after decode (source lines 101-104).
//!
//! The three `config_corpus_dimensions_*` tests at the bottom of this file
//! are P5.23's matrix entries for dimensions that had no executable case
//! anywhere before this step: a present-but-wrong-typed field reaching the
//! same R3 recovery a syntax failure does, an unrecognized top-level field
//! not surviving a load_app_config/save_app_config cycle (the typed-schema
//! counterpart to `config_lifecycle.rs`'s
//! `unknown_fields_survive_read_modify_write`, which pins the *opposite*
//! behavior for the untyped `load_config`/`save_config` primitive), and a
//! rename-step interruption exercised through a real `save_app_config`
//! call rather than simulated below the primitive. See
//! `fixtures/config-corpus-dimensions/` for the full matrix, including the
//! dimensions this file's other tests already cover.
//!
//! The last two tests port the two *portable* fixtures P5.4 left in
//! `fixtures/config-roundtrip/` for this step (`r3-corrupt-file-algorithm`,
//! `r3-corrupt-file-does-not-wipe-original`) — both originally hand-simulated
//! in Swift because `ConfigManager.shared` is a singleton that can't be
//! driven directly in a test. Here they run through the real, composed
//! `load_app_config` entrypoint instead of a hand-simulation. The third
//! fixture in that directory,
//! `config-manager-corrupt-config-copy-path-is-nil-on-normal-load`, is a
//! live sanity check against the real `ConfigManager.shared` singleton on
//! whatever machine runs the Swift suite — its own `notes` field says
//! plainly it isn't "a reproducible unit-test scenario" and flags itself
//! for review rather than being treated as equivalent to the other six.
//! Deliberately not ported here for that reason.

use msc_domain::app_config_schema::AppConfig;
use msc_infrastructure::atomic_write::temp_path_for;
use msc_infrastructure::config_repository::{
    ConfigSaveError, corrupt_backup_path, load_app_config, save_app_config,
};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TMP_ROOT: &str = "/private/tmp/msc2-fixture-app-config-repository";

fn resolve_tmp(s: &str) -> String {
    s.replace("<TMP>", TMP_ROOT)
}

struct Fixture {
    input: Value,
}

fn load_roundtrip_fixture(case: &str) -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/config-roundtrip")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    let json: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()));
    Fixture {
        input: json["input"].clone(),
    }
}

fn fixed_now() -> SystemTime {
    UNIX_EPOCH + Duration::from_nanos(1_700_000_000_000_000_000)
}

#[test]
fn app_config_repository_normal_load_returns_typed_config() {
    let path = PathBuf::from("/srv/msc2/server_config_swift.json");
    let raw = r#"{
        "schemaVersion": 1,
        "config_version": 1,
        "java_path": "/usr/bin/java",
        "servers_root": "/srv/msc2/servers",
        "remote_api_port": 25585
    }"#;
    let fs = FakeFileSystem::new().with_file(path.clone(), raw.as_bytes().to_vec(), false);
    let defaults = AppConfig::default_config("/srv/msc2/servers");

    let outcome = load_app_config(&fs, &path, &defaults, fixed_now())
        .unwrap_or_else(|e| panic!("load_app_config failed: {e}"));

    assert!(
        outcome.corrupt_backup_path.is_none(),
        "a clean file must not produce a corrupt backup"
    );
    assert_eq!(outcome.config.java_path, "/usr/bin/java");
    assert_eq!(outcome.config.servers_root, "/srv/msc2/servers");
    assert_eq!(outcome.config.remote_api_port, 25585);
}

#[test]
fn app_config_repository_save_then_load_round_trips() {
    let path = PathBuf::from("/srv/msc2/server_config_swift.json");
    let fs = FakeFileSystem::new().with_dir("/srv/msc2");
    let defaults = AppConfig::default_config("/srv/msc2/servers");

    let mut config = defaults.clone();
    config.java_path = "/opt/java21/bin/java".to_string();
    config.remote_api_port = 25585;
    config.has_shown_handbook = true;

    save_app_config(&fs, &path, &config).unwrap_or_else(|e| panic!("save_app_config failed: {e}"));

    // `save_config`'s own invariant (P3.7) is satisfied: the encoded value
    // on disk carries `schemaVersion`, even though `AppConfig` itself never
    // writes that literal key -- see this crate's `config_repository`
    // module doc comment.
    let on_disk: Value = serde_json::from_slice(&fs.read(&path).unwrap()).unwrap();
    assert_eq!(on_disk["schemaVersion"], Value::from(config.config_version));
    assert_eq!(
        on_disk["config_version"],
        Value::from(config.config_version)
    );

    let outcome = load_app_config(&fs, &path, &defaults, fixed_now())
        .unwrap_or_else(|e| panic!("load_app_config failed: {e}"));
    assert!(outcome.corrupt_backup_path.is_none());
    assert_eq!(outcome.config, config);
}

#[test]
fn app_config_repository_malformed_json_backs_up_then_replaces_original_with_defaults() {
    let path = PathBuf::from("/srv/msc2/server_config_swift.json");
    let garbage = b"this is not json {{{ :::".to_vec();
    let fs = FakeFileSystem::new().with_file(path.clone(), garbage.clone(), false);
    let defaults = AppConfig::default_config("/srv/msc2/servers");
    let now = fixed_now();

    let outcome = load_app_config(&fs, &path, &defaults, now)
        .unwrap_or_else(|e| panic!("load_app_config failed: {e}"));

    let expected_backup = corrupt_backup_path(&path, now);
    assert_eq!(outcome.corrupt_backup_path, Some(expected_backup.clone()));

    // Byte-for-byte: the backup preserves exactly what was on disk before
    // recovery ran, not a re-serialization of it.
    assert_eq!(fs.read(&expected_backup).unwrap(), garbage);

    // The original path is *replaced* with defaults once recovery
    // completes -- distinct from the backup-preservation assertion above.
    assert_eq!(outcome.config, defaults);
    let on_disk: Value = serde_json::from_slice(&fs.read(&path).unwrap()).unwrap();
    assert_eq!(
        on_disk["config_version"],
        Value::from(defaults.config_version)
    );
}

#[test]
fn app_config_repository_clamps_out_of_range_remote_api_port_to_default() {
    let path = PathBuf::from("/srv/msc2/server_config_swift.json");
    let raw = r#"{ "schemaVersion": 1, "config_version": 1, "remote_api_port": 70000 }"#;
    let fs = FakeFileSystem::new().with_file(path.clone(), raw.as_bytes().to_vec(), false);
    let defaults = AppConfig::default_config("/srv/msc2/servers");

    let outcome = load_app_config(&fs, &path, &defaults, fixed_now())
        .unwrap_or_else(|e| panic!("load_app_config failed: {e}"));

    assert_eq!(
        outcome.config.remote_api_port,
        AppConfig::DEFAULT_REMOTE_API_PORT
    );

    // A port of 0 is out of MSC 1's `1...65535` range too, not just "too high".
    let raw_zero = r#"{ "schemaVersion": 1, "config_version": 1, "remote_api_port": 0 }"#;
    let fs_zero =
        FakeFileSystem::new().with_file(path.clone(), raw_zero.as_bytes().to_vec(), false);
    let outcome_zero = load_app_config(&fs_zero, &path, &defaults, fixed_now())
        .unwrap_or_else(|e| panic!("load_app_config failed: {e}"));
    assert_eq!(
        outcome_zero.config.remote_api_port,
        AppConfig::DEFAULT_REMOTE_API_PORT
    );
}

#[test]
fn app_config_repository_r3_corrupt_file_algorithm() {
    let fixture = load_roundtrip_fixture("r3-corrupt-file-algorithm");
    let config_path = PathBuf::from(resolve_tmp(fixture.input["configPath"].as_str().unwrap()));
    let garbage = fixture.input["fsTree"][fixture.input["configPath"].as_str().unwrap()]["content"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    let fs = FakeFileSystem::new().with_file(config_path.clone(), garbage.clone(), false);
    let defaults = AppConfig::default_config("/srv/msc2/servers");
    let now = fixed_now();

    let outcome = load_app_config(&fs, &config_path, &defaults, now)
        .unwrap_or_else(|e| panic!("load_app_config failed: {e}"));

    let expected_backup = corrupt_backup_path(&config_path, now);
    assert_eq!(
        outcome.corrupt_backup_path,
        Some(expected_backup.clone()),
        "corruptCopyCreated"
    );
    assert_eq!(
        fs.read(&expected_backup).unwrap(),
        garbage,
        "corruptCopyContentEqualsOriginal"
    );
    assert_eq!(outcome.config, defaults, "usedDefaults");
}

#[test]
fn app_config_repository_r3_corrupt_file_does_not_wipe_original() {
    // MSC 1's own version of this fixture only simulates the catch block's
    // copy step (source `AppConfigRoundTripTests.swift` line 357-380) and
    // stops there -- deliberately never reaching `save()` -- so it can
    // assert the *original* file is untouched immediately after the copy.
    // `load_app_config` composes the whole algorithm through to
    // completion, including the final overwrite (proved by the malformed-
    // JSON test above), so "the original file is untouched" isn't an
    // observable postcondition of the composed call the way it was of the
    // Swift test's truncated simulation. What *is* still true, and is what
    // this fixture's underlying claim reduces to at this boundary, is that
    // the backup step is a copy, not a destructive move: the bytes it
    // preserves are exactly what was on disk before recovery touched
    // anything, never a partially-mutated version of them.
    let fixture = load_roundtrip_fixture("r3-corrupt-file-does-not-wipe-original");
    let config_path = PathBuf::from(resolve_tmp(fixture.input["configPath"].as_str().unwrap()));
    let garbage = fixture.input["fsTree"][fixture.input["configPath"].as_str().unwrap()]["content"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    let fs = FakeFileSystem::new().with_file(config_path.clone(), garbage.clone(), false);
    let defaults = AppConfig::default_config("/srv/msc2/servers");
    let now = fixed_now();

    let outcome = load_app_config(&fs, &config_path, &defaults, now)
        .unwrap_or_else(|e| panic!("load_app_config failed: {e}"));

    let backup_path = outcome
        .corrupt_backup_path
        .expect("decode failure must produce a corrupt backup");
    assert_eq!(
        fs.read(&backup_path).unwrap(),
        garbage,
        "the backup must be an unmodified copy of the original bytes"
    );
}

#[test]
fn config_corpus_dimensions_wrong_type_field_triggers_corruption_recovery() {
    let path = PathBuf::from("/srv/msc2/server_config_swift.json");
    // Valid JSON syntax -- `serde_json::from_slice` succeeds -- but
    // `initial_setup_done` is a string where `AppConfig::decode` needs a
    // bool, so the *struct* decode fails. MSC 1's `ConfigManager.init`
    // catches this the same way it catches a JSON syntax error (the whole
    // `decoder.decode(AppConfig.self, from: data)` call sits inside its one
    // `do`/`catch`), so this must reach the same R3 recovery a garbled file
    // does, not bubble up as a bare decode error.
    let raw = br#"{
        "schemaVersion": 1,
        "config_version": 1,
        "java_path": "/usr/bin/java",
        "initial_setup_done": "yes"
    }"#
    .to_vec();
    let fs = FakeFileSystem::new().with_file(path.clone(), raw.clone(), false);
    let defaults = AppConfig::default_config("/srv/msc2/servers");
    let now = fixed_now();

    let outcome = load_app_config(&fs, &path, &defaults, now)
        .unwrap_or_else(|e| panic!("wrong-type field must recover, not error: {e}"));

    let expected_backup = corrupt_backup_path(&path, now);
    assert_eq!(outcome.corrupt_backup_path, Some(expected_backup.clone()));
    assert_eq!(
        fs.read(&expected_backup).unwrap(),
        raw,
        "the backup must preserve the original bytes byte-for-byte"
    );
    assert_eq!(
        outcome.config, defaults,
        "a struct-decode failure falls back to defaults, same as a syntax failure"
    );
}

#[test]
fn config_corpus_dimensions_unknown_top_level_field_does_not_survive_a_save_cycle() {
    // Contrast with `config_lifecycle.rs`'s
    // `config_lifecycle_unknown_fields_survive_read_modify_write`: that
    // test proves the *untyped* `load_config`/`save_config` primitive
    // preserves a field it doesn't recognize (it operates on a bare
    // `serde_json::Value`). Once the same JSON passes through the *typed*
    // `AppConfig` schema, the field is gone after one round trip --
    // `AppConfig::decode` never reads it and `AppConfig::encode` never
    // re-emits it, matching MSC 1's manually-implemented `Codable`
    // (`init(from:)`/`encode(to:)`, `AppConfig.swift` lines 614-868): only
    // the keys named in `CodingKeys` are ever read or written, so a key
    // outside that set is silently dropped, not preserved the way a
    // catch-all `[String: Any]` dictionary would.
    let path = PathBuf::from("/srv/msc2/server_config_swift.json");
    let raw = br#"{
        "schemaVersion": 1,
        "config_version": 1,
        "java_path": "/usr/bin/java",
        "a_future_field_this_port_does_not_know_about": "should not survive"
    }"#
    .to_vec();
    let fs = FakeFileSystem::new().with_file(path.clone(), raw, false);
    let defaults = AppConfig::default_config("/srv/msc2/servers");
    let now = fixed_now();

    let outcome = load_app_config(&fs, &path, &defaults, now)
        .unwrap_or_else(|e| panic!("load_app_config failed: {e}"));
    assert!(outcome.corrupt_backup_path.is_none());
    assert_eq!(
        outcome.config.java_path, "/usr/bin/java",
        "a known field alongside the unknown one must still decode"
    );

    save_app_config(&fs, &path, &outcome.config)
        .unwrap_or_else(|e| panic!("save_app_config failed: {e}"));

    let on_disk: Value = serde_json::from_slice(&fs.read(&path).unwrap()).unwrap();
    assert!(
        on_disk
            .get("a_future_field_this_port_does_not_know_about")
            .is_none(),
        "an unrecognized field must not survive a decode-then-encode cycle"
    );
}

#[test]
fn config_corpus_dimensions_atomic_write_interruption_leaves_previous_config_intact() {
    // The primitive-level `fixtures/atomic-write/destination-untouched-
    // before-rename` case never calls `atomic_write` at all -- it writes
    // straight to the temp path to simulate a crash between the two steps.
    // That's not evidence the real consumer (`save_config`, and above it
    // `save_app_config`) still behaves that way if either changed how it
    // calls the primitive. This test drives the actual composed
    // `save_app_config` call and injects the rename failure inside it.
    let path = PathBuf::from("/srv/msc2/server_config_swift.json");
    let previous = br#"{"schemaVersion":1,"config_version":1,"java_path":"/usr/bin/java"}"#;
    let fs = FakeFileSystem::new()
        .with_file(path.clone(), previous.to_vec(), false)
        .with_failing_rename(path.clone());
    let defaults = AppConfig::default_config("/srv/msc2/servers");
    let mut config = defaults.clone();
    config.java_path = "/opt/java21/bin/java".to_string();

    let err = save_app_config(&fs, &path, &config)
        .expect_err("an injected rename failure must surface, not silently succeed");
    assert!(matches!(err, ConfigSaveError::Write(_)));

    assert_eq!(
        fs.read(&path).unwrap(),
        previous,
        "the previous config at `path` must be exactly what it was before this call"
    );
    assert!(
        fs.read(&temp_path_for(&path)).is_err(),
        "atomic_write cleans up its temp file even when the rename fails"
    );
}
