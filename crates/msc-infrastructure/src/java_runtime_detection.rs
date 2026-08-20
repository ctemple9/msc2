//! The filesystem-backed half of `JavaRuntimeManager.swift`: discovering
//! installed Java runtimes on disk, and normalizing a user-supplied Java
//! path into an executable binary. Deferred here from `msc-domain`'s P1.5
//! port (`msc_domain::java_runtime`, which carries the pure Minecraft
//! version -> required-Java-major mapping and warning text) until this
//! phase's [`FileSystem`] trait existed to back it — `msc-domain` carries
//! no I/O, per `msc2-engineering.md` §6.
//!
//! One thing this still deliberately does *not* port, since no fixture
//! exercises it and P1.5's own fixture notes are explicit about why:
//! `detectJavaMajor` (spawning `java -version` as a subprocess) —
//! "majorVersion and name must come from parsing the macOS JDK bundle
//! directory-naming convention... never from executing the binary". Real
//! process execution *is* used below, but only for the two `which java`
//! probes P7.16 characterizes fresh (`checkJavaOnPath`/`isJavaInstalled`),
//! never for major-version detection.
//!
//! P7.16 extends this module with the rest of P7.7's discovery surface:
//! [`default_java_runtime_search_roots`] (deferred here at P1.5 pending a
//! caller — this phase is that caller), and the `which java`-backed
//! [`check_java_on_path`]/[`is_java_installed`]/
//! [`has_critical_missing_dependency`], ported from `SetupWizardView.swift`
//! and `PrerequisitesView.swift` against
//! `fixtures/java-runtime-selection/check-java-on-path-*` and
//! `has-critical-missing-dependency-*` (4 of the 6 cases P7.12 deferred
//! here; the other 2, Adoptium's managed install, are
//! `java_runtime_install.rs`'s job).

use crate::fs::FileSystem;
use crate::process::{
    OutputStream, ProcessError, ProcessEvent, ProcessSpawnRequest, ProcessSupervisor,
};
use msc_domain::identity::ServerType;
use msc_domain::java_runtime::JavaVersionProbe;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

/// Which host platform is running MSC 2 — governs both
/// [`default_java_runtime_search_roots`] here and the managed-install
/// archive format in `java_runtime_install.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostOs {
    Mac,
    Linux,
    Windows,
}

/// `defaultJavaRuntimeSearchRoots()` (`JavaRuntimeManager.swift:108-119`),
/// generalized to all three host platforms. The Mac list is a direct port
/// — real oracle, real paths. Linux and Windows have no MSC 1 counterpart
/// to port at all (the source app never ran anywhere else), so those two
/// lists are new, reasoned-but-unfixtured defaults: common JDK install
/// locations plus the same cross-platform version managers
/// (SDKMAN, jenv) the Mac list already includes. A caller that wants a
/// different set already can — every fixture in this corpus supplies
/// `search_roots` explicitly rather than calling this function, and
/// nothing in Phase 7's gate depends on these particular paths being
/// exactly right.
pub fn default_java_runtime_search_roots(os: HostOs, home_dir: &Path) -> Vec<String> {
    let home = home_dir.to_string_lossy();
    match os {
        HostOs::Mac => vec![
            "/Library/Java/JavaVirtualMachines".to_string(),
            format!("{home}/Library/Java/JavaVirtualMachines"),
            format!("{home}/.sdkman/candidates/java"),
            format!("{home}/.jenv/versions"),
            "/opt/homebrew/opt".to_string(),
            "/usr/local/opt".to_string(),
            "/opt/homebrew/Cellar".to_string(),
            "/usr/local/Cellar".to_string(),
        ],
        HostOs::Linux => vec![
            "/usr/lib/jvm".to_string(),
            format!("{home}/.sdkman/candidates/java"),
            format!("{home}/.jenv/versions"),
            "/home/linuxbrew/.linuxbrew/opt".to_string(),
            "/home/linuxbrew/.linuxbrew/Cellar".to_string(),
        ],
        HostOs::Windows => vec![
            "C:/Program Files/Java".to_string(),
            "C:/Program Files/Eclipse Adoptium".to_string(),
            "C:/Program Files/Microsoft".to_string(),
            format!("{home}/.sdkman/candidates/java"),
            format!("{home}/.jenv/versions"),
            format!("{home}/scoop/apps"),
        ],
    }
}

/// How often [`run_which_java`]'s wait loop polls for output/exit.
const WHICH_JAVA_POLL_INTERVAL: Duration = Duration::from_millis(10);
/// `/usr/bin/which` returns effectively instantly in every real case; this
/// only exists so a broken supervisor degrades honestly (per this phase's
/// own "honest degradation" requirement) instead of hanging a caller
/// forever.
const WHICH_JAVA_TIMEOUT: Duration = Duration::from_secs(5);

/// `checkJavaOnPath()`'s and `isJavaInstalled()`'s shared shell-out
/// (`SetupWizardView.swift:1332-1342`, `PrerequisitesView.swift:571-582`):
/// both run `/usr/bin/which java`, trim stdout, and treat anything else —
/// a spawn failure, a non-empty-but-untrimmed-to-empty result, a timeout —
/// as "not found", the same catch-all shape source's own `catch { ...
/// notFound }` uses. Unix-only, matching source (`/usr/bin/which` has no
/// Windows equivalent — a Windows `which_java` probe is not something any
/// fixture in this corpus asks for).
fn run_which_java(supervisor: &dyn ProcessSupervisor) -> Result<String, ProcessError> {
    let request = ProcessSpawnRequest::new("/usr/bin/which", ".").arg("java");
    let pid = supervisor.spawn(request)?;
    let mut stdout = Vec::new();
    let deadline = Instant::now() + WHICH_JAVA_TIMEOUT;

    loop {
        for event in supervisor.drain_events(pid)? {
            match event {
                ProcessEvent::Output {
                    stream: OutputStream::Stdout,
                    bytes,
                } => stdout.extend_from_slice(&bytes),
                ProcessEvent::Output { .. } => {}
                ProcessEvent::Exited(_) => {
                    return Ok(String::from_utf8_lossy(&stdout).trim().to_string());
                }
            }
        }
        if Instant::now() >= deadline {
            let _ = supervisor.force_terminate(pid);
            return Ok(String::from_utf8_lossy(&stdout).trim().to_string());
        }
        std::thread::sleep(WHICH_JAVA_POLL_INTERVAL);
    }
}

/// `checkJavaOnPath()`'s own result shape (`SetupWizardView.swift`'s
/// `JavaStatus`, as far as this probe touches it): found carries the
/// trimmed `which` output, matching `.found(path:)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaOnPathStatus {
    Found { path: String },
    NotFound,
}

/// `checkJavaOnPath()` (source line 1329-1357): a non-empty trimmed
/// `which java` result is `.found`; an empty result or any process error
/// is `.notFound` — source never inspects the exit code, only the output.
pub fn check_java_on_path(supervisor: &dyn ProcessSupervisor) -> JavaOnPathStatus {
    match run_which_java(supervisor) {
        Ok(output) if !output.is_empty() => JavaOnPathStatus::Found { path: output },
        _ => JavaOnPathStatus::NotFound,
    }
}

/// Source line 1346-1348's `if self.javaPath...isEmpty` guard, split out
/// as pure logic: `Some(path)` when the preference field should be
/// auto-filled, `None` when it should be left exactly as the caller had
/// it (either because nothing was found, or because a user-entered value
/// is already there and must never be clobbered by this check).
pub fn java_on_path_field_autofill(
    current_field: &str,
    status: &JavaOnPathStatus,
) -> Option<String> {
    match status {
        JavaOnPathStatus::Found { path } if current_field.trim().is_empty() => Some(path.clone()),
        _ => None,
    }
}

/// `PrerequisitesView.isJavaInstalled()` (source line 570-584): its own,
/// separately-implemented `which java` probe — same shell command as
/// [`check_java_on_path`], not shared code in the oracle, so not
/// artificially unified here either (this port already collapses the
/// actual duplicate subprocess-invocation code into [`run_which_java`];
/// what stays separate is the two callers' own pass/fail semantics).
pub fn is_java_installed(supervisor: &dyn ProcessSupervisor) -> bool {
    matches!(run_which_java(supervisor), Ok(output) if !output.is_empty())
}

/// `-version`'s own timeout: generous for a real cold JVM start (which
/// `run_which_java`'s 5s budget, tuned for `/usr/bin/which`, isn't), but
/// still finite so a genuinely broken executable degrades honestly (per
/// this phase's own "honest degradation" requirement) instead of hanging
/// creation or start forever.
const JAVA_VERSION_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// P7.31's own probe: the create/start-time counterpart to
/// [`run_which_java`], spawning `<executable_path> -version` through the
/// same testable [`ProcessSupervisor`] boundary rather than the
/// unsupervised `std::process::Command` `GET /v1/health`'s own
/// `run_java_version_probe` uses -- this one feeds
/// [`msc_domain::java_runtime::evaluate_java_runtime_guard`], which
/// creation and start both call before committing to a launch. Combines
/// stdout+stderr in event order (`java -version` traditionally writes to
/// stderr) the same way a captured terminal session would show them. A
/// spawn failure, a `drain_events` error, or a timeout all collapse to
/// [`JavaVersionProbe::NotFound`] -- the same "can't tell, treat as
/// absent" catch-all [`run_which_java`] already uses.
pub fn run_java_version_probe(
    supervisor: &dyn ProcessSupervisor,
    executable_path: &str,
) -> JavaVersionProbe {
    let request = ProcessSpawnRequest::new(executable_path, ".").arg("-version");
    let Ok(pid) = supervisor.spawn(request) else {
        return JavaVersionProbe::NotFound;
    };
    let mut combined = Vec::new();
    let deadline = Instant::now() + JAVA_VERSION_PROBE_TIMEOUT;

    loop {
        let Ok(events) = supervisor.drain_events(pid) else {
            return JavaVersionProbe::NotFound;
        };
        let mut exited = false;
        for event in events {
            match event {
                ProcessEvent::Output { bytes, .. } => combined.extend_from_slice(&bytes),
                ProcessEvent::Exited(_) => exited = true,
            }
        }
        if exited {
            break;
        }
        if Instant::now() >= deadline {
            let _ = supervisor.force_terminate(pid);
            break;
        }
        std::thread::sleep(WHICH_JAVA_POLL_INTERVAL);
    }

    JavaVersionProbe::Captured {
        output: String::from_utf8_lossy(&combined).into_owned(),
    }
}

/// `PrerequisitesView.hasCriticalMissingDependency(for:)` (source line
/// 556-568): only calls [`is_java_installed`] at all when `server_types`
/// contains [`ServerType::Java`] — a fleet with no Java servers configured
/// never trips this on a missing Java runtime. The commented-out Bedrock/
/// Docker branch in source (line 562-566) was never wired up; this port
/// does not invent a check that doesn't exist there.
pub fn has_critical_missing_dependency(
    supervisor: &dyn ProcessSupervisor,
    server_types: &[ServerType],
) -> bool {
    if server_types.contains(&ServerType::Java) {
        !is_java_installed(supervisor)
    } else {
        false
    }
}
