use msc_application::import::{
    DetectedWorld, RawImportFileSystem, RawScanEntry, ScannedServerInfo, resolve_unwrap_root,
    scan_server_directory,
};
use msc_domain::identity::{JavaServerFlavor, ServerType};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// In-memory [`RawImportFileSystem`] built from a fixture's `entries`/
/// `files` (or `extractedEntries`/`extractedFiles`) trees. Intermediate
/// directories a fixture never lists explicitly (e.g. `libraries`,
/// `libraries/net`, ... for a deeply nested `unix_args.txt`) are
/// synthesized automatically, so fixtures only need to declare the paths
/// that actually matter to the case they characterize.
#[derive(Default)]
struct FakeRawFs {
    entries: BTreeMap<PathBuf, (bool, u64)>,
    files: BTreeMap<PathBuf, String>,
}

impl FakeRawFs {
    fn declare(&mut self, path: PathBuf, is_file: bool, size: u64) {
        let mut cur = path.as_path();
        while let Some(parent) = cur.parent() {
            self.entries
                .entry(parent.to_path_buf())
                .or_insert((false, 0));
            cur = parent;
        }
        self.entries.insert(path, (is_file, size));
    }
}

impl RawImportFileSystem for FakeRawFs {
    fn list_dir(&self, path: &Path) -> Vec<RawScanEntry> {
        self.entries
            .iter()
            .filter(|(p, _)| p.parent() == Some(path))
            .filter_map(|(p, (is_file, _))| {
                p.file_name().map(|name| RawScanEntry {
                    name: name.to_string_lossy().into_owned(),
                    is_file: *is_file,
                })
            })
            .collect()
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.entries.get(path).is_some_and(|(is_file, _)| !is_file)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.entries.get(path).is_some_and(|(is_file, _)| *is_file)
    }

    fn read_to_string(&self, path: &Path) -> Option<String> {
        self.files.get(path).cloned()
    }

    fn file_size(&self, path: &Path) -> u64 {
        self.entries.get(path).map(|(_, size)| *size).unwrap_or(0)
    }
}

fn load(case: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/raw-server-import")
        .join(format!("{case}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()))
}

fn build_fs(root: &Path, entries_key: &str, files_key: &str, input: &Value) -> FakeRawFs {
    let mut fs = FakeRawFs::default();
    fs.entries.insert(root.to_path_buf(), (false, 0));
    if let Some(entries) = input[entries_key].as_array() {
        for entry in entries {
            let rel = entry["path"].as_str().expect("entry.path");
            let is_file = match entry["kind"].as_str().expect("entry.kind") {
                "file" => true,
                "directory" => false,
                other => panic!("unsupported entry kind: {other}"),
            };
            let size = entry["sizeBytes"].as_u64().unwrap_or(0);
            fs.declare(root.join(rel), is_file, size);
        }
    }
    if let Some(files) = input[files_key].as_object() {
        for (rel, contents) in files {
            let path = root.join(rel);
            fs.declare(path.clone(), true, 0);
            fs.files
                .insert(path, contents.as_str().expect("file contents").to_string());
        }
    }
    fs
}

fn server_type_raw(value: ServerType) -> &'static str {
    value.raw_value()
}

fn java_flavor_raw(value: Option<JavaServerFlavor>) -> Option<&'static str> {
    value.map(|f| f.raw_value())
}

fn assert_world(actual: &DetectedWorld, expected: &Value, case: &str) {
    assert_eq!(
        actual.name,
        expected["name"].as_str().expect("world.name"),
        "{case}: world name"
    );
    assert_eq!(
        actual.folder_path,
        expected["folderPath"].as_str().expect("world.folderPath"),
        "{case}: world folderPath"
    );
    assert_eq!(
        actual.size_bytes,
        expected["sizeBytes"].as_u64().expect("world.sizeBytes"),
        "{case}: world sizeBytes"
    );
    assert_eq!(
        actual.has_nether,
        expected["hasNether"].as_bool().expect("world.hasNether"),
        "{case}: world hasNether"
    );
    assert_eq!(
        actual.has_end,
        expected["hasEnd"].as_bool().expect("world.hasEnd"),
        "{case}: world hasEnd"
    );
}

fn assert_scan(actual: &ScannedServerInfo, expected: &Value, case: &str) {
    assert_eq!(
        server_type_raw(actual.server_type),
        expected["serverType"]
            .as_str()
            .expect("expected serverType"),
        "{case}: serverType"
    );
    assert_eq!(
        actual.port,
        expected["port"].as_i64().expect("expected port"),
        "{case}: port"
    );
    assert_eq!(
        actual.max_players,
        expected["maxPlayers"]
            .as_i64()
            .expect("expected maxPlayers"),
        "{case}: maxPlayers"
    );
    assert_eq!(
        actual.eula_accepted,
        expected["eulaAccepted"]
            .as_bool()
            .expect("expected eulaAccepted"),
        "{case}: eulaAccepted"
    );
    assert_eq!(
        actual.default_world_name,
        expected["defaultWorldName"]
            .as_str()
            .expect("expected defaultWorldName"),
        "{case}: defaultWorldName"
    );
    assert_eq!(
        java_flavor_raw(actual.java_flavor),
        expected["javaFlavor"].as_str(),
        "{case}: javaFlavor"
    );
    assert_eq!(
        actual.detected_mc_version.as_deref(),
        expected["detectedMCVersion"].as_str(),
        "{case}: detectedMCVersion"
    );
    assert_eq!(
        actual.detected_loader_version.as_deref(),
        expected["detectedLoaderVersion"].as_str(),
        "{case}: detectedLoaderVersion"
    );

    let expected_worlds = expected["worlds"].as_array().expect("expected worlds");
    assert_eq!(
        actual.worlds.len(),
        expected_worlds.len(),
        "{case}: worlds count"
    );
    for (actual_world, expected_world) in actual.worlds.iter().zip(expected_worlds) {
        assert_world(actual_world, expected_world, case);
    }
}

/// Every non-zip fixture: build an in-memory tree rooted at `serverDir`
/// and scan it directly.
fn assert_directory_case(case: &str) {
    let fixture = load(case);
    let input = &fixture["input"];
    let server_dir = PathBuf::from(input["serverDir"].as_str().expect("serverDir"));
    let fs = build_fs(&server_dir, "entries", "files", input);
    let actual = scan_server_directory(&fs, &server_dir);
    assert_scan(&actual, &fixture["expected"], case);
}

macro_rules! directory_case {
    ($fn_name:ident, $case:literal) => {
        #[test]
        fn $fn_name() {
            assert_directory_case($case);
        }
    };
}

directory_case!(
    raw_server_scan_bedrock_missing_properties_use_defaults_not_typed_model,
    "bedrock-missing-properties-use-defaults-not-typed-model"
);
directory_case!(
    raw_server_scan_bedrock_selected_when_only_binary_present,
    "bedrock-selected-when-only-binary-present"
);
directory_case!(
    raw_server_scan_configured_level_name_sorted_first,
    "configured-level-name-sorted-first"
);
directory_case!(
    raw_server_scan_eula_missing_file_defaults_to_false_not_null,
    "eula-missing-file-defaults-to-false-not-null"
);
directory_case!(
    raw_server_scan_fabric_launcher_and_loader_version_lexicographic_quirk,
    "fabric-launcher-and-loader-version-lexicographic-quirk"
);
directory_case!(
    raw_server_scan_forge_detected_via_unix_args_signature,
    "forge-detected-via-unix-args-signature"
);
directory_case!(
    raw_server_scan_java_properties_port_maxplayers_levelname,
    "java-properties-port-maxplayers-levelname"
);
directory_case!(
    raw_server_scan_java_selected_when_jar_and_bedrock_binary_both_present,
    "java-selected-when-jar-and-bedrock-binary-both-present"
);
directory_case!(
    raw_server_scan_missing_jar_and_binary_still_classified_java,
    "missing-jar-and-binary-still-classified-java"
);
directory_case!(
    raw_server_scan_neoforge_detected_via_unix_args_signature,
    "neoforge-detected-via-unix-args-signature"
);
directory_case!(
    raw_server_scan_nether_and_end_companions_grouped_with_summed_size,
    "nether-and-end-companions-grouped-with-summed-size"
);
directory_case!(
    raw_server_scan_purpur_jar_name_matched,
    "purpur-jar-name-matched"
);
directory_case!(
    raw_server_scan_unmatched_jar_falls_back_to_paper,
    "unmatched-jar-falls-back-to-paper"
);
directory_case!(
    raw_server_scan_vanilla_minecraft_server_name_matched,
    "vanilla-minecraft-server-name-matched"
);
directory_case!(
    raw_server_scan_worlds_discovered_from_root_and_worlds_subdirectory,
    "worlds-discovered-from-root-and-worlds-subdirectory"
);

/// The one zip fixture: its `expected.worlds[*].folderPath` is written
/// relative to the *pre-unwrap* staging root (e.g. `"family_paper/world"`),
/// not the resolved scan directory `scan_server_directory` itself works
/// relative to (which would report just `"world"`) — see the fixture's own
/// `notes`. This test reconstructs that convention by prefixing the
/// unwrapped root's own relative name back onto each returned world path
/// before comparing, rather than changing what `scan_server_directory`
/// reports (which stays relative to whatever directory it's handed, the
/// simpler and more broadly useful contract).
#[test]
fn raw_server_scan_zip_single_root_folder_unwrapped_before_scan() {
    let fixture = load("zip-single-root-folder-unwrapped-before-scan");
    let input = &fixture["input"];
    let staging = PathBuf::from("/staging");
    let fs = build_fs(&staging, "extractedEntries", "extractedFiles", input);

    let resolved = resolve_unwrap_root(&fs, &staging);
    let unwrapped_relative = resolved
        .strip_prefix(&staging)
        .expect("resolved root should be under staging")
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        unwrapped_relative,
        fixture["expected"]["unwrappedRootPath"]
            .as_str()
            .expect("expected unwrappedRootPath")
    );

    let actual = scan_server_directory(&fs, &resolved);
    let expected = &fixture["expected"];

    assert_eq!(
        server_type_raw(actual.server_type),
        expected["serverType"].as_str().unwrap()
    );
    assert_eq!(actual.port, expected["port"].as_i64().unwrap());
    assert_eq!(actual.max_players, expected["maxPlayers"].as_i64().unwrap());
    assert_eq!(
        actual.eula_accepted,
        expected["eulaAccepted"].as_bool().unwrap()
    );
    assert_eq!(
        actual.default_world_name,
        expected["defaultWorldName"].as_str().unwrap()
    );
    assert_eq!(
        java_flavor_raw(actual.java_flavor),
        expected["javaFlavor"].as_str()
    );
    assert_eq!(
        actual.detected_mc_version.as_deref(),
        expected["detectedMCVersion"].as_str()
    );
    assert_eq!(
        actual.detected_loader_version.as_deref(),
        expected["detectedLoaderVersion"].as_str()
    );

    let expected_worlds = expected["worlds"].as_array().expect("expected worlds");
    assert_eq!(actual.worlds.len(), expected_worlds.len());
    for (actual_world, expected_world) in actual.worlds.iter().zip(expected_worlds) {
        assert_eq!(actual_world.name, expected_world["name"].as_str().unwrap());
        let full_folder_path = format!("{unwrapped_relative}/{}", actual_world.folder_path);
        assert_eq!(
            full_folder_path,
            expected_world["folderPath"].as_str().unwrap()
        );
        assert_eq!(
            actual_world.size_bytes,
            expected_world["sizeBytes"].as_u64().unwrap()
        );
        assert_eq!(
            actual_world.has_nether,
            expected_world["hasNether"].as_bool().unwrap()
        );
        assert_eq!(
            actual_world.has_end,
            expected_world["hasEnd"].as_bool().unwrap()
        );
    }
}
