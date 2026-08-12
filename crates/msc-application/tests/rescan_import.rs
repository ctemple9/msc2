//! P5.22: `rescan_and_import_servers`, MSC 1's config-recovery rescan
//! (`AppViewModel+ConfigRecovery.swift:103-183`). No fixture oracle exists
//! for this function either (same precedent P5.20's own test file set) —
//! these tests exercise real temp-directory trees directly, matching
//! `raw_server_import.rs`'s approach.

use msc_application::import::{StdRawImportFileSystem, rescan_and_import_servers};
use msc_domain::identity::{JavaServerFlavor, ServerType};
use std::path::{Path, PathBuf};

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "msc2-rescan-import-{name}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
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

/// A snapshot of every regular file under `root`, relative to `root` — used
/// to prove a rescan pass mutates nothing on disk.
fn file_snapshot(root: &Path) -> Vec<(PathBuf, String)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(PathBuf, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                let contents = std::fs::read_to_string(&path).unwrap_or_default();
                out.push((path.strip_prefix(root).unwrap().to_path_buf(), contents));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

#[test]
fn rescan_import_finds_servers_under_root_and_typed_subdirectories_and_skips_the_typed_dirs_themselves()
 {
    let temp = TempRoot::new("root-typed-overlap");
    let root = temp.path.join("servers");

    // A server sitting directly under the root.
    write_file(&root.join("direct_server/paper-1.21.1-1.jar"), "");
    // A server under the `java` typed subdirectory.
    write_file(&root.join("java/java_server/paper-1.21.1-1.jar"), "");
    // A server under the `bedrock` typed subdirectory.
    write_file(&root.join("bedrock/bedrock_server_dir/bedrock_server"), "");

    let before = file_snapshot(&root);
    let result = rescan_and_import_servers(&StdRawImportFileSystem, &root, &[]);
    let after = file_snapshot(&root);

    let names: Vec<String> = result
        .added
        .iter()
        .map(|s| s.display_name.clone())
        .collect();
    assert_eq!(names.len(), 3, "expected 3 servers added, got {names:?}");
    assert!(names.contains(&"direct server".to_string()));
    assert!(names.contains(&"java server".to_string()));
    assert!(names.contains(&"bedrock server dir".to_string()));

    // The `java`/`bedrock` typed directories themselves surface as
    // candidates from the root-level listing (source line 112-135), but
    // have no jar/binary of their own directly inside them, so they're
    // skipped, not added.
    assert_eq!(
        result.skipped, 2,
        "expected java/ and bedrock/ to be skipped"
    );

    assert_eq!(before, after, "rescan must not mutate the filesystem");
}

#[test]
fn rescan_import_skips_already_tracked_directories() {
    let temp = TempRoot::new("tracked-paths");
    let root = temp.path.join("servers");
    let tracked_dir = root.join("java/tracked_server");
    write_file(&tracked_dir.join("paper-1.21.1-1.jar"), "");
    write_file(&root.join("java/untracked_server/paper-1.21.1-1.jar"), "");

    let existing = vec![tracked_dir.to_string_lossy().into_owned()];
    let result = rescan_and_import_servers(&StdRawImportFileSystem, &root, &existing);

    assert_eq!(result.added.len(), 1);
    assert_eq!(result.added[0].display_name, "untracked server");
    // The `java/` typed subdirectory itself also surfaces as a candidate
    // from the root-level listing (see the root/typed-subdirectory-overlap
    // test above) and is skipped for lacking its own jar/binary.
    assert_eq!(result.skipped, 1);
}

#[test]
fn rescan_import_skips_directories_with_neither_jar_nor_bedrock_binary() {
    let temp = TempRoot::new("nonservers");
    let root = temp.path.join("servers");
    write_file(&root.join("java/not_a_server/readme.txt"), "hello");
    write_file(&root.join("java/real_server/paper-1.21.1-1.jar"), "");

    let result = rescan_and_import_servers(&StdRawImportFileSystem, &root, &[]);

    assert_eq!(result.added.len(), 1);
    assert_eq!(result.added[0].display_name, "real server");
    // One for `not_a_server` itself, one for the `java/` typed subdirectory
    // (see the root/typed-subdirectory-overlap test above).
    assert_eq!(result.skipped, 2);
}

#[test]
fn rescan_import_classifies_java_and_bedrock_and_detects_java_flavor() {
    let temp = TempRoot::new("java-bedrock-detection");
    let root = temp.path.join("servers");
    write_file(&root.join("java/paper_server/paper-1.21.1-131.jar"), "");
    write_file(&root.join("bedrock/bedrock_server_1/bedrock_server"), "");

    let result = rescan_and_import_servers(&StdRawImportFileSystem, &root, &[]);
    assert_eq!(result.added.len(), 2);

    let java_server = result
        .added
        .iter()
        .find(|s| s.display_name == "paper server")
        .expect("java server should be added");
    assert_eq!(java_server.server_type, ServerType::Java);
    assert!(java_server.has_ever_started);
    assert_eq!(java_server.java_flavor, JavaServerFlavor::Paper);
    assert_eq!(java_server.minecraft_version.as_deref(), Some("1.21.1"));
    assert!(java_server.paper_jar_path.contains("paper-1.21.1-131.jar"));

    let bedrock_server = result
        .added
        .iter()
        .find(|s| s.display_name == "bedrock server 1")
        .expect("bedrock server should be added");
    assert_eq!(bedrock_server.server_type, ServerType::Bedrock);
    assert!(bedrock_server.has_ever_started);
}

#[test]
fn rescan_import_performs_no_filesystem_mutation() {
    let temp = TempRoot::new("no-mutation");
    let root = temp.path.join("servers");
    write_file(&root.join("java/some_server/paper-1.21.1-1.jar"), "");

    let before = file_snapshot(&temp.path);
    let result = rescan_and_import_servers(&StdRawImportFileSystem, &root, &[]);
    let after = file_snapshot(&temp.path);

    assert_eq!(result.added.len(), 1);
    assert_eq!(before, after);
}
