//! Essential P24.4 regression checks: rejected bytes cannot enter staging and
//! failed activation cannot remove pre-existing or subsequently edited files.
//! Controlled local files only; expected runtime below 100 ms per case. Running
//! these tests still requires Cameron's explicit command authorization.
use msc_application::content_install::{self, InstallFile, InstallReview, StoredInstallReview};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::identity::JavaServerFlavor;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> (Fixture, ConfigServer, StoredInstallReview) {
    let root = std::env::temp_dir().join(format!("msc-content-install-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("mods")).unwrap();
    std::fs::write(root.join("mods/owner.txt"), b"keep this").unwrap();
    let mut server = ConfigServer::new(
        "safety",
        "Safety",
        root.to_str().unwrap(),
        "server.jar",
        1.0,
        2.0,
    );
    server.java_flavor = JavaServerFlavor::Fabric;
    server.minecraft_version = Some("1.21.1".into());
    let buffer = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(buffer);
    zip.start_file("fabric.mod.json", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"{}").unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    let record = StoredInstallReview {
        allow_incompatible: false,
        id: uuid::Uuid::new_v4().to_string(),
        owner: "owner".into(),
        server_id: server.id.clone(),
        server_dir: server.server_dir.clone(),
        minecraft_version: "1.21.1".into(),
        flavor: "fabric".into(),
        created_unix: 0,
        collection_revision: 0,
        replacements: BTreeMap::new(),
        baseline: content_install::content_baseline(&server).unwrap(),
        review: InstallReview {
            provider: "curseforge".into(),
            warnings: Vec::new(),
            files: vec![InstallFile {
                project_id: "1".into(),
                version_id: "2".into(),
                title: "Example".into(),
                file_name: "example.jar".into(),
                byte_size: bytes.len() as u64,
                sha1: msc_infrastructure::download_staging::sha1_hex(&bytes),
                download_url: None,
                distribution_allowed: None,
                required_projects: Vec::new(),
                incompatible_projects: Vec::new(),
            }],
        },
    };
    let path = content_install::staged_file_path(&record, &record.review.files[0]).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
    (Fixture(root), server, record)
}

#[test]
fn wrong_manual_bytes_cannot_replace_verified_staging() {
    let (root, _, record) = fixture();
    let wrong = root.0.join("wrong.jar");
    std::fs::write(&wrong, b"not the selected jar").unwrap();
    let destination = content_install::staged_file_path(&record, &record.review.files[0]).unwrap();
    let before = std::fs::read(&destination).unwrap();
    assert!(content_install::accept_manual(&record, "2", &wrong, "example.jar").is_err());
    assert_eq!(std::fs::read(destination).unwrap(), before);
}

#[test]
fn failed_metadata_commit_restores_preexisting_content() {
    let (root, server, record) = fixture();
    let result = content_install::activate(&server, &record, &|| false, &mut |_| {
        Err("controlled metadata save failure".into())
    });
    assert!(result.is_err());
    assert!(!root.0.join("mods/example.jar").exists());
    assert_eq!(
        std::fs::read(root.0.join("mods/owner.txt")).unwrap(),
        b"keep this"
    );
    assert_eq!(
        content_install::content_baseline(&server).unwrap(),
        record.baseline
    );
}

#[test]
fn recovery_preserves_owner_edits_after_interruption() {
    let (root, server, record) = fixture();
    std::fs::write(
        root.0.join("mods/example.jar"),
        b"owner changed this after the interrupted install",
    )
    .unwrap();
    let marker = root.0.join(".msc2-content/install-active.json");
    std::fs::write(&marker, serde_json::to_vec(&record).unwrap()).unwrap();
    assert!(content_install::recover_activation(&server).is_err());
    assert_eq!(
        std::fs::read(root.0.join("mods/example.jar")).unwrap(),
        b"owner changed this after the interrupted install"
    );
    assert!(marker.exists());
}

// Essential update regressions: the original must survive a failed metadata
// commit whether the provider keeps or changes its filename. Same controlled
// fixture and runtime bound as the install checks above; not auto-authorized.
#[test]
fn failed_same_name_update_restores_original_bytes() {
    let (root, server, mut record) = fixture();
    let original = root.0.join("mods/example.jar");
    std::fs::write(&original, b"original installed bytes").unwrap();
    record.baseline = content_install::content_baseline(&server).unwrap();
    record
        .replacements
        .insert("example.jar".into(), "example.jar".into());
    let result = content_install::activate(&server, &record, &|| false, &mut |_| {
        Err("controlled metadata save failure".into())
    });
    assert!(result.is_err());
    assert_eq!(
        std::fs::read(original).unwrap(),
        b"original installed bytes"
    );
    assert_eq!(
        content_install::content_baseline(&server).unwrap(),
        record.baseline
    );
}

#[test]
fn failed_renamed_update_restores_original_filename() {
    let (root, server, mut record) = fixture();
    std::fs::write(root.0.join("mods/previous.jar"), b"previous release").unwrap();
    record.baseline = content_install::content_baseline(&server).unwrap();
    record
        .replacements
        .insert("previous.jar".into(), "example.jar".into());
    let result = content_install::activate(&server, &record, &|| false, &mut |_| {
        Err("controlled metadata save failure".into())
    });
    assert!(result.is_err());
    assert!(!root.0.join("mods/example.jar").exists());
    assert_eq!(
        std::fs::read(root.0.join("mods/previous.jar")).unwrap(),
        b"previous release"
    );
    assert_eq!(
        content_install::content_baseline(&server).unwrap(),
        record.baseline
    );
}
