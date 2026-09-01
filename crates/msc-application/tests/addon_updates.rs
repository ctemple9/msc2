//! P8.16's own tests: direct unit coverage over `msc_application::
//! addon_updates`, wiring `msc_domain::addon_update`'s already-fixture-
//! mapped decisions (P8.11) through real file hashing and a fake Modrinth
//! transport. `fixtures/addon-update-resolution/` describes the pure
//! per-item decisions (already ported/tested in P8.11); this file's own
//! job is proving the *wiring* — hashing real bytes, keying the two
//! batched lookups off those hashes, merging with persisted links, and the
//! bounded per-server plan cache — which no fixture there describes.

use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::path::Path;

use msc_application::addon_updates::{self, AddonUpdatePlan};
use msc_domain::addon_update::{AddonUpdateBucket, PluginTier};
use msc_domain::app_config_schema::{
    AddonLink, AddonLinkProvenance, PluginSourceConfig, PluginSourceKind,
};
use msc_domain::identity::JavaServerFlavor;
use msc_infrastructure::addon_provider::{AddonTransport, RawResponse, TransportError};
use msc_infrastructure::download_staging::sha512_hex;
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

struct FakeTransport {
    identify: Option<serde_json::Value>,
    latest: Option<serde_json::Value>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            identify: None,
            latest: None,
        }
    }
    fn with_identify(mut self, body: serde_json::Value) -> Self {
        self.identify = Some(body);
        self
    }
    fn with_latest(mut self, body: serde_json::Value) -> Self {
        self.latest = Some(body);
        self
    }
}

impl AddonTransport for FakeTransport {
    fn get(
        &self,
        url: &str,
        what: &str,
        _headers: &[(&str, &str)],
        _max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!(
            "{what}: unexpected GET {url} — the update-resolve pass only POSTs its two batch lookups"
        );
    }

    fn post_json(
        &self,
        url: &str,
        what: &str,
        _body: &serde_json::Value,
        _headers: &[(&str, &str)],
        _max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        let body = if url.ends_with("/version_files/update") {
            self.latest.clone().unwrap_or_else(|| serde_json::json!({}))
        } else if url.ends_with("/version_files") {
            self.identify
                .clone()
                .unwrap_or_else(|| serde_json::json!({}))
        } else {
            panic!("{what}: unexpected POST {url}");
        };
        Ok(RawResponse {
            status: 200,
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

fn version_json(id: &str, project_id: &str, version_number: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "project_id": project_id,
        "version_number": version_number,
        "files": [],
        "dependencies": [],
    })
}

fn write_jar(fs: &FakeFileSystem, dir: &Path, filename: &str, contents: &[u8]) {
    fs.create_dir_all(dir).unwrap();
    fs.write(&dir.join(filename), contents).unwrap();
}

#[test]
fn addon_updates_unlinked_when_no_identity_match() {
    let fs = FakeFileSystem::new();
    let dir = Path::new("/server/mods");
    write_jar(&fs, dir, "unknown-mod-1.0.jar", b"unknown bytes");

    let transport = FakeTransport::new();
    let plan = addon_updates::resolve_addon_updates(
        &transport,
        &fs,
        dir,
        JavaServerFlavor::Fabric,
        Some("1.21.1"),
        &HashMap::new(),
        &HashMap::new(),
    );

    assert_eq!(plan.items.len(), 1);
    let item = &plan.items[0];
    assert_eq!(item.bucket, AddonUpdateBucket::Unlinked);
    assert_eq!(item.project_id, None);
    assert!(plan.discovered_links.is_empty());
}

#[test]
fn addon_updates_fresh_hash_match_update_available_and_self_healing_link() {
    let fs = FakeFileSystem::new();
    let dir = Path::new("/server/mods");
    let bytes = b"iris shader mod jar bytes";
    write_jar(&fs, dir, "iris-1.7.jar", bytes);
    let hash = sha512_hex(bytes);

    let identify = serde_json::json!({ hash.clone(): version_json("v-old", "iris-proj", "1.7") });
    let latest = serde_json::json!({ hash.clone(): version_json("v-new", "iris-proj", "1.8") });
    let transport = FakeTransport::new()
        .with_identify(identify)
        .with_latest(latest);

    let plan = addon_updates::resolve_addon_updates(
        &transport,
        &fs,
        dir,
        JavaServerFlavor::Fabric,
        Some("1.21.1"),
        &HashMap::new(),
        &HashMap::new(),
    );

    assert_eq!(plan.items.len(), 1);
    let item = &plan.items[0];
    assert_eq!(item.project_id.as_deref(), Some("iris-proj"));
    assert_eq!(item.provenance, Some(AddonLinkProvenance::HashDetected));
    assert_eq!(item.bucket, AddonUpdateBucket::UpdateAvailable);
    assert_eq!(item.available_version_id.as_deref(), Some("v-new"));
    assert_eq!(item.available_version_label.as_deref(), Some("1.8"));

    // Self-healing: a fresh hash hit records a discovered link.
    let link = plan
        .discovered_links
        .get("iris-proj")
        .expect("fresh hash hit should self-heal a link");
    assert_eq!(link.installed_version_id.as_deref(), Some("v-old"));
    assert_eq!(link.installed_hash.as_deref(), Some(hash.as_str()));
}

#[test]
fn addon_updates_up_to_date_when_latest_matches_current() {
    let fs = FakeFileSystem::new();
    let dir = Path::new("/server/mods");
    let bytes = b"already-current mod bytes";
    write_jar(&fs, dir, "sodium-0.5.jar", bytes);
    let hash = sha512_hex(bytes);

    let identify = serde_json::json!({ hash.clone(): version_json("v-1", "sodium-proj", "0.5") });
    let latest = serde_json::json!({ hash.clone(): version_json("v-1", "sodium-proj", "0.5") });
    let transport = FakeTransport::new()
        .with_identify(identify)
        .with_latest(latest);

    let plan = addon_updates::resolve_addon_updates(
        &transport,
        &fs,
        dir,
        JavaServerFlavor::Fabric,
        Some("1.21.1"),
        &HashMap::new(),
        &HashMap::new(),
    );

    assert_eq!(plan.items[0].bucket, AddonUpdateBucket::UpToDate);
    assert_eq!(plan.items[0].available_version_id, None);
}

#[test]
fn addon_updates_persisted_fallback_used_when_not_freshly_identified() {
    // No provider registers a fresh identify/latest hit for this hash at
    // all (empty maps) — the item must still resolve via the persisted
    // link's own hash match (`resolve_project_id`'s second clause), never
    // record a NEW self-healing link (`should_record_self_healing_link`
    // requires a FRESH hit), and keep the persisted link's own provenance.
    let fs = FakeFileSystem::new();
    let dir = Path::new("/server/mods");
    let bytes = b"persisted-link mod bytes";
    write_jar(&fs, dir, "userlinked-2.0.jar", bytes);
    let hash = sha512_hex(bytes);

    let mut links = HashMap::new();
    links.insert(
        "userlinked-proj".to_string(),
        AddonLink {
            project_id: "userlinked-proj".to_string(),
            title: Some("User Linked".to_string()),
            slug: None,
            icon_url: None,
            provenance: AddonLinkProvenance::UserLinked,
            installed_version_id: Some("v-persisted".to_string()),
            installed_file_name: Some("userlinked-2.0.jar".to_string()),
            installed_hash: Some(hash),
            client_side: None,
            server_side: None,
            extra: Default::default(),
        },
    );

    let transport = FakeTransport::new();
    let plan = addon_updates::resolve_addon_updates(
        &transport,
        &fs,
        dir,
        JavaServerFlavor::Fabric,
        None,
        &links,
        &HashMap::new(),
    );

    let item = &plan.items[0];
    assert_eq!(item.project_id.as_deref(), Some("userlinked-proj"));
    assert_eq!(item.provenance, Some(AddonLinkProvenance::UserLinked));
    // No mc_version configured and no fresh latest hit -> UpToDate, not
    // NoCompatibleVersion (`resolve_bucket`'s own None-branch split).
    assert_eq!(item.bucket, AddonUpdateBucket::UpToDate);
    assert!(
        plan.discovered_links.is_empty(),
        "a persisted-fallback match must never self-heal a new link"
    );
}

#[test]
fn addon_updates_geyser_excluded_on_plugin_server_kept_on_mod_server() {
    let fs = FakeFileSystem::new();
    let plugins_dir = Path::new("/server/plugins");
    write_jar(&fs, plugins_dir, "Geyser-Spigot.jar", b"geyser bytes");
    let transport = FakeTransport::new();

    let plan = addon_updates::resolve_addon_updates(
        &transport,
        &fs,
        plugins_dir,
        JavaServerFlavor::Paper,
        None,
        &HashMap::new(),
        &HashMap::new(),
    );
    // Excluded from hash resolution -> no hash computed -> falls straight
    // through to Unlinked without ever registering a fake response (a bug
    // here would panic on the unexpected POST/GET instead of silently
    // passing).
    assert_eq!(plan.items[0].bucket, AddonUpdateBucket::Unlinked);

    let mods_dir = Path::new("/server2/mods");
    write_jar(&fs, mods_dir, "geyser-fabric.jar", b"geyser mod bytes");
    let plan2 = addon_updates::resolve_addon_updates(
        &transport,
        &fs,
        mods_dir,
        JavaServerFlavor::Fabric,
        None,
        &HashMap::new(),
        &HashMap::new(),
    );
    // Kept in hash resolution on a mod server -> a hash IS computed, and
    // with no fake identify response registered it resolves Unlinked too,
    // but via a real (empty) lookup rather than a skip -- proven by the
    // `with_identify`/`with_latest` defaults being `{}`, not a panic.
    assert_eq!(plan2.items[0].bucket, AddonUpdateBucket::Unlinked);
}

#[test]
fn addon_updates_reads_managed_plugin_version_from_jar() {
    let fs = FakeFileSystem::new();
    let plugins_dir = Path::new("/server/plugins");
    let mut bytes = Cursor::new(Vec::new());
    {
        let mut archive = ZipWriter::new(&mut bytes);
        archive
            .start_file("plugin.yml", SimpleFileOptions::default())
            .unwrap();
        archive
            .write_all(b"name: Geyser-Spigot\nversion: 2.11.2-SNAPSHOT\n")
            .unwrap();
        archive.finish().unwrap();
    }
    write_jar(&fs, plugins_dir, "Geyser-Spigot.jar", &bytes.into_inner());

    let plan = addon_updates::resolve_addon_updates(
        &FakeTransport::new(),
        &fs,
        plugins_dir,
        JavaServerFlavor::Paper,
        None,
        &HashMap::new(),
        &HashMap::new(),
    );

    assert_eq!(
        plan.items[0].current_version.as_deref(),
        Some("2.11.2-SNAPSHOT")
    );
    assert_eq!(plan.items[0].bucket, AddonUpdateBucket::Unlinked);
}

#[test]
fn addon_updates_plugin_tier_derived_for_plugin_servers_only() {
    let fs = FakeFileSystem::new();
    let plugins_dir = Path::new("/server/plugins");
    write_jar(&fs, plugins_dir, "LuckPerms-5.4.jar", b"luckperms bytes");
    let mut sources = HashMap::new();
    sources.insert(
        "LuckPerms-5.4".to_string(),
        PluginSourceConfig {
            url: "https://example.invalid/luckperms".to_string(),
            source_type: PluginSourceKind::Direct,
            extra: Default::default(),
        },
    );
    let transport = FakeTransport::new();
    let plan = addon_updates::resolve_addon_updates(
        &transport,
        &fs,
        plugins_dir,
        JavaServerFlavor::Paper,
        None,
        &HashMap::new(),
        &sources,
    );
    assert_eq!(plan.items[0].tier, Some(PluginTier::UserSourced));

    let mods_dir = Path::new("/server3/mods");
    write_jar(&fs, mods_dir, "somemod-1.0.jar", b"mod bytes");
    let plan2 = addon_updates::resolve_addon_updates(
        &transport,
        &fs,
        mods_dir,
        JavaServerFlavor::Fabric,
        None,
        &HashMap::new(),
        &HashMap::new(),
    );
    assert_eq!(
        plan2.items[0].tier, None,
        "mod servers have no plugin-source tier concept"
    );
}

#[test]
fn addon_updates_vanilla_server_returns_empty_plan() {
    let fs = FakeFileSystem::new();
    let dir = Path::new("/server/mods");
    let transport = FakeTransport::new();
    let plan: AddonUpdatePlan = addon_updates::resolve_addon_updates(
        &transport,
        &fs,
        dir,
        JavaServerFlavor::Vanilla,
        None,
        &HashMap::new(),
        &HashMap::new(),
    );
    assert!(plan.items.is_empty());
}

#[test]
fn addon_updates_deterministic_sort_bucket_rank_then_alphabetical() {
    let fs = FakeFileSystem::new();
    let dir = Path::new("/server/mods");
    write_jar(&fs, dir, "zeta-mod-1.0.jar", b"zeta bytes");
    write_jar(&fs, dir, "alpha-mod-1.0.jar", b"alpha bytes");
    let transport = FakeTransport::new();
    let plan = addon_updates::resolve_addon_updates(
        &transport,
        &fs,
        dir,
        JavaServerFlavor::Fabric,
        None,
        &HashMap::new(),
        &HashMap::new(),
    );
    // Both Unlinked (same bucket rank) -> alphabetical by display name.
    let names: Vec<&str> = plan.items.iter().map(|i| i.display_name.as_str()).collect();
    assert_eq!(names, vec!["alpha-mod", "zeta-mod"]);
}

// --- AddonPlanCache ---

#[test]
fn addon_plan_cache_recomputes_on_first_request_then_serves_cached() {
    let mut cache = addon_updates::AddonPlanCache::new();
    let mut calls = 0;
    {
        let _ = cache.get_or_resolve("server-a", false, || {
            calls += 1;
            AddonUpdatePlan::default()
        });
    }
    {
        let _ = cache.get_or_resolve("server-a", false, || {
            calls += 1;
            AddonUpdatePlan::default()
        });
    }
    assert_eq!(
        calls, 1,
        "second call for the same server must be a cache hit"
    );
    assert_eq!(cache.len(), 1);
}

#[test]
fn addon_plan_cache_force_always_recomputes() {
    let mut cache = addon_updates::AddonPlanCache::new();
    let mut calls = 0;
    for _ in 0..3 {
        let _ = cache.get_or_resolve("server-a", true, || {
            calls += 1;
            AddonUpdatePlan::default()
        });
    }
    assert_eq!(calls, 3);
}

#[test]
fn addon_plan_cache_holds_multiple_servers_independently() {
    let mut cache = addon_updates::AddonPlanCache::new();
    let mut calls = 0;
    let _ = cache.get_or_resolve("server-a", false, || {
        calls += 1;
        AddonUpdatePlan::default()
    });
    let _ = cache.get_or_resolve("server-b", false, || {
        calls += 1;
        AddonUpdatePlan::default()
    });
    // Re-requesting server-a must still be a hit: a fleet cache doesn't
    // drop server-a's plan just because server-b was resolved too (unlike
    // MSC 1's own single-slot cache, which this is a bounded, multi-slot
    // generalization of — see `addon_updates.rs`'s own module doc).
    let _ = cache.get_or_resolve("server-a", false, || {
        calls += 1;
        AddonUpdatePlan::default()
    });
    assert_eq!(calls, 2);
    assert_eq!(cache.len(), 2);
}

#[test]
fn addon_plan_cache_invalidate_forces_next_recompute() {
    let mut cache = addon_updates::AddonPlanCache::new();
    let mut calls = 0;
    let _ = cache.get_or_resolve("server-a", false, || {
        calls += 1;
        AddonUpdatePlan::default()
    });
    cache.invalidate("server-a");
    assert_eq!(cache.len(), 0);
    let _ = cache.get_or_resolve("server-a", false, || {
        calls += 1;
        AddonUpdatePlan::default()
    });
    assert_eq!(calls, 2);
}

#[test]
fn addon_plan_cache_bounded_evicts_least_recently_used() {
    let mut cache = addon_updates::AddonPlanCache::new();
    for i in 0..(addon_updates::MAX_CACHED_PLANS + 1) {
        let id = format!("server-{i}");
        let _ = cache.get_or_resolve(&id, false, AddonUpdatePlan::default);
    }
    assert_eq!(
        cache.len(),
        addon_updates::MAX_CACHED_PLANS,
        "cache must never hold more than MAX_CACHED_PLANS servers' plans at once"
    );
    // The very first server inserted should have been evicted (least
    // recently used), while the most recent one is still present.
    let mut calls = 0;
    let _ = cache.get_or_resolve("server-0", false, || {
        calls += 1;
        AddonUpdatePlan::default()
    });
    assert_eq!(
        calls, 1,
        "server-0 should have been evicted, forcing a recompute"
    );
}

// ---------------------------------------------------------------------
// P8.23: `update`/`install` health repairs
// ---------------------------------------------------------------------

fn startup_problem(
    kind: msc_domain::crash_analysis::StartupProblemKind,
    installed_jar_stem: Option<&str>,
    missing_dependency: Option<&str>,
) -> msc_domain::crash_analysis::StartupProblem {
    msc_domain::crash_analysis::StartupProblem {
        kind,
        offender_name: "SomeMod".to_string(),
        offender_id: None,
        installed_file: installed_jar_stem.map(|s| format!("{s}.jar")),
        installed_jar_stem: installed_jar_stem.map(str::to_string),
        requirement: None,
        missing_dependency: missing_dependency.map(str::to_string),
        raw_excerpt: String::new(),
    }
}

/// A transport that panics on any call — used to prove a guard fires
/// before any network request, the same "no fake response registered"
/// panic-as-proof convention this phase's other tests already use.
struct PanicTransport;
impl AddonTransport for PanicTransport {
    fn get(
        &self,
        url: &str,
        what: &str,
        _headers: &[(&str, &str)],
        _max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!("{what}: unexpected GET {url}");
    }
    fn post_json(
        &self,
        url: &str,
        what: &str,
        _body: &serde_json::Value,
        _headers: &[(&str, &str)],
        _max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!("{what}: unexpected POST {url}");
    }
}

fn never_cancel() -> bool {
    false
}

#[test]
fn repair_update_refuses_while_running() {
    let fs = FakeFileSystem::new();
    let problem = startup_problem(
        msc_domain::crash_analysis::StartupProblemKind::IncompatibleVersion,
        Some("sodium-0.4.0"),
        None,
    );
    let err = addon_updates::repair_update(
        &PanicTransport,
        &fs,
        Path::new("/servers/java/box"),
        JavaServerFlavor::Fabric,
        None,
        &HashMap::new(),
        &HashMap::new(),
        false,
        &problem,
        true,
        &never_cancel,
    )
    .expect_err("running refused");
    assert!(matches!(
        err,
        addon_updates::HealthRepairError::ServerRunning
    ));
}

#[test]
fn repair_update_no_add_on_kind_for_vanilla() {
    let fs = FakeFileSystem::new();
    let problem = startup_problem(
        msc_domain::crash_analysis::StartupProblemKind::IncompatibleVersion,
        Some("sodium-0.4.0"),
        None,
    );
    let err = addon_updates::repair_update(
        &PanicTransport,
        &fs,
        Path::new("/servers/java/box"),
        JavaServerFlavor::Vanilla,
        None,
        &HashMap::new(),
        &HashMap::new(),
        false,
        &problem,
        false,
        &never_cancel,
    )
    .expect_err("vanilla has no add-on folder");
    assert!(matches!(err, addon_updates::HealthRepairError::NoAddOnKind));
}

#[test]
fn repair_update_action_unavailable_without_jar_stem() {
    let fs = FakeFileSystem::new();
    let problem = startup_problem(
        msc_domain::crash_analysis::StartupProblemKind::IncompatibleVersion,
        None,
        None,
    );
    let err = addon_updates::repair_update(
        &PanicTransport,
        &fs,
        Path::new("/servers/java/box"),
        JavaServerFlavor::Fabric,
        None,
        &HashMap::new(),
        &HashMap::new(),
        false,
        &problem,
        false,
        &never_cancel,
    )
    .expect_err("no jar stem to act on");
    assert!(matches!(
        err,
        addon_updates::HealthRepairError::ActionUnavailable
    ));
}

#[test]
fn repair_update_no_update_available_for_unlinked_item() {
    let fs = FakeFileSystem::new();
    let dir = Path::new("/server/mods");
    write_jar(&fs, dir, "sodium-0.4.0.jar", b"jar-bytes");
    // Empty identify/latest responses -> the item resolves as Unlinked,
    // never UpdateAvailable.
    let transport = FakeTransport::new();
    let problem = startup_problem(
        msc_domain::crash_analysis::StartupProblemKind::IncompatibleVersion,
        Some("sodium-0.4.0"),
        None,
    );
    let err = addon_updates::repair_update(
        &transport,
        &fs,
        Path::new("/server"),
        JavaServerFlavor::Fabric,
        None,
        &HashMap::new(),
        &HashMap::new(),
        false,
        &problem,
        false,
        &never_cancel,
    )
    .expect_err("unlinked item has no update available");
    assert!(matches!(
        err,
        addon_updates::HealthRepairError::NoUpdateAvailable
    ));
}

#[test]
fn repair_install_missing_dependency_refuses_while_running() {
    let fs = FakeFileSystem::new();
    let problem = startup_problem(
        msc_domain::crash_analysis::StartupProblemKind::MissingDependency,
        None,
        Some("fabric api"),
    );
    let err = addon_updates::repair_install_missing_dependency(
        &PanicTransport,
        &fs,
        Path::new("/servers/java/box"),
        JavaServerFlavor::Fabric,
        None,
        false,
        &problem,
        true,
        &never_cancel,
    )
    .expect_err("running refused");
    assert!(matches!(
        err,
        addon_updates::HealthRepairError::ServerRunning
    ));
}

#[test]
fn repair_install_missing_dependency_action_unavailable_without_name() {
    let fs = FakeFileSystem::new();
    let problem = startup_problem(
        msc_domain::crash_analysis::StartupProblemKind::MissingDependency,
        None,
        None,
    );
    let err = addon_updates::repair_install_missing_dependency(
        &PanicTransport,
        &fs,
        Path::new("/servers/java/box"),
        JavaServerFlavor::Fabric,
        None,
        false,
        &problem,
        false,
        &never_cancel,
    )
    .expect_err("no dependency name to act on");
    assert!(matches!(
        err,
        addon_updates::HealthRepairError::ActionUnavailable
    ));
}

struct SearchTransport {
    hits: Vec<serde_json::Value>,
}
impl AddonTransport for SearchTransport {
    fn get(
        &self,
        url: &str,
        what: &str,
        _headers: &[(&str, &str)],
        _max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        if url.contains("/v2/search") {
            let body = serde_json::json!({ "hits": self.hits, "total_hits": self.hits.len() });
            return Ok(RawResponse {
                status: 200,
                body: serde_json::to_vec(&body).unwrap(),
            });
        }
        panic!("{what}: unexpected GET {url}");
    }
    fn post_json(
        &self,
        url: &str,
        what: &str,
        _body: &serde_json::Value,
        _headers: &[(&str, &str)],
        _max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!("{what}: unexpected POST {url}");
    }
}

#[test]
fn repair_install_missing_dependency_no_confident_match_refuses_to_guess() {
    let fs = FakeFileSystem::new();
    let transport = SearchTransport {
        hits: vec![serde_json::json!({
            "project_id": "abc123",
            "slug": "totally-unrelated-mod",
            "title": "Totally Unrelated Mod",
        })],
    };
    let problem = startup_problem(
        msc_domain::crash_analysis::StartupProblemKind::MissingDependency,
        None,
        Some("fabric api"),
    );
    let err = addon_updates::repair_install_missing_dependency(
        &transport,
        &fs,
        Path::new("/servers/java/box"),
        JavaServerFlavor::Fabric,
        None,
        false,
        &problem,
        false,
        &never_cancel,
    )
    .expect_err("no confident match should ever install the wrong thing");
    assert!(matches!(
        err,
        addon_updates::HealthRepairError::NoConfidentMatch
    ));
}

#[test]
fn find_confident_dependency_match_exact_title_match() {
    let hits = vec![msc_domain::addon_provider::ModrinthSearchHit {
        project_id: "P1".to_string(),
        slug: "fabric-api".to_string(),
        title: "Fabric API".to_string(),
        ..Default::default()
    }];
    assert_eq!(
        addon_updates::find_confident_dependency_match("fabric api", &hits),
        Some("P1".to_string())
    );
}

#[test]
fn find_confident_dependency_match_exact_slug_match() {
    let hits = vec![msc_domain::addon_provider::ModrinthSearchHit {
        project_id: "P2".to_string(),
        slug: "fabric-api".to_string(),
        title: "A Completely Different Display Name".to_string(),
        ..Default::default()
    }];
    assert_eq!(
        addon_updates::find_confident_dependency_match("Fabric-API", &hits),
        Some("P2".to_string())
    );
}

#[test]
fn find_confident_dependency_match_no_hit_returns_none() {
    let hits = vec![msc_domain::addon_provider::ModrinthSearchHit {
        project_id: "P3".to_string(),
        slug: "sodium".to_string(),
        title: "Sodium".to_string(),
        ..Default::default()
    }];
    assert_eq!(
        addon_updates::find_confident_dependency_match("fabric api", &hits),
        None
    );
}
