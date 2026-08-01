//! Port of `NetworkSafetyTests.swift`. One test per `fixtures/network-safety/`
//! case, so a failing case names itself in `cargo nextest run` output the
//! same way a failing Python fixture does.
//!
//! Test functions are prefixed `network_safety_` so the plan's Verify
//! command (a plain nextest substring filter, which matches on test name,
//! not file/binary name) selects all of them.

mod support;

use msc_domain::network_safety::is_local_or_private_host;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("network-safety/{case}.json")))
}

/// Reads a `{"host": "..."}` input paired with a plain boolean `expected`.
fn assert_single(case: &str) {
    let fixture = load(case);
    let host = fixture.input["host"].as_str().expect("host");
    let expected = fixture.expected.as_bool().expect("bool expected");
    assert_eq!(is_local_or_private_host(host), expected, "case: {case}");
}

/// Reads a `{"hosts": [...]}` input paired with a `{"results": [bool, ...]}`
/// expected shape.
fn assert_batch(case: &str) {
    let fixture = load(case);
    let hosts = fixture.input["hosts"].as_array().expect("hosts array");
    let expected: Vec<bool> = fixture.expected["results"]
        .as_array()
        .expect("results array")
        .iter()
        .map(|v| v.as_bool().expect("bool result"))
        .collect();
    let actual: Vec<bool> = hosts
        .iter()
        .map(|h| is_local_or_private_host(h.as_str().expect("host")))
        .collect();
    assert_eq!(actual, expected, "case: {case}");
}

#[test]
fn network_safety_loopback_and_localhost() {
    assert_batch("loopback-and-localhost");
}

#[test]
fn network_safety_mdns_local_suffix() {
    assert_single("mdns-local-suffix");
}

#[test]
fn network_safety_private_class_a_10() {
    assert_single("private-class-a-10");
}

#[test]
fn network_safety_private_class_c_192_168() {
    assert_single("private-class-c-192-168");
}

#[test]
fn network_safety_172_private_range_boundaries() {
    assert_batch("172-private-range-boundaries");
}

#[test]
fn network_safety_link_local_169_254() {
    assert_single("link-local-169-254");
}

#[test]
fn network_safety_public_addresses_rejected() {
    assert_batch("public-addresses-rejected");
}

#[test]
fn network_safety_empty_rejected() {
    assert_batch("empty-rejected");
}

#[test]
fn network_safety_ipv6_loopback() {
    assert_single("ipv6-loopback");
}

#[test]
fn network_safety_ipv6_link_local() {
    assert_batch("ipv6-link-local");
}

#[test]
fn network_safety_ipv6_ula() {
    assert_batch("ipv6-ula");
}

#[test]
fn network_safety_ipv6_public_rejected() {
    assert_batch("ipv6-public-rejected");
}

#[test]
fn network_safety_tailscale_magic_dns() {
    assert_batch("tailscale-magic-dns");
}
