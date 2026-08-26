//! Fixture-backed port of MSC 1's `PlayerNBTReader`.

mod support;

use msc_domain::player_nbt::{InventoryItem, ItemEnchantment, PlayerStats, offline_uuid, read_all};
use serde_json::Value;
use std::fs;
use std::path::Path;
use support::Fixture;

fn load(path: impl AsRef<Path>) -> Fixture {
    support::load(path)
}

fn hex_bytes(raw: &str) -> Vec<u8> {
    assert!(raw.len().is_multiple_of(2), "hex input has odd length");
    raw.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16).unwrap();
            let low = (pair[1] as char).to_digit(16).unwrap();
            ((high << 4) | low) as u8
        })
        .collect()
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    use std::io::Write;

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

fn fixture_bytes(fixture: &Fixture) -> Vec<u8> {
    if let Some(raw) = fixture.input.get("raw_dat_hex").and_then(Value::as_str) {
        return hex_bytes(raw);
    }
    if let Some(nbt) = fixture.input.get("nbt_hex").and_then(Value::as_str) {
        return gzip(&hex_bytes(nbt));
    }
    let sample = fixture.input["dat_file"].as_str().unwrap();
    let relative = sample.strip_prefix("fixtures/").unwrap_or(sample);
    fs::read(support::fixtures_dir().join(relative))
        .unwrap_or_else(|error| panic!("{sample}: could not read real sample: {error}"))
}

fn assert_stats(actual: &PlayerStats, expected: &Value) {
    assert_eq!(actual.health, expected["health"].as_f64().unwrap() as f32);
    assert_eq!(
        actual.max_health,
        expected["maxHealth"].as_f64().unwrap() as f32
    );
    assert_eq!(
        actual.food_level,
        expected["foodLevel"].as_i64().unwrap() as i32
    );
    assert_eq!(
        actual.xp_level,
        expected["xpLevel"].as_i64().unwrap() as i32
    );
    assert_eq!(
        actual.xp_total,
        expected["xpTotal"].as_i64().unwrap() as i32
    );
    assert_eq!(
        actual.game_mode,
        expected["gameMode"].as_i64().unwrap() as i32
    );
    assert_eq!(
        actual.game_mode_display(),
        expected["gameModeDisplay"].as_str().unwrap()
    );
    assert_eq!(actual.pos_x, expected["posX"].as_f64().unwrap());
    assert_eq!(actual.pos_y, expected["posY"].as_f64().unwrap());
    assert_eq!(actual.pos_z, expected["posZ"].as_f64().unwrap());
    assert_eq!(
        actual.dimension_display(),
        expected["dimensionDisplay"].as_str().unwrap()
    );
    assert_eq!(actual.score, expected["score"].as_i64().unwrap() as i32);
}

fn assert_inventory(actual: &[InventoryItem], expected: &Value, case: &str) {
    let expected_items = expected.as_array().unwrap();
    assert_eq!(
        actual.len(),
        expected_items.len(),
        "{case}: inventory length"
    );
    for (item, expected) in actual.iter().zip(expected_items) {
        assert_eq!(item.slot, expected["slot"].as_i64().unwrap() as i32);
        assert_eq!(item.item_id, expected["itemID"].as_str().unwrap());
        assert_eq!(item.icon_name(), expected["iconName"].as_str().unwrap());
        assert_eq!(item.count, expected["count"].as_i64().unwrap() as i32);
        assert_eq!(
            item.display_name(),
            expected["displayName"].as_str().unwrap()
        );
        assert_eq!(item.damage, expected["damage"].as_i64().unwrap() as i32);

        let enchantments = expected["enchantments"].as_array().unwrap();
        assert_eq!(item.enchantments.len(), enchantments.len());
        for (enchantment, expected) in item.enchantments.iter().zip(enchantments) {
            assert_eq!(enchantment.id, expected["id"].as_str().unwrap());
            assert_eq!(
                enchantment.level,
                expected["level"].as_i64().unwrap() as i32
            );
            assert_eq!(
                enchantment.display_name(),
                expected["displayName"].as_str().unwrap()
            );
        }
    }

    let expected_custom_names: &[Option<&str>] = match case {
        "inventory-custom-name-json-empty-and-invalid-fallback" => &[None, Some("not-json-name")],
        "inventory-legacy-enchantments-display-and-damage" => &[Some("Stormbringer")],
        "inventory-modern-components-enchantments-name-and-damage" => &[Some("Builder")],
        "inventory-stored-enchantment-slot-int-and-count-clamp" => &[Some("Ancient Tome")],
        _ => &[],
    };
    if !expected_custom_names.is_empty() {
        for (item, expected) in actual.iter().zip(expected_custom_names) {
            assert_eq!(item.custom_name.as_deref(), *expected);
        }
    } else {
        assert!(actual.iter().all(|item| item.custom_name.is_none()));
    }
}

#[test]
fn player_nbt_fixture_corpus() {
    let directory = support::fixtures_dir().join("player-nbt");
    let mut paths: Vec<_> = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("json"))
        .collect();
    paths.sort();
    assert_eq!(paths.len(), 13);

    for path in paths {
        let fixture = load(&path);
        let (stats, inventory) = read_all(&fixture_bytes(&fixture));
        match fixture.expected["stats"].as_object() {
            Some(_) => {
                let stats = stats.unwrap_or_else(|| panic!("{}: stats should parse", fixture.case));
                assert_stats(&stats, &fixture.expected["stats"]);
            }
            None => assert!(stats.is_none(), "{}: stats should fail", fixture.case),
        }
        assert_inventory(&inventory, &fixture.expected["inventory"], &fixture.case);
    }
}

#[test]
fn player_nbt_display_helpers_match_swift_capitalization_and_fallbacks() {
    let stats = PlayerStats {
        health: 20.0,
        max_health: 20.0,
        food_level: 20,
        xp_level: 0,
        xp_total: 0,
        game_mode: 9,
        pos_x: 0.0,
        pos_y: 0.0,
        pos_z: 0.0,
        dimension: "custom:ancient_ruins".to_owned(),
        score: 0,
    };
    assert_eq!(stats.game_mode_display(), "Unknown (9)");
    assert_eq!(stats.dimension_display(), "Ancient Ruins");

    let enchantment = ItemEnchantment {
        id: "minecraft:fire_aspect".to_owned(),
        level: 0,
    };
    assert_eq!(enchantment.display_name(), "Fire Aspect 0");
    assert_eq!(
        (ItemEnchantment {
            level: 5,
            ..enchantment.clone()
        })
        .display_name(),
        "Fire Aspect V"
    );
    assert_eq!(
        (ItemEnchantment {
            level: 6,
            ..enchantment
        })
        .display_name(),
        "Fire Aspect 6"
    );

    let item = InventoryItem {
        slot: 0,
        item_id: "minecraft:diamond_sword".to_owned(),
        count: 1,
        enchantments: Vec::new(),
        custom_name: Some(String::new()),
        damage: 0,
    };
    assert_eq!(item.display_name(), "Diamond Sword");
    assert_eq!(item.icon_name(), "diamond_sword");
}

#[test]
fn offline_uuid_matches_java_name_uuid_vectors() {
    let vectors = [
        ("Notch", "b50ad385-829d-3141-a216-7e7d7539ba7f"),
        ("Bob", "faa5dca3-c3d4-354b-ae1b-dde9e5a14b3b"),
        ("jeb_", "a762f560-4fce-3236-812a-b80efff0b62b"),
        ("Dinnerbone", "4d258a81-2358-3084-8166-05b9faccad80"),
        ("", "fc5bc365-aedf-30a8-8b89-04e462e29bde"),
    ];

    for (username, expected) in vectors {
        assert_eq!(offline_uuid(username).to_string(), expected);
    }
}
