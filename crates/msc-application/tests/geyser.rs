use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;

use msc_application::geyser;
use msc_infrastructure::download_staging::sha256_hex;
use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::geyser::{GeyserProject, latest_build_url};
use msc_infrastructure::helper_acquisition::HelperPlatform;
use msc_infrastructure::jar_provider::{JarProviderError, Transport};
use serde_json::json;
use uuid::Uuid;

fn server_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("msc-geyser-{}", Uuid::new_v4()));
    fs::create_dir_all(dir.join("plugins/Geyser-Spigot")).unwrap();
    fs::write(dir.join("plugins/Geyser-Spigot.jar"), b"geyser").unwrap();
    fs::write(dir.join("plugins/floodgate-spigot.jar"), b"floodgate").unwrap();
    fs::write(dir.join("plugins/Geyser-Spigot/config.yml"), "bedrock:\n  address: 0.0.0.0 # public listener\n  port: 19132\nremote:\n  bedrock:\n    port: 9999\n").unwrap();
    dir
}

struct FakeTransport {
    responses: Mutex<HashMap<String, Vec<u8>>>,
}

impl FakeTransport {
    fn for_project(project: GeyserProject, version: &str, build: u64, bytes: &[u8]) -> Self {
        let asset_url = format!(
            "https://download.geysermc.org/v2/projects/{}/versions/{version}/builds/{build}/downloads/spigot",
            project.api_name()
        );
        Self {
            responses: Mutex::new(HashMap::from([
                (
                    latest_build_url(project),
                    serde_json::to_vec(&json!({
                        "version": version,
                        "build": build,
                        "downloads": { "spigot": { "sha256": sha256_hex(bytes) } }
                    }))
                    .unwrap(),
                ),
                (asset_url, bytes.to_vec()),
            ])),
        }
    }
}

impl Transport for FakeTransport {
    fn get(&self, url: &str, what: &str, max_bytes: u64) -> Result<Vec<u8>, JarProviderError> {
        let bytes = self
            .responses
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or_else(|| JarProviderError::Network(format!("{what}: no response for {url}")))?;
        if bytes.len() as u64 > max_bytes {
            return Err(JarProviderError::ResponseTooLarge {
                what: what.into(),
                max_bytes,
            });
        }
        Ok(bytes)
    }
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

#[test]
fn installs_latest_geyser_after_upstream_checksum_verification() {
    let dir = server_dir();
    let cache = dir.join("helper-cache");
    let bytes = b"new verified geyser jar";
    let transport = FakeTransport::for_project(GeyserProject::Geyser, "2.11.2", 42, bytes);

    let installed = geyser::install_latest(
        &StdFileSystem,
        &transport,
        &cache,
        &dir,
        GeyserProject::Geyser,
        HelperPlatform::LinuxX86_64,
    )
    .unwrap();

    assert_eq!(installed.build.display_version(), "2.11.2 (build 42)");
    assert_eq!(installed.plugin_path, dir.join("plugins/Geyser-Spigot.jar"));
    assert_eq!(fs::read(&installed.plugin_path).unwrap(), bytes);
    assert_eq!(
        installed.acquired.metadata.checksum_source,
        msc_infrastructure::helper_acquisition::ChecksumSource::UpstreamPublished
    );
    let _ = fs::remove_dir_all(dir);
}
