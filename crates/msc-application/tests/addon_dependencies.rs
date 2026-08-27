//! P8.15's own tests, direct unit coverage over `msc_domain::
//! addon_dependency`'s already-fixture-mapped decisions (P8.12) rather
//! than a second fixture-mapped set of its own -- this step's job is
//! wiring those decisions through real provider/store calls with
//! recursion, ordering, cycle detection, and cancellation, which
//! `fixtures/modrinth-dependencies/` doesn't itself describe (no fixture
//! there mentions network wiring, recursion order, or cancellation at
//! all). Case names below cite the fixture they exercise the *shape* of
//! where one exists.

use std::cell::Cell;
use std::collections::HashMap;
use std::path::Path;

use msc_application::addon_dependencies::{self, DependencyInstallOutcome};
use msc_domain::addon_dependency::ModrinthDependency;
use msc_domain::addon_provider::ModrinthVersionInfo;
use msc_domain::identity::JavaServerFlavor;
use msc_infrastructure::addon_provider::{AddonTransport, RawResponse, TransportError};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};

const BASE: &str = "https://api.modrinth.com";

/// Routes a GET by URL shape rather than an exact literal match, so tests
/// don't need to hand-replicate `modrinth_project_versions`'s query-string
/// encoding: `{BASE}/v2/project/{id}` is a project lookup keyed by `id`,
/// `{BASE}/v2/project/{slug}/version...` is a version-list lookup keyed by
/// `slug` (ignoring any query string), and everything else is a plain
/// download URL lookup. A lookup miss panics -- the same "prove this
/// wasn't called" technique `tests/addon_provider.rs`'s own `FakeTransport`
/// established, load-bearing for the already-installed/already-present/
/// cycle-detected/cancellation tests below, which assert a *skip* partly by
/// never registering the response that skipping should avoid fetching.
struct FakeTransport {
    projects: HashMap<String, (u16, Vec<u8>)>,
    versions: HashMap<String, (u16, Vec<u8>)>,
    downloads: HashMap<String, (u16, Vec<u8>)>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            projects: HashMap::new(),
            versions: HashMap::new(),
            downloads: HashMap::new(),
        }
    }

    fn with_project(mut self, id: &str, status: u16, body: serde_json::Value) -> Self {
        self.projects
            .insert(id.to_string(), (status, body.to_string().into_bytes()));
        self
    }

    fn with_versions(mut self, slug: &str, versions: Vec<serde_json::Value>) -> Self {
        let body = serde_json::Value::Array(versions).to_string().into_bytes();
        self.versions.insert(slug.to_string(), (200, body));
        self
    }

    fn with_download(mut self, url: &str, bytes: &[u8]) -> Self {
        self.downloads
            .insert(url.to_string(), (200, bytes.to_vec()));
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
        let prefix = format!("{BASE}/v2/project/");
        if let Some(rest) = url.strip_prefix(&prefix) {
            if let Some(idx) = rest.find("/version") {
                let slug = &rest[..idx];
                let (status, body) = self.versions.get(slug).unwrap_or_else(|| {
                    panic!("{what}: no fake versions response for slug {slug} (url {url})")
                });
                return Ok(RawResponse {
                    status: *status,
                    body: body.clone(),
                });
            }
            let (status, body) = self.projects.get(rest).unwrap_or_else(|| {
                panic!("{what}: no fake project response for id {rest} (url {url})")
            });
            return Ok(RawResponse {
                status: *status,
                body: body.clone(),
            });
        }
        let (status, body) = self
            .downloads
            .get(url)
            .unwrap_or_else(|| panic!("{what}: no fake download response for {url}"));
        Ok(RawResponse {
            status: *status,
            body: body.clone(),
        })
    }

    fn post_json(
        &self,
        _url: &str,
        what: &str,
        _body: &serde_json::Value,
        _headers: &[(&str, &str)],
        _max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!("{what}: install_required_dependencies never POSTs");
    }
}

fn no_cancel() -> impl Fn() -> bool {
    || false
}

fn root_version(dependencies: Vec<ModrinthDependency>) -> ModrinthVersionInfo {
    ModrinthVersionInfo {
        id: "root-version".to_string(),
        project_id: "root-project".to_string(),
        name: String::new(),
        version_number: "1.0.0".to_string(),
        version_type: String::new(),
        game_versions: Vec::new(),
        loaders: Vec::new(),
        date_published: None,
        files: Vec::new(),
        dependencies,
    }
}

fn dep(project_id: Option<&str>, dependency_type: &str) -> ModrinthDependency {
    ModrinthDependency {
        project_id: project_id.map(str::to_string),
        dependency_type: dependency_type.to_string(),
    }
}

fn required(project_id: &str) -> ModrinthDependency {
    dep(Some(project_id), "required")
}

fn project_json(id: &str, slug: &str) -> serde_json::Value {
    serde_json::json!({ "id": id, "slug": slug, "title": slug })
}

fn version_json(
    id: &str,
    project_id: &str,
    files: Vec<serde_json::Value>,
    dependencies: Vec<serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "project_id": project_id,
        "version_number": "1.0.0",
        "files": files,
        "dependencies": dependencies,
    })
}

fn file_json(url: &str, filename: &str) -> serde_json::Value {
    serde_json::json!({
        "url": url,
        "filename": filename,
        "primary": true,
        "hashes": {},
        "size": 0,
    })
}

fn dep_json(project_id: Option<&str>, dependency_type: &str) -> serde_json::Value {
    serde_json::json!({ "project_id": project_id, "dependency_type": dependency_type })
}

// --- guards that skip the whole call before any network access ---

#[test]
fn empty_required_list_returns_immediately() {
    let transport = FakeTransport::new();
    let fs = FakeFileSystem::new();
    let report = addon_dependencies::install_required_dependencies(
        &transport,
        &fs,
        &root_version(vec![]),
        JavaServerFlavor::Fabric,
        None,
        Path::new("server"),
        &[],
        &no_cancel(),
    );
    assert!(report.results.is_empty());
    assert!(!report.cancelled);
}

#[test]
fn no_add_on_kind_server_returns_without_processing() {
    let transport = FakeTransport::new();
    let fs = FakeFileSystem::new();
    // Vanilla has no add-on kind at all -- required.isEmpty is false here
    // (one dependency present), so this proves the addOnKind guard, not
    // the empty-list guard.
    let report = addon_dependencies::install_required_dependencies(
        &transport,
        &fs,
        &root_version(vec![required("P1")]),
        JavaServerFlavor::Vanilla,
        None,
        Path::new("server"),
        &[],
        &no_cancel(),
    );
    assert!(report.results.is_empty());
}

#[test]
fn only_required_dependency_type_installed_optional_skipped() {
    let transport = FakeTransport::new()
        .with_project("P1", 200, project_json("P1", "p1-slug"))
        .with_versions("p1-slug", vec![]); // no compatible version, kept simple
    let fs = FakeFileSystem::new();

    let report = addon_dependencies::install_required_dependencies(
        &transport,
        &fs,
        &root_version(vec![
            dep(Some("P1"), "required"),
            dep(Some("P2"), "optional"),
            dep(Some("P3"), "incompatible"),
            dep(Some("P4"), "embedded"),
        ]),
        JavaServerFlavor::Fabric,
        None,
        Path::new("server"),
        &[],
        &no_cancel(),
    );

    // P2/P3/P4 never even reach a lookup -- no fake response was
    // registered for them, so a panic would surface here if they had.
    assert_eq!(report.results.len(), 1);
    assert_eq!(report.results[0].project_id, "P1");
    assert_eq!(
        report.results[0].outcome,
        DependencyInstallOutcome::NoCompatibleVersion
    );
}

#[test]
fn dependency_without_project_id_skipped() {
    let transport = FakeTransport::new()
        .with_project("P2", 200, project_json("P2", "p2-slug"))
        .with_versions("p2-slug", vec![]);
    let fs = FakeFileSystem::new();

    let report = addon_dependencies::install_required_dependencies(
        &transport,
        &fs,
        &root_version(vec![dep(None, "required"), dep(Some("P2"), "required")]),
        JavaServerFlavor::Fabric,
        None,
        Path::new("server"),
        &[],
        &no_cancel(),
    );

    assert_eq!(report.results.len(), 1);
    assert_eq!(report.results[0].project_id, "P2");
}

// --- already-present checks short-circuit before any version lookup ---

#[test]
fn already_installed_by_mod_id_match_skipped_not_redownloaded() {
    let transport = FakeTransport::new().with_project("P1", 200, project_json("P1", "fabric-api"));
    let fs = FakeFileSystem::new();

    let report = addon_dependencies::install_required_dependencies(
        &transport,
        &fs,
        &root_version(vec![required("P1")]),
        JavaServerFlavor::Fabric,
        None,
        Path::new("server"),
        &["fabric-api".to_string()],
        &no_cancel(),
    );

    assert_eq!(report.results.len(), 1);
    assert_eq!(report.results[0].outcome, DependencyInstallOutcome::Skipped);
}

#[test]
fn already_present_by_filename_slug_scan_skipped() {
    let transport =
        FakeTransport::new().with_project("P1", 200, project_json("P1", "cloth-config"));
    let fs = FakeFileSystem::new().with_file(
        "server/mods/cloth-config-api-11.1.118-fabric.jar",
        b"bytes".to_vec(),
        false,
    );

    let report = addon_dependencies::install_required_dependencies(
        &transport,
        &fs,
        &root_version(vec![required("P1")]),
        JavaServerFlavor::Fabric,
        None,
        Path::new("server"),
        &[],
        &no_cancel(),
    );

    assert_eq!(report.results.len(), 1);
    assert_eq!(report.results[0].outcome, DependencyInstallOutcome::Skipped);
}

// --- no compatible version, continuing to the next sibling ---

#[test]
fn no_compatible_version_found_logs_and_continues_not_fatal() {
    let transport = FakeTransport::new()
        .with_project("P1", 200, project_json("P1", "no-version-mod"))
        .with_versions("no-version-mod", vec![])
        .with_project("P2", 200, project_json("P2", "good-mod"))
        .with_versions(
            "good-mod",
            vec![version_json(
                "v1",
                "P2",
                vec![file_json("https://cdn.example/good.jar", "good.jar")],
                vec![],
            )],
        )
        .with_download("https://cdn.example/good.jar", b"good-bytes");
    let fs = FakeFileSystem::new();

    let report = addon_dependencies::install_required_dependencies(
        &transport,
        &fs,
        &root_version(vec![required("P1"), required("P2")]),
        JavaServerFlavor::Fabric,
        None,
        Path::new("server"),
        &[],
        &no_cancel(),
    );

    assert_eq!(report.results.len(), 2);
    assert_eq!(
        report.results[0].outcome,
        DependencyInstallOutcome::NoCompatibleVersion
    );
    assert!(matches!(
        report.results[1].outcome,
        DependencyInstallOutcome::Installed { .. }
    ));
    assert_eq!(
        fs.read(Path::new("server/mods/good.jar")).unwrap(),
        b"good-bytes"
    );
}

#[test]
fn no_primary_file_on_best_version_also_treated_as_no_compatible() {
    let transport = FakeTransport::new()
        .with_project("P1", 200, project_json("P1", "weird-mod"))
        .with_versions("weird-mod", vec![version_json("v1", "P1", vec![], vec![])]);
    let fs = FakeFileSystem::new();

    let report = addon_dependencies::install_required_dependencies(
        &transport,
        &fs,
        &root_version(vec![required("P1")]),
        JavaServerFlavor::Fabric,
        None,
        Path::new("server"),
        &[],
        &no_cancel(),
    );

    assert_eq!(
        report.results[0].outcome,
        DependencyInstallOutcome::NoCompatibleVersion
    );
}

// --- per-dependency failure never aborts the batch ---

#[test]
fn per_dependency_failure_logs_and_continues_not_fatal() {
    let transport = FakeTransport::new()
        .with_project("WILL_FAIL", 500, serde_json::json!({}))
        .with_project("WILL_SUCCEED", 200, project_json("WILL_SUCCEED", "ok-mod"))
        .with_versions(
            "ok-mod",
            vec![version_json(
                "v1",
                "WILL_SUCCEED",
                vec![file_json("https://cdn.example/ok.jar", "ok.jar")],
                vec![],
            )],
        )
        .with_download("https://cdn.example/ok.jar", b"ok-bytes");
    let fs = FakeFileSystem::new();

    let report = addon_dependencies::install_required_dependencies(
        &transport,
        &fs,
        &root_version(vec![required("WILL_FAIL"), required("WILL_SUCCEED")]),
        JavaServerFlavor::Fabric,
        None,
        Path::new("server"),
        &[],
        &no_cancel(),
    );

    assert_eq!(report.results.len(), 2);
    assert!(matches!(
        report.results[0].outcome,
        DependencyInstallOutcome::Failed(_)
    ));
    assert!(matches!(
        report.results[1].outcome,
        DependencyInstallOutcome::Installed { .. }
    ));
    assert_eq!(
        fs.read(Path::new("server/mods/ok.jar")).unwrap(),
        b"ok-bytes"
    );
}

// --- recursion: successful install resolves its own transitive deps ---

#[test]
fn successful_install_recurses_for_transitive_dependencies() {
    let transport = FakeTransport::new()
        .with_project("A", 200, project_json("A", "mod-a"))
        .with_versions(
            "mod-a",
            vec![version_json(
                "va",
                "A",
                vec![file_json("https://cdn.example/a.jar", "a.jar")],
                vec![dep_json(Some("B"), "required")],
            )],
        )
        .with_download("https://cdn.example/a.jar", b"A-bytes")
        .with_project("B", 200, project_json("B", "mod-b"))
        .with_versions(
            "mod-b",
            vec![version_json(
                "vb",
                "B",
                vec![file_json("https://cdn.example/b.jar", "b.jar")],
                vec![],
            )],
        )
        .with_download("https://cdn.example/b.jar", b"B-bytes");
    let fs = FakeFileSystem::new();

    let report = addon_dependencies::install_required_dependencies(
        &transport,
        &fs,
        &root_version(vec![required("A")]),
        JavaServerFlavor::Fabric,
        None,
        Path::new("server"),
        &[],
        &no_cancel(),
    );

    // Parent-before-child: A's own result is recorded before its
    // recursive child B's.
    assert_eq!(report.results.len(), 2);
    assert_eq!(report.results[0].project_id, "A");
    assert_eq!(report.results[1].project_id, "B");
    assert_eq!(fs.read(Path::new("server/mods/a.jar")).unwrap(), b"A-bytes");
    assert_eq!(fs.read(Path::new("server/mods/b.jar")).unwrap(), b"B-bytes");
}

// --- diamond dependency: not prevented by depth, prevented by the
// already-present check running before every recursive install ---

#[test]
fn diamond_dependency_both_parents_check_already_present_before_recursive_install() {
    let transport = FakeTransport::new()
        .with_project("B", 200, project_json("B", "mod-b"))
        .with_versions(
            "mod-b",
            vec![version_json(
                "vb",
                "B",
                vec![file_json("https://cdn.example/b.jar", "b.jar")],
                vec![dep_json(Some("D"), "required")],
            )],
        )
        .with_download("https://cdn.example/b.jar", b"B-bytes")
        .with_project("C", 200, project_json("C", "mod-c"))
        .with_versions(
            "mod-c",
            vec![version_json(
                "vc",
                "C",
                vec![file_json("https://cdn.example/c.jar", "c.jar")],
                vec![dep_json(Some("D"), "required")],
            )],
        )
        .with_download("https://cdn.example/c.jar", b"C-bytes")
        // D's project is fetched by both branches (the already-present
        // check needs D's own slug first), but D's own version list is
        // registered only ONCE below -- if the code fetched it a second
        // time (installing D twice) this test would panic instead of
        // asserting a wrong count.
        .with_project("D", 200, project_json("D", "mod-d"))
        .with_versions(
            "mod-d",
            vec![version_json(
                "vd",
                "D",
                vec![file_json("https://cdn.example/d.jar", "mod-d.jar")],
                vec![],
            )],
        )
        .with_download("https://cdn.example/d.jar", b"D-bytes");
    let fs = FakeFileSystem::new();

    let report = addon_dependencies::install_required_dependencies(
        &transport,
        &fs,
        &root_version(vec![required("B"), required("C")]),
        JavaServerFlavor::Fabric,
        None,
        Path::new("server"),
        &[],
        &no_cancel(),
    );

    let installed_d = report
        .results
        .iter()
        .filter(|r| {
            r.project_id == "D" && matches!(r.outcome, DependencyInstallOutcome::Installed { .. })
        })
        .count();
    let skipped_d = report
        .results
        .iter()
        .filter(|r| r.project_id == "D" && r.outcome == DependencyInstallOutcome::Skipped)
        .count();
    assert_eq!(installed_d, 1, "D must be downloaded exactly once");
    assert_eq!(
        skipped_d, 1,
        "the second parent must find D already present"
    );
}

// --- cycle detection: A -> B -> A terminates without an infinite loop ---

#[test]
fn cycle_a_b_a_is_detected_and_terminates() {
    let transport = FakeTransport::new()
        .with_project("A", 200, project_json("A", "mod-a"))
        .with_versions(
            "mod-a",
            vec![version_json(
                "va",
                "A",
                vec![file_json("https://cdn.example/a.jar", "mod-a.jar")],
                vec![dep_json(Some("B"), "required")],
            )],
        )
        .with_download("https://cdn.example/a.jar", b"A-bytes")
        .with_project("B", 200, project_json("B", "mod-b"))
        .with_versions(
            "mod-b",
            vec![version_json(
                "vb",
                "B",
                vec![file_json("https://cdn.example/b.jar", "mod-b.jar")],
                vec![dep_json(Some("A"), "required")],
            )],
        )
        .with_download("https://cdn.example/b.jar", b"B-bytes");
    let fs = FakeFileSystem::new();

    let report = addon_dependencies::install_required_dependencies(
        &transport,
        &fs,
        &root_version(vec![required("A")]),
        JavaServerFlavor::Fabric,
        None,
        Path::new("server"),
        &[],
        &no_cancel(),
    );

    assert_eq!(report.results.len(), 3);
    assert_eq!(report.results[0].project_id, "A");
    assert!(matches!(
        report.results[0].outcome,
        DependencyInstallOutcome::Installed { .. }
    ));
    assert_eq!(report.results[1].project_id, "B");
    assert!(matches!(
        report.results[1].outcome,
        DependencyInstallOutcome::Installed { .. }
    ));
    assert_eq!(report.results[2].project_id, "A");
    assert_eq!(
        report.results[2].outcome,
        DependencyInstallOutcome::CycleDetected
    );
}

// --- cancellation stops resolution and rolls back this operation's own
// newly-installed files ---

#[test]
fn cancellation_between_dependencies_rolls_back_installed_files() {
    let transport = FakeTransport::new()
        .with_project("A", 200, project_json("A", "mod-a"))
        .with_versions(
            "mod-a",
            vec![version_json(
                "va",
                "A",
                vec![file_json("https://cdn.example/a.jar", "a.jar")],
                vec![],
            )],
        )
        .with_download("https://cdn.example/a.jar", b"A-bytes");
    let fs = FakeFileSystem::new();

    let calls = Cell::new(0u32);
    let should_cancel = || {
        calls.set(calls.get() + 1);
        calls.get() > 1
    };

    let report = addon_dependencies::install_required_dependencies(
        &transport,
        &fs,
        &root_version(vec![required("A"), required("B")]),
        JavaServerFlavor::Fabric,
        None,
        Path::new("server"),
        &[],
        &should_cancel,
    );

    assert!(report.cancelled);
    // B was never reached at all -- cancellation is checked before each
    // sibling's own work begins, and no fake response was registered for
    // B (a fetch attempt would have panicked).
    assert_eq!(report.results.len(), 1);
    assert_eq!(report.results[0].project_id, "A");
    assert!(
        fs.read(Path::new("server/mods/a.jar")).is_err(),
        "A's own file must be rolled back once the operation is cancelled"
    );
}
