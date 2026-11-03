//! Essential: server transport must retain hidden/unknown files and empty
//! directories, and must reject unsafe, changed or cancelled source trees.
use msc_infrastructure::source_tree;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        loop {
            let path = std::env::temp_dir().join(format!(
                "msc-source-tree-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("scratch creation failed: {e}"),
            }
        }
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn source_tree_preserves_hidden_unknown_and_empty_entries_for_copy_and_archive() {
    let scratch = Scratch::new();
    let source = scratch.0.join("source");
    fs::create_dir(&source).unwrap();
    fs::create_dir(source.join("empty")).unwrap();
    fs::write(source.join(".hidden"), b"abc").unwrap();
    fs::write(source.join("unknown.config"), b"def").unwrap();
    let long_name = "x".repeat(120);
    fs::write(source.join(&long_name), b"ghi").unwrap();
    let root = source_tree::open(&source, true).unwrap();
    let snapshot = source_tree::snapshot(&root, &source, &|| false).unwrap();
    assert_eq!(snapshot.bytes, 9);
    assert_eq!(snapshot.entries, 4);
    let copy = scratch.0.join("copy");
    fs::create_dir(&copy).unwrap();
    let mut copied = 0;
    source_tree::copy(&root, &source, &copy, &snapshot, &|| false, &mut |block| {
        copied += block.len();
        Ok(())
    })
    .unwrap();
    assert_eq!(copied, 9);
    let tar = scratch.0.join("server.tar");
    let output = fs::File::create(&tar).unwrap();
    source_tree::archive(&root, &source, output, &snapshot, &|| false).unwrap();
    assert_eq!(fs::metadata(&tar).unwrap().len(), snapshot.archive_bytes);
    let extracted = scratch.0.join("extracted");
    fs::create_dir(&extracted).unwrap();
    source_tree::extract_tar(&tar, &extracted, &|| false).unwrap();
    for directory in [&source, &copy, &extracted] {
        assert_eq!(fs::read(directory.join(".hidden")).unwrap(), b"abc");
        assert_eq!(fs::read(directory.join("unknown.config")).unwrap(), b"def");
        assert!(directory.join("empty").is_dir());
        assert_eq!(fs::read(directory.join(&long_name)).unwrap(), b"ghi");
    }
    assert_eq!(
        source_tree::snapshot(&root, &source, &|| false).unwrap(),
        snapshot
    );
}
#[test]
fn source_tree_refuses_changed_cancelled_and_overlapping_inputs() {
    let scratch = Scratch::new();
    let source = scratch.0.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("data"), b"abc").unwrap();
    let root = source_tree::open(&source, true).unwrap();
    let snapshot = source_tree::snapshot(&root, &source, &|| false).unwrap();
    let destination = scratch.0.join("destination");
    fs::create_dir(&destination).unwrap();
    assert!(
        source_tree::copy(
            &root,
            &source,
            &destination,
            &snapshot,
            &|| true,
            &mut |_| Ok(())
        )
        .is_err()
    );
    assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
    fs::write(source.join("data"), b"changed").unwrap();
    assert!(
        source_tree::copy(
            &root,
            &source,
            &destination,
            &snapshot,
            &|| false,
            &mut |_| Ok(())
        )
        .is_err()
    );
    assert_eq!(fs::read(source.join("data")).unwrap(), b"changed");
    let overlap = source.join("inside");
    fs::create_dir(&overlap).unwrap();
    assert!(
        source_tree::copy(&root, &source, &overlap, &snapshot, &|| false, &mut |_| Ok(
            ()
        ))
        .is_err()
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&destination, source.join("link")).unwrap();
        assert!(source_tree::snapshot(&root, &source, &|| false).is_err());
        fs::remove_file(source.join("link")).unwrap();
        let fifo = std::ffi::CString::new(source.join("pipe").to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        assert!(source_tree::snapshot(&root, &source, &|| false).is_err());
    }
}
