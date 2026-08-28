//! P5.20: the mutating raw folder/ZIP importer (`import_raw_server`).
//! No fixture oracle exists for this function — P5.18 scoped
//! `fixtures/raw-server-import/` to the read-only scan half only — so
//! these tests exercise real temp-directory trees directly, the same
//! "genuinely disk-shaped" precedent `transfer_apply.rs` set for
//! P5.15/P5.16.

use msc_application::import::{
    ImportedRawServer, RawImportError, RawImportOverrides, RawImportRequest, RawImportSource,
    import_raw_server,
};
use msc_domain::identity::{JavaServerFlavor, ServerType};
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "msc2-raw-import-{name}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        // macOS's real temp dir lives under a symlink (`/var` ->
        // `/private/var`); `safe_path` resolves symlinks (matching
        // MSC 1's `resolvingSymlinksInPath()`), so comparisons below
        // need the same canonical form or every path assertion would
        // spuriously fail on `/var/...` vs. `/private/var/...`.
        let path = std::fs::canonicalize(&path).unwrap();
        Self { path }
    }

    fn source_dir(&self) -> PathBuf {
        self.path.join("source")
    }

    fn servers_root(&self) -> PathBuf {
        self.path.join("servers")
    }

    fn home_dir(&self) -> PathBuf {
        self.path.join("home")
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed).to_string()
}

fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

/// A minimal on-disk Paper folder: one jar, server.properties, eula.txt.
fn write_paper_source(dir: &Path) {
    write_file(&dir.join("paper-1.21.1-131.jar"), "");
    write_file(
        &dir.join("server.properties"),
        "server-port=25565\nmax-players=20\nlevel-name=world\n",
    );
    write_file(&dir.join("eula.txt"), "eula=true\n");
}

fn write_zip(zip_path: &Path, entries: &[(&str, &[u8])]) {
    let file = std::fs::File::create(zip_path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    for (name, bytes) in entries {
        zip.start_file(*name, opts).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}

fn base_request(
    temp: &TempRoot,
    source: RawImportSource,
    server_type: ServerType,
) -> RawImportRequest {
    RawImportRequest {
        display_name: "My Server".to_string(),
        server_type,
        source,
        servers_root: temp.servers_root(),
        overrides: RawImportOverrides::default(),
    }
}

#[test]
fn raw_server_import_copies_folder_and_registers_server() {
    let temp = TempRoot::new("copy-folder");
    write_paper_source(&temp.source_dir());

    let request = base_request(
        &temp,
        RawImportSource::Folder(temp.source_dir()),
        ServerType::Java,
    );
    let imported: ImportedRawServer =
        import_raw_server(&request, &temp.home_dir()).expect("import should succeed");

    let expected_dest = temp.servers_root().join("java").join("my_server");
    assert!(expected_dest.join("paper-1.21.1-131.jar").is_file());
    assert_eq!(imported.config.display_name, "My Server");
    assert_eq!(imported.config.server_dir, expected_dest.to_string_lossy());
    assert_eq!(imported.config.server_type, ServerType::Java);
    assert_eq!(imported.config.java_flavor, JavaServerFlavor::Paper);
    assert_eq!(imported.config.minecraft_version.as_deref(), Some("1.21.1"));
    assert!(
        imported
            .config
            .paper_jar_path
            .contains("paper-1.21.1-131.jar"),
        "paper_jar_path: {}",
        imported.config.paper_jar_path
    );

    // Original source is untouched (a copy, not a move).
    assert!(temp.source_dir().join("paper-1.21.1-131.jar").is_file());
}

#[test]
fn raw_server_import_extracts_zip_and_unwraps_single_root() {
    let temp = TempRoot::new("zip-unwrap");
    let zip_path = temp.path.join("server.zip");
    write_zip(
        &zip_path,
        &[
            ("family_paper/paper-1.21.1-131.jar", b""),
            (
                "family_paper/server.properties",
                b"server-port=25565\nmax-players=20\nlevel-name=world\n",
            ),
            ("family_paper/eula.txt", b"eula=true\n"),
        ],
    );

    let request = base_request(&temp, RawImportSource::Zip(zip_path), ServerType::Java);
    let imported =
        import_raw_server(&request, &temp.home_dir()).expect("zip import should succeed");

    // Source line 136/478-494: the unwrap makes the *effective* server
    // directory the nested `family_paper` folder, not the destination
    // folder the zip was extracted into — `server_dir` reflects that.
    let expected_dest = temp
        .servers_root()
        .join("java")
        .join("my_server")
        .join("family_paper");
    assert_eq!(imported.config.server_dir, expected_dest.to_string_lossy());
    assert!(expected_dest.join("paper-1.21.1-131.jar").is_file());
}

#[test]
fn raw_server_import_refuses_existing_destination() {
    let temp = TempRoot::new("collision");
    write_paper_source(&temp.source_dir());
    let dest = temp.servers_root().join("java").join("my_server");
    std::fs::create_dir_all(&dest).unwrap();
    write_file(&dest.join("marker.txt"), "already here");

    let request = base_request(
        &temp,
        RawImportSource::Folder(temp.source_dir()),
        ServerType::Java,
    );
    let error = import_raw_server(&request, &temp.home_dir())
        .expect_err("a colliding destination must be refused");
    assert!(matches!(error, RawImportError::DestinationExists { .. }));

    // The pre-existing destination is left exactly as it was.
    assert!(dest.join("marker.txt").is_file());
    assert!(!dest.join("paper-1.21.1-131.jar").exists());
}

#[test]
fn raw_server_import_rejects_traversal_zip_without_leaving_destination() {
    let temp = TempRoot::new("zip-traversal");
    let zip_path = temp.path.join("evil.zip");
    write_zip(&zip_path, &[("../../etc/passwd", b"pwned")]);

    let request = base_request(&temp, RawImportSource::Zip(zip_path), ServerType::Java);
    let error = import_raw_server(&request, &temp.home_dir())
        .expect_err("a traversal zip entry must be rejected");
    assert!(matches!(error, RawImportError::UnsafeZipEntry { .. }));

    let dest = temp.servers_root().join("java").join("my_server");
    assert!(
        !dest.exists(),
        "partial destination must be removed on failure"
    );
}

#[cfg(unix)]
#[test]
fn raw_server_import_rejects_symlink_in_folder_source() {
    let temp = TempRoot::new("folder-symlink");
    write_paper_source(&temp.source_dir());
    let outside = temp.path.join("outside-secret.txt");
    write_file(&outside, "secret");
    std::os::unix::fs::symlink(&outside, temp.source_dir().join("escape-link")).unwrap();

    let request = base_request(
        &temp,
        RawImportSource::Folder(temp.source_dir()),
        ServerType::Java,
    );
    let error = import_raw_server(&request, &temp.home_dir())
        .expect_err("a symlink in the source folder must be rejected");
    assert!(matches!(error, RawImportError::UnsafeSymlink { .. }));

    let dest = temp.servers_root().join("java").join("my_server");
    assert!(
        !dest.exists(),
        "partial destination must be removed on failure"
    );
}

#[test]
fn raw_server_import_applies_port_max_players_world_name_and_eula_overrides() {
    let temp = TempRoot::new("overrides");
    write_paper_source(&temp.source_dir());

    let mut request = base_request(
        &temp,
        RawImportSource::Folder(temp.source_dir()),
        ServerType::Java,
    );
    request.overrides = RawImportOverrides {
        port: Some(25577),
        max_players: Some(8),
        active_world_name: Some("survival".to_string()),
        eula_accepted: Some(true),
        enable_playit: Some(true),
    };

    let imported = import_raw_server(&request, &temp.home_dir()).expect("import should succeed");
    let dest = PathBuf::from(&imported.config.server_dir);
    let props = std::fs::read_to_string(dest.join("server.properties")).unwrap();
    assert!(props.contains("server-port=25577"));
    assert!(props.contains("max-players=8"));
    assert!(props.contains("level-name=survival"));

    let eula = std::fs::read_to_string(dest.join("eula.txt")).unwrap();
    assert_eq!(eula, "eula=true\n");
    assert!(imported.config.playit_enabled);
}

/// Source line 81: `enablePlayit: Bool = false` -- an import that never
/// supplies the override must register with playit disabled, matching
/// every other freshly-imported server's default and `ConfigServer::new`'s
/// own default (`app_config_schema.rs`), not silently inherit `true` from
/// whatever the copied source folder happened to contain.
#[test]
fn raw_server_import_defaults_playit_disabled_when_override_omitted() {
    let temp = TempRoot::new("playit-default");
    write_paper_source(&temp.source_dir());

    let request = base_request(
        &temp,
        RawImportSource::Folder(temp.source_dir()),
        ServerType::Java,
    );
    let imported = import_raw_server(&request, &temp.home_dir()).expect("import should succeed");
    assert!(!imported.config.playit_enabled);
}

#[test]
fn raw_server_import_eula_override_false_leaves_source_file_untouched() {
    let temp = TempRoot::new("eula-false");
    write_file(&temp.source_dir().join("paper-1.21.1-131.jar"), "");
    write_file(&temp.source_dir().join("eula.txt"), "eula=false\n");

    let mut request = base_request(
        &temp,
        RawImportSource::Folder(temp.source_dir()),
        ServerType::Java,
    );
    request.overrides.eula_accepted = Some(false);

    let imported = import_raw_server(&request, &temp.home_dir()).expect("import should succeed");
    let dest = PathBuf::from(&imported.config.server_dir);
    let eula = std::fs::read_to_string(dest.join("eula.txt")).unwrap();
    assert_eq!(eula, "eula=false\n");
}

/// Source line 150-172: a Bedrock import's `cfgServer.bedrockPort` is
/// stamped from the *pre-override* scanned port, even though the
/// override is faithfully written to `server.properties` on disk. A
/// genuine MSC 1 quirk, preserved as-is per CLAUDE.md.
#[test]
fn raw_server_import_bedrock_port_override_quirk_not_reflected_in_config() {
    let temp = TempRoot::new("bedrock-quirk");
    write_file(&temp.source_dir().join("bedrock_server"), "");
    write_file(
        &temp.source_dir().join("server.properties"),
        "server-port=19132\nmax-players=10\n",
    );

    let mut request = base_request(
        &temp,
        RawImportSource::Folder(temp.source_dir()),
        ServerType::Bedrock,
    );
    request.overrides.port = Some(19999);

    let imported = import_raw_server(&request, &temp.home_dir()).expect("import should succeed");
    let dest = PathBuf::from(&imported.config.server_dir);
    let props = std::fs::read_to_string(dest.join("server.properties")).unwrap();
    assert!(props.contains("server-port=19999"), "on-disk port: {props}");
    assert_eq!(
        imported.config.bedrock_port,
        Some(19132),
        "bedrock_port should keep the pre-override scanned value"
    );
}

#[test]
fn raw_server_import_empty_display_name_is_rejected() {
    let temp = TempRoot::new("empty-name");
    write_paper_source(&temp.source_dir());

    let mut request = base_request(
        &temp,
        RawImportSource::Folder(temp.source_dir()),
        ServerType::Java,
    );
    request.display_name = "   ".to_string();

    let error = import_raw_server(&request, &temp.home_dir()).expect_err("must be rejected");
    assert!(matches!(error, RawImportError::EmptyDisplayName));
}

#[test]
fn raw_server_import_sanitizes_and_length_limits_destination_name() {
    let temp = TempRoot::new("sanitize-name");
    write_paper_source(&temp.source_dir());

    let mut request = base_request(
        &temp,
        RawImportSource::Folder(temp.source_dir()),
        ServerType::Java,
    );
    request.display_name = "  Cam's Modded!! Server (2)  ".to_string();

    let imported = import_raw_server(&request, &temp.home_dir()).expect("import should succeed");
    let dest = PathBuf::from(&imported.config.server_dir);
    let folder_name = dest.file_name().unwrap().to_string_lossy().into_owned();
    assert_eq!(folder_name, "cams_modded_server_2");
    assert!(
        folder_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    );
}

#[test]
fn raw_server_import_rejects_source_not_found() {
    let temp = TempRoot::new("missing-source");
    let request = base_request(
        &temp,
        RawImportSource::Folder(temp.source_dir()),
        ServerType::Java,
    );
    let error = import_raw_server(&request, &temp.home_dir()).expect_err("must be rejected");
    assert!(matches!(error, RawImportError::SourceNotFound { .. }));
}
