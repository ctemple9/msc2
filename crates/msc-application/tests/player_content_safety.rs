//! Essential P24.7 checks for the independent player review workflow, which
//! existing server-install rollback coverage does not exercise. Tiny controlled
//! local files, no network or timing assumptions; expected <100 ms each.
use msc_application::content_management::{self as content, Change};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::identity::JavaServerFlavor;
use std::collections::BTreeMap;
use std::path::PathBuf;
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> (Fixture, ConfigServer) {
    let root = std::env::temp_dir().join(format!("msc-player-content-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("mods")).unwrap();
    std::fs::write(root.join("mods/example.jar"), b"original").unwrap();
    let mut server = ConfigServer::new(
        "player-safety",
        "Player safety",
        root.to_str().unwrap(),
        "server.jar",
        1.0,
        2.0,
    );
    server.java_flavor = JavaServerFlavor::Fabric;
    server.minecraft_version = Some("1.21.1".into());
    (Fixture(root), server)
}
fn copy_change(sha512: String) -> Change {
    Change {
        action: "copyServerToPlayers".into(),
        path: "mods/example.jar".into(),
        sha512: Some(sha512),
        audience: None,
        optional: None,
        enabled: None,
        redistribution_evidence: None,
        staged_upload_id: None,
    }
}
#[test]
fn changed_server_bytes_invalidate_player_review_without_touching_server() {
    let (fixture, server) = fixture();
    let snapshot = content::snapshot(&server).unwrap();
    let sha512 = snapshot.collection.current_files["mods/example.jar"]
        .sha512
        .clone();
    let review = content::review(
        &server,
        "owner",
        0,
        vec![copy_change(sha512)],
        &BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(
        std::fs::read(fixture.0.join("mods/example.jar")).unwrap(),
        b"original"
    );
    std::fs::write(fixture.0.join("mods/example.jar"), b"changed").unwrap();
    assert!(content::load_review(&server, &review.id, "owner").is_err());
    assert_eq!(
        std::fs::read(fixture.0.join("mods/example.jar")).unwrap(),
        b"changed"
    );
}
#[test]
fn retained_player_bytes_are_independent_and_review_is_owner_bound() {
    let (fixture, server) = fixture();
    let snapshot = content::snapshot(&server).unwrap();
    let sha512 = snapshot.collection.current_files["mods/example.jar"]
        .sha512
        .clone();
    let review = content::review(
        &server,
        "owner",
        0,
        vec![copy_change(sha512)],
        &BTreeMap::new(),
    )
    .unwrap();
    assert!(content::load_review(&server, &review.id, "another-owner").is_err());
    let retained =
        content::player_source(&server, &review.result.player_files["mods/example.jar"]).unwrap();
    std::fs::write(fixture.0.join("mods/example.jar"), b"changed").unwrap();
    assert_eq!(std::fs::read(retained).unwrap(), b"original");
}
