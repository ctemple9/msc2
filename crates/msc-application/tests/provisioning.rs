//! Port of `fixtures/server-creation/`'s 24 cases (P7.6), exercising
//! `msc_application::provisioning::create_download_and_go_server` (P7.17)
//! against the four download-and-go families it provisions (Vanilla,
//! Paper, Purpur, Fabric). Real on-disk server directories via
//! `StdFileSystem`, the same precedent every other archive-touching test
//! file in this phase already set; network calls go through a local
//! [`FakeTransport`], per this phase's own "provisioning tests never
//! touch the network" rule.

use msc_application::provisioning::{
    self, CreateServerError, NewServerRequest, WorldSource, real_copy_existing_world_folder,
    real_unzip_world_backup,
};
use msc_domain::identity::JavaServerFlavor;
use msc_domain::world_profile::WorldProfile;
use msc_infrastructure::download_staging::sha256_hex;
use msc_infrastructure::fs::{FileSystem, StdFileSystem};
use msc_infrastructure::geyser::{GeyserProject, latest_build_url};
use msc_infrastructure::jar_provider::{JarProviderError, Transport};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Barrier, Mutex};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "msc2-provisioning-test-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct FakeTransport {
    responses: Mutex<HashMap<String, Vec<u8>>>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
        }
    }

    fn with(self, url: &str, bytes: impl Into<Vec<u8>>) -> Self {
        self.responses
            .lock()
            .unwrap()
            .insert(url.to_string(), bytes.into());
        self
    }
}

impl Transport for FakeTransport {
    fn get(&self, url: &str, what: &str, _max_bytes: u64) -> Result<Vec<u8>, JarProviderError> {
        self.responses
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or_else(|| {
                JarProviderError::Network(format!("{what}: no fake response registered for {url}"))
            })
    }
}

fn vanilla_transport() -> FakeTransport {
    FakeTransport::new()
        .with(
            "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json",
            br#"{"latest":{"release":"1.21.4","snapshot":"1.21.4"},"versions":[{"id":"1.21.4","type":"release","url":"https://meta/1.21.4.json"}]}"#.to_vec(),
        )
        .with(
            "https://meta/1.21.4.json",
            br#"{"downloads":{"server":{"url":"https://dl/vanilla-1.21.4.jar","sha1":"37eca23153ab9d806451264be85d0d931a08c35d"}}}"#.to_vec(),
        )
        .with("https://dl/vanilla-1.21.4.jar", b"FAKE-VANILLA-JAR".to_vec())
}

fn paper_transport() -> FakeTransport {
    FakeTransport::new()
        .with(
            "https://fill.papermc.io/v3/projects/paper",
            br#"{"versions":{"1.21":["1.21.4"]}}"#.to_vec(),
        )
        .with(
            "https://fill.papermc.io/v3/projects/paper/versions/1.21.4/builds",
            br#"[{"id":231,"channel":"STABLE","downloads":{"server:default":{"url":"https://dl/paper-1.21.4-231.jar","checksums":{"sha256":"b90451bf06476ab0348852d0af747a6962e6b648e9a20ba261501e12e0d7b321"}}}}]"#.to_vec(),
        )
        .with("https://dl/paper-1.21.4-231.jar", b"FAKE-PAPER-JAR".to_vec())
}

fn cross_play_paper_transport() -> FakeTransport {
    let geyser_bytes = b"FAKE-GEYSER-JAR";
    let floodgate_bytes = b"FAKE-FLOODGATE-JAR";
    let geyser_version = "2.11.2";
    let geyser_build = 42;
    let floodgate_version = "2.2.5";
    let floodgate_build = 9;
    paper_transport()
        .with(
            &latest_build_url(GeyserProject::Geyser),
            serde_json::to_vec(&serde_json::json!({
                "version": geyser_version,
                "build": geyser_build,
                "downloads": { "spigot": { "sha256": sha256_hex(geyser_bytes) } }
            }))
            .unwrap(),
        )
        .with(
            &format!(
                "https://download.geysermc.org/v2/projects/geyser/versions/{geyser_version}/builds/{geyser_build}/downloads/spigot"
            ),
            geyser_bytes.to_vec(),
        )
        .with(
            &latest_build_url(GeyserProject::Floodgate),
            serde_json::to_vec(&serde_json::json!({
                "version": floodgate_version,
                "build": floodgate_build,
                "downloads": { "spigot": { "sha256": sha256_hex(floodgate_bytes) } }
            }))
            .unwrap(),
        )
        .with(
            &format!(
                "https://download.geysermc.org/v2/projects/floodgate/versions/{floodgate_version}/builds/{floodgate_build}/downloads/spigot"
            ),
            floodgate_bytes.to_vec(),
        )
}

fn purpur_transport() -> FakeTransport {
    FakeTransport::new()
        .with(
            "https://api.purpurmc.org/v2/purpur",
            br#"{"versions":["1.20.1","1.21.4"]}"#.to_vec(),
        )
        .with(
            "https://api.purpurmc.org/v2/purpur/1.21.4",
            br#"{"builds":{"latest":"2"}}"#.to_vec(),
        )
        // P7.35: purpur_download_version now fetches this per-build
        // metadata hop for its published md5 before downloading.
        .with(
            "https://api.purpurmc.org/v2/purpur/1.21.4/latest",
            br#"{"md5":"1ad2b5ba90cdb6b1e82ff4a6f0bfaf4a"}"#.to_vec(),
        )
        .with(
            "https://api.purpurmc.org/v2/purpur/1.21.4/latest/download",
            b"FAKE-PURPUR-JAR".to_vec(),
        )
}

fn fabric_transport() -> FakeTransport {
    FakeTransport::new()
        .with(
            "https://meta.fabricmc.net/v2/versions/game",
            br#"[{"version":"1.21.4","stable":true}]"#.to_vec(),
        )
        .with(
            "https://meta.fabricmc.net/v2/versions/loader/1.21.4",
            br#"[{"loader":{"version":"0.16.9","stable":true}}]"#.to_vec(),
        )
        .with(
            "https://meta.fabricmc.net/v2/versions/installer",
            br#"[{"version":"1.0.1","stable":true}]"#.to_vec(),
        )
        .with(
            "https://meta.fabricmc.net/v2/versions/loader/1.21.4/0.16.9/1.0.1/server/jar",
            b"FAKE-FABRIC-JAR".to_vec(),
        )
}

fn base_request<'a>(
    flavor: JavaServerFlavor,
    world_source: WorldSource<'a>,
) -> NewServerRequest<'a> {
    NewServerRequest {
        name: "Test Server",
        initial_world_name: None,
        flavor,
        port: 25565,
        enable_cross_play: false,
        cross_play_bedrock_port: None,
        enable_playit: false,
        enable_xbox_broadcast: false,
        difficulty: "normal",
        gamemode: "survival",
        world_seed: None,
        initial_world_profile: None,
        world_source,
        save_downloaded_jars: false,
        default_banner_color_hex: "#3366FF",
    }
}

fn always_ok2(_a: &Path, _b: &Path) -> bool {
    true
}

fn always_ok3(_a: &Path, _b: &Path, _c: &str) -> bool {
    true
}

fn always_fail2(_a: &Path, _b: &Path) -> bool {
    false
}

fn always_fail3(_a: &Path, _b: &Path, _c: &str) -> bool {
    false
}

// ---- Minimal big-endian NBT byte-building helpers, for the imported-
// metadata fixture only. Mirrors `msc-domain/tests/world_nbt.rs`'s own
// small local helpers rather than exposing this crate's private NBT
// writer across a crate boundary just for one test.

fn be_nbt_string(s: &str) -> Vec<u8> {
    let mut out = (s.len() as i16).to_be_bytes().to_vec();
    out.extend_from_slice(s.as_bytes());
    out
}

fn be_entry(name: &str, tag: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend(be_nbt_string(name));
    out.extend_from_slice(payload);
    out
}

fn be_int_entry(name: &str, value: i32) -> Vec<u8> {
    be_entry(name, 3, &value.to_be_bytes())
}

fn be_compound_payload(entries: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for e in entries {
        out.extend_from_slice(e);
    }
    out.push(0);
    out
}

fn be_java_root(data_entries: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![10u8];
    out.extend(be_nbt_string(""));
    out.extend(be_compound_payload(&[be_entry(
        "Data",
        10,
        &be_compound_payload(data_entries),
    )]));
    out
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

fn write_backup_zip_with_level_dat(zip_path: &Path, level_dat: &[u8]) {
    fs::create_dir_all(zip_path.parent().unwrap()).unwrap();
    let file = fs::File::create(zip_path).unwrap();
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    zip.start_file("myworld/level.dat", opts).unwrap();
    zip.write_all(level_dat).unwrap();
    zip.finish().unwrap();
}

fn read_properties(server_dir: &Path) -> HashMap<String, String> {
    let text = fs::read_to_string(server_dir.join("server.properties")).unwrap();
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            line.split_once('=')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

// ---------------------------------------------------------------------
// fixtures/server-creation/name-trimmed-before-use.json
// fixtures/server-creation/folder-name-lowercased-and-spaces-to-underscores.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_name_trimmed_and_folder_derived() {
    let tmp = TempDir::new("name-trim");
    let transport = vanilla_transport();
    let mut request = base_request(JavaServerFlavor::Vanilla, WorldSource::Fresh);
    request.name = "  My Cool SMP  ";

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .expect("create should succeed");

    assert_eq!(created.config.display_name, "My Cool SMP");
    // Forward slash, not `std::path::MAIN_SEPARATOR`: `server_dir` is a
    // config-stored path, and this codebase's convention for those is
    // always `/` regardless of host OS (`msc_domain::app_config_schema
    // ::join_path`, `msc_infrastructure::fs::join_forward_slash`) -- not
    // the native separator `tmp.path()`'s own real, native-Windows temp
    // directory happens to use. Found needing this by P7.29's Windows CI
    // leg: this test's real `StdFileSystem`/`TempDir` combination had
    // never run on Windows before, so the MAIN_SEPARATOR assumption here
    // had never been checked against it.
    assert!(
        created.config.server_dir.ends_with("java/my_cool_smp"),
        "server_dir was {}",
        created.config.server_dir
    );
}

// ---------------------------------------------------------------------
// fixtures/server-creation/empty-name-after-trim-refused-no-directory-created.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_empty_name_refused_no_directory_created() {
    let tmp = TempDir::new("empty-name");
    let transport = vanilla_transport();
    let mut request = base_request(JavaServerFlavor::Vanilla, WorldSource::Fresh);
    request.name = "   ";

    let err = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap_err();

    assert!(matches!(err, CreateServerError::EmptyName));
    assert!(!tmp.path().join("java").exists());
}

// ---------------------------------------------------------------------
// fixtures/server-creation/pre-existing-folder-refused-with-message.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_pre_existing_folder_refused_with_message() {
    let tmp = TempDir::new("pre-existing");
    let existing_dir = tmp.path().join("java").join("existing");
    fs::create_dir_all(&existing_dir).unwrap();

    let transport = vanilla_transport();
    let mut request = base_request(JavaServerFlavor::Vanilla, WorldSource::Fresh);
    request.name = "Existing";

    let err = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap_err();

    let message = err.to_string();
    assert!(
        message.starts_with("A server folder named \"existing\" already exists at"),
        "message was: {message}"
    );
    assert!(message.contains("Choose a different name, or remove that folder."));
}

// ---------------------------------------------------------------------
// P7.33: closing the check-then-create race
// `docs/msc2/families/phase7-scope.md`'s P7.1 note flagged (and P7.30's
// gate audit confirmed was still open) -- two concurrent creates of the
// same server name must never both "win" the old `fs.stat`-then-
// `fs.create_dir_all` two-step.
// ---------------------------------------------------------------------

#[test]
fn provisioning_concurrent_creates_of_same_name_never_both_succeed() {
    for attempt in 0..16 {
        let tmp = TempDir::new(&format!("race-{attempt}"));
        let transport = vanilla_transport();
        let mut request = base_request(JavaServerFlavor::Vanilla, WorldSource::Fresh);
        request.name = "Race";
        let barrier = Barrier::new(2);
        let tmp_path = tmp.path();

        let run = || {
            barrier.wait();
            provisioning::create_download_and_go_server(
                &StdFileSystem,
                &transport,
                tmp_path,
                tmp_path,
                &tmp_path.join("templates/paper"),
                &tmp_path.join("templates/plugin"),
                &request,
                "2026-08-20T00:00:00Z",
                always_ok2,
                always_ok3,
            )
        };

        let (first, second) = std::thread::scope(|scope| {
            let a = scope.spawn(run);
            let b = scope.spawn(run);
            (a.join().unwrap(), b.join().unwrap())
        });

        let outcomes = [&first, &second];
        let successes = outcomes.iter().filter(|r| r.is_ok()).count();
        let already_exists = outcomes
            .iter()
            .filter(|r| matches!(r, Err(CreateServerError::FolderAlreadyExists { .. })))
            .count();
        assert_eq!(
            successes, 1,
            "exactly one concurrent create should win, attempt {attempt}: {first:?} / {second:?}"
        );
        assert_eq!(
            already_exists, 1,
            "the loser must see FolderAlreadyExists, not silently overwrite, attempt {attempt}"
        );
    }
}

// ---------------------------------------------------------------------
// P7.33: `sweep_orphaned_server_directory`, used by
// `msc_application::operations::LifecycleOperations::reconcile_on_startup`
// after an interrupted create reconciles to `Failed` on restart.
// ---------------------------------------------------------------------

#[test]
fn provisioning_sweep_orphaned_server_directory_removes_it() {
    let tmp = TempDir::new("sweep-present");
    let dir = tmp.path().join("java").join("orphaned");
    fs::create_dir_all(&dir).unwrap();

    provisioning::sweep_orphaned_server_directory(&StdFileSystem, tmp.path(), "orphaned");

    assert!(StdFileSystem.stat(&dir).is_err());
}

#[test]
fn provisioning_sweep_orphaned_server_directory_is_a_no_op_when_already_gone() {
    let tmp = TempDir::new("sweep-absent");

    // No panic, no error surfaced -- a normal in-process rollback already
    // beat the restart to it, the common case this function's own doc
    // describes.
    provisioning::sweep_orphaned_server_directory(&StdFileSystem, tmp.path(), "never-existed");
}

// ---------------------------------------------------------------------
// fixtures/server-creation/download-and-go-branch-downloads-jar-to-paper-jar-path.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_download_and_go_branch_writes_paper_jar() {
    let tmp = TempDir::new("jar-path");
    let transport = paper_transport();
    let request = base_request(JavaServerFlavor::Paper, WorldSource::Fresh);

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();

    assert!(created.config.paper_jar_path.ends_with("paper.jar"));
    assert_eq!(
        fs::read(&created.config.paper_jar_path).unwrap(),
        b"FAKE-PAPER-JAR"
    );
    assert_eq!(created.config.minecraft_version.as_deref(), Some("1.21.4"));
    assert_eq!(created.config.server_build.as_deref(), Some("231"));
}

// ---------------------------------------------------------------------
// fixtures/server-creation/eula-txt-written-as-eula-false.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_eula_txt_written_as_eula_false() {
    let tmp = TempDir::new("eula");
    let transport = vanilla_transport();
    let request = base_request(JavaServerFlavor::Vanilla, WorldSource::Fresh);

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();

    let server_dir = PathBuf::from(&created.config.server_dir);
    assert_eq!(
        fs::read_to_string(server_dir.join("eula.txt")).unwrap(),
        "eula=false\n"
    );
}

// ---------------------------------------------------------------------
// fixtures/server-creation/server-properties-exact-key-set-fresh-world.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_server_properties_exact_key_set_fresh_world() {
    let tmp = TempDir::new("props");
    let transport = vanilla_transport();
    let mut request = base_request(JavaServerFlavor::Vanilla, WorldSource::Fresh);
    request.name = "Fresh World Server";
    request.port = 25566;
    request.difficulty = "hard";
    request.gamemode = "survival";

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();

    let props = read_properties(&PathBuf::from(&created.config.server_dir));
    let mut keys: Vec<&String> = props.keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "difficulty",
            "gamemode",
            "level-name",
            "max-players",
            "motd",
            "online-mode",
            "server-port",
        ]
    );
    assert_eq!(props["server-port"], "25566");
    assert_eq!(props["motd"], "Fresh World Server");
    assert_eq!(props["max-players"], "20");
    assert_eq!(props["online-mode"], "true");
    assert_eq!(props["difficulty"], "hard");
    assert_eq!(props["gamemode"], "survival");
    assert_eq!(props["level-name"], "Fresh World Server");
    assert!(!props.contains_key("level-seed"));
}

// ---------------------------------------------------------------------
// fixtures/server-creation/addon-folder-none-for-vanilla.json
// fixtures/server-creation/addon-folder-plugins-for-plugin-flavor.json
// fixtures/server-creation/addon-folder-mods-for-modded-flavor.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_addon_folder_none_for_vanilla() {
    let tmp = TempDir::new("addon-vanilla");
    let transport = vanilla_transport();
    let request = base_request(JavaServerFlavor::Vanilla, WorldSource::Fresh);

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();

    let server_dir = PathBuf::from(&created.config.server_dir);
    assert!(!server_dir.join("plugins").exists());
    assert!(!server_dir.join("mods").exists());
}

#[test]
fn provisioning_addon_folder_plugins_for_plugin_flavor() {
    let tmp = TempDir::new("addon-plugin");
    let transport = paper_transport();
    let request = base_request(JavaServerFlavor::Paper, WorldSource::Fresh);

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();

    let server_dir = PathBuf::from(&created.config.server_dir);
    assert!(server_dir.join("plugins").is_dir());
}

/// Fixture's own example flavor is NeoForge (install-step, P7.18's
/// scope); Fabric exercises the identical `addOnKind`/`folderName` rule
/// this module's own `What:` line says is provisioning-kind-independent.
#[test]
fn provisioning_addon_folder_mods_for_modded_flavor() {
    let tmp = TempDir::new("addon-modded");
    let transport = fabric_transport();
    let request = base_request(JavaServerFlavor::Fabric, WorldSource::Fresh);

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();

    let server_dir = PathBuf::from(&created.config.server_dir);
    assert!(server_dir.join("mods").is_dir());
}

// ---------------------------------------------------------------------
// fixtures/server-creation/configserver-field-set-newly-created-server.json
// fixtures/server-creation/configserver-ram-default-plugin-2-4gb-vs-modded-3-6gb.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_configserver_field_set() {
    let tmp = TempDir::new("configserver-fields");
    let transport = paper_transport();
    let mut request = base_request(JavaServerFlavor::Paper, WorldSource::Fresh);
    request.name = "New Server";
    request.default_banner_color_hex = "#3366FF";

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();

    let config = created.config;
    assert_eq!(config.display_name, "New Server");
    assert_eq!(config.min_ram_gb, 2.0);
    assert_eq!(config.max_ram_gb, 4.0);
    assert_eq!(config.notes, "");
    assert_eq!(config.java_flavor, JavaServerFlavor::Paper);
    assert_eq!(config.minecraft_version.as_deref(), Some("1.21.4"));
    assert_eq!(config.server_build.as_deref(), Some("231"));
    assert_eq!(config.loader_version, None);
    assert_eq!(config.banner_color_hex.as_deref(), Some("#3366FF"));
    assert!(!config.playit_enabled);
    assert!(!config.xbox_broadcast_enabled);
    assert_eq!(config.bedrock_port, None);
    assert!(!config.id.is_empty());
}

#[test]
fn provisioning_ram_default_plugin_2_4gb_vs_modded_3_6gb() {
    let tmp = TempDir::new("ram-default");

    let paper_transport = paper_transport();
    let paper_request = base_request(JavaServerFlavor::Paper, WorldSource::Fresh);
    let paper_created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &paper_transport,
        tmp.path(),
        &tmp.path().join("paper-root"),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &paper_request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();
    assert_eq!(paper_created.config.min_ram_gb, 2.0);
    assert_eq!(paper_created.config.max_ram_gb, 4.0);

    let fabric_transport = fabric_transport();
    let fabric_request = base_request(JavaServerFlavor::Fabric, WorldSource::Fresh);
    let fabric_created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &fabric_transport,
        tmp.path(),
        &tmp.path().join("fabric-root"),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &fabric_request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();
    assert_eq!(fabric_created.config.min_ram_gb, 3.0);
    assert_eq!(fabric_created.config.max_ram_gb, 6.0);
}

// ---------------------------------------------------------------------
// fixtures/server-creation/cross-play-template-copy-applied-for-plugin-addon-when-enabled.json
// fixtures/server-creation/cross-play-template-copy-skipped-for-non-plugin-addon.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_cross_play_downloads_official_plugins_for_plugin_addon() {
    let tmp = TempDir::new("crossplay-applied");

    let transport = cross_play_paper_transport();
    let mut request = base_request(JavaServerFlavor::Paper, WorldSource::Fresh);
    request.enable_cross_play = true;
    request.cross_play_bedrock_port = Some(19132);

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();

    let plugins_dir = PathBuf::from(&created.config.server_dir).join("plugins");
    assert_eq!(
        fs::read(plugins_dir.join("Geyser-Spigot.jar")).unwrap(),
        b"FAKE-GEYSER-JAR"
    );
    assert_eq!(
        fs::read(plugins_dir.join("floodgate-spigot.jar")).unwrap(),
        b"FAKE-FLOODGATE-JAR"
    );
    let geyser_config = fs::read_to_string(plugins_dir.join("Geyser-Spigot/config.yml")).unwrap();
    assert!(geyser_config.contains("port: 19132"));
    assert!(geyser_config.contains("auth-type: floodgate"));
    assert_eq!(created.config.bedrock_port, Some(19132));
}

#[test]
fn provisioning_cross_play_refuses_non_plugin_addon() {
    let tmp = TempDir::new("crossplay-skipped");

    let transport = fabric_transport();
    let mut request = base_request(JavaServerFlavor::Fabric, WorldSource::Fresh);
    request.enable_cross_play = true;
    request.cross_play_bedrock_port = Some(19132);

    let err = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap_err();

    assert!(matches!(
        err,
        CreateServerError::CrossPlayUnsupported { .. }
    ));
}

// ---------------------------------------------------------------------
// fixtures/server-creation/paper-archive-first-shortcut-hit-copies-archived-jar-writes-sidecar.json
// fixtures/server-creation/paper-archive-first-shortcut-miss-falls-through-to-download.json
// fixtures/server-creation/archive-shortcut-skipped-when-save-downloaded-jars-disabled-or-non-paper.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_paper_archive_first_shortcut_hit() {
    let tmp = TempDir::new("archive-hit");
    let template_dir = tmp.path().join("templates/paper");
    fs::create_dir_all(&template_dir).unwrap();
    fs::write(
        template_dir.join("paper-1.21.4-build231.jar"),
        b"ARCHIVED-PAPER-JAR",
    )
    .unwrap();

    // Only the metadata-check URLs are registered — no jar-download URL
    // at all, so a fall-through to a real download would fail the whole
    // create with a "no fake response registered" error, proving the
    // archive path was actually taken.
    let transport = FakeTransport::new()
        .with(
            "https://fill.papermc.io/v3/projects/paper",
            br#"{"versions":{"1.21":["1.21.4"]}}"#.to_vec(),
        )
        .with(
            "https://fill.papermc.io/v3/projects/paper/versions/1.21.4/builds",
            br#"[{"id":231,"channel":"STABLE","downloads":{"server:default":{"url":"https://dl/paper-1.21.4-231.jar","checksums":{"sha256":"b90451bf06476ab0348852d0af747a6962e6b648e9a20ba261501e12e0d7b321"}}}}]"#.to_vec(),
        );

    let mut request = base_request(JavaServerFlavor::Paper, WorldSource::Fresh);
    request.save_downloaded_jars = true;

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &template_dir,
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();

    assert_eq!(
        fs::read(&created.config.paper_jar_path).unwrap(),
        b"ARCHIVED-PAPER-JAR"
    );
    assert_eq!(created.config.minecraft_version.as_deref(), Some("1.21.4"));
    assert_eq!(created.config.server_build.as_deref(), Some("231"));

    let sidecar_path = PathBuf::from(&created.config.server_dir).join(".msc_paper_version.json");
    let sidecar: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sidecar_path).unwrap()).unwrap();
    assert_eq!(sidecar["mcVersion"], "1.21.4");
    assert_eq!(sidecar["build"], 231);
}

#[test]
fn provisioning_paper_archive_first_shortcut_miss_falls_through() {
    let tmp = TempDir::new("archive-miss");
    let template_dir = tmp.path().join("templates/paper");
    // No archived jar present — the archive check must miss and fall
    // through to a real download.
    let transport = paper_transport();

    let mut request = base_request(JavaServerFlavor::Paper, WorldSource::Fresh);
    request.save_downloaded_jars = true;

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &template_dir,
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();

    assert_eq!(
        fs::read(&created.config.paper_jar_path).unwrap(),
        b"FAKE-PAPER-JAR"
    );
    // Archived afterward, so the next server of the same version hits
    // the shortcut instead.
    assert!(template_dir.join("paper-1.21.4-build231.jar").is_file());
}

#[test]
fn provisioning_archive_shortcut_skipped_for_non_paper() {
    let tmp = TempDir::new("archive-non-paper");
    // No Paper URLs registered at all — Purpur's own alignment probe
    // gracefully falls back (matching source's `try?`), and the
    // archive-first branch itself is never entered for a non-Paper
    // flavor regardless of `save_downloaded_jars`.
    let transport = purpur_transport();

    let mut request = base_request(JavaServerFlavor::Purpur, WorldSource::Fresh);
    request.save_downloaded_jars = true;

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();

    assert_eq!(created.config.minecraft_version.as_deref(), Some("1.21.4"));
    assert_eq!(created.config.server_build.as_deref(), Some("2"));
    assert_eq!(
        fs::read(&created.config.paper_jar_path).unwrap(),
        b"FAKE-PURPUR-JAR"
    );
}

// ---------------------------------------------------------------------
// fixtures/server-creation/catch-block-removes-newdir-on-any-thrown-error.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_catch_block_removes_newdir_on_download_failure() {
    let tmp = TempDir::new("catch-cleanup");
    // No responses registered at all — the jar download fails
    // immediately.
    let transport = FakeTransport::new();
    let request = base_request(JavaServerFlavor::Vanilla, WorldSource::Fresh);

    let err = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap_err();

    assert!(matches!(err, CreateServerError::Download(_)));
    assert!(!tmp.path().join("java").join("test_server").exists());
}

// ---------------------------------------------------------------------
// fixtures/server-creation/world-source-backup-zip-failure-aborts-returns-false.json
// fixtures/server-creation/world-source-existing-folder-failure-aborts-returns-false.json
//
// Deliberate strengthening over the oracle, per this module's own doc:
// both failures roll `new_dir` back completely rather than leaving a
// half-provisioned server behind.
// ---------------------------------------------------------------------

#[test]
fn provisioning_world_source_backup_zip_failure_rolls_back() {
    let tmp = TempDir::new("backup-zip-fail");
    let transport = vanilla_transport();
    let zip_path = tmp.path().join("backup.zip");
    fs::write(&zip_path, b"not really a zip").unwrap();
    let request = base_request(JavaServerFlavor::Vanilla, WorldSource::BackupZip(&zip_path));

    let err = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_fail2,
        always_ok3,
    )
    .unwrap_err();

    assert!(matches!(err, CreateServerError::WorldSourceFailed));
    assert!(!tmp.path().join("java").join("test_server").exists());
}

#[test]
fn provisioning_world_source_existing_folder_failure_rolls_back() {
    let tmp = TempDir::new("existing-folder-fail");
    let transport = vanilla_transport();
    let src_folder = tmp.path().join("src-world");
    fs::create_dir_all(&src_folder).unwrap();
    let request = base_request(
        JavaServerFlavor::Vanilla,
        WorldSource::ExistingFolder(&src_folder),
    );

    let err = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_fail3,
    )
    .unwrap_err();

    assert!(matches!(err, CreateServerError::WorldSourceFailed));
    assert!(!tmp.path().join("java").join("test_server").exists());
}

// ---------------------------------------------------------------------
// fixtures/server-creation/initial-world-slot-failure-deletes-directory-sets-error.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_initial_world_slot_failure_deletes_directory() {
    let tmp = TempDir::new("slot-fail");
    let transport = vanilla_transport();
    // A non-existent backup zip makes `import_zip_as_new_slot` fail with
    // `WorldError::NoSourceZip`, collapsed by this module into
    // `InitialWorldSlotFailed` — the same "guard returns nil" shape
    // source's own `createInitialPersistentWorldSlot` has.
    let missing_zip = tmp.path().join("does-not-exist.zip");
    let request = base_request(
        JavaServerFlavor::Vanilla,
        WorldSource::BackupZip(&missing_zip),
    );

    let err = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap_err();

    assert!(matches!(err, CreateServerError::InitialWorldSlotFailed));
    assert!(!tmp.path().join("java").join("test_server").exists());
}

// ---------------------------------------------------------------------
// fixtures/server-creation/imported-metadata-overrides-difficulty-gamemode-seed.json
// ---------------------------------------------------------------------

#[test]
fn provisioning_imported_metadata_overrides_wizard_values() {
    let tmp = TempDir::new("imported-metadata");
    let transport = vanilla_transport();

    let level_dat = gzip(&be_java_root(&[
        be_int_entry("Difficulty", 3), // hard
        be_int_entry("GameType", 0),   // survival
        be_int_entry("RandomSeed", 1234567890),
    ]));
    let zip_path = tmp.path().join("backup.zip");
    write_backup_zip_with_level_dat(&zip_path, &level_dat);

    let mut request = base_request(JavaServerFlavor::Vanilla, WorldSource::BackupZip(&zip_path));
    request.difficulty = "peaceful";
    request.gamemode = "creative";
    request.world_seed = None;

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        real_unzip_world_backup,
        real_copy_existing_world_folder,
    )
    .unwrap();

    let props = read_properties(&PathBuf::from(&created.config.server_dir));
    assert_eq!(props["difficulty"], "hard");
    assert_eq!(props["gamemode"], "survival");
    assert_eq!(props["level-seed"], "1234567890");
}

#[test]
fn provisioning_applies_the_first_world_profile_before_first_start() {
    let tmp = TempDir::new("world-profile");
    let transport = vanilla_transport();
    let mut profile = WorldProfile::new();
    profile.identity.name = Some("Profile World".to_string());
    profile.identity.level_name = Some("profile_level".to_string());
    profile.identity.seed = Some("profile-seed".to_string());
    profile.generation.world_type = Some("flat".to_string());
    profile.generation.structures = Some(false);
    profile.generation.bonus_chest = Some(true);
    profile.generation.generator_options = Some("{}".to_string());
    profile.gameplay.difficulty = Some("hard".to_string());
    profile.gameplay.default_game_mode = Some("creative".to_string());
    profile.gameplay.hardcore = Some(true);
    profile.gameplay.commands = Some(true);
    profile
        .gameplay
        .gamerules
        .insert("keepInventory".to_string(), "true".to_string());

    let mut request = base_request(JavaServerFlavor::Vanilla, WorldSource::Fresh);
    request.initial_world_profile = Some(&profile);

    let created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();

    let props = read_properties(&PathBuf::from(&created.config.server_dir));
    assert_eq!(created.world_slot.name, "Profile World");
    assert_eq!(
        created.world_slot.world_level_name.as_deref(),
        Some("profile_level")
    );
    assert_eq!(props["level-name"], "profile_level");
    assert_eq!(props["level-seed"], "profile-seed");
    assert_eq!(props["level-type"], "minecraft\\:flat");
    assert_eq!(props["generate-structures"], "false");
    assert_eq!(props["bonus-chest"], "true");
    assert_eq!(props["generator-settings"], "{}");
    assert_eq!(props["difficulty"], "hard");
    assert_eq!(props["gamemode"], "creative");
    assert_eq!(props["hardcore"], "true");
    assert_eq!(props["enable-command-block"], "true");

    let saved = msc_infrastructure::world_store::load_profile(
        &StdFileSystem,
        &PathBuf::from(&created.config.server_dir),
        &created.world_slot,
    );
    assert_eq!(saved.gameplay.gamerules["keepInventory"], "true");
}

// ---------------------------------------------------------------------
// fixtures/server-creation/install-step-branch-skips-jar-download-runs-installer.json
//
// NeoForge/Forge are P7.18's job — this module refuses them outright
// rather than silently mis-provisioning an install-step flavor.
// ---------------------------------------------------------------------

#[test]
fn provisioning_install_step_flavor_refused() {
    let tmp = TempDir::new("install-step-refused");
    let transport = FakeTransport::new();
    let request = base_request(JavaServerFlavor::NeoForge, WorldSource::Fresh);

    let err = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &transport,
        tmp.path(),
        tmp.path(),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap_err();

    assert!(matches!(
        err,
        CreateServerError::UnsupportedFlavor(JavaServerFlavor::NeoForge)
    ));
}

// ---------------------------------------------------------------------
// fixtures/server-creation/record-loader-version-called-only-for-modded-category.json
//
// The fixture's own example flavor (NeoForge) is install-step and out
// of this module's scope; Fabric/Paper exercise the identical
// `should_record_loader_version` condition this module surfaces via
// `CreatedServer::should_record_loader_version`.
// ---------------------------------------------------------------------

#[test]
fn provisioning_record_loader_version_only_for_modded_category() {
    let tmp = TempDir::new("record-loader");

    let fabric_transport = fabric_transport();
    let fabric_request = base_request(JavaServerFlavor::Fabric, WorldSource::Fresh);
    let fabric_created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &fabric_transport,
        tmp.path(),
        &tmp.path().join("fabric-root"),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &fabric_request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();
    assert!(fabric_created.should_record_loader_version);
    assert_eq!(
        fabric_created.config.loader_version.as_deref(),
        Some("0.16.9")
    );

    let paper_transport = paper_transport();
    let paper_request = base_request(JavaServerFlavor::Paper, WorldSource::Fresh);
    let paper_created = provisioning::create_download_and_go_server(
        &StdFileSystem,
        &paper_transport,
        tmp.path(),
        &tmp.path().join("paper-root"),
        &tmp.path().join("templates/paper"),
        &tmp.path().join("templates/plugin"),
        &paper_request,
        "2026-08-18T00:00:00Z",
        always_ok2,
        always_ok3,
    )
    .unwrap();
    assert!(!paper_created.should_record_loader_version);
}
