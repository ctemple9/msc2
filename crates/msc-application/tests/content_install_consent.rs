//! Essential P24.11: explicit setup consent must not turn the ordinary install
//! path into an implicit bypass, or bypass unavailable releases. Existing local
//! staging/rollback tests never resolve provider metadata. Fixed in-memory
//! responses, no network/filesystem/global mutation; expected <10ms.
use msc_application::content_install::{self as installs, ReviewContext};
use msc_domain::identity::JavaServerFlavor;
use msc_infrastructure::addon_provider::{
    AddonTransport, CURSEFORGE_API_KEY_SECRET, RawResponse, TransportError,
};
use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};
use serde_json::json;
use std::collections::BTreeMap;
struct Catalog {
    available: bool,
}
impl AddonTransport for Catalog {
    fn get(
        &self,
        url: &str,
        _what: &str,
        _headers: &[(&str, &str)],
        _limit: u64,
    ) -> Result<RawResponse, TransportError> {
        let root = json!({"id":200,"modId":1,"isAvailable":self.available,"displayName":"Other Minecraft","fileName":"root.jar","fileDate":"2026-01-01","releaseType":1,"fileLength":10,"hashes":[{"algo":1,"value":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}],"gameVersions":["1.20.1","Fabric"],"dependencies":[{"modId":2,"relationType":3}]});
        let dependency = json!({"id":201,"modId":2,"isAvailable":true,"displayName":"Server dependency","fileName":"dependency.jar","fileDate":"2026-01-01","releaseType":1,"fileLength":10,"hashes":[{"algo":1,"value":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}],"gameVersions":["1.21.1","Fabric"],"dependencies":[]});
        let body = if url.ends_with("/mods/1/files/200") {
            json!({"data":root})
        } else if url.contains("/mods/2/files?") {
            assert!(url.contains("gameVersion=1.21.1"));
            json!({"data":[dependency],"pagination":{"index":0,"pageSize":50,"resultCount":1,"totalCount":1}})
        } else if url.ends_with("/mods/1") || url.ends_with("/mods/2") {
            let id = if url.ends_with("/1") { 1 } else { 2 };
            json!({"data":{"id":id,"gameId":432,"classId":6,"name":format!("Project {id}"),"slug":format!("project-{id}"),"summary":"","downloadCount":0,"links":{}}})
        } else {
            panic!("Unexpected metadata request {url}")
        };
        Ok(RawResponse {
            status: 200,
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
    fn post_json(
        &self,
        _url: &str,
        _what: &str,
        _body: &serde_json::Value,
        _headers: &[(&str, &str)],
        _limit: u64,
    ) -> Result<RawResponse, TransportError> {
        panic!("Review must not mutate a provider")
    }
}
#[test]
fn mismatch_requires_explicit_consent_and_keeps_dependencies_on_server_setup() {
    let secrets = FakeSecretStore::new();
    secrets
        .set(CURSEFORGE_API_KEY_SECRET, "fixture-key")
        .unwrap();
    let pins = BTreeMap::new();
    let context = || ReviewContext {
        flavor: JavaServerFlavor::Fabric,
        game_version: "1.21.1",
        installed: &pins,
    };
    assert!(
        installs::review_curseforge_with_pins(
            &Catalog { available: true },
            &secrets,
            1,
            200,
            context(),
            &|| false
        )
        .is_err()
    );
    let review = installs::review_curseforge_with_consent(
        &Catalog { available: true },
        &secrets,
        1,
        200,
        context(),
        true,
        &|| false,
    )
    .unwrap();
    assert_eq!(
        review
            .files
            .iter()
            .map(|file| file.version_id.as_str())
            .collect::<Vec<_>>(),
        vec!["200", "201"]
    );
    assert!(!review.warnings.is_empty());
    assert!(
        installs::review_curseforge_with_consent(
            &Catalog { available: false },
            &secrets,
            1,
            200,
            context(),
            true,
            &|| false
        )
        .is_err()
    );
}
