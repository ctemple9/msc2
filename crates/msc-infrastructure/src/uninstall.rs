//! Read-only inventory for complete local uninstall. This module deliberately
//! has no deletion or process-control functions; execution must revalidate it.

pub mod native;

use crate::fs::FileSystem;
use crate::path_safety::safe_path;
use crate::service::{ServiceInstallRequest, ServiceState, ServiceStatusReport};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io;
use std::path::{Component, Path, PathBuf};

pub const CONFIRMATION: &str = "UNINSTALL MSC 2";
const AGENT: &str = "com.ctemple.msc2.agent";
const CONFIG: &str = "server_config_swift.json";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Platform {
    Macos,
    Windows,
    Linux,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    DataTree,
    ServerTree,
    Configuration,
    ClientState,
    Installation,
    Installer,
    CommandLink,
    Service,
    CredentialNamespace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EntryState {
    Present,
    Missing,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub kind: EntryKind,
    pub path: Option<PathBuf>,
    pub identity: String,
    pub evidence: String,
    pub state: EntryState,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub computer: String,
    pub platform: Platform,
    pub entries: Vec<Entry>,
    pub exclusions: Vec<String>,
    pub warnings: Vec<String>,
    /// Fingerprint of the preview, not authority to execute it. The native
    /// executor must rediscover targets and compare before asking to delete.
    pub fingerprint: String,
}

impl Inventory {
    pub fn is_blocked(&self) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.state == EntryState::Blocked)
    }
}

/// Values come from local OS service inspection, never the active HTTP host.
/// Keep only path overrides: serializing the complete service environment
/// could expose passwords or bootstrap credentials in the preview.
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    pub data_dir: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
    pub servers_root: Option<PathBuf>,
}

impl Overrides {
    pub fn from_environment(environment: &BTreeMap<String, String>) -> Self {
        Self {
            data_dir: environment.get("MSC2_DATA_DIR").map(PathBuf::from),
            config_path: environment.get("MSC2_APP_CONFIG_PATH").map(PathBuf::from),
            servers_root: environment
                .get("MSC2_AGENT_SERVERS_ROOT")
                .map(PathBuf::from),
        }
    }
}

/// Platform adapters supply locally inspected definitions and package identity
/// findings. No remote hosts or tokens are accepted by the shared inventory.
pub struct Request {
    pub computer: String,
    pub platform: Platform,
    pub home: PathBuf,
    pub overrides: Overrides,
    pub services: Vec<ServiceStatusReport>,
    pub protected_paths: Vec<PathBuf>,
    /// Additional platform-resolved WebView/cache locations, scoped to MSC's
    /// bundle ID. Unknown locations must be reported as findings, not guesses.
    pub client_paths: Vec<PathBuf>,
    pub headless_roots: Vec<PathBuf>,
    pub installer_candidates: Vec<InstallerCandidate>,
    pub findings: Vec<Entry>,
}

pub struct InstallerCandidate {
    pub path: PathBuf,
    /// Set only after verifying signed release metadata. Filenames alone are
    /// never evidence that a downloaded file is an MSC installer.
    pub verified_sha256: Option<String>,
}

pub fn inventory(fs: &dyn FileSystem, request: &Request) -> Inventory {
    let mut result = Inventory {
        computer: request.computer.clone(),
        platform: request.platform,
        entries: Vec::new(),
        exclusions: vec![
            "Remote agents and their servers (no remote requests are made)".into(),
            "MSC 1, source checkouts, and unrelated files".into(),
            "Separately installed Java, Tailscale, Docker, and OS package caches".into(),
        ],
        warnings: vec![
            "Unknown or renamed installers are not discovered by filename; select and verify them explicitly.".into(),
            "A preview does not stop servers, elevate privileges, or remove anything.".into(),
        ],
        fingerprint: String::new(),
    };
    let home = &request.home;
    if !home.is_absolute() {
        result.entries.push(blocked(
            EntryKind::DataTree,
            home,
            "Local home",
            "The local home must be absolute.",
        ));
        finish(&mut result);
        return result;
    }
    let default_roots = match request.platform {
        Platform::Macos => vec![
            home.join("Library/Application Support/MSC 2"),
            home.join("Library/Application Support/MSC2"),
        ],
        Platform::Windows => vec![
            home.join("AppData/Roaming/MSC2"),
            home.join("AppData/Local/MSC2"),
        ],
        Platform::Linux => vec![
            home.join(".local/share/msc2"),
            PathBuf::from("/var/lib/msc2"),
        ],
    };
    let mut roots: Vec<(PathBuf, Overrides)> = default_roots
        .into_iter()
        .map(|path| (path, Overrides::default()))
        .collect();
    if let Some(data) = &request.overrides.data_dir {
        roots.push((data.clone(), request.overrides.clone()));
    } else {
        roots.push((roots[0].0.clone(), request.overrides.clone()));
    }
    for report in &request.services {
        if !matches!(
            report.service_name.as_str(),
            AGENT | "com.ctemple.msc2.bedrock-helper" | "msc2-credential-helper"
        ) {
            result.entries.push(Entry {
                kind: EntryKind::Service,
                path: None,
                identity: report.service_name.as_str().into(),
                evidence: "Local service inspection".into(),
                state: EntryState::Blocked,
                problem: Some("Unexpected service identity; refuse to control it.".into()),
            });
            continue;
        }
        result.entries.push(Entry {
            kind: EntryKind::Service,
            path: None,
            identity: report.service_name.as_str().into(),
            evidence: format!("Local OS service state: {:?}", report.state),
            state: if report.state == ServiceState::NotInstalled {
                EntryState::Missing
            } else {
                EntryState::Present
            },
            problem: None,
        });
        if report.state != ServiceState::NotInstalled && report.definition.is_none() {
            result.entries.last_mut().unwrap().state = EntryState::Blocked;
            result.entries.last_mut().unwrap().problem =
                Some("Installed service definition could not be inspected.".into());
        }
        if report.service_name.as_str() == AGENT
            && let Some(definition) = &report.definition
        {
            add_service_roots(definition, &mut roots);
        }
    }
    if request.services.is_empty() {
        result.entries.push(Entry {
            kind: EntryKind::Service,
            path: None,
            identity: AGENT.into(),
            evidence: "Local inspection required".into(),
            state: EntryState::Blocked,
            problem: Some(
                "No local service inspection was supplied; do not assume the agent is absent."
                    .into(),
            ),
        });
    }
    for (root, overrides) in roots {
        add_data(fs, request, &mut result, &root, &overrides);
    }
    let mut client_paths = request.client_paths.clone();
    match request.platform {
        Platform::Macos => {
            client_paths.extend([
                home.join("Library/Application Support/com.ctemple.msc2"),
                home.join("Library/Caches/com.ctemple.msc2"),
                home.join("Library/WebKit/com.ctemple.msc2"),
                home.join("Library/Preferences/com.ctemple.msc2.plist"),
                home.join("Library/Saved Application State/com.ctemple.msc2.savedState"),
            ]);
        }
        Platform::Windows => client_paths.extend([
            home.join("AppData/Local/com.ctemple.msc2"),
            home.join("AppData/Roaming/com.ctemple.msc2"),
        ]),
        Platform::Linux => client_paths.extend([
            home.join(".config/com.ctemple.msc2"),
            home.join(".cache/com.ctemple.msc2"),
            home.join(".local/share/com.ctemple.msc2"),
        ]),
    }
    for path in client_paths {
        add_path(
            fs,
            request,
            &mut result,
            EntryKind::ClientState,
            &path,
            "MSC bundle-ID client state",
        );
    }
    if request.platform == Platform::Windows {
        result.entries.push(Entry {
            kind: EntryKind::CredentialNamespace,
            path: None,
            identity: "MSC2:".into(),
            evidence: "MSC's Windows Credential Manager target prefix, current user only".into(),
            state: EntryState::Present,
            problem: None,
        });
    }
    for root in &request.headless_roots {
        add_headless(fs, request, &mut result, root);
    }
    for candidate in &request.installer_candidates {
        add_installer(fs, request, &mut result, candidate);
    }
    // Native package/bundle findings are displayed, not silently upgraded into
    // filesystem deletion permission. P19.2 must validate the OS identity again.
    for finding in &request.findings {
        let mut finding = finding.clone();
        if let Some(path) = &finding.path
            && let Err(problem) = validate_target(fs, request, path)
        {
            finding.state = EntryState::Blocked;
            finding.problem = Some(problem);
        }
        result.entries.push(finding);
    }
    if !request
        .findings
        .iter()
        .any(|entry| entry.kind == EntryKind::Installation)
    {
        result.entries.push(Entry {
            kind: EntryKind::Installation, path: None, identity: "Desktop/package inspection".into(),
            evidence: "Local native inspection required".into(), state: EntryState::Blocked,
            problem: Some("The desktop/package installation has not been inspected; absence cannot be assumed.".into()),
        });
    }
    block_overlapping_targets(&mut result);
    finish(&mut result);
    result
}

fn add_service_roots(definition: &ServiceInstallRequest, roots: &mut Vec<(PathBuf, Overrides)>) {
    let overrides = Overrides::from_environment(&definition.environment);
    let root = overrides
        .data_dir
        .clone()
        .unwrap_or_else(|| definition.working_directory.clone());
    roots.push((root, overrides));
}

fn add_data(
    fs: &dyn FileSystem,
    request: &Request,
    result: &mut Inventory,
    root: &Path,
    overrides: &Overrides,
) {
    let before = result.entries.len();
    add_path(
        fs,
        request,
        result,
        EntryKind::DataTree,
        root,
        "Platform MSC data directory or locally inspected service path override",
    );
    if result.entries[before].state == EntryState::Blocked {
        return;
    }
    let config = overrides
        .config_path
        .clone()
        .unwrap_or_else(|| root.join(CONFIG));
    add_path(
        fs,
        request,
        result,
        EntryKind::Configuration,
        &config,
        "Local agent configuration path",
    );
    let bytes = match fs.read(&config) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if overrides.data_dir.is_some() && fs.stat(root).is_ok() {
                result.entries.push(blocked(EntryKind::DataTree, root, "Custom local data path", "Custom data directory has no readable MSC configuration; ownership is ambiguous."));
            }
            if let Some(server_root) = &overrides.servers_root {
                result.entries.push(blocked(
                    EntryKind::ServerTree,
                    server_root,
                    "Custom servers root",
                    "No readable MSC configuration establishes ownership of this server root.",
                ));
            }
            return;
        }
        Err(error) => {
            result.entries.push(blocked(
                EntryKind::Configuration,
                &config,
                "Local configuration",
                &format!("Cannot inspect configuration: {error}"),
            ));
            return;
        }
    };
    let defaults = msc_domain::app_config_schema::AppConfig::default_config(
        root.join("servers").display().to_string(),
    );
    let config_value = serde_json::from_slice::<serde_json::Value>(&bytes);
    let config_value = match config_value {
        Ok(value) => value,
        Err(error) => {
            result.entries.push(blocked(
                EntryKind::Configuration,
                &config,
                "Local configuration",
                &format!(
                    "Corrupt configuration; preserve it until targets can be reviewed: {error}"
                ),
            ));
            return;
        }
    };
    if !config_value
        .get("servers")
        .is_some_and(serde_json::Value::is_array)
        || !config_value
            .get("config_version")
            .is_some_and(serde_json::Value::is_number)
    {
        result.entries.push(blocked(
            EntryKind::Configuration,
            &config,
            "Local configuration",
            "File does not contain the MSC configuration version and server registry.",
        ));
        return;
    }
    let app = match msc_domain::app_config_schema::AppConfig::decode(&config_value, &defaults) {
        Ok(app) => app,
        Err(error) => {
            result.entries.push(blocked(
                EntryKind::Configuration,
                &config,
                "Local configuration",
                &format!("Invalid configuration: {error}"),
            ));
            return;
        }
    };
    let server_root = overrides
        .servers_root
        .clone()
        .unwrap_or_else(|| PathBuf::from(&app.servers_root));
    add_path(
        fs,
        request,
        result,
        EntryKind::ServerTree,
        &server_root,
        "Configured local managed servers root (entire tree, including worlds/backups)",
    );
    for server in &app.servers {
        let path = PathBuf::from(&server.server_dir);
        // Registry membership is shown as evidence, but does not override any
        // protected path or symlink boundary, even for an imported server.
        add_path(
            fs,
            request,
            result,
            EntryKind::ServerTree,
            &path,
            &format!(
                "Registered local server {} ({})",
                server.display_name, server.id
            ),
        );
    }
}

fn add_path(
    fs: &dyn FileSystem,
    request: &Request,
    result: &mut Inventory,
    kind: EntryKind,
    path: &Path,
    evidence: &str,
) {
    if let Err(problem) = validate_target(fs, request, path) {
        result.entries.push(blocked(kind, path, evidence, &problem));
        return;
    }
    let (state, problem) = match fs.stat(path) {
        Ok(_) => (EntryState::Present, None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => (EntryState::Missing, None),
        Err(error) => (
            EntryState::Blocked,
            Some(format!("Cannot inspect target: {error}")),
        ),
    };
    result.entries.push(Entry {
        kind,
        path: Some(path.to_path_buf()),
        identity: path.display().to_string(),
        evidence: evidence.into(),
        state,
        problem,
    });
}

/// Public so the executor can repeat the same boundary check immediately before
/// deletion. Symbolic links in the target's ancestry are refused, not followed.
pub fn validate_target(fs: &dyn FileSystem, request: &Request, path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err("Target must be absolute without traversal components.".into());
    }
    let canonical = safe_path(fs, path, None, &request.home).map_err(|error| error.to_string())?;
    if canonical != path {
        return Err("Target or an ancestor resolves through a symbolic link; review the real location separately.".into());
    }
    if path
        .components()
        .any(|component| component.as_os_str() == ".git")
    {
        return Err("Source-control metadata is protected.".into());
    }
    if fs.stat(&path.join(".git")).is_ok() || fs.stat(&path.join("Cargo.toml")).is_ok() {
        return Err("Refusing to inventory a developer source tree for removal.".into());
    }
    let mut prefix = PathBuf::new();
    for part in path.components() {
        prefix.push(part);
        if fs.stat(&prefix.join(".git")).is_ok() || fs.stat(&prefix.join("Cargo.toml")).is_ok() {
            return Err("Target is inside a developer source tree.".into());
        }
        if fs.read_link(&prefix).is_ok() {
            return Err("Target contains a symbolic link.".into());
        }
    }
    let protected = [
        request.home.clone(),
        request
            .home
            .join("Documents/Swift Projects/minecraft-server-controller"),
    ];
    for keep in protected.iter().chain(request.protected_paths.iter()) {
        if keep.starts_with(path)
            || path == keep
            || (keep != &request.home && path.starts_with(keep))
        {
            return Err(format!(
                "Target overlaps protected location {}.",
                keep.display()
            ));
        }
    }
    let shared = [
        "/Applications",
        "/Library",
        "/Library/Application Support",
        "/usr",
        "/usr/local",
        "/usr/local/lib",
        "/usr/lib",
        "/var",
        "/var/lib",
        "/srv",
        "/opt",
        "/etc",
        "/home",
        "/Users",
        "/tmp",
    ];
    if shared.iter().any(|shared| path == Path::new(shared))
        || path
            .components()
            .filter(|part| matches!(part, Component::Normal(_)))
            .count()
            < 2
    {
        return Err("Target is a shared parent directory, not an MSC-owned object.".into());
    }
    for relative in [
        "Documents",
        "Downloads",
        "Desktop",
        "Library",
        "Library/Application Support",
        "Library/Caches",
        ".local",
        ".local/share",
        ".config",
        ".cache",
        "AppData",
        "AppData/Local",
        "AppData/Roaming",
    ] {
        if path == request.home.join(relative) {
            return Err("Target is a shared user directory.".into());
        }
    }
    Ok(())
}

fn add_headless(fs: &dyn FileSystem, request: &Request, result: &mut Inventory, root: &Path) {
    if validate_target(fs, request, root).is_err() {
        result.entries.push(blocked(
            EntryKind::Installation,
            root,
            "Headless install root",
            "Unsafe installation root.",
        ));
        return;
    }
    let marker = root.join(".msc2-owned");
    match fs.read(&marker) {
        Ok(bytes) if bytes == b"msc2-headless-archive\n" || bytes == b"msc2-headless-archive" => {
            add_path(
                fs,
                request,
                result,
                EntryKind::Installation,
                root,
                "MSC headless ownership marker",
            )
        }
        Ok(_) => result.entries.push(blocked(
            EntryKind::Installation,
            root,
            "Headless install",
            "Unrecognized ownership marker.",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            // macOS keeps marked version directories inside an architecture
            // root. Descend only one level; never crawl an arbitrary disk tree.
            match fs.list(root) {
                Ok(children) => {
                    for child in children {
                        if child.parent() != Some(root) {
                            continue;
                        }
                        if fs.read(&child.join(".msc2-owned")).is_ok() {
                            add_headless(fs, request, result, &child);
                        } else {
                            result.warnings.push(format!(
                                "Unmarked installation content retained: {}",
                                child.display()
                            ));
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => add_path(
                    fs,
                    request,
                    result,
                    EntryKind::Installation,
                    root,
                    "Headless installation not present",
                ),
                Err(error) => result.entries.push(blocked(
                    EntryKind::Installation,
                    root,
                    "Headless install",
                    &error.to_string(),
                )),
            }
        }
        Err(error) => result.entries.push(blocked(
            EntryKind::Installation,
            root,
            "Headless install",
            &error.to_string(),
        )),
    }
}

fn add_installer(
    fs: &dyn FileSystem,
    request: &Request,
    result: &mut Inventory,
    candidate: &InstallerCandidate,
) {
    let Some(expected) = &candidate.verified_sha256 else {
        result.entries.push(blocked(
            EntryKind::Installer,
            &candidate.path,
            "Selected installer",
            "Installer has no verified signed-release checksum.",
        ));
        return;
    };
    if let Err(problem) = validate_target(fs, request, &candidate.path) {
        result.entries.push(blocked(
            EntryKind::Installer,
            &candidate.path,
            "Selected installer",
            &problem,
        ));
        return;
    }
    match fs.read(&candidate.path) {
        Ok(bytes) if hex(&Sha256::digest(&bytes)).eq_ignore_ascii_case(expected) => add_path(
            fs,
            request,
            result,
            EntryKind::Installer,
            &candidate.path,
            &format!("SHA-256 matches verified MSC signed release metadata: {expected}"),
        ),
        Ok(_) => result.entries.push(blocked(
            EntryKind::Installer,
            &candidate.path,
            "Selected installer",
            "Installer checksum changed or is not an MSC release artifact.",
        )),
        Err(error) => result.entries.push(blocked(
            EntryKind::Installer,
            &candidate.path,
            "Selected installer",
            &error.to_string(),
        )),
    }
}

fn block_overlapping_targets(result: &mut Inventory) {
    let blocked_paths: Vec<_> = result
        .entries
        .iter()
        .filter(|entry| entry.state == EntryState::Blocked)
        .filter_map(|entry| entry.path.clone())
        .collect();
    for entry in &mut result.entries {
        if let Some(path) = &entry.path
            && blocked_paths
                .iter()
                .any(|blocked| blocked.starts_with(path))
        {
            entry.state = EntryState::Blocked;
            entry.problem.get_or_insert_with(|| "Contains another blocked target; removing its parent would bypass the protection.".into());
        }
    }
}

fn blocked(kind: EntryKind, path: &Path, evidence: &str, problem: &str) -> Entry {
    Entry {
        kind,
        path: Some(path.to_path_buf()),
        identity: path.display().to_string(),
        evidence: evidence.into(),
        state: EntryState::Blocked,
        problem: Some(problem.into()),
    }
}

fn finish(result: &mut Inventory) {
    result.entries.sort_by(|a, b| {
        a.identity
            .cmp(&b.identity)
            .then(a.evidence.cmp(&b.evidence))
    });
    result.entries.dedup_by(|a, b| {
        a.identity == b.identity
            && a.evidence == b.evidence
            && a.kind == b.kind
            && a.state == b.state
    });
    result.fingerprint = hex(&Sha256::digest(
        serde_json::to_vec(&(&result.computer, &result.platform, &result.entries))
            .expect("inventory serializes"),
    ));
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
