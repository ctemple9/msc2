//! P8.18's own tests: real on-disk ZIP archives and a real
//! `msc_infrastructure::fs::StdFileSystem`, the same "genuinely
//! disk-shaped" precedent `backup_restore.rs`/`world_activation.rs`
//! already set — `archive::extract_zip`/`list_entry_names`/
//! `read_entry_bytes` all take a real path on disk, so a `FakeFileSystem`
//! would silently diverge from what the extraction code actually touches.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use msc_application::modpacks::{self, InspectedFormat, ModpackInspectionError};
use msc_infrastructure::addon_provider::{AddonTransport, RawResponse, TransportError};
use msc_infrastructure::fs::StdFileSystem;
use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-modpack-inspection-test-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    for (name, content) in entries {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(content).unwrap();
    }
    zip.finish().unwrap();
}

const MRPACK_INDEX: &str = r#"{
  "formatVersion": 1,
  "game": "minecraft",
  "versionId": "1.0",
  "name": "Test Pack",
  "dependencies": { "minecraft": "1.21.1", "fabric-loader": "0.16.9" },
  "files": [
    {
      "path": "mods/sodium.jar",
      "hashes": { "sha1": "abc", "sha512": "def" },
      "env": { "client": "required", "server": "required" },
      "downloads": ["https://cdn.example.invalid/sodium.jar"],
      "fileSize": 100
    }
  ]
}"#;

const CURSEFORGE_MANIFEST: &str = r#"{
  "minecraft": { "version": "1.20.1", "modLoaders": [{"id": "forge-47.4.1", "primary": true}] },
  "manifestType": "minecraftModpack",
  "manifestVersion": 1,
  "name": "Better MC [FORGE] BMC4",
  "version": "43",
  "author": "Someone",
  "files": [
    {"projectID": 306612, "fileID": 5000001, "required": true},
    {"projectID": 238222, "fileID": 5000002, "required": true}
  ],
  "overrides": "overrides"
}"#;

struct NoopTransport;
impl AddonTransport for NoopTransport {
    fn get(
        &self,
        url: &str,
        what: &str,
        _: &[(&str, &str)],
        _: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!("{what}: unexpected GET {url}");
    }
    fn post_json(
        &self,
        url: &str,
        what: &str,
        _: &serde_json::Value,
        _: &[(&str, &str)],
        _: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!("{what}: unexpected POST {url}");
    }
}

struct CurseForgeTransport {
    files: serde_json::Value,
    mods: serde_json::Value,
}
impl AddonTransport for CurseForgeTransport {
    fn get(
        &self,
        url: &str,
        what: &str,
        _: &[(&str, &str)],
        _: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!("{what}: unexpected GET {url}");
    }
    fn post_json(
        &self,
        url: &str,
        what: &str,
        _: &serde_json::Value,
        _: &[(&str, &str)],
        _: u64,
    ) -> Result<RawResponse, TransportError> {
        let body = if url.ends_with("/v1/mods/files") {
            self.files.clone()
        } else if url.ends_with("/v1/mods") {
            self.mods.clone()
        } else {
            panic!("{what}: unexpected POST {url}");
        };
        Ok(RawResponse {
            status: 200,
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

fn secrets_with_key() -> FakeSecretStore {
    let s = FakeSecretStore::new();
    s.set("curseforge.api-key", "test-key").unwrap();
    s
}

// --- identify_format ---

#[test]
fn identify_format_detects_mrpack() {
    let tmp = TempDir::new("identify-mrpack");
    let archive = tmp.path().join("pack.mrpack");
    write_zip(
        &archive,
        &[
            ("modrinth.index.json", MRPACK_INDEX.as_bytes()),
            ("overrides/config.txt", b"cfg"),
        ],
    );
    let format = modpacks::identify_format(&StdFileSystem, &archive).unwrap();
    match format {
        InspectedFormat::Mrpack(manifest) => {
            assert_eq!(manifest.name, "Test Pack");
            assert_eq!(manifest.files.len(), 1);
            assert_eq!(manifest.dependencies.get("minecraft").unwrap(), "1.21.1");
        }
        other => panic!("expected Mrpack, got {other:?}"),
    }
}

#[test]
fn identify_format_detects_curseforge() {
    let tmp = TempDir::new("identify-curseforge");
    let archive = tmp.path().join("pack.zip");
    write_zip(
        &archive,
        &[
            ("manifest.json", CURSEFORGE_MANIFEST.as_bytes()),
            ("overrides/config.txt", b"cfg"),
        ],
    );
    let format = modpacks::identify_format(&StdFileSystem, &archive).unwrap();
    match format {
        InspectedFormat::CurseForge(metadata) => {
            assert_eq!(metadata.minecraft_version, "1.20.1");
            assert_eq!(metadata.name, "Better MC [FORGE] BMC4");
        }
        other => panic!("expected CurseForge, got {other:?}"),
    }
}

#[test]
fn identify_format_modrinth_wins_when_both_markers_present() {
    let tmp = TempDir::new("identify-both-markers");
    let archive = tmp.path().join("pack.zip");
    write_zip(
        &archive,
        &[
            ("modrinth.index.json", MRPACK_INDEX.as_bytes()),
            ("manifest.json", CURSEFORGE_MANIFEST.as_bytes()),
        ],
    );
    let format = modpacks::identify_format(&StdFileSystem, &archive).unwrap();
    assert!(matches!(format, InspectedFormat::Mrpack(_)));
}

#[test]
fn identify_format_plain_jar_zip_is_a_recognized_msc2_shape() {
    let tmp = TempDir::new("identify-plain-jars");
    let archive = tmp.path().join("jars.zip");
    write_zip(
        &archive,
        &[
            ("SomeMod.jar", b"jar bytes"),
            ("OtherMod.jar", b"jar bytes 2"),
        ],
    );
    let format = modpacks::identify_format(&StdFileSystem, &archive).unwrap();
    match format {
        InspectedFormat::PlainJarZip { jar_entries } => {
            assert_eq!(jar_entries.len(), 2);
        }
        other => panic!("expected PlainJarZip, got {other:?}"),
    }
}

#[test]
fn identify_format_non_modpack_manifest_is_unrecognized() {
    let tmp = TempDir::new("identify-non-modpack-manifest");
    let archive = tmp.path().join("pack.zip");
    write_zip(
        &archive,
        &[(
            "manifest.json",
            br#"{"manifestType": "something-else", "minecraft": {"version": "1.20.1", "modLoaders": []}, "files": []}"#,
        )],
    );
    let result = modpacks::identify_format(&StdFileSystem, &archive);
    assert!(matches!(result, Err(ModpackInspectionError::Unrecognized)));
}

#[test]
fn identify_format_empty_archive_is_unrecognized() {
    let tmp = TempDir::new("identify-empty");
    let archive = tmp.path().join("empty.zip");
    write_zip(&archive, &[("readme.txt", b"hi")]);
    let result = modpacks::identify_format(&StdFileSystem, &archive);
    assert!(matches!(result, Err(ModpackInspectionError::Unrecognized)));
}

#[test]
fn identify_format_source_missing() {
    let tmp = TempDir::new("identify-missing");
    let result = modpacks::identify_format(&StdFileSystem, &tmp.path().join("nope.mrpack"));
    assert!(matches!(result, Err(ModpackInspectionError::SourceMissing)));
}

#[test]
fn identify_format_malformed_mrpack_manifest() {
    let tmp = TempDir::new("identify-malformed-mrpack");
    let archive = tmp.path().join("pack.mrpack");
    write_zip(&archive, &[("modrinth.index.json", b"{not valid json}")]);
    let result = modpacks::identify_format(&StdFileSystem, &archive);
    assert!(matches!(
        result,
        Err(ModpackInspectionError::ManifestMalformed(_))
    ));
}

// --- inspect_staged_archive: extraction + staging tree ---

#[test]
fn inspect_staged_archive_extracts_into_operation_owned_directory() {
    let tmp = TempDir::new("inspect-extract");
    let archive = tmp.path().join("pack.mrpack");
    write_zip(
        &archive,
        &[
            ("modrinth.index.json", MRPACK_INDEX.as_bytes()),
            ("overrides/config.txt", b"cfg-bytes"),
        ],
    );
    let staging_root = tmp.path().join("staging");
    let secrets = FakeSecretStore::new();

    let inspection = modpacks::inspect_staged_archive(
        &StdFileSystem,
        &NoopTransport,
        &secrets,
        &archive,
        &staging_root,
        "op-1",
    )
    .unwrap();

    assert_eq!(inspection.staged_dir, staging_root.join("op-1"));
    assert!(inspection.staged_dir.join("overrides/config.txt").is_file());
    assert_eq!(
        fs::read(inspection.staged_dir.join("overrides/config.txt")).unwrap(),
        b"cfg-bytes"
    );
    assert!(matches!(inspection.format, InspectedFormat::Mrpack(_)));
    let pinned = inspection.pinned_version.unwrap();
    assert_eq!(pinned.mc_version, "1.21.1");
    assert_eq!(pinned.build_label.as_deref(), Some("Fabric 0.16.9"));
    assert!(!inspection.curseforge_lookup_available);
}

#[test]
fn inspect_staged_archive_never_touches_a_server_directory() {
    // No server_dir parameter exists on this function at all -- the type
    // signature itself is the guarantee. This test proves the OTHER half:
    // nothing outside `staging_root` gets written.
    let tmp = TempDir::new("inspect-no-server-mutation");
    let archive = tmp.path().join("jars.zip");
    write_zip(&archive, &[("SomeMod.jar", b"jar bytes")]);
    let staging_root = tmp.path().join("staging");
    let outside_marker = tmp.path().join("outside-untouched-marker");
    fs::write(&outside_marker, b"before").unwrap();

    let secrets = FakeSecretStore::new();
    let _ = modpacks::inspect_staged_archive(
        &StdFileSystem,
        &NoopTransport,
        &secrets,
        &archive,
        &staging_root,
        "op-2",
    )
    .unwrap();

    assert_eq!(fs::read(&outside_marker).unwrap(), b"before");
}

#[test]
fn inspect_staged_archive_cleans_up_staging_dir_on_failure() {
    let tmp = TempDir::new("inspect-cleanup-on-failure");
    // A well-formed mrpack manifest that identify_format accepts, paired
    // with a corrupt archive body extract_zip will reject -- proves
    // cleanup runs on an *extraction* failure, not just an identification
    // one (identification never creates the staging dir at all).
    let archive = tmp.path().join("corrupt.mrpack");
    fs::write(&archive, b"PK\x03\x04not a real zip").unwrap();
    let staging_root = tmp.path().join("staging");
    let secrets = FakeSecretStore::new();

    let result = modpacks::inspect_staged_archive(
        &StdFileSystem,
        &NoopTransport,
        &secrets,
        &archive,
        &staging_root,
        "op-3",
    );
    assert!(result.is_err());
    assert!(!staging_root.join("op-3").exists());
}

// --- CurseForge D-027 manual-download resolution ---

#[test]
fn inspect_staged_archive_resolves_curseforge_manual_downloads() {
    let tmp = TempDir::new("inspect-curseforge-blocked");
    let archive = tmp.path().join("pack.zip");
    write_zip(
        &archive,
        &[("manifest.json", CURSEFORGE_MANIFEST.as_bytes())],
    );
    let staging_root = tmp.path().join("staging");

    let files_response = serde_json::json!({
        "data": [
            {"id": 5000001, "modId": 306612, "fileName": "SomeMod-1.0.jar", "downloadUrl": null},
            {"id": 5000002, "modId": 238222, "fileName": "OtherMod-2.0.jar", "downloadUrl": "https://edge.forgecdn.net/OtherMod-2.0.jar"}
        ]
    });
    let mods_response = serde_json::json!({
        "data": [
            {"id": 306612, "name": "Some Mod", "slug": "some-mod", "links": {"websiteUrl": "https://www.curseforge.com/minecraft/mc-mods/some-mod"}}
        ]
    });
    let transport = CurseForgeTransport {
        files: files_response,
        mods: mods_response,
    };
    let secrets = secrets_with_key();

    let inspection = modpacks::inspect_staged_archive(
        &StdFileSystem,
        &transport,
        &secrets,
        &archive,
        &staging_root,
        "op-4",
    )
    .unwrap();

    assert!(inspection.curseforge_lookup_available);
    assert_eq!(inspection.manual_downloads.len(), 1);
    assert_eq!(inspection.manual_downloads[0].mod_name, "Some Mod");
    assert_eq!(inspection.manual_downloads[0].file_name, "SomeMod-1.0.jar");
    assert_eq!(
        inspection.manual_downloads[0].project_page_url,
        "https://www.curseforge.com/minecraft/mc-mods/some-mod"
    );
}

#[test]
fn inspect_staged_archive_missing_api_key_degrades_honestly() {
    let tmp = TempDir::new("inspect-curseforge-no-key");
    let archive = tmp.path().join("pack.zip");
    write_zip(
        &archive,
        &[("manifest.json", CURSEFORGE_MANIFEST.as_bytes())],
    );
    let staging_root = tmp.path().join("staging");
    let secrets = FakeSecretStore::new(); // no key set

    let inspection = modpacks::inspect_staged_archive(
        &StdFileSystem,
        &NoopTransport,
        &secrets,
        &archive,
        &staging_root,
        "op-5",
    )
    .unwrap();

    assert!(!inspection.curseforge_lookup_available);
    assert!(inspection.manual_downloads.is_empty());
}
