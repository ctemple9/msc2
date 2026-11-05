//! Essential P24.8: original-pack recovery must never overwrite an existing
//! server config or world. Existing imports intentionally write server files,
//! so their checks cannot protect this non-destructive recovery path.
//! Tiny controlled archive/files, network forbidden; expected <100 ms.
use msc_application::{content_recovery, modpacks};
use msc_domain::{app_config_schema::ConfigServer, identity::JavaServerFlavor};
use msc_infrastructure::addon_provider::{AddonTransport, RawResponse, TransportError};
use std::io::Write;
use std::path::PathBuf;
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct NoNetwork;
impl AddonTransport for NoNetwork {
    fn get(
        &self,
        _: &str,
        _: &str,
        _: &[(&str, &str)],
        _: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!("Recovery fixture must not fetch provider metadata");
    }
    fn post_json(
        &self,
        _: &str,
        _: &str,
        _: &serde_json::Value,
        _: &[(&str, &str)],
        _: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!("Recovery fixture must not fetch provider metadata");
    }
}
#[test]
fn original_pack_recovery_retains_client_overrides_without_changing_server_or_world() {
    let fixture = Fixture(
        std::env::temp_dir().join(format!("msc-content-recovery-{}", uuid::Uuid::new_v4())),
    );
    let server_dir = fixture.0.join("server");
    let staged = fixture.0.join("staged");
    std::fs::create_dir_all(server_dir.join("config")).unwrap();
    std::fs::create_dir_all(server_dir.join("world")).unwrap();
    std::fs::create_dir_all(staged.join("client-overrides/config")).unwrap();
    std::fs::write(
        server_dir.join("config/example.toml"),
        b"server customization",
    )
    .unwrap();
    std::fs::write(server_dir.join("world/level.dat"), b"world stays intact").unwrap();
    std::fs::write(
        staged.join("client-overrides/config/example.toml"),
        b"client settings",
    )
    .unwrap();
    let manifest=serde_json::json!({"name":"Recovery fixture","versionId":"1","game":"minecraft","dependencies":{"minecraft":"1.21.1","fabric-loader":"0.16.0"},"files":[]}).to_string();
    let archive = fixture.0.join("original.mrpack");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
    zip.start_file(
        "modrinth.index.json",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(manifest.as_bytes()).unwrap();
    zip.start_file(
        "client-overrides/config/example.toml",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(b"client settings").unwrap();
    zip.finish().unwrap();
    let original_bytes = std::fs::read(&archive).unwrap();
    let inspection = modpacks::ModpackInspection {
        origin: None,
        archive_path: Some(archive),
        format: modpacks::InspectedFormat::Mrpack(
            msc_domain::modpack_manifest::parse_mrpack_manifest(&manifest).unwrap(),
        ),
        pinned_version: None,
        manual_downloads: Vec::new(),
        curseforge_lookup_available: false,
        override_file_count: 1,
        staged_dir: staged,
    };
    let mut server = ConfigServer::new(
        "recovery",
        "Recovery",
        server_dir.to_str().unwrap(),
        "server.jar",
        1.0,
        2.0,
    );
    server.java_flavor = JavaServerFlavor::Fabric;
    server.minecraft_version = Some("1.21.1".into());
    server.loader_version = Some("0.16.0".into());
    let recovered = content_recovery::review(
        &server,
        "owner",
        0,
        &[],
        &[],
        Some(&inspection),
        None,
        &NoNetwork,
        &msc_infrastructure::secret_store::FakeSecretStore::new(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        std::fs::read(server_dir.join("config/example.toml")).unwrap(),
        b"server customization"
    );
    assert_eq!(
        std::fs::read(server_dir.join("world/level.dat")).unwrap(),
        b"world stays intact"
    );
    let entry = &recovered.review.result.player_files["config/example.toml"];
    let player = msc_application::content_management::player_source(&server, entry).unwrap();
    assert_eq!(std::fs::read(player).unwrap(), b"client settings");
    let pack_id = recovered.pack_id.unwrap();
    assert_eq!(
        std::fs::read(server_dir.join(format!(".msc2-content/packs/{pack_id}/original.zip")))
            .unwrap(),
        original_bytes
    );
}
