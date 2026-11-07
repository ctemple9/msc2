//! Essential P24.15: frozen bytes must survive source edits, manifest tampering
//! must be detected, and owner attestations must not bypass known restrictions.
//! Earlier retention/activation tests do not exercise publication registry or
//! eligibility. Tiny local files, no network/timing assumptions; <100 ms/case.
//! These tests are not authorized to run by a plan's Verify line.
use msc_application::{content_publications as publications, publication_review as review};
use msc_domain::{
    app_config_schema::ConfigServer,
    content_collection::{ContentAudience, ContentCollection, ContentEntry},
    identity::JavaServerFlavor,
};
use std::{collections::BTreeMap, path::PathBuf};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> (Fixture, ConfigServer, publications::PreparedPublicationFile) {
    let root =
        std::env::temp_dir().join(format!("msc-publication-safety-{}", uuid::Uuid::new_v4()));
    let relative = ".msc2-content/player/blobs/source";
    std::fs::create_dir_all(root.join(".msc2-content/player/blobs")).unwrap();
    std::fs::write(root.join(relative), b"setting = true\n").unwrap();
    let identity =
        msc_infrastructure::addon_provider::fingerprint::identify(&root.join(relative)).unwrap();
    let entry = ContentEntry {
        path: "config/example.toml".into(),
        sha512: identity.sha512.clone(),
        byte_size: identity.byte_size,
        audience: ContentAudience::Players,
        optional: false,
        source: None,
        redistribution_evidence: Some("Owner-authored configuration, permission to share".into()),
        classification_evidence: vec!["Owner selected player configuration".into()],
        unresolved_reasons: Vec::new(),
        extra: BTreeMap::from([(
            "retainedPath".into(),
            serde_json::Value::String(relative.into()),
        )]),
    };
    let mut collection = ContentCollection {
        revision: 5,
        ..Default::default()
    };
    collection
        .player_files
        .insert(entry.path.clone(), entry.clone());
    let mut server = ConfigServer::new(
        "publication-safety",
        "Publication safety",
        root.to_str().unwrap(),
        "server.jar",
        1.0,
        2.0,
    );
    server.java_flavor = JavaServerFlavor::Fabric;
    server.minecraft_version = Some("1.21.1".into());
    server.loader_version = Some("0.16.0".into());
    server.content_collection = Some(collection);
    let file = publications::PublishedFile {
        path: entry.path,
        sha512: identity.sha512,
        sha1: identity.sha1,
        byte_size: identity.byte_size,
        optional: false,
        enabled: true,
        source: None,
        classification_evidence: entry.classification_evidence,
        redistribution_evidence: entry.redistribution_evidence.unwrap(),
        world_slot: None,
    };
    (
        Fixture(root),
        server,
        publications::PreparedPublicationFile {
            audience: "players".into(),
            source_relative: relative.into(),
            file,
        },
    )
}
fn freeze(
    server: &mut ConfigServer,
    prepared: publications::PreparedPublicationFile,
) -> publications::Publication {
    let mut saved = None;
    let publication = publications::freeze(
        server,
        5,
        None,
        vec!["players".into()],
        Some(msc_application::content_environment::PlayerSetup {
            loader: "fabric".into(),
            loader_version: Some("0.16.0".into()),
        }),
        msc_application::content_guides::VERSION,
        &[prepared],
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
    publication
}
#[test]
fn frozen_publication_survives_source_edits_and_removal() {
    let (fixture, mut server, prepared) = fixture();
    let source = fixture.0.join(&prepared.source_relative);
    let publication = freeze(&mut server, prepared);
    std::fs::write(&source, b"setting = false\n").unwrap();
    std::fs::remove_file(source).unwrap();
    let loaded = publications::load(&server, &publication.id).unwrap();
    let frozen = publications::frozen_file(
        &server,
        &loaded,
        &loaded.player_files["config/example.toml"],
    )
    .unwrap();
    assert_eq!(std::fs::read(frozen).unwrap(), b"setting = true\n");
    assert!(publications::frozen_guide(&server, &loaded, "PLAYER.txt").is_ok());
}
#[test]
fn altered_manifest_is_rejected_even_if_it_remains_valid_json() {
    let (fixture, mut server, prepared) = fixture();
    let publication = freeze(&mut server, prepared);
    let manifest = fixture.0.join(format!(
        ".msc2-content/publications/{}/manifest.json",
        publication.id
    ));
    let mut changed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    changed["serverName"] = serde_json::json!("Changed after publication");
    std::fs::write(manifest, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(publications::load(&server, &publication.id).is_err());
}
#[test]
fn attestation_cannot_embed_known_restricted_content_or_skip_incomplete_consent() {
    let (_fixture, mut server, prepared) = fixture();
    server
        .content_collection
        .as_mut()
        .unwrap()
        .player_files
        .get_mut("config/example.toml")
        .unwrap()
        .extra
        .insert(
            "providerDistributionAllowed".into(),
            serde_json::Value::Bool(false),
        );
    let request = review::Request {
        pack_export: false,
        audiences: vec!["players".into()],
        collection_revision: 5,
        version_name: None,
        player_setup: None,
        approvals: vec![review::Approval {
            audience: "players".into(),
            path: "config/example.toml".into(),
            sha512: prepared.file.sha512,
            redistribution_evidence: "Owner asks to share".into(),
            reviewed_configuration: true,
        }],
        excluded: Vec::new(),
        server_paths: Vec::new(),
        datapacks: Vec::new(),
    };
    let reviewed = review::prepare(&server, "owner", request, &|| false).unwrap();
    assert!(reviewed.files.is_empty());
    assert!(reviewed.issues.iter().any(|issue| issue.required));
    assert!(
        publications::freeze(
            &server,
            5,
            None,
            vec!["players".into()],
            Some(msc_application::content_environment::PlayerSetup {
                loader: "fabric".into(),
                loader_version: Some("0.16.0".into())
            }),
            msc_application::content_guides::VERSION,
            &reviewed.files,
            reviewed.issues,
            reviewed.exclusions,
            reviewed.content_fingerprint,
            Vec::new(),
            Vec::new(),
            false,
            &|| false,
            &mut |_| panic!("Incomplete publication must not reach registry commit")
        )
        .is_err()
    );
}

// Essential P24.32: a filtered client mod must survive export while an owner
// removal must not be resurrected. Also protects disabled/added/updated bytes.
// Existing publication tests do not select from an original pack. Local tiny
// files, no network, timing or launcher assumptions; expected <100 ms.
#[test]
fn full_pack_export_applies_owner_edits_without_losing_filtered_client_content() {
    let (fixture, mut server, _) = fixture();
    let root = &fixture.0;
    std::fs::create_dir_all(root.join("mods")).unwrap();
    std::fs::create_dir_all(root.join("config")).unwrap();
    let mut collection = ContentCollection::default();
    for (name, installed) in [
        ("client", false),
        ("removed", true),
        ("disabled", true),
        ("updated", true),
    ] {
        let retained = format!(".msc2-content/packs/base/{name}");
        std::fs::create_dir_all(root.join(".msc2-content/packs/base")).unwrap();
        std::fs::write(root.join(&retained), name.as_bytes()).unwrap();
        let identity =
            msc_infrastructure::addon_provider::fingerprint::identify(&root.join(&retained))
                .unwrap();
        let path = format!("mods/{name}.jar");
        let entry = ContentEntry {
            path: path.clone(),
            sha512: identity.sha512,
            byte_size: identity.byte_size,
            audience: ContentAudience::Both,
            optional: false,
            source: None,
            redistribution_evidence: Some("Fixture permission".into()),
            classification_evidence: vec!["Pack manifest".into()],
            unresolved_reasons: vec![],
            extra: BTreeMap::from([
                ("retainedPath".into(), retained.into()),
                ("packServerInstalled".into(), installed.into()),
            ]),
        };
        collection
            .upstream_files
            .insert(path.clone(), entry.clone());
        collection
            .upstream_player_files
            .insert(path.clone(), entry.clone());
        collection.player_files.insert(path.clone(), entry.clone());
        collection.current_files.insert(path, entry);
    }
    std::fs::write(root.join("mods/disabled.jar.disabled"), b"disabled").unwrap();
    std::fs::write(root.join("mods/updated.jar"), b"new version").unwrap();
    std::fs::write(root.join("mods/updated.jar.disabled"), b"old disabled copy").unwrap();
    std::fs::write(root.join("mods/added.jar"), b"addition").unwrap();
    std::fs::write(root.join("config/added.toml"), b"enabled = true").unwrap();
    server.content_collection = Some(collection.clone());
    let mut snapshot = msc_application::content_management::snapshot(&server).unwrap();
    msc_application::pack_export_selection::project(&server, &mut snapshot, &|| false).unwrap();
    let players = &snapshot.collection.player_files;
    assert!(players.contains_key("mods/client.jar"));
    assert!(!players.contains_key("mods/removed.jar"));
    assert_eq!(players["mods/disabled.jar"].extra["enabled"], false);
    assert!(players.contains_key("mods/added.jar"));
    assert!(players.contains_key("mods/updated.jar.disabled"));
    assert_ne!(
        players["mods/updated.jar"].extra.get("enabled"),
        Some(&false.into())
    );
    assert!(players.contains_key("config/added.toml"));
    assert_eq!(
        players["mods/updated.jar"].extra["retainedPath"],
        "mods/updated.jar"
    );
    assert_ne!(
        players["mods/updated.jar"].sha512,
        collection.player_files["mods/updated.jar"].sha512
    );
    assert!(
        !snapshot
            .collection
            .current_files
            .contains_key("mods/client.jar")
    );
    assert!(
        !snapshot
            .collection
            .current_files
            .contains_key("mods/removed.jar")
    );
    assert!(snapshot.requirements.is_empty());
    assert_eq!(server.content_collection.as_ref().unwrap(), &collection);
}
