use msc_application::output_reducer::JavaOutputReducer;
use msc_application::player_profiles::{self, PlayerProfileError};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem, StdFileSystem};
use std::path::Path;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

const SERVER_DIR: &str = "/server";
const VANILLA_DIR: &str = "/server/custom-world/playerdata";
const PAPER_DIR: &str = "/server/custom-world/players/data";
const FIRST_UUID: &str = "11111111-1111-4111-8111-111111111111";
const SECOND_UUID: &str = "22222222-2222-4222-8222-222222222222";

fn put_u16(bytes: &mut Vec<u8>, value: usize) {
    bytes.extend_from_slice(&(value as u16).to_be_bytes());
}

fn put_string(bytes: &mut Vec<u8>, value: &str) {
    put_u16(bytes, value.len());
    bytes.extend_from_slice(value.as_bytes());
}

fn named_tag(bytes: &mut Vec<u8>, tag: u8, name: &str) {
    bytes.push(tag);
    put_string(bytes, name);
}

fn player_dat() -> Vec<u8> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut nbt = Vec::new();
    nbt.push(10);
    put_string(&mut nbt, "");
    named_tag(&mut nbt, 5, "Health");
    nbt.extend_from_slice(&20.0f32.to_be_bytes());
    named_tag(&mut nbt, 9, "Inventory");
    nbt.push(10);
    nbt.extend_from_slice(&1i32.to_be_bytes());
    named_tag(&mut nbt, 1, "Slot");
    nbt.push(0);
    named_tag(&mut nbt, 8, "id");
    put_string(&mut nbt, "minecraft:diamond");
    named_tag(&mut nbt, 1, "Count");
    nbt.push(3);
    nbt.push(0);
    nbt.push(0);

    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(&nbt).unwrap();
    gzip.finish().unwrap()
}

fn reducer_with_online_names(names: &[&str]) -> JavaOutputReducer {
    let mut reducer = JavaOutputReducer::new();
    for name in names {
        reducer.process_line(&format!("[Server thread/INFO]: {name} joined the game"));
    }
    reducer
}

#[test]
fn player_profiles_scan_both_layouts_deduplicates_and_eagerly_reads_data() {
    let first = Uuid::parse_str(FIRST_UUID).unwrap();
    let second = Uuid::parse_str(SECOND_UUID).unwrap();
    let dat = player_dat();
    let fs = FakeFileSystem::new()
        .with_file(
            format!("{SERVER_DIR}/server.properties"),
            b"level-name=custom-world\n".to_vec(),
            false,
        )
        .with_file(format!("{VANILLA_DIR}/{FIRST_UUID}.dat"), dat.clone(), false)
        .with_file(
            format!("{VANILLA_DIR}/{FIRST_UUID}.dat_old"),
            dat.clone(),
            false,
        )
        .with_file(format!("{PAPER_DIR}/{FIRST_UUID}.dat"), dat.clone(), false)
        .with_file(format!("{PAPER_DIR}/{SECOND_UUID}.dat"), dat, false)
        .with_file(
            format!("{SERVER_DIR}/usercache.json"),
            format!("[{{\"name\":\"Alice\",\"uuid\":\"{FIRST_UUID}\"}},{{\"name\":\"Bob\",\"uuid\":\"{SECOND_UUID}\"}}]").into_bytes(),
            false,
        )
        .with_file(
            format!("{SERVER_DIR}/ops.json"),
            format!("[{{\"uuid\":\"{SECOND_UUID}\"}}]").into_bytes(),
            false,
        )
        .with_file(
            format!("{SERVER_DIR}/java_hidden.json"),
            format!("[\"{FIRST_UUID}\"]").into_bytes(),
            false,
        )
        .with_modified(
            format!("{VANILLA_DIR}/{FIRST_UUID}.dat"),
            SystemTime::UNIX_EPOCH + Duration::from_secs(10),
        );

    let reducer = reducer_with_online_names(&["Alice", "bob"]);
    let profiles = player_profiles::load_player_profiles(&fs, Path::new(SERVER_DIR), &reducer)
        .expect("profile scan should succeed");

    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[0].uuid, first);
    assert_eq!(
        profiles[0].dat_file_path,
        Path::new(VANILLA_DIR).join(format!("{FIRST_UUID}.dat"))
    );
    assert_eq!(profiles[0].username.as_deref(), Some("Alice"));
    assert!(profiles[0].is_online, "an exact username match is online");
    assert!(profiles[0].is_hidden);
    assert!(!profiles[0].is_op);
    assert_eq!(profiles[0].stats.as_ref().unwrap().health, 20.0);
    assert_eq!(profiles[0].inventory[0].item_id, "minecraft:diamond");
    assert_eq!(
        profiles[0].last_modified,
        SystemTime::UNIX_EPOCH + Duration::from_secs(10)
    );

    assert_eq!(profiles[1].uuid, second);
    assert!(!profiles[1].is_online, "online matching preserves case");
    assert!(profiles[1].is_op);
    assert!(!profiles[1].is_hidden);
}

#[test]
fn player_profiles_missing_or_malformed_sidecars_are_empty() {
    let uuid = Uuid::parse_str(FIRST_UUID).unwrap();
    let fs = FakeFileSystem::new()
        .with_file(
            format!("{SERVER_DIR}/server.properties"),
            b"level-name=custom-world\n".to_vec(),
            false,
        )
        .with_file(
            format!("{VANILLA_DIR}/{FIRST_UUID}.dat"),
            player_dat(),
            false,
        )
        .with_file(
            format!("{SERVER_DIR}/usercache.json"),
            b"not json".to_vec(),
            false,
        )
        .with_file(format!("{SERVER_DIR}/ops.json"), b"{{".to_vec(), false)
        .with_file(
            format!("{SERVER_DIR}/java_hidden.json"),
            b"[]{}".to_vec(),
            false,
        );

    let profiles = player_profiles::load_player_profiles(
        &fs,
        Path::new(SERVER_DIR),
        &JavaOutputReducer::new(),
    )
    .unwrap();

    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].uuid, uuid);
    assert!(profiles[0].username.is_none());
    assert!(!profiles[0].is_op);
    assert!(!profiles[0].is_hidden);
}

#[test]
fn player_profiles_hidden_sidecar_is_atomic_and_reversible() {
    let uuid = Uuid::parse_str(FIRST_UUID).unwrap();
    let fs = FakeFileSystem::new().with_dir(SERVER_DIR);

    assert!(!player_profiles::is_hidden(
        &fs,
        Path::new(SERVER_DIR),
        &uuid
    ));
    player_profiles::hide(&fs, Path::new(SERVER_DIR), &uuid).unwrap();
    assert!(player_profiles::is_hidden(
        &fs,
        Path::new(SERVER_DIR),
        &uuid
    ));
    assert_eq!(
        fs.read(Path::new(SERVER_DIR).join("java_hidden.json").as_path())
            .unwrap(),
        format!("[\n  \"{FIRST_UUID}\"\n]").as_bytes()
    );

    player_profiles::unhide(&fs, Path::new(SERVER_DIR), &uuid).unwrap();
    assert!(!player_profiles::is_hidden(
        &fs,
        Path::new(SERVER_DIR),
        &uuid
    ));
}

#[test]
fn player_profiles_hidden_write_reports_missing_server_directory() {
    let uuid = Uuid::parse_str(FIRST_UUID).unwrap();
    let fs = StdFileSystem;
    let error = player_profiles::hide(&fs, Path::new("/definitely-not-a-real-server"), &uuid)
        .expect_err("missing parent must not be hidden");
    assert!(matches!(error, PlayerProfileError::Io(_)));
}
