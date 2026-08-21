//! Modrinth required-dependency resolution decisions.
//!
//! Ported from `installRequiredDependencies(of:into:depth:)`
//! (`AppViewModel+ModManagement.swift:271-328`), per
//! `docs/msc2/addons/phase8-scope.md` and `fixtures/modrinth-dependencies/`
//! (P8.6). Every function here is a pure decision over already-known
//! inputs (a parsed dependency list, an already-installed-mod-id snapshot,
//! a folder listing) -- the real network fetch, download, and recursive
//! orchestration is `msc-application`'s job (P8.15). Optional dependencies
//! are never resolved here at all: only `dependencyType == "required"`
//! survives [`required_dependencies_with_project_id`], matching
//! `rolling-plan.md`'s own "Optional dependencies remain explanatory, not
//! silently installed."
//!
//! **P8.15 amendment:** [`ModrinthDependency`] now derives [`Deserialize`]
//! so it can decode directly out of a version response's own embedded
//! `dependencies` array (`ModrinthVersionInfo.dependencies`,
//! `addon_provider.rs`'s own P8.15 amendment) -- Modrinth's wire shape is
//! already snake_case (`project_id`/`dependency_type`), matching this
//! struct's field names with no rename needed, the same as
//! `ModrinthSearchHit`'s own undecorated derive.

use crate::identity::AddOnKind;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ModrinthDependency {
    #[serde(default)]
    pub project_id: Option<String>,
    pub dependency_type: String,
}

/// Line 276 (type filter) + line 290 (projectId guard), combined: only
/// `"required"` dependencies survive, and among those, only ones that
/// carry a `projectId` at all -- a required dependency expressed solely as
/// a `versionId` is silently skipped, not resolved through an alternate
/// lookup path.
pub fn required_dependencies_with_project_id(deps: &[ModrinthDependency]) -> Vec<&str> {
    deps.iter()
        .filter(|d| d.dependency_type == "required")
        .filter_map(|d| d.project_id.as_deref())
        .collect()
}

/// Lines 293-299's two independent already-present checks, combined: a
/// mod-ID match against the pre-loop `discoveredMods` snapshot, then (if
/// that misses) a lowercase-contains scan of the raw folder listing for
/// the dependency's own slug -- the second check catches a mod installed
/// through a path that never registered a modId match.
pub fn dependency_already_present(
    dependency_slug: &str,
    installed_mod_ids: &[String],
    files_on_disk: &[String],
) -> bool {
    if installed_mod_ids.iter().any(|id| id == dependency_slug) {
        return true;
    }
    let slug_lower = dependency_slug.to_lowercase();
    files_on_disk
        .iter()
        .any(|f| f.to_lowercase().contains(&slug_lower))
}

/// MSC 1's own guard (line 275): a flat recursion-depth cap of 3, not
/// cycle detection. Characterized here for oracle-fidelity only -- the
/// real port ([`cycle_detected`]) uses a visited-project-id set instead,
/// per `phase8-scope.md`'s own explicit finding that a depth cap and a
/// real cycle detector are not behaviorally identical for a diamond-shaped
/// (non-cyclic) dependency graph, which the depth cap alone does not
/// protect from a duplicate download (see [`dependency_already_present`]
/// for what actually does).
pub fn msc1_depth_cap_exceeded(depth: u32) -> bool {
    depth >= 3
}

/// New, decided policy (not a port): P8.15's dependency installer tracks
/// which project ids are already being resolved in the current call stack
/// and refuses to recurse into one already present -- a genuine cycle of
/// any length is caught this way, rather than merely a depth-3
/// coincidence the way [`msc1_depth_cap_exceeded`] happens to stop one.
pub fn cycle_detected(project_id: &str, visited: &[String]) -> bool {
    visited.iter().any(|v| v == project_id)
}

/// The taxonomy of what happens to ONE dependency inside the `for`/
/// `do`-`catch` loop (lines 290-316). Every variant is non-fatal to the
/// batch: whether a dependency is skipped as already-present, has no
/// compatible build, or fails outright (network/decode/download error),
/// the loop always continues to the next sibling dependency rather than
/// aborting `installRequiredDependencies` for the ones still to come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyOutcome {
    Skipped,
    NoCompatibleVersion,
    Failed,
    Installed,
}

impl DependencyOutcome {
    pub fn is_fatal_to_batch(self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyRefreshTarget {
    Mods,
    Plugins,
}

/// Lines 319-323: dispatched once after the ENTIRE required-dependency
/// loop at this recursion level completes, not once per downloaded
/// dependency.
pub fn dependency_refresh_target(add_on_kind: AddOnKind) -> DependencyRefreshTarget {
    match add_on_kind {
        AddOnKind::Mod => DependencyRefreshTarget::Mods,
        AddOnKind::Plugin => DependencyRefreshTarget::Plugins,
    }
}
