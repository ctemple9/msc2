//! Essential P24.9: generic pack rollback must restore config bytes after a
//! failed metadata save, and preserve a file edited after activation. Existing
//! JAR installer checks do not exercise this generic pack journal. Controlled
//! local bytes, no timing/network assumptions; expected <100 ms each, not run.
use msc_application::pack_activation::{self, Activation, FileChange};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::content_collection::{ContentAudience, ContentCollection, ContentEntry};
use sha2::{Digest, Sha512};
use std::collections::BTreeMap;
use std::path::PathBuf;
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha512::digest(bytes))
}
fn fixture() -> (Fixture, ConfigServer, Activation) {
    let root = std::env::temp_dir().join(format!("msc-pack-activation-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("config")).unwrap();
    std::fs::create_dir_all(root.join(".msc2-content/player/blobs")).unwrap();
    std::fs::write(root.join("config/example.toml"), b"before").unwrap();
    let relative = format!(".msc2-content/player/blobs/{}", hash(b"after"));
    std::fs::write(root.join(&relative), b"after").unwrap();
    let server = ConfigServer::new(
        "pack-safety",
        "Pack safety",
        root.to_str().unwrap(),
        "server.jar",
        1.0,
        2.0,
    );
    let entry = ContentEntry {
        path: "config/example.toml".into(),
        sha512: hash(b"after"),
        byte_size: 5,
        audience: ContentAudience::Both,
        optional: false,
        source: None,
        redistribution_evidence: None,
        classification_evidence: Vec::new(),
        unresolved_reasons: Vec::new(),
        extra: BTreeMap::from([("retainedPath".into(), serde_json::Value::String(relative))]),
    };
    let mut collection = ContentCollection {
        revision: 1,
        ..Default::default()
    };
    collection
        .current_files
        .insert(entry.path.clone(), entry.clone());
    let activation = Activation {
        id: uuid::Uuid::new_v4().to_string(),
        server_id: server.id.clone(),
        server_dir: server.server_dir.clone(),
        collection_revision: 0,
        changes: vec![FileChange {
            path: entry.path.clone(),
            before_sha512: Some(hash(b"before")),
            after: Some(entry),
        }],
        collection,
    };
    (Fixture(root), server, activation)
}
#[test]
fn failed_pack_metadata_commit_restores_original_config_bytes() {
    let (fixture, server, activation) = fixture();
    pack_activation::prepare(&server, &activation, &|| false).unwrap();
    assert!(
        pack_activation::activate(&server, &activation, &|| false, &mut |_| Err(
            "controlled save failure".into()
        ))
        .is_err()
    );
    assert_eq!(
        std::fs::read(fixture.0.join("config/example.toml")).unwrap(),
        b"before"
    );
    assert!(!fixture.0.join(".msc2-content/pack-active.json").exists());
}
#[test]
fn failed_pack_commit_preserves_subsequent_owner_edit_and_blocks_recovery() {
    let (fixture, server, activation) = fixture();
    pack_activation::prepare(&server, &activation, &|| false).unwrap();
    let target = fixture.0.join("config/example.toml");
    assert!(
        pack_activation::activate(&server, &activation, &|| false, &mut |_| {
            std::fs::write(&target, b"owner edit").unwrap();
            Err("controlled save failure".into())
        })
        .is_err()
    );
    assert_eq!(std::fs::read(&target).unwrap(), b"owner edit");
    assert!(pack_activation::recover(&server).is_err());
    assert!(fixture.0.join(".msc2-content/pack-active.json").exists());
}
