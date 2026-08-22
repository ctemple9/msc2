use std::fs;

use msc_application::geyser;
use msc_infrastructure::fs::StdFileSystem;
use uuid::Uuid;

fn server_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("msc-geyser-{}", Uuid::new_v4()));
    fs::create_dir_all(dir.join("plugins/Geyser-Spigot")).unwrap();
    fs::write(dir.join("plugins/Geyser-Spigot.jar"), b"geyser").unwrap();
    fs::write(dir.join("plugins/floodgate-spigot.jar"), b"floodgate").unwrap();
    fs::write(dir.join("plugins/Geyser-Spigot/config.yml"), "bedrock:\n  address: 0.0.0.0 # public listener\n  port: 19132\nremote:\n  bedrock:\n    port: 9999\n").unwrap();
    dir
}

#[test]
fn detects_managed_cross_play_jars_and_patches_only_top_level_bedrock_values() {
    let dir = server_dir();
    let fs = StdFileSystem;
    let installation = geyser::installation(&fs, &dir);
    assert!(installation.geyser_installed);
    assert!(installation.floodgate_installed);

    let config = geyser::update_config(&fs, &dir, Some("192.168.1.10"), Some(19133)).unwrap();
    assert_eq!(config.address, "192.168.1.10");
    assert_eq!(config.port, Some(19133));
    let saved = fs::read_to_string(dir.join("plugins/Geyser-Spigot/config.yml")).unwrap();
    assert!(saved.contains("  address: \"192.168.1.10\" # public listener"));
    assert!(saved.contains("  port: 19133"));
    assert!(saved.contains("    port: 9999"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn refuses_invalid_listener_values_without_touching_geyser_yaml() {
    let dir = server_dir();
    let fs = StdFileSystem;
    let path = geyser::config_path(&dir);
    let before = fs::read_to_string(&path).unwrap();
    assert!(geyser::update_config(&fs, &dir, Some("bad\naddress"), None).is_err());
    assert!(geyser::update_config(&fs, &dir, None, Some(0)).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
    let _ = fs::remove_dir_all(dir);
}
