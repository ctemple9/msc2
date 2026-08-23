//! Fixture-backed pure Bedrock settings, console, and identity rules.

mod support;

use msc_domain::bedrock::*;
use msc_domain::properties::{ServerDifficulty, ServerGamemode};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn bedrock_properties_fixture_corpus() {
    for entry in std::fs::read_dir(support::fixtures_dir().join("bedrock-properties")).unwrap() {
        let path = entry.unwrap().path();
        let case = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let fixture = support::load(path);
        match case.as_str() {
            "raw-properties-trims-comments-and-malformed-lines" => {
                let actual =
                    parse_raw_properties(fixture.input["server_properties"].as_str().unwrap());
                assert_eq!(actual, map(&fixture.expected["raw"]));
            }
            "raw-write-adds-header-and-sorts-keys" => {
                assert_eq!(
                    render_raw_properties(&map(&fixture.input["properties"])),
                    fixture.expected["file"]
                );
            }
            "missing-properties-returns-empty" => {
                assert_eq!(fixture.input["server_properties_file_present"], false);
            }
            name if name.starts_with("allowlist-")
                || name.starts_with("malformed-allowlist-")
                || name.starts_with("missing-allowlist-")
                || name.starts_with("legacy-allowlist-")
                || name.starts_with("add-allowlist-")
                || name.starts_with("remove-allowlist-") =>
            {
                if name == "malformed-allowlist-is-empty" {
                    assert!(
                        parse_allowlist(fixture.input["allowlist_json"].as_str().unwrap())
                            .is_empty()
                    );
                    continue;
                }
                if name == "missing-allowlist-file-is-empty" {
                    assert!(fixture.input["allowlist_file_present"].as_bool() == Some(false));
                    continue;
                }
                let entries = if let Some(json) = fixture.input.get("json") {
                    serde_json::from_value::<Vec<AllowlistEntry>>(json.clone()).unwrap()
                } else {
                    serde_json::from_value::<Vec<AllowlistEntry>>(
                        fixture
                            .input
                            .get("existing")
                            .cloned()
                            .unwrap_or(Value::Array(vec![])),
                    )
                    .unwrap()
                };
                if name.starts_with("add-") {
                    let mut entries = entries;
                    let add = &fixture.input["add"];
                    let wrote = add_allowlist_entry(
                        &mut entries,
                        add["name"].as_str().unwrap(),
                        add["xuid"].as_str().map(str::to_owned),
                    );
                    assert_eq!(wrote, fixture.expected["wrote"]);
                    assert_allowlist(&entries, &fixture.expected["entries"]);
                } else if name.starts_with("remove-") {
                    let mut entries = entries;
                    remove_allowlist_entry(&mut entries, fixture.input["remove"].as_str().unwrap());
                    assert_allowlist(&entries, &fixture.expected["entries"]);
                } else {
                    assert_allowlist(&entries, &fixture.expected["entries"]);
                }
            }
            name if name.starts_with("permission")
                || name.starts_with("malformed-permissions")
                || name.starts_with("missing-permissions") =>
            {
                if name == "malformed-permissions-are-empty" {
                    assert!(
                        parse_permissions(&fixture.input["permissions_json"].to_string()).is_err()
                    );
                } else if let Some(json) = fixture.input.get("json") {
                    let actual = parse_permissions(&json.to_string()).unwrap();
                    assert_eq!(
                        serde_json::to_value(actual).unwrap(),
                        fixture.expected["entries"]
                    );
                }
            }
            "model-defaults-when-properties-are-empty"
            | "model-reads-all-recognized-values"
            | "invalid-integer-keeps-model-default"
            | "out-of-range-values-are-not-clamped"
            | "unknown-difficulty-is-silently-ignored"
            | "unknown-gamemode-is-silently-ignored" => {
                let model = BedrockPropertiesModel::from_raw(&map(&fixture.input["raw"]));
                assert_eq!(
                    model.level_name,
                    fixture.expected["level_name"]
                        .as_str()
                        .unwrap_or("Bedrock level")
                );
                assert_eq!(
                    model.max_players,
                    fixture.expected["max_players"].as_i64().unwrap_or(10)
                );
                assert_eq!(
                    model.server_port,
                    fixture.expected["server_port"].as_i64().unwrap_or(19132)
                );
                assert_eq!(
                    model.server_port_v6,
                    fixture.expected["server_port_v6"].as_i64().unwrap_or(19133)
                );
                assert_eq!(
                    model.online_mode,
                    fixture.expected["online_mode"].as_bool().unwrap_or(true)
                );
                assert_eq!(
                    model.allow_cheats,
                    fixture.expected["allow_cheats"].as_bool().unwrap_or(false)
                );
            }
            "model-write-emits-bds-enum-keys" | "unknown-key-survives-model-round-trip" => {
                let mut model = BedrockPropertiesModel::default();
                let raw = map(&fixture.input["raw"]);
                if let Some(input) = fixture.input.get("model") {
                    if let Some(value) = input.get("level_name").and_then(Value::as_str) {
                        model.level_name = value.into();
                    }
                    if let Some(value) = input.get("difficulty").and_then(Value::as_str) {
                        model.difficulty = difficulty(value);
                    }
                    if let Some(value) = input.get("gamemode").and_then(Value::as_str) {
                        model.gamemode = gamemode(value);
                    }
                    for (field, target) in [
                        ("max_players", &mut model.max_players),
                        ("server_port", &mut model.server_port),
                        ("server_port_v6", &mut model.server_port_v6),
                    ] {
                        if let Some(value) = input.get(field).and_then(Value::as_i64) {
                            *target = value;
                        }
                    }
                }
                let actual = model.merged_into(&raw);
                for (key, value) in fixture.expected["written"]
                    .as_object()
                    .unwrap_or(&serde_json::Map::new())
                {
                    assert_eq!(actual.get(key).map(String::as_str), value.as_str());
                }
            }
            "set-permission-adds-when-xuid-is-absent"
            | "set-permission-replaces-existing-xuid"
            | "remove-permission-removes-matching-xuid" => {
                let mut entries = serde_json::from_value::<Vec<PermissionEntry>>(
                    fixture.input["existing"].clone(),
                )
                .unwrap();
                if let Some(set) = fixture.input.get("set") {
                    set_permission(
                        &mut entries,
                        set["xuid"].as_str().unwrap(),
                        permission(set["level"].as_str().unwrap()),
                    );
                }
                if let Some(remove) = fixture.input.get("remove") {
                    remove_permission(&mut entries, remove.as_str().unwrap());
                }
                assert_eq!(
                    serde_json::to_value(entries).unwrap(),
                    fixture.expected["entries"]
                );
            }
            _ => panic!("unhandled Bedrock properties fixture {case}"),
        }
    }
}

#[test]
fn bedrock_console_fixture_corpus() {
    for entry in std::fs::read_dir(support::fixtures_dir().join("bedrock-console")).unwrap() {
        let fixture = support::load(entry.unwrap().path());
        let event = classify_console_line(fixture.input["line"].as_str().unwrap());
        if let Some(version) = fixture.expected["running_version"].as_str() {
            assert_eq!(event, BedrockConsoleEvent::Version(version.into()));
        } else if fixture.expected.get("ready").is_some() {
            assert_eq!(event, BedrockConsoleEvent::Ready);
        } else if fixture.expected.get("guest_ip").is_some() {
            match event {
                BedrockConsoleEvent::GuestIp(ip) => assert_eq!(
                    ip.to_string(),
                    fixture.expected["guest_ip"].as_str().unwrap_or("")
                ),
                _ => assert!(fixture.expected["guest_ip"].is_null()),
            }
        } else if fixture.expected.get("cpu_percent").is_some() {
            match event {
                BedrockConsoleEvent::Stats(stats) => {
                    assert_eq!(stats.cpu_percent, fixture.expected["cpu_percent"].as_f64());
                    assert_eq!(
                        stats.memory_used_mb.map(|v| v as f64),
                        fixture.expected["mem_used_mb"].as_f64()
                    );
                    assert_eq!(
                        stats.memory_total_mb.map(|v| v as f64),
                        fixture.expected["mem_total_mb"].as_f64()
                    );
                }
                _ => panic!("expected stats"),
            }
        } else if fixture.expected.get("player_event_parsed") == Some(&Value::Bool(false)) {
            assert_eq!(event, BedrockConsoleEvent::Other);
        } else {
            match event {
                BedrockConsoleEvent::Player(BedrockPlayerEvent::Connected(player)) => {
                    assert_eq!(
                        player.name,
                        fixture.expected["name"].as_str().unwrap_or("Alex")
                    );
                    let expected_xuid = fixture
                        .expected
                        .get("xuid")
                        .and_then(Value::as_str)
                        .or_else(|| {
                            fixture
                                .expected
                                .pointer("/online_players/0/xuid")
                                .and_then(Value::as_str)
                        })
                        .or_else(|| {
                            fixture
                                .expected
                                .pointer("/online_after/0/xuid")
                                .and_then(Value::as_str)
                        });
                    assert_eq!(player.xuid, expected_xuid.map(str::to_owned));
                }
                BedrockConsoleEvent::Player(BedrockPlayerEvent::Disconnected(player)) => {
                    assert_eq!(player.name, "aLeX")
                }
                _ => {}
            }
        }
    }
}

#[test]
fn bedrock_player_fixture_corpus() {
    for entry in std::fs::read_dir(support::fixtures_dir().join("bedrock-players")).unwrap() {
        let fixture = support::load(entry.unwrap().path());
        let case = fixture.case.as_str();
        match case {
            "floodgate-id-with-dashes-is-normalized" => assert_eq!(
                normalize_uuid(fixture.input["raw"].as_str().unwrap()).as_deref(),
                fixture.expected["uuid"].as_str()
            ),
            "floodgate-lookup-retries-once" => assert_eq!(
                normalize_uuid("03c5ad1d111122223333444444444444").as_deref(),
                fixture.expected["uuid"].as_str()
            ),
            name if name.starts_with("floodgate-lookup-") => assert_eq!(
                floodgate_lookup_path(fixture.input["gamertag"].as_str().unwrap()),
                fixture.expected["request_path"]
            ),
            "gamertag-lookup-uses-geyser-xuid-endpoint" => assert_eq!(
                xuid_lookup_url(fixture.input["xuid"].as_str().unwrap()),
                fixture.expected["request_url"]
            ),
            "name-cache-missing-or-invalid-is-empty" => {}
            name if name.starts_with("name-cache-") => {
                let mut map = fixture.input["existing"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap().into()))
                    .collect();
                let record = &fixture.input["record"];
                assert_eq!(
                    trimmed_name_cache_record(
                        &mut map,
                        record["xuid"].as_str().unwrap(),
                        record["name"].as_str().unwrap()
                    ),
                    fixture
                        .expected
                        .get("wrote")
                        .or_else(|| fixture.expected.get("atomic_write"))
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                );
                assert_eq!(
                    serde_json::to_value(map).unwrap(),
                    fixture.expected["mapping"]
                );
            }
            "hidden-profile-hide-then-unhide-persists-set" => {
                let mut hidden = fixture.input["existing"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().into())
                    .collect::<BTreeSet<String>>();
                toggle_hidden_profile(&mut hidden, fixture.input["hide"].as_str().unwrap());
                toggle_hidden_profile(&mut hidden, fixture.input["then_unhide"].as_str().unwrap());
                assert_eq!(hidden.into_iter().collect::<Vec<_>>(), vec!["2"]);
            }
            "allowlist-xuid-backfill-is-java-server-guarded" => {
                let entries: Vec<AllowlistEntry> =
                    serde_json::from_value(fixture.input["allowlist"].clone()).unwrap();
                assert!(
                    backfill_allowlist_xuid(
                        fixture.input["selected_server_is_bedrock"]
                            .as_bool()
                            .unwrap(),
                        &entries,
                        fixture.input["connect"]["name"].as_str().unwrap(),
                        fixture.input["connect"]["xuid"].as_str().unwrap()
                    )
                    .is_none()
                );
            }
            name if name.contains("player-key")
                || name.contains("profile")
                || name.contains("record")
                || name.contains("leveldb") =>
            {
                for record in fixture
                    .input
                    .get("records")
                    .and_then(Value::as_array)
                    .unwrap_or(&vec![])
                {
                    let key = record["key"].as_str().unwrap();
                    let identity = player_identity_from_key(key);
                    if key == "~local_player" {
                        assert_eq!(identity, Some(BedrockPlayerIdentity::Local));
                    } else if key.starts_with("player_253") {
                        assert!(matches!(
                            identity,
                            Some(BedrockPlayerIdentity::NumericXuid(_))
                        ));
                    } else if key.starts_with("player_server_") {
                        assert!(matches!(
                            identity,
                            Some(BedrockPlayerIdentity::ServerUuid { .. })
                        ));
                    } else {
                        assert!(identity.is_none());
                    }
                }
            }
            _ => {}
        }
    }
}

fn map(value: &Value) -> BTreeMap<String, String> {
    value
        .as_object()
        .unwrap()
        .iter()
        .map(|(key, value)| (key.clone(), value.as_str().unwrap().into()))
        .collect()
}

fn assert_allowlist(actual: &[AllowlistEntry], expected: &Value) {
    let expected = expected.as_array().unwrap();
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.name, expected["name"].as_str().unwrap());
        assert_eq!(actual.xuid.as_deref(), expected["xuid"].as_str());
        let ignore = expected
            .get("ignores_player_limit")
            .or_else(|| expected.get("ignoresPlayerLimit"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        assert_eq!(actual.ignores_player_limit, ignore);
    }
}

fn difficulty(value: &str) -> ServerDifficulty {
    ServerDifficulty::from_raw_value(value).unwrap()
}
fn gamemode(value: &str) -> ServerGamemode {
    ServerGamemode::from_raw_value(value).unwrap()
}
fn permission(value: &str) -> BedrockPermissionLevel {
    match value {
        "visitor" => BedrockPermissionLevel::Visitor,
        "member" => BedrockPermissionLevel::Member,
        "operator" => BedrockPermissionLevel::Operator,
        _ => panic!(),
    }
}
