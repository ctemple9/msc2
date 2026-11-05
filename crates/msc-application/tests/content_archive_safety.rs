//! Essential P24.16: archive generation must select one audience, preserve
//! snapshot bytes, keep optional cosmetics inactive, escape HTML and clean up
//! cancelled output. Snapshot tests alone never inspect an actual ZIP.
//! Tiny local fixtures, no network/timing assumptions; expected <150 ms/case.
use msc_application::{content_archives as archives, content_publications as publications};
use msc_domain::{
    app_config_schema::ConfigServer, content_collection::ContentCollection,
    identity::JavaServerFlavor,
};
use std::{cell::Cell, io::Read, path::PathBuf};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> (Fixture, ConfigServer, publications::Publication) {
    let root = std::env::temp_dir().join(format!("msc-content-archive-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join(".msc2-content/player/blobs")).unwrap();
    std::fs::create_dir_all(root.join("world")).unwrap();
    std::fs::write(root.join("world/level.dat"), b"never export world").unwrap();
    let mut server = ConfigServer::new(
        "archive-safety",
        "Server <script>bad()</script>",
        root.to_str().unwrap(),
        "server.jar",
        1.0,
        2.0,
    );
    server.java_flavor = JavaServerFlavor::Fabric;
    server.minecraft_version = Some("1.21.1".into());
    server.loader_version = Some("0.16.0".into());
    server.content_collection = Some(ContentCollection {
        revision: 1,
        ..Default::default()
    });
    let mut files = Vec::new();
    for (audience, path, bytes, optional, enabled) in [
        (
            "players",
            "shaderpacks/example.zip",
            b"player bytes".as_slice(),
            true,
            false,
        ),
        (
            "server",
            "mods/server-only.jar",
            b"server bytes".as_slice(),
            false,
            true,
        ),
    ] {
        let relative = format!(".msc2-content/player/blobs/{audience}");
        std::fs::write(root.join(&relative), bytes).unwrap();
        let identity =
            msc_infrastructure::addon_provider::fingerprint::identify(&root.join(&relative))
                .unwrap();
        files.push(publications::PreparedPublicationFile {
            audience: audience.into(),
            source_relative: relative,
            file: publications::PublishedFile {
                path: path.into(),
                sha512: identity.sha512,
                sha1: identity.sha1,
                byte_size: identity.byte_size,
                optional,
                enabled,
                source: None,
                classification_evidence: vec!["Reviewed owner fixture".into()],
                redistribution_evidence: "Owner authored these fixture bytes".into(),
                world_slot: None,
            },
        });
    }
    let mut saved = None;
    let publication = publications::freeze(
        &server,
        1,
        None,
        vec!["players".into(), "server".into()],
        Some(msc_application::content_environment::PlayerSetup {
            loader: "fabric".into(),
            loader_version: Some("0.16.0".into()),
        }),
        msc_application::content_guides::VERSION,
        &files,
        Vec::new(),
        Vec::new(),
        "a".repeat(128),
        Vec::new(),
        Vec::new(),
        false,
        &|| false,
        &mut |collection| {
            saved = Some(collection);
            Ok(())
        },
    )
    .unwrap();
    server.content_collection = saved;
    (Fixture(root), server, publication)
}
#[test]
fn player_zip_uses_frozen_bytes_and_omits_server_and_world_data() {
    let (fixture, server, publication) = fixture();
    std::fs::write(
        fixture.0.join(".msc2-content/player/blobs/players"),
        b"changed live source",
    )
    .unwrap();
    let output = fixture.0.join("exports/players.zip");
    archives::build(
        &server,
        &publication.id,
        "players",
        false,
        &output,
        &|| false,
        &mut |_, _| {},
    )
    .unwrap();
    let mut zip = zip::ZipArchive::new(std::fs::File::open(output).unwrap()).unwrap();
    assert!(zip.by_name("mods/server-only.jar").is_err());
    assert!(zip.by_name("world/level.dat").is_err());
    assert!(zip.by_name("instructions/SERVER.txt").is_err());
    assert!(zip.by_name("shaderpacks/example.zip").is_err());
    let mut bytes = Vec::new();
    zip.by_name("optional-content/shaderpacks/example.zip")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes, b"player bytes");
    let mut html = String::new();
    zip.by_name("README.html")
        .unwrap()
        .read_to_string(&mut html)
        .unwrap();
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;"));
    let mut manifest = String::new();
    zip.by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    assert!(!manifest.contains("server-only.jar"));
}
#[test]
fn cancellation_leaves_no_admitted_archive_or_partial_file() {
    let (fixture, server, publication) = fixture();
    let cancel = Cell::new(false);
    let output = fixture.0.join("exports/cancelled.zip");
    assert!(
        archives::build(
            &server,
            &publication.id,
            "players",
            false,
            &output,
            &|| cancel.get(),
            &mut |written, _| {
                if written > 0 {
                    cancel.set(true);
                }
            }
        )
        .is_err()
    );
    assert!(!output.exists());
    assert!(
        !std::fs::read_dir(output.parent().unwrap())
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .path()
                .extension()
                .is_some_and(|extension| extension == "part"))
    );
}

// Essential P24.17: a nested launcher archive must carry the same frozen player
// bytes without server leakage, and pin the real loader. Reuses tiny local
// fixtures; no external launcher/network/timing dependency; expected <150ms.
#[test]
fn nested_mrpack_pins_setup_and_preserves_player_scope() {
    let (fixture, server, publication) = fixture();
    let output = fixture.0.join("exports/nested.zip");
    let result = archives::build_with_options(
        &server,
        &publication.id,
        "players",
        false,
        true,
        &output,
        &|| false,
        &mut |_, _| {},
    )
    .unwrap();
    let mut outer = zip::ZipArchive::new(std::fs::File::open(output).unwrap()).unwrap();
    let mut bytes = Vec::new();
    outer
        .by_name(result.mrpack_file_name.as_deref().unwrap())
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    let mut inner = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let index: serde_json::Value =
        serde_json::from_reader(inner.by_name("modrinth.index.json").unwrap()).unwrap();
    assert_eq!(index["formatVersion"], 1);
    assert_eq!(index["dependencies"]["minecraft"], "1.21.1");
    assert_eq!(index["dependencies"]["fabric-loader"], "0.16.0");
    assert_eq!(index["files"].as_array().unwrap().len(), 0);
    let mut payload = Vec::new();
    inner
        .by_name("client-overrides/optional-content/shaderpacks/example.zip")
        .unwrap()
        .read_to_end(&mut payload)
        .unwrap();
    assert_eq!(payload, b"player bytes");
    assert!(
        inner
            .file_names()
            .all(|name| !name.contains("server-only") && !name.contains("world/"))
    );
    let mut unsupported = publication.clone();
    unsupported.loader = "paper".into();
    assert!(!msc_application::content_mrpack::eligibility(&unsupported, "server").supported);
    unsupported.player_setup = None;
    unsupported.player_setup_reviewed = false;
    let player = msc_application::content_mrpack::eligibility(&unsupported, "players");
    assert!(player.supported);
    assert!(!player.dependencies.contains_key("fabric-loader"));
}
