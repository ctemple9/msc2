//! P8.17's own tests: direct unit coverage over `msc_application::addons`,
//! the real install/update/toggle/remove/source-linking mutations built on
//! P8.14's verified storage and P8.15's dependency installer.

use std::collections::HashMap;
use std::path::Path;

use msc_application::addon_updates::AddonUpdateItem;
use msc_application::addons::{self, AddonMutationError};
use msc_domain::addon_provider::ModrinthVersionInfo;
use msc_domain::addon_update::AddonUpdateBucket;
use msc_domain::app_config_schema::{AddonLink, PluginSourceConfig, PluginSourceKind};
use msc_domain::identity::JavaServerFlavor;
use msc_infrastructure::addon_provider::{AddonTransport, RawResponse, TransportError};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};

/// Routes both GET (jar/asset downloads, project/version lookups) and POST
/// (Modrinth batch endpoints, unused here) by exact URL or URL-shape,
/// matching the established `tests/addon_dependencies.rs` convention. A
/// lookup miss panics — proves a test's "this should never be called"
/// expectations (e.g. pack-managed refusal never reaching the network).
struct FakeTransport {
    gets: HashMap<String, (u16, Vec<u8>)>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            gets: HashMap::new(),
        }
    }
    fn with_get(mut self, url: &str, status: u16, body: Vec<u8>) -> Self {
        self.gets.insert(url.to_string(), (status, body));
        self
    }
    fn with_json(self, url: &str, body: serde_json::Value) -> Self {
        self.with_get(url, 200, serde_json::to_vec(&body).unwrap())
    }
}

impl AddonTransport for FakeTransport {
    fn get(
        &self,
        url: &str,
        what: &str,
        _headers: &[(&str, &str)],
        _max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        let (status, body) = self
            .gets
            .get(url)
            .unwrap_or_else(|| panic!("{what}: no fake response registered for {url}"));
        Ok(RawResponse {
            status: *status,
            body: body.clone(),
        })
    }

    fn post_json(
        &self,
        url: &str,
        what: &str,
        _body: &serde_json::Value,
        headers: &[(&str, &str)],
        max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        self.get(url, what, headers, max_bytes)
    }
}

fn version(
    id: &str,
    project_id: &str,
    version_number: &str,
    filename: &str,
    url: &str,
) -> ModrinthVersionInfo {
    let json = serde_json::json!({
        "id": id,
        "project_id": project_id,
        "version_number": version_number,
        "files": [{
            "url": url,
            "filename": filename,
            "primary": true,
            "hashes": {},
            "size": 0,
        }],
        "dependencies": [],
    });
    serde_json::from_value(json).unwrap()
}

fn item_with_update(
    filename: &str,
    is_enabled: bool,
    available: ModrinthVersionInfo,
) -> AddonUpdateItem {
    AddonUpdateItem {
        filename: filename.to_string(),
        jar_stem: filename.trim_end_matches(".jar").to_string(),
        is_enabled,
        display_name: filename.to_string(),
        project_id: Some(available.project_id.clone()),
        provenance: None,
        tier: None,
        bucket: AddonUpdateBucket::UpdateAvailable,
        available_version_id: Some(available.id.clone()),
        available_version_label: Some(available.version_number.clone()),
        available_version: Some(available),
    }
}

fn write_jar(fs: &FakeFileSystem, dir: &Path, filename: &str, contents: &[u8]) {
    fs.create_dir_all(dir).unwrap();
    fs.write(&dir.join(filename), contents).unwrap();
}

// --- install_from_catalog ---

#[test]
fn install_from_catalog_writes_the_primary_file() {
    let fs = FakeFileSystem::new();
    let server_dir = Path::new("/server");
    let v = version(
        "v1",
        "sodium-proj",
        "0.5",
        "sodium-0.5.jar",
        "https://cdn.example.invalid/sodium-0.5.jar",
    );
    let transport = FakeTransport::new().with_get(
        "https://cdn.example.invalid/sodium-0.5.jar",
        200,
        b"jar bytes".to_vec(),
    );

    let outcome = addons::install_from_catalog(
        &transport,
        &fs,
        server_dir,
        JavaServerFlavor::Fabric,
        &v,
        Some("1.21.1"),
        &[],
        false,
        &|| false,
    )
    .unwrap();

    assert_eq!(
        outcome.installed_path,
        Path::new("/server/mods/sodium-0.5.jar")
    );
    assert_eq!(fs.read(&outcome.installed_path).unwrap(), b"jar bytes");
    assert!(outcome.dependencies.results.is_empty());
}

#[test]
fn install_from_catalog_refused_on_pack_managed_server_without_touching_network() {
    let fs = FakeFileSystem::new();
    let server_dir = Path::new("/server");
    let v = version(
        "v1",
        "p",
        "1.0",
        "x.jar",
        "https://cdn.example.invalid/x.jar",
    );
    // No fake response registered at all — a real attempt would panic.
    let transport = FakeTransport::new();

    let result = addons::install_from_catalog(
        &transport,
        &fs,
        server_dir,
        JavaServerFlavor::Fabric,
        &v,
        None,
        &[],
        true,
        &|| false,
    );
    assert!(matches!(result, Err(AddonMutationError::PackManaged)));
}

#[test]
fn install_from_catalog_vanilla_has_no_add_on_folder() {
    let fs = FakeFileSystem::new();
    let v = version(
        "v1",
        "p",
        "1.0",
        "x.jar",
        "https://cdn.example.invalid/x.jar",
    );
    let transport = FakeTransport::new();
    let result = addons::install_from_catalog(
        &transport,
        &fs,
        Path::new("/server"),
        JavaServerFlavor::Vanilla,
        &v,
        None,
        &[],
        false,
        &|| false,
    );
    assert!(matches!(result, Err(AddonMutationError::NoAddOnKind)));
}

// --- install_from_staged_local_jar ---

#[test]
fn install_from_staged_local_jar_copies_verbatim() {
    let fs = FakeFileSystem::new();
    fs.create_dir_all(Path::new("/staging")).unwrap();
    fs.write(Path::new("/staging/upload.jar"), b"local jar bytes")
        .unwrap();

    let dest = addons::install_from_staged_local_jar(
        &fs,
        Path::new("/server"),
        JavaServerFlavor::Fabric,
        Path::new("/staging/upload.jar"),
        "MyMod-1.0.jar",
        false,
    )
    .unwrap();

    assert_eq!(dest, Path::new("/server/mods/MyMod-1.0.jar"));
    assert_eq!(fs.read(&dest).unwrap(), b"local jar bytes");
}

#[test]
fn install_from_staged_local_jar_refused_on_pack_managed_server() {
    let fs = FakeFileSystem::new();
    fs.create_dir_all(Path::new("/staging")).unwrap();
    fs.write(Path::new("/staging/upload.jar"), b"x").unwrap();
    let result = addons::install_from_staged_local_jar(
        &fs,
        Path::new("/server"),
        JavaServerFlavor::Fabric,
        Path::new("/staging/upload.jar"),
        "MyMod-1.0.jar",
        true,
    );
    assert!(matches!(result, Err(AddonMutationError::PackManaged)));
}

// --- update_one / update_all ---

#[test]
fn update_one_same_filename_overwrites_in_place() {
    let fs = FakeFileSystem::new();
    let server_dir = Path::new("/server");
    write_jar(
        &fs,
        &server_dir.join("mods"),
        "sodium-0.5.jar",
        b"old bytes",
    );
    let v = version(
        "v2",
        "sodium-proj",
        "0.6",
        "sodium-0.5.jar", // same filename on purpose
        "https://cdn.example.invalid/sodium-0.5.jar",
    );
    let transport = FakeTransport::new().with_get(
        "https://cdn.example.invalid/sodium-0.5.jar",
        200,
        b"new bytes".to_vec(),
    );
    let item = item_with_update("sodium-0.5.jar", true, v);

    let outcome = addons::update_one(
        &transport,
        &fs,
        server_dir,
        JavaServerFlavor::Fabric,
        &item,
        Some("1.21.1"),
        &[],
        false,
        &|| false,
    )
    .unwrap();

    assert_eq!(fs.read(&outcome.installed_path).unwrap(), b"new bytes");
}

#[test]
fn update_one_filename_change_removes_stale_file() {
    let fs = FakeFileSystem::new();
    let server_dir = Path::new("/server");
    write_jar(
        &fs,
        &server_dir.join("mods"),
        "sodium-0.5.jar",
        b"old bytes",
    );
    let v = version(
        "v2",
        "sodium-proj",
        "0.6",
        "sodium-0.6.jar",
        "https://cdn.example.invalid/sodium-0.6.jar",
    );
    let transport = FakeTransport::new().with_get(
        "https://cdn.example.invalid/sodium-0.6.jar",
        200,
        b"new bytes".to_vec(),
    );
    let item = item_with_update("sodium-0.5.jar", true, v);

    let outcome = addons::update_one(
        &transport,
        &fs,
        server_dir,
        JavaServerFlavor::Fabric,
        &item,
        None,
        &[],
        false,
        &|| false,
    )
    .unwrap();

    assert_eq!(
        outcome.installed_path,
        server_dir.join("mods/sodium-0.6.jar")
    );
    assert!(fs.stat(&server_dir.join("mods/sodium-0.5.jar")).is_err());
    assert!(fs.stat(&outcome.installed_path).is_ok());
}

#[test]
fn update_one_preserves_disabled_state_across_replacement() {
    let fs = FakeFileSystem::new();
    let server_dir = Path::new("/server");
    write_jar(
        &fs,
        &server_dir.join("mods"),
        "sodium-0.5.jar.disabled",
        b"old bytes",
    );
    let v = version(
        "v2",
        "sodium-proj",
        "0.6",
        "sodium-0.6.jar",
        "https://cdn.example.invalid/sodium-0.6.jar",
    );
    let transport = FakeTransport::new().with_get(
        "https://cdn.example.invalid/sodium-0.6.jar",
        200,
        b"new bytes".to_vec(),
    );
    let item = item_with_update("sodium-0.5.jar", false, v);

    let outcome = addons::update_one(
        &transport,
        &fs,
        server_dir,
        JavaServerFlavor::Fabric,
        &item,
        None,
        &[],
        false,
        &|| false,
    )
    .unwrap();

    assert_eq!(
        outcome.installed_path,
        server_dir.join("mods/sodium-0.6.jar.disabled")
    );
}

#[test]
fn update_one_refused_on_pack_managed_server() {
    let fs = FakeFileSystem::new();
    let v = version(
        "v2",
        "p",
        "0.6",
        "x.jar",
        "https://cdn.example.invalid/x.jar",
    );
    let item = item_with_update("x.jar", true, v);
    let transport = FakeTransport::new();
    let result = addons::update_one(
        &transport,
        &fs,
        Path::new("/server"),
        JavaServerFlavor::Fabric,
        &item,
        None,
        &[],
        true,
        &|| false,
    );
    assert!(matches!(result, Err(AddonMutationError::PackManaged)));
}

#[test]
fn update_one_no_update_available_when_item_carries_none() {
    let fs = FakeFileSystem::new();
    let item = AddonUpdateItem {
        filename: "x.jar".to_string(),
        jar_stem: "x".to_string(),
        is_enabled: true,
        display_name: "x".to_string(),
        project_id: None,
        provenance: None,
        tier: None,
        bucket: AddonUpdateBucket::Unlinked,
        available_version_id: None,
        available_version_label: None,
        available_version: None,
    };
    let transport = FakeTransport::new();
    let result = addons::update_one(
        &transport,
        &fs,
        Path::new("/server"),
        JavaServerFlavor::Fabric,
        &item,
        None,
        &[],
        false,
        &|| false,
    );
    assert!(matches!(result, Err(AddonMutationError::NoUpdateAvailable)));
}

#[test]
fn update_all_continues_past_a_per_item_failure() {
    let fs = FakeFileSystem::new();
    let server_dir = Path::new("/server");
    write_jar(&fs, &server_dir.join("mods"), "a.jar", b"old a");
    write_jar(&fs, &server_dir.join("mods"), "b.jar", b"old b");

    let v_a = version(
        "va",
        "proj-a",
        "2.0",
        "a.jar",
        "https://cdn.example.invalid/a.jar",
    );
    let v_b = version(
        "vb",
        "proj-b",
        "2.0",
        "b.jar",
        "https://cdn.example.invalid/b.jar",
    );
    // a.jar's download succeeds; b.jar's returns a server error.
    let transport = FakeTransport::new()
        .with_get("https://cdn.example.invalid/a.jar", 200, b"new a".to_vec())
        .with_get("https://cdn.example.invalid/b.jar", 500, Vec::new());

    let items = vec![
        item_with_update("a.jar", true, v_a),
        item_with_update("b.jar", true, v_b),
    ];

    let results = addons::update_all(
        &transport,
        &fs,
        server_dir,
        JavaServerFlavor::Fabric,
        &items,
        None,
        &[],
        false,
        &|| false,
    );

    assert_eq!(results.len(), 2);
    assert!(results[0].outcome.is_ok());
    assert!(results[1].outcome.is_err());
    // a.jar's own success wasn't rolled back by b.jar's later failure.
    assert_eq!(fs.read(&server_dir.join("mods/a.jar")).unwrap(), b"new a");
}

// --- toggle / remove ---

#[test]
fn toggle_flips_enabled_to_disabled() {
    let fs = FakeFileSystem::new();
    write_jar(&fs, Path::new("/server/mods"), "x.jar", b"bytes");
    let new_path = addons::toggle(&fs, Path::new("/server/mods/x.jar"), false).unwrap();
    assert_eq!(new_path, Path::new("/server/mods/x.jar.disabled"));
    assert!(fs.stat(Path::new("/server/mods/x.jar")).is_err());
    assert!(fs.stat(&new_path).is_ok());
}

#[test]
fn toggle_refused_on_pack_managed_server() {
    let fs = FakeFileSystem::new();
    write_jar(&fs, Path::new("/server/mods"), "x.jar", b"bytes");
    let result = addons::toggle(&fs, Path::new("/server/mods/x.jar"), true);
    assert!(matches!(result, Err(AddonMutationError::PackManaged)));
    // Untouched.
    assert!(fs.stat(Path::new("/server/mods/x.jar")).is_ok());
}

#[test]
fn remove_deletes_the_jar() {
    let fs = FakeFileSystem::new();
    write_jar(&fs, Path::new("/server/mods"), "x.jar", b"bytes");
    addons::remove(&fs, Path::new("/server/mods/x.jar"), false).unwrap();
    assert!(fs.stat(Path::new("/server/mods/x.jar")).is_err());
}

#[test]
fn remove_refused_on_pack_managed_server() {
    let fs = FakeFileSystem::new();
    write_jar(&fs, Path::new("/server/mods"), "x.jar", b"bytes");
    let result = addons::remove(&fs, Path::new("/server/mods/x.jar"), true);
    assert!(matches!(result, Err(AddonMutationError::PackManaged)));
    assert!(fs.stat(Path::new("/server/mods/x.jar")).is_ok());
}

// --- manual Modrinth link / plugin-source set-remove ---

#[test]
fn manual_addon_link_set_and_remove() {
    let mut links: HashMap<String, AddonLink> = HashMap::new();
    addons::set_manual_addon_link(
        &mut links,
        "proj-x",
        Some("Project X".to_string()),
        Some("project-x".to_string()),
    );
    assert!(links.contains_key("proj-x"));
    assert_eq!(links["proj-x"].title.as_deref(), Some("Project X"));

    addons::remove_addon_link(&mut links, "proj-x");
    assert!(!links.contains_key("proj-x"));
}

#[test]
fn plugin_source_set_and_remove_round_trip() {
    let mut sources: HashMap<String, PluginSourceConfig> = HashMap::new();
    addons::set_plugin_source(
        &mut sources,
        "LuckPerms-5.4",
        PluginSourceConfig {
            url: "https://example.invalid/luckperms".to_string(),
            source_type: PluginSourceKind::Direct,
            extra: Default::default(),
        },
    );
    assert!(sources.contains_key("LuckPerms-5.4"));

    let sources = addons::remove_plugin_source(sources, "LuckPerms-5.4").unwrap_or_default();
    assert!(!sources.contains_key("LuckPerms-5.4"));
}

// --- update_plugin_from_source ---

#[test]
fn update_plugin_from_source_direct_downloads_and_rekeys() {
    let fs = FakeFileSystem::new();
    let plugins_dir = Path::new("/server/plugins");
    write_jar(&fs, plugins_dir, "MyPlugin-1.0.jar", b"old bytes");
    let mut sources = HashMap::new();
    sources.insert(
        "MyPlugin-1.0".to_string(),
        PluginSourceConfig {
            url: "https://example.invalid/downloads/MyPlugin-2.0.jar".to_string(),
            source_type: PluginSourceKind::Direct,
            extra: Default::default(),
        },
    );
    let transport = FakeTransport::new().with_get(
        "https://example.invalid/downloads/MyPlugin-2.0.jar",
        200,
        b"new bytes".to_vec(),
    );

    let source = sources.get("MyPlugin-1.0").unwrap().clone();
    let outcome = addons::update_plugin_from_source(
        &transport,
        &fs,
        plugins_dir,
        "MyPlugin-1.0",
        "MyPlugin",
        true,
        &source,
        None,
        &[],
        false,
        &mut sources,
    )
    .unwrap();

    assert_eq!(outcome.installed_path, plugins_dir.join("MyPlugin-2.0.jar"));
    assert_eq!(outcome.rekeyed_to.as_deref(), Some("MyPlugin-2.0"));
    // Stale prior copy removed (display-name-prefix sweep).
    assert!(fs.stat(&plugins_dir.join("MyPlugin-1.0.jar")).is_err());
    // Rekeyed in the sources map.
    assert!(!sources.contains_key("MyPlugin-1.0"));
    assert!(sources.contains_key("MyPlugin-2.0"));
}

#[test]
fn update_plugin_from_source_refused_on_pack_managed_server() {
    let fs = FakeFileSystem::new();
    let plugins_dir = Path::new("/server/plugins");
    let mut sources = HashMap::new();
    let source = PluginSourceConfig {
        url: "https://example.invalid/x.jar".to_string(),
        source_type: PluginSourceKind::Direct,
        extra: Default::default(),
    };
    let transport = FakeTransport::new();
    let result = addons::update_plugin_from_source(
        &transport,
        &fs,
        plugins_dir,
        "x",
        "x",
        true,
        &source,
        None,
        &[],
        true,
        &mut sources,
    );
    assert!(matches!(result, Err(AddonMutationError::PackManaged)));
}

#[test]
fn update_plugin_from_source_github_selects_jar_asset() {
    let fs = FakeFileSystem::new();
    let plugins_dir = Path::new("/server/plugins");
    let mut sources = HashMap::new();
    let source = PluginSourceConfig {
        url: "https://github.com/EssentialsX/Essentials".to_string(),
        source_type: PluginSourceKind::Github,
        extra: Default::default(),
    };
    let release = serde_json::json!({
        "assets": [
            {"name": "EssentialsX-2.20.1.jar", "browser_download_url": "https://cdn.example.invalid/EssentialsX-2.20.1.jar"},
            {"name": "checksums.txt", "browser_download_url": "https://cdn.example.invalid/checksums.txt"},
        ]
    });
    let transport = FakeTransport::new()
        .with_json(
            "https://api.github.com/repos/EssentialsX/Essentials/releases/latest",
            release,
        )
        .with_get(
            "https://cdn.example.invalid/EssentialsX-2.20.1.jar",
            200,
            b"jar bytes".to_vec(),
        );

    let outcome = addons::update_plugin_from_source(
        &transport,
        &fs,
        plugins_dir,
        "EssentialsX",
        "EssentialsX",
        true,
        &source,
        None,
        &[],
        false,
        &mut sources,
    )
    .unwrap();

    assert_eq!(
        outcome.installed_path,
        plugins_dir.join("EssentialsX-2.20.1.jar")
    );
    assert_eq!(fs.read(&outcome.installed_path).unwrap(), b"jar bytes");
}
