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

/// P3.20a: `list()` used to build its result with `Path::join`, which
/// inserts `std::path::MAIN_SEPARATOR` — a backslash on Windows — even
/// though every fixture path here is written with forward slashes. That
/// broke any caller comparing the result as a raw string (found in
/// `audit_log.rs`'s test by P3.20's exit gate check, on Windows CI only).
/// `PathBuf` equality is component-based and wouldn't have caught this, so
/// this test checks the literal string form directly.
#[test]
fn fake_file_system_list_returns_forward_slash_paths() {
    let tree = json!({
        "/srv/app/audit/audit-2023-10-15.jsonl": {
            "type": "file",
            "executable": false
        }
    });
    let fs = FakeFileSystem::from_tree(&tree);

    let listed = fs
        .list(std::path::Path::new("/srv/app/audit"))
        .expect("list");
    let as_strings: Vec<String> = listed
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();

    assert_eq!(as_strings, vec!["/srv/app/audit/audit-2023-10-15.jsonl"]);
}

/// P7.33: `create_dir_exclusive` succeeds once, then refuses cleanly with
/// `AlreadyExists` on a second call against the same path — the atomic
/// claim `msc-application::provisioning`'s creation functions now use
/// instead of a `stat`-then-`create_dir_all` two-step.
#[test]
fn std_file_system_create_dir_exclusive_refuses_second_claim() {
    let dir = temp_dir("std-create-dir-exclusive");
    let claimed = dir.join("claimed");
    let fs = StdFileSystem;

    fs.create_dir_exclusive(&claimed).expect("first claim");
    assert!(fs.stat(&claimed).expect("stat").is_dir);

    let error = fs
        .create_dir_exclusive(&claimed)
        .expect_err("second claim of the same path must fail");
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn fake_file_system_create_dir_exclusive_refuses_second_claim() {
    let fs = FakeFileSystem::new();
    let claimed = PathBuf::from("/servers/java/new-server");

    fs.create_dir_exclusive(&claimed).expect("first claim");
    assert!(fs.stat(&claimed).expect("stat").is_dir);

    let error = fs
        .create_dir_exclusive(&claimed)
        .expect_err("second claim of the same path must fail");
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
}

/// A path that already holds a stored file counts as taken too — not
/// just one previously claimed via `create_dir_exclusive`/`create_dir_all`
/// itself.
#[test]
fn fake_file_system_create_dir_exclusive_refuses_path_already_a_file() {
    let fs = FakeFileSystem::new().with_file("/servers/java/existing", "eula=false", false);

    let error = fs
        .create_dir_exclusive(std::path::Path::new("/servers/java/existing"))
        .expect_err("a stored file already occupies this path");
    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
}
