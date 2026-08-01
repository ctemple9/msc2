//! The filesystem-backed half of `JavaRuntimeManager.swift`: discovering
//! installed Java runtimes on disk, and normalizing a user-supplied Java
//! path into an executable binary. Deferred here from `msc-domain`'s P1.5
//! port (`msc_domain::java_runtime`, which carries the pure Minecraft
//! version -> required-Java-major mapping and warning text) until this
//! phase's [`FileSystem`] trait existed to back it — `msc-domain` carries
//! no I/O, per `msc2-engineering.md` §6.
//!
//! Two things this deliberately does *not* port, since no fixture exercises
//! either and both need capabilities beyond a `FileSystem` trait:
//! - `detectJavaMajor` (spawning `java -version` as a subprocess) — the
//!   fixtures' own notes confirm the ported behavior should come from
//!   [`infer_java_major_version`]'s path-text inference alone ("majorVersion
//!   and name must come from parsing the macOS JDK bundle directory-naming
//!   convention... never from executing the binary"). Real process
//!   execution is a separate substrate concern for a later phase.
//! - `defaultJavaRuntimeSearchRoots` (resolving the current user's home
//!   directory and OS-specific install locations) — every fixture supplies
//!   `searchRoots` explicitly; a caller wiring this into Settings can build
//!   its own default list once that phase exists.

use crate::fs::FileSystem;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedJavaRuntime {
    pub name: String,
    pub executable_path: String,
    pub home_path: String,
    pub major_version: Option<i64>,
}

/// Normalizes a java path so MSC always receives an executable binary, not
/// a JDK home directory. Bare command names containing no `/` are returned
/// unchanged — resolved against `PATH` at launch, not here. A directory
/// containing `bin/java` is expanded to that binary, handling the common
/// mistake of pasting a `JAVA_HOME` path into Preferences instead of the
/// `java` binary itself.
///
/// `Ok` carries the normalized path; `Err` carries a message describing why
/// no executable could be resolved.
pub fn normalized_java_executable_path(
    fs: &dyn FileSystem,
    raw_path: &str,
) -> Result<String, String> {
    if !raw_path.contains('/') {
        return Ok(raw_path.to_string());
    }
    let path = Path::new(raw_path);
    let meta = fs
        .stat(path)
        .map_err(|_| format!("Java path does not exist: {raw_path}"))?;

    if meta.is_dir {
        let candidate = path.join("bin/java");
        return match fs.stat(&candidate) {
            Ok(m) if m.executable => Ok(candidate.to_string_lossy().into_owned()),
            _ => Err(format!(
                "'{raw_path}' is a Java HOME directory but has no executable at bin/java"
            )),
        };
    }

    if meta.executable {
        Ok(raw_path.to_string())
    } else {
        Err(format!("'{raw_path}' exists but is not executable"))
    }
}

/// Walks `search_roots`, the way `detectInstalledJavaRuntimes` walks
/// `defaultJavaRuntimeSearchRoots()`: each root and its immediate
/// subdirectories are inspected both directly and via a `Contents/Home`
/// child (the macOS JDK bundle layout), Homebrew `Cellar` roots get one
/// extra level of inspection for `<package>/<version>/bin/java`, and
/// candidates under an `opt`/`Cellar` root are filtered to names that look
/// like a JDK before being inspected at all.
pub fn detect_installed_java_runtimes(
    fs: &dyn FileSystem,
    search_roots: &[String],
) -> Vec<DetectedJavaRuntime> {
    let mut runtimes: BTreeMap<String, DetectedJavaRuntime> = BTreeMap::new();

    for root in search_roots {
        let root_path = PathBuf::from(root);
        inspect_candidate(fs, &mut runtimes, &root_path);

        let Ok(children) = fs.list(&root_path) else {
            continue;
        };
        for child in &children {
            if !is_directory(fs, child) {
                continue;
            }
            if !should_inspect_candidate(&root_path, child) {
                continue;
            }
            inspect_candidate(fs, &mut runtimes, child);

            if root_path.file_name().and_then(|n| n.to_str()) == Some("Cellar")
                && let Ok(versions) = fs.list(child)
            {
                for version in &versions {
                    if is_directory(fs, version) {
                        inspect_candidate(fs, &mut runtimes, version);
                    }
                }
            }
        }
    }

    let mut result: Vec<DetectedJavaRuntime> = runtimes.into_values().collect();
    result.sort_by(compare_runtimes);
    result
}

fn is_directory(fs: &dyn FileSystem, path: &Path) -> bool {
    fs.stat(path).map(|m| m.is_dir).unwrap_or(false)
}

/// Mirrors `inspectCandidate`: a candidate is checked both as a JDK home
/// directly, and as the parent of a macOS `Contents/Home` bundle layout.
fn inspect_candidate(
    fs: &dyn FileSystem,
    runtimes: &mut BTreeMap<String, DetectedJavaRuntime>,
    candidate: &Path,
) {
    insert_runtime(fs, runtimes, candidate);
    insert_runtime(fs, runtimes, &candidate.join("Contents/Home"));
}

fn insert_runtime(
    fs: &dyn FileSystem,
    runtimes: &mut BTreeMap<String, DetectedJavaRuntime>,
    home: &Path,
) {
    let java_path = home.join("bin/java");
    let Ok(meta) = fs.stat(&java_path) else {
        return;
    };
    if !meta.is_file || !meta.executable {
        return;
    }
    let executable_path = java_path.to_string_lossy().into_owned();
    if runtimes.contains_key(&executable_path) {
        return;
    }
    let home_path = home.to_string_lossy().into_owned();
    let major_version = infer_java_major_version(&home_path);
    runtimes.insert(
        executable_path.clone(),
        DetectedJavaRuntime {
            name: java_runtime_display_name(home),
            executable_path,
            home_path,
            major_version,
        },
    );
}

/// Only `opt`/`Cellar` roots filter their candidates — everywhere else
/// (e.g. `/Library/Java/JavaVirtualMachines`), every subdirectory is a
/// plausible JDK install and gets inspected.
fn should_inspect_candidate(root: &Path, candidate: &Path) -> bool {
    let root_name = root.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if root_name != "opt" && root_name != "Cellar" {
        return true;
    }
    let name = candidate
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();
    ["jdk", "java", "temurin", "zulu", "corretto", "openjdk"]
        .iter()
        .any(|needle| name.contains(needle))
}

/// The last two path components being `Contents/Home` (a macOS JDK bundle)
/// names the runtime after the bundle directory two levels up
/// (`temurin-21.jdk`); anything else is named after its own last component.
fn java_runtime_display_name(home: &Path) -> String {
    let components: Vec<String> = home
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if components.len() >= 3
        && components[components.len() - 2] == "Contents"
        && components[components.len() - 1] == "Home"
    {
        return cleaned_java_runtime_name(&components[components.len() - 3]);
    }
    let last = components.last().cloned().unwrap_or_default();
    cleaned_java_runtime_name(&last)
}

fn cleaned_java_runtime_name(raw: &str) -> String {
    replace_case_insensitive(raw, ".jdk").replace('_', " ")
}

fn replace_case_insensitive(input: &str, pattern: &str) -> String {
    let lower_pattern = pattern.to_lowercase();
    let lower_input = input.to_lowercase();
    let mut result = String::with_capacity(input.len());
    let mut rest = input;
    let mut lower_rest = lower_input.as_str();
    while let Some(idx) = lower_rest.find(&lower_pattern) {
        result.push_str(&rest[..idx]);
        rest = &rest[idx + pattern.len()..];
        lower_rest = &lower_rest[idx + pattern.len()..];
    }
    result.push_str(rest);
    result
}

/// Descending by major version (newest first), `Some` before `None`, and
/// name as the tiebreak — mirrors the Swift sort closure. The name compare
/// is a plain case-insensitive one rather than a full port of
/// `localizedStandardCompare`'s natural-number-aware collation: no fixture
/// pins tie-break ordering precisely enough to require it.
fn compare_runtimes(a: &DetectedJavaRuntime, b: &DetectedJavaRuntime) -> std::cmp::Ordering {
    match (a.major_version, b.major_version) {
        (Some(l), Some(r)) if l != r => r.cmp(&l),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    }
}

/// Hand-written stand-in for the Swift source's
/// `(?<!\d)(1[.]8|8|11|16|17|18|19|20|21|22|23|24|25|26)(?!\d)` regex — no
/// lookaround support in a dependency-free port, so this walks the string
/// itself: leftmost match wins, alternatives are tried in the same priority
/// order the regex alternation would, and a match is rejected if the
/// character immediately before or after it is also an ASCII digit.
fn infer_java_major_version(text: &str) -> Option<i64> {
    const TOKENS: [&str; 14] = [
        "1.8", "8", "11", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26",
    ];
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    for start in 0..len {
        if start > 0 && chars[start - 1].is_ascii_digit() {
            continue;
        }
        for token in TOKENS {
            let token_chars: Vec<char> = token.chars().collect();
            let end = start + token_chars.len();
            if end > len || chars[start..end] != token_chars[..] {
                continue;
            }
            if end < len && chars[end].is_ascii_digit() {
                continue;
            }
            return Some(if token == "1.8" {
                8
            } else {
                token.parse().unwrap()
            });
        }
    }
    None
}
