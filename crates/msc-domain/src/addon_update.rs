//! Add-on identity, update-bucket assignment, discovered-link merging, and
//! plugin-source tier/rekey/sort rules.
//!
//! Ported from `AddonUpdateResolver.swift`,
//! `AppViewModel+AddonUpdates.swift::mergeDiscoveredLinks`, and
//! `AppViewModel+ComponentsVersions.swift`/`AppViewModel+PluginManagement.swift`'s
//! plugin-source-tier/rekey/sort logic, per `docs/msc2/addons/phase8-scope.md`
//! and `fixtures/addon-update-resolution/` + `fixtures/plugin-source-mapping/`
//! (P8.5). Every function here is a pure decision over already-fetched/
//! already-computed inputs (hash lookups, persisted links, disk listings) --
//! the real orchestration (hashing files, calling Modrinth, holding a
//! per-server plan cache) is `msc-application`'s job (P8.16/P8.17).

use crate::app_config_schema::{AddonLink, AddonLinkProvenance, PluginSourceConfig};
use crate::identity::AddOnKind;
use std::collections::HashMap;

// --- File enumeration / jar stem ---

/// `AddonUpdateResolver`'s directory filter (line 112-117): accepts
/// anything ending `.jar` or `.jar.disabled`, case-insensitively.
pub fn is_addon_file(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.ends_with(".jar") || lower.ends_with(".jar.disabled")
}

/// `DiskFile.jarStem` (line 120-126): an enabled file only has its `.jar`
/// extension stripped; a disabled file has the full `.jar.disabled` suffix
/// dropped, converging on the same stem regardless of enabled state.
pub fn jar_stem(filename: &str) -> String {
    let lower = filename.to_lowercase();
    if lower.ends_with(".jar.disabled") {
        filename[..filename.len() - ".jar.disabled".len()].to_string()
    } else if lower.ends_with(".jar") {
        filename[..filename.len() - ".jar".len()].to_string()
    } else {
        filename.to_string()
    }
}

/// `AddonUpdateResolver`'s plugin-server-only Geyser/Floodgate exclusion
/// (line 129-138): mod servers keep them in the resolve pass since
/// Fabric/NeoForge builds have no dedicated updater.
pub fn should_exclude_from_hash_resolution(add_on_kind: AddOnKind, stem: &str) -> bool {
    if add_on_kind != AddOnKind::Plugin {
        return false;
    }
    let lower = stem.to_lowercase();
    lower.contains("geyser") || lower.contains("floodgate")
}

/// Lines 153-158: `versionsFromHashes` and `latestVersionsForHashes` run
/// concurrently via `async let`, both awaited -- a folder of any size costs
/// exactly this many Modrinth requests per resolve pass, not one per file.
/// Recorded as a constant so P8.16's async implementation has a concrete
/// invariant to hold itself to, per `fixtures/addon-update-resolution/hash-identify-and-latest-lookup-are-concurrent-not-sequential.json`.
pub const RESOLVE_PASS_MODRINTH_REQUEST_COUNT: u32 = 2;

// --- Identity resolution ---

/// Lines 174-177's three-way fallback chain: a fresh exact-hash Modrinth
/// match always wins; only when it misses does a persisted link matching
/// the file's current hash get a look, then a persisted link matching the
/// current filename, in that order. `None` (all three miss) is `.unlinked`.
pub fn resolve_project_id<'a>(
    fresh_hash_match: Option<&'a str>,
    persisted_by_hash: Option<&'a str>,
    persisted_by_filename: Option<&'a str>,
) -> Option<&'a str> {
    fresh_hash_match
        .or(persisted_by_hash)
        .or(persisted_by_filename)
}

/// Line 182: provenance surfaced to the UI reflects THIS pass's
/// identification method for a fresh hit, but preserves whatever the
/// persisted link already recorded when this pass didn't independently
/// re-confirm it by hash.
pub fn resolve_provenance(
    fresh_hash_match: bool,
    persisted_provenance: Option<AddonLinkProvenance>,
) -> AddonLinkProvenance {
    if fresh_hash_match {
        AddonLinkProvenance::HashDetected
    } else {
        persisted_provenance.unwrap_or(AddonLinkProvenance::UserLinked)
    }
}

/// Line 185: a self-healing link write only happens off a genuine fresh
/// hash-identify hit -- an item resolved purely through a persisted
/// fallback clause produces no `discoveredLinks` entry.
pub fn should_record_self_healing_link(fresh_hash_match: bool) -> bool {
    fresh_hash_match
}

/// Line 252-253: an unlinked jar's embedded metadata name (when parseable)
/// is trusted over the cruder filename heuristic.
pub fn unlinked_name_guess(
    embedded_display_name: Option<&str>,
    filename_heuristic: &str,
) -> String {
    embedded_display_name
        .filter(|s| !s.is_empty())
        .unwrap_or(filename_heuristic)
        .to_string()
}

// --- Update bucket assignment ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonUpdateBucket {
    UpdateAvailable,
    NoCompatibleVersion,
    UpToDate,
    Unlinked,
}

impl AddonUpdateBucket {
    /// `AddonUpdateBucket.sortRank` (line 283-291).
    pub fn sort_rank(self) -> u8 {
        match self {
            Self::UpdateAvailable => 0,
            Self::NoCompatibleVersion => 1,
            Self::UpToDate => 2,
            Self::Unlinked => 3,
        }
    }
}

/// Lines 211-231: bucket assignment for an item that resolved to a real
/// project id (an `.unlinked` item never reaches this function at all).
/// `latest_version_id` is the latest-compatible build's id for this hash,
/// if any; `current_version_id` is `idVersion?.id ?? link?.installedVersionId`
/// -- the fresh hit if this pass had one, else the persisted value, per
/// [`resolve_current_version_id`]. Returns the bucket and, for
/// `UpdateAvailable`, the available version id.
pub fn resolve_bucket(
    latest_version_id: Option<&str>,
    current_version_id: Option<&str>,
    mc_version_configured: bool,
) -> (AddonUpdateBucket, Option<String>) {
    match latest_version_id {
        Some(latest) if Some(latest) != current_version_id => {
            (AddonUpdateBucket::UpdateAvailable, Some(latest.to_string()))
        }
        Some(_) => (AddonUpdateBucket::UpToDate, None),
        None if mc_version_configured => (AddonUpdateBucket::NoCompatibleVersion, None),
        None => (AddonUpdateBucket::UpToDate, None),
    }
}

/// Line 212: `idVersion?.id ?? link?.installedVersionId` -- the comparison
/// falls back to the PERSISTED installed version id when this pass had no
/// fresh hash-identify hit.
pub fn resolve_current_version_id<'a>(
    fresh_hash_version_id: Option<&'a str>,
    persisted_installed_version_id: Option<&'a str>,
) -> Option<&'a str> {
    fresh_hash_version_id.or(persisted_installed_version_id)
}

/// Lines 275-278 + `sortRank`: bucket rank first, then case-insensitive
/// display name within an equal rank.
pub fn addon_update_sort_key(display_name: &str, bucket: AddonUpdateBucket) -> (u8, String) {
    (bucket.sort_rank(), display_name.to_lowercase())
}

// --- Stale-plan cache policy ---

/// `resolveAddonUpdates(for:force:)`'s cache-hit guard (line 27-29):
/// `!force && addonPlanServerId == cfg.id` short-circuits, so recompute is
/// needed exactly when this is false.
pub fn should_recompute_addon_plan(
    cached_server_id: Option<&str>,
    current_server_id: &str,
    force: bool,
) -> bool {
    force || cached_server_id != Some(current_server_id)
}

// --- cleanVersionLabel ---

const KNOWN_LOADER_PREFIXES: &[&str] = &[
    "bukkit",
    "spigot",
    "paper",
    "purpur",
    "folia",
    "fabric",
    "forge",
    "neoforge",
    "quilt",
    "velocity",
    "bungeecord",
    "waterfall",
];

fn looks_version_shaped(rest: &str) -> bool {
    let mut chars = rest.chars();
    match chars.next() {
        Some(c) if c.is_ascii_digit() => true,
        Some('v') => chars.next().is_some_and(|c| c.is_ascii_digit()),
        _ => false,
    }
}

/// `cleanVersionLabel(_:)` (line 298-311): strips a known loader prefix
/// (`<loader>-`) only when what remains looks version-shaped (starts with
/// a digit, or `v` followed by a digit) -- so project names aren't
/// mangled. A string matching no known prefix, or whose remainder isn't
/// version-shaped, is returned completely unchanged.
pub fn clean_version_label(raw: &str) -> String {
    let lower = raw.to_lowercase();
    for prefix in KNOWN_LOADER_PREFIXES {
        let with_dash = format!("{prefix}-");
        if lower.starts_with(&with_dash) {
            let rest = &raw[with_dash.len()..];
            if looks_version_shaped(rest) {
                return rest.to_string();
            }
        }
    }
    raw.to_string()
}

// --- mergeDiscoveredLinks ---

/// `mergeDiscoveredLinks(_:into:)` (line 59-72): when the prior persisted
/// link's provenance is `userLinked`, the merge keeps `title`/`slug`/
/// `provenance`/`iconURL` from the PRIOR entry, refreshes
/// `installedVersionId`/`installedFileName`/`installedHash` from the
/// discovered entry, and falls back to the prior `clientSide`/`serverSide`
/// only when the discovered pass couldn't determine them (line 66-67).
/// Any other prior provenance (or no prior at all) is a wholesale
/// overwrite (line 71-72): the discovered entry replaces it entirely.
pub fn merge_discovered_link(prior: Option<&AddonLink>, discovered: &AddonLink) -> AddonLink {
    match prior {
        Some(p) if p.provenance == AddonLinkProvenance::UserLinked => AddonLink {
            project_id: discovered.project_id.clone(),
            title: p.title.clone(),
            slug: p.slug.clone(),
            icon_url: p.icon_url.clone(),
            provenance: AddonLinkProvenance::UserLinked,
            installed_version_id: discovered.installed_version_id.clone(),
            installed_file_name: discovered.installed_file_name.clone(),
            installed_hash: discovered.installed_hash.clone(),
            client_side: discovered
                .client_side
                .clone()
                .or_else(|| p.client_side.clone()),
            server_side: discovered
                .server_side
                .clone()
                .or_else(|| p.server_side.clone()),
            extra: p.extra.clone(),
        },
        _ => discovered.clone(),
    }
}

// --- Plugin-source tiers, findSource, rekey, sort ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginTier {
    Managed,
    UserSourced,
    Unmanaged,
}

/// `findSource(for:in:)` (line 209-216): an exact key match short-circuits
/// before the prefix loop ever runs; the prefix check is symmetric (either
/// the current stem or the recorded key may be the longer one).
pub fn find_source<'a>(
    jar_stem: &str,
    sources: &'a HashMap<String, PluginSourceConfig>,
) -> Option<&'a PluginSourceConfig> {
    if let Some(exact) = sources.get(jar_stem) {
        return Some(exact);
    }
    let lower = jar_stem.to_lowercase();
    sources
        .iter()
        .find(|(k, _)| {
            let kl = k.to_lowercase();
            lower.starts_with(&kl) || kl.starts_with(&lower)
        })
        .map(|(_, v)| v)
}

/// Tier derivation (line 151-159): the Geyser/Floodgate managed check
/// short-circuits BEFORE `findSource` is even consulted, so a Geyser jar
/// is always `.managed` even if a stray `pluginSources` entry exists for
/// it.
pub fn derive_plugin_tier(
    jar_stem: &str,
    sources: &HashMap<String, PluginSourceConfig>,
) -> PluginTier {
    let lower = jar_stem.to_lowercase();
    if lower.contains("geyser") || lower.contains("floodgate") {
        return PluginTier::Managed;
    }
    if find_source(jar_stem, sources).is_some() {
        return PluginTier::UserSourced;
    }
    PluginTier::Unmanaged
}

/// Lines 188-196's full comparator: tier rank first, then (within the
/// managed tier only) a hardcoded Geyser-before-Floodgate sub-order, then
/// case-insensitive display name.
pub fn plugin_entry_sort_key(
    jar_stem: &str,
    display_name: &str,
    tier: PluginTier,
) -> (PluginTier, u8, String) {
    let managed_subrank =
        if tier == PluginTier::Managed && !jar_stem.to_lowercase().contains("geyser") {
            1
        } else {
            0
        };
    (tier, managed_subrank, display_name.to_lowercase())
}

/// The stale-prefix sweep shared by [`set_plugin_source`] and
/// [`remove_plugin_source`] (lines 221-224 / 240-243): the SAME symmetric
/// prefix check `find_source` uses, applied as a removal filter against
/// `jar_stem` rather than a lookup.
fn strip_prefix_related(sources: &mut HashMap<String, PluginSourceConfig>, jar_stem: &str) {
    let lower = jar_stem.to_lowercase();
    sources.retain(|k, _| {
        let kl = k.to_lowercase();
        !(lower.starts_with(&kl) || kl.starts_with(&lower))
    });
}

/// `setPluginSource(jarStem:url:type:)` (line 214-229): strips any
/// prefix-related stale entry, then writes the new key -- create and
/// replace share this one path.
pub fn set_plugin_source(
    sources: &mut HashMap<String, PluginSourceConfig>,
    jar_stem: &str,
    config: PluginSourceConfig,
) {
    strip_prefix_related(sources, jar_stem);
    sources.insert(jar_stem.to_string(), config);
}

/// `removePluginSource(jarStem:)` (line 232-245): removes the exact key,
/// then sweeps prefix-related entries the same way, then collapses an
/// empty result to `None` so the persisted field stays genuinely absent
/// rather than an empty object.
pub fn remove_plugin_source(
    mut sources: HashMap<String, PluginSourceConfig>,
    jar_stem: &str,
) -> Option<HashMap<String, PluginSourceConfig>> {
    sources.remove(jar_stem);
    strip_prefix_related(&mut sources, jar_stem);
    if sources.is_empty() {
        None
    } else {
        Some(sources)
    }
}

/// `downloadLatestForPlugin`'s stale-jar cleanup sweep (line 343-352): run
/// BEFORE moving the freshly-downloaded temp file into its final name,
/// this removes every existing enabled-or-disabled jar whose lowercased
/// name has the entry's lowercased display name as a prefix -- both the
/// active and disabled prior copies, not just the currently-active one.
pub fn stale_jars_to_remove(existing_files: &[String], display_name: &str) -> Vec<String> {
    let prefix = display_name.to_lowercase();
    existing_files
        .iter()
        .filter(|f| {
            let lower = f.to_lowercase();
            is_addon_file(f) && lower.starts_with(&prefix)
        })
        .cloned()
        .collect()
}

/// `downloadLatestForPlugin`'s `finalName` derivation (line 335-337): the
/// download URL's own last path segment is used verbatim when it already
/// ends in `.jar` (preserving casing); otherwise a name is synthesized
/// from the entry's display name and fetched version, falling back to the
/// literal word "latest" when no version string is known.
pub fn plugin_final_filename(
    download_url: &str,
    display_name: &str,
    online_version: Option<&str>,
) -> String {
    let last = download_url.rsplit('/').next().unwrap_or("");
    if last.to_lowercase().ends_with(".jar") {
        last.to_string()
    } else {
        format!("{display_name}-{}.jar", online_version.unwrap_or("latest"))
    }
}

/// `downloadLatestForPlugin`'s post-download rekey (line 355-368): `None`
/// when the newly-downloaded file's stem matches the entry's existing
/// stem (rekey skipped); `Some(new_stem)` otherwise.
pub fn plugin_source_rekey(entry_jar_stem: &str, final_filename: &str) -> Option<String> {
    let new_stem = jar_stem(final_filename);
    if new_stem != entry_jar_stem {
        Some(new_stem)
    } else {
        None
    }
}

/// `downloadPluginWithSourceCheck`'s `.direct` short-circuit (line 262-270):
/// a direct source skips `fetchOnlineVersion` entirely and dispatches
/// straight to download with the literal `"(direct)"` version string;
/// every other source type goes through the async online-version check
/// first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginVersionDispatch {
    DirectImmediate { version: &'static str },
    FetchOnlineFirst,
}

pub fn plugin_version_dispatch(
    source_type: crate::app_config_schema::PluginSourceKind,
) -> PluginVersionDispatch {
    use crate::app_config_schema::PluginSourceKind;
    match source_type {
        PluginSourceKind::Direct => PluginVersionDispatch::DirectImmediate {
            version: "(direct)",
        },
        PluginSourceKind::Github | PluginSourceKind::Modrinth | PluginSourceKind::Hangar => {
            PluginVersionDispatch::FetchOnlineFirst
        }
    }
}

/// `updateManagedPluginOnlineVersions` (line 220-232): managed-tier
/// entries never call a provider API for their own online version -- it's
/// mirrored from whichever Geyser/Floodgate build-check snapshot the
/// Components tab already computed separately.
pub fn managed_plugin_online_version(
    jar_stem: &str,
    geyser_online: &str,
    floodgate_online: &str,
) -> Option<String> {
    let lower = jar_stem.to_lowercase();
    if lower.contains("geyser") {
        Some(geyser_online.to_string())
    } else if lower.contains("floodgate") {
        Some(floodgate_online.to_string())
    } else {
        None
    }
}
