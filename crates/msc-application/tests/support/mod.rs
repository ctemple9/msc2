//! Fixture-loading support shared by msc-application's integration tests.
//! Same shape as `msc-domain/tests/support/mod.rs` (per
//! `docs/msc2/fixture-format.md`) -- duplicated rather than shared via a
//! path dependency because cargo integration-test binaries can't import
//! another crate's `tests/` module directly.

use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

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
