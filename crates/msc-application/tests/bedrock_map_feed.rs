//! A new Bedrock world must receive the position feed without replacing other packs.

use msc_application::bedrock_map_feed::ensure_active_world_feed;
use std::fs;

#[test]
fn installs_feed_for_each_active_world_and_keeps_other_packs() {
    let dir = std::env::temp_dir().join(format!("msc2-map-feed-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(dir.join("worlds/First")).unwrap();
    fs::write(dir.join("server.properties"), b"level-name=First\n").unwrap();
    fs::write(
        dir.join("worlds/First/world_behavior_packs.json"),
        br#"[{"pack_id":"other","version":[1,0,0]}]"#,
    )
    .unwrap();

    ensure_active_world_feed(&dir).unwrap();
    ensure_active_world_feed(&dir).unwrap();
    let first: serde_json::Value = serde_json::from_slice(
        &fs::read(dir.join("worlds/First/world_behavior_packs.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(first.as_array().unwrap().len(), 2);
    assert_eq!(first[0]["pack_id"], "other");
    let feed_id = first[1]["pack_id"].as_str().unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(dir.join("behavior_packs/msc-map-player-feed/manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["header"]["uuid"], feed_id);
    assert!(
        dir.join("behavior_packs/msc-map-player-feed/scripts/main.js")
            .is_file()
    );

    fs::write(dir.join("server.properties"), b"level-name=Second\n").unwrap();
    ensure_active_world_feed(&dir).unwrap();
    let second: serde_json::Value = serde_json::from_slice(
        &fs::read(dir.join("worlds/Second/world_behavior_packs.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(second.as_array().unwrap().len(), 1);
    assert_eq!(second[0]["pack_id"], feed_id);
    fs::remove_dir_all(dir).unwrap();
}
