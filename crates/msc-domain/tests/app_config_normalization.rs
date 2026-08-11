//! Port of the 9 fixtures in `fixtures/app-config-normalization/`:
//! `AppConfig.init(from:)`'s decode-time normalization pass (P5.5), plus
//! the concrete `has_shown_welcome_guide` renamed-field case. None of
//! these fixtures were extracted from an existing Swift XCTest -- no
//! MSC 1 test exercises this pass directly -- so each was characterized
//! straight from `AppConfig.swift`'s `init(from:)` and each fixture's
//! `source` field points at the relevant source line(s) rather than a
//! test function.

mod support;

use msc_domain::app_config_schema::AppConfig;
use serde_json::Value;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("app-config-normalization/{case}.json")))
}

fn decode(json: &Value) -> AppConfig {
    let defaults = AppConfig::default_config("/tmp/test-servers");
    AppConfig::decode(json, &defaults).expect("decode should succeed")
}

#[test]
fn preferred_pairing_host_trim_and_blank_becomes_nil() {
    let fixture = load("preferred-pairing-host-trim-and-blank-becomes-nil");
    let input_cases = fixture.input["cases"].as_array().unwrap();
    let expected_cases = fixture.expected["cases"].as_array().unwrap();
    assert_eq!(input_cases.len(), expected_cases.len());

    for (input_case, expected_case) in input_cases.iter().zip(expected_cases) {
        let decoded = decode(&input_case["json"]);
        assert_eq!(
            decoded.remote_api_preferred_pairing_host.as_deref(),
            expected_case["remoteApiPreferredPairingHost"].as_str()
        );
    }
}

#[test]
fn shared_access_trims_label_and_token() {
    let fixture = load("shared-access-trims-label-and-token");
    let decoded = decode(&fixture.input["json"]);

    assert_eq!(decoded.remote_api_shared_access.len(), 1);
    let entry = &decoded.remote_api_shared_access[0];
    let expected = &fixture.expected["entries"][0];
    assert_eq!(entry.id, expected["id"].as_str().unwrap());
    assert_eq!(entry.label, expected["label"].as_str().unwrap());
    assert_eq!(entry.token, expected["token"].as_str().unwrap());
}

#[test]
fn shared_access_blank_id_gets_generated_id() {
    let fixture = load("shared-access-blank-id-gets-generated-id");
    let decoded = decode(&fixture.input["json"]);

    let expected = &fixture.expected;
    assert_eq!(
        expected["entryCount"].as_i64().unwrap() as usize,
        decoded.remote_api_shared_access.len()
    );
    let entry = &decoded.remote_api_shared_access[0];
    assert_eq!(entry.label, expected["label"].as_str().unwrap());
    assert_eq!(entry.token, expected["token"].as_str().unwrap());

    let blank_input_id = expected["idIsBlankInInput"].as_str().unwrap();
    assert_ne!(
        entry.id, blank_input_id,
        "a blank id must not survive decode unchanged"
    );
    assert!(
        !entry.id.trim().is_empty(),
        "generated id must be non-blank"
    );
}

#[test]
fn shared_access_drops_blank_token_entry() {
    let fixture = load("shared-access-drops-blank-token-entry");
    let decoded = decode(&fixture.input["json"]);

    let expected = &fixture.expected;
    assert_eq!(
        decoded.remote_api_shared_access.len(),
        expected["remainingCount"].as_i64().unwrap() as usize
    );
    let remaining_tokens: Vec<&str> = decoded
        .remote_api_shared_access
        .iter()
        .map(|e| e.token.as_str())
        .collect();
    let expected_tokens: Vec<&str> = expected["remainingTokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(remaining_tokens, expected_tokens);
}

#[test]
fn shared_access_dedupes_by_token_keeps_first() {
    let fixture = load("shared-access-dedupes-by-token-keeps-first");
    let decoded = decode(&fixture.input["json"]);

    let expected = &fixture.expected;
    assert_eq!(
        decoded.remote_api_shared_access.len(),
        expected["remainingCount"].as_i64().unwrap() as usize
    );
    let kept = &decoded.remote_api_shared_access[0];
    assert_eq!(kept.id, expected["keptId"].as_str().unwrap());
    assert_eq!(kept.label, expected["keptLabel"].as_str().unwrap());
    assert_eq!(kept.role, expected["keptRole"].as_str().unwrap());
}

#[test]
fn duplicate_server_ids_preserved() {
    let fixture = load("duplicate-server-ids-preserved");
    let decoded = decode(&fixture.input["json"]);

    let expected = &fixture.expected;
    assert_eq!(
        decoded.servers.len(),
        expected["serverCount"].as_i64().unwrap() as usize
    );
    let ids: Vec<&str> = decoded.servers.iter().map(|s| s.id.as_str()).collect();
    let expected_ids: Vec<&str> = expected["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(ids, expected_ids);
    let names: Vec<&str> = decoded
        .servers
        .iter()
        .map(|s| s.display_name.as_str())
        .collect();
    let expected_names: Vec<&str> = expected["displayNames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(names, expected_names);
}

#[test]
fn duplicate_server_standardized_paths_preserved() {
    let fixture = load("duplicate-server-standardized-paths-preserved");
    let decoded = decode(&fixture.input["json"]);

    let expected = &fixture.expected;
    assert_eq!(
        decoded.servers.len(),
        expected["serverCount"].as_i64().unwrap() as usize
    );
    let dirs: Vec<&str> = decoded
        .servers
        .iter()
        .map(|s| s.server_dir.as_str())
        .collect();
    let expected_dirs: Vec<&str> = expected["serverDirs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(dirs, expected_dirs);
    let ids: Vec<&str> = decoded.servers.iter().map(|s| s.id.as_str()).collect();
    let expected_ids: Vec<&str> = expected["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(ids, expected_ids);
}

#[test]
fn server_id_path_conflict_preserved() {
    let fixture = load("server-id-path-conflict-preserved");
    let decoded = decode(&fixture.input["json"]);

    let expected = &fixture.expected;
    assert_eq!(
        decoded.servers.len(),
        expected["serverCount"].as_i64().unwrap() as usize
    );
    let first = &decoded.servers[0];
    assert_eq!(first.id, expected["firstServer"]["id"].as_str().unwrap());
    assert_eq!(
        first.server_dir,
        expected["firstServer"]["serverDir"].as_str().unwrap()
    );
    let second = &decoded.servers[1];
    assert_eq!(second.id, expected["secondServer"]["id"].as_str().unwrap());
    assert_eq!(
        second.server_dir,
        expected["secondServer"]["serverDir"].as_str().unwrap()
    );
}

#[test]
fn has_shown_welcome_guide_renames_to_has_shown_handbook() {
    let fixture = load("has-shown-welcome-guide-renames-to-has-shown-handbook");
    let input_cases = fixture.input["cases"].as_array().unwrap();
    let expected_cases = fixture.expected["cases"].as_array().unwrap();
    assert_eq!(input_cases.len(), expected_cases.len());

    for (input_case, expected_case) in input_cases.iter().zip(expected_cases) {
        let decoded = decode(&input_case["json"]);
        assert_eq!(
            decoded.has_shown_handbook,
            expected_case["hasShownHandbook"].as_bool().unwrap()
        );
    }
}
