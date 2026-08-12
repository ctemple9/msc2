//! Port of the apply-side fixtures in `fixtures/transfer-package/`
//! (P5.12/P5.15): the build-stage half of `applyTransferImport(...)`.
//!
//! `live-world-plus-slot-layout.json` and `older-package-no-live-worlds.json`
//! aren't decoded from their JSON here (their `package_server_entry.server`
//! objects are narrative pins, not `ConfigServer::decode`-ready — unlike
//! the export/inspect fixtures' full `input.server` objects) — instead
//! each fixture's staged layout and expectation are reproduced directly as
//! a real temp-directory tree, the same "genuinely disk-shaped" precedent
//! P5.13/P5.14's own tests set.

use msc_application::transfer::{
    TransferApplyRequest, TransferInspection, TransferManifest, TransferServerEntry,
    apply_transfer_import,
};
use msc_domain::app_config_schema::ConfigServer;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("msc2-transfer-apply-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn staging_root(&self) -> PathBuf {
        self.path.join("staging")
    }

    fn servers_root(&self) -> PathBuf {
        self.path.join("servers")
    }

    fn pkg_dir(&self, folder_name: &str) -> PathBuf {
        self.staging_root().join("servers").join(folder_name)
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn minimal_server(
    id: &str,
    display_name: &str,
    server_type: &str,
    java_flavor: &str,
) -> ConfigServer {
    ConfigServer::decode(&serde_json::json!({
        "id": id,
        "display_name": display_name,
        "server_dir": "",
        "paper_jar_path": "",
        "min_ram_gb": 2.0,
        "max_ram_gb": 4.0,
        "server_type": server_type,
        "java_flavor": java_flavor
    }))
    .unwrap()
}

fn manifest_entry(
    server: ConfigServer,
    folder_name: &str,
    bundled_paper_jar: bool,
) -> TransferServerEntry {
    TransferServerEntry {
        server,
        folder_name: folder_name.to_string(),
        java_port: None,
        paper_mc_version: None,
        paper_build: None,
        bundled_paper_jar,
        plugin_links: Vec::new(),
    }
}

fn manifest(servers: Vec<TransferServerEntry>) -> TransferManifest {
    TransferManifest {
        format_version: 2,
        app_config_version: 1,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        source_machine_name: "Test Mac".to_string(),
        servers,
    }
}

fn inspection(staging_root: PathBuf, manifest: TransferManifest) -> TransferInspection {
    TransferInspection {
        staging_root,
        manifest,
        conflicts: Vec::new(),
    }
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = std::fs::File::create(path).unwrap();
    let mut zip = ZipWriter::new(file);
    for (name, bytes) in entries {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn basic_apply_restores_bundled_files_applies_port_override_and_reidentifies_server() {
    let temp = TempRoot::new("basic");
    let pkg_dir = temp.pkg_dir("smp");
    std::fs::create_dir_all(pkg_dir.join("configs")).unwrap();
    std::fs::write(
        pkg_dir.join("configs/server.properties"),
        "level-name=world\nserver-port=25565\n",
    )
    .unwrap();
    std::fs::create_dir_all(pkg_dir.join("plugins")).unwrap();
    std::fs::write(pkg_dir.join("plugins/example.jar"), b"jar-bytes").unwrap();
    std::fs::create_dir_all(pkg_dir.join("world")).unwrap();
    std::fs::write(pkg_dir.join("world/level.dat"), b"level-bytes").unwrap();
    std::fs::write(pkg_dir.join("paper.jar"), b"paper-bytes").unwrap();

    let server = minimal_server("srv-1", "SMP", "java", "paper");
    let source_id = server.id.clone();
    let inspection = inspection(
        temp.staging_root(),
        manifest(vec![manifest_entry(server, "smp", true)]),
    );

    let mut java_port_overrides = HashMap::new();
    java_port_overrides.insert(source_id.clone(), 25999);
    let request = TransferApplyRequest {
        servers_root: temp.servers_root(),
        java_port_overrides,
        bedrock_port_overrides: HashMap::new(),
    };

    let result = apply_transfer_import(&inspection, &request);

    assert_eq!(result.imported, 1);
    assert_eq!(result.skipped, 0);
    let applied = &result.servers[0];
    assert_ne!(
        applied.id, source_id,
        "apply must generate a fresh id, never reuse the source's"
    );

    let dest = PathBuf::from(&applied.server_dir);
    assert_eq!(dest, temp.servers_root().join("java").join("smp"));
    assert!(dest.join("plugins/example.jar").is_file());
    assert!(dest.join("world/level.dat").is_file());
    assert!(dest.join("paper.jar").is_file());
    assert_eq!(
        applied.paper_jar_path,
        dest.join("paper.jar").to_string_lossy()
    );
    assert!(applied.xbox_broadcast_config_path.is_none());

    let props = std::fs::read_to_string(dest.join("server.properties")).unwrap();
    assert!(
        props.contains("server-port=25999"),
        "java port override must be applied: {props}"
    );
    assert!(props.contains("level-name=world"));
}

#[test]
fn folder_name_collision_picks_noncolliding_destination() {
    let temp = TempRoot::new("collision");
    std::fs::create_dir_all(temp.servers_root().join("java").join("smp")).unwrap();
    std::fs::create_dir_all(temp.pkg_dir("smp")).unwrap();

    let server = minimal_server("srv-2", "SMP", "java", "paper");
    let inspection = inspection(
        temp.staging_root(),
        manifest(vec![manifest_entry(server, "smp", false)]),
    );
    let request = TransferApplyRequest {
        servers_root: temp.servers_root(),
        java_port_overrides: HashMap::new(),
        bedrock_port_overrides: HashMap::new(),
    };

    let result = apply_transfer_import(&inspection, &request);

    assert_eq!(result.imported, 1);
    assert_eq!(
        result.servers[0].server_dir,
        temp.servers_root()
            .join("java")
            .join("smp-2")
            .to_string_lossy()
    );
}

/// `fixtures/transfer-package/live-world-plus-slot-layout.json`: live world
/// folders present alongside `world_slots/` — the live folders win and the
/// active-slot fallback is never invoked, even though `active_slot_id.txt`
/// names a different, populated slot.
#[test]
fn live_world_folders_take_precedence_over_slot_marker() {
    let temp = TempRoot::new("live-precedence");
    let pkg_dir = temp.pkg_dir("smp");
    std::fs::create_dir_all(pkg_dir.join("configs")).unwrap();
    std::fs::write(
        pkg_dir.join("configs/server.properties"),
        "level-name=world\n",
    )
    .unwrap();
    std::fs::write(pkg_dir.join("paper.jar"), b"jar").unwrap();
    for dir in ["world", "world_nether", "world_the_end"] {
        std::fs::create_dir_all(pkg_dir.join(dir)).unwrap();
        std::fs::write(pkg_dir.join(dir).join("level.dat"), b"live").unwrap();
    }
    std::fs::create_dir_all(pkg_dir.join("world_slots/slot-b")).unwrap();
    write_zip(
        &pkg_dir.join("world_slots/slot-a/world.zip"),
        &[("world/SHOULD_NOT_APPEAR.txt", b"slot data")],
    );
    std::fs::write(pkg_dir.join("world_slots/active_slot_id.txt"), "slot-a\n").unwrap();

    let server = minimal_server("srv-old-smp", "SMP", "java", "paper");
    let inspection = inspection(
        temp.staging_root(),
        manifest(vec![manifest_entry(server, "smp", true)]),
    );
    let request = TransferApplyRequest {
        servers_root: temp.servers_root(),
        java_port_overrides: HashMap::new(),
        bedrock_port_overrides: HashMap::new(),
    };

    let result = apply_transfer_import(&inspection, &request);

    assert_eq!(result.imported, 1);
    let dest = PathBuf::from(&result.servers[0].server_dir);
    for dir in ["world", "world_nether", "world_the_end"] {
        assert!(
            dest.join(dir).join("level.dat").is_file(),
            "live world folder {dir} must be restored"
        );
    }
    assert!(
        !dest.join("world/SHOULD_NOT_APPEAR.txt").exists(),
        "slot-a's archive must never be extracted when live world folders exist"
    );
}

/// `fixtures/transfer-package/older-package-no-live-worlds.json`: no live
/// world folders in the package — falls back to materializing the slot
/// named by `active_slot_id.txt` (slot-b here, not slot-a).
#[test]
fn slot_fallback_materializes_active_world_when_no_live_folders() {
    let temp = TempRoot::new("slot-fallback");
    let pkg_dir = temp.pkg_dir("smp-legacy");
    std::fs::create_dir_all(pkg_dir.join("configs")).unwrap();
    std::fs::write(
        pkg_dir.join("configs/server.properties"),
        "level-name=world\n",
    )
    .unwrap();
    std::fs::write(pkg_dir.join("paper.jar"), b"jar").unwrap();
    write_zip(
        &pkg_dir.join("world_slots/slot-a/world.zip"),
        &[("world/WRONG.txt", b"not the active slot")],
    );
    write_zip(
        &pkg_dir.join("world_slots/slot-b/world.zip"),
        &[("world/level.dat", b"active slot data")],
    );
    std::fs::write(pkg_dir.join("world_slots/active_slot_id.txt"), "slot-b\n").unwrap();

    let server = minimal_server("srv-legacy-smp", "Legacy SMP", "java", "paper");
    let inspection = inspection(
        temp.staging_root(),
        manifest(vec![manifest_entry(server, "smp-legacy", true)]),
    );
    let request = TransferApplyRequest {
        servers_root: temp.servers_root(),
        java_port_overrides: HashMap::new(),
        bedrock_port_overrides: HashMap::new(),
    };

    let result = apply_transfer_import(&inspection, &request);

    assert_eq!(result.imported, 1);
    let dest = PathBuf::from(&result.servers[0].server_dir);
    assert!(!dest.join("world_nether").exists());
    assert!(!dest.join("world_the_end").exists());
    let restored = std::fs::read_to_string(dest.join("world/level.dat")).unwrap();
    assert_eq!(restored, "active slot data");
    assert!(
        !dest.join("world/WRONG.txt").exists(),
        "must materialize the *active* slot (slot-b), not slot-a"
    );
}

#[test]
fn bedrock_live_world_restored_and_bedrock_port_override_applied() {
    let temp = TempRoot::new("bedrock");
    let pkg_dir = temp.pkg_dir("realm");
    std::fs::create_dir_all(pkg_dir.join("worlds")).unwrap();
    std::fs::write(pkg_dir.join("worlds/level.dat"), b"bedrock-world").unwrap();

    let server = minimal_server("srv-bedrock-1", "Realm", "bedrock", "paper");
    let source_id = server.id.clone();
    let inspection = inspection(
        temp.staging_root(),
        manifest(vec![manifest_entry(server, "realm", false)]),
    );

    let mut bedrock_port_overrides = HashMap::new();
    bedrock_port_overrides.insert(source_id, 19200);
    let request = TransferApplyRequest {
        servers_root: temp.servers_root(),
        java_port_overrides: HashMap::new(),
        bedrock_port_overrides,
    };

    let result = apply_transfer_import(&inspection, &request);

    assert_eq!(result.imported, 1);
    let applied = &result.servers[0];
    assert_eq!(applied.bedrock_port, Some(19200));
    let dest = PathBuf::from(&applied.server_dir);
    assert!(dest.join("worlds/level.dat").is_file());
    assert_eq!(
        dest.parent().unwrap().file_name().unwrap().to_str(),
        Some("bedrock")
    );
}

/// `libraries/` bundling is a flavor gate, not an existence check (format
/// doc, `forge-libraries-bundled.json`) — proved here on the apply side
/// too (source line 431), not just at export.
#[test]
fn libraries_restored_only_for_forge_flavors() {
    let temp = TempRoot::new("libraries");
    std::fs::create_dir_all(temp.pkg_dir("modded").join("libraries/net/example")).unwrap();
    std::fs::write(
        temp.pkg_dir("modded").join("libraries/net/example/lib.jar"),
        b"lib",
    )
    .unwrap();
    std::fs::create_dir_all(temp.pkg_dir("vanilla").join("libraries/stray")).unwrap();
    std::fs::write(temp.pkg_dir("vanilla").join("libraries/stray/x.jar"), b"x").unwrap();

    let neoforge_server = minimal_server("srv-forge-1", "Modded", "java", "neoforge");
    let paper_server = minimal_server("srv-paper-1", "Vanilla", "java", "paper");
    let inspection = inspection(
        temp.staging_root(),
        manifest(vec![
            manifest_entry(neoforge_server, "modded", false),
            manifest_entry(paper_server, "vanilla", false),
        ]),
    );
    let request = TransferApplyRequest {
        servers_root: temp.servers_root(),
        java_port_overrides: HashMap::new(),
        bedrock_port_overrides: HashMap::new(),
    };

    let result = apply_transfer_import(&inspection, &request);

    assert_eq!(result.imported, 2);
    let modded_dest = PathBuf::from(&result.servers[0].server_dir);
    let vanilla_dest = PathBuf::from(&result.servers[1].server_dir);
    assert!(modded_dest.join("libraries/net/example/lib.jar").is_file());
    assert!(
        !vanilla_dest.join("libraries").exists(),
        "a Paper server's libraries/ must never be restored, even if present in the package"
    );
}

/// Source's per-entry loop only hard-fails (skip + remove the partial
/// destination) on a wholesale-subdirectory copy failure — every other
/// per-file/per-subdir copy in the loop is best-effort. A permission-denied
/// source directory is a real-world way to trigger that hard failure.
#[cfg(unix)]
#[test]
fn wholesale_subdir_copy_failure_skips_and_cleans_up_destination() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempRoot::new("wholesale-failure");
    let pkg_dir = temp.pkg_dir("smp");
    let unreadable = pkg_dir.join("plugins/locked");
    std::fs::create_dir_all(&unreadable).unwrap();
    std::fs::write(unreadable.join("a.jar"), b"x").unwrap();
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();

    let server = minimal_server("srv-3", "SMP", "java", "paper");
    let inspection = inspection(
        temp.staging_root(),
        manifest(vec![manifest_entry(server, "smp", false)]),
    );
    let request = TransferApplyRequest {
        servers_root: temp.servers_root(),
        java_port_overrides: HashMap::new(),
        bedrock_port_overrides: HashMap::new(),
    };

    let result = apply_transfer_import(&inspection, &request);

    // Restore permissions before any cleanup (ours or TempRoot::drop's)
    // tries to remove the now-unreadable directory.
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(result.imported, 0);
    assert_eq!(result.skipped, 1);
    assert!(
        !temp.servers_root().join("java").join("smp").exists(),
        "a hard failure must remove the partial destination, not leave it behind"
    );
}
