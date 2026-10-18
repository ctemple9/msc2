//! Owned storage discovery and bounded background scans. No cleanup authority.
use msc_domain::{app_config_schema::AppConfig, world};
use msc_infrastructure::{
    storage::{self, Category, Group, Limits, Manifest, Progress, Root, Rule, Snapshot},
    uninstall::{EntryKind, EntryState, Inventory},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

/// Supply the actual decoded configuration and locally inspected ownership.
/// Native platform discovery owns package/service inspection; this service
/// never runs package commands or searches other homes during a file walk.
pub struct Discovery<'a> {
    pub host_id: &'a str,
    pub config: &'a AppConfig,
    /// Effective runtime root, including MSC2_AGENT_SERVERS_ROOT/service overrides.
    pub servers_root: &'a Path,
    pub ownership: &'a Inventory,
    /// Verified bundle/package paths from native discovery, not name guesses.
    pub desktop_installations: &'a [PathBuf],
}

pub fn discover(input: &Discovery<'_>) -> Manifest {
    if input.ownership.entries.len() > 2048 || input.config.servers.len() > 512 {
        return Manifest {
            host_id: input.host_id.into(),
            generation: input.ownership.fingerprint.clone(),
            roots: vec![],
            excluded_roots: vec![],
            issues: vec!["Ownership discovery exceeds the source budget".into()],
        };
    }
    let mut roots = BTreeMap::<PathBuf, Root>::new();
    let mut issues = vec![];
    let exclusions: Vec<_> = input
        .ownership
        .entries
        .iter()
        .filter(|entry| entry.state == EntryState::Blocked)
        .filter_map(|entry| entry.path.clone())
        .collect();
    for entry in &input.ownership.entries {
        if entry.state == EntryState::Blocked {
            issues.push(
                entry
                    .problem
                    .clone()
                    .unwrap_or_else(|| "Ownership could not be established".into()),
            );
            continue;
        }
        // Missing known default locations need no traversal. Missing configured
        // server/data roots still become explicit partial coverage below.
        if entry.state == EntryState::Missing {
            continue;
        }
        let Some(path) = &entry.path else {
            if entry.kind == EntryKind::Installation && entry.state == EntryState::Present {
                issues.push(
                    "Installed package file coverage is unavailable from native discovery".into(),
                );
            }
            continue;
        };
        if exclusions.iter().any(|p| path.starts_with(p)) {
            continue;
        }
        let category = match entry.kind {
            EntryKind::DataTree | EntryKind::Configuration => Category::Agent,
            EntryKind::ServerTree => Category::Servers,
            EntryKind::ClientState => {
                if path.components().any(|part| {
                    part.as_os_str()
                        .to_string_lossy()
                        .starts_with("com.ctemple.msc2")
                }) {
                    Category::DesktopApp
                } else {
                    Category::Agent
                }
            }
            EntryKind::Installer => Category::Cache,
            EntryKind::Installation => {
                if input.desktop_installations.contains(path) {
                    Category::DesktopApp
                } else {
                    Category::Agent
                }
            }
            _ => continue,
        };
        let rules = if entry.kind == EntryKind::DataTree {
            data_rules()
        } else {
            vec![]
        };
        let root = Root {
            path: path.clone(),
            category,
            label: match category {
                Category::Agent => "Agent and protected data",
                Category::DesktopApp => "Desktop app",
                Category::Servers => "Unregistered server files (protected)",
                Category::Cache => "Verified installer download",
                Category::SharedTools => "Shared tools",
            }
            .into(),
            server_id: None,
            evidence: entry.evidence.clone(),
            rules,
        };
        // The uninstall inventory deliberately includes overlapping parent paths.
        // Exact duplicate evidence merges here; child boundaries split at scan time.
        roots.entry(path.clone()).or_insert(root);
    }
    for server in &input.config.servers {
        let path = PathBuf::from(&server.server_dir);
        let Some(root) = roots.get_mut(&path) else {
            issues.push(format!(
                "Registered server {} has no readable owned root",
                server.id
            ));
            continue;
        };
        if root.server_id.is_some() || root.label == "Conflicting server ownership (protected)" {
            issues.push(
                "Multiple registered servers claim one root; data counted once and protected"
                    .into(),
            );
            root.server_id = None;
            root.rules.clear();
            root.label = "Conflicting server ownership (protected)".into();
            continue;
        }
        if root.category != Category::Servers {
            issues.push(
                "Registered server overlaps another installation/data root; ownership is ambiguous"
                    .into(),
            );
            continue;
        }
        root.category = Category::Servers;
        root.server_id = Some(server.id.clone());
        root.label = format!("{} / Logs & other files (protected)", server.display_name);
        root.rules = vec![
            rule(
                "backups",
                Category::Servers,
                if server.server_type == msc_domain::identity::ServerType::Bedrock {
                    "Bedrock collection backups"
                } else {
                    "Backups"
                },
                Some(&server.id),
                true,
            ),
            rule(
                "world_slots",
                Category::Servers,
                if server.server_type == msc_domain::identity::ServerType::Bedrock {
                    "Saved world collection"
                } else {
                    "Saved world"
                },
                Some(&server.id),
                true,
            ),
            rule(
                "_addon_cache",
                Category::Cache,
                "Add-on cache",
                Some(&server.id),
                true,
            ),
            rule(
                ".msc2-staging",
                Category::Cache,
                "Import/export staging (protected)",
                Some(&server.id),
                true,
            ),
        ];
        for component in [
            "mods",
            "plugins",
            "libraries",
            "config",
            "resourcepacks",
            "behavior_packs",
            "resource_packs",
            "server.properties",
            "eula.txt",
            "permissions.json",
            "allowlist.json",
            "whitelist.json",
            "ops.json",
            "banned-ips.json",
            "banned-players.json",
            "bedrock_server",
            "bedrock_server.exe",
        ] {
            root.rules.push(rule(
                component,
                Category::Servers,
                "Mods & server files",
                Some(&server.id),
                false,
            ));
        }
        let jar_path = Path::new(&server.paper_jar_path);
        let jar = if jar_path.is_absolute() {
            jar_path.strip_prefix(&path).ok()
        } else {
            Some(jar_path)
        };
        if let Some(jar) = jar.filter(|jar| safe_relative(jar)) {
            root.rules.push(rule(
                jar,
                Category::Servers,
                "Mods & server files",
                Some(&server.id),
                false,
            ));
        }
        let properties = storage::read_small(&path.join("server.properties"), 256 * 1024);
        let raw_level = match properties {
            Ok(bytes) => String::from_utf8_lossy(&bytes).lines().find_map(|line| {
                let (key, value) = line.trim().split_once('=')?;
                (key.trim() == "level-name").then(|| value.trim().to_string())
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                issues.push(format!(
                    "World attribution unavailable for {}: {error}",
                    server.id
                ));
                continue;
            }
        };
        let name = world::current_level_name(server.server_type, raw_level.as_deref());
        for folder in world::backup_root_folder_candidates(server.server_type, &name) {
            let collides = root
                .rules
                .iter()
                .any(|rule| Path::new(&folder).starts_with(&rule.relative));
            if collides {
                issues.push(format!(
                    "World location overlaps protected component/backup data for {}",
                    server.id
                ));
            } else if safe_relative(Path::new(&folder)) {
                let mut world_rule = rule(
                    folder,
                    Category::Servers,
                    if server.server_type == msc_domain::identity::ServerType::Bedrock {
                        "Active Bedrock world"
                    } else {
                        "Active world"
                    },
                    Some(&server.id),
                    false,
                );
                world_rule.group = Group::Worlds;
                root.rules.push(world_rule);
            } else {
                issues.push(format!(
                    "External/unsafe world path excluded for {}",
                    server.id
                ));
            }
        }
    }
    let server_root = input.servers_root.to_path_buf();
    if !roots.contains_key(&server_root) {
        issues.push("Configured servers root unavailable; coverage is incomplete".into());
    }
    if let Some(root) = roots.get_mut(&server_root) {
        root.rules.push(rule(
            ".msc2-staging",
            Category::Cache,
            "Import/export staging (protected)",
            None,
            true,
        ));
    }
    roots.retain(|path, _| !exclusions.iter().any(|excluded| path.starts_with(excluded)));
    // Generation contains path configuration and ownership evidence, not secrets
    // from AppConfig. Live object identities are added by the scanner separately.
    let mut manifest = Manifest {
        host_id: input.host_id.into(),
        generation: String::new(),
        roots: roots.into_values().collect(),
        excluded_roots: exclusions,
        issues,
    };
    manifest.generation = storage::identity(&[
        input.host_id,
        &input.ownership.fingerprint,
        &format!("{:?}", manifest.roots),
        &format!("{:?}", manifest.excluded_roots),
    ]);
    manifest
}

fn safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}
fn rule(
    path: impl Into<PathBuf>,
    category: Category,
    label: &str,
    server: Option<&str>,
    children: bool,
) -> Rule {
    let relative = path.into();
    let group = if relative == Path::new("backups") {
        Group::Backups
    } else if relative == Path::new("world_slots") {
        Group::Worlds
    } else {
        match category {
            Category::Servers => Group::Components,
            Category::Cache => Group::Cache,
            Category::SharedTools => Group::Tools,
            Category::DesktopApp => Group::Installation,
            Category::Agent => Group::Other,
        }
    };
    Rule {
        relative,
        group,
        category,
        label: label.into(),
        server_id: server.map(str::to_owned),
        children,
    }
}
fn data_rules() -> Vec<Rule> {
    vec![
        rule(
            "runtimes",
            Category::SharedTools,
            "Managed Java runtimes",
            None,
            true,
        ),
        rule(
            "helpers",
            Category::SharedTools,
            "Shared helpers",
            None,
            true,
        ),
        rule(
            "map-dependencies",
            Category::SharedTools,
            "Map tools",
            None,
            true,
        ),
        rule(
            "map-dependencies/java-assets",
            Category::Cache,
            "Map textures",
            None,
            true,
        ),
        rule(
            "map-staging",
            Category::Cache,
            "Map snapshots and output (protected)",
            None,
            true,
        ),
        rule(
            "agent/builds",
            Category::Agent,
            "Staged agent builds (protected)",
            None,
            true,
        ),
    ]
}

static ACTIVE_HOSTS: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());
struct Admission {
    host_id: String,
}
impl Drop for Admission {
    fn drop(&mut self) {
        ACTIVE_HOSTS.lock().unwrap().remove(&self.host_id);
    }
}

#[derive(Clone)]
pub struct ScanHandle {
    pub id: String,
    cancelled: Arc<AtomicBool>,
    progress: Arc<Mutex<Progress>>,
    result: Arc<Mutex<Option<Arc<Snapshot>>>>,
    done: Arc<AtomicBool>,
    failure: Arc<Mutex<Option<String>>>,
}
impl ScanHandle {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    pub fn progress(&self) -> Progress {
        self.progress.lock().unwrap().clone()
    }
    pub fn result(&self) -> Option<Arc<Snapshot>> {
        self.result.lock().unwrap().clone()
    }
    pub fn failure(&self) -> Option<String> {
        self.failure.lock().unwrap().clone()
    }
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }
}

struct State {
    generation: String,
    epoch: u64,
    active: Option<ScanHandle>,
    latest: Option<Arc<Snapshot>>,
    complete: Option<Arc<Snapshot>>,
}

/// One instance per authenticated host identity. API/native adapters retain it;
/// renders/polls read cached metadata and never initiate a traversal.
pub struct InventoryService {
    host_id: String,
    state: Arc<Mutex<State>>,
}
impl InventoryService {
    pub fn new(host_id: impl Into<String>) -> Self {
        Self {
            host_id: host_id.into(),
            state: Arc::new(Mutex::new(State {
                generation: String::new(),
                epoch: 0,
                active: None,
                latest: None,
                complete: None,
            })),
        }
    }
    /// Invalidation cancels old work; its late results cannot enter this cache.
    pub fn invalidate(&self, generation: &str) {
        let mut state = self.state.lock().unwrap();
        if state.generation != generation {
            if let Some(active) = &state.active {
                active.cancel();
            }
            state.epoch = state.epoch.wrapping_add(1);
            state.generation = generation.into();
            state.latest = None;
            state.complete = None;
        }
    }
    /// Host removal must invalidate even if its last configuration did not change.
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        if let Some(active) = &state.active {
            active.cancel();
        }
        state.epoch = state.epoch.wrapping_add(1);
        state.generation.clear();
        state.latest = None;
        state.complete = None;
    }
    pub fn cached(&self, generation: &str) -> Option<Arc<Snapshot>> {
        let state = self.state.lock().unwrap();
        (state.generation == generation)
            .then(|| state.latest.clone())
            .flatten()
    }
    pub fn last_complete(&self, generation: &str) -> Option<Arc<Snapshot>> {
        let state = self.state.lock().unwrap();
        (state.generation == generation)
            .then(|| state.complete.clone())
            .flatten()
    }
    pub fn is_stale(snapshot: &Snapshot) -> bool {
        storage::now().saturating_sub(snapshot.completed_at) > STALE_AFTER.as_secs()
    }

    pub fn start(&self, manifest: Manifest) -> Result<ScanHandle, String> {
        if manifest.host_id != self.host_id {
            return Err("Inventory belongs to another computer".into());
        }
        self.invalidate(&manifest.generation);
        let mut state = self.state.lock().unwrap();
        if state.generation != manifest.generation {
            return Err("Storage context changed before scan admission".into());
        }
        if state.active.as_ref().is_some_and(|scan| !scan.is_done()) {
            return Err("A scan is already running on this computer".into());
        }
        {
            let mut hosts = ACTIVE_HOSTS.lock().unwrap();
            if hosts.contains(&self.host_id) {
                return Err("A scan is already running on this computer".into());
            }
            if hosts.len() >= 2 {
                return Err("Two host scans are already running".into());
            }
            hosts.insert(self.host_id.clone());
        }
        let admission = Admission {
            host_id: self.host_id.clone(),
        };
        let handle = ScanHandle {
            id: uuid::Uuid::new_v4().to_string(),
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(Mutex::new(Progress::default())),
            result: Arc::new(Mutex::new(None)),
            done: Arc::new(AtomicBool::new(false)),
            failure: Arc::new(Mutex::new(None)),
        };
        state.active = Some(handle.clone());
        let worker = handle.clone();
        let shared = self.state.clone();
        let epoch = state.epoch;
        // Holding state through spawn prevents invalidation from racing admission.
        std::thread::Builder::new()
            .name("storage-inventory".into())
            .spawn(move || {
                let _admission = admission;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    storage::scan(
                        &manifest,
                        Limits::default(),
                        &|| worker.cancelled.load(Ordering::Acquire),
                        &mut |p| *worker.progress.lock().unwrap() = p.clone(),
                    )
                }));
                if let Ok(mut snapshot) = result {
                    snapshot.id = worker.id.clone();
                    let snapshot = Arc::new(snapshot);
                    *worker.result.lock().unwrap() = Some(snapshot.clone());
                    let mut state = shared.lock().unwrap();
                    if state.epoch == epoch && state.generation == manifest.generation {
                        if state.complete.as_ref().is_some_and(|old| {
                            snapshot.progress.roots_completed == manifest.roots.len()
                                && old.root_identity != snapshot.root_identity
                        }) {
                            state.complete = None;
                        }
                        if !snapshot.partial {
                            state.complete = Some(snapshot.clone());
                        }
                        state.latest = Some(snapshot);
                    }
                }
                if worker.result.lock().unwrap().is_none() {
                    *worker.failure.lock().unwrap() = Some(
                        "Storage worker stopped unexpectedly; previous results remain dated".into(),
                    );
                }
                worker.done.store(true, Ordering::Release);
            })
            .map_err(|error| {
                state.active = None;
                error.to_string()
            })?;
        Ok(handle)
    }
}

/// Blocking callers can poll metadata themselves; no timer/deep-scan loop is
/// installed by this service. Persistence/operation journaling belongs to P21.3.
pub const STALE_AFTER: Duration = Duration::from_secs(15 * 60);

#[cfg(test)]
mod tests {
    use super::*;
    use msc_domain::app_config_schema::ConfigServer;
    use msc_infrastructure::uninstall::{Entry, Platform};

    // The scanner's boundary tests cannot catch lost custom-server roots or
    // counting external Java as MSC-owned. This tiny end-to-end discovery case
    // also checks custom world names and backup companion accounting. <1 s,
    // no running service, network, sleeps or permission/environment assumptions.
    #[test]
    fn custom_server_worlds_and_backups_are_partitioned_without_external_java() {
        struct Fixture(PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let path =
            std::env::temp_dir().join(format!("msc-storage-discovery-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        let fixture = Fixture(std::fs::canonicalize(path).unwrap());
        let data = fixture.0.join("data");
        let managed = data.join("servers");
        let custom = fixture.0.join("custom");
        for path in [
            &managed,
            &custom.join("oak"),
            &custom.join("backups"),
            &data.join("runtimes"),
        ] {
            std::fs::create_dir_all(path).unwrap();
        }
        std::fs::write(custom.join("server.properties"), b"level-name=oak\n").unwrap();
        std::fs::write(custom.join("oak/level.dat"), b"world").unwrap();
        std::fs::write(custom.join("backups/save.zip"), b"zip").unwrap();
        std::fs::write(custom.join("backups/save.meta.json"), b"{}").unwrap();
        std::fs::write(data.join("runtimes/managed-java"), b"java").unwrap();
        std::fs::write(fixture.0.join("external-java"), b"not MSC-owned").unwrap();
        let mut config = AppConfig::default_config(managed.display().to_string());
        config.java_path = fixture.0.join("external-java").display().to_string();
        config.servers.push(ConfigServer::new(
            "server-1",
            "Survival",
            custom.display().to_string(),
            custom.join("server.jar").display().to_string(),
            1.0,
            2.0,
        ));
        let ownership = Inventory {
            computer: "fixture".into(),
            platform: Platform::Macos,
            entries: [
                (data.clone(), EntryKind::DataTree),
                (managed.clone(), EntryKind::ServerTree),
                (custom.clone(), EntryKind::ServerTree),
            ]
            .into_iter()
            .map(|(path, kind)| Entry {
                kind,
                path: Some(path),
                identity: "fixture".into(),
                evidence: "controlled native ownership".into(),
                state: EntryState::Present,
                problem: None,
            })
            .collect(),
            exclusions: vec![],
            warnings: vec![],
            fingerprint: "ownership-1".into(),
        };
        let manifest = discover(&Discovery {
            host_id: "fixture",
            config: &config,
            servers_root: &managed,
            ownership: &ownership,
            desktop_installations: &[],
        });
        let snapshot = storage::scan(&manifest, Limits::default(), &|| false, &mut |_| {});
        assert!(!snapshot.partial, "{:?}", snapshot.issues);
        assert_eq!(snapshot.logical_bytes, 29);
        assert_eq!(
            snapshot
                .items
                .iter()
                .filter(|i| i.group == Group::Worlds)
                .map(|i| i.logical_bytes)
                .sum::<u64>(),
            5
        );
        let backups: Vec<_> = snapshot
            .items
            .iter()
            .filter(|i| i.domain_id.as_deref() == Some("save.zip"))
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].logical_bytes, 5);
        assert_eq!(
            snapshot
                .items
                .iter()
                .filter(|i| i.category == Category::SharedTools)
                .map(|i| i.logical_bytes)
                .sum::<u64>(),
            4
        );
    }
}
