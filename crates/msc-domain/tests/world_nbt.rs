//! Port of `fixtures/world-nbt/`'s 14 fixtures (P6.7).
//!
//! `msc_domain::nbt`'s byte-level `NBTReader`/`NBTValue` machinery is
//! private (see that module's doc comment), so these tests drive it
//! black-box: each case hand-builds the raw `level.dat`-shaped bytes the
//! fixture describes and asserts on [`msc_domain::nbt::imported_world_metadata_from_level_dat`]'s
//! result, the same public entry point `msc-infrastructure`/
//! `msc-application` will call once they read real bytes off disk/out of a
//! zip. Test functions are prefixed `world_nbt_` so the plan's Verify
//! command (a plain nextest substring filter on test name) selects them.

mod support;

use msc_domain::identity::ServerType;
use msc_domain::nbt::{
    ImportedWorldMetadata, first_level_dat_path, imported_world_metadata_from_level_dat,
    merge_sidecar_metadata, trimmed_sidecar_seed,
};
use std::io::Write;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("world-nbt/{case}.json")))
}

// ---- Big-endian (Java) byte-building helpers ----

fn be_nbt_string(s: &str) -> Vec<u8> {
    let mut out = (s.len() as i16).to_be_bytes().to_vec();
    out.extend_from_slice(s.as_bytes());
    out
}

fn be_entry(name: &str, tag: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend(be_nbt_string(name));
    out.extend_from_slice(payload);
    out
}

fn be_int_entry(name: &str, value: i32) -> Vec<u8> {
    be_entry(name, 3, &value.to_be_bytes())
}

/// A compound's payload: concatenated entries, then the end tag — no
/// leading tag/name of its own (the caller's `be_entry` already wrote
/// that for a nested compound field; `be_root_compound` writes it for the
/// unnamed root).
fn be_compound_payload(entries: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for e in entries {
        out.extend_from_slice(e);
    }
    out.push(0);
    out
}

fn be_root_compound(entries: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![10u8];
    out.extend(be_nbt_string(""));
    out.extend(be_compound_payload(entries));
    out
}

/// A real `level.dat`'s NBT root has exactly one top-level key, `Data`,
/// wrapping everything this reader looks at — `extract_seed_string`'s
/// `Data`-prefixed paths only fire against input shaped this way.
fn be_java_root(data_entries: &[Vec<u8>]) -> Vec<u8> {
    be_root_compound(&[be_entry("Data", 10, &be_compound_payload(data_entries))])
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

// ---- Little-endian (Bedrock) byte-building helpers ----

fn le_nbt_string(s: &str) -> Vec<u8> {
    let mut out = (s.len() as i16).to_le_bytes().to_vec();
    out.extend_from_slice(s.as_bytes());
    out
}

fn le_int_entry(name: &str, value: i32) -> Vec<u8> {
    let mut out = vec![3u8];
    out.extend(le_nbt_string(name));
    out.extend(value.to_le_bytes());
    out
}

fn le_root_compound(entries: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![10u8];
    out.extend(le_nbt_string(""));
    for e in entries {
        out.extend_from_slice(e);
    }
    out.push(0);
    out
}

#[test]
fn world_nbt_all_twelve_tag_types_accepted_by_reader() {
    // Every tag type appears as a sibling field alongside the four fields
    // this reader actually extracts — if any tag type broke the reader,
    // the whole compound would fail to parse and every field below would
    // come back `None` instead of its real value.
    let data = be_java_root(&[
        be_int_entry("GameType", 1),
        be_int_entry("Difficulty", 2),
        be_int_entry("RandomSeed", 42),
        be_int_entry("DayTime", 100),
        be_entry("a_byte", 1, &[7]),
        be_entry("a_short", 2, &7i16.to_be_bytes()),
        be_entry("a_long", 4, &7i64.to_be_bytes()),
        be_entry("a_float", 5, &1.5f32.to_be_bytes()),
        be_entry("a_double", 6, &2.5f64.to_be_bytes()),
        be_entry("a_bytearray", 7, &{
            let mut p = 2i32.to_be_bytes().to_vec();
            p.extend_from_slice(&[9, 9]);
            p
        }),
        be_entry("a_string", 8, &be_nbt_string("hello")),
        be_entry("a_list", 9, &{
            let mut p = vec![3u8];
            p.extend(1i32.to_be_bytes());
            p.extend(5i32.to_be_bytes());
            p
        }),
        be_entry(
            "a_nested_compound",
            10,
            &be_compound_payload(&[be_int_entry("inner", 1)]),
        ),
        be_entry("an_intarray", 11, &{
            let mut p = 1i32.to_be_bytes().to_vec();
            p.extend(3i32.to_be_bytes());
            p
        }),
        be_entry("a_longarray", 12, &{
            let mut p = 1i32.to_be_bytes().to_vec();
            p.extend(3i64.to_be_bytes());
            p
        }),
    ]);
    let result = imported_world_metadata_from_level_dat(&gzip(&data), ServerType::Java);
    assert_eq!(result.gamemode.as_deref(), Some("creative"));
    assert_eq!(result.difficulty.as_deref(), Some("normal"));
    assert_eq!(result.seed.as_deref(), Some("42"));
    assert_eq!(result.day_time, Some(100));
}

#[test]
fn world_nbt_root_tag_not_compound_rejected() {
    // First byte 0x03 (int) instead of 0x0A (compound).
    let bytes = [3u8, 0, 0, 0, 0, 0, 1];
    let result = imported_world_metadata_from_level_dat(&gzip(&bytes), ServerType::Java);
    assert_eq!(result, ImportedWorldMetadata::default());
}

#[test]
fn world_nbt_malformed_nbt_truncated_mid_tag_returns_nil() {
    // Root compound header, one child (int "GameType") whose 4-byte
    // payload is cut off after 1 byte.
    let mut bytes = vec![10u8];
    bytes.extend(be_nbt_string(""));
    bytes.push(3);
    bytes.extend(be_nbt_string("GameType"));
    bytes.push(0);
    let result = imported_world_metadata_from_level_dat(&gzip(&bytes), ServerType::Java);
    assert_eq!(result, ImportedWorldMetadata::default());
}

#[test]
fn world_nbt_java_gzip_corrupt_input_fails_before_nbt_parse() {
    let result = imported_world_metadata_from_level_dat(b"not valid gzip", ServerType::Java);
    assert_eq!(result, ImportedWorldMetadata::default());
}

#[test]
fn world_nbt_bedrock_headered_little_endian_8_byte_header_skipped() {
    let payload = le_root_compound(&[
        le_int_entry("GameType", 1),
        le_int_entry("Difficulty", 2),
        le_int_entry("RandomSeed", 123456789),
        le_int_entry("Time", 4000),
    ]);
    let mut raw = vec![0u8; 8]; // version + payload-length header, ignored
    raw.extend(payload);

    let result = imported_world_metadata_from_level_dat(&raw, ServerType::Bedrock);
    assert_eq!(result.seed.as_deref(), Some("123456789"));
    assert_eq!(result.difficulty.as_deref(), Some("normal"));
    assert_eq!(result.gamemode.as_deref(), Some("creative"));
    assert_eq!(result.day_time, Some(4000));
}

#[test]
fn world_nbt_bedrock_unheadered_payload_parsed_directly_as_fallback() {
    let raw = le_root_compound(&[le_int_entry("Time", 999)]);
    let result = imported_world_metadata_from_level_dat(&raw, ServerType::Bedrock);
    assert_eq!(result.day_time, Some(999));
}

#[test]
fn world_nbt_difficulty_enum_all_values_mapped_unmapped_returns_nil() {
    let fixture = load("difficulty-enum-all-values-mapped-unmapped-returns-nil");
    let raws = fixture.input["raw_difficulty_values"].as_array().unwrap();
    let expected = fixture.expected["mapped_strings"].as_array().unwrap();
    for (raw, want) in raws.iter().zip(expected) {
        let data = be_java_root(&[be_int_entry("Difficulty", raw.as_i64().unwrap() as i32)]);
        let result = imported_world_metadata_from_level_dat(&gzip(&data), ServerType::Java);
        assert_eq!(result.difficulty.as_deref(), want.as_str());
    }
}

#[test]
fn world_nbt_gamemode_enum_all_values_mapped_unmapped_returns_nil() {
    let fixture = load("gamemode-enum-all-values-mapped-unmapped-returns-nil");
    let raws = fixture.input["raw_gametype_values"].as_array().unwrap();
    let expected = fixture.expected["mapped_strings"].as_array().unwrap();
    for (raw, want) in raws.iter().zip(expected) {
        let data = be_java_root(&[be_int_entry("GameType", raw.as_i64().unwrap() as i32)]);
        let result = imported_world_metadata_from_level_dat(&gzip(&data), ServerType::Java);
        assert_eq!(result.gamemode.as_deref(), want.as_str());
    }
}

#[test]
fn world_nbt_seed_java_worldgensettings_seed_preferred_over_randomseed() {
    let data = be_java_root(&[
        be_entry(
            "WorldGenSettings",
            10,
            &be_compound_payload(&[be_int_entry("seed", 111)]),
        ),
        be_int_entry("RandomSeed", 222),
    ]);
    let result = imported_world_metadata_from_level_dat(&gzip(&data), ServerType::Java);
    assert_eq!(result.seed.as_deref(), Some("111"));
}

#[test]
fn world_nbt_seed_recursive_findinteger_fallback_when_no_known_path_matches() {
    let deeper = be_compound_payload(&[be_int_entry("RandomSeed", 999)]);
    let nesting = be_compound_payload(&[be_entry("DeeperStill", 10, &deeper)]);
    let data = be_java_root(&[be_entry("SomeUnexpectedNesting", 10, &nesting)]);
    let result = imported_world_metadata_from_level_dat(&gzip(&data), ServerType::Java);
    assert_eq!(result.seed.as_deref(), Some("999"));
}

#[test]
fn world_nbt_zip_member_selection_excludes_macosx_picks_first_listing_match() {
    let fixture = load("zip-member-selection-excludes-macosx-picks-first-listing-match");
    let listing: Vec<&str> = fixture.input["unzip_dash_Z_dash_1_listing_order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let selected = first_level_dat_path(&listing);
    assert_eq!(
        selected.as_deref(),
        fixture.expected["selected_member_path"].as_str()
    );
}

#[test]
fn world_nbt_adjacent_backup_sidecar_seed_overrides_parsed_level_dat_seed() {
    let fixture = load("adjacent-backup-sidecar-seed-overrides-parsed-level-dat-seed");
    let sidecar_raw = fixture.input["sidecar_worldSeed_field"].as_str();
    let sidecar_seed = trimmed_sidecar_seed(sidecar_raw);
    assert_eq!(
        sidecar_seed.is_some(),
        fixture.expected["sidecar_value_trimmed_of_whitespace"]
            .as_bool()
            .unwrap()
    );

    let raw_seed = fixture.input["level_dat_inside_zip_random_seed"]
        .as_i64()
        .unwrap() as i32;
    let data = be_java_root(&[be_int_entry("RandomSeed", raw_seed)]);
    let parsed = imported_world_metadata_from_level_dat(&gzip(&data), ServerType::Java);
    let merged = merge_sidecar_metadata(sidecar_seed, parsed);
    assert_eq!(
        merged.seed.as_deref(),
        fixture.expected["final_seed"].as_str()
    );
}

#[test]
fn world_nbt_java_real_legacy_fields_full_extraction_succeeds() {
    let fixture = load("java-real-legacy-fields-full-extraction-succeeds");
    let sample_path = fixture.input["real_sample"]["file"].as_str().unwrap();
    let raw = std::fs::read(support::fixtures_dir().parent().unwrap().join(sample_path))
        .expect("real sample fixture is committed");
    let result = imported_world_metadata_from_level_dat(&raw, ServerType::Java);
    assert_eq!(result.seed.as_deref(), fixture.expected["seed"].as_str());
    assert_eq!(
        result.difficulty.as_deref(),
        fixture.expected["difficulty"].as_str()
    );
    assert_eq!(
        result.gamemode.as_deref(),
        fixture.expected["gamemode"].as_str()
    );
    assert_eq!(result.day_time, fixture.expected["day_time"].as_i64());
}

#[test]
fn world_nbt_java_real_modern_format_legacy_fields_absent() {
    let fixture = load("java-real-modern-format-legacy-fields-absent");
    let sample_path = fixture.input["real_sample"]["file"].as_str().unwrap();
    let raw = std::fs::read(support::fixtures_dir().parent().unwrap().join(sample_path))
        .expect("real sample fixture is committed");
    let result = imported_world_metadata_from_level_dat(&raw, ServerType::Java);
    assert_eq!(result.seed, None);
    assert_eq!(result.difficulty, None);
    assert_eq!(
        result.gamemode.as_deref(),
        fixture.expected["gamemode"].as_str()
    );
    assert_eq!(result.day_time, fixture.expected["day_time"].as_i64());
}
