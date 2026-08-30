use msc_application::bedrock_runtime::{BedrockRuntimeBackend, BedrockRuntimeCapabilities};
use msc_application::bedrock_service::{BedrockImportReadiness, reconcile_bedrock_import};
use msc_application::provisioning::{
    BedrockCreateRequest, BedrockWorldSource, create_bedrock_server,
};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::identity::ServerType;
use msc_domain::world_profile::WorldProfile;
use msc_infrastructure::fs::StdFileSystem;
use std::fs;
use std::path::{Path, PathBuf};

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "msc2-bedrock-imports-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
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

fn config(path: &Path) -> ConfigServer {
    let mut config = ConfigServer::new(
        "bedrock-1",
        "Imported Bedrock",
        path.to_string_lossy(),
        "",
        0.0,
        0.0,
    );
    config.server_type = ServerType::Bedrock;
    config.bedrock_port = Some(19999);
    config
}

fn supported() -> BedrockRuntimeCapabilities {
    BedrockRuntimeCapabilities::supported(BedrockRuntimeBackend::Native)
}

fn properties() -> &'static [u8] {
    b"server-name=Imported\nlevel-name=Realm\nmax-players=14\nserver-port=19140\nserver-portv6=19141\nonline-mode=false\nallow-cheats=true\ndifficulty=normal\ngamemode=creative\n"
}

#[test]
fn imported_bedrock_record_uses_directory_settings_and_host_capability() {
    let temp = TempDir::new("reconcile");
    fs::write(temp.path().join("bedrock_server"), b"fake bds").unwrap();
    fs::write(temp.path().join("server.properties"), properties()).unwrap();

    let reconciled =
        reconcile_bedrock_import(&StdFileSystem, &config(temp.path()), &supported()).unwrap();

    assert_eq!(reconciled.config.bedrock_port, Some(19140));
    assert_eq!(reconciled.directory.as_ref().unwrap().level_name, "Realm");
    assert_eq!(reconciled.directory.as_ref().unwrap().max_players, 14);
    assert_eq!(reconciled.directory.as_ref().unwrap().server_port_v6, 19141);
    assert!(!reconciled.directory.as_ref().unwrap().online_mode);
    assert!(reconciled.directory.as_ref().unwrap().allow_cheats);
    assert_eq!(reconciled.readiness, BedrockImportReadiness::Ready);
}

#[test]
fn imported_bedrock_record_reports_unavailable_without_executable_or_backend() {
    let temp = TempDir::new("unavailable");
    fs::write(temp.path().join("server.properties"), properties()).unwrap();

    let missing_executable =
        reconcile_bedrock_import(&StdFileSystem, &config(temp.path()), &supported()).unwrap();
    assert!(matches!(
        missing_executable.readiness,
        BedrockImportReadiness::Unavailable { .. }
    ));

    fs::write(temp.path().join("bedrock_server"), b"fake bds").unwrap();
    let unavailable_host = BedrockRuntimeCapabilities::unavailable(
        BedrockRuntimeBackend::Sidecar,
        "Intel macOS sidecar is unavailable",
    );
    let reconciled =
        reconcile_bedrock_import(&StdFileSystem, &config(temp.path()), &unavailable_host).unwrap();
    assert_eq!(
        reconciled.readiness,
        BedrockImportReadiness::Unavailable {
            reason: "Intel macOS sidecar is unavailable".to_owned()
        }
    );
}

#[test]
fn fresh_bedrock_creation_writes_native_config_and_active_slot_transactionally() {
    let temp = TempDir::new("fresh");
    let request = BedrockCreateRequest {
        name: "  Survival Realm  ",
        initial_world_name: Some("New Realm"),
        bedrock_version: Some("1.21.80.3"),
        port: 19132,
        max_players: 20,
        enable_playit: true,
        enable_xbox_broadcast: false,
        difficulty: "easy",
        gamemode: "survival",
        world_seed: Some(" 12345 "),
        initial_world_profile: None,
        world_source: BedrockWorldSource::Fresh,
    };

    let created = create_bedrock_server(
        &StdFileSystem,
        temp.path(),
        &request,
        "2026-08-23T12:00:00Z",
    )
    .unwrap();
    let server_dir = PathBuf::from(&created.config.server_dir);
    assert_eq!(created.config.server_type, ServerType::Bedrock);
    assert_eq!(created.config.bedrock_port, Some(19132));
    assert_eq!(created.config.bedrock_version.as_deref(), Some("1.21.80.3"));
    assert_eq!(created.world_slot.name, "New Realm");
    assert_eq!(
        fs::read_to_string(server_dir.join("server.properties")).unwrap(),
        "# Modified via MSC 2\nallow-cheats=false\ndifficulty=easy\ngamemode=survival\nlevel-name=New Realm\nlevel-seed=12345\nmax-players=20\nonline-mode=true\nserver-name=Survival Realm\nserver-port=19132\nserver-portv6=19133\n"
    );
    assert_eq!(
        fs::read_to_string(server_dir.join("allowlist.json")).unwrap(),
        "[]\n"
    );
    assert_eq!(
        fs::read_to_string(server_dir.join("world_slots/active_slot_id.txt")).unwrap(),
        format!("{}\n", created.world_slot.id)
    );
}

#[test]
fn fresh_bedrock_creation_applies_the_first_world_profile() {
    let temp = TempDir::new("fresh-profile");
    let mut profile = WorldProfile::new();
    profile.identity.name = Some("Configured Realm".to_string());
    profile.identity.level_name = Some("configured_level".to_string());
    profile.identity.seed = Some("bedrock-seed".to_string());
    profile.generation.structures = Some(false);
    profile.gameplay.difficulty = Some("hard".to_string());
    profile.gameplay.default_game_mode = Some("creative".to_string());
    profile.gameplay.cheats = Some(true);
    profile.gameplay.coordinates = Some(false);
    profile.gameplay.starting_map = Some(true);

    let request = BedrockCreateRequest {
        name: "Survival Realm",
        initial_world_name: None,
        bedrock_version: Some("1.21.80.3"),
        port: 19132,
        max_players: 20,
        enable_playit: false,
        enable_xbox_broadcast: false,
        difficulty: "normal",
        gamemode: "survival",
        world_seed: None,
        initial_world_profile: Some(&profile),
        world_source: BedrockWorldSource::Fresh,
    };

    let created = create_bedrock_server(
        &StdFileSystem,
        temp.path(),
        &request,
        "2026-08-23T12:00:00Z",
    )
    .unwrap();
    let server_dir = PathBuf::from(&created.config.server_dir);
    let properties = fs::read_to_string(server_dir.join("server.properties")).unwrap();
    assert!(properties.contains("level-name=configured_level\n"));
    assert!(properties.contains("level-seed=bedrock-seed\n"));
    assert!(properties.contains("difficulty=hard\n"));
    assert!(properties.contains("gamemode=creative\n"));
    assert!(properties.contains("allow-cheats=true\n"));
    assert!(properties.contains("show-coordinates=false\n"));
    assert!(properties.contains("starting-map=true\n"));
    assert_eq!(created.world_slot.name, "Configured Realm");
    assert_eq!(
        created.world_slot.world_seed.as_deref(),
        Some("bedrock-seed")
    );
}

#[test]
fn existing_bedrock_world_wrapper_is_unwrapped_and_archived() {
    let temp = TempDir::new("existing");
    let source = temp.path().join("export");
    let world = source.join("Realm");
    fs::create_dir_all(world.join("db")).unwrap();
    fs::write(world.join("level.dat"), b"not an NBT file").unwrap();
    fs::write(world.join("db/chunk"), b"chunk").unwrap();

    let request = BedrockCreateRequest {
        name: "Imported Realm",
        initial_world_name: None,
        bedrock_version: None,
        port: 19132,
        max_players: 10,
        enable_playit: false,
        enable_xbox_broadcast: false,
        difficulty: "easy",
        gamemode: "survival",
        world_seed: Some("ignored for imports"),
        initial_world_profile: None,
        world_source: BedrockWorldSource::ExistingFolder(&source),
    };
    let created = create_bedrock_server(
        &StdFileSystem,
        temp.path(),
        &request,
        "2026-08-23T12:00:00Z",
    )
    .unwrap();
    let server_dir = PathBuf::from(&created.config.server_dir);
    assert!(server_dir.join("worlds/Realm/level.dat").is_file());
    assert!(
        server_dir
            .join("world_slots")
            .join(&created.world_slot.id)
            .join("world.zip")
            .is_file()
    );
    assert!(
        fs::read_to_string(server_dir.join("server.properties"))
            .unwrap()
            .contains("level-name=Realm\n")
    );
}

#[test]
fn failed_bedrock_creation_removes_candidate_directory() {
    let temp = TempDir::new("rollback");
    let source = temp.path().join("ambiguous");
    fs::create_dir_all(source.join("One")).unwrap();
    fs::create_dir_all(source.join("Two")).unwrap();
    fs::write(source.join("One/level.dat"), b"one").unwrap();
    fs::write(source.join("Two/level.dat"), b"two").unwrap();

    let request = BedrockCreateRequest {
        name: "Broken Import",
        initial_world_name: None,
        bedrock_version: None,
        port: 19132,
        max_players: 10,
        enable_playit: false,
        enable_xbox_broadcast: false,
        difficulty: "easy",
        gamemode: "survival",
        world_seed: None,
        initial_world_profile: None,
        world_source: BedrockWorldSource::ExistingFolder(&source),
    };
    assert!(
        create_bedrock_server(
            &StdFileSystem,
            temp.path(),
            &request,
            "2026-08-23T12:00:00Z"
        )
        .is_err()
    );
    assert!(!temp.path().join("bedrock/broken_import").exists());
}
