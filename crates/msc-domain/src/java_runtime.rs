//! Pure Java runtime compatibility guards and, as of P7.12, the pure half
//! of runtime *selection*.
//!
//! Ported from `JavaRuntimeManager.swift`'s pure subset: the
//! Minecraft-version-to-required-Java-major mapping, the
//! compatibility-warning text generator, and `parseMajor(fromVersionOutput:)`
//! (given the captured `java -version` text; capturing it is the caller's
//! job). Detecting installed runtimes on disk and normalizing candidate
//! filesystem paths stay unported here — `msc-domain` carries no I/O, per
//! `msc2-engineering.md` §6 — and live in `msc-infrastructure`'s
//! `java_runtime_detection` (Phase 3).
//!
//! P7.12 additions: `validateLooksLikeJava` (`ServerProcessManager.swift`,
//! given captured process output), `JavaInstaller`'s fixed
//! `minecraftInstallOptions` table and `recommendedOption`, and the
//! per-server-then-global java-path precedence rules
//! (`AppViewModel+ServerCreation.swift`'s create-time override,
//! `AppViewModel+ServerSettings.swift`'s `resolvedJavaPath`). Scope call:
//! `fixtures/java-runtime-selection/`'s other 6 cases (the managed Adoptium
//! install, `checkJavaOnPath`, `hasCriticalMissingDependency`) need real
//! filesystem/process/network I/O and belong to P7.16's infrastructure
//! step, not this domain-only one — this step's own `Files:` list (just
//! `java_runtime.rs`, no infrastructure file) already implies as much.

use std::fmt;

/// The Java major version a given Minecraft version needs to run.
/// Conservative mapping; unknown versions assume the current requirement
/// (Java 25).
pub fn required_java_major(minecraft_version: Option<&str>) -> i64 {
    let Some(version) = minecraft_version else {
        return 25; // unknown -> assume newest
    };
    let numeric_parts: Vec<i64> = version
        .split('.')
        .filter_map(|p| p.parse::<i64>().ok())
        .collect();
    let Some(&first) = numeric_parts.first() else {
        return 25;
    };
    if first == 1 {
        // Classic "1.x" scheme (up to 1.21.x, the last before year-based numbering).
        let minor = numeric_parts.get(1).copied().unwrap_or(0);
        if minor >= 21 {
            return 21; // 1.20.5 / 1.21.x -> Java 21
        }
        if minor >= 17 {
            return 17; // 1.17-1.20.4 -> Java 17
        }
        return 8; // <=1.16 -> Java 8
    }
    // Year-based scheme (2026+): Minecraft 26.1 is the first to require Java 25.
    25
}

/// Core warning logic (takes pre-detected major versions, so no process
/// needs to be spawned to test it). Returns `None` when no warning is
/// needed.
pub fn compatibility_warning_text(
    minecraft_version: Option<&str>,
    required: i64,
    detected: i64,
) -> Option<String> {
    let version_text = match minecraft_version {
        Some(v) => format!("Minecraft {v}"),
        None => "this Minecraft version".to_string(),
    };
    if detected < required {
        return Some(format!(
            "{version_text} needs Java {required}, but the configured Java is version {detected}. \
Install Java {required} (e.g. Temurin/Adoptium) and set it in Preferences, or choose an older Minecraft version."
        ));
    }
    // Java-17-era Minecraft (1.17-1.20.x, required=17) is known to have classpath and
    // ASM issues with Java 21+. Warn when a newer runtime is configured so the user
    // understands why a modpack might fail, without blocking the start.
    if detected > required && required <= 17 {
        return Some(format!(
            "{version_text} modpacks are usually built and tested for Java {required}, but the configured Java is version {detected}. \
If this server fails to start, install Java {required} (e.g. Temurin/Adoptium) and set it in Preferences."
        ));
    }
    None
}

/// `JavaRuntimeManager.parseMajor(fromVersionOutput:)`: takes the first
/// double-quoted token in a captured `java -version` banner (vendor-agnostic
/// — GraalVM's "java version", Temurin's/Zulu's "openjdk version" prefixes
/// are never consulted), splits it on `.`/`_`. The legacy `"1.x.y_z"` scheme
/// (pre-Java-9) returns the *second* component; every later scheme returns
/// the first.
pub fn parse_major(version_output: &str) -> Option<i64> {
    let start = version_output.find('"')? + 1;
    let rest = &version_output[start..];
    let end = rest.find('"')?;
    let quoted = &rest[..end];
    let parts: Vec<&str> = quoted.split(['.', '_']).collect();
    let first: i64 = parts.first()?.parse().ok()?;
    if first == 1 {
        parts.get(1)?.parse().ok()
    } else {
        Some(first)
    }
}

/// `ServerProcessManager.validateLooksLikeJava`'s guard: the captured
/// stdout+stderr of a `-version` invocation, lowercased, must contain at
/// least one of five vendor-agnostic substrings. Each is independently
/// sufficient — this doesn't require any particular combination.
fn looks_like_java_output(captured_output: &str) -> bool {
    let lower = captured_output.to_lowercase();
    lower.contains("openjdk")
        || lower.contains("java version")
        || lower.contains("java(tm)")
        || lower.contains("runtime environment")
        || lower.contains("hotspot")
}

/// A configured Java path whose captured `-version` output doesn't look
/// like a JVM at all (e.g. someone pasted a different binary's path by
/// mistake). `display` is the configured path as given, not a resolved
/// binary name; `first_output_line` is only the first line of what was
/// captured, matching source's `maxSplits: 1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotAJavaBinary {
    pub display: String,
    pub first_output_line: String,
}

impl fmt::Display for NotAJavaBinary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Java path does not appear to be a JVM: {}\n\nOutput: {}",
            self.display, self.first_output_line
        )
    }
}

impl std::error::Error for NotAJavaBinary {}

/// `validateLooksLikeJava(executableURL:arguments:display:)`, given the
/// captured output directly — spawning the process to capture it is the
/// caller's job (`ServerProcessManager.swift:144-170`).
pub fn validate_looks_like_java(
    display: &str,
    captured_output: &str,
) -> Result<(), NotAJavaBinary> {
    if looks_like_java_output(captured_output) {
        return Ok(());
    }
    let first_line = captured_output
        .split('\n')
        .find(|line| !line.is_empty())
        .unwrap_or("(no output)")
        .to_string();
    Err(NotAJavaBinary {
        display: display.to_string(),
        first_output_line: first_line,
    })
}

/// One row of `JavaInstaller.minecraftInstallOptions`'s fixed table
/// (`JavaInstaller.swift:37-46`) — a hand-authored list, not derived from
/// any API, deliberately offering only these four majors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JavaInstallOption {
    pub major: i64,
    pub title: &'static str,
    pub minecraft_range: &'static str,
    pub is_recommended: bool,
}

pub const MINECRAFT_INSTALL_OPTIONS: [JavaInstallOption; 4] = [
    JavaInstallOption {
        major: 25,
        title: "Java 25",
        minecraft_range: "Minecraft 26.1 (latest) and newer",
        is_recommended: true,
    },
    JavaInstallOption {
        major: 21,
        title: "Java 21",
        minecraft_range: "Minecraft 1.20.5 – 1.21.x",
        is_recommended: false,
    },
    JavaInstallOption {
        major: 17,
        title: "Java 17",
        minecraft_range: "Minecraft 1.17 – 1.20.4",
        is_recommended: false,
    },
    JavaInstallOption {
        major: 8,
        title: "Java 8",
        minecraft_range: "Minecraft 1.16.5 and older",
        is_recommended: false,
    },
];

/// `JavaInstaller.recommendedOption(forMinecraftVersion:)`
/// (`JavaInstaller.swift:51-56`): look up `requiredJavaMajor`'s result by
/// exact major match first, falling back to the table's one `isRecommended`
/// entry and then its first entry. Per that fixture's own finding, the two
/// fallbacks are unreachable with any real `required_java_major` output
/// (its four possible results are exactly this table's four majors) —
/// kept anyway, matching source, rather than simplified into an `unwrap`.
pub fn recommended_option(minecraft_version: Option<&str>) -> JavaInstallOption {
    let major = required_java_major(minecraft_version);
    MINECRAFT_INSTALL_OPTIONS
        .iter()
        .find(|o| o.major == major)
        .or_else(|| MINECRAFT_INSTALL_OPTIONS.iter().find(|o| o.is_recommended))
        .copied()
        .unwrap_or(MINECRAFT_INSTALL_OPTIONS[0])
}

/// `javaPath ?? configManager.config.javaPath`
/// (`AppViewModel+ServerCreation.swift:144`): a create call's explicit
/// per-call override wins over the app-wide default. MSC 1 has no
/// persisted per-server java path — this override is scoped to one create
/// call, not saved for later starts of that server.
pub fn resolve_create_time_java_path<'a>(
    create_request_java_path: Option<&'a str>,
    global_config_java_path: &'a str,
) -> &'a str {
    create_request_java_path.unwrap_or(global_config_java_path)
}

/// Settings' own `resolvedJavaPath(_:)` (`AppViewModel+ServerSettings.swift:404-405`,
/// distinct from [`resolve_create_time_java_path`]'s create-flow fallback):
/// what actually gets written to `cfg.javaPath` when Preferences or the
/// Setup Wizard is saved. An empty field is never stored empty — it becomes
/// the bare `"java"` command, resolved against `PATH` at launch time. The
/// same trim-and-default rule as [`crate::launch_shape::effective_java_command`];
/// exposed under this name too since it's a materially different call site
/// (writing stored config, not resolving a launch).
pub fn resolved_settings_java_path(trimmed_input: &str) -> String {
    crate::launch_shape::effective_java_command(trimmed_input)
}
