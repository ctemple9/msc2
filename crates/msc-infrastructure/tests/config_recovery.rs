//! Port of the 6 fixtures in `fixtures/config-recovery/`:
//! `AppViewModel+ConfigRecovery.swift`'s corrupt-backup discovery
//! (`findCorruptBackups`, `serverCountInBackup`) and recovery merge
//! (`restoreServersFromBackup`, P5.7). No MSC 1 test file exercises any of
//! these three functions directly (only UI call sites reference them), so
//! every fixture here was characterized straight from source rather than
//! extracted from an XCTest — each fixture's `source.test` names the
//! function itself, the same precedent P5.5's `app-config-normalization`
//! fixtures set. `rescanAndImportServers`, the source file's separate
//! untracked-folder recovery path, is P5.22's own step, not this one's.

use msc_domain::app_config_schema::{AppConfig, ConfigServer};
use msc_infrastructure::config_repository::{
    BackupRestoreResult, find_corrupt_backups, restore_servers_from_backup, server_count_in_backup,
};
use msc_infrastructure::fs::FakeFileSystem;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn load_fixture(case: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/config-recovery")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()))
}

fn servers_from(list: &[Value]) -> Vec<ConfigServer> {
    list.iter()
        .map(|v| ConfigServer::decode(v).unwrap_or_else(|e| panic!("bad fixture server: {e}")))
        .collect()
}

fn app_config_with_servers(servers: Vec<ConfigServer>) -> AppConfig {
    let mut config = AppConfig::default_config("/servers");
    config.servers = servers;
    config
}

fn server_ids(config: &AppConfig) -> Vec<&str> {
    config.servers.iter().map(|s| s.id.as_str()).collect()
}

/// Runs `restore_servers_from_backup` against a fixture whose `input` has
/// `live.servers` and `backup.servers` arrays, asserting the fixture's
/// `restored`/`skipped`/`error`/`finalServerCount` expectations.
fn run_merge_fixture(case: &str) -> (AppConfig, BackupRestoreResult, Value) {
    let fixture = load_fixture(case);
    let live_servers = servers_from(fixture["input"]["live"]["servers"].as_array().unwrap());
    let backup_servers = servers_from(fixture["input"]["backup"]["servers"].as_array().unwrap());

    let live = app_config_with_servers(live_servers);
    let defaults = AppConfig::default_config("/servers");
    let mut backup_config = defaults.clone();
    backup_config.servers = backup_servers;

    let backup_path = PathBuf::from("/srv/msc2/server_config_swift.json.corrupt-1");
    let fs = FakeFileSystem::new().with_file(
        backup_path.clone(),
        serde_json::to_vec(&backup_config.encode()).unwrap(),
        false,
    );

    let (merged, result) = restore_servers_from_backup(&fs, &backup_path, &defaults, &live);
    let expected = &fixture["expected"];
    assert_eq!(
        result.restored,
        expected["restored"].as_u64().unwrap() as usize,
        "restored count for {case}"
    );
    assert_eq!(
        result.skipped,
        expected["skipped"].as_u64().unwrap() as usize,
        "skipped count for {case}"
    );
    assert_eq!(
        merged.servers.len(),
        expected["finalServerCount"].as_u64().unwrap() as usize,
        "final server count for {case}"
    );
    (merged, result, fixture)
}

#[test]
fn pure_restore() {
    let (merged, result, fixture) = run_merge_fixture("pure-restore");
    assert!(result.error.is_none());
    let expected_ids: Vec<&str> = fixture["expected"]["restoredServerIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    for id in expected_ids {
        assert!(
            server_ids(&merged).contains(&id),
            "expected restored id {id} to be present"
        );
    }
    assert!(server_ids(&merged).contains(&"srv-existing"));
}

#[test]
fn live_path_collision() {
    let (merged, result, _fixture) = run_merge_fixture("live-path-collision");
    assert!(result.error.is_none());
    assert_eq!(server_ids(&merged), vec!["srv-existing"]);
}

#[test]
fn live_id_collision() {
    let (merged, result, _fixture) = run_merge_fixture("live-id-collision");
    assert!(result.error.is_none());
    assert_eq!(merged.servers.len(), 1);
    assert_eq!(merged.servers[0].server_dir, "/servers/existing");
}

#[test]
fn duplicate_entries_in_backup() {
    // Pins MSC 1's real behavior: `existingPaths`/`existingIDs` are
    // captured once before the merge loop runs and never updated inside
    // it, so two backup entries that duplicate each other -- but collide
    // with nothing already live -- both restore rather than the second
    // being treated as a duplicate of the first.
    let (merged, result, _fixture) = run_merge_fixture("duplicate-entries-in-backup");
    assert!(result.error.is_none());
    assert_eq!(merged.servers.len(), 2);
    assert!(merged.servers.iter().all(|s| s.id == "srv-dup"));
}

#[test]
fn unreadable_backup_no_mutation() {
    let fixture = load_fixture("unreadable-backup-no-mutation");
    let live_servers = servers_from(fixture["input"]["live"]["servers"].as_array().unwrap());
    let live = app_config_with_servers(live_servers);
    let defaults = AppConfig::default_config("/servers");

    // No `with_file` call for the backup path: `fs.read` fails exactly
    // like `Data(contentsOf:)` failing in source, line 56.
    let fs = FakeFileSystem::new();
    let backup_path = PathBuf::from("/srv/msc2/server_config_swift.json.corrupt-1");

    let (merged, result) = restore_servers_from_backup(&fs, &backup_path, &defaults, &live);

    assert_eq!(result.restored, 0);
    assert_eq!(result.skipped, 0);
    assert!(
        result.error.is_some(),
        "an unreadable backup must report an error"
    );
    assert_eq!(merged, live, "the live config must be returned unchanged");
}

#[test]
fn discovery_ordering() {
    let fixture = load_fixture("discovery-ordering");
    let config_path = PathBuf::from(fixture["input"]["configPath"].as_str().unwrap());
    let parent = config_path.parent().unwrap();

    let mut fs = FakeFileSystem::new();
    for name in fixture["input"]["dirEntries"].as_array().unwrap() {
        let name = name.as_str().unwrap();
        fs = fs.with_file(parent.join(name), Vec::new(), false);
    }

    let backups = find_corrupt_backups(&fs, &config_path);
    let found: Vec<String> = backups
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();

    let expected: Vec<String> = fixture["expected"]["orderedBackupFiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(found, expected);
}

#[test]
fn server_count_in_backup_reads_cheaply_without_full_decode() {
    let path = PathBuf::from("/srv/msc2/server_config_swift.json.corrupt-1");
    let raw = br#"{"servers": [{"id": "a"}, {"id": "b"}, {"id": "c"}]}"#.to_vec();
    let fs = FakeFileSystem::new().with_file(path.clone(), raw, false);
    assert_eq!(server_count_in_backup(&fs, &path), Some(3));
}

#[test]
fn server_count_in_backup_returns_none_when_unreadable_or_malformed() {
    let fs = FakeFileSystem::new();
    let missing = PathBuf::from("/srv/msc2/server_config_swift.json.corrupt-1");
    assert_eq!(server_count_in_backup(&fs, &missing), None);

    let malformed_path = PathBuf::from("/srv/msc2/server_config_swift.json.corrupt-2");
    let fs = FakeFileSystem::new().with_file(malformed_path.clone(), b"not json".to_vec(), false);
    assert_eq!(server_count_in_backup(&fs, &malformed_path), None);

    let no_servers_path = PathBuf::from("/srv/msc2/server_config_swift.json.corrupt-3");
    let fs = FakeFileSystem::new().with_file(no_servers_path.clone(), b"{}".to_vec(), false);
    assert_eq!(server_count_in_backup(&fs, &no_servers_path), None);
}
