//! Essential P24.6 checks: wrong player asset bytes must not replace retained
//! content, and client/server overrides at the same path must remain distinct.
//! Existing import checks inspect the running server, not the retained copy.
//! Controlled tiny local files, no timing/network; expected <100 ms per case.
use msc_application::pack_retention::{self, ExpectedPackFile, RetainedFile, RetainedPack};
use msc_domain::content_collection::ContentAudience;
use sha2::{Digest, Sha512};
use std::collections::BTreeMap;
use std::path::PathBuf;
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn retained(path: &str, bytes: &[u8]) -> RetainedFile {
    RetainedFile {
        path: path.into(),
        sha512: format!("{:x}", Sha512::digest(bytes)),
        byte_size: bytes.len() as u64,
    }
}
fn fixture() -> (Fixture, RetainedPack) {
    let directory =
        std::env::temp_dir().join(format!("msc-pack-retention-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let pack = RetainedPack {
        schema_version: 1,
        origin: None,
        relative_root: format!(".msc2-content/packs/{id}"),
        id,
        format: "mrpack".into(),
        name: "Safety".into(),
        version: "1".into(),
        original: retained("original.zip", b"original"),
        override_files: BTreeMap::new(),
        resolved_files: BTreeMap::new(),
        unresolved: BTreeMap::new(),
        expected_files: BTreeMap::new(),
        server_excluded_file_ids: Vec::new(),
    };
    (Fixture(directory), pack)
}
#[test]
fn manual_player_recovery_rejects_wrong_bytes_and_preserves_accepted_content() {
    let (fixture, mut pack) = fixture();
    let path = "resourcepacks/example.zip";
    let expected = retained("", b"good");
    pack.expected_files.insert(
        path.into(),
        ExpectedPackFile {
            path: path.into(),
            byte_size: 4,
            sha1: None,
            sha512: Some(expected.sha512),
            mirrors: Vec::new(),
            source: None,
            audience: Some(ContentAudience::Players),
            optional: true,
        },
    );
    pack.unresolved
        .insert(path.into(), "Manual download".into());
    let uploaded = fixture.0.join("upload.bin");
    std::fs::write(&uploaded, b"evil").unwrap();
    assert!(pack_retention::accept_manual_file(&fixture.0, &mut pack, path, &uploaded).is_err());
    assert!(pack.unresolved.contains_key(path));
    assert!(pack.resolved_files.is_empty());
    std::fs::write(&uploaded, b"good").unwrap();
    pack_retention::accept_manual_file(&fixture.0, &mut pack, path, &uploaded).unwrap();
    let destination = pack_retention::root(&fixture.0, &pack)
        .unwrap()
        .join("files")
        .join(path);
    std::fs::write(&uploaded, b"evil").unwrap();
    assert!(pack_retention::accept_manual_file(&fixture.0, &mut pack, path, &uploaded).is_err());
    assert_eq!(std::fs::read(destination).unwrap(), b"good");
}
#[test]
fn side_specific_overrides_keep_different_bytes_for_the_same_game_path() {
    let (fixture, mut pack) = fixture();
    let path = "config/example.toml";
    std::fs::create_dir_all(fixture.0.join("config")).unwrap();
    std::fs::write(fixture.0.join(path), b"server").unwrap();
    for (layer, bytes) in [
        ("client-overrides", b"client"),
        ("server-overrides", b"server"),
    ] {
        let retained_path = format!("overrides/{layer}/{path}");
        let file = pack_retention::root(&fixture.0, &pack)
            .unwrap()
            .join(&retained_path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, bytes).unwrap();
        pack.override_files
            .insert(format!("{layer}/{path}"), retained(&retained_path, bytes));
    }
    let collection = pack_retention::collection(&fixture.0, &pack, None).unwrap();
    assert_eq!(
        collection.current_files[path].sha512,
        retained("", b"server").sha512
    );
    assert_eq!(
        collection.player_files[path].sha512,
        retained("", b"client").sha512
    );
    assert_eq!(std::fs::read(fixture.0.join(path)).unwrap(), b"server");
}
