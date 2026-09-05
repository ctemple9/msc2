//! Exercises the three `fixtures/jar-templates/` cases P7.15's own
//! module doc left for P7.21 (`jar-summary-geyser-floodgate-*`,
//! `export-server-as-template-*`, `create-server-from-template-*`), plus
//! `list_server_templates`'s display-title composition, which no
//! fixture names directly (ported from `TemplateItemDisplay.swift` for
//! contract completeness — see `templates.rs`'s own doc).

use msc_application::provisioning::{self, CreateServerError, NewServerRequest, WorldSource};
use msc_application::templates::{self, CreateFromTemplateRequest};
use msc_domain::identity::JavaServerFlavor;
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, SystemTime};

struct Fixture {
    expected: Value,
}

fn load(case: &str) -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/jar-templates")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    let json: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()));
    Fixture {
        expected: json["expected"].clone(),
    }
}

const HOME_DIR: &str = "/home/msc";

// --- jarSummary(for:) ---

#[test]
fn jar_summary_geyser_floodgate_pick_newest_by_modification_date() {
    let fixture = load("jar-summary-geyser-floodgate-pick-newest-by-modification-date");
    let plugins_dir = "/servers/java/box/plugins";

    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_225_600); // 2026-01-01
    let new = SystemTime::UNIX_EPOCH + Duration::from_secs(1_772_582_400); // 2026-03-01

    let fs = FakeFileSystem::new()
        .with_file(
            format!("{plugins_dir}/Geyser-Spigot-2.4.0.jar"),
            b"a".to_vec(),
            false,
        )
        .with_modified(format!("{plugins_dir}/Geyser-Spigot-2.4.0.jar"), old)
        .with_file(
            format!("{plugins_dir}/geyser-spigot-2.5.0.jar"),
            b"b".to_vec(),
            false,
        )
        .with_modified(format!("{plugins_dir}/geyser-spigot-2.5.0.jar"), new)
        .with_file(
            format!("{plugins_dir}/Floodgate-Spigot.jar"),
            b"c".to_vec(),
            false,
        );
    // Floodgate-Spigot.jar is left at the FakeFileSystem default
    // (`SystemTime::UNIX_EPOCH`) — this module's own sentinel for "no
    // readable modification date", matching `fs.rs`'s own
    // `unwrap_or(SystemTime::UNIX_EPOCH)` convention.

    let summary = templates::jar_summary(&fs, Path::new(plugins_dir), Path::new(HOME_DIR), "")
        .expect("jar_summary");

    assert_eq!(summary.paper_filename, "paper.jar");
    let geyser = summary.geyser.expect("geyser candidate found");
    assert_eq!(geyser.filename, "geyser-spigot-2.5.0.jar");
    assert_eq!(geyser.modified, Some(new));
    let floodgate = summary.floodgate.expect("floodgate candidate found");
    assert_eq!(floodgate.filename, "Floodgate-Spigot.jar");
    assert_eq!(floodgate.modified, None);

    assert_eq!(
        fixture.expected["geyserFilename"]
            .as_str()
            .unwrap()
            .split(" \u{2014} ")
            .next()
            .unwrap(),
        geyser.filename
    );
    assert_eq!(
        fixture.expected["floodgateFilename"].as_str().unwrap(),
        floodgate.filename
    );
}

#[test]
fn jar_summary_undated_candidate_only_used_as_last_resort() {
    // A second undated candidate must NOT replace the first undated one
    // (source's `else if xURLForSummary == nil` only fires while nothing
    // has been picked at all) — not directly named by the fixture above
    // (which only has one undated Floodgate candidate), so proven here
    // against the pure selection rule this module's own doc explains.
    let plugins_dir = "/servers/java/box/plugins";
    let fs = FakeFileSystem::new()
        .with_file(format!("{plugins_dir}/geyser-a.jar"), b"a".to_vec(), false)
        .with_file(format!("{plugins_dir}/geyser-b.jar"), b"b".to_vec(), false);

    let summary = templates::jar_summary(&fs, Path::new(plugins_dir), Path::new(HOME_DIR), "")
        .expect("jar_summary");
    assert_eq!(summary.geyser.unwrap().filename, "geyser-a.jar");
}

// --- templateMutationProvider: "exportServer" ---

#[test]
fn export_server_as_template_jar_and_plugins_no_running_server_check() {
    let _fixture = load("export-server-as-template-jar-and-plugins-no-running-server-check");
    let server_dir = "/servers/java/box";
    let paper_template_dir = "/servers/_paper_templates";
    let plugin_template_dir = "/servers/_plugin_templates";

    let fs = FakeFileSystem::new()
        .with_file(
            format!("{server_dir}/paper.jar"),
            b"jar-bytes".to_vec(),
            false,
        )
        .with_file(
            format!("{server_dir}/.msc_paper_version.json"),
            br#"{"mcVersion":"1.21.4","build":231,"timestamp":"2026-01-01T00:00:00Z"}"#.to_vec(),
            false,
        )
        .with_file(
            format!("{server_dir}/plugins/Geyser-Spigot.jar"),
            b"g".to_vec(),
            false,
        )
        .with_file(
            format!("{server_dir}/plugins/someplugin.jar"),
            b"s".to_vec(),
            false,
        );

    // `serverIsRunning: true` in the fixture's input is exactly the
    // point being characterized: this call takes no running-server
    // parameter at all, matching source's confirmed absence of that
    // guard for this action (see this module's own doc).
    let result = templates::export_server_as_template(
        &fs,
        Path::new(HOME_DIR),
        Path::new(paper_template_dir),
        Path::new(plugin_template_dir),
        Path::new(server_dir),
        "",
        true,
        true,
    );

    assert_eq!(result.exported_count, 3);
    assert!(
        fs.read(
            Path::new(paper_template_dir)
                .join("paper-1.21.4-build231.jar")
                .as_path()
        )
        .is_ok()
    );
    assert!(
        fs.read(
            Path::new(plugin_template_dir)
                .join("Geyser-Spigot.jar")
                .as_path()
        )
        .is_ok()
    );
    assert!(
        fs.read(
            Path::new(plugin_template_dir)
                .join("someplugin.jar")
                .as_path()
        )
        .is_ok()
    );
}

#[test]
fn export_server_as_template_falls_back_to_source_filename_without_sidecar() {
    let server_dir = "/servers/java/box";
    let paper_template_dir = "/servers/_paper_templates";
    let plugin_template_dir = "/servers/_plugin_templates";
    let fs = FakeFileSystem::new().with_file(
        format!("{server_dir}/custom.jar"),
        b"jar-bytes".to_vec(),
        false,
    );

    let result = templates::export_server_as_template(
        &fs,
        Path::new(HOME_DIR),
        Path::new(paper_template_dir),
        Path::new(plugin_template_dir),
        Path::new(server_dir),
        "custom.jar",
        true,
        false,
    );

    assert_eq!(result.exported_count, 1);
    assert!(
        fs.read(Path::new(paper_template_dir).join("custom.jar").as_path())
            .is_ok()
    );
}

// --- templateMutationProvider: "createServer" ---

#[test]
fn create_server_from_template_resolves_flavor_from_filename_prefix() {
    let _fixture = load("create-server-from-template-resolves-flavor-from-filename-prefix");
    assert_eq!(
        templates::template_flavor_for_filename("purpur-1.21.4-build2270.jar"),
        JavaServerFlavor::Purpur
    );
    assert_eq!(
        templates::template_flavor_for_filename("pufferfish-1.21.4.jar"),
        JavaServerFlavor::Pufferfish
    );
    assert_eq!(
        templates::template_flavor_for_filename("minecraft_server-1.21.4.jar"),
        JavaServerFlavor::Vanilla
    );
    assert_eq!(
        templates::template_flavor_for_filename("fabric-server-launch-1.21.4.jar"),
        JavaServerFlavor::Fabric
    );
    assert_eq!(
        templates::template_flavor_for_filename("paper-1.21.4-build231.jar"),
        JavaServerFlavor::Paper
    );
    // Everything unrecognized falls to Paper too (source's own
    // fallthrough, not a special-cased match on "paper-").
    assert_eq!(
        templates::template_flavor_for_filename("some-custom-name.jar"),
        JavaServerFlavor::Paper
    );

    let servers_root = "/servers";
    let plugin_template_dir = "/servers/_plugin_templates";
    let template_path = "/servers/_paper_templates/purpur-1.21.4-build2270.jar";
    let fs = FakeFileSystem::new().with_file(template_path, b"template-bytes".to_vec(), false);

    let request = CreateFromTemplateRequest {
        name: "From Template",
        initial_world_name: None,
        port: 25565,
        enable_cross_play: false,
        cross_play_bedrock_port: None,
        enable_playit: false,
        difficulty: "normal",
        gamemode: "survival",
        world_seed: None,
        default_banner_color_hex: "#5865F2",
    };

    let created = templates::create_server_from_template(
        &fs,
        Path::new(HOME_DIR),
        Path::new(servers_root),
        Path::new(plugin_template_dir),
        Path::new(template_path),
        "purpur-1.21.4-build2270.jar",
        &request,
        "2026-08-19T00:00:00Z",
    )
    .expect("create_server_from_template");

    assert_eq!(created.config.java_flavor, JavaServerFlavor::Purpur);
    assert_eq!(
        created.config.paper_jar_path,
        "/servers/java/from_template/paper.jar"
    );
    // `ComponentVersionParsing.parsePaperJarFilename` only recognizes a
    // `paper-*` filename — a Purpur template leaves the resolved
    // version/build unset in source too (this module's own doc).
    assert_eq!(created.config.minecraft_version, None);
    assert_eq!(created.config.server_build, None);

    let copied = fs
        .read(Path::new("/servers/java/from_template/paper.jar"))
        .expect("jar copied into new server dir");
    assert_eq!(copied, b"template-bytes");
}

#[test]
fn create_server_from_template_resolves_version_from_paper_filename() {
    let servers_root = "/servers";
    let plugin_template_dir = "/servers/_plugin_templates";
    let template_path = "/servers/_paper_templates/paper-1.21.4-build231.jar";
    let fs = FakeFileSystem::new().with_file(template_path, b"paper-bytes".to_vec(), false);

    let request = CreateFromTemplateRequest {
        name: "Paper From Template",
        initial_world_name: None,
        port: 25565,
        enable_cross_play: false,
        cross_play_bedrock_port: None,
        enable_playit: false,
        difficulty: "normal",
        gamemode: "survival",
        world_seed: None,
        default_banner_color_hex: "#5865F2",
    };

    let created = templates::create_server_from_template(
        &fs,
        Path::new(HOME_DIR),
        Path::new(servers_root),
        Path::new(plugin_template_dir),
        Path::new(template_path),
        "paper-1.21.4-build231.jar",
        &request,
        "2026-08-19T00:00:00Z",
    )
    .expect("create_server_from_template");

    assert_eq!(created.config.java_flavor, JavaServerFlavor::Paper);
    assert_eq!(created.config.minecraft_version.as_deref(), Some("1.21.4"));
    assert_eq!(created.config.server_build.as_deref(), Some("231"));
    let sidecar = fs
        .read(Path::new(
            "/servers/java/paper_from_template/.msc_paper_version.json",
        ))
        .expect("sidecar written");
    let value: Value = serde_json::from_slice(&sidecar).unwrap();
    assert_eq!(value["mcVersion"], "1.21.4");
    assert_eq!(value["build"], 231);
}

#[test]
fn create_server_from_template_rolls_back_on_folder_already_exists() {
    let servers_root = "/servers";
    let plugin_template_dir = "/servers/_plugin_templates";
    let template_path = "/servers/_paper_templates/paper-1.21.4-build231.jar";
    let fs = FakeFileSystem::new()
        .with_file(template_path, b"paper-bytes".to_vec(), false)
        .with_dir("/servers/java/dup");

    let request = CreateFromTemplateRequest {
        name: "Dup",
        initial_world_name: None,
        port: 25565,
        enable_cross_play: false,
        cross_play_bedrock_port: None,
        enable_playit: false,
        difficulty: "normal",
        gamemode: "survival",
        world_seed: None,
        default_banner_color_hex: "#5865F2",
    };

    let err = templates::create_server_from_template(
        &fs,
        Path::new(HOME_DIR),
        Path::new(servers_root),
        Path::new(plugin_template_dir),
        Path::new(template_path),
        "paper-1.21.4-build231.jar",
        &request,
        "2026-08-19T00:00:00Z",
    )
    .expect_err("folder already exists");
    assert!(matches!(err, CreateServerError::FolderAlreadyExists { .. }));
}

// --- list_server_templates: displayName composition, no dedicated fixture ---

#[test]
fn list_server_templates_derives_display_titles_and_ids() {
    let paper_dir = "/servers/_paper_templates";
    let plugin_dir = "/servers/_plugin_templates";
    let fs = FakeFileSystem::new()
        .with_file(
            format!("{paper_dir}/paper-1.20.4-build120.jar"),
            b"a".to_vec(),
            false,
        )
        .with_file(
            format!("{paper_dir}/paper-1.21.1-120.jar"),
            b"b".to_vec(),
            false,
        )
        .with_file(format!("{paper_dir}/custom-name.jar"), b"c".to_vec(), false)
        .with_file(
            format!("{plugin_dir}/Geyser-Spigot-2.4.2.jar"),
            b"d".to_vec(),
            false,
        )
        .with_file(
            format!("{plugin_dir}/floodgate-2.2.0.jar"),
            b"e".to_vec(),
            false,
        )
        .with_file(format!("{plugin_dir}/Geyser.jar"), b"f".to_vec(), false);

    let result = templates::list_server_templates(
        &fs,
        Path::new(paper_dir),
        Path::new(plugin_dir),
        Path::new(HOME_DIR),
    )
    .expect("list_server_templates");

    let by_name = |items: &[templates::TemplateListItem], name: &str| {
        items
            .iter()
            .find(|i| i.filename == name)
            .unwrap_or_else(|| panic!("{name} not listed"))
            .clone()
    };

    let build_style = by_name(&result.paper, "paper-1.20.4-build120.jar");
    assert_eq!(build_style.display_name, "Paper 1.20.4 (build 120)");
    assert_eq!(build_style.id, "paper:paper-1.20.4-build120.jar");
    assert_eq!(build_style.kind, "paper");

    let dash_style = by_name(&result.paper, "paper-1.21.1-120.jar");
    assert_eq!(dash_style.display_name, "Paper 1.21.1 (build 120)");

    let custom = by_name(&result.paper, "custom-name.jar");
    assert_eq!(custom.display_name, "custom-name");

    let geyser = by_name(&result.plugin, "Geyser-Spigot-2.4.2.jar");
    assert_eq!(geyser.display_name, "Geyser-Spigot (2.4.2)");
    assert_eq!(geyser.id, "plugin:Geyser-Spigot-2.4.2.jar");
    assert_eq!(geyser.kind, "plugin");

    let floodgate = by_name(&result.plugin, "floodgate-2.2.0.jar");
    assert_eq!(floodgate.display_name, "floodgate (2.2.0)");

    let bare_geyser = by_name(&result.plugin, "Geyser.jar");
    assert_eq!(bare_geyser.display_name, "Geyser");
}

#[test]
fn create_server_from_template_empty_name_refused() {
    let fs = FakeFileSystem::new().with_file(
        "/servers/_paper_templates/paper-1.21.4-build231.jar",
        b"x".to_vec(),
        false,
    );
    let request = CreateFromTemplateRequest {
        name: "   ",
        initial_world_name: None,
        port: 25565,
        enable_cross_play: false,
        cross_play_bedrock_port: None,
        enable_playit: false,
        difficulty: "normal",
        gamemode: "survival",
        world_seed: None,
        default_banner_color_hex: "#5865F2",
    };
    let err = templates::create_server_from_template(
        &fs,
        Path::new(HOME_DIR),
        Path::new("/servers"),
        Path::new("/servers/_plugin_templates"),
        Path::new("/servers/_paper_templates/paper-1.21.4-build231.jar"),
        "paper-1.21.4-build231.jar",
        &request,
        "2026-08-19T00:00:00Z",
    )
    .expect_err("empty name refused");
    assert!(matches!(err, CreateServerError::EmptyName));
}

// Sanity check that `finish_server_creation`'s new `Option<&str>` signature
// (widened for this step) still behaves identically for P7.17/P7.18's own
// always-`Some` callers — regression coverage for the visibility/signature
// edit this step made to `provisioning.rs`, not a P7.21 fixture case.
#[test]
fn create_download_and_go_server_still_resolves_version_after_option_widening() {
    use msc_infrastructure::jar_provider::JarProviderError;
    use msc_infrastructure::jar_provider::Transport;

    struct NeverTransport;
    impl Transport for NeverTransport {
        fn get(
            &self,
            _url: &str,
            _what: &str,
            _max_bytes: u64,
        ) -> Result<Vec<u8>, JarProviderError> {
            Err(JarProviderError::Network(
                "not reachable in this test".into(),
            ))
        }
    }

    let fs = FakeFileSystem::new();
    let request = NewServerRequest {
        name: "Regression",
        initial_world_name: None,
        specific_version_id: None,
        specific_loader_version: None,
        flavor: JavaServerFlavor::Vanilla,
        port: 25565,
        enable_cross_play: false,
        cross_play_bedrock_port: None,
        enable_playit: false,
        enable_xbox_broadcast: false,
        difficulty: "normal",
        gamemode: "survival",
        world_seed: None,
        initial_world_profile: None,
        world_source: WorldSource::Fresh,
        save_downloaded_jars: false,
        default_banner_color_hex: "#5865F2",
    };
    let err = provisioning::create_download_and_go_server(
        &fs,
        &NeverTransport,
        Path::new(HOME_DIR),
        Path::new("/servers"),
        Path::new("/servers/_paper_templates"),
        Path::new("/servers/_plugin_templates"),
        &request,
        "2026-08-19T00:00:00Z",
        |_, _| false,
        |_, _, _| false,
    )
    .expect_err("network unreachable");
    // Proves `finish_server_creation`'s `Option<&str>` signature widening
    // (this step's own edit to `provisioning.rs`) didn't disturb
    // P7.17/P7.18's existing download-and-go control flow: the error still
    // surfaces as `Download`, not a type/argument-order regression.
    assert!(matches!(err, CreateServerError::Download(_)));
}
