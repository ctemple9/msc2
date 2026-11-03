use flate2::{Compression, write::GzEncoder};
use msc_application::java_world_settings::apply_existing_world;
use msc_domain::world_profile::WorldProfile;
use msc_domain::{identity::ServerType, nbt::NbtValue};
use msc_infrastructure::fs::FakeFileSystem;
use msc_infrastructure::fs::FileSystem;
use std::io::Write;
use std::{collections::BTreeMap, path::Path};

#[test]
fn saved_loader_identity_survives_but_requested_file_escape_is_rejected() {
    let fs = FakeFileSystem::new();
    let dir = Path::new("/server");
    let path = dir.join("world/level.dat");
    let loader =
        "openloader//Users/example/Library/Application Support/MSC 2/datapacks/sawmill.zip";
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(&[10, 0, 0, 0]).unwrap();
    let fields = BTreeMap::from([(
        "DataPacks".into(),
        NbtValue::Compound(BTreeMap::from([
            (
                "Enabled".into(),
                NbtValue::List(vec![
                    NbtValue::String("vanilla".into()),
                    NbtValue::String(loader.into()),
                ]),
            ),
            (
                "Disabled".into(),
                NbtValue::List(vec![NbtValue::String("loader/Disabled Pack".into())]),
            ),
        ])),
    )]);
    let bytes =
        msc_infrastructure::java_nbt::update_level_dat(&gzip.finish().unwrap(), &fields).unwrap();
    fs.write(&path, &bytes).unwrap();
    let mut profile = WorldProfile::default();
    for _ in 0..2 {
        apply_existing_world(&fs, dir, "world", &profile).unwrap();
        let saved = fs.read(&path).unwrap();
        let metadata =
            msc_domain::nbt::imported_world_metadata_from_level_dat(&saved, ServerType::Java);
        assert!(metadata.data_packs.iter().any(|id| id == loader));
        assert!(
            msc_domain::nbt::java_runtime_metadata(&saved)
                .1
                .contains(&"loader/Disabled Pack".into())
        );
    }
    let before = fs.read(&path).unwrap();
    profile.generation.data_packs = vec!["file/../outside.zip".into()];
    let error = apply_existing_world(&fs, dir, "world", &profile).unwrap_err();
    assert!(error.to_string().contains("file/../outside.zip"));
    assert_eq!(fs.read(&path).unwrap(), before);
}
