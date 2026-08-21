//! P8.13's own tests. Every provider-logic test here runs against
//! [`FakeTransport`], fed from `corpus/addons/`'s real recorded responses
//! (P8.3) where a matching capture exists, and a synthetic body otherwise
//! (the batch endpoints `version_files`/`version_files/update`/`projects`
//! have no single-response real capture to point at, only per-item ones) --
//! never a real request to any of the four providers, per the same
//! "provisioning tests never touch the network" precedent
//! `tests/jar_provider.rs` established in P7.13.
//!
//! The one exception, `addon_provider_http_transport_enforces_size_cap_and_status_passthrough`,
//! spins up a real local loopback HTTP server to prove [`HttpTransport`]'s
//! own bounding behavior and its `http_status_as_error(false)` status
//! passthrough -- testing this crate's own transport code against bytes it
//! controls, not touching the network in the sense the precedent guards
//! against.

use msc_infrastructure::addon_provider::{
    self, AddonTransport, CURSEFORGE_API_KEY_SECRET, HttpTransport, RawResponse, TransportError,
};
use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/addons")
}

struct FakeCall {
    status: u16,
    body: Vec<u8>,
}

struct FakeTransport {
    responses: Mutex<HashMap<String, FakeCall>>,
}

impl FakeTransport {
    fn new() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
        }
    }

    fn with_file(self, url: &str, relative_corpus_path: &str) -> Self {
        self.with_file_status(url, relative_corpus_path, 200)
    }

    fn with_file_status(self, url: &str, relative_corpus_path: &str, status: u16) -> Self {
        let bytes = std::fs::read(corpus_dir().join(relative_corpus_path))
            .unwrap_or_else(|e| panic!("reading {relative_corpus_path}: {e}"));
        self.responses.lock().unwrap().insert(
            url.to_string(),
            FakeCall {
                status,
                body: bytes,
            },
        );
        self
    }

    fn with_json(self, url: &str, status: u16, body: serde_json::Value) -> Self {
        self.responses.lock().unwrap().insert(
            url.to_string(),
            FakeCall {
                status,
                body: serde_json::to_vec(&body).unwrap(),
            },
        );
        self
    }

    fn with_status(self, url: &str, status: u16) -> Self {
        self.responses.lock().unwrap().insert(
            url.to_string(),
            FakeCall {
                status,
                body: Vec::new(),
            },
        );
        self
    }
}

impl AddonTransport for FakeTransport {
    fn get(
        &self,
        url: &str,
        what: &str,
        _headers: &[(&str, &str)],
        max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        let responses = self.responses.lock().unwrap();
        let call = responses
            .get(url)
            .unwrap_or_else(|| panic!("{what}: no fake response registered for {url}"));
        if call.body.len() as u64 > max_bytes {
            return Err(TransportError::ResponseTooLarge {
                what: what.to_string(),
                max_bytes,
            });
        }
        Ok(RawResponse {
            status: call.status,
            body: call.body.clone(),
        })
    }

    fn post_json(
        &self,
        url: &str,
        what: &str,
        _body: &serde_json::Value,
        _headers: &[(&str, &str)],
        max_bytes: u64,
    ) -> Result<RawResponse, TransportError> {
        self.get(url, what, &[], max_bytes)
    }
}

// --- Modrinth ---

#[test]
fn addon_provider_modrinth_search_against_real_corpus() {
    let url = format!(
        "https://api.modrinth.com/v2/search?query=&facets={}&index=downloads&limit=5&offset=0",
        urlencoding_test_helper("[[\"project_type:mod\"],[\"categories:fabric\"]]")
    );
    let transport = FakeTransport::new().with_file(&url, "modrinth/search-sodium.json");
    let result =
        addon_provider::modrinth_search(&transport, "", "mod", &["fabric".to_string()], None, 5, 0)
            .unwrap();
    assert!(result.hits.iter().any(|h| h.slug == "sodium"));
}

#[test]
fn addon_provider_modrinth_version_from_hash_real_capture() {
    let hash = "a7fbb629793c52f0be8b049f787cb598879239b1ad8e1de62e103c8b9efff140e3232b93ef1f14e505d262897d8cf9505b1126396429ad4056bff969c8674e52";
    let url = format!("https://api.modrinth.com/v2/version_file/{hash}?algorithm=sha512");
    let transport = FakeTransport::new().with_file(&url, "modrinth/version-file-hash-iris.json");
    let info = addon_provider::modrinth_version_from_hash(&transport, hash)
        .unwrap()
        .unwrap();
    assert_eq!(info.project_id, "YL57xq9U");
}

#[test]
fn addon_provider_modrinth_version_from_hash_404_returns_none() {
    let transport = FakeTransport::new().with_status(
        "https://api.modrinth.com/v2/version_file/deadbeef?algorithm=sha512",
        404,
    );
    let info = addon_provider::modrinth_version_from_hash(&transport, "deadbeef").unwrap();
    assert!(info.is_none());
}

#[test]
fn addon_provider_modrinth_versions_from_hashes_empty_input_makes_no_request() {
    let transport = FakeTransport::new();
    let result = addon_provider::modrinth_versions_from_hashes(&transport, &[]).unwrap();
    assert!(result.is_empty());
}

#[test]
fn addon_provider_modrinth_versions_from_hashes_batch_decodes_hash_keyed_map() {
    let transport = FakeTransport::new().with_json(
        "https://api.modrinth.com/v2/version_files",
        200,
        serde_json::json!({
            "abc123": {
                "id": "bAo1Qhte",
                "project_id": "YL57xq9U",
                "version_number": "1.8.14",
                "files": []
            }
        }),
    );
    let result =
        addon_provider::modrinth_versions_from_hashes(&transport, &["abc123".to_string()]).unwrap();
    assert_eq!(result["abc123"].project_id, "YL57xq9U");
}

#[test]
fn addon_provider_modrinth_latest_versions_for_hashes_batch_decodes_hash_keyed_map() {
    let transport = FakeTransport::new().with_json(
        "https://api.modrinth.com/v2/version_files/update",
        200,
        serde_json::json!({
            "abc123": {
                "id": "newid",
                "project_id": "YL57xq9U",
                "version_number": "1.8.15",
                "files": []
            }
        }),
    );
    let result = addon_provider::modrinth_latest_versions_for_hashes(
        &transport,
        &["abc123".to_string()],
        &["fabric".to_string()],
        &["1.21.1".to_string()],
    )
    .unwrap();
    assert_eq!(result["abc123"].id, "newid");
}

#[test]
fn addon_provider_modrinth_projects_batch_empty_makes_no_request() {
    let transport = FakeTransport::new();
    let result = addon_provider::modrinth_projects(&transport, &[]).unwrap();
    assert!(result.is_empty());
}

#[test]
fn addon_provider_modrinth_projects_batch_decodes_array() {
    let url = format!(
        "https://api.modrinth.com/v2/projects?ids={}",
        urlencoding_test_helper("[\"YL57xq9U\"]")
    );
    let transport = FakeTransport::new().with_json(
        &url,
        200,
        serde_json::json!([{"id": "YL57xq9U", "title": "Iris"}]),
    );
    let result = addon_provider::modrinth_projects(&transport, &["YL57xq9U".to_string()]).unwrap();
    assert_eq!(result[0]["title"], "Iris");
}

// --- Hangar ---

#[test]
fn addon_provider_hangar_fetch_latest_real_corpus_fallback_url() {
    let url = "https://hangar.papermc.io/api/v1/projects/EssentialsX/Essentials/versions?platform=PAPER&channel=Release";
    let transport = FakeTransport::new().with_file(url, "hangar/versions-latest-essentials.json");
    let (version, download_url) =
        addon_provider::hangar_fetch_latest(&transport, "EssentialsX", "Essentials", None).unwrap();
    assert_eq!(version.name, "2.22.0");
    assert_eq!(
        download_url,
        "https://hangar.papermc.io/api/v1/projects/EssentialsX/Essentials/versions/2.22.0/PAPER/download"
    );
}

#[test]
fn addon_provider_hangar_fetch_latest_empty_result_errors() {
    let url =
        "https://hangar.papermc.io/api/v1/projects/A/B/versions?platform=PAPER&channel=Release";
    let transport = FakeTransport::new().with_json(url, 200, serde_json::json!({"result": []}));
    let err = addon_provider::hangar_fetch_latest(&transport, "A", "B", None).unwrap_err();
    assert!(
        err.to_string()
            .contains("No compatible version found on Hangar")
    );
}

// --- CurseForge ---

#[test]
fn addon_provider_curseforge_files_missing_key_errors_before_request() {
    let transport = FakeTransport::new();
    let secrets = FakeSecretStore::new();
    let err = addon_provider::curseforge_files(&transport, &secrets, &[8287121]).unwrap_err();
    assert!(
        err.to_string()
            .contains("No CurseForge API key is configured")
    );
}

#[test]
fn addon_provider_curseforge_files_real_corpus_blocked_file() {
    let secrets = FakeSecretStore::new();
    secrets.set(CURSEFORGE_API_KEY_SECRET, "test-key").unwrap();
    let transport = FakeTransport::new().with_file(
        "https://api.curseforge.com/v1/mods/files",
        "curseforge/mods-files-blocked-entityculling.json",
    );
    let files = addon_provider::curseforge_files(&transport, &secrets, &[8287121]).unwrap();
    assert_eq!(files[0].mod_id, 448233);
    assert!(files[0].download_url.is_none());
}

#[test]
fn addon_provider_curseforge_mods_real_corpus() {
    let secrets = FakeSecretStore::new();
    secrets.set(CURSEFORGE_API_KEY_SECRET, "test-key").unwrap();
    let transport = FakeTransport::new().with_file(
        "https://api.curseforge.com/v1/mods",
        "curseforge/mods-metadata-entityculling.json",
    );
    let mods = addon_provider::curseforge_mods(&transport, &secrets, &[448233]).unwrap();
    assert_eq!(mods[0].slug, "entityculling");
}

#[test]
fn addon_provider_curseforge_files_unauthorized_status_maps_to_domain_error() {
    let secrets = FakeSecretStore::new();
    secrets.set(CURSEFORGE_API_KEY_SECRET, "bad-key").unwrap();
    let transport =
        FakeTransport::new().with_status("https://api.curseforge.com/v1/mods/files", 403);
    let err = addon_provider::curseforge_files(&transport, &secrets, &[1]).unwrap_err();
    assert!(err.to_string().contains("CurseForge rejected the API key"));
}

// --- GitHub ---

#[test]
fn addon_provider_github_latest_release_real_corpus() {
    let transport = FakeTransport::new().with_file(
        "https://api.github.com/repos/EssentialsX/Essentials/releases/latest",
        "github/releases-latest-essentialsx.json",
    );
    let release =
        addon_provider::github_latest_release(&transport, "EssentialsX", "Essentials").unwrap();
    assert!(
        release
            .assets
            .iter()
            .any(|a| a.name == "EssentialsX-2.22.0.jar")
    );
}

#[test]
fn addon_provider_github_latest_release_non_2xx_status_errors() {
    let transport =
        FakeTransport::new().with_status("https://api.github.com/repos/x/y/releases/latest", 404);
    let err = addon_provider::github_latest_release(&transport, "x", "y").unwrap_err();
    assert!(err.to_string().contains("GitHub returned status 404"));
}

// --- HttpTransport's own bounding behavior (real loopback server) ---

#[test]
fn addon_provider_http_transport_enforces_size_cap_and_status_passthrough() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        for stream in listener.incoming().take(2) {
            let mut stream = stream.unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let body = b"a very small body but the caller's cap is even smaller";
            let response = format!(
                "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.write_all(body).unwrap();
        }
    });

    let transport = HttpTransport::new();
    let url = format!("http://{addr}/");

    // Status passthrough: a 404 is NOT turned into a transport error --
    // `http_status_as_error(false)` is what makes this possible.
    let resp = transport.get(&url, "test", &[], 4096).unwrap();
    assert_eq!(resp.status, 404);

    // Size cap: the same body, capped below its real length, errors.
    let err = transport.get(&url, "test", &[], 4).unwrap_err();
    assert!(matches!(err, TransportError::ResponseTooLarge { .. }));

    handle.join().unwrap();
}

#[test]
fn addon_provider_http_transport_timeout_is_bounded() {
    // A listener that accepts but never writes a response -- proves
    // REQUEST_TIMEOUT actually bounds the call rather than hanging
    // forever. Uses a short-timeout agent built the same way
    // `HttpTransport::new` does, just with a smaller duration so this
    // test doesn't take 20 real seconds.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            std::thread::sleep(Duration::from_secs(5));
            drop(stream);
        }
    });

    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_millis(200)))
        .http_status_as_error(false)
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let start = std::time::Instant::now();
    let result = agent.get(format!("http://{addr}/")).call();
    assert!(result.is_err());
    assert!(start.elapsed() < Duration::from_secs(2));

    handle.join().unwrap();
}

/// Mirrors `addon_provider`'s own private `urlencode` closely enough for
/// this test file to build the exact URL key `FakeTransport` needs to
/// match against -- not exported from the crate itself since it's an
/// internal request-building detail, not part of the module's public
/// contract.
fn urlencoding_test_helper(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
