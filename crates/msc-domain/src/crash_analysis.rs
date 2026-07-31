//! Turns a failed modded-server start into structured, mod-attributed
//! problems the UI can act on (delete / disable / view on Modrinth).
//!
//! Ported from `StartupCrashAnalyzer.swift`'s fixture-tested subset:
//! `analyze`'s Fabric "Mod '...' requires ..." dependency-resolver format,
//! its Forge/NeoForge dependency-block and Sinytra-Connector-entrypoint
//! formats, and the cross-separator `match_installed_mod`/
//! `normalized_identifier` helpers used to map a log's mod-id back to an
//! installed jar.
//!
//! Deliberately NOT ported (untested by any fixture, and not named in
//! P1.7's scope): Fabric's runtime/stack-trace failure branch
//! (`parseRuntimeFailure` in MSC 1 — mapping-mismatch/Mixin crashes),
//! Forge/NeoForge client-only-mod detection (`parseForgeClientOnlyMods` —
//! "invalid dist DEDICATED_SERVER"), and the Paper/Spigot plugin scanner
//! (`analyzePaperPlugins`). `combinedLog`/`newestCrashReport` (reading
//! `logs/latest.log` and `crash-reports/*.txt` off disk) are I/O and stay
//! out of `msc-domain` entirely — `analyze` here takes the console excerpt
//! directly instead of a `serverDir` to read from, matching every fixture
//! (MSC 1's own tests always pass a fresh nonexistent `serverDir`, so
//! `combinedLog` always falls back to the excerpt anyway).
//!
//! `flavor` is a plain loader-family string (`"fabric"`, `"quilt"`,
//! `"forge"`, `"neoforge"`, or anything else) rather than the full
//! `JavaServerFlavor` enum — that type is P1.8's scope, and `analyze` here
//! only needs to know which of two dependency-log grammars to try.

use std::collections::HashSet;

/// An installed mod jar, as reported by the mods-directory scanner.
#[derive(Debug, Clone, PartialEq)]
pub struct ModEntry {
    pub filename: String,
    pub jar_stem: String,
    /// Human-readable name: from fabric.mod.json / mods.toml, or derived
    /// from the filename.
    pub display_name: String,
    /// The loader's mod ID (e.g. "fabric-api"), from the manifest. `None`
    /// for unrecognized jars.
    pub mod_id: Option<String>,
    pub version: Option<String>,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupProblemKind {
    /// Offender needs something that isn't installed.
    MissingDependency,
    /// Offender is built for a different MC/loader/mod version.
    IncompatibleVersion,
    /// Same mod present twice.
    Duplicate,
    /// Threw while loading.
    LoadError,
    Unknown,
}

impl StartupProblemKind {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::MissingDependency => "missingDependency",
            Self::IncompatibleVersion => "incompatibleVersion",
            Self::Duplicate => "duplicate",
            Self::LoadError => "loadError",
            Self::Unknown => "unknown",
        }
    }
}

/// One parsed startup problem, attributed to an installed add-on when we
/// can map it.
#[derive(Debug, Clone, PartialEq)]
pub struct StartupProblem {
    pub kind: StartupProblemKind,
    /// Display name of the mod that has the problem (the one we can
    /// delete/disable).
    pub offender_name: String,
    /// The loader mod-id, when known (used to map back to a file on disk).
    pub offender_id: Option<String>,
    /// The offender's jar filename on disk, when matched to an installed
    /// mod.
    pub installed_file: Option<String>,
    pub installed_jar_stem: Option<String>,
    /// Plain-English requirement, e.g. "requires version 1.21 of
    /// minecraft".
    pub requirement: Option<String>,
    /// For `MissingDependency`: the name of the absent dependency to
    /// install (e.g. "fabric-api"). `None` for non-installable targets
    /// (minecraft/java/loader).
    pub missing_dependency: Option<String>,
    pub raw_excerpt: String,
}

impl StartupProblem {
    /// Mirrors MSC 1's `StartupProblem.id` (`Identifiable` conformance),
    /// used only for de-duplicating problems parsed from the same text.
    fn dedupe_id(&self) -> String {
        format!(
            "{}|{}|{}",
            self.kind.raw_value(),
            self.offender_id.as_deref().unwrap_or(&self.offender_name),
            self.requirement.as_deref().unwrap_or("")
        )
    }
}

const FABRIC_BULLET_TRIM: &[char] = &[' ', '\t', '-', '\u{2013}', '\u{2022}', '\u{00A0}'];

const LOADER_MOD_IDS: &[&str] = &[
    "minecraft",
    "forge",
    "neoforge",
    "fml",
    "javafml",
    "java",
    "lowcodefml",
    "mcp",
];

/// Parses a failed start into structured problems. `console_excerpt` is
/// recent console output captured this run.
pub fn analyze(
    flavor: &str,
    console_excerpt: &[String],
    installed_mods: &[ModEntry],
) -> Vec<StartupProblem> {
    let text = console_excerpt.join("\n");
    if text.is_empty() {
        return Vec::new();
    }
    match flavor {
        "fabric" | "quilt" => parse_fabric(&text, installed_mods),
        "neoforge" | "forge" => parse_forge(&text, installed_mods),
        _ => Vec::new(),
    }
}

// MARK: Fabric / Quilt

/// Fabric Loader prints canonical lines like:
///   - Mod 'Sodium' (sodium) 0.4.0 requires version 1.21 of minecraft, but
///     only ... is present!
///   - Mod 'X' (x) 1.0 requires any version of fabric api, which is
///     missing!
fn parse_fabric(text: &str, installed_mods: &[ModEntry]) -> Vec<StartupProblem> {
    let mut problems = Vec::new();
    let mut seen = HashSet::new();

    for raw in text.split('\n') {
        let bullet = raw.trim_matches(FABRIC_BULLET_TRIM);
        if bullet.starts_with("Mod '")
            && bullet.contains("requires")
            && !bullet.contains("recommends")
            && let Some(problem) = parse_requires_line(bullet, installed_mods)
            && seen.insert(problem.dedupe_id())
        {
            problems.push(problem);
        }
    }
    problems
}

fn parse_requires_line(line: &str, installed_mods: &[ModEntry]) -> Option<StartupProblem> {
    // Name between the first pair of single quotes.
    let open_idx = line.find("Mod '")?;
    let after_open = &line[open_idx + "Mod '".len()..];
    let close_idx = after_open.find('\'')?;
    let name = &after_open[..close_idx];
    if name.is_empty() {
        return None;
    }

    // mod-id inside the first parentheses after the name.
    let after_name = &after_open[close_idx + 1..];
    let mut mod_id: Option<String> = None;
    if let (Some(p_open), Some(p_close)) = (after_name.find('('), after_name.find(')'))
        && p_open <= p_close
    {
        let candidate = after_name[p_open + 1..p_close].trim();
        if !candidate.is_empty() {
            mod_id = Some(candidate.to_string());
        }
    }

    let req_idx = line.find("requires")?;
    let is_missing = line.to_lowercase().contains("which is missing");
    let kind = if is_missing {
        StartupProblemKind::MissingDependency
    } else {
        StartupProblemKind::IncompatibleVersion
    };

    // Short requirement clause: from "requires" up to the first comma.
    let mut clause = &line[req_idx..];
    if let Some(comma_idx) = clause.find(',') {
        clause = &clause[..comma_idx];
    }
    let clause = clause.trim_matches(|c: char| " \t!.".contains(c));

    // Map offender back to an installed mod (by id, then by display name).
    let match_entry = mod_id
        .as_ref()
        .and_then(|id| {
            installed_mods
                .iter()
                .find(|m| m.mod_id.as_deref() == Some(id.as_str()))
        })
        .or_else(|| {
            installed_mods
                .iter()
                .find(|m| m.display_name.to_lowercase() == name.to_lowercase())
        });

    let requirement = if clause.is_empty() {
        None
    } else {
        let mut chars = clause.chars();
        let first_upper: String = chars
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default();
        Some(format!("{first_upper}{}", chars.as_str()))
    };

    // For a missing dependency, capture the target's name so we can offer
    // to install it. It sits between the last " of " and the comma, e.g.
    // "requires any version of fabric api, which is missing!" -> "fabric api".
    let mut missing_dep: Option<String> = None;
    if is_missing && let Some(of_idx) = line.rfind(" of ") {
        let mut target = &line[of_idx + " of ".len()..];
        if let Some(comma_idx) = target.find(',') {
            target = &target[..comma_idx];
        }
        let target = target.trim();
        let non_installable = [
            "minecraft",
            "java",
            "fabricloader",
            "fabric loader",
            "fabric-loader",
            "quilt_loader",
        ];
        if !target.is_empty() && !non_installable.contains(&target.to_lowercase().as_str()) {
            missing_dep = Some(target.to_string());
        }
    }

    Some(StartupProblem {
        kind,
        offender_name: name.to_string(),
        offender_id: mod_id,
        installed_file: match_entry.map(|m| m.filename.clone()),
        installed_jar_stem: match_entry.map(|m| m.jar_stem.clone()),
        requirement,
        missing_dependency: missing_dep,
        raw_excerpt: line.to_string(),
    })
}

// MARK: NeoForge / Forge

/// Modern Forge/NeoForge print (in the log and crash report):
///   Mod ID: 'jei', Requested by: 'somemod', Expected range: '[15.2,)',
///   Actual version: '[MISSING]'
/// "Requested by" is the installed mod with the unmet requirement; "Mod
/// ID" is the dependency. We attribute the actionable offender accordingly
/// (see `parse_forge_dependency_line`).
fn parse_forge(text: &str, installed_mods: &[ModEntry]) -> Vec<StartupProblem> {
    let mut problems = Vec::new();
    let mut seen = HashSet::new();

    for raw in text.split('\n') {
        let line = raw.trim();
        // Sinytra Connector surfaces Fabric-side failures inside a Forge
        // start as an EarlyLoadingException — parse those before the
        // Forge dependency-line format.
        if let Some(problem) = parse_connector_entrypoint_failure(line, installed_mods) {
            if seen.insert(problem.dedupe_id()) {
                problems.push(problem);
            }
            continue;
        }
        if !(line.contains("Mod ID:")
            && line.contains("Requested by:")
            && line.contains("Actual version:"))
        {
            continue;
        }
        if let Some(problem) = parse_forge_dependency_line(line, installed_mods)
            && seen.insert(problem.dedupe_id())
        {
            problems.push(problem);
        }
    }
    problems
}

/// Sinytra Connector can surface a Fabric entrypoint failure inside a
/// Forge start as:
///   net.minecraftforge.fml.loading.EarlyLoadingException: Could not
///   execute entrypoint stage 'main' due to errors, provided by
///   'particle_effects'
/// Returns `None` (rather than guessing) when the line lacks a `provided
/// by '<modid>'`.
fn parse_connector_entrypoint_failure(
    line: &str,
    installed_mods: &[ModEntry],
) -> Option<StartupProblem> {
    if !(line.contains("Could not execute entrypoint stage") && line.contains("provided by")) {
        return None;
    }
    let offender_id = quoted_value_after("provided by", line)?;
    let match_entry = match_installed_mod(&offender_id, installed_mods);
    Some(StartupProblem {
        kind: StartupProblemKind::LoadError,
        offender_name: match_entry
            .map(|m| m.display_name.clone())
            .unwrap_or_else(|| offender_id.clone()),
        offender_id: Some(offender_id),
        installed_file: match_entry.map(|m| m.filename.clone()),
        installed_jar_stem: match_entry.map(|m| m.jar_stem.clone()),
        requirement: Some(
            "Failed while loading a Connector/Fabric entrypoint on the dedicated server."
                .to_string(),
        ),
        missing_dependency: None,
        raw_excerpt: line.to_string(),
    })
}

fn parse_forge_dependency_line(line: &str, installed_mods: &[ModEntry]) -> Option<StartupProblem> {
    let dep_id = quoted_value_after("Mod ID:", line)?;
    let requested_by = quoted_value_after("Requested by:", line)?;
    let expected = quoted_value_after("Expected range:", line);
    let actual = quoted_value_after("Actual version:", line);
    let is_missing = actual
        .as_deref()
        .unwrap_or("")
        .to_uppercase()
        .contains("MISSING");
    let dep_is_loader = LOADER_MOD_IDS.contains(&dep_id.to_lowercase().as_str());

    // Offender = the mod we can act on (delete/disable/update).
    //  - missing dep, or the requester needs a different MC/loader version -> requester
    //  - a real dependency is the wrong version -> that dependency (so "Update" fixes it)
    let offender_id: String;
    let kind: StartupProblemKind;
    let mut missing_dep: Option<String> = None;
    let requirement: String;

    if is_missing {
        kind = StartupProblemKind::MissingDependency;
        offender_id = requested_by;
        missing_dep = if dep_is_loader {
            None
        } else {
            Some(dep_id.clone())
        };
        requirement = format!("Requires {} {}", dep_id, expected.as_deref().unwrap_or(""))
            .trim()
            .to_string();
    } else if dep_is_loader {
        kind = StartupProblemKind::IncompatibleVersion;
        offender_id = requested_by;
        requirement = format!(
            "Needs {} {} (have {})",
            dep_id,
            expected.as_deref().unwrap_or(""),
            actual.as_deref().unwrap_or("?")
        );
    } else {
        kind = StartupProblemKind::IncompatibleVersion;
        offender_id = dep_id.clone();
        requirement = format!(
            "Needs version {} (have {}); required by {}",
            expected.as_deref().unwrap_or("?"),
            actual.as_deref().unwrap_or("?"),
            requested_by
        );
    }

    let match_entry = match_installed_mod(&offender_id, installed_mods);

    Some(StartupProblem {
        kind,
        offender_name: match_entry
            .map(|m| m.display_name.clone())
            .unwrap_or_else(|| offender_id.clone()),
        offender_id: Some(offender_id),
        installed_file: match_entry.map(|m| m.filename.clone()),
        installed_jar_stem: match_entry.map(|m| m.jar_stem.clone()),
        requirement: Some(requirement),
        missing_dependency: missing_dep,
        raw_excerpt: line.to_string(),
    })
}

/// Extracts the first `'...'`-quoted value following `label` in `line`.
fn quoted_value_after(label: &str, line: &str) -> Option<String> {
    let label_idx = line.find(label)?;
    let after = &line[label_idx + label.len()..];
    let q1 = after.find('\'')?;
    let rest = &after[q1 + 1..];
    let q2 = rest.find('\'')?;
    let value = rest[..q2].trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

/// Maps a loader mod-id or display name back to an installed jar, tolerant
/// of the separator drift between Forge internal ids, Modrinth slugs, and
/// human names — `particle_effects`, `particle-effects`, and "Particle
/// Effects" all match the same jar. Tries (in order): normalized id,
/// normalized display name, punctuation-stripped id, punctuation-stripped
/// name, then a jar-stem prefix match. Returns `None` if nothing matches —
/// the caller keeps the raw offender id rather than mis-attributing.
pub fn match_installed_mod<'a>(
    id_or_name: &str,
    installed_mods: &'a [ModEntry],
) -> Option<&'a ModEntry> {
    let wanted = normalized_identifier(id_or_name);
    if wanted.is_empty() {
        return None;
    }
    let wanted_compact = compact_identifier(id_or_name);

    installed_mods
        .iter()
        .find(|m| normalized_identifier(m.mod_id.as_deref().unwrap_or("")) == wanted)
        .or_else(|| {
            installed_mods
                .iter()
                .find(|m| normalized_identifier(&m.display_name) == wanted)
        })
        .or_else(|| {
            installed_mods
                .iter()
                .find(|m| compact_identifier(m.mod_id.as_deref().unwrap_or("")) == wanted_compact)
        })
        .or_else(|| {
            installed_mods
                .iter()
                .find(|m| compact_identifier(&m.display_name) == wanted_compact)
        })
        .or_else(|| {
            installed_mods
                .iter()
                .find(|m| compact_identifier(&m.jar_stem).starts_with(&wanted_compact))
        })
}

/// Lowercases and collapses every run of non-alphanumerics to a single
/// dash, trimming leading/trailing dashes. "Particle Effects" ->
/// "particle-effects"; "particle_effects" -> "particle-effects".
///
/// Identical algorithm to `slug::normalized_slug` — MSC 1 itself
/// duplicates this function verbatim between `StartupCrashAnalyzer` and
/// `ModrinthSlugNormalizer` rather than sharing it, so this port keeps the
/// same two independent copies.
pub fn normalized_identifier(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    let mut result = String::new();
    let mut previous_was_dash = false;
    for c in lower.chars() {
        if c.is_alphanumeric() {
            result.push(c);
            previous_was_dash = false;
        } else if !previous_was_dash {
            result.push('-');
            previous_was_dash = true;
        }
    }
    result.trim_matches('-').to_string()
}

/// Like `normalized_identifier` but with all separators removed —
/// "particle-effects" and "particleeffects" collapse together for the
/// loosest (prefix) matching tier.
pub fn compact_identifier(raw: &str) -> String {
    normalized_identifier(raw).replace('-', "")
}
