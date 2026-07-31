//! Pure Java runtime compatibility guards.
//!
//! Ported from `JavaRuntimeManager.swift`'s pure subset: the
//! Minecraft-version-to-required-Java-major mapping, and the
//! compatibility-warning text generator. Detecting the installed Java's
//! major version (spawning `java -version`) and normalizing candidate
//! filesystem paths stay unported here — `msc-domain` carries no I/O, per
//! `msc2-engineering.md` §6 — and move to `msc-infrastructure` once Phase 3
//! builds the filesystem substrate behind a trait.

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
