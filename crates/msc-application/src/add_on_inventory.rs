//! P7.36: the local `mods/`/`plugins/` directory scanner the crash
//! analyzers need — `refreshDiscoveredMods`
//! (`AppViewModel+ModManagement.swift:142-187`, symbol-ledger `mods`
//! domain) and `refreshDiscoveredPlugins`
//! (`AppViewModel+ComponentsVersions.swift:114-205`, symbol-ledger
//! `plugins` domain), minus every field only Phase 8's Modrinth/GitHub/
//! Hangar update-resolution feature needs (`PluginEntry.tier`/
//! `sourceConfig`/online-version fields — [`msc_domain::crash_analysis
//! ::PluginEntry`]'s own doc explains why). This module builds only the
//! inventory-reading half: enough for [`msc_domain::crash_analysis::analyze`]/
//! [`msc_domain::crash_analysis::analyze_paper_plugins`] to attribute a
//! startup problem to an installed jar. Add/remove/toggle/download/
//! Modrinth-resolution stay Phase 8.
//!
//! **Mods read jar-internal metadata; plugins don't — confirmed by
//! reading both scanners directly.** `refreshDiscoveredMods` parses
//! `fabric.mod.json`/`META-INF/mods.toml` via `ModJarMetadataParser`,
//! falling back to filename heuristics (`PluginNameParser`) only when no
//! manifest is found. `refreshDiscoveredPlugins` never calls
//! `ModJarMetadataParser` at all — Paper/Bukkit plugin discovery is
//! filename-heuristic only (`plugin.yml` reading exists in source, but
//! only for the separate add-on-update-resolver/client-export features,
//! never this scan). [`scan_plugins`] preserves that asymmetry rather
//! than "improving" plugins to read `plugin.yml` too.
//!
//! **Zip reading is a native library call, not `unzip -p`.** MSC 1's
//! `ModJarMetadataParser.extractFromZip` shells out to `/usr/bin/unzip`;
//! this port uses [`msc_infrastructure::archive::read_entry_bytes`]
//! (already the established `msc-application` pattern —
//! `provisioning.rs`'s `imported_metadata_from_zip` composes it the same
//! way, a real path alongside a `&dyn FileSystem`-based directory list in
//! the same function) since a real zip-reading crate is already in this
//! crate's dependency graph. The *outcome* MSC 1's `nil`-on-any-failure
//! behavior produces — "can't read this jar's manifest, fall back to
//! filename heuristics" — is preserved exactly; only the mechanism
//! differs.
//!
//! **Malformed archives, duplicates, and path containment — MSC 1 has no
//! oracle behavior for any of the three** (confirmed by reading both
//! scanners: no corrupt-archive branch, no dedup logic, and
//! `FileManager.contentsOfDirectory` makes a hostile filename structurally
//! impossible in a local macOS app). Decided for this port, since MSC 2 is
//! a headless, remotely-driven agent rather than a local app: a jar this
//! module can't open or whose expected manifest entry is absent/malformed
//! degrades to the same filename-heuristic fallback a jar with no
//! manifest gets (never a hard error — one bad jar must never blank the
//! whole inventory); two jars that produce the same stem/id are **not**
//! deduplicated, listed as two separate entries exactly as MSC 1's own
//! non-deduplicating `.map` would; and every listed entry's filename is
//! re-validated ([`safe_file_name`]) to contain no path separators before
//! being joined into a destination path, a defense `fs.list()` should
//! never need but costs nothing to keep.
//!
//! Hand-rolled TOML line-scanning (not a `toml` crate dependency) mirrors
//! `ModJarMetadataParser.parseModsToml`/`tomlStringValue` byte-for-byte:
//! source itself never uses a real TOML parser either (`.toml` has no
//! Foundation decoder), just a `[[mods]]`-section walk reading `key =
//! "value"`/`key='value'` assignments — porting a full TOML grammar here
//! would be *more* capable than the oracle, not more faithful to it.

use msc_domain::crash_analysis::{ModEntry, PluginEntry};
use msc_infrastructure::archive;
use msc_infrastructure::fs::FileSystem;
use std::path::Path;

/// `refreshDiscoveredMods` (source doc above). Scans `mods_dir` for
/// `.jar`/`.jar.disabled` entries, reads each jar's `fabric.mod.json`
/// then `META-INF/mods.toml` (first hit wins, matching
/// `ModJarMetadataParser.parse`'s own try-Fabric-then-Forge order), and
/// falls back to filename heuristics for whichever of
/// `mod_id`/`display_name`/`version` the manifest didn't provide (or when
/// no manifest was found at all). Sorted by `display_name`, matching
/// source's own `.sorted { $0.displayName.lowercased() < ... }`.
pub fn scan_mods(fs: &dyn FileSystem, mods_dir: &Path) -> Vec<ModEntry> {
    let Ok(entries) = fs.list(mods_dir) else {
        return Vec::new();
    };
    let mut mods: Vec<ModEntry> = entries
        .into_iter()
        .filter_map(|path| {
            let filename = safe_file_name(&path)?;
            if !is_addon_jar_filename(&filename) {
                return None;
            }
            if !fs.stat(&path).map(|m| m.is_file).unwrap_or(false) {
                return None;
            }
            let (is_enabled, jar_stem) = enabled_and_stem(&filename);
            let (mod_id, manifest_name, manifest_version) =
                mod_jar_metadata(&path).unwrap_or((None, None, None));
            let display_name = manifest_name.unwrap_or_else(|| extract_display_name(&jar_stem));
            let version = manifest_version.or_else(|| extract_version(&jar_stem));
            Some(ModEntry {
                filename,
                jar_stem,
                display_name,
                mod_id,
                version,
                is_enabled,
            })
        })
        .collect();
    mods.sort_by_key(|m| m.display_name.to_lowercase());
    mods
}

/// `refreshDiscoveredPlugins` (source doc above) — filename-heuristic
/// only, no jar-internal read. Sorted by `display_name`, matching this
/// module's own `scan_mods` (source's `refreshDiscoveredPlugins` sorts
/// managed-plugins-first via `tier`, a Phase 8 field this port doesn't
/// carry — alphabetical by display name is the closest faithful ordering
/// without it).
pub fn scan_plugins(fs: &dyn FileSystem, plugins_dir: &Path) -> Vec<PluginEntry> {
    let Ok(entries) = fs.list(plugins_dir) else {
        return Vec::new();
    };
    let mut plugins: Vec<PluginEntry> = entries
        .into_iter()
        .filter_map(|path| {
            let filename = safe_file_name(&path)?;
            if !is_addon_jar_filename(&filename) {
                return None;
            }
            if !fs.stat(&path).map(|m| m.is_file).unwrap_or(false) {
                return None;
            }
            let (is_enabled, jar_stem) = enabled_and_stem(&filename);
            let display_name = extract_display_name(&jar_stem);
            let version = extract_version(&jar_stem);
            Some(PluginEntry {
                filename,
                jar_stem,
                display_name,
                version,
                is_enabled,
            })
        })
        .collect();
    plugins.sort_by_key(|p| p.display_name.to_lowercase());
    plugins
}

/// A listed entry's own filename, re-validated to contain no path
/// separators — see this module's own doc on why. `None` skips the entry
/// entirely rather than risking a path escaping `mods_dir`/`plugins_dir`.
fn safe_file_name(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    if name.contains('/') || name.contains('\\') || name == ".." || name == "." {
        return None;
    }
    Some(name.to_string())
}

fn is_addon_jar_filename(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".jar") || lower.ends_with(".jar.disabled")
}

/// `(isEnabled, jarStem)` from a `.jar`/`.jar.disabled` filename — both
/// scanners derive this identically (source lines 165-168, 142-145).
fn enabled_and_stem(filename: &str) -> (bool, String) {
    let lower = filename.to_lowercase();
    if let Some(len) = lower.strip_suffix(".jar.disabled").map(str::len) {
        (false, filename[..len].to_string())
    } else {
        let len = lower.len() - ".jar".len();
        (true, filename[..len].to_string())
    }
}

// ---------------------------------------------------------------------
// PluginNameParser (filename heuristics, shared by mods and plugins)
// ---------------------------------------------------------------------

/// `PluginNameParser.extractDisplayName(from:)` (`PluginNameParser.swift:
/// 22-32`): accumulates `-`-separated stem parts until one looks like a
/// version, joining what came before. Falls back to the whole stem when
/// nothing was accumulated (the first part already looked like a
/// version).
pub fn extract_display_name(stem: &str) -> String {
    let mut name_parts = Vec::new();
    for part in stem.split('-') {
        if looks_like_version_component(part) {
            break;
        }
        name_parts.push(part);
    }
    let name = name_parts.join("-");
    if name.is_empty() {
        stem.to_string()
    } else {
        name
    }
}

/// `PluginNameParser.extractVersion(from:)` (`PluginNameParser.swift:
/// 35-45`): the symmetric walk — once a version-looking part is seen,
/// collects it and everything after.
pub fn extract_version(stem: &str) -> Option<String> {
    let mut version_parts = Vec::new();
    let mut collecting = false;
    for part in stem.split('-') {
        if looks_like_version_component(part) {
            collecting = true;
        }
        if collecting {
            version_parts.push(part);
        }
    }
    let version = version_parts.join("-");
    if version.is_empty() {
        None
    } else {
        Some(version)
    }
}

/// `looksLikeVersionComponent(_:)` (`PluginNameParser.swift:51-59`).
/// Swift's `Character.isNumber` covers Unicode numerics; jar filenames in
/// practice are ASCII, so this checks `is_ascii_digit` — a deliberate,
/// documented narrowing rather than a behavior this port claims to match
/// byte-for-byte on non-ASCII input no real filename produces.
fn looks_like_version_component(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first.is_ascii_digit() {
        return true;
    }
    let lower_first = first.to_ascii_lowercase();
    if lower_first == 'v' || lower_first == 'b' {
        return chars.next().is_some_and(|c| c.is_ascii_digit());
    }
    false
}

// ---------------------------------------------------------------------
// ModJarMetadataParser (fabric.mod.json / META-INF/mods.toml)
// ---------------------------------------------------------------------

type ModManifest = (Option<String>, Option<String>, Option<String>); // (mod_id, display_name, version)

/// `ModJarMetadataParser.parse(jarURL:)` (`ModJarMetadataParser.swift:
/// 28-32`): Fabric first, then Forge/NeoForge. `jar_path` must be a real
/// path on disk — see this module's own doc on why `archive::
/// read_entry_bytes` can't go through the `FileSystem` trait.
fn mod_jar_metadata(jar_path: &Path) -> Option<ModManifest> {
    if let Ok(Some(bytes)) = archive::read_entry_bytes(jar_path, "fabric.mod.json")
        && let Some(meta) = parse_fabric_mod_json(&bytes)
    {
        return Some(meta);
    }
    if let Ok(Some(bytes)) = archive::read_entry_bytes(jar_path, "META-INF/mods.toml")
        && let Ok(text) = String::from_utf8(bytes)
        && let Some(meta) = parse_mods_toml(&text)
    {
        return Some(meta);
    }
    None
}

/// `parseFabric(jarURL:)` (`ModJarMetadataParser.swift:109-120`). `None`
/// on invalid JSON or when neither `id` nor `name` is present — a version
/// starting with `"${"` (an unresolved Gradle template token) is dropped,
/// matching source's own `flatMap`.
fn parse_fabric_mod_json(bytes: &[u8]) -> Option<ModManifest> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let mod_id = value.get("id").and_then(|v| v.as_str()).map(str::to_string);
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let version = value
        .get("version")
        .and_then(|v| v.as_str())
        .filter(|v| !v.starts_with("${"))
        .map(str::to_string);
    if mod_id.is_none() && name.is_none() {
        return None;
    }
    Some((mod_id, name, version))
}

/// `parseModsToml(_:)` (`ModJarMetadataParser.swift:133-157`): a
/// `[[mods]]`-section line scanner, not a real TOML parser (this module's
/// own doc). Only ever reads the *first* `[[mods]]` block — stops as soon
/// as it hits the next `[[` section header while already holding
/// `modId`/`displayName`.
fn parse_mods_toml(text: &str) -> Option<ModManifest> {
    let mut in_mods_section = false;
    let mut mod_id: Option<String> = None;
    let mut display_name: Option<String> = None;
    let mut version: Option<String> = None;

    for line in text.split('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with("[[") {
            if in_mods_section && (mod_id.is_some() || display_name.is_some()) {
                break;
            }
            in_mods_section = trimmed == "[[mods]]";
            continue;
        }
        if !in_mods_section {
            continue;
        }
        if let Some(v) = toml_string_value(trimmed, "modId") {
            mod_id = Some(v);
        }
        if let Some(v) = toml_string_value(trimmed, "displayName") {
            display_name = Some(v);
        }
        if let Some(v) = toml_string_value(trimmed, "version") {
            version = if v.starts_with("${") { None } else { Some(v) };
        }
    }

    if mod_id.is_none() && display_name.is_none() {
        return None;
    }
    Some((mod_id, display_name, version))
}

/// `tomlStringValue(line:key:)` (`ModJarMetadataParser.swift:178-195`):
/// `key = "value"` or `key='value'` (single or double quotes), taking
/// everything up to the matching quote character.
fn toml_string_value(line: &str, key: &str) -> Option<String> {
    let spaced = format!("{key} = ");
    let tight = format!("{key}=");
    let rest = line
        .strip_prefix(spaced.as_str())
        .or_else(|| line.strip_prefix(tight.as_str()))?;
    let rest = rest.trim_start_matches(' ');
    let mut chars = rest.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let after_quote = &rest[quote.len_utf8()..];
    let end = after_quote.find(quote)?;
    Some(after_quote[..end].to_string())
}
