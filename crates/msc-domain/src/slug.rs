//! Modrinth slug/alias resolution for mapping a loader mod-id (as it
//! appears in a Forge/Fabric crash log) to the Modrinth project slug it
//! actually lives under.
//!
//! Ported from `ModrinthSlugNormalizer.swift`. Forge internal ids are NOT
//! Modrinth slugs — a crash names `connectormod`, but the project is
//! `connector`; `kotlinforforge` -> `kotlin-for-forge`. The Fabric-API
//! alias is loader-conditional: on a Forge/NeoForge server (running Fabric
//! mods through Sinytra Connector) the server-usable project is
//! `forgified-fabric-api`, but on a real Fabric/Quilt server `fabric-api`
//! is already correct and must NOT be rewritten — callers pass
//! `forge_family` so the same offender resolves correctly per server type.

/// Aliases that hold regardless of server loader.
const COMMON_ALIASES: &[(&str, &str)] = &[
    ("connectormod", "connector"),
    ("connector-mod", "connector"),
    ("kotlinforforge", "kotlin-for-forge"),
    ("kotlin-for-forge", "kotlin-for-forge"),
];

/// Aliases that apply ONLY on Forge-family servers (Forge/NeoForge). On
/// Fabric/Quilt these must be left alone — `fabric-api` is the correct
/// project there.
const FORGE_FAMILY_ALIASES: &[(&str, &str)] = &[
    ("fabric-api", "forgified-fabric-api"),
    ("fabricapi", "forgified-fabric-api"),
    ("fabric-api-base", "forgified-fabric-api"),
    ("forgified-fabric-api", "forgified-fabric-api"),
];

/// Lowercases and collapses every run of non-alphanumerics to a single
/// dash, trimming leading/trailing dashes. "Fabric API" -> "fabric-api";
/// "connector_mod" -> "connector-mod".
pub fn normalized_slug(raw: &str) -> String {
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

/// The best-known Modrinth slug for a raw mod-id/name. Applies the common
/// alias table unconditionally, then the Forge-family alias table only
/// when `forge_family` is true. Returns the plain normalized slug when no
/// alias matches.
pub fn canonical_slug(raw: &str, forge_family: bool) -> String {
    let normalized = normalized_slug(raw);
    if let Some((_, alias)) = COMMON_ALIASES.iter().find(|(k, _)| *k == normalized) {
        return alias.to_string();
    }
    if forge_family
        && let Some((_, alias)) = FORGE_FAMILY_ALIASES.iter().find(|(k, _)| *k == normalized)
    {
        return alias.to_string();
    }
    normalized
}

/// The search-query form: the canonical slug, or the raw text when
/// normalization empties it.
pub fn search_query(raw: &str, forge_family: bool) -> String {
    let canonical = canonical_slug(raw, forge_family);
    if canonical.is_empty() {
        raw.to_string()
    } else {
        canonical
    }
}

/// True when a slug was rewritten by an alias (i.e. the canonical form is a
/// *known* project, not just a normalized guess) — the identity ladder
/// trusts these directly.
pub fn is_known_alias(raw: &str, forge_family: bool) -> bool {
    canonical_slug(raw, forge_family) != normalized_slug(raw)
}
