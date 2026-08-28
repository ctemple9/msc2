use msc_application::host_reset::{HostResetMode, HostResetWorkflow, recover_files};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use std::path::Path;

#[test]
fn reset_modes_obey_the_preserve_delete_boundary() {
    let fs: &'static dyn FileSystem = Box::leak(Box::new(
        FakeFileSystem::new()
            .with_dir("/agent")
            .with_file("/agent/config.json", b"config".to_vec(), false)
            .with_file(
                "/srv/msc2/servers/paper/world/level.dat",
                b"world".to_vec(),
                false,
            )
            .with_file(
                "/agent/helpers/xbox-broadcast/151/MCXboxBroadcastStandalone.jar",
                b"broadcast".to_vec(),
                false,
            )
            .with_file("/srv/msc2/unrelated/keep", b"keep".to_vec(), false),
    ));
    let workflow = HostResetWorkflow::new(fs, "/agent/config.json", "/srv/msc2/servers")
        .expect("the configured root is narrow enough to reset");
    workflow.begin(HostResetMode::Configuration).unwrap();
    workflow.apply_files(HostResetMode::Configuration).unwrap();
    assert!(fs.stat(Path::new("/agent/config.json")).is_err());
    assert!(
        fs.stat(Path::new("/srv/msc2/servers/paper/world/level.dat"))
            .is_ok()
    );
    assert!(
        fs.stat(Path::new(
            "/agent/helpers/xbox-broadcast/151/MCXboxBroadcastStandalone.jar"
        ))
        .is_ok()
    );
    assert!(fs.stat(Path::new("/srv/msc2/unrelated/keep")).is_ok());
    workflow.finish().unwrap();

    let workflow = HostResetWorkflow::new(fs, "/agent/config.json", "/srv/msc2/servers")
        .and_then(|workflow| workflow.with_helper_cache("/agent/helpers"))
        .expect("the same configured roots remain valid");
    workflow.begin(HostResetMode::Everything).unwrap();
    workflow.apply_files(HostResetMode::Everything).unwrap();
    assert!(
        fs.stat(Path::new("/srv/msc2/servers/paper/world/level.dat"))
            .is_err()
    );
    assert!(
        fs.stat(Path::new(
            "/agent/helpers/xbox-broadcast/151/MCXboxBroadcastStandalone.jar"
        ))
        .is_err()
    );
    assert!(fs.stat(Path::new("/srv/msc2/unrelated/keep")).is_ok());
}

#[test]
fn interrupted_reset_recovery_is_idempotent() {
    let fs: &'static dyn FileSystem = Box::leak(Box::new(
        FakeFileSystem::new()
            .with_dir("/agent")
            .with_file("/agent/config.json", b"config".to_vec(), false)
            .with_file("/srv/msc2/servers/paper/world", b"world".to_vec(), false),
    ));
    let workflow = HostResetWorkflow::new(fs, "/agent/config.json", "/srv/msc2/servers")
        .and_then(|workflow| workflow.with_helper_cache("/agent/helpers"))
        .unwrap();
    workflow.begin(HostResetMode::Everything).unwrap();

    assert_eq!(
        recover_files(fs, "/agent/config.json").unwrap(),
        Some(HostResetMode::Everything)
    );
    assert_eq!(
        recover_files(fs, "/agent/config.json").unwrap(),
        Some(HostResetMode::Everything)
    );
    assert!(fs.stat(Path::new("/agent/config.json")).is_err());
    assert!(fs.stat(Path::new("/srv/msc2/servers/paper/world")).is_err());
    msc_application::host_reset::finish_recovery(fs, "/agent/config.json").unwrap();
    assert_eq!(recover_files(fs, "/agent/config.json").unwrap(), None);
}
