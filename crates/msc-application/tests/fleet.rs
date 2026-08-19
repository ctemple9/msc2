//! P7.20: `msc_application::fleet::{delete_server, rename_server,
//! read_eula, accept_eula}` against `deleteServerProvider`/
//! `deleteServerFromDisk`/`deleteServer(withId:)`
//! (`AppViewModel+APIWiringServerMgmt.swift:43-67`,
//! `AppViewModel+ConfigHelpers.swift:66-106`), `renameServerProvider`
//! (`AppViewModel+APIWiringServerMgmt.swift:19-41`), and `EULAManager`
//! (`EULAManager.swift`). No dedicated `fixtures/` directory exists for
//! this behavior — read directly from source with file:line citations,
//! same practice this phase already established for P7.9/P7.16/P7.19.

use msc_application::fleet::{
    self, AcceptEulaError, DeleteServerError, EulaState, RenameServerError,
};
use msc_domain::app_config_schema::{AppConfig, ConfigServer};
use msc_domain::identity::ServerType;
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use std::path::Path;

fn config_with_server(id: &str, name: &str, server_dir: &str) -> AppConfig {
    let mut config = AppConfig::default_config("/home/msc/servers");
    let server = ConfigServer::new(id, name, server_dir, "paper.jar", 2.0, 4.0);
    config.servers.push(server);
    config.active_server_id = Some(id.to_string());
    config
}

// --- delete ---

#[test]
fn delete_server_refuses_empty_id() {
    let mut config = config_with_server("s1", "Box", "/servers/java/box");
    let fs = FakeFileSystem::new().with_file("/servers/java/box/paper.jar", b"x".to_vec(), false);
    let err = fleet::delete_server(&fs, &mut config, "   ", false).expect_err("empty id refused");
    assert!(matches!(err, DeleteServerError::EmptyServerId));
}

#[test]
fn delete_server_refuses_unknown_id() {
    let mut config = config_with_server("s1", "Box", "/servers/java/box");
    let fs = FakeFileSystem::new();
    let err = fleet::delete_server(&fs, &mut config, "nope", false).expect_err("not found");
    assert!(matches!(err, DeleteServerError::ServerNotFound));
}

#[test]
fn delete_server_refuses_while_active_and_running() {
    let mut config = config_with_server("s1", "Box", "/servers/java/box");
    let fs = FakeFileSystem::new().with_file("/servers/java/box/paper.jar", b"x".to_vec(), false);
    let err =
        fleet::delete_server(&fs, &mut config, "s1", true).expect_err("running server refused");
    assert!(matches!(err, DeleteServerError::ServerRunning));
    // Refused before any disk mutation.
    assert!(fs.read(Path::new("/servers/java/box/paper.jar")).is_ok());
    assert_eq!(config.servers.len(), 1);
}

#[test]
fn delete_server_removes_directory_and_deregisters_reselecting_active() {
    let mut config = config_with_server("s1", "Box", "/servers/java/box");
    config.servers.push(ConfigServer::new(
        "s2",
        "Second",
        "/servers/java/second",
        "paper.jar",
        2.0,
        4.0,
    ));
    // `s1` is active; deleting it must fall back to the fleet's own
    // remaining array order (`servers.first?.id`), not `s2` by name.
    let fs = FakeFileSystem::new()
        .with_file("/servers/java/box/paper.jar", b"x".to_vec(), false)
        .with_file("/servers/java/second/paper.jar", b"y".to_vec(), false);

    let deleted = fleet::delete_server(&fs, &mut config, "s1", false).expect("delete succeeds");
    assert_eq!(deleted.removed_display_name, "Box");
    assert_eq!(deleted.new_active_server_id.as_deref(), Some("s2"));
    assert_eq!(config.active_server_id.as_deref(), Some("s2"));
    assert_eq!(config.servers.len(), 1);
    assert!(fs.read(Path::new("/servers/java/box/paper.jar")).is_err());
    // The untouched server's own directory survives.
    assert!(fs.read(Path::new("/servers/java/second/paper.jar")).is_ok());
}

#[test]
fn delete_server_tolerates_already_missing_folder() {
    let mut config = config_with_server("s1", "Box", "/servers/java/box");
    // No files seeded under /servers/java/box at all -- `fs.stat` fails,
    // and source tolerates this (logs, doesn't throw) rather than
    // treating it as `delete_failed`.
    let fs = FakeFileSystem::new();

    let deleted = fleet::delete_server(&fs, &mut config, "s1", false)
        .expect("missing folder tolerated, not an error");
    assert_eq!(deleted.new_active_server_id, None);
    assert!(config.servers.is_empty());
}

#[test]
fn delete_server_keeps_active_id_when_a_different_server_is_deleted() {
    let mut config = config_with_server("s1", "Box", "/servers/java/box");
    config.servers.push(ConfigServer::new(
        "s2",
        "Second",
        "/servers/java/second",
        "paper.jar",
        2.0,
        4.0,
    ));
    config.active_server_id = Some("s1".to_string());
    let fs =
        FakeFileSystem::new().with_file("/servers/java/second/paper.jar", b"y".to_vec(), false);

    fleet::delete_server(&fs, &mut config, "s2", false).expect("delete s2");
    assert_eq!(config.active_server_id.as_deref(), Some("s1"));
    assert_eq!(config.servers.len(), 1);
}

// --- rename ---

#[test]
fn rename_server_only_touches_display_name() {
    let mut config = config_with_server("s1", "Box", "/servers/java/box");
    fleet::rename_server(&mut config, "s1", "  Renamed Box  ").expect("rename succeeds");
    assert_eq!(config.servers[0].display_name, "Renamed Box");
    assert_eq!(config.servers[0].server_dir, "/servers/java/box");
}

#[test]
fn rename_server_refuses_empty_id_and_empty_name() {
    let mut config = config_with_server("s1", "Box", "/servers/java/box");
    assert!(matches!(
        fleet::rename_server(&mut config, "", "New Name").unwrap_err(),
        RenameServerError::EmptyServerId
    ));
    assert!(matches!(
        fleet::rename_server(&mut config, "s1", "   ").unwrap_err(),
        RenameServerError::EmptyName
    ));
}

#[test]
fn rename_server_refuses_unknown_id() {
    let mut config = config_with_server("s1", "Box", "/servers/java/box");
    assert!(matches!(
        fleet::rename_server(&mut config, "nope", "New Name").unwrap_err(),
        RenameServerError::ServerNotFound
    ));
}

#[test]
fn rename_server_allows_duplicate_display_names() {
    // Source has no collision check at all -- two servers may share a
    // display name.
    let mut config = config_with_server("s1", "Box", "/servers/java/box");
    config.servers.push(ConfigServer::new(
        "s2",
        "Second",
        "/servers/java/second",
        "paper.jar",
        2.0,
        4.0,
    ));
    fleet::rename_server(&mut config, "s2", "Box").expect("rename succeeds");
    assert_eq!(config.servers[0].display_name, "Box");
    assert_eq!(config.servers[1].display_name, "Box");
}

// --- EULAManager ---

#[test]
fn read_eula_missing_file_is_missing_state() {
    let fs = FakeFileSystem::new();
    assert_eq!(
        fleet::read_eula(&fs, Path::new("/servers/java/box")),
        EulaState::Missing
    );
}

#[test]
fn read_eula_true_and_false_and_malformed_value() {
    let fs = FakeFileSystem::new()
        .with_file("/servers/java/a/eula.txt", b"eula=true\n".to_vec(), false)
        .with_file("/servers/java/b/eula.txt", b"eula=false\n".to_vec(), false)
        // Neither "true" nor "false" -- still reads as ExplicitlyFalse,
        // since source's own test is `.contains("true")`, not a strict
        // boolean parse (`EULAManager.swift:26-28`).
        .with_file("/servers/java/c/eula.txt", b"eula=maybe\n".to_vec(), false)
        // No `eula=` line at all -- the real "neither true nor false"
        // case this step's plan text names, reads as Missing just like
        // an absent file.
        .with_file(
            "/servers/java/d/eula.txt",
            b"# just a comment\n".to_vec(),
            false,
        );

    assert_eq!(
        fleet::read_eula(&fs, Path::new("/servers/java/a")),
        EulaState::Accepted
    );
    assert_eq!(
        fleet::read_eula(&fs, Path::new("/servers/java/b")),
        EulaState::ExplicitlyFalse
    );
    assert_eq!(
        fleet::read_eula(&fs, Path::new("/servers/java/c")),
        EulaState::ExplicitlyFalse
    );
    assert_eq!(
        fleet::read_eula(&fs, Path::new("/servers/java/d")),
        EulaState::Missing
    );
}

#[test]
fn read_eula_is_case_insensitive_and_trims_whitespace() {
    let fs = FakeFileSystem::new().with_file(
        "/servers/java/box/eula.txt",
        b"  EULA=TRUE  \n".to_vec(),
        false,
    );
    assert_eq!(
        fleet::read_eula(&fs, Path::new("/servers/java/box")),
        EulaState::Accepted
    );
}

#[test]
fn accept_eula_writes_the_exact_commented_format() {
    let mut config = config_with_server("s1", "Box", "/servers/java/box");
    config.servers[0].server_type = ServerType::Java;
    let fs = FakeFileSystem::new();
    fleet::accept_eula(&fs, &config, "s1").expect("accept succeeds");
    let bytes = fs.read(Path::new("/servers/java/box/eula.txt")).unwrap();
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "# EULA accepted via MinecraftServerController\neula=true\n\n"
    );
    assert_eq!(
        fleet::read_eula(&fs, Path::new("/servers/java/box")),
        EulaState::Accepted
    );
}

#[test]
fn accept_eula_refuses_bedrock_server() {
    let mut config = config_with_server("s1", "Box", "/servers/bedrock/box");
    config.servers[0].server_type = ServerType::Bedrock;
    let fs = FakeFileSystem::new();
    let err = fleet::accept_eula(&fs, &config, "s1").expect_err("bedrock refused");
    assert!(matches!(err, AcceptEulaError::UnsupportedServerType));
}

#[test]
fn accept_eula_refuses_unknown_server() {
    let config = config_with_server("s1", "Box", "/servers/java/box");
    let fs = FakeFileSystem::new();
    let err = fleet::accept_eula(&fs, &config, "nope").expect_err("not found");
    assert!(matches!(err, AcceptEulaError::ServerNotFound));
}
