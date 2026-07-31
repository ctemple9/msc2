//! Fixture-loading support shared by msc-domain's integration tests.
//!
//! Native counterpart to `tools/fixture-runner/run.py`: deserializes a
//! fixture file into the shape defined by `docs/msc2/fixture-format.md`
//! and compares a domain's computed `actual` against its `expected`.
//! Real domains register a `compute_actual` arm as their Rust port lands
//! (mirrors `run.py`'s `ACTUAL_COMPUTERS` table) — `_selftest` is the only
//! one wired so far, on purpose, since P1.2 exists to prove the harness
//! itself before any real domain uses it.

use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

// `case`, `source`, and `notes` aren't read by the P1.2 self-test — they
// exist because they're part of the fixture-format.md schema this struct
// deserializes, and P1.3+ tests (e.g. per-case test naming) will read them.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct FixtureSource {
    pub file: String,
    pub test: String,
    pub line: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Fixture {
    pub domain: String,
    pub case: String,
    pub source: FixtureSource,
    pub input: Value,
    pub expected: Value,
    #[serde(default)]
    pub notes: Option<String>,
}

/// The workspace's `fixtures/` directory, resolved from this crate's
/// manifest path so tests don't depend on the process's working directory.
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

pub fn load(path: impl AsRef<Path>) -> Fixture {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: could not read fixture: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{}: could not parse fixture JSON: {e}", path.display()))
}

/// `run.py`'s `ACTUAL_COMPUTERS` table. `_selftest` is computed as the
/// identity function on purpose (see `fixtures/_selftest/*.json`), so
/// `pass.json` matches and `fail.json` doesn't.
///
/// `support/mod.rs` is compiled once per integration-test binary (each
/// `tests/*.rs` file that does `mod support;` gets its own copy), and only
/// `fixture_harness.rs` calls this generic entry point — other test files
/// use `support::load` directly and assert with domain-specific logic. Hence
/// `#[allow(dead_code)]`: it's unused in those binaries, not unused overall.
#[allow(dead_code)]
fn compute_actual(fixture: &Fixture) -> Value {
    match fixture.domain.as_str() {
        "_selftest" => fixture.input.clone(),
        other => panic!(
            "no actual-computer registered for domain '{other}' (expected until its Rust port lands)"
        ),
    }
}

/// Mirrors `run.py`'s `full_compare`: true on a match, false on a mismatch.
#[allow(dead_code)]
pub fn full_compare(fixture: &Fixture) -> bool {
    compute_actual(fixture) == fixture.expected
}
