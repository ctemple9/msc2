//! Component version parsing and comparison.
//!
//! Ported from `ComponentVersionParsing.swift` and the `MCVersionComparator`
//! it also defines (MSC 1, U4b): filename parsing for Paper/Geyser/Floodgate
//! jars, and a pure dotted-integer version comparator used to detect
//! downgrades across both Java (3-part) and Bedrock (4-part) version strings.

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaperJarVersion {
    pub mc_version: String,
    pub build: i64,
}

impl PaperJarVersion {
    pub fn display_string(&self) -> String {
        format!("{} (build {})", self.mc_version, self.build)
    }

    pub fn compact_string(&self) -> String {
        format!("{}-{}", self.mc_version, self.build)
    }
}

fn deleting_path_extension(filename: &str) -> &str {
    Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename)
}

/// Swift's `split(separator:)` omits empty subsequences by default; Rust's
/// `str::split` does not, so callers that mirror Swift's behavior filter here.
fn split_omitting_empty(s: &str, separator: char) -> Vec<&str> {
    s.split(separator).filter(|p| !p.is_empty()).collect()
}

/// Parses common Paper template patterns:
/// - paper-<mc>-build<build>.jar
/// - paper-<mc>-<build>.jar
pub fn parse_paper_jar_filename(filename: &str) -> Option<PaperJarVersion> {
    let base = deleting_path_extension(filename);
    if !base.to_lowercase().starts_with("paper-") {
        return None;
    }

    let rest = &base["paper-".len()..];
    let parts = split_omitting_empty(rest, '-');
    if parts.len() < 2 {
        return None;
    }

    let mc_version = parts[0].to_string();
    let build_part = parts[1];

    if build_part.to_lowercase().starts_with("build") {
        let b = &build_part["build".len()..];
        if let Ok(build) = b.parse::<i64>() {
            return Some(PaperJarVersion { mc_version, build });
        }
    }

    if let Ok(build) = build_part.parse::<i64>() {
        return Some(PaperJarVersion { mc_version, build });
    }

    None
}

/// Extracts a trailing integer from names like:
/// - Geyser-spigot-1004.jar
/// - floodgate-spigot-121.jar
pub fn parse_trailing_build_number(filename: &str) -> Option<i64> {
    let base = deleting_path_extension(filename);
    let parts = split_omitting_empty(base, '-');
    let last = parts.last()?;
    last.parse::<i64>().ok()
}

/// A very small helper for comparing optional build numbers.
pub fn build_display_string(build: Option<i64>) -> Option<String> {
    build.map(|b| format!("build {b}"))
}

/// Returns `true` if `target` is a strict version downgrade from `current`.
/// Returns `false` whenever comparison is impossible (blank, "LATEST", or any
/// non-integer dot-separated segment such as a snapshot like "24w14a").
pub fn is_downgrade(current: Option<&str>, target: &str) -> bool {
    let Some(current) = current else { return false };
    if current.is_empty() {
        return false;
    }
    let c = current.trim().to_lowercase();
    let t = target.trim().to_lowercase();
    if c == "latest" || t.is_empty() || t == "latest" {
        return false;
    }
    let (Some(cv), Some(tv)) = (parse_components(&c), parse_components(&t)) else {
        return false;
    };
    compare_components(&tv, &cv) == std::cmp::Ordering::Less
}

/// `pub(crate)`: reused by `identity.rs`'s `supports_vanilla_tick_query`,
/// which needs the same dotted-integer numeric compare Swift's
/// `.compare(_:options:.numeric)` performs.
pub(crate) fn parse_components(s: &str) -> Option<Vec<i64>> {
    let parts = split_omitting_empty(s, '.');
    if parts.is_empty() {
        return None;
    }
    parts.iter().map(|p| p.parse::<i64>().ok()).collect()
}

pub(crate) fn compare_components(a: &[i64], b: &[i64]) -> std::cmp::Ordering {
    let count = a.len().max(b.len());
    for i in 0..count {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        match av.cmp(&bv) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}
