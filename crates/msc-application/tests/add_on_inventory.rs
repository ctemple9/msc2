//! Port of `fixtures/installed-addons/`'s 8 cases (P7.36), exercising
//! `msc_application::add_on_inventory::{scan_mods, scan_plugins}` against
//! real on-disk jars via `StdFileSystem` -- `msc_infrastructure::archive
//! ::read_entry_bytes` (the zip-entry reader `scan_mods` calls) takes a
//! real path, not a `&dyn FileSystem` handle, matching the established
//! `provisioning.rs`/`jar_provider.rs` precedent this module's own doc
//! cites.

use msc_application::add_on_inventory::{scan_mods, scan_plugins};
use msc_domain::crash_analysis::{ModEntry, PluginEntry};
use msc_infrastructure::fs::StdFileSystem;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-add-on-inventory-test-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
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

fn write_test_jar(path: &Path, entries: &[(String, String)]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    for (name, contents) in entries {
        zip.start_file(name, opts).unwrap();
        std::io::Write::write_all(&mut zip, contents.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/installed-addons")
}

fn load(case: &str) -> Value {
    let path = fixtures_dir().join(format!("{case}.json"));
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()))
}

/// Materializes every `input.files[]` entry onto real disk under `dir`:
/// `zipEntries` (a map, possibly empty) builds a real jar via
/// [`write_test_jar`]; `corruptBytes` writes the literal string as
/// non-zip bytes; `zipEntries: null` writes the filename as a plain text
/// file (contents irrelevant, proving the extension filter never opens
/// it).
fn build_files(dir: &Path, files: &[Value]) {
    for file in files {
        let name = file["name"].as_str().unwrap();
        let path = dir.join(name);
        if let Some(entries) = file.get("zipEntries") {
            if entries.is_null() {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(&path, b"not a jar").unwrap();
                continue;
            }
            let entries: Vec<(String, String)> = entries
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                .collect();
            write_test_jar(&path, &entries);
        } else if let Some(corrupt) = file.get("corruptBytes").and_then(Value::as_str) {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, corrupt.as_bytes()).unwrap();
        } else {
            panic!("fixture file {name} has neither zipEntries nor corruptBytes");
        }
    }
}

fn expected_mod_entries(fixture: &Value) -> Vec<ModEntry> {
    fixture
        .expected_entries()
        .iter()
        .map(mod_entry_from)
        .collect()
}

fn expected_plugin_entries(fixture: &Value) -> Vec<PluginEntry> {
    fixture
        .expected_entries()
        .iter()
        .map(plugin_entry_from)
        .collect()
}

trait ExpectedEntries {
    fn expected_entries(&self) -> Vec<Value>;
}

impl ExpectedEntries for Value {
    fn expected_entries(&self) -> Vec<Value> {
        self["expected"]["entries"].as_array().unwrap().clone()
    }
}

fn mod_entry_from(v: &Value) -> ModEntry {
    ModEntry {
        filename: v["filename"].as_str().unwrap().to_string(),
        jar_stem: v["jarStem"].as_str().unwrap().to_string(),
        display_name: v["displayName"].as_str().unwrap().to_string(),
        mod_id: v["modId"].as_str().map(str::to_string),
        version: v["version"].as_str().map(str::to_string),
        is_enabled: v["isEnabled"].as_bool().unwrap(),
    }
}

fn plugin_entry_from(v: &Value) -> PluginEntry {
    PluginEntry {
        filename: v["filename"].as_str().unwrap().to_string(),
        jar_stem: v["jarStem"].as_str().unwrap().to_string(),
        display_name: v["displayName"].as_str().unwrap().to_string(),
        version: v["version"].as_str().map(str::to_string),
        is_enabled: v["isEnabled"].as_bool().unwrap(),
    }
}

fn run_mods(case: &str) {
    let fixture = load(case);
    let tmp = TempDir::new(case);
    let mods_dir = tmp.path().join("mods");
    fs::create_dir_all(&mods_dir).unwrap();
    build_files(&mods_dir, fixture["input"]["files"].as_array().unwrap());

    let actual = scan_mods(&StdFileSystem, &mods_dir);
    assert_eq!(actual, expected_mod_entries(&fixture), "case {case}");
}

fn run_plugins(case: &str) {
    let fixture = load(case);
    let tmp = TempDir::new(case);
    let plugins_dir = tmp.path().join("plugins");
    fs::create_dir_all(&plugins_dir).unwrap();
    build_files(&plugins_dir, fixture["input"]["files"].as_array().unwrap());

    let actual = scan_plugins(&StdFileSystem, &plugins_dir);
    assert_eq!(actual, expected_plugin_entries(&fixture), "case {case}");
}

#[test]
fn add_on_inventory_mods_enabled_and_disabled_jars_listed_with_filename_heuristics() {
    run_mods("mods-enabled-and-disabled-jars-listed-with-filename-heuristics");
}

#[test]
fn add_on_inventory_mods_fabric_mod_json_metadata_preferred_over_filename() {
    run_mods("mods-fabric-mod-json-metadata-preferred-over-filename");
}

#[test]
fn add_on_inventory_mods_forge_mods_toml_metadata_parsed_first_block_only() {
    run_mods("mods-forge-mods-toml-metadata-parsed-first-block-only");
}

#[test]
fn add_on_inventory_mods_corrupt_archive_falls_back_to_filename_heuristics() {
    run_mods("mods-corrupt-archive-falls-back-to-filename-heuristics");
}

#[test]
fn add_on_inventory_mods_duplicate_jar_stems_both_listed_not_deduplicated() {
    run_mods("mods-duplicate-jar-stems-both-listed-not-deduplicated");
}

#[test]
fn add_on_inventory_mods_non_jar_files_in_directory_are_ignored() {
    run_mods("mods-non-jar-files-in-directory-are-ignored");
}

#[test]
fn add_on_inventory_plugins_filename_heuristic_only_even_with_plugin_yml_present() {
    run_plugins("plugins-filename-heuristic-only-even-with-plugin-yml-present");
}

#[test]
fn add_on_inventory_plugins_enabled_and_disabled_jars_listed_with_filename_heuristics() {
    run_plugins("plugins-enabled-and-disabled-jars-listed-with-filename-heuristics");
}

// --- P7.36's own decided-for-you defense: not an oracle-fidelity case
// (MSC 1 has no path-containment handling at all -- confirmed by reading
// both scanners directly, see add_on_inventory.rs's own module doc), so
// this is a plain test rather than a fixture. ---

#[test]
fn add_on_inventory_scan_mods_never_reads_outside_the_target_directory() {
    let tmp = TempDir::new("path-containment");
    let mods_dir = tmp.path().join("mods");
    fs::create_dir_all(&mods_dir).unwrap();
    write_test_jar(&mods_dir.join("Legit.jar"), &[]);
    // A file living *outside* mods_dir, sharing its parent -- proves
    // fs.list() scoping to mods_dir alone (not a suffix/substring match)
    // is what keeps it out, the same boundary a hostile filename would
    // need to escape.
    write_test_jar(&tmp.path().join("Sneaky.jar"), &[]);

    let entries = scan_mods(&StdFileSystem, &mods_dir);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].filename, "Legit.jar");
}
