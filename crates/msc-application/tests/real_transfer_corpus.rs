//! P5.24: runs a real MSC 1-generated `.msctransfer` package through
//! inspection and a merge apply into a temporary owned root, proving at
//! least one server and its manifest-declared config payload arrive.
//!
//! Driven entirely by `MSC2_TRANSFER_PACKAGE_PATH` so
//! `tools/phase5/real-corpus-check.py`'s exercise mode can point this at
//! either the real P5.3 package (P5.25, supplied out-of-band since it
//! carries real world data) or this step's own self-test fixture
//! (`tools/phase5/fixtures/exercise-pass/sample.msctransfer`). If the env
//! var isn't set, this test is a no-op pass -- `cargo nextest run
//! --workspace` must keep working on a clone with no transfer package
//! configured. Both the package and its staging/apply roots live entirely
//! under a fresh temp directory; `package_path` is only ever opened for
//! reading.

use msc_application::transfer::{
    TransferApplyRequest, apply_transfer_import, inspect_transfer_package,
};
use std::path::PathBuf;

const TRANSFER_PACKAGE_ENV: &str = "MSC2_TRANSFER_PACKAGE_PATH";

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "msc2-real-transfer-corpus-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create isolated temp root");
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn real_transfer_package_inspects_and_applies_into_a_temporary_root() {
    let Some(package_path) = std::env::var(TRANSFER_PACKAGE_ENV).ok() else {
        println!("{TRANSFER_PACKAGE_ENV} not set -- skipping (see P5.24/P5.25)");
        return;
    };
    let package_path = PathBuf::from(package_path);
    assert!(
        package_path.is_file(),
        "{}: {TRANSFER_PACKAGE_ENV} does not name an existing file",
        package_path.display()
    );
    // Real packages run 100s of MB (P5.3's is ~600MB) -- compare size and
    // mtime rather than reading the whole file twice, since `package_path`
    // is never opened for writing by anything below (only `fs::File::open`
    // inside `inspect_transfer_package`), so this is a defensive check
    // against a mistake, not a load-bearing proof that needs full content
    // hashing.
    let original_meta = std::fs::metadata(&package_path)
        .unwrap_or_else(|e| panic!("{}: stat transfer package: {e}", package_path.display()));

    let root = TempRoot::new("run");
    let staging_root = root.path.join("staging");
    let servers_root = root.path.join("servers");

    let inspection = inspect_transfer_package(&package_path, &staging_root, &[], &[])
        .unwrap_or_else(|e| panic!("{}: inspect: {e}", package_path.display()));
    assert!(
        !inspection.manifest.servers.is_empty(),
        "{}: manifest declares no servers",
        package_path.display()
    );

    let result = apply_transfer_import(
        &inspection,
        &TransferApplyRequest {
            servers_root: servers_root.clone(),
            ..Default::default()
        },
    );
    assert!(
        result.imported >= 1,
        "{}: apply imported 0 of {} manifest-declared server(s) ({} skipped)",
        package_path.display(),
        inspection.manifest.servers.len(),
        result.skipped
    );

    for server in &result.servers {
        let server_dir = PathBuf::from(&server.server_dir);
        let arrived = std::fs::read_dir(&server_dir)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false);
        assert!(
            arrived,
            "{}: imported server {:?} has an empty destination directory ({}) -- \
             no manifest-declared world/config payload arrived",
            package_path.display(),
            server.display_name,
            server_dir.display()
        );
    }

    let after_meta = std::fs::metadata(&package_path)
        .unwrap_or_else(|e| panic!("{}: recheck transfer package: {e}", package_path.display()));
    assert_eq!(
        after_meta.len(),
        original_meta.len(),
        "{}: transfer package size changed during exercise",
        package_path.display()
    );
    assert_eq!(
        after_meta.modified().ok(),
        original_meta.modified().ok(),
        "{}: transfer package mtime changed during exercise",
        package_path.display()
    );
}
