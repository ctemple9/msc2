//! P8.22's own tests: `msc_application::client_export`, mapped 1:1 against
//! `fixtures/client-addon-export/`'s 28 cases (P8.8's own characterization
//! of `AppViewModel+ClientExport.swift`).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use msc_application::client_export::{
    self, ClientExportItem, ClientExportZipError, ClientSideStatus,
};
use msc_domain::app_config_schema::{AddonLink, AddonLinkProvenance, ConfigServer};
use msc_domain::identity::JavaServerFlavor;
use msc_infrastructure::fs::StdFileSystem;

struct TempDir(PathBuf);
impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-client-export-test-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
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

fn server(tmp: &TempDir, flavor: JavaServerFlavor) -> ConfigServer {
    let mut cfg = ConfigServer::new(
        "srv-1",
        "My Server",
        tmp.path().to_string_lossy().into_owned(),
        "",
        2.0,
        4.0,
    );
    cfg.java_flavor = flavor;
    cfg.minecraft_version = Some("1.20.1".to_string());
    cfg
}

fn add_on_dir(cfg: &ConfigServer, flavor: JavaServerFlavor) -> PathBuf {
    let kind = flavor.add_on_kind().unwrap();
    let dir = PathBuf::from(&cfg.server_dir).join(kind.folder_name());
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes a real, minimal zip at `path` whose only entry is
/// `fabric.mod.json` with the given `name`/`environment` (either may be
/// omitted). A real jar on real disk -- `ModJarMetadataParser`'s own port
/// reads zip bytes directly, never through the `FileSystem` trait.
fn write_fabric_jar(path: &Path, name: Option<&str>, environment: Option<&str>) {
    let file = fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("fabric.mod.json", options).unwrap();
    let mut obj = serde_json::Map::new();
    obj.insert(
        "id".to_string(),
        serde_json::Value::String("examplemod".to_string()),
    );
    if let Some(n) = name {
        obj.insert("name".to_string(), serde_json::Value::String(n.to_string()));
    }
    if let Some(e) = environment {
        obj.insert(
            "environment".to_string(),
            serde_json::Value::String(e.to_string()),
        );
    }
    use std::io::Write;
    zip.write_all(serde_json::to_string(&obj).unwrap().as_bytes())
        .unwrap();
    zip.finish().unwrap();
}

/// A real, minimal zip with no recognizable mod manifest at all -- one
/// harmless entry so it's still a valid (non-empty) archive.
fn write_plain_jar(path: &Path) {
    let file = fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file(
        "META-INF/MANIFEST.MF",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    use std::io::Write;
    zip.write_all(b"Manifest-Version: 1.0\n").unwrap();
    zip.finish().unwrap();
}

fn addon_link(client_side: Option<&str>, title: Option<&str>, slug: Option<&str>) -> AddonLink {
    AddonLink {
        project_id: "proj-1".to_string(),
        title: title.map(str::to_string),
        slug: slug.map(str::to_string),
        icon_url: None,
        provenance: AddonLinkProvenance::Installed,
        installed_version_id: None,
        installed_file_name: None,
        installed_hash: None,
        client_side: client_side.map(str::to_string),
        server_side: None,
        extra: serde_json::Map::new(),
    }
}

fn find<'a>(items: &'a [ClientExportItem], stem: &str) -> &'a ClientExportItem {
    items
        .iter()
        .find(|i| i.jar_stem == stem)
        .unwrap_or_else(|| panic!("no export item named {stem}"))
}

// ---------------------------------------------------------------------
// fixtures/client-addon-export/status-*, jar-environment-*, modrinth-*
// ---------------------------------------------------------------------

#[test]
fn status_sourced_from_persisted_modrinth_link_first() {
    let tmp = TempDir::new("link-wins");
    let mut cfg = server(&tmp, JavaServerFlavor::Fabric);
    let dir = add_on_dir(&cfg, JavaServerFlavor::Fabric);
    write_fabric_jar(&dir.join("examplemod.jar"), None, Some("server"));

    let mut links = HashMap::new();
    links.insert(
        "proj-1".to_string(),
        AddonLink {
            installed_file_name: Some("examplemod.jar".to_string()),
            ..addon_link(Some("required"), None, None)
        },
    );
    cfg.addon_links = Some(links);

    let items = client_export::build_client_export_items(&StdFileSystem, &cfg);
    let item = find(&items, "examplemod");
    assert_eq!(item.client_status, ClientSideStatus::Required);
    assert_eq!(item.status_source, "Modrinth");
}

#[test]
fn status_falls_back_to_embedded_jar_environment_for_unlinked_mods() {
    let tmp = TempDir::new("env-fallback");
    let cfg = server(&tmp, JavaServerFlavor::Fabric);
    let dir = add_on_dir(&cfg, JavaServerFlavor::Fabric);
    write_fabric_jar(&dir.join("examplemod.jar"), None, Some("client"));

    let items = client_export::build_client_export_items(&StdFileSystem, &cfg);
    let item = find(&items, "examplemod");
    assert_eq!(item.client_status, ClientSideStatus::Required);
    assert_eq!(item.status_source, "mod manifest");
}

#[test]
fn status_unknown_with_source_assumed_when_neither_signal_available() {
    let tmp = TempDir::new("assumed");
    let cfg = server(&tmp, JavaServerFlavor::Fabric);
    let dir = add_on_dir(&cfg, JavaServerFlavor::Fabric);
    write_plain_jar(&dir.join("examplemod.jar"));

    let items = client_export::build_client_export_items(&StdFileSystem, &cfg);
    let item = find(&items, "examplemod");
    assert_eq!(item.client_status, ClientSideStatus::Unknown);
    assert_eq!(item.status_source, "assumed");
}

#[test]
fn jar_environment_client_value_maps_to_required_not_excluded() {
    assert_eq!(
        client_export::client_side_status_from_environment("client"),
        ClientSideStatus::Required
    );
}

#[test]
fn jar_environment_wildcard_maps_to_required() {
    assert_eq!(
        client_export::client_side_status_from_environment("*"),
        ClientSideStatus::Required
    );
}

#[test]
fn jar_environment_server_maps_to_server_only() {
    assert_eq!(
        client_export::client_side_status_from_environment("server"),
        ClientSideStatus::ServerOnly
    );
}

#[test]
fn modrinth_server_side_value_unsupported_maps_to_server_only() {
    assert_eq!(
        client_export::client_side_status_from_modrinth("unsupported"),
        ClientSideStatus::ServerOnly
    );
}

#[test]
fn modrinth_server_side_value_unrecognized_maps_to_unknown_default_case() {
    assert_eq!(
        client_export::client_side_status_from_modrinth("some-future-modrinth-value"),
        ClientSideStatus::Unknown
    );
}

#[test]
fn status_required_optional_unknown_selected_by_default() {
    assert!(ClientSideStatus::Required.is_selected_by_default());
    assert!(ClientSideStatus::Optional.is_selected_by_default());
    assert!(ClientSideStatus::Unknown.is_selected_by_default());
}

#[test]
fn status_server_only_not_selected_by_default() {
    assert!(!ClientSideStatus::ServerOnly.is_selected_by_default());
}

// ---------------------------------------------------------------------
// Geyser/Floodgate exclusion, plugin vs modded server filtering
// ---------------------------------------------------------------------

#[test]
fn geyser_floodgate_excluded_from_export_entirely() {
    let tmp = TempDir::new("geyser-excluded");
    let cfg = server(&tmp, JavaServerFlavor::Fabric);
    let dir = add_on_dir(&cfg, JavaServerFlavor::Fabric);
    write_plain_jar(&dir.join("Geyser-Fabric.jar"));
    write_plain_jar(&dir.join("floodgate-fabric.jar"));
    write_plain_jar(&dir.join("examplemod.jar"));

    let items = client_export::build_client_export_items(&StdFileSystem, &cfg);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].jar_stem, "examplemod");
}

#[test]
fn plugin_server_drops_server_only_and_unknown_items() {
    let tmp = TempDir::new("plugin-drops-unknown");
    let cfg = server(&tmp, JavaServerFlavor::Paper);
    let dir = add_on_dir(&cfg, JavaServerFlavor::Paper);
    // No link, no fabric.mod.json (plugins never carry one) -> Unknown.
    write_plain_jar(&dir.join("SomePlugin.jar"));

    let items = client_export::build_client_export_items(&StdFileSystem, &cfg);
    assert!(items.is_empty());
}

#[test]
fn modded_server_keeps_unknown_and_server_only_items() {
    let tmp = TempDir::new("mod-keeps-unknown");
    let cfg = server(&tmp, JavaServerFlavor::Fabric);
    let dir = add_on_dir(&cfg, JavaServerFlavor::Fabric);
    write_plain_jar(&dir.join("unknownmod.jar"));
    write_fabric_jar(&dir.join("serveronlymod.jar"), None, Some("server"));

    let items = client_export::build_client_export_items(&StdFileSystem, &cfg);
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|i| i.jar_stem == "unknownmod"));
    assert!(items.iter().any(|i| i.jar_stem == "serveronlymod"));
}

#[test]
fn disabled_jar_included_in_export_list_identically_to_active_jar() {
    let tmp = TempDir::new("disabled-included");
    let cfg = server(&tmp, JavaServerFlavor::Fabric);
    let dir = add_on_dir(&cfg, JavaServerFlavor::Fabric);
    write_fabric_jar(&dir.join("examplemod.jar.disabled"), None, Some("client"));

    let items = client_export::build_client_export_items(&StdFileSystem, &cfg);
    let item = find(&items, "examplemod");
    assert_eq!(item.file_name, "examplemod.jar.disabled");
    assert_eq!(item.client_status, ClientSideStatus::Required);
    assert!(item.is_selected);
}

// ---------------------------------------------------------------------
// Ordering, display name, Modrinth URL
// ---------------------------------------------------------------------

#[test]
fn deterministic_ordering_required_optional_unknown_server_only_then_alphabetical() {
    let tmp = TempDir::new("ordering");
    let cfg = server(&tmp, JavaServerFlavor::Fabric);
    let dir = add_on_dir(&cfg, JavaServerFlavor::Fabric);
    write_fabric_jar(&dir.join("zzz_serveronly.jar"), None, Some("server"));
    write_plain_jar(&dir.join("bbb_unknown.jar"));
    write_fabric_jar(&dir.join("ccc_required.jar"), None, Some("client"));
    write_fabric_jar(&dir.join("aaa_required.jar"), None, Some("client"));

    let items = client_export::build_client_export_items(&StdFileSystem, &cfg);
    let stems: Vec<&str> = items.iter().map(|i| i.jar_stem.as_str()).collect();
    assert_eq!(
        stems,
        vec![
            "aaa_required",
            "ccc_required",
            "bbb_unknown",
            "zzz_serveronly",
        ]
    );
}

#[test]
fn display_name_precedence_persisted_link_title_then_jar_metadata_then_filename_heuristic() {
    let tmp = TempDir::new("display-name");
    let mut cfg = server(&tmp, JavaServerFlavor::Fabric);
    let dir = add_on_dir(&cfg, JavaServerFlavor::Fabric);
    write_fabric_jar(&dir.join("linked.jar"), Some("Jar Name"), None);
    write_fabric_jar(&dir.join("unlinked.jar"), Some("Manifest Name"), None);
    write_plain_jar(&dir.join("bare_stem_mod.jar"));

    let mut links = HashMap::new();
    links.insert(
        "proj-1".to_string(),
        AddonLink {
            installed_file_name: Some("linked.jar".to_string()),
            ..addon_link(None, Some("Linked Title"), None)
        },
    );
    cfg.addon_links = Some(links);

    let items = client_export::build_client_export_items(&StdFileSystem, &cfg);
    assert_eq!(find(&items, "linked").display_name, "Linked Title");
    assert_eq!(find(&items, "unlinked").display_name, "Manifest Name");
    assert_ne!(find(&items, "bare_stem_mod").display_name, "");
}

#[test]
fn modrinth_url_built_from_slug_else_project_id_else_nil() {
    let with_slug = ClientExportItem {
        jar_stem: "a".into(),
        file_name: "a.jar".into(),
        display_name: "A".into(),
        icon_url: None,
        project_id: Some("proj-1".into()),
        slug: Some("cool-mod".into()),
        client_status: ClientSideStatus::Required,
        status_source: "Modrinth".into(),
        is_selected: true,
        jar_path: PathBuf::from("/tmp/a.jar"),
    };
    assert_eq!(
        with_slug.modrinth_url().as_deref(),
        Some("https://modrinth.com/project/cool-mod")
    );

    let mut without_slug = with_slug.clone();
    without_slug.slug = None;
    assert_eq!(
        without_slug.modrinth_url().as_deref(),
        Some("https://modrinth.com/project/proj-1")
    );

    let mut unlinked = with_slug.clone();
    unlinked.slug = None;
    unlinked.project_id = None;
    assert_eq!(unlinked.modrinth_url(), None);
}

// ---------------------------------------------------------------------
// Empty / no-add-on-kind guards
// ---------------------------------------------------------------------

#[test]
fn empty_add_on_folder_returns_empty_list_no_error() {
    let tmp = TempDir::new("empty-folder");
    let cfg = server(&tmp, JavaServerFlavor::Fabric);
    add_on_dir(&cfg, JavaServerFlavor::Fabric);

    let items = client_export::build_client_export_items(&StdFileSystem, &cfg);
    assert!(items.is_empty());
}

#[test]
fn no_add_on_kind_server_returns_empty_list() {
    let tmp = TempDir::new("vanilla-empty");
    let cfg = server(&tmp, JavaServerFlavor::Vanilla);

    let items = client_export::build_client_export_items(&StdFileSystem, &cfg);
    assert!(items.is_empty());
}

// ---------------------------------------------------------------------
// copyClientLinksToClipboard
// ---------------------------------------------------------------------

#[test]
fn copy_links_no_op_when_nothing_selected() {
    let mut item = sample_item("A", true);
    item.is_selected = false;
    assert_eq!(client_export::client_links_text(&[item]), None);
}

#[test]
fn copy_links_only_includes_selected_items() {
    let selected = sample_item("Selected", true);
    let mut not_selected = sample_item("NotSelected", true);
    not_selected.is_selected = false;

    let text = client_export::client_links_text(&[selected, not_selected]).unwrap();
    assert!(text.contains("Selected"));
    assert!(!text.contains("NotSelected"));
}

#[test]
fn copy_link_text_shows_no_link_placeholder_when_modrinth_url_nil() {
    let item = sample_item("NoLink", false);
    let text = client_export::client_links_text(&[item]).unwrap();
    assert!(text.contains("(no link)"));
}

fn sample_item(display_name: &str, with_link: bool) -> ClientExportItem {
    ClientExportItem {
        jar_stem: display_name.to_lowercase(),
        file_name: format!("{}.jar", display_name.to_lowercase()),
        display_name: display_name.to_string(),
        icon_url: None,
        project_id: with_link.then(|| "proj-1".to_string()),
        slug: None,
        client_status: ClientSideStatus::Required,
        status_source: "assumed".into(),
        is_selected: true,
        jar_path: PathBuf::from("/tmp/does-not-matter.jar"),
    }
}

// ---------------------------------------------------------------------
// export ZIP: filename derivation, deterministic archive, no-op guard
// ---------------------------------------------------------------------

#[test]
fn export_zip_filename_includes_server_name_and_mc_version_with_slash_colon_sanitized() {
    let tmp = TempDir::new("zip-filename");
    let mut cfg = server(&tmp, JavaServerFlavor::Fabric);
    cfg.display_name = "My/Server:1".to_string();
    cfg.minecraft_version = Some("1.20.1".to_string());
    assert_eq!(
        client_export::client_export_zip_name(&cfg),
        "My-Server1-client-1.20.1.zip"
    );
}

#[test]
fn export_zip_defaults_mc_version_segment_to_literal_mods_when_unset() {
    let tmp = TempDir::new("zip-mc-default");
    let mut cfg = server(&tmp, JavaServerFlavor::Fabric);
    cfg.minecraft_version = None;
    assert_eq!(
        client_export::client_export_zip_name(&cfg),
        "My Server-client-mods.zip"
    );
}

#[test]
fn export_zip_no_op_when_nothing_selected() {
    let tmp = TempDir::new("zip-nothing-selected");
    let mut item = sample_item("A", true);
    item.is_selected = false;
    let dest = tmp.path().join("out.zip");

    let err = client_export::write_client_export_zip(&[item], &dest).unwrap_err();
    assert!(matches!(err, ClientExportZipError::NothingSelected));
    assert!(!dest.exists());
}

#[test]
fn export_zip_writes_flat_deterministic_archive_of_selected_jars() {
    let tmp = TempDir::new("zip-real");
    let cfg = server(&tmp, JavaServerFlavor::Fabric);
    let dir = add_on_dir(&cfg, JavaServerFlavor::Fabric);
    write_plain_jar(&dir.join("bmod.jar"));
    write_fabric_jar(&dir.join("amod.jar"), None, Some("client"));

    let items = client_export::build_client_export_items(&StdFileSystem, &cfg);
    assert_eq!(items.len(), 2);

    let dest = tmp.path().join("export.zip");
    client_export::write_client_export_zip(&items, &dest).unwrap();

    let file = fs::File::open(&dest).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();
    names.sort();
    assert_eq!(names, vec!["amod.jar".to_string(), "bmod.jar".to_string()]);
}
