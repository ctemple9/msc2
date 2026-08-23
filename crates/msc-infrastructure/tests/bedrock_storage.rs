//! Fixture-backed tests for the bounded Bedrock NBT and LevelDB readers.

use msc_infrastructure::bedrock_leveldb::{self, LevelDbError};
use msc_infrastructure::bedrock_nbt::{self, NbtError};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn fixture_count(domain: &str) -> usize {
    fs::read_dir(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(domain),
    )
    .unwrap()
    .count()
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "msc2-bedrock-storage-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn string(value: &str) -> Vec<u8> {
    let mut bytes = (value.len() as u16).to_le_bytes().to_vec();
    bytes.extend_from_slice(value.as_bytes());
    bytes
}

fn named(tag: u8, name: &str, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![tag];
    bytes.extend(string(name));
    bytes.extend_from_slice(payload);
    bytes
}

fn i32_tag(name: &str, value: i32) -> Vec<u8> {
    named(3, name, &value.to_le_bytes())
}
fn float_tag(name: &str, value: f32) -> Vec<u8> {
    named(5, name, &value.to_le_bytes())
}
fn str_tag(name: &str, value: &str) -> Vec<u8> {
    named(8, name, &string(value))
}

fn list_tag(name: &str, tag: u8, values: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = vec![tag];
    payload.extend_from_slice(&(values.len() as i32).to_le_bytes());
    for value in values {
        payload.extend_from_slice(value);
    }
    named(9, name, &payload)
}

fn compound_tag(name: &str, fields: &[Vec<u8>]) -> Vec<u8> {
    named(10, name, &compound_payload(fields))
}

fn compound_payload(fields: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = Vec::new();
    for field in fields {
        payload.extend_from_slice(field);
    }
    payload.push(0);
    payload
}

fn root(fields: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = vec![10];
    bytes.extend(string(""));
    for field in fields {
        bytes.extend_from_slice(field);
    }
    bytes.push(0);
    bytes
}

fn item(name: &str, slot: i32, count: i32) -> Vec<u8> {
    compound_payload(&[
        str_tag("Name", name),
        i32_tag("Slot", slot),
        i32_tag("Count", count),
    ])
}

#[test]
fn bedrock_nbt_fixture_corpus_is_present() {
    assert_eq!(fixture_count("bedrock-nbt"), 32);
}

#[test]
fn bedrock_nbt_reads_stats_dimensions_xp_and_positions() {
    let fields = vec![
        float_tag("Health", 18.0),
        i32_tag("FoodLevel", 17),
        i32_tag("PlayerLevel", 16),
        float_tag("PlayerLevelProgress", 0.5),
        i32_tag("playerGameType", 1),
        i32_tag("Score", 9),
        i32_tag("DimensionId", 1),
        list_tag(
            "Pos",
            5,
            &[
                1.25f32.to_le_bytes().to_vec(),
                64.5f32.to_le_bytes().to_vec(),
                (-2.75f32).to_le_bytes().to_vec(),
            ],
        ),
        list_tag(
            "Attributes",
            10,
            &[compound_payload(&[
                str_tag("Name", "minecraft:health"),
                float_tag("Base", 40.0),
            ])],
        ),
    ];
    let player = bedrock_nbt::read_player_nbt(&root(&fields)).unwrap();
    let stats = player.stats.unwrap();
    assert_eq!(stats.health, 18.0);
    assert_eq!(stats.max_health, 40.0);
    assert_eq!(stats.food_level, 17);
    assert_eq!(stats.xp_level, 16);
    assert_eq!(stats.xp_total, 373);
    assert_eq!(stats.dimension, "minecraft:the_nether");
    assert_eq!(stats.position, [1.25, 64.5, -2.75]);
}

#[test]
fn bedrock_nbt_reads_inventory_variants_and_safe_display_name() {
    let enchantment = compound_payload(&[i32_tag("id", 9), i32_tag("lvl", 3)]);
    let display = compound_tag("display", &[str_tag("Name", r#"{"text":"Pickaxe"}"#)]);
    let tag = compound_tag("tag", &[list_tag("ench", 10, &[enchantment]), display]);
    let inventory_item = compound_payload(&[
        str_tag("Name", "minecraft:stone"),
        i32_tag("Slot", 5),
        i32_tag("Count", 0),
        i32_tag("Damage", 7),
        tag,
    ]);
    let player = bedrock_nbt::read_player_nbt(&root(&[
        list_tag("Inventory", 10, &[inventory_item]),
        list_tag("Armor", 10, &[item("minecraft:helmet", 0, 1)]),
        list_tag("Offhand", 10, &[item("minecraft:shield", 0, 1)]),
    ]))
    .unwrap();
    assert_eq!(player.inventory[0].slot, 5);
    assert_eq!(player.inventory[0].count, 1);
    assert_eq!(player.inventory[0].damage, 7);
    assert_eq!(
        player.inventory[0].enchantments[0].id,
        "minecraft:sharpness"
    );
    assert_eq!(player.inventory[0].custom_name.as_deref(), Some("Pickaxe"));
    assert_eq!(player.inventory[1].slot, 103);
    assert_eq!(player.inventory[2].slot, -106);
}

#[test]
fn bedrock_nbt_rejects_corrupt_unknown_and_bounded_inputs() {
    assert!(matches!(
        bedrock_nbt::read_player_nbt(&[0, 0, 0]),
        Err(NbtError::Corrupt(_))
    ));
    assert!(matches!(
        bedrock_nbt::read_player_nbt(&[10, 0, 0, 99]),
        Err(NbtError::Unsupported(_)) | Err(NbtError::Corrupt(_))
    ));
    let mut huge = vec![10, 0, 0, 8];
    huge.extend_from_slice(&i16::MAX.to_le_bytes());
    assert!(matches!(
        bedrock_nbt::read_player_nbt(&huge),
        Err(NbtError::Corrupt(_)) | Err(NbtError::LimitExceeded(_))
    ));
}

fn varint(mut value: u64) -> Vec<u8> {
    let mut output = Vec::new();
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        output.push(byte);
        if value == 0 {
            return output;
        }
    }
}

fn block_record(key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut block = Vec::new();
    block.extend(varint(0));
    block.extend(varint(key.len() as u64));
    block.extend(varint(value.len() as u64));
    block.extend_from_slice(key);
    block.extend_from_slice(value);
    block.extend_from_slice(&0u32.to_le_bytes());
    block.extend_from_slice(&1u32.to_le_bytes());
    block
}

fn table_with_record(key: &str, value: &[u8]) -> Vec<u8> {
    let mut internal = key.as_bytes().to_vec();
    internal.extend_from_slice(&20u64.to_le_bytes());
    let data = block_record(&internal, value);
    let mut data_with_trailer = data.clone();
    data_with_trailer.push(0);
    data_with_trailer.extend_from_slice(&[0; 4]);
    let index_offset = data_with_trailer.len() as u64;
    let index = block_record(b"z", &{
        let mut handle = varint(0);
        handle.extend(varint(data.len() as u64));
        handle
    });
    let mut index_with_trailer = index.clone();
    index_with_trailer.push(0);
    index_with_trailer.extend_from_slice(&[0; 4]);
    let mut table = data_with_trailer;
    table.extend(index_with_trailer);
    let mut footer = Vec::new();
    footer.extend(varint(0));
    footer.extend(varint(0));
    footer.extend(varint(index_offset));
    footer.extend(varint(index.len() as u64));
    footer.resize(40, 0);
    footer.extend_from_slice(&MAGIC.to_le_bytes());
    table.extend(footer);
    table
}

const MAGIC: u64 = 0xdb4775248b80fb57;

fn write_wal(path: &Path, key: &str, value: &[u8], record_type: u8) {
    let mut batch = vec![0; 12];
    batch[8..12].copy_from_slice(&1u32.to_le_bytes());
    batch.push(1);
    batch.extend(varint(key.len() as u64));
    batch.extend(key.as_bytes());
    batch.extend(varint(value.len() as u64));
    batch.extend(value);
    let mut wal = vec![0; 4];
    wal.extend_from_slice(&(batch.len() as u16).to_le_bytes());
    wal.push(record_type);
    wal.extend(batch);
    fs::write(path, wal).unwrap();
}

#[test]
fn bedrock_leveldb_fixture_corpus_is_present() {
    assert_eq!(fixture_count("bedrock-leveldb"), 22);
}

#[test]
fn bedrock_leveldb_reads_tables_then_sorted_wal_overlays() {
    let temp = TempDir::new("overlay");
    fs::write(
        temp.path().join("000010.ldb"),
        table_with_record("player_1", b"table"),
    )
    .unwrap();
    write_wal(&temp.path().join("000002.log"), "player_1", b"older-log", 1);
    write_wal(&temp.path().join("000010.log"), "player_1", b"newer-log", 1);
    let data = bedrock_leveldb::read_player_data(temp.path()).unwrap();
    assert_eq!(
        data.get("player_1").map(Vec::as_slice),
        Some(b"newer-log".as_slice())
    );
}

#[test]
fn bedrock_leveldb_filters_non_player_records_and_handles_missing_directory() {
    let temp = TempDir::new("filter");
    write_wal(&temp.path().join("000001.log"), "chunk_0_0", b"ignored", 1);
    let data = bedrock_leveldb::read_player_data(temp.path()).unwrap();
    assert!(data.is_empty());
    assert!(
        bedrock_leveldb::read_player_data(&temp.path().join("missing"))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn bedrock_leveldb_surfaces_corrupt_and_unsupported_files() {
    let temp = TempDir::new("corrupt");
    fs::write(temp.path().join("000001.ldb"), vec![0; 47]).unwrap();
    assert!(matches!(
        bedrock_leveldb::read_player_data(temp.path()),
        Err(LevelDbError::Corrupt(_))
    ));
    let temp = TempDir::new("wal-corrupt");
    let mut file = fs::File::create(temp.path().join("000001.log")).unwrap();
    file.write_all(&[0, 0, 0, 0, 0xff, 0xff, 1]).unwrap();
    assert!(matches!(
        bedrock_leveldb::read_player_data(temp.path()),
        Err(LevelDbError::Corrupt(_))
    ));
}
