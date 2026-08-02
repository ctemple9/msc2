use msc_application::import::{
    DirectoryEntry, ImportedPaperServer, PaperImportError, PaperImportFileSystem,
    PaperImportRequest, PaperServerRegistry, import_existing_paper_server,
};
use msc_domain::identity::JavaServerFlavor;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

struct Fixture {
    input: Value,
    expected: Value,
}

#[derive(Default)]
struct FakeFileSystem {
    directories: BTreeMap<PathBuf, Vec<DirectoryEntry>>,
    files: BTreeMap<PathBuf, String>,
}

impl PaperImportFileSystem for FakeFileSystem {
    fn read_dir(&self, path: &Path) -> Result<Vec<DirectoryEntry>, PaperImportError> {
        self.directories
            .get(path)
            .cloned()
            .ok_or_else(|| PaperImportError::ReadDirectory {
                path: path.to_path_buf(),
                message: "missing fake directory".to_string(),
            })
    }

    fn read_to_string(&self, path: &Path) -> Result<Option<String>, PaperImportError> {
        Ok(self.files.get(path).cloned())
    }
}

#[derive(Default)]
struct FakeRegistry {
    servers: Vec<ImportedPaperServer>,
}

impl PaperServerRegistry for FakeRegistry {
    fn register(&mut self, server: ImportedPaperServer) -> Result<(), PaperImportError> {
        self.servers.push(server);
        Ok(())
    }
}

fn load(case: &str) -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/paper-import")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    let json: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()));
    Fixture {
        input: json["input"].clone(),
        expected: json["expected"].clone(),
    }
}

fn fixture_request(input: &Value) -> PaperImportRequest {
    PaperImportRequest::new(
        input["displayName"].as_str().expect("displayName"),
        input["serverDir"].as_str().expect("serverDir"),
    )
}

fn fixture_fs(input: &Value) -> FakeFileSystem {
    let server_dir = PathBuf::from(input["serverDir"].as_str().expect("serverDir"));
    let entries = input["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|entry| {
            let name = entry["name"].as_str().expect("entry.name");
            match entry["kind"].as_str().expect("entry.kind") {
                "file" => DirectoryEntry::file(name),
                "directory" => DirectoryEntry::directory(name),
                other => panic!("unsupported entry kind: {other}"),
            }
        })
        .collect::<Vec<_>>();

    let mut files = BTreeMap::new();
    let files_json = input["files"].as_object().expect("files");
    for (relative, contents) in files_json {
        files.insert(
            server_dir.join(relative),
            contents.as_str().expect("file contents").to_string(),
        );
    }

    FakeFileSystem {
        directories: BTreeMap::from([(server_dir, entries)]),
        files,
    }
}

fn assert_import_case(case: &str) {
    let fixture = load(case);
    let fs = fixture_fs(&fixture.input);
    let request = fixture_request(&fixture.input);
    let mut registry = FakeRegistry::default();
    let actual = import_existing_paper_server(&fs, &mut registry, &request);

    if let Some(error_contains) = fixture.expected["errorContains"].as_str() {
        let error = actual.expect_err("fixture expected import to fail");
        assert!(
            error.to_string().contains(error_contains),
            "{case}: {error}"
        );
        assert!(
            registry.servers.is_empty(),
            "{case}: failed import registered"
        );
        return;
    }

    let imported = actual.expect("fixture expected import to succeed");
    assert_eq!(registry.servers, vec![imported.clone()]);

    assert_eq!(
        imported.id.as_str(),
        fixture.expected["id"].as_str().expect("expected id")
    );
    assert_eq!(
        imported.display_name,
        fixture.expected["displayName"]
            .as_str()
            .expect("expected displayName")
    );
    assert_eq!(
        imported.paper_jar_path,
        PathBuf::from(
            fixture.expected["paperJarPath"]
                .as_str()
                .expect("expected paperJarPath")
        )
    );
    assert_eq!(
        imported.eula_accepted,
        fixture.expected["eulaAccepted"].as_bool()
    );
    assert_eq!(
        imported.game_port,
        fixture.expected["gamePort"]
            .as_i64()
            .expect("expected gamePort")
    );
    assert_eq!(
        imported.max_players,
        fixture.expected["maxPlayers"]
            .as_i64()
            .expect("expected maxPlayers")
    );
    assert_eq!(
        imported.world_name,
        fixture.expected["worldName"]
            .as_str()
            .expect("expected worldName")
    );

    let expected_raw = fixture.expected["rawProperties"]
        .as_object()
        .expect("expected rawProperties");
    for (key, value) in expected_raw {
        assert_eq!(
            imported
                .properties
                .raw_properties
                .get(key)
                .map(String::as_str),
            value.as_str(),
            "{case}: raw property {key}"
        );
    }

    let lifecycle_server = imported.lifecycle_server();
    assert_eq!(lifecycle_server.id, imported.id);
    assert_eq!(lifecycle_server.flavor, JavaServerFlavor::Paper);
    assert_eq!(lifecycle_server.directory, imported.server_dir);
}

#[test]
fn paper_import_detects_existing_paper_server() {
    assert_import_case("detects-existing-paper-server");
}

#[test]
fn paper_import_missing_properties_use_minecraft_defaults() {
    assert_import_case("missing-properties-use-minecraft-defaults");
}

#[test]
fn paper_import_eula_false_is_not_accepted() {
    assert_import_case("eula-false-is-not-accepted");
}

#[test]
fn paper_import_preserves_unknown_server_properties() {
    assert_import_case("preserves-unknown-server-properties");
}

#[test]
fn paper_import_prefers_paper_jar_over_generic_jar() {
    assert_import_case("prefers-paper-jar-over-generic-jar");
}

#[test]
fn paper_import_rejects_directory_without_java_jar() {
    assert_import_case("rejects-directory-without-java-jar");
}
