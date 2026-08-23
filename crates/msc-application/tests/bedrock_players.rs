//! P10.20 application-service checks: Bedrock's JSON files and properties
//! are updated atomically, while unknown properties and known XUIDs survive.

use msc_application::bedrock_players::{
    load_name_cache, mutate_allowlist, read_allowlist, record_name, set_player_permission,
};
use msc_application::bedrock_settings::update;
use msc_domain::bedrock::BedrockPermissionLevel;
use msc_infrastructure::fs::StdFileSystem;
use std::collections::BTreeMap;
use std::fs;

fn server_dir(tag: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("msc2-bedrock-players-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn allowlist_and_name_cache_use_atomic_replacements() {
    let dir = server_dir("json");
    let fs = StdFileSystem;
    fs::write(
        dir.join("allowlist.json"),
        br#"[{"name":"Alex","xuid":"7","ignoresPlayerLimit":true}]"#,
    )
    .unwrap();

    let entries = mutate_allowlist(&fs, &dir, "add", "Steve").unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].xuid.as_deref(), Some("7"));

    let cache = record_name(&fs, &dir, " 8\n", " Alex ").unwrap();
    assert_eq!(cache.get("8").map(String::as_str), Some("Alex"));
    assert_eq!(load_name_cache(&fs, &dir), cache);
    assert_eq!(read_allowlist(&fs, &dir).len(), 2);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn settings_keep_unknown_keys_and_reject_invalid_values_before_write() {
    let dir = server_dir("settings");
    let fs = StdFileSystem;
    fs::write(
        dir.join("server.properties"),
        b"level-name=world\nmax-players=10\nfuture-bds-key=kept\n",
    )
    .unwrap();
    let changes = BTreeMap::from([
        ("max-players".to_owned(), "20".to_owned()),
        ("server-port".to_owned(), "not-a-port".to_owned()),
    ]);
    let result = update(&fs, &dir, &changes).unwrap();
    assert_eq!(result.applied_keys, vec!["max-players"]);
    assert_eq!(result.rejected.len(), 1);
    let written = String::from_utf8(fs::read(dir.join("server.properties")).unwrap()).unwrap();
    assert!(written.contains("future-bds-key=kept"));
    assert!(written.contains("max-players=20"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn permissions_replace_xuid_and_use_bedrock_levels() {
    let dir = server_dir("permissions");
    let fs = StdFileSystem;
    fs::write(
        dir.join("permissions.json"),
        br#"[{"permission":"member","xuid":"7"}]"#,
    )
    .unwrap();
    let permissions = set_player_permission(&fs, &dir, "7", "operator").unwrap();
    assert_eq!(permissions[0].permission, BedrockPermissionLevel::Operator);
    assert_eq!(permissions[0].xuid, "7");
    let _ = fs::remove_dir_all(dir);
}
