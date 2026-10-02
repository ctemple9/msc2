//! Essential deletion-boundary checks. All inputs are in-memory, with no
//! services, real directories, network calls, sleeps, or destructive execution.
use msc_infrastructure::fs::FakeFileSystem;
use msc_infrastructure::service::{ServiceInstallRequest, ServiceStatusReport};
use msc_infrastructure::uninstall::{
    self, EntryKind, EntryState, InstallerCandidate, Overrides, Platform, Request,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn request() -> Request {
    Request {
        computer: "fixture-host".into(),
        platform: Platform::Linux,
        home: PathBuf::from("/home/player"),
        overrides: Overrides::default(),
        services: vec![ServiceStatusReport::not_installed("com.ctemple.msc2.agent")],
        protected_paths: vec![PathBuf::from("/work/msc2")],
        client_paths: vec![],
        headless_roots: vec![],
        installer_candidates: vec![],
        findings: vec![],
    }
}

#[test]
fn rejects_shared_roots_traversal_source_trees_and_symlink_targets() {
    let fs = FakeFileSystem::new()
        .with_symlink("/home/player/.local/share/msc2", "/home/player/Documents")
        .with_dir("/work/other/.git");
    let req = request();
    for path in [
        "/",
        "/home/player",
        "/home/player/Documents",
        "/home/player/Downloads",
        "/home/player/.local/share",
        "/home/player/a/../Documents",
        "/work/msc2",
        "/work/msc2/crates",
        "/home/player/.local/share/msc2",
        "/work/other",
        "relative/server",
    ] {
        assert!(
            uninstall::validate_target(&fs, &req, Path::new(path)).is_err(),
            "accepted {path}"
        );
    }
    assert!(uninstall::validate_target(&fs, &req, Path::new("/home/player/servers/paper")).is_ok());
}

#[test]
fn corrupt_config_blocks_its_parent_data_deletion() {
    let fs = FakeFileSystem::new()
        .with_dir("/home/player/.local/share/msc2")
        .with_file(
            "/home/player/.local/share/msc2/server_config_swift.json",
            b"{broken".to_vec(),
            false,
        );
    let result = uninstall::inventory(&fs, &request());
    assert!(result.is_blocked());
    assert!(
        result
            .entries
            .iter()
            .any(|entry| entry.kind == EntryKind::DataTree
                && entry.path.as_deref() == Some(Path::new("/home/player/.local/share/msc2"))
                && entry.state == EntryState::Blocked)
    );
}

#[test]
fn custom_missing_config_does_not_authorize_unrelated_directory_deletion() {
    let fs = FakeFileSystem::new().with_dir("/home/player/other");
    let mut req = request();
    req.overrides.data_dir = Some(PathBuf::from("/home/player/other"));
    let result = uninstall::inventory(&fs, &req);
    assert!(result.entries.iter().any(|entry| entry.path.as_deref()
        == Some(Path::new("/home/player/other"))
        && entry.state == EntryState::Blocked));
}

#[test]
fn uses_local_service_overrides_without_disclosing_other_environment_values() {
    let fs = FakeFileSystem::new();
    let mut req = request();
    let definition = ServiceInstallRequest::new(
        "com.ctemple.msc2.agent",
        "/opt/msc2/msc",
        "/opt/msc2/data",
        "/opt/msc2/logs/agent.log",
        48001,
    )
    .env("MSC2_DATA_DIR", "/srv/msc2/data")
    .env("MSC2_APP_CONFIG_PATH", "/srv/msc2/config.json")
    .env("MSC2_AGENT_SERVERS_ROOT", "/srv/msc2/servers")
    .env("PASSWORD", "private-fixture-value");
    req.services = vec![ServiceStatusReport::stopped(definition)];
    let result = uninstall::inventory(&fs, &req);
    let encoded = serde_json::to_string(&result).unwrap();
    assert!(encoded.contains("/srv/msc2/data"));
    assert!(encoded.contains("/srv/msc2/config.json"));
    assert!(!encoded.contains("private-fixture-value"));
    let overrides = Overrides::from_environment(&BTreeMap::from([
        ("MSC2_DATA_DIR".into(), "/srv/msc2/data".into()),
        ("SECRET".into(), "private-fixture-value".into()),
    ]));
    assert_eq!(
        overrides.data_dir.as_deref(),
        Some(Path::new("/srv/msc2/data"))
    );
}

#[test]
fn installer_filename_is_not_ownership_and_changed_checksum_is_blocked() {
    let fs = FakeFileSystem::new().with_file(
        "/home/player/Downloads/MSC_2.dmg",
        b"unrelated-file".to_vec(),
        false,
    );
    for checksum in [None, Some("0".repeat(64))] {
        let mut req = request();
        req.installer_candidates.push(InstallerCandidate {
            path: PathBuf::from("/home/player/Downloads/MSC_2.dmg"),
            verified_sha256: checksum,
        });
        let result = uninstall::inventory(&fs, &req);
        assert!(
            result
                .entries
                .iter()
                .any(|entry| entry.kind == EntryKind::Installer
                    && entry.state == EntryState::Blocked)
        );
    }
}

#[test]
fn preview_has_no_remote_targets_and_does_not_modify_input_files() {
    let fs = FakeFileSystem::new().with_file(
        "/home/player/.local/share/msc2/keep",
        b"data".to_vec(),
        false,
    );
    let result = uninstall::inventory(&fs, &request());
    assert!(
        result
            .entries
            .iter()
            .filter_map(|entry| entry.path.as_ref())
            .all(|path| path.is_absolute())
    );
    assert_eq!(
        msc_infrastructure::fs::FileSystem::read(
            &fs,
            Path::new("/home/player/.local/share/msc2/keep")
        )
        .unwrap(),
        b"data"
    );
    assert!(!result.fingerprint.is_empty());
}
