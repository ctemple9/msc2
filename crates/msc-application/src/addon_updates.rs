//! Durable add-on inventory and update resolution — the application
//! service behind `resolveAddonUpdates(for:force:)`
//! (`AddonUpdateResolver.swift:20-40` plus the identity/bucket helpers at
//! lines 141-291), wiring `msc_domain::addon_update`'s pure decisions
//! (P8.11) to real file hashing and real Modrinth lookups
//! (`msc_infrastructure::addon_provider`, P8.13) and this crate's own
//! `add_on_inventory` disk scanner (P7.36/P8.16).
//!
//! **Two batched Modrinth requests per resolve pass, never one per file**
//! — [`resolve_addon_updates`] calls `modrinth_versions_from_hashes`
//! (exact identity) and `modrinth_latest_versions_for_hashes` (compatible
//! latest) exactly once each, both already chunked at
//! [`msc_infrastructure::addon_provider::MAX_BATCH_SIZE`] — matching
//! `RESOLVE_PASS_MODRINTH_REQUEST_COUNT`
//! (`fixtures/addon-update-resolution/
//! hash-identify-and-latest-lookup-are-concurrent-not-sequential.json`).
//! The oracle runs both lookups concurrently via `async let`; this port
//! runs them sequentially (this crate's synchronous, blocking-transport
//! convention — every other Phase 7/8 provider call already does the
//! same) but preserves the *request-count* invariant the fixture actually
//! asserts, which is what a folder of any size costs.
//!
//! **`AddonPlanCache` — D-021's "bounded by construction" for a fleet
//! agent.** `should_recompute_addon_plan` (P8.11) characterizes MSC 1's
//! own single in-memory plan slot: one desktop app showing one server's
//! Components tab at a time, so `cached_server_id != current_server_id`
//! (a "you switched servers" event) drops the old plan. A headless agent
//! managing several servers at once has no single "currently displayed"
//! server to key that on — so [`AddonPlanCache`] applies the exact same
//! ported function *per cache slot*, treating "the server id this slot
//! currently holds, if any" as that slot's own `cached_server_id`. For any
//! one server, this reproduces MSC 1's own behavior exactly (recompute iff
//! forced or this server has no cached plan yet); the genuinely new part —
//! not a port, decided for this step per D-021 point 2 ("no unbounded
//! growth is acceptable in a long-lived agent") — is that
//! [`MAX_CACHED_PLANS`] bounds how many *servers'* plans this cache holds
//! at once, evicting the least-recently-resolved one once the bound is
//! hit, the same shape this workspace's other bounded caches
//! (`console_buffer.rs`'s ring buffer, `metrics.rs`'s history) already
//! use.

use std::collections::HashMap;
use std::path::Path;

use msc_domain::addon_provider::ModrinthVersionInfo;
use msc_domain::addon_update::{self, AddonUpdateBucket, PluginTier};
use msc_domain::app_config_schema::{AddonLink, AddonLinkProvenance, PluginSourceConfig};
use msc_domain::identity::{AddOnKind, JavaServerFlavor};

use msc_infrastructure::addon_provider::{self as provider, AddonTransport};
use msc_infrastructure::download_staging::sha512_hex;
use msc_infrastructure::fs::FileSystem;

use crate::add_on_inventory;

/// See this module's own doc on why this bounds *servers*, not add-ons —
/// one server's own plan already has no per-item cap (matching the
/// oracle, which never caps a single server's add-on count either).
pub const MAX_CACHED_PLANS: usize = 16;

/// One resolved add-on's identity, update status, and (for a plugin
/// server) source tier — the `/v1/addons` response's own per-item shape
/// (P8.24's job to wire into the actual DTO; this is the application
/// layer's typed result).
#[derive(Debug, Clone)]
pub struct AddonUpdateItem {
    pub filename: String,
    pub jar_stem: String,
    pub is_enabled: bool,
    pub display_name: String,
    pub project_id: Option<String>,
    pub provenance: Option<AddonLinkProvenance>,
    pub tier: Option<PluginTier>,
    pub bucket: AddonUpdateBucket,
    pub available_version_id: Option<String>,
    pub available_version_label: Option<String>,
    /// P8.17 amendment: the full latest-compatible Modrinth version (files
    /// included) when `bucket == UpdateAvailable` — the exact response
    /// this pass's own `modrinth_latest_versions_for_hashes` call already
    /// fetched. Carried through rather than dropped down to just an id/
    /// label, so `addons::update_one`/`update_all` can install directly
    /// from it without a second, redundant Modrinth request for data this
    /// resolve pass already has in hand.
    pub available_version: Option<ModrinthVersionInfo>,
}

#[derive(Debug, Clone, Default)]
pub struct AddonUpdatePlan {
    pub items: Vec<AddonUpdateItem>,
    /// Self-healing links discovered this pass, keyed by project id —
    /// `mergeDiscoveredLinks`' own input (`msc_domain::addon_update::
    /// merge_discovered_link`, P8.11). The caller (P8.17) merges this into
    /// `ConfigServer.addon_links` and persists it; this function itself
    /// never writes config.
    pub discovered_links: HashMap<String, AddonLink>,
}

struct DiskEntry {
    filename: String,
    jar_stem: String,
    is_enabled: bool,
    display_name: String,
}

fn list_disk_entries(fs: &dyn FileSystem, dir: &Path, kind: AddOnKind) -> Vec<DiskEntry> {
    match kind {
        AddOnKind::Mod => add_on_inventory::scan_mods(fs, dir)
            .into_iter()
            .map(|m| DiskEntry {
                filename: m.filename,
                jar_stem: m.jar_stem,
                is_enabled: m.is_enabled,
                display_name: m.display_name,
            })
            .collect(),
        AddOnKind::Plugin => add_on_inventory::scan_plugins(fs, dir)
            .into_iter()
            .map(|p| {
                let path = dir.join(&p.filename);
                let display_name = add_on_inventory::plugin_yml_name(&path)
                    .filter(|s| !s.is_empty())
                    .unwrap_or(p.display_name);
                DiskEntry {
                    filename: p.filename,
                    jar_stem: p.jar_stem,
                    is_enabled: p.is_enabled,
                    display_name,
                }
            })
            .collect(),
    }
}

/// `resolveAddonUpdates(for:force:)`'s real body once the stale-plan-cache
/// guard has already decided a recompute is needed (that guard is
/// [`AddonPlanCache`]'s job, not this function's — this function always
/// does a full resolve). Vanilla (`flavor.add_on_kind() == None`) returns
/// an empty plan, matching the oracle's own hidden add-on browser for it.
pub fn resolve_addon_updates(
    transport: &dyn AddonTransport,
    fs: &dyn FileSystem,
    add_on_dir: &Path,
    flavor: JavaServerFlavor,
    minecraft_version: Option<&str>,
    persisted_links: &HashMap<String, AddonLink>,
    plugin_sources: &HashMap<String, PluginSourceConfig>,
) -> AddonUpdatePlan {
    let Some(add_on_kind) = flavor.add_on_kind() else {
        return AddonUpdatePlan::default();
    };
    let entries = list_disk_entries(fs, add_on_dir, add_on_kind);

    let mut hashes: HashMap<String, String> = HashMap::new();
    let mut sha512s: Vec<String> = Vec::new();
    for entry in &entries {
        if addon_update::should_exclude_from_hash_resolution(add_on_kind, &entry.jar_stem) {
            continue;
        }
        let path = add_on_dir.join(&entry.filename);
        if let Ok(bytes) = fs.read(&path) {
            let hash = sha512_hex(&bytes);
            sha512s.push(hash.clone());
            hashes.insert(entry.filename.clone(), hash);
        }
    }

    let loaders: Vec<String> = flavor
        .modrinth_loader_facets()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let game_versions: Vec<String> = minecraft_version
        .map(|v| vec![v.to_string()])
        .unwrap_or_default();

    let identify = provider::modrinth_versions_from_hashes(transport, &sha512s).unwrap_or_default();
    let latest = provider::modrinth_latest_versions_for_hashes(
        transport,
        &sha512s,
        &loaders,
        &game_versions,
    )
    .unwrap_or_default();

    let mut by_installed_hash: HashMap<&str, &AddonLink> = HashMap::new();
    let mut by_installed_filename: HashMap<&str, &AddonLink> = HashMap::new();
    for link in persisted_links.values() {
        if let Some(h) = &link.installed_hash {
            by_installed_hash.insert(h.as_str(), link);
        }
        if let Some(f) = &link.installed_file_name {
            by_installed_filename.insert(f.as_str(), link);
        }
    }

    let mc_version_configured = minecraft_version.is_some();
    let mut items = Vec::with_capacity(entries.len());
    let mut discovered_links = HashMap::new();

    for entry in &entries {
        let hash = hashes.get(&entry.filename);
        let fresh: Option<&ModrinthVersionInfo> = hash.and_then(|h| identify.get(h));
        let latest_v: Option<&ModrinthVersionInfo> = hash.and_then(|h| latest.get(h));

        let persisted_by_hash = hash
            .and_then(|h| by_installed_hash.get(h.as_str()))
            .copied();
        let persisted_by_filename = by_installed_filename.get(entry.filename.as_str()).copied();

        let tier = (add_on_kind == AddOnKind::Plugin)
            .then(|| addon_update::derive_plugin_tier(&entry.jar_stem, plugin_sources));

        let project_id = addon_update::resolve_project_id(
            fresh.map(|v| v.project_id.as_str()),
            persisted_by_hash.map(|l| l.project_id.as_str()),
            persisted_by_filename.map(|l| l.project_id.as_str()),
        );

        let Some(project_id) = project_id else {
            items.push(AddonUpdateItem {
                filename: entry.filename.clone(),
                jar_stem: entry.jar_stem.clone(),
                is_enabled: entry.is_enabled,
                display_name: entry.display_name.clone(),
                project_id: None,
                provenance: None,
                tier,
                bucket: AddonUpdateBucket::Unlinked,
                available_version_id: None,
                available_version_label: None,
                available_version: None,
            });
            continue;
        };
        let project_id = project_id.to_string();

        let persisted_provenance = persisted_by_hash
            .or(persisted_by_filename)
            .map(|l| l.provenance);
        let provenance = addon_update::resolve_provenance(fresh.is_some(), persisted_provenance);

        if addon_update::should_record_self_healing_link(fresh.is_some())
            && let Some(v) = fresh
        {
            discovered_links.insert(
                project_id.clone(),
                AddonLink {
                    project_id: project_id.clone(),
                    title: None,
                    slug: None,
                    icon_url: None,
                    provenance: AddonLinkProvenance::HashDetected,
                    installed_version_id: Some(v.id.clone()),
                    installed_file_name: Some(entry.filename.clone()),
                    installed_hash: hash.cloned(),
                    client_side: None,
                    server_side: None,
                    extra: Default::default(),
                },
            );
        }

        let current_version_id = addon_update::resolve_current_version_id(
            fresh.map(|v| v.id.as_str()),
            persisted_by_hash
                .or(persisted_by_filename)
                .and_then(|l| l.installed_version_id.as_deref()),
        );
        let latest_version_id = latest_v.map(|v| v.id.as_str());
        let (bucket, available_version_id) = addon_update::resolve_bucket(
            latest_version_id,
            current_version_id,
            mc_version_configured,
        );
        let (available_version_label, available_version) =
            if bucket == AddonUpdateBucket::UpdateAvailable {
                (
                    latest_v.map(|v| addon_update::clean_version_label(&v.version_number)),
                    latest_v.cloned(),
                )
            } else {
                (None, None)
            };

        items.push(AddonUpdateItem {
            filename: entry.filename.clone(),
            jar_stem: entry.jar_stem.clone(),
            is_enabled: entry.is_enabled,
            display_name: entry.display_name.clone(),
            project_id: Some(project_id),
            provenance: Some(provenance),
            tier,
            bucket,
            available_version_id,
            available_version_label,
            available_version,
        });
    }

    items.sort_by(|a, b| {
        addon_update::addon_update_sort_key(&a.display_name, a.bucket).cmp(
            &addon_update::addon_update_sort_key(&b.display_name, b.bucket),
        )
    });

    AddonUpdatePlan {
        items,
        discovered_links,
    }
}

/// A bounded, per-server cache of the last-resolved [`AddonUpdatePlan`] —
/// see this module's own doc for how it generalizes
/// `should_recompute_addon_plan` to a fleet agent.
#[derive(Default)]
pub struct AddonPlanCache {
    entries: HashMap<String, AddonUpdatePlan>,
    /// Least-recently-touched first, most-recently-touched last.
    order: Vec<String>,
}

impl AddonPlanCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the cached plan for `server_id`, recomputing via `resolve`
    /// exactly when `should_recompute_addon_plan` says to for this slot
    /// (forced, or nothing cached for this server yet) — `resolve` is
    /// never called on a genuine cache hit.
    pub fn get_or_resolve(
        &mut self,
        server_id: &str,
        force: bool,
        resolve: impl FnOnce() -> AddonUpdatePlan,
    ) -> &AddonUpdatePlan {
        let cached_marker = self
            .entries
            .contains_key(server_id)
            .then(|| server_id.to_string());
        if addon_update::should_recompute_addon_plan(cached_marker.as_deref(), server_id, force) {
            let plan = resolve();
            self.insert(server_id, plan);
        } else {
            self.touch(server_id);
        }
        self.entries
            .get(server_id)
            .expect("inserted above, or already present per should_recompute_addon_plan")
    }

    /// Drops `server_id`'s cached plan outright — P8.17 calls this after
    /// any mutation that changes what's on disk (install/update/toggle/
    /// remove), so the next read is never served a plan describing a
    /// folder state that no longer exists. `should_recompute_addon_plan`
    /// alone can't express this: from its point of view an invalidated-but-
    /// not-yet-requeried server still "has a cached plan" (this slot's
    /// `cached_server_id` would still equal `server_id`), so eviction has
    /// to be an explicit, separate operation, not something a smarter
    /// recompute predicate could subsume.
    pub fn invalidate(&mut self, server_id: &str) {
        self.entries.remove(server_id);
        self.order.retain(|id| id != server_id);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn insert(&mut self, server_id: &str, plan: AddonUpdatePlan) {
        self.entries.insert(server_id.to_string(), plan);
        self.touch(server_id);
        while self.order.len() > MAX_CACHED_PLANS {
            let evicted = self.order.remove(0);
            self.entries.remove(&evicted);
        }
    }

    fn touch(&mut self, server_id: &str) {
        self.order.retain(|id| id != server_id);
        self.order.push(server_id.to_string());
    }
}
