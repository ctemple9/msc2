//! Client-only classification (the manifest/Modrinth/embedded-jar/known-list
//! precedence), the disabled-jar sibling path, Modrinth project-id
//! extraction from a mirror list, and the pack-managed mutation guard.
//!
//! Ported from `ModpackClientOnlyClassifier.swift`, per
//! `docs/msc2/addons/phase8-scope.md` and `fixtures/modpack-client-only/`
//! (P6/P7 + P8.6's two closed gaps). The pack-managed guard has no MSC 1
//! source to port at all -- `phase8-scope.md`'s own "Pack-managed guard"
//! finding confirms production code checks `packManaged` nowhere before a
//! mutation; [`pack_mutation_refused`]/[`pack_replace_refused`] are new,
//! agent-owned policy characterized against the decided contract in
//! `fixtures/pack-managed-guard/`, cited there against `phase8-scope.md`/
//! `rolling-plan.md` rather than an oracle file/line.
//!
//! Two precedence chains, not one (`phase8-scope.md`'s central finding for
//! this domain): a manifest-listed `.mrpack` entry is filtered by
//! [`is_manifest_server_unsupported`] alone, pre-download -- Tier 0's
//! hardcoded blocklist is never consulted for it. An override-tree or
//! CurseForge-imported jar (no per-file manifest env available) instead
//! checks [`known_client_only_reason`] (Tier 0) first, falling back to
//! [`client_only_reason`] (Tier 2 Modrinth / Tier 3 embedded jar) only when
//! Tier 0 doesn't match. Both paths are real MSC 2 call sites; this module
//! doesn't collapse them into one shared "try all four tiers in order"
//! function, since that isn't what either real path does.

/// `MrpackEnv` (per-file manifest environment declaration).
#[derive(Debug, Clone)]
pub struct MrpackEnv {
    pub client: Option<String>,
    pub server: Option<String>,
}

/// `isManifestServerUnsupported(_:)`: a case-insensitive check against the
/// manifest's own `server` field. A missing env block, or one with a nil
/// `server`, is not server-unsupported.
pub fn is_manifest_server_unsupported(env: Option<&MrpackEnv>) -> bool {
    env.and_then(|e| e.server.as_deref())
        .map(|s| s.eq_ignore_ascii_case("unsupported"))
        .unwrap_or(false)
}

/// `isModsJar(_:)`: case-insensitive on both the `mods/` directory prefix
/// and the `.jar` extension.
pub fn is_mods_jar(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.starts_with("mods/") && lower.ends_with(".jar")
}

const TIER0_PREFIXES: &[&str] = &[
    "oculus",
    "mekalus",
    "rubidium",
    "embeddium",
    "iris",
    "optifine",
    "optifabric",
];

/// Tier 0: `knownClientOnlyReason(forJarStem:)`, the hardcoded
/// shader/renderer blocklist. Matches an exact stem, or a stem followed by
/// one of exactly three separators (`-`, `_`, `+`) then anything -- never
/// a bare substring, so `"irisshaders-companion-mod"` correctly does NOT
/// match `"iris"`.
pub fn known_client_only_reason(jar_stem: &str) -> Option<String> {
    let lower = jar_stem.to_lowercase();
    for prefix in TIER0_PREFIXES {
        let is_match = lower == *prefix
            || lower.starts_with(&format!("{prefix}-"))
            || lower.starts_with(&format!("{prefix}_"))
            || lower.starts_with(&format!("{prefix}+"));
        if is_match {
            return Some(format!("Known client-only shader/renderer mod ({prefix})."));
        }
    }
    None
}

/// Tier 2 (Modrinth `server_side`) then Tier 3 (embedded jar
/// `environment`): Modrinth wins whenever it has an opinion at all --
/// `server_side` present and non-empty and not `"unsupported"` keeps the
/// mod enabled even if the jar's own manifest says `environment=client`,
/// and `server_side == "unsupported"` disables it even if the jar's own
/// manifest says `environment=server`. An empty-string `server_side` is
/// treated as "Modrinth has no signal," not as a keep-enabled verdict, so
/// the jar-environment fallback still applies in that case.
pub fn client_only_reason(
    modrinth_server_side: Option<&str>,
    modrinth_project_title: Option<&str>,
    jar_environment: Option<&str>,
) -> Option<String> {
    let has_modrinth_signal = modrinth_server_side.is_some_and(|s| !s.is_empty());
    if has_modrinth_signal {
        let side = modrinth_server_side.unwrap();
        if side == "unsupported" {
            let title = modrinth_project_title.unwrap_or("This mod");
            return Some(format!("{title} is marked server-unsupported on Modrinth."));
        }
        return None;
    }
    if jar_environment == Some("client") {
        return Some("The jar's embedded fabric.mod.json declares environment=client.".to_string());
    }
    None
}

/// `disabledURL(forActiveJar:)` (line 122-124): pure, filesystem-free --
/// `appendingPathExtension("disabled")` ADDS a component rather than
/// replacing the existing `.jar` one.
pub fn disabled_url(jar_url: &str) -> String {
    format!("{jar_url}.disabled")
}

/// `disableJar(at:fm:)`'s decision (line 131-146), separated from its
/// filesystem effect: given whether the active jar and a `.disabled`
/// sibling currently exist, which action to take. `msc-infrastructure`
/// (P8.14) performs the actual rename/delete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisableJarAction {
    /// No active jar at that path; nothing to disable.
    NoOp,
    /// Rename the active jar to its `.disabled` sibling.
    Rename,
    /// A `.disabled` sibling already exists -- never clobber it; drop the
    /// freshly-downloaded active duplicate instead.
    DropActiveKeepExistingDisabled,
}

pub fn decide_disable_jar_action(active_exists: bool, disabled_exists: bool) -> DisableJarAction {
    if !active_exists {
        DisableJarAction::NoOp
    } else if disabled_exists {
        DisableJarAction::DropActiveKeepExistingDisabled
    } else {
        DisableJarAction::Rename
    }
}

/// `modrinthProjectId(fromDownloadURLs:)` (line 81-105): extracts the
/// project id from the first `cdn.modrinth.com/data/{id}/...` URL among a
/// mirror list, preferring it regardless of list order; a mirror missing
/// the `data` path segment, or hosted elsewhere, contributes nothing.
pub fn modrinth_project_id(download_urls: &[String]) -> Option<String> {
    download_urls
        .iter()
        .find_map(|u| modrinth_project_id_from_url(u))
}

fn modrinth_project_id_from_url(url: &str) -> Option<String> {
    if !url.contains("cdn.modrinth.com") {
        return None;
    }
    let idx = url.find("/data/")?;
    let after = &url[idx + "/data/".len()..];
    let id = after.split('/').next()?;
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
    }
}

// --- Pack-managed mutation guard (new policy, not a port) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonMutationKind {
    Install,
    Remove,
    Toggle,
    Update,
}

/// The decided contract's mutation matrix: a pack-managed server refuses
/// every individual add-on mutation, regardless of kind. `health/repair`'s
/// `Update`/`Install` actions (P8.23) route through this exact same gate
/// since they route through the exact same mutation paths P8.17 builds,
/// not a parallel implementation. Contrast: MSC 1 itself never gates any
/// of these calls (`AddonUpdateSheet.swift:272-277`,
/// `DetailsComponentsTabView.swift:1148-1157` only change confirmation
/// copy) -- this is Phase 8's own invented enforcement, not a port.
pub fn pack_mutation_refused(pack_managed: bool, _kind: AddonMutationKind) -> bool {
    pack_managed
}

/// Whole-pack replacement is the one sanctioned escape hatch, but only
/// when EXPLICITLY requested -- an import call that merely happens to
/// target an already-pack-managed server, with no explicit replace
/// intent, is refused the same as any other individual-looking mutation.
/// MSC 1 has no re-import guard of any kind to port from (`importModpack`/
/// CurseForge import both run unconditionally today), so this distinction
/// is invented here, not inherited.
pub fn pack_replace_refused(pack_managed: bool, explicit_replace_intent: bool) -> bool {
    pack_managed && !explicit_replace_intent
}
