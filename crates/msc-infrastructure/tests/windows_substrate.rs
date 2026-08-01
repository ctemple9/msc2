//! Windows-specific substrate validation (P3.19, per D-017 and §8's named
//! hazards: "path separators and length limits, file-locking semantics —
//! Windows will not let you delete an open file... case-insensitive path
//! comparison"). CI has built and tested this crate on Windows since P3.4,
//! but nothing before this exercised a behavior that's actually
//! Windows-specific — these three tests are the first. `#[cfg(windows)]`
//! at the module level, so the file compiles to nothing and contributes no
//! tests on macOS/Linux, per the same gate pattern the platform crates'
//! `SecretStore` modules already use.

#![cfg(windows)]

use msc_infrastructure::atomic_write::{AtomicWriteError, atomic_write};
use msc_infrastructure::fs::{FakeFileSystem, StdFileSystem};
use msc_infrastructure::path_safety::{PathSafetyError, safe_path};
use std::path::PathBuf;

/// (1) P3.5's path safety against backslash-separated and long
/// (>260-character) inputs. `resolve` never has to touch a real file for
/// components that don't exist (a candidate need not exist yet — see
/// `path_safety`'s own doc comment), so a `FakeFileSystem` root is enough;
/// what's under test is `safe_path`'s own component handling, not the OS
/// filesystem.
#[test]
fn path_safety_backslash_and_long_paths() {
    let fs = FakeFileSystem::new();
    let root = PathBuf::from(r"C:\Users\steve\MSC2Test\ServerRoot");
    let home = PathBuf::from(r"C:\Users\steve");

    // A plain backslash-separated in-root path resolves cleanly.
    let in_root = safe_path(&fs, &root, Some(r"config\server.properties"), &home)
        .expect("backslash-separated in-root path should resolve");
    assert_eq!(
        in_root,
        root.join("config").join("server.properties"),
        "backslash components should split the same way forward-slash ones do"
    );

    // A backslash-based `..` escape attempt is still caught.
    let escape = safe_path(&fs, &root, Some(r"..\..\Windows\System32"), &home);
    assert!(
        matches!(escape, Err(PathSafetyError::Escape { .. })),
        "backslash-based .. escape should still be rejected, got {escape:?}"
    );

    // A path whose full length exceeds the classic 260-character MAX_PATH
    // limit resolves without truncation or panic — extended-length paths
    // are routine for deeply nested modpack/world directory trees.
    let long_segment = "a".repeat(50);
    let deep_relative = [long_segment.as_str(); 8].join(r"\"); // > 260 chars total with root
    let long_path = safe_path(&fs, &root, Some(&deep_relative), &home)
        .unwrap_or_else(|e| panic!("long path should resolve, got {e:?}"));
    assert!(
        long_path.as_os_str().len() > 260,
        "test fixture itself should exceed 260 chars, got {} chars",
        long_path.as_os_str().len()
    );
    assert_eq!(
        long_path,
        root.join(&deep_relative),
        "a long path with no .. or symlinks should resolve to a plain join"
    );
}

/// (2) P3.6's atomic write against a destination another handle already
/// has open. `std::fs::File::open` on Windows requests
/// `FILE_SHARE_READ | FILE_SHARE_WRITE` but not `FILE_SHARE_DELETE`, so a
/// rename that would replace the open file is refused by the OS — assert
/// `atomic_write` surfaces that as a clear `Io` error, and leaves the
/// original destination content in place, rather than hanging or silently
/// succeeding. Needs the real filesystem: this is real Windows lock
/// semantics, not something `FakeFileSystem` can model.
#[test]
fn atomic_write_destination_locked_by_open_handle() {
    let fs = StdFileSystem;
    let dir = std::env::temp_dir().join(format!(
        "msc2-windows-substrate-{}-{}",
        std::process::id(),
        "atomic-write-locked"
    ));
    std::fs::create_dir_all(&dir).expect("create temp test dir");
    let dest = dir.join("locked.txt");
    std::fs::write(&dest, b"original").expect("seed destination file");

    // Hold an open handle on the destination for the whole atomic_write
    // call, without FILE_SHARE_DELETE, so the rename step can't replace it.
    let handle = std::fs::File::open(&dest).expect("open destination to hold a lock");

    let result = atomic_write(&fs, &dest, b"new contents");

    assert!(
        matches!(result, Err(AtomicWriteError::Io(_))),
        "expected a clear Io error while the destination is locked, got {result:?}"
    );
    assert_eq!(
        std::fs::read(&dest).expect("destination should still be readable"),
        b"original",
        "destination content must be untouched when the rename is refused"
    );

    drop(handle);
    let _ = std::fs::remove_dir_all(&dir);
}

/// (3) P3.5 again, with two candidate paths differing only in case.
/// Windows' filesystem is case-insensitive but case-preserving, so a
/// request that differs from the root's own spelling only in case (here,
/// via a `..`-then-back-down request rather than a real symlink — no OS
/// call is needed to provoke this, `safe_path`'s own lexical/string
/// comparison is the thing under test) refers to the exact same directory
/// on disk and must not be classified as an escape.
#[test]
fn path_safety_case_difference_is_not_an_escape() {
    let fs = FakeFileSystem::new();
    let root = PathBuf::from(r"C:\Users\steve\MSC2Test\ServerRoot");
    let home = PathBuf::from(r"C:\Users\steve");

    let result = safe_path(&fs, &root, Some(r"..\SERVERROOT\file.txt"), &home);

    assert!(
        result.is_ok(),
        "a case-differing spelling of the root itself refers to the same real \
         directory on Windows and must not be treated as an escape, got {result:?}"
    );
}
