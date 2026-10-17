//! Read-only, bounded owned-root measurement. Inventory never grants removal.
#[path = "storage/native.rs"]
mod native;

use native::{Identity, Measurement};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Servers,
    DesktopApp,
    Agent,
    SharedTools,
    Cache,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Group {
    Worlds,
    Backups,
    Components,
    Other,
    Installation,
    Tools,
    Cache,
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub relative: PathBuf,
    pub category: Category,
    pub group: Group,
    pub label: String,
    /// Existing server identity; never a path or an execution authority.
    pub server_id: Option<String>,
    /// Aggregate immediate children as domain items, not one row per file.
    pub children: bool,
}

#[derive(Clone, Debug)]
pub struct Root {
    pub path: PathBuf,
    pub category: Category,
    pub label: String,
    pub server_id: Option<String>,
    pub evidence: String,
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug)]
pub struct Manifest {
    pub host_id: String,
    /// Caller-owned configuration/ownership generation; no credentials or config contents.
    pub generation: String,
    pub roots: Vec<Root>,
    pub excluded_roots: Vec<PathBuf>,
    pub issues: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub root_id: String,
    #[serde(default)]
    pub physical_root_id: String,
    pub volume_id: Option<String>,
    pub category: Category,
    pub group: Group,
    pub domain_id: Option<String>,
    pub server_id: Option<String>,
    pub label: String,
    #[serde(default)]
    pub display_path: Option<String>,
    pub logical_bytes: u64,
    pub allocated_bytes: Option<u64>,
    pub partial: bool,
    pub absent: bool,
    pub shared_references: u64,
    #[serde(default)]
    pub has_hard_links: bool,
    pub protection: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub id: String,
    #[serde(default)]
    pub physical_id: String,
    #[serde(default)]
    pub label: String,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub observed_at: u64,
    pub problem: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub root_id: Option<String>,
    pub reason: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub roots_completed: usize,
    pub entries: u64,
    pub logical_bytes_visited: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub schema_version: u32,
    pub id: String,
    pub host_id: String,
    pub generation: String,
    pub root_identity: String,
    pub started_at: u64,
    pub completed_at: u64,
    pub partial: bool,
    pub cancelled: bool,
    /// The display basis is always logical; allocation is separate and nullable.
    pub logical_bytes: u64,
    pub allocated_bytes: Option<u64>,
    pub items: Vec<Item>,
    pub volumes: Vec<Volume>,
    pub issues: Vec<Issue>,
    pub progress: Progress,
}

#[derive(Clone, Copy)]
pub struct Limits {
    pub entries: u64,
    pub depth: usize,
    pub duration: Duration,
    pub identities: usize,
    pub items: usize,
    pub item_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            entries: 1_000_000,
            depth: 30,
            duration: Duration::from_secs(300),
            identities: 100_000,
            items: 10_000,
            item_bytes: 8 * 1024 * 1024,
        }
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn identity(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

// These are read-only internal entry points. Callers supply discovery evidence,
// never deserialize a network request directly into a Manifest.
pub fn scan(
    manifest: &Manifest,
    limits: Limits,
    cancelled: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(&Progress),
) -> Snapshot {
    scan_with(manifest, limits, cancelled, progress, &mut |_| Ok(()))
}

struct Source<'a> {
    root: &'a Root,
    id: &'a str,
    volume: &'a str,
    device: u64,
    mount: u64,
}

struct Walker<'a> {
    manifest: &'a Manifest,
    limits: Limits,
    start: Instant,
    last_progress: Instant,
    cancelled: &'a dyn Fn() -> bool,
    report: &'a mut dyn FnMut(&Progress),
    inspect: &'a mut dyn FnMut(&Path) -> io::Result<()>,
    result: Snapshot,
    boundaries: BTreeSet<PathBuf>,
    seen: BTreeMap<Identity, (usize, u64, Option<u64>)>,
    directories: BTreeSet<Identity>,
    buckets: BTreeMap<(String, String), usize>,
    item_bytes: usize,
    root_ids: Vec<String>,
    physical_roots: BTreeMap<String, String>,
    stopped: bool,
}

fn within_manifest_budget(manifest: &Manifest) -> bool {
    if manifest.host_id.len() > 128 || manifest.generation.len() > 128 {
        return false;
    }
    if manifest.roots.len() > 512
        || manifest.excluded_roots.len() > 512
        || manifest.issues.len() > 256
    {
        return false;
    }
    let mut size = manifest
        .host_id
        .len()
        .saturating_add(manifest.generation.len());
    for root in &manifest.roots {
        if root.rules.len() > 128
            || root.path.as_os_str().len() > 4096
            || root.label.len() > 4096
            || root.evidence.len() > 4096
            || root.server_id.as_ref().is_some_and(|id| id.len() > 128)
        {
            return false;
        }
        size = size.saturating_add(
            root.path.as_os_str().len() * 4 + root.label.len() + root.evidence.len() + 512,
        );
        for rule in &root.rules {
            if rule.relative.as_os_str().len() > 4096
                || rule.label.len() > 4096
                || rule.server_id.as_ref().is_some_and(|id| id.len() > 128)
            {
                return false;
            }
            size =
                size.saturating_add(rule.relative.as_os_str().len() * 4 + rule.label.len() + 128);
        }
    }
    for path in &manifest.excluded_roots {
        size = size.saturating_add(path.as_os_str().len() * 4 + 128);
    }
    for reason in manifest.issues.iter().take(256) {
        size = size.saturating_add(reason.len());
    }
    size <= 1024 * 1024
}

fn scan_with(
    manifest: &Manifest,
    limits: Limits,
    cancelled: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(&Progress),
    inspect: &mut dyn FnMut(&Path) -> io::Result<()>,
) -> Snapshot {
    // Caller limits can only tighten the implementation's memory/handle bounds.
    let defaults = Limits::default();
    let limits = Limits {
        entries: limits.entries.min(defaults.entries),
        depth: limits.depth.min(defaults.depth),
        duration: limits.duration.min(defaults.duration),
        identities: limits.identities.min(defaults.identities),
        items: limits.items.min(defaults.items),
        item_bytes: limits.item_bytes.min(defaults.item_bytes),
    };
    let mut walker = Walker {
        manifest,
        limits,
        start: Instant::now(),
        last_progress: Instant::now(),
        cancelled,
        report: progress,
        inspect,
        result: Snapshot {
            schema_version: 1,
            id: String::new(),
            host_id: manifest.host_id.clone(),
            generation: manifest.generation.clone(),
            root_identity: String::new(),
            started_at: now(),
            completed_at: 0,
            partial: false,
            cancelled: false,
            logical_bytes: 0,
            allocated_bytes: Some(0),
            items: vec![],
            volumes: vec![],
            issues: vec![],
            progress: Progress::default(),
        },
        boundaries: BTreeSet::new(),
        seen: BTreeMap::new(),
        directories: BTreeSet::new(),
        buckets: BTreeMap::new(),
        item_bytes: 0,
        root_ids: vec![],
        physical_roots: BTreeMap::new(),
        stopped: false,
    };
    for reason in manifest.issues.iter().take(256) {
        walker.issue(None, reason);
    }
    // Bounded source/rule metadata leaves room for identity tables and item data.
    if !within_manifest_budget(manifest) {
        walker.issue(None, "Discovery exceeds the source metadata budget");
    } else {
        let mut roots: Vec<_> = manifest.roots.iter().collect();
        roots.sort_by(|a, b| a.path.cmp(&b.path));
        for root in &roots {
            walker.boundaries.insert(root.path.clone());
        }
        for excluded in &manifest.excluded_roots {
            walker.boundaries.insert(excluded.clone());
        }
        let mut previous: Option<&Path> = None;
        for root in roots {
            if manifest
                .excluded_roots
                .iter()
                .any(|excluded| root.path.starts_with(excluded))
            {
                walker.issue(
                    None,
                    "Source excluded because ownership could not be established",
                );
                continue;
            }
            if previous == Some(&root.path) {
                walker.issue(
                    None,
                    "Conflicting ownership of one root; counted once and protected",
                );
                continue;
            }
            previous = Some(&root.path);
            if walker.check().is_err() {
                break;
            }
            walker.root(root);
            walker.result.progress.roots_completed += 1;
        }
    }
    if walker.result.partial {
        // A skipped source can alias a measured item; do not certify any item
        // as complete after an incomplete host traversal.
        for item in &mut walker.result.items {
            item.partial = true;
        }
    }
    walker.result.logical_bytes = walker.result.items.iter().map(|i| i.logical_bytes).sum();
    walker.result.allocated_bytes = walker
        .result
        .items
        .iter()
        .try_fold(0u64, |total, i| total.checked_add(i.allocated_bytes?));
    walker.result.completed_at = now();
    walker.result.root_identity = identity(
        &walker
            .root_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    );
    static NEXT_SNAPSHOT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    walker.result.id = identity(&[
        &manifest.host_id,
        &manifest.generation,
        &walker.result.root_identity,
        &SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_string(),
        &NEXT_SNAPSHOT
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .to_string(),
    ]);
    (walker.report)(&walker.result.progress);
    walker.result
}

impl Walker<'_> {
    fn issue(&mut self, root: Option<&str>, reason: &str) {
        self.result.partial = true;
        if self.result.issues.len() < 256 {
            self.result.issues.push(Issue {
                root_id: root.map(str::to_owned),
                reason: reason.chars().take(512).collect(),
            });
        }
    }
    fn check(&mut self) -> io::Result<()> {
        if (self.cancelled)() {
            self.result.cancelled = true;
            self.stopped = true;
        }
        if self.start.elapsed() >= self.limits.duration
            || self.result.progress.entries >= self.limits.entries
        {
            self.stopped = true;
        }
        if self.stopped {
            self.issue(None, "Scan cancelled or traversal budget reached");
            return Err(io::Error::other("scan stopped"));
        }
        if self.last_progress.elapsed() >= Duration::from_millis(500) {
            (self.report)(&self.result.progress);
            self.last_progress = Instant::now();
        }
        Ok(())
    }
    fn bucket(
        &mut self,
        root: &Root,
        root_id: &str,
        volume: Option<&str>,
        relative: &Path,
    ) -> io::Result<usize> {
        let rule = root
            .rules
            .iter()
            .filter(|r| relative.starts_with(&r.relative))
            .max_by_key(|r| r.relative.components().count());
        let (category, server, label, key) = if let Some(rule) = rule {
            let child = if rule.children {
                relative
                    .strip_prefix(&rule.relative)
                    .ok()
                    .and_then(|p| p.components().next())
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
            } else {
                None
            };
            let child = child.map(|name| {
                if rule.group == Group::Backups {
                    name.strip_suffix(".meta.json")
                        .map_or_else(|| name.clone(), |base| format!("{base}.zip"))
                } else {
                    name
                }
            });
            let key = format!(
                "{}:{}",
                rule.relative.display(),
                child.as_deref().unwrap_or("")
            );
            (
                rule.category,
                rule.server_id.clone(),
                child.map_or_else(|| rule.label.clone(), |c| format!("{} / {c}", rule.label)),
                key,
            )
        } else {
            (
                root.category,
                root.server_id.clone(),
                root.label.clone(),
                String::new(),
            )
        };
        let domain_id = rule
            .filter(|rule| rule.children && matches!(rule.group, Group::Backups | Group::Worlds))
            .and_then(|_| key.split_once(':').map(|(_, child)| child.to_string()))
            .filter(|child| !child.is_empty());
        let group = rule.map_or_else(
            || match category {
                Category::Servers | Category::Agent => Group::Other,
                Category::DesktopApp => Group::Installation,
                Category::SharedTools => Group::Tools,
                Category::Cache => Group::Cache,
            },
            |r| r.group,
        );
        let lookup = (root_id.to_string(), key.clone());
        if let Some(index) = self.buckets.get(&lookup) {
            return Ok(*index);
        }
        let display_path = root.path.join(relative).to_string_lossy().into_owned();
        let bytes = serde_json::to_string(&[&label, &key, &display_path])
            .map_err(io::Error::other)?
            .len()
            + 768;
        if self.result.items.len() >= self.limits.items
            || self.item_bytes + bytes > self.limits.item_bytes
        {
            self.stopped = true;
            return Err(io::Error::other("inventory item budget reached"));
        }
        self.item_bytes += bytes;
        let index = self.result.items.len();
        self.result.items.push(Item { id: identity(&[&self.manifest.host_id, root_id, &key]), root_id: root_id.into(), physical_root_id: self.physical_roots.get(root_id).cloned().unwrap_or_default(),
            volume_id: volume.map(str::to_owned), category, group, domain_id, server_id: server, label,
            display_path: Some(display_path),
            logical_bytes: 0, allocated_bytes: Some(0), partial: false, absent: false, shared_references: 0, has_hard_links: false,
            protection: "Inventory only; ownership and live references must be checked by the action's owner".into() });
        self.buckets.insert(lookup, index);
        Ok(index)
    }
    fn root(&mut self, root: &Root) {
        let path_text = root.path.to_string_lossy();
        let missing_id = identity(&[&self.manifest.host_id, &path_text]);
        let file = match native::open(&root.path) {
            Ok(file) => file,
            Err(error) => {
                self.root_ids.push(missing_id.clone());
                if let Ok(index) = self.bucket(root, &missing_id, None, Path::new("")) {
                    self.result.items[index].partial = true;
                    self.result.items[index].allocated_bytes = None;
                }
                // Missing configured roots may be an unmounted disk, not absence.
                self.issue(Some(&missing_id), &format!("Source unavailable: {error}"));
                return;
            }
        };
        let meta = match native::measure(&file) {
            Ok(meta) => meta,
            Err(error) => {
                self.issue(Some(&missing_id), &error.to_string());
                return;
            }
        };
        let root_id = identity(&[
            &self.manifest.host_id,
            &path_text,
            &meta.identity.0.to_string(),
            &meta.identity.1.to_string(),
            &meta.mount.to_string(),
        ]);
        self.physical_roots.insert(
            root_id.clone(),
            identity(&[&meta.identity.0.to_string(), &meta.identity.1.to_string()]),
        );
        self.root_ids.push(root_id.clone());
        let volume = identity(&[&self.manifest.host_id, &meta.identity.0.to_string()]);
        if !self.result.volumes.iter().any(|v| v.id == volume) {
            // Check identity around pathname-based capacity inspection. It is a
            // dated sample, never permission or a guarantee about quotas.
            let capacity = fs2::total_space(&root.path)
                .and_then(|total| fs2::available_space(&root.path).map(|free| (total, free)));
            let unchanged = native::open(&root.path)
                .and_then(|f| native::measure(&f))
                .is_ok_and(|m| m.identity == meta.identity && m.mount == meta.mount);
            let (total, available, problem) = match capacity {
                Ok((total, free)) if unchanged => (Some(total), Some(free), None),
                result => (
                    None,
                    None,
                    Some(format!(
                        "Capacity unavailable or source changed: {result:?}"
                    )),
                ),
            };
            self.result.volumes.push(Volume {
                id: volume.clone(),
                physical_id: identity(&[&meta.identity.0.to_string()]),
                label: native::volume_label(&file, &root.path),
                total_bytes: total,
                available_bytes: available,
                observed_at: now(),
                problem,
            });
        }
        let source = Source {
            root,
            id: &root_id,
            volume: &volume,
            device: meta.identity.0,
            mount: meta.mount,
        };
        let result = self.visit(&source, &file, Path::new(""), 0);
        if let Err(error) = result {
            self.issue(Some(&root_id), &error.to_string());
        }
        if !native::open(&root.path)
            .and_then(|f| native::measure(&f))
            .is_ok_and(|m| m.identity == meta.identity && m.mount == meta.mount)
        {
            self.issue(Some(&root_id), "Source identity changed during scan");
        }
    }
    fn visit(
        &mut self,
        source: &Source<'_>,
        file: &File,
        relative: &Path,
        depth: usize,
    ) -> io::Result<()> {
        let Source {
            root,
            id: root_id,
            volume,
            device,
            mount,
        } = *source;
        self.check()?;
        self.result.progress.entries += 1;
        if depth > self.limits.depth {
            return Err(io::Error::other("depth/open-handle budget reached"));
        }
        let path = root.path.join(relative);
        if !relative.as_os_str().is_empty() && self.boundaries.contains(&path) {
            return Ok(());
        }
        (self.inspect)(&path)?;
        let meta = native::measure(file)?;
        if meta.identity.0 != device || meta.mount != mount {
            return Err(io::Error::other("unconfigured mounted volume excluded"));
        }
        let bucket = self.bucket(root, root_id, Some(volume), relative)?;
        if file.metadata()?.is_dir() {
            if self.directories.contains(&meta.identity) {
                return Ok(());
            }
            self.reserve_identity()?;
            self.directories.insert(meta.identity);
            let before = crate::source_tree::fingerprint(file)?;
            let result = crate::source_tree::storage_children(file, |name, child, _directory| {
                let relative = relative.join(name);
                if relative.as_os_str().len() > 4096 {
                    return Err(io::Error::other("path length budget reached"));
                }
                if let Err(error) = self.visit(source, &child, &relative, depth + 1) {
                    self.issue(Some(root_id), &error.to_string());
                    if self.stopped {
                        return Err(error);
                    }
                }
                Ok(())
            });
            if crate::source_tree::fingerprint(file)? != before {
                self.issue(Some(root_id), "Directory changed during scan");
            }
            result
        } else {
            self.charge(bucket, meta)
        }
    }
    fn reserve_identity(&mut self) -> io::Result<()> {
        if self.seen.len() + self.directories.len() >= self.limits.identities {
            self.stopped = true;
            return Err(io::Error::other("identity memory budget reached"));
        }
        Ok(())
    }
    fn charge(&mut self, bucket: usize, meta: Measurement) -> io::Result<()> {
        self.result.items[bucket].has_hard_links |= meta.links != Some(1);
        self.result.progress.logical_bytes_visited = self
            .result
            .progress
            .logical_bytes_visited
            .saturating_add(meta.logical);
        if let Some(&(old, logical, allocated)) = self.seen.get(&meta.identity) {
            self.result.items[bucket].shared_references += 1;
            let rank = |i: &Item| (i.category, i.root_id.clone(), i.id.clone());
            if meta.logical != logical || meta.allocated != allocated {
                self.issue(None, "Hard-linked file changed during scan");
            }
            if rank(&self.result.items[bucket]) < rank(&self.result.items[old]) {
                self.result.items[old].logical_bytes -= logical;
                if let (Some(total), Some(bytes)) =
                    (&mut self.result.items[old].allocated_bytes, allocated)
                {
                    *total -= bytes;
                }
                self.result.items[bucket].logical_bytes += logical;
                self.result.items[bucket].allocated_bytes = self.result.items[bucket]
                    .allocated_bytes
                    .zip(allocated)
                    .and_then(|(a, b)| a.checked_add(b));
                self.seen
                    .insert(meta.identity, (bucket, logical, allocated));
            }
            return Ok(());
        }
        self.reserve_identity()?;
        self.result.items[bucket].logical_bytes = self.result.items[bucket]
            .logical_bytes
            .checked_add(meta.logical)
            .ok_or_else(|| io::Error::other("logical size overflow"))?;
        self.result.items[bucket].allocated_bytes = self.result.items[bucket]
            .allocated_bytes
            .zip(meta.allocated)
            .and_then(|(a, b)| a.checked_add(b));
        self.seen
            .insert(meta.identity, (bucket, meta.logical, meta.allocated));
        Ok(())
    }
}

/// Read only a small known metadata file through the no-follow boundary.
pub fn read_small(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    use std::io::Read;
    let file = native::open_read(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > limit {
        return Err(io::Error::other("metadata exceeds the inspection budget"));
    }
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(io::Error::other(
            "metadata grew beyond the inspection budget",
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("msc-storage-{}-{}", std::process::id(), now()));
            // Unit cases run concurrently; an atomic suffix avoids shared state.
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let path = path.with_extension(
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    .to_string(),
            );
            std::fs::create_dir(&path).unwrap();
            Self(std::fs::canonicalize(path).unwrap())
        }
        fn root(&self, path: &str, category: Category) -> Root {
            Root {
                path: self.0.join(path),
                category,
                label: path.into(),
                server_id: None,
                evidence: "controlled owned fixture".into(),
                rules: vec![],
            }
        }
        fn manifest(&self, roots: Vec<Root>) -> Manifest {
            Manifest {
                host_id: "fixture-host".into(),
                generation: "config-1".into(),
                roots,
                excluded_roots: vec![],
                issues: vec![],
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    // Existing uninstall/source-copy tests do not check host-wide deduplication
    // or category partitioning. Tiny files, no clocks/networks, expected <1 s.
    #[test]
    fn overlapping_roots_and_hard_links_are_counted_once() {
        let fixture = Fixture::new();
        std::fs::create_dir_all(fixture.0.join("data/cache")).unwrap();
        std::fs::write(fixture.0.join("data/keep"), b"123").unwrap();
        let payload = if cfg!(unix) {
            "data/cache/payload:cache"
        } else {
            "data/cache/payload"
        };
        std::fs::write(fixture.0.join(payload), b"12345").unwrap();
        std::fs::hard_link(
            fixture.0.join("data/keep"),
            fixture.0.join("data/cache/alias"),
        )
        .unwrap();
        let manifest = fixture.manifest(vec![
            fixture.root("data", Category::Agent),
            fixture.root("data/cache", Category::Cache),
        ]);
        let snapshot = scan(&manifest, Limits::default(), &|| false, &mut |_| {});
        assert!(!snapshot.partial, "{:?}", snapshot.issues);
        assert_eq!(snapshot.logical_bytes, 8);
        assert_eq!(
            snapshot
                .items
                .iter()
                .filter(|i| i.category == Category::Agent)
                .map(|i| i.logical_bytes)
                .sum::<u64>(),
            3
        );
        assert_eq!(
            snapshot
                .items
                .iter()
                .filter(|i| i.category == Category::Cache)
                .map(|i| i.logical_bytes)
                .sum::<u64>(),
            5
        );
        assert_eq!(snapshot.volumes.len(), 1);
        assert_eq!(
            snapshot
                .items
                .iter()
                .map(|i| i.shared_references)
                .sum::<u64>(),
            1
        );
        let stopped = scan(&manifest, Limits::default(), &|| true, &mut |_| {});
        assert!(stopped.cancelled && stopped.partial);
        let limited = scan(
            &manifest,
            Limits {
                identities: 1,
                ..Limits::default()
            },
            &|| false,
            &mut |_| {},
        );
        assert!(limited.partial);
        assert!(limited.logical_bytes < 8);
    }

    // Inject denial instead of chmod: this remains meaningful under root and on
    // Windows. Protects against reporting unreadable sources as complete zero.
    #[test]
    fn denied_source_and_blocked_child_remain_partial_without_losing_known_bytes() {
        let fixture = Fixture::new();
        std::fs::create_dir_all(fixture.0.join("data/blocked")).unwrap();
        std::fs::write(fixture.0.join("data/known"), b"123").unwrap();
        std::fs::write(fixture.0.join("data/denied"), b"1234567").unwrap();
        std::fs::write(fixture.0.join("data/blocked/private"), b"do not inspect").unwrap();
        let mut manifest = fixture.manifest(vec![fixture.root("data", Category::Agent)]);
        manifest.excluded_roots.push(fixture.0.join("data/blocked"));
        manifest.issues.push("Child ownership is unknown".into());
        let result = scan_with(
            &manifest,
            Limits::default(),
            &|| false,
            &mut |_| {},
            &mut |path| {
                if path == fixture.0.join("data/denied") {
                    Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "controlled denial",
                    ))
                } else {
                    assert!(!path.starts_with(fixture.0.join("data/blocked")));
                    Ok(())
                }
            },
        );
        assert!(result.partial);
        assert_eq!(result.logical_bytes, 3);
        assert!(result.items.iter().all(|i| i.partial));
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.reason.contains("controlled denial"))
        );
    }

    // Outside files and a symlinked ancestor must never enter owned totals.
    // Unix symlinks need no machine-dependent Windows developer-mode privilege.
    #[cfg(unix)]
    #[test]
    fn link_boundaries_cannot_scan_external_files_or_substituted_ancestors() {
        let fixture = Fixture::new();
        std::fs::create_dir_all(fixture.0.join("owned")).unwrap();
        std::fs::create_dir_all(fixture.0.join("outside/nested")).unwrap();
        std::fs::write(
            fixture.0.join("outside/nested/private"),
            b"private user data",
        )
        .unwrap();
        std::os::unix::fs::symlink(fixture.0.join("outside"), fixture.0.join("owned/link"))
            .unwrap();
        let manifest = fixture.manifest(vec![fixture.root("owned", Category::Agent)]);
        let snapshot = scan(&manifest, Limits::default(), &|| false, &mut |_| {});
        assert!(snapshot.partial);
        assert_eq!(snapshot.logical_bytes, 0);
        let manifest = fixture.manifest(vec![fixture.root("owned/link/nested", Category::Agent)]);
        let snapshot = scan(&manifest, Limits::default(), &|| false, &mut |_| {});
        assert!(snapshot.partial);
        assert_eq!(snapshot.logical_bytes, 0);
        assert!(read_small(&fixture.0.join("owned/link/nested/private"), 1024).is_err());
    }
}

/// Re-resolve a known discovered root before native reveal/review. Callers
/// accept an inventory ID, never an arbitrary path from an external request.
pub fn verified_root_id(host: &str, path: &Path) -> io::Result<String> {
    let file = native::open(path)?;
    let meta = native::measure(&file)?;
    Ok(identity(&[
        host,
        &path.to_string_lossy(),
        &meta.identity.0.to_string(),
        &meta.identity.1.to_string(),
        &meta.mount.to_string(),
    ]))
}

/// Bounded metadata observation for an owned cleanup target. File contents are
/// never buffered; root identity and all child metadata participate in the proof.
pub fn observe_tree(path: &Path, cancelled: &dyn Fn() -> bool) -> io::Result<(u64, String)> {
    struct Observation<'a> {
        cancelled: &'a dyn Fn() -> bool,
        start: Instant,
        entries: u64,
        bytes: u64,
        seen: BTreeSet<Identity>,
        digest: [u8; 32],
    }
    fn visit(
        file: &File,
        relative: &Path,
        depth: usize,
        observation: &mut Observation<'_>,
    ) -> io::Result<()> {
        if (observation.cancelled)()
            || observation.start.elapsed() > Duration::from_secs(300)
            || depth > 30
            || observation.entries >= 1_000_000
        {
            return Err(io::Error::other(
                "Cleanup inspection cancelled or budget reached",
            ));
        }
        observation.entries += 1;
        let before = crate::source_tree::fingerprint(file)?;
        let mut hash = Sha256::new();
        hash.update(relative.as_os_str().as_encoded_bytes());
        hash.update(&before);
        for (slot, byte) in observation.digest.iter_mut().zip(hash.finalize()) {
            *slot ^= byte;
        }
        let meta = native::measure(file)?;
        if !file.metadata()?.is_dir() {
            if !observation.seen.contains(&meta.identity) {
                if observation.seen.len() >= 100_000 {
                    return Err(io::Error::other("Cleanup identity budget reached"));
                }
                observation.bytes = observation
                    .bytes
                    .checked_add(meta.logical)
                    .ok_or_else(|| io::Error::other("Cleanup size overflow"))?;
                observation.seen.insert(meta.identity);
            }
        } else {
            crate::source_tree::storage_children(file, |name, child, _| {
                visit(&child, &relative.join(name), depth + 1, observation)
            })?;
        }
        if crate::source_tree::fingerprint(file)? != before {
            return Err(io::Error::other("Cleanup source changed during inspection"));
        }
        Ok(())
    }
    let root = native::open(path)?;
    let mut observation = Observation {
        cancelled,
        start: Instant::now(),
        entries: 0,
        bytes: 0,
        seen: BTreeSet::new(),
        digest: [0; 32],
    };
    visit(&root, Path::new(""), 0, &mut observation)?;
    Ok((
        observation.bytes,
        format!("{:x}", Sha256::digest(observation.digest)),
    ))
}

pub fn open_owned_file(path: &Path) -> io::Result<File> {
    native::open_read(path)
}
