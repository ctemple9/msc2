//! P3.4's own tests: prove `StdFileSystem` round-trips against the real
//! filesystem and `FakeFileSystem` behaves the same way in memory,
//! including construction straight from a fixture-shaped `fsTree`.

use msc_infrastructure::fs::{FakeFileSystem, FileSystem, StdFileSystem};
use serde_json::json;
use std::path::PathBuf;

fn temp_dir(case: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "msc-infrastructure-fs-test-{case}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn std_file_system_round_trips_read_write_stat() {
    let dir = temp_dir("std-round-trip");
    let path = dir.join("hello.txt");

    let fs = StdFileSystem;
    fs.write(&path, b"hello").expect("write");
    assert_eq!(fs.read(&path).expect("read"), b"hello");

    let meta = fs.stat(&path).expect("stat");
    assert!(meta.is_file);
    assert!(!meta.is_dir);

    let renamed = dir.join("renamed.txt");
    fs.rename(&path, &renamed).expect("rename");
    assert!(fs.read(&path).is_err());
    assert_eq!(fs.read(&renamed).expect("read renamed"), b"hello");

    fs.remove(&renamed).expect("remove");
    assert!(fs.read(&renamed).is_err());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn std_file_system_lists_directory_entries() {
    let dir = temp_dir("std-list");
    let fs = StdFileSystem;
    fs.write(&dir.join("a.txt"), b"a").expect("write a");
    fs.write(&dir.join("b.txt"), b"b").expect("write b");

    let mut entries: Vec<_> = fs
        .list(&dir)
        .expect("list")
        .into_iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    entries.sort();
    assert_eq!(entries, vec!["a.txt", "b.txt"]);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn fake_file_system_round_trips_read_write_stat() {
    let fs = FakeFileSystem::new().with_file("/srv/server1/eula.txt", "eula=false", false);

    assert_eq!(
        fs.read(std::path::Path::new("/srv/server1/eula.txt"))
            .expect("read"),
        b"eula=false"
    );

    let meta = fs
        .stat(std::path::Path::new("/srv/server1/eula.txt"))
        .expect("stat");
    assert!(meta.is_file);
    assert!(!meta.executable);

    fs.write(std::path::Path::new("/srv/server1/eula.txt"), b"eula=true")
        .expect("write");
    assert_eq!(
        fs.read(std::path::Path::new("/srv/server1/eula.txt"))
            .expect("read after write"),
        b"eula=true"
    );

    assert!(
        fs.read(std::path::Path::new("/srv/server1/missing"))
            .is_err()
    );
}

/// The shape P3.18's deferred `java-runtime-guards` fsTree fixtures use —
/// proves `FakeFileSystem::from_tree` consumes it without reshaping.
#[test]
fn fake_file_system_builds_from_fixture_fs_tree() {
    let tree = json!({
        "<TMP>/broken-21.jdk/Contents/Home/bin/java": {
            "type": "file",
            "executable": false
        },
        "<TMP>/good-21.jdk/Contents/Home/bin/java": {
            "type": "file",
            "executable": true
        }
    });

    let fs = FakeFileSystem::from_tree(&tree);

    let broken = std::path::Path::new("<TMP>/broken-21.jdk/Contents/Home/bin/java");
    let good = std::path::Path::new("<TMP>/good-21.jdk/Contents/Home/bin/java");

    assert!(!fs.stat(broken).expect("stat broken").executable);
    assert!(fs.stat(good).expect("stat good").executable);

    let dir = std::path::Path::new("<TMP>/good-21.jdk/Contents/Home/bin");
    let listed = fs.list(dir).expect("list");
    assert_eq!(listed, vec![good.to_path_buf()]);
}
