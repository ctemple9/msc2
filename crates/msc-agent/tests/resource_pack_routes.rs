//! P9.8's route-facing invariant.  P9.13 mounts the frozen HTTP routes; this
//! test proves the underlying serving hand-off only accepts an approved name.

use msc_infrastructure::resource_pack_store::ResourcePackStore;
use std::fs;
use uuid::Uuid;

#[test]
fn public_pack_lookup_cannot_read_an_arbitrary_server_path() {
    let dir = std::env::temp_dir().join(format!("msc-resource-pack-route-{}", Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("server.properties"), "secret=value\n").unwrap();
    let store = ResourcePackStore::new(&dir);
    store.publish("approved.zip", b"pack bytes").unwrap();

    assert_eq!(
        store.read_approved_bytes("approved.zip").unwrap(),
        b"pack bytes"
    );
    assert!(store.read_approved_bytes("../server.properties").is_err());
    assert!(store.read_approved_bytes("server.properties").is_err());
    let _ = fs::remove_dir_all(dir);
}
