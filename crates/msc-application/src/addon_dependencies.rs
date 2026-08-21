//! The bounded dependency installer: resolves and installs a Modrinth
//! version's required dependencies, transitively, through the provider
//! (P8.13/P8.15's own amendment) and store (P8.14) boundaries.
//!
//! Ported from `installRequiredDependencies(of:into:depth:)`
//! (`AppViewModel+ModManagement.swift:271-328`), per
//! `docs/msc2/addons/phase8-scope.md`, `fixtures/modrinth-dependencies/`
//! (P8.6), and `msc_domain::addon_dependency`'s pure decisions (P8.12).
//! Every per-dependency failure is non-fatal to the batch (the loop always
//! continues to the next sibling), matching the oracle's own `do`/`catch`
//! inside the `for dep in required` loop -- this port's ONE new failure
//! mode the oracle doesn't have is cooperative cancellation (`should_cancel`),
//! which stops the whole resolution and rolls back every file this
//! operation itself installed, since MSC 1 has no cancellation concept to
//! preserve fidelity to.
//!
//! **Cycle detection, not the oracle's depth cap:** `msc_domain::
//! addon_dependency::cycle_detected` tracks project ids currently on the
//! recursion's own ancestor stack (pushed before a project_id's own work
//! starts, popped once its whole subtree -- including recursive children
//! -- has been processed), catching a genuine A->B->A cycle of any length.
//! It does NOT prevent a legitimate diamond shape (B and C both requiring
//! D) from resolving D twice on its own -- `dependency_already_present`'s
//! on-disk/mod-id scan is what catches that, run before every fetch, the
//! same as the oracle (`phase8-scope.md`'s own diamond-dependency finding,
//! `fixtures/modrinth-dependencies/diamond-dependency-...json`).
//!
//! **Deterministic parent-before-child ordering:** each dependency's own
//! outcome is recorded in `DependencyInstallReport::results` as soon as it
//! resolves (install succeeded/failed/skipped/etc.), *before* recursing
//! into that dependency's own transitive dependencies -- a pre-order
//! walk, matching the oracle's own `logAppMessage(...)` (line 312) running
//! before its recursive call (line 313).

use std::path::{Path, PathBuf};

use msc_domain::addon_dependency::{self, ModrinthDependency};
use msc_domain::addon_provider::{self as domain, ModrinthVersionInfo};
use msc_domain::identity::JavaServerFlavor;

use msc_infrastructure::addon_provider::AddonTransport;
use msc_infrastructure::addon_store::{self, AddonStoreError};
use msc_infrastructure::download_staging::ExpectedChecksum;
use msc_infrastructure::fs::FileSystem;

/// What happened to one required dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyInstallOutcome {
    /// Already installed (mod-ID match or a filename slug scan).
    Skipped,
    /// This project id is already an ancestor of itself in the current
    /// recursion -- a genuine cycle, not a mere depth coincidence.
    CycleDetected,
    /// No version of this dependency matched the server's loader/MC
    /// version, or the best match had no primary file.
    NoCompatibleVersion,
    /// The project fetch, version-list fetch, directory creation, or
    /// download/verify step failed. Carries a human-readable reason;
    /// never fatal to sibling dependencies.
    Failed(String),
    /// Installed at `path`.
    Installed { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyInstallResult {
    pub project_id: String,
    pub outcome: DependencyInstallOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DependencyInstallReport {
    /// Pre-order (parent-before-child): each dependency's own result is
    /// appended as soon as it resolves, before recursing into its
    /// transitive dependencies.
    pub results: Vec<DependencyInstallResult>,
    /// True if `should_cancel` fired mid-resolution. When true, every
    /// file this operation installed has already been removed --
    /// `results` still reflects what was attempted, for diagnostics, but
    /// none of it is left on disk.
    pub cancelled: bool,
}

struct ResolveCtx<'a> {
    transport: &'a dyn AddonTransport,
    fs: &'a dyn FileSystem,
    flavor: JavaServerFlavor,
    minecraft_version: Option<&'a str>,
    server_dir: &'a Path,
    installed_mod_ids: &'a [String],
    should_cancel: &'a dyn Fn() -> bool,
}

/// Installs `version`'s required dependencies, transitively. Returns a
/// report of every dependency this call (and its recursive children)
/// touched; never returns an `Err` -- per-dependency failure is recorded
/// in the report, not propagated, matching the oracle's own
/// log-and-continue shape. `should_cancel` is checked before starting
/// each sibling dependency; if it fires, resolution stops immediately and
/// every file this call installed (at any recursion depth) is removed
/// before returning.
#[allow(clippy::too_many_arguments)]
pub fn install_required_dependencies(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    version: &ModrinthVersionInfo,
    flavor: JavaServerFlavor,
    minecraft_version: Option<&str>,
    server_dir: &Path,
    installed_mod_ids: &[String],
    should_cancel: &dyn Fn() -> bool,
) -> DependencyInstallReport {
    let ctx = ResolveCtx {
        transport,
        fs,
        flavor,
        minecraft_version,
        server_dir,
        installed_mod_ids,
        should_cancel,
    };
    let mut report = DependencyInstallReport::default();
    let mut visited: Vec<String> = Vec::new();
    let mut installed_paths: Vec<PathBuf> = Vec::new();

    resolve_required(
        &ctx,
        &version.dependencies,
        &mut visited,
        &mut installed_paths,
        &mut report,
    );

    if report.cancelled {
        for path in installed_paths.iter().rev() {
            let _ = fs.remove(path);
        }
    }

    report
}

fn resolve_required(
    ctx: &ResolveCtx,
    deps: &[ModrinthDependency],
    visited: &mut Vec<String>,
    installed_paths: &mut Vec<PathBuf>,
    report: &mut DependencyInstallReport,
) {
    if report.cancelled {
        return;
    }
    let required = addon_dependency::required_dependencies_with_project_id(deps);
    if required.is_empty() {
        return;
    }
    let Some(add_on_kind) = ctx.flavor.add_on_kind() else {
        return;
    };
    let folder = ctx.server_dir.join(add_on_kind.folder_name());

    for project_id in required {
        if (ctx.should_cancel)() {
            report.cancelled = true;
            return;
        }
        if addon_dependency::cycle_detected(project_id, visited) {
            report.results.push(DependencyInstallResult {
                project_id: project_id.to_string(),
                outcome: DependencyInstallOutcome::CycleDetected,
            });
            continue;
        }
        visited.push(project_id.to_string());
        resolve_one(ctx, project_id, &folder, visited, installed_paths, report);
        visited.pop();
        if report.cancelled {
            return;
        }
    }
}

fn resolve_one(
    ctx: &ResolveCtx,
    project_id: &str,
    folder: &Path,
    visited: &mut Vec<String>,
    installed_paths: &mut Vec<PathBuf>,
    report: &mut DependencyInstallReport,
) {
    let record = |report: &mut DependencyInstallReport, outcome: DependencyInstallOutcome| {
        report.results.push(DependencyInstallResult {
            project_id: project_id.to_string(),
            outcome,
        });
    };

    let project =
        match msc_infrastructure::addon_provider::modrinth_project(ctx.transport, project_id) {
            Ok(p) => p,
            Err(e) => {
                record(report, DependencyInstallOutcome::Failed(e.to_string()));
                return;
            }
        };

    let files_on_disk = list_filenames(ctx.fs, folder);
    if addon_dependency::dependency_already_present(
        &project.slug,
        ctx.installed_mod_ids,
        &files_on_disk,
    ) {
        record(report, DependencyInstallOutcome::Skipped);
        return;
    }

    let loaders: Vec<String> = ctx
        .flavor
        .modrinth_loader_facets()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let versions = match msc_infrastructure::addon_provider::modrinth_project_versions(
        ctx.transport,
        &project.slug,
        &loaders,
        ctx.minecraft_version,
    ) {
        Ok(v) => v,
        Err(e) => {
            record(report, DependencyInstallOutcome::Failed(e.to_string()));
            return;
        }
    };
    let Some(best) = versions.first() else {
        record(report, DependencyInstallOutcome::NoCompatibleVersion);
        return;
    };
    let Some(primary) = domain::modrinth_primary_file(&best.files) else {
        record(report, DependencyInstallOutcome::NoCompatibleVersion);
        return;
    };

    if let Err(e) = ctx.fs.create_dir_all(folder) {
        record(report, DependencyInstallOutcome::Failed(e.to_string()));
        return;
    }

    let dest = folder.join(&primary.filename);
    let expected_checksum = primary
        .hashes
        .get("sha1")
        .map(|hex| ExpectedChecksum::sha1(hex.clone()));

    if let Err(e) = addon_store::install_verified_file(
        ctx.transport,
        ctx.fs,
        &primary.url,
        &best.version_number,
        expected_checksum.as_ref(),
        &dest,
    ) {
        record(report, addon_store_failure(e));
        return;
    }

    installed_paths.push(dest.clone());
    record(report, DependencyInstallOutcome::Installed { path: dest });

    resolve_required(ctx, &best.dependencies, visited, installed_paths, report);
}

fn addon_store_failure(e: AddonStoreError) -> DependencyInstallOutcome {
    DependencyInstallOutcome::Failed(e.to_string())
}

fn list_filenames(fs: &dyn FileSystem, folder: &Path) -> Vec<String> {
    fs.list(folder)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string))
        .collect()
}
