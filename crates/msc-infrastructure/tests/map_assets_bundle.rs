//! Essential: prevent launcher secrets/code escaping through a resource-only import,
//! and reject forged paths/hashes before a candidate can render. Controlled bytes;
//! no network, clocks or helpers. Expected combined runtime below one second.
use msc_infrastructure::map_assets::{bundle, hash};
use serde_json::json;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("msc-bundle-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn write_zip(path: &std::path::Path, entries: &[(&str, &[u8])]) {
    let mut zip = ZipWriter::new(fs::File::create(path).unwrap());
    for (name, bytes) in entries {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}
#[test]
fn matching_instance_transfers_resources_and_selection_without_private_or_executable_files() {
    let scratch = Scratch::new();
    let instance = scratch.0.join("instance");
    let game = instance.join(".minecraft");
    fs::create_dir_all(game.join("mods")).unwrap();
    fs::write(instance.join("mmc-pack.json"), json!({"components":[{"uid":"net.minecraft","version":"1.21.1"},{"uid":"net.fabricmc.fabric-loader","version":"0.16.10"}]}).to_string()).unwrap();
    fs::write(game.join("accounts.json"), "private-account-sentinel").unwrap();
    fs::write(
        game.join("options.txt"),
        "resourcePacks:[\"vanilla\"]\nlastServer:private-server-sentinel\n",
    )
    .unwrap();
    write_zip(
        &game.join("mods/panel.jar"),
        &[
            ("fabric.mod.json", br#"{"id":"panel","version":"1.0.0"}"#),
            (
                "assets/panel/models/block/panel.json",
                br#"{"elements":[]}"#,
            ),
            ("panel/Main.class", b"executable-sentinel"),
        ],
    );
    let bundle_path = scratch.0.join("portable.zip");
    let manifest =
        bundle::export_instance(&instance, &scratch.0.join("scratch"), &bundle_path, &|| {
            false
        })
        .unwrap();
    assert!(manifest.selection_known);
    assert_eq!(manifest.minecraft_version, "1.21.1");
    assert_eq!(manifest.layers[0].sources[0].declared_mods[0].id, "panel");
    let imported = bundle::unpack(&bundle_path, &scratch.0.join("host"), &|| false).unwrap();
    assert_eq!(imported.layers[0].resources, manifest.layers[0].resources);
    let mut zip = ZipArchive::new(fs::File::open(&bundle_path).unwrap()).unwrap();
    for i in 0..zip.len() {
        let mut bytes = Vec::new();
        zip.by_index(i).unwrap().read_to_end(&mut bytes).unwrap();
        for secret in [
            "private-account-sentinel",
            "private-server-sentinel",
            "executable-sentinel",
        ] {
            assert!(!bytes.windows(secret.len()).any(|v| v == secret.as_bytes()));
        }
    }
}
#[test]
fn malicious_or_case_colliding_resources_and_forged_objects_are_refused() {
    let scratch = Scratch::new();
    let object = br#"{"elements":[]}"#;
    let sha = hash(object);
    let mut manifest = bundle::Manifest {
        schema_version: 1,
        minecraft_version: "1.21.1".into(),
        loader: "vanilla".into(),
        loader_version: None,
        selection_known: true,
        layers: vec![bundle::Layer {
            id: hash(b"layer"),
            kind: "pack".into(),
            label: "Pack".into(),
            sources: vec![],
            resources: std::collections::BTreeMap::from([(
                "assets/demo/models/block/panel.json".into(),
                sha.clone(),
            )]),
        }],
        selected_packs: vec![],
        mod_order: None,
        evidence: "local_hashed".into(),
        required_sources: vec![],
        curseforge_files: vec![],
    };
    manifest.layers[0]
        .resources
        .insert("assets/demo/models/block/Panel.json".into(), sha.clone());
    assert!(manifest.validate().is_err());
    manifest.layers[0]
        .resources
        .remove("assets/demo/models/block/Panel.json");
    manifest.layers[0]
        .resources
        .insert("../outside".into(), sha.clone());
    assert!(manifest.validate().is_err());
    manifest.layers[0].resources.remove("../outside");
    let path = scratch.0.join("forged.zip");
    let raw = serde_json::to_vec(&manifest).unwrap();
    write_zip(
        &path,
        &[
            ("manifest.json", &raw),
            (&format!("objects/{sha}"), b"wrong content"),
        ],
    );
    assert!(bundle::unpack(&path, &scratch.0.join("candidate"), &|| false).is_err());
    assert!(!scratch.0.join("outside").exists());
    assert!(!scratch.0.join("candidate/layers").exists());
}
