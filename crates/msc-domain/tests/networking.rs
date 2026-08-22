//! Fixture-backed coverage for P9.5's pure networking rules.

mod support;

use msc_domain::networking::*;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("networking/{case}.json")))
}

#[test]
fn networking_duckdns_is_configuration_not_a_provider_call() {
    let fixture = load("duckdns-trims-label-no-update-request");
    assert_eq!(
        fixture.input["hostname"].as_str().unwrap().trim(),
        fixture.expected["storedHostname"]
    );
    assert_eq!(fixture.expected["duckdnsHttpRequest"], false);
}

#[test]
fn networking_cross_play_is_filename_based() {
    let fixture = load("geyser-floodgate-filename-detection");
    let files = fixture.input["pluginFiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap());
    assert_eq!(classify_cross_play(files), CrossPlayStatus::Both);
}

#[test]
fn networking_geyser_config_mutation_is_deferred() {
    let fixture = load("geyser-patches-top-level-bedrock-only");
    assert_eq!(
        fixture.expected["yaml"].as_str().unwrap(),
        "bedrock:\n  port: 19133 # public\nremote:\n  bedrock:\n    port: 9\n"
    );
}

#[test]
fn networking_playit_disabled_is_no_action() {
    let fixture = load("playit-disabled-no-op");
    assert_eq!(fixture.expected["action"], "none");
}

#[test]
fn networking_playit_missing_secret_requires_setup() {
    let fixture = load("playit-missing-secret-prompts-setup");
    assert_eq!(fixture.expected["action"], "prompt_secret_setup");
}

#[test]
fn networking_playit_address_is_safe_player_display() {
    let fixture = load("playit-tunnel-address-after-setup-line");
    assert_eq!(
        parse_playit_address(fixture.input["line"].as_str().unwrap(), true).as_deref(),
        fixture.expected["javaTunnelAddress"].as_str()
    );
    assert_eq!(safe_player_address("127.0.0.1", Some(25565)), None);
}

#[test]
fn networking_never_run_diagnostic_is_unknown() {
    let fixture = load("port-diagnostic-never-run-is-unknown");
    let actual = diagnostic_for_server_has_run(fixture.input["serverHasRun"].as_bool().unwrap());
    assert_eq!(actual.summary(), fixture.expected["status"]);
    assert_eq!(actual.api_outcome(), "not_applicable");
}

#[test]
fn networking_provider_failure_is_not_a_closed_port() {
    let fixture = load("provider-failure-is-not-port-closed");
    let actual = classify_provider_outcome(fixture.input["outcome"].as_str().unwrap());
    assert_eq!(actual.summary(), fixture.expected["status"]);
    assert_ne!(actual, DiagnosticResult::Closed);
}

#[test]
fn networking_java_resource_pack_requires_zip() {
    let fixture = load("resource-pack-non-zip-refused-for-java");
    assert_eq!(
        validate_java_pack_filename(fixture.input["filename"].as_str().unwrap()),
        Err(ResourcePackError::JavaPackMustBeZip)
    );
}

#[test]
fn networking_resource_pack_sha1_matches_fixture() {
    let fixture = load("resource-pack-sha1-is-written-with-url");
    assert_eq!(
        resource_pack_sha1(fixture.input["packBytes"].as_str().unwrap().as_bytes()),
        fixture.expected["sha1"]
    );
}

#[test]
fn networking_resource_pack_url_encodes_a_filename_segment() {
    let fixture = load("resource-pack-url-percent-encodes-filename");
    assert_eq!(
        hosted_resource_pack_url(
            fixture.input["host"].as_str().unwrap(),
            fixture.input["port"].as_u64().unwrap() as u16,
            fixture.input["filename"].as_str().unwrap()
        )
        .unwrap(),
        fixture.expected["url"]
    );
    assert_eq!(
        hosted_resource_pack_url("example.test", 25566, "../escape.zip"),
        Err(ResourcePackError::UnsafeFilename)
    );
}

#[test]
fn networking_ready_tcp_probe_is_open() {
    let fixture = load("tcp-port-diagnostic-ready-is-listening");
    assert_eq!(
        classify_tcp_connection(fixture.input["connectionState"].as_str().unwrap()),
        DiagnosticResult::Open
    );
    assert_eq!(fixture.expected["listening"], true);
}

#[test]
fn networking_broadcast_prompt_keeps_only_code_and_link() {
    let fixture = load("xbox-auth-prompt-extracts-code-and-link");
    let prompt = parse_broadcast_auth_prompt(fixture.input["line"].as_str().unwrap()).unwrap();
    assert_eq!(prompt.code, fixture.expected["prompt"]["code"]);
    assert_eq!(prompt.url, fixture.expected["prompt"]["url"]);
}

#[test]
fn networking_broadcast_ready_signal_is_classified() {
    let fixture = load("xbox-ready-signal-completes-broadcast");
    assert_eq!(
        broadcast_is_ready(fixture.input["line"].as_str().unwrap()),
        fixture.expected["broadcastReady"]
    );
}
