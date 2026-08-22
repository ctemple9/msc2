use msc_application::resource_packs::ResourcePackService;
use msc_infrastructure::fs::StdFileSystem;
use std::fs;
use uuid::Uuid;

fn server_dir() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("msc-resource-packs-{}", Uuid::new_v4()));
    fs::create_dir_all(&path).unwrap();
    fs::write(
        path.join("server.properties"),
        "motd=pack test\ncustom=value\n",
    )
    .unwrap();
    path
}

#[test]
fn publishing_a_pack_writes_its_hosted_url_sha1_and_requirement() {
    let dir = server_dir();
    let service = ResourcePackService::new(&dir, &StdFileSystem);

    let pack = service
        .publish_and_activate("My Pack #1.zip", b"abc", "packs.example.test", 8123, true)
        .unwrap();

    assert_eq!(pack.sha1, "a9993e364706816aba3e25717850c26c9cd0d89d");
    assert_eq!(fs::read(pack.path).unwrap(), b"abc");
    let properties = fs::read_to_string(dir.join("server.properties")).unwrap();
    assert!(
        properties.contains("resource-pack=http://packs.example.test:8123/My%20Pack%20%231.zip")
    );
    assert!(properties.contains("resource-pack-sha1=a9993e364706816aba3e25717850c26c9cd0d89d"));
    assert!(properties.contains("require-resource-pack=true"));
    assert!(properties.contains("custom=value"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn disable_and_remove_clear_an_active_pack_without_exposing_other_files() {
    let dir = server_dir();
    let service = ResourcePackService::new(&dir, &StdFileSystem);
    service
        .publish_and_activate("safe.zip", b"pack", "packs.example.test", 8123, true)
        .unwrap();

    assert!(service.approved_bytes("../server.properties").is_err());
    service.remove("safe.zip").unwrap();
    assert!(!dir.join("resource-packs/safe.zip").exists());
    let properties = fs::read_to_string(dir.join("server.properties")).unwrap();
    assert!(properties.contains("resource-pack=\n"));
    assert!(properties.contains("require-resource-pack=false"));
    let _ = fs::remove_dir_all(dir);
}
