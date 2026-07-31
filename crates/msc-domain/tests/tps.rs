//! Port of the pure-parser cases from `TpsMonitoringTests.swift`. One test
//! per `fixtures/tps/` case that exercises `TpsLineParser`. The 8 cases in
//! that domain testing `JavaServerFlavor` instead are P1.8's, not this
//! step's — see `rolling-plan.md` P1.4's amended scope note.
//!
//! Test functions are prefixed `tps_` so the plan's Verify command (a plain
//! nextest substring filter, which matches on test name, not file/binary
//! name) selects all of them.

mod support;

use msc_domain::tps::{Sample, is_spark_tps_header, parse, parse_spark_values};
use serde_json::Value;
use support::Fixture;

fn load(case: &str) -> Fixture {
    support::load(support::fixtures_dir().join(format!("tps/{case}.json")))
}

/// Compares an `Option<Sample>` against an expected shape that's either
/// `null`, or an object naming a subset of `t1`/`t5`/`t15` (some source
/// tests only assert a subset of the sample's fields).
fn assert_sample(case: &str, actual: Option<Sample>, expected: &Value) {
    match expected {
        Value::Null => assert!(
            actual.is_none(),
            "case {case}: expected None, got {actual:?}"
        ),
        Value::Object(obj) => {
            let actual = actual.unwrap_or_else(|| panic!("case {case}: expected Some, got None"));
            if let Some(v) = obj.get("t1") {
                assert_eq!(v.as_f64().expect("t1"), actual.t1, "case {case} t1");
            }
            if let Some(v) = obj.get("t5") {
                assert_eq!(v.as_f64(), actual.t5, "case {case} t5");
            }
            if let Some(v) = obj.get("t15") {
                assert_eq!(v.as_f64(), actual.t15, "case {case} t15");
            }
        }
        other => panic!("case {case}: unexpected expected shape: {other:?}"),
    }
}

fn assert_parse_single(case: &str) {
    let fixture = load(case);
    let line = fixture.input["line"].as_str().expect("line");
    assert_sample(case, parse(line), &fixture.expected);
}

fn assert_parse_array(case: &str) {
    let fixture = load(case);
    let lines = fixture.input["lines"].as_array().expect("lines array");
    let expected = fixture.expected["results"]
        .as_array()
        .expect("results array");
    assert_eq!(
        lines.len(),
        expected.len(),
        "case {case}: lines/results length mismatch"
    );
    for (line, exp) in lines.iter().zip(expected.iter()) {
        assert_sample(case, parse(line.as_str().expect("line")), exp);
    }
}

fn assert_spark_values_single(case: &str) {
    let fixture = load(case);
    let line = fixture.input["line"].as_str().expect("line");
    assert_sample(case, parse_spark_values(line), &fixture.expected);
}

fn assert_spark_values_array(case: &str) {
    let fixture = load(case);
    let lines = fixture.input["lines"].as_array().expect("lines array");
    let expected = fixture.expected["results"]
        .as_array()
        .expect("results array");
    assert_eq!(
        lines.len(),
        expected.len(),
        "case {case}: lines/results length mismatch"
    );
    for (line, exp) in lines.iter().zip(expected.iter()) {
        assert_sample(case, parse_spark_values(line.as_str().expect("line")), exp);
    }
}

// --- TpsLineParser.parse: Paper family ---

#[test]
fn tps_paper_tps_three_values() {
    assert_parse_single("paper-tps-three-values");
}

#[test]
fn tps_paper_tps_color_code_stripped_shape() {
    assert_parse_single("paper-tps-color-code-stripped-shape");
}

#[test]
fn tps_paper_tps_malformed_trio_returns_nil() {
    assert_parse_array("paper-tps-malformed-trio-returns-nil");
}

// --- TpsLineParser.parse: legacy Forge ---

#[test]
fn tps_forge_tps_overall_single_value() {
    assert_parse_single("forge-tps-overall-single-value");
}

#[test]
fn tps_forge_tps_degraded_tick_count() {
    assert_parse_single("forge-tps-degraded-tick-count");
}

#[test]
fn tps_forge_tps_case_and_spacing_tolerant() {
    assert_parse_single("forge-tps-case-and-spacing-tolerant");
}

// --- TpsLineParser.parse: modern NeoForge (MC 1.21+) ---

#[test]
fn tps_neoforge121_overall_single_value() {
    assert_parse_single("neoforge121-overall-single-value");
}

#[test]
fn tps_neoforge121_degraded_tps() {
    assert_parse_single("neoforge121-degraded-tps");
}

#[test]
fn tps_neoforge121_per_dimension_line_ignored() {
    assert_parse_single("neoforge121-per-dimension-line-ignored");
}

// --- TpsLineParser.parse: vanilla /tick query ---

#[test]
fn tps_vanilla_tick_healthy_derives_twenty() {
    assert_parse_single("vanilla-tick-healthy-derives-twenty");
}

#[test]
fn tps_vanilla_tick_at_budget_is_twenty() {
    assert_parse_single("vanilla-tick-at-budget-is-twenty");
}

#[test]
fn tps_vanilla_tick_overloaded_derives_reduced_tps() {
    assert_parse_single("vanilla-tick-overloaded-derives-reduced-tps");
}

#[test]
fn tps_vanilla_tick_with_server_prefix() {
    assert_parse_single("vanilla-tick-with-server-prefix");
}

#[test]
fn tps_vanilla_tick_sibling_lines_ignored() {
    assert_parse_array("vanilla-tick-sibling-lines-ignored");
}

// --- TpsLineParser.parse: garbage / neither format ---

#[test]
fn tps_garbage_lines_yield_nil() {
    assert_parse_array("garbage-lines-yield-nil");
}

// --- TpsLineParser spark helpers ---

#[test]
fn tps_spark_header_detection() {
    let fixture = load("spark-header-detection");
    let lines = fixture.input["lines"].as_array().expect("lines array");
    let expected: Vec<bool> = fixture.expected["results"]
        .as_array()
        .expect("results array")
        .iter()
        .map(|v| v.as_bool().expect("bool result"))
        .collect();
    let actual: Vec<bool> = lines
        .iter()
        .map(|l| is_spark_tps_header(l.as_str().expect("line")))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn tps_spark_values_map_windows_to_1m_5m_15m() {
    assert_spark_values_single("spark-values-map-windows-to-1m-5m-15m");
}

#[test]
fn tps_spark_values_tolerates_colour_codes_and_asterisks() {
    assert_spark_values_single("spark-values-tolerates-colour-codes-and-asterisks");
}

#[test]
fn tps_spark_values_needs_five_numbers() {
    assert_spark_values_array("spark-values-needs-five-numbers");
}
