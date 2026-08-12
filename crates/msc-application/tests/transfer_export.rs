//! Port of the export-side fixtures in `fixtures/transfer-package/`
//! (P5.12/P5.13): `exportServerTransfer(to:)`. No MSC 1 test exercises this
//! function — every fixture here was characterized straight from
//! `AppViewModel+ServerTransfer.swift`, per the format doc.

use msc_application::transfer::{
    TransferExportRequest, TransferExportServerInput, export_server_transfer,
};
use msc_domain::app_config_schema::ConfigServer;
use serde_json::Value;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

fn load_fixture(case: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/transfer-package")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()))
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "msc2-transfer-export-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Builds a real directory tree from a fixture's `server_dir_contents`
/// shape (`{"name/": "dir"|"absent", "name": "file"|"absent"}`). Files get
/// placeholder bytes; `server.properties` is overwritten afterward with
/// the fixture's real `server_properties` object, since export parses it.
fn build_server_dir(root: &Path, contents: &Value) {
    for (name, kind) in contents.as_object().unwrap() {
        let kind = kind.as_str().unwrap();
        if kind == "absent" {
            continue;
        }
        let target = root.join(name.trim_end_matches('/'));
        if name.ends_with('/') || kind == "dir" {
            std::fs::create_dir_all(&target).unwrap();
        } else {
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(&target, b"placeholder").unwrap();
        }
    }
}

fn write_server_properties(root: &Path, props: &Value) {
    let mut content = String::new();
    for (key, value) in props.as_object().unwrap() {
        content.push_str(&format!("{key}={}\n", value.as_str().unwrap()));
    }
    std::fs::write(root.join("server.properties"), content).unwrap();
}

/// Decodes `input.server` (already the exact snake_case wire shape
/// `ConfigServer::decode` expects) and re-roots `server_dir`/
/// `paper_jar_path` onto the real temp directory this test built, since
/// the fixture's own paths (`/servers/java/main-smp`) don't exist on disk.
fn config_server_for(fixture: &Value, root: &Path) -> ConfigServer {
    let mut server = ConfigServer::decode(&fixture["input"]["server"]).expect("server decodes");
    server.server_dir = root.to_string_lossy().into_owned();
    if !server.paper_jar_path.is_empty() {
        server.paper_jar_path = root.join("paper.jar").to_string_lossy().into_owned();
    }
    server
}

fn export_fixture(
    case: &str,
) -> (
    Value,
    ConfigServer,
    Vec<u8>,
    msc_application::transfer::TransferManifest,
) {
    let fixture = load_fixture(case);
    let temp = TempDir::new(case);
    build_server_dir(&temp.path, &fixture["input"]["server_dir_contents"]);
    if let Some(props) = fixture["input"].get("server_properties") {
        write_server_properties(&temp.path, props);
    }
    let server = config_server_for(&fixture, &temp.path);

    let sidecar = fixture["input"].get("paper_version_sidecar");
    let paper_mc_version = sidecar
        .and_then(|s| s.get("mc_version"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let paper_build = sidecar.and_then(|s| s.get("build")).and_then(Value::as_i64);

    let request = TransferExportRequest {
        servers: vec![TransferExportServerInput {
            server: server.clone(),
            paper_mc_version,
            paper_build,
        }],
        created_at: "2026-01-01T00:00:00Z".to_string(),
        source_machine_name: "Test Mac".to_string(),
        app_config_version: 1,
    };

    let mut cursor = Cursor::new(Vec::new());
    let manifest = export_server_transfer(&request, &mut cursor).expect("export succeeds");
    (fixture, server, cursor.into_inner(), manifest)
}

fn archive_entry_names(bytes: &[u8]) -> Vec<String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect()
}

fn assert_bundled(names: &[String], prefix: &str, expected: &[&str]) {
    for entry in expected {
        let full = format!("{prefix}/{entry}");
        assert!(
            names.iter().any(|n| n == &full),
            "missing bundled entry {full}, have {names:?}"
        );
    }
}

fn assert_not_bundled(names: &[String], prefix: &str, excluded: &[&str]) {
    for entry in excluded {
        let want_prefix = format!("{prefix}/{entry}");
        assert!(
            !names.iter().any(|n| n.starts_with(&want_prefix)),
            "unexpected bundled entry under {want_prefix}, have {names:?}"
        );
    }
}

#[test]
fn java_paper_full_export_bundles_expected_entries_and_manifest() {
    let (fixture, _server, bytes, manifest) = export_fixture("java-paper-full-export");
    let names = archive_entry_names(&bytes);
    let entry = &manifest.servers[0];
    let prefix = format!("servers/{}", entry.folder_name);

    let expected = &fixture["expected"];
    let bundled: Vec<&str> = expected["bundled_top_level_entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let not_bundled: Vec<&str> = expected["not_bundled"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_bundled(&names, &prefix, &bundled);
    assert_not_bundled(&names, &prefix, &not_bundled);
    assert!(names.iter().any(|n| n == "manifest.json"));

    assert_eq!(entry.folder_name, "main_smp");
    assert_eq!(entry.java_port, Some(25565));
    assert_eq!(entry.paper_mc_version.as_deref(), Some("1.21.4"));
    assert_eq!(entry.paper_build, Some(130));
    assert!(entry.bundled_paper_jar);
    assert_eq!(entry.plugin_links.len(), 1);
    assert_eq!(entry.plugin_links[0].filename, "EssentialsX");
    assert_eq!(
        entry.plugin_links[0].url,
        "https://example.com/essentialsx.jar"
    );
    assert_eq!(entry.plugin_links[0].plugin_type, "modrinth");

    assert_eq!(entry.server.server_dir, "");
    assert_eq!(entry.server.paper_jar_path, "");
    assert_eq!(entry.server.xbox_broadcast_config_path, None);
    assert_eq!(entry.server.xbox_broadcast_alt_email, None);
    assert_eq!(entry.server.xbox_broadcast_alt_gamertag, None);
    assert_eq!(entry.server.xbox_broadcast_alt_password, None);
    assert_eq!(entry.server.xbox_broadcast_alt_avatar_path, None);

    assert_eq!(manifest.format_version, 2);
    assert_eq!(manifest.app_config_version, 1);

    // Round-trip: the bytes actually written as manifest.json decode back
    // to the same manifest this function returned.
    let mut archive = ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut manifest_entry = archive.by_name("manifest.json").unwrap();
    let mut manifest_text = String::new();
    std::io::Read::read_to_string(&mut manifest_entry, &mut manifest_text).unwrap();
    let manifest_value: Value = serde_json::from_str(&manifest_text).unwrap();
    let decoded = msc_application::transfer::TransferManifest::decode(&manifest_value).unwrap();
    assert_eq!(decoded, manifest);
}

#[test]
fn bedrock_worlds_export_bundles_worlds_not_java_entries() {
    let (fixture, _server, bytes, manifest) = export_fixture("bedrock-worlds-export");
    let names = archive_entry_names(&bytes);
    let entry = &manifest.servers[0];
    let prefix = format!("servers/{}", entry.folder_name);

    let bundled: Vec<&str> = fixture["expected"]["bundled_top_level_entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_bundled(&names, &prefix, &bundled);
    assert_not_bundled(
        &names,
        &prefix,
        &[
            "paper.jar",
            "configs/",
            "world/",
            "world_nether/",
            "world_the_end/",
        ],
    );

    assert_eq!(entry.java_port, None);
    assert!(!entry.bundled_paper_jar);
}

#[test]
fn forge_libraries_bundled_only_for_forge_family_flavor() {
    let (fixture, _server, bytes, manifest) = export_fixture("forge-libraries-bundled");
    let names = archive_entry_names(&bytes);
    let entry = &manifest.servers[0];
    let prefix = format!("servers/{}", entry.folder_name);

    let bundled: Vec<&str> = fixture["expected"]["bundled_top_level_entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let not_bundled: Vec<&str> = fixture["expected"]["not_bundled"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_bundled(&names, &prefix, &bundled);
    assert_not_bundled(&names, &prefix, &not_bundled);

    assert!(!entry.bundled_paper_jar);
    assert_eq!(entry.java_port, Some(25566));
    assert_eq!(entry.paper_mc_version, None);
    assert_eq!(entry.paper_build, None);
}

#[test]
fn no_bundled_paper_jar_when_jar_missing_on_disk() {
    let (fixture, _server, bytes, manifest) = export_fixture("no-bundled-paper-jar");
    let names = archive_entry_names(&bytes);
    let entry = &manifest.servers[0];
    let prefix = format!("servers/{}", entry.folder_name);

    let bundled: Vec<&str> = fixture["expected"]["bundled_top_level_entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_bundled(&names, &prefix, &bundled);
    assert_not_bundled(&names, &prefix, &["paper.jar"]);

    assert!(!entry.bundled_paper_jar);
    assert_eq!(entry.java_port, Some(25577));
}

#[test]
fn unique_folder_names_dedupe_on_collision_within_one_export() {
    let temp_a = TempDir::new("dedupe-a");
    let temp_b = TempDir::new("dedupe-b");
    std::fs::write(temp_a.path.join("server.properties"), "server-port=25000\n").unwrap();
    std::fs::write(temp_b.path.join("server.properties"), "server-port=25001\n").unwrap();

    let server_a = ConfigServer::new(
        "srv-a",
        "Main SMP!!",
        temp_a.path.to_string_lossy(),
        "",
        4.0,
        4.0,
    );
    let server_b = ConfigServer::new(
        "srv-b",
        "Main SMP??",
        temp_b.path.to_string_lossy(),
        "",
        4.0,
        4.0,
    );

    let request = TransferExportRequest {
        servers: vec![
            TransferExportServerInput {
                server: server_a,
                paper_mc_version: None,
                paper_build: None,
            },
            TransferExportServerInput {
                server: server_b,
                paper_mc_version: None,
                paper_build: None,
            },
        ],
        created_at: "2026-01-01T00:00:00Z".to_string(),
        source_machine_name: "Test Mac".to_string(),
        app_config_version: 1,
    };

    let mut cursor = Cursor::new(Vec::new());
    let manifest = export_server_transfer(&request, &mut cursor).expect("export succeeds");
    assert_eq!(manifest.servers[0].folder_name, "main_smp");
    assert_eq!(manifest.servers[1].folder_name, "main_smp-2");
}
