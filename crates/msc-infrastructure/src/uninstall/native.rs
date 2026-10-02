//! Local discovery and unprivileged cleanup shared by desktop/CLI. Elevated
//! service and installed-file removal remain narrow platform operations.
use super::*;
use crate::fs::StdFileSystem;
use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Installation {
    MacBundle { path: PathBuf },
    MarkedArchive { path: PathBuf },
    LinuxArchive { path: PathBuf },
    Deb { name: String },
    Rpm { name: String },
    Msi { product: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    pub origin: PathBuf,
    pub desktop_origin: Option<PathBuf>,
    pub installers: Vec<PathBuf>,
}

pub trait LocalServices {
    fn inspect(&self) -> Result<Vec<ServiceStatusReport>, String>;
    fn remove(&self) -> Result<(), String>;
    fn remove_installation(&self, installation: &Installation) -> Result<(), String>;
    fn remove_data(&self, path: &Path) -> Result<(), String> {
        remove_path(path)
    }
    fn clear_credentials(&self) -> Result<(), String>;
}

pub struct Preview {
    pub inventory: Inventory,
    pub request: Request,
    pub installations: Vec<Installation>,
    pub agent_data_dir: PathBuf,
    pub agent_running: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemovalResult {
    pub state: &'static str,
    pub removed: Vec<String>,
    pub remaining: Vec<String>,
}

/// Do not assume an absent service on an inspection failure. Package/bundle
/// identity must come from the OS or MSC's explicit ownership marker.
pub fn discover(
    services: &dyn LocalServices,
    options: &Options,
    trusted_key: Option<&str>,
) -> Result<Preview, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or("Cannot determine the current user's home.")?;
    let reports = services.inspect()?;
    let agent = reports
        .iter()
        .find(|report| report.service_name.as_str() == AGENT);
    let platform = if cfg!(target_os = "macos") {
        Platform::Macos
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else {
        Platform::Linux
    };
    let mut overrides = Overrides::default();
    for (key, slot) in [
        ("MSC2_DATA_DIR", &mut overrides.data_dir),
        ("MSC2_APP_CONFIG_PATH", &mut overrides.config_path),
        ("MSC2_AGENT_SERVERS_ROOT", &mut overrides.servers_root),
    ] {
        *slot = std::env::var_os(key)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
    }
    let agent_data_dir = agent
        .and_then(|report| report.definition.as_ref())
        .map(|definition| {
            Overrides::from_environment(&definition.environment)
                .data_dir
                .unwrap_or_else(|| definition.working_directory.clone())
        })
        .unwrap_or_else(crate::config_repository::default_app_data_dir);
    // Refuse another installing account; complete removal is per installing user.
    if let Some(user) = agent
        .and_then(|report| report.definition.as_ref())
        .and_then(|definition| definition.run_user.as_ref())
    {
        let current = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .map_err(|_| "Cannot determine the local account.")?;
        let matches = user == &current
            || user
                .rsplit('\\')
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case(&current));
        if !matches {
            return Err(format!(
                "The local agent belongs to {user}; sign in as that account to uninstall."
            ));
        }
    }
    let mut request = Request {
        computer: std::env::var("COMPUTERNAME")
            .or_else(|_| capture("hostname", &[]))
            .unwrap_or_else(|_| "This computer".into()),
        platform,
        home: home.clone(),
        overrides,
        services: reports,
        protected_paths: Vec::new(),
        client_paths: Vec::new(),
        headless_roots: Vec::new(),
        installer_candidates: Vec::new(),
        findings: Vec::new(),
    };
    for definition in request
        .services
        .iter()
        .filter_map(|report| report.definition.as_ref())
    {
        if definition.log_path.is_file() {
            request.client_paths.push(definition.log_path.clone());
        }
    }
    let mut installations = inspect_installations(options, &request)?;
    match platform {
        Platform::Macos => {
            for arch in ["aarch64", "x86_64"] {
                request
                    .headless_roots
                    .push(PathBuf::from(format!("/usr/local/lib/msc2/{arch}")));
            }
            let helper = PathBuf::from("/Library/Application Support/MSC 2/bedrock-helper");
            if helper.exists() {
                request.findings.push(path_finding(
                    &helper,
                    "MSC system Bedrock helper (removed with its service)",
                ));
            }
        }
        Platform::Windows => {
            for base in [
                std::env::var_os("ProgramFiles"),
                std::env::var_os("LOCALAPPDATA"),
            ]
            .into_iter()
            .flatten()
            {
                request
                    .headless_roots
                    .push(PathBuf::from(base).join("MSC2/bin"));
            }
            request
                .client_paths
                .push(PathBuf::from(r"C:\ProgramData\MSC2\Services"));
        }
        Platform::Linux => {
            let marker = Path::new("/usr/lib/msc2/.msc2-installation-mode");
            if fs::read_to_string(marker).is_ok_and(|value| value.trim() == "standalone-archive") {
                installations.push(Installation::LinuxArchive {
                    path: PathBuf::from("/usr/lib/msc2"),
                });
            }
        }
    }
    for installation in &installations {
        let (path, identity) = match installation {
            Installation::MacBundle { path }
            | Installation::MarkedArchive { path }
            | Installation::LinuxArchive { path } => {
                (Some(path.clone()), path.display().to_string())
            }
            Installation::Deb { name } => (None, format!("Debian package {name}")),
            Installation::Rpm { name } => (None, format!("RPM package {name}")),
            Installation::Msi { product } => (None, format!("Windows MSI {product}")),
        };
        request.findings.push(Entry {
            kind: EntryKind::Installation,
            path,
            identity,
            evidence: "Verified local installation identity".into(),
            state: EntryState::Present,
            problem: None,
        });
    }
    if installations.is_empty() {
        request.findings.push(Entry { kind: EntryKind::Installation, path: None, identity: "Desktop/package installation".into(), evidence: "Local package/bundle inspection found no MSC desktop package".into(), state: if options.desktop_origin.is_some() { EntryState::Blocked } else { EntryState::Missing }, problem: options.desktop_origin.as_ref().map(|_| "This desktop is not a verified installed package. Uninstall from the installed app; developer checkouts are protected.".into()) });
    }
    // Resolve archives before treating an otherwise absent desktop as an error.
    let mut preliminary = inventory(&StdFileSystem, &request);
    for entry in &preliminary.entries {
        if entry.kind == EntryKind::Installation
            && entry.state == EntryState::Present
            && entry.evidence == "MSC headless ownership marker"
            && let Some(path) = &entry.path
        {
            installations.push(Installation::MarkedArchive { path: path.clone() });
        }
    }
    let checksums = installer_checksums(&agent_data_dir, trusted_key)?;
    let mut candidates = options.installers.clone();
    // Bounded Downloads scan, with signed release checksum verification. Never
    // crawl the home/disk or infer ownership just from MSC in a filename.
    if let Ok(entries) = fs::read_dir(home.join("Downloads")) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|kind| kind.is_file())
                && let Some(name) = entry.file_name().to_str()
                && checksums.contains_key(name)
            {
                candidates.push(entry.path());
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    for path in candidates {
        let checksum = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| checksums.get(name))
            .cloned();
        request.installer_candidates.push(InstallerCandidate {
            path,
            verified_sha256: checksum,
        });
    }
    preliminary = inventory(&StdFileSystem, &request);
    // A service binary in a dev-build cache is part of its data tree, but a
    // source-tree executable must never become an installation deletion target.
    let agent_running = agent_running(&request);
    Ok(Preview {
        inventory: preliminary,
        request,
        installations,
        agent_data_dir,
        agent_running,
    })
}

fn agent_running(request: &Request) -> bool {
    request.services.iter().any(|report| {
        report.service_name.as_str() == AGENT && report.state == ServiceState::Running
    })
}

fn path_finding(path: &Path, evidence: &str) -> Entry {
    Entry {
        kind: EntryKind::Installation,
        path: Some(path.to_path_buf()),
        identity: path.display().to_string(),
        evidence: evidence.into(),
        state: EntryState::Present,
        problem: None,
    }
}

pub fn capture(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("Could not inspect {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().into())
}

pub fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("Could not run {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} failed ({status})."))
    }
}

pub fn inspect_installations(
    options: &Options,
    request: &Request,
) -> Result<Vec<Installation>, String> {
    let mut result = Vec::new();
    #[cfg(target_os = "macos")]
    {
        let mut paths = vec![
            PathBuf::from("/Applications/MSC 2.app"),
            request.home.join("Applications/MSC 2.app"),
        ];
        for origin in [
            &options.origin,
            options.desktop_origin.as_ref().unwrap_or(&options.origin),
        ] {
            if let Some(bundle) = origin
                .ancestors()
                .find(|path| path.extension().is_some_and(|extension| extension == "app"))
            {
                paths.push(bundle.to_path_buf());
            }
        }
        paths.sort();
        paths.dedup();
        for path in paths {
            if !path.exists() {
                continue;
            }
            validate_target(&StdFileSystem, request, &path)?;
            verify_mac_bundle(&path)?;
            result.push(Installation::MacBundle { path });
        }
    }
    #[cfg(target_os = "linux")]
    {
        let mut binaries = vec![
            options.origin.clone(),
            PathBuf::from("/usr/bin/msc2-desktop-web"),
            PathBuf::from("/usr/lib/msc2/msc"),
        ];
        if let Some(origin) = &options.desktop_origin {
            binaries.push(origin.clone());
        }
        for path in binaries.into_iter().filter(|path| path.is_file()) {
            if let Ok(owner) = capture("dpkg-query", &["-S", &path.display().to_string()]) {
                if let Some((name, _)) = owner.split_once(": ")
                    && matches!(name, "msc2" | "msc2-desktop-web" | "msc-2")
                {
                    result.push(Installation::Deb { name: name.into() });
                }
            } else if let Ok(name) = capture(
                "rpm",
                &["-qf", "--qf", "%{NAME}", &path.display().to_string()],
            ) && matches!(name.as_str(), "msc2" | "msc2-desktop-web" | "msc-2")
            {
                result.push(Installation::Rpm { name });
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        // WindowsInstaller=1 and a GUID key are required. Never execute an
        // arbitrary UninstallString from the registry or use Win32_Product.
        let output = capture(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$ErrorActionPreference='Stop'; Get-ItemProperty 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*','HKLM:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*','HKLM:\\Software\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*' -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -eq 'MSC 2' -and $_.Publisher -eq 'Cameron Temple' -and $_.WindowsInstaller -eq 1 } | ForEach-Object { $_.PSChildName }",
            ],
        )?;
        for product in output
            .lines()
            .map(str::trim)
            .filter(|value| valid_msi_product(value))
        {
            result.push(Installation::Msi {
                product: product.into(),
            });
        }
        let _ = (options, request);
    }
    result.sort_by_key(|installation| format!("{installation:?}"));
    result.dedup();
    Ok(result)
}

#[cfg(target_os = "macos")]
pub fn verify_mac_bundle(path: &Path) -> Result<(), String> {
    let plist = path.join("Contents/Info.plist");
    let id = capture(
        "/usr/bin/plutil",
        &[
            "-extract",
            "CFBundleIdentifier",
            "raw",
            "-o",
            "-",
            &plist.display().to_string(),
        ],
    )?;
    if id != "com.ctemple.msc2" {
        return Err(format!(
            "Refusing unrecognized app bundle {}.",
            path.display()
        ));
    }
    Ok(())
}

pub fn valid_msi_product(value: &str) -> bool {
    value.is_ascii()
        && value.len() == 38
        && value.starts_with('{')
        && value.ends_with('}')
        && value[1..37].chars().enumerate().all(|(index, ch)| {
            if [8, 13, 18, 23].contains(&index) {
                ch == '-'
            } else {
                ch.is_ascii_hexdigit()
            }
        })
}

fn installer_checksums(
    data: &Path,
    trusted_key: Option<&str>,
) -> Result<BTreeMap<String, String>, String> {
    let mut checksums = BTreeMap::new();
    let Some(key) = trusted_key else {
        return Ok(checksums);
    };
    let bytes = decode_key(key)?;
    let target = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", _) => "x86_64-apple-darwin",
        ("windows", _) => "x86_64-pc-windows-msvc",
        _ => "x86_64-unknown-linux-gnu",
    }
    .into();
    let config = crate::release_update::UpdateClientConfig {
        repository: "ctemple9/msc2".into(),
        current_version: "0.0.0".into(),
        api_major: 1,
        api_minor: 0,
        target,
        trusted_key: bytes,
        channel: crate::release_update::UpdateChannel::Desktop,
        linux_package_format: None,
    };
    if let Ok(entries) = fs::read_dir(data.join("updates")) {
        for entry in entries.flatten() {
            let Some(id) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if let Ok(staged) = crate::release_update::verify_staged(&config, data, &id) {
                for platform in staged.manifest.platforms.values() {
                    for asset in &platform.assets {
                        if matches!(
                            asset.role.as_str(),
                            "desktop" | "package-deb" | "package-rpm"
                        ) {
                            checksums.insert(asset.filename.clone(), asset.sha256.clone());
                        }
                    }
                }
            }
        }
    }
    Ok(checksums)
}

fn decode_key(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Invalid release-signing key in this package.".into());
    }
    let mut key = [0; 32];
    for (index, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|error| error.to_string())?;
    }
    Ok(key)
}

/// Called only after local Minecraft shutdown has been verified. The caller
/// must close the desktop before removing its live WebView state.
pub fn remove(
    services: &dyn LocalServices,
    preview: Preview,
    confirmation: &str,
) -> Result<RemovalResult, String> {
    if confirmation != CONFIRMATION {
        return Err("Exact UNINSTALL MSC 2 confirmation is required.".into());
    }
    if preview.inventory.is_blocked() {
        return Err("Uninstall inventory contains blocked targets; nothing was removed.".into());
    }
    let mut result = RemovalResult {
        state: "uninstalled",
        removed: Vec::new(),
        remaining: Vec::new(),
    };
    // A failed stop/unregistration must never be followed by data deletion.
    services.remove()?;
    if services
        .inspect()?
        .iter()
        .any(|report| report.state != ServiceState::NotInstalled)
    {
        return Err("An MSC service is still registered; data was retained.".into());
    }
    services.clear_credentials()?;
    let mut paths: Vec<_> = preview
        .inventory
        .entries
        .iter()
        .filter(|entry| {
            entry.state == EntryState::Present
                && matches!(
                    entry.kind,
                    EntryKind::DataTree
                        | EntryKind::ServerTree
                        | EntryKind::Configuration
                        | EntryKind::ClientState
                        | EntryKind::Installer
                )
        })
        .filter_map(|entry| entry.path.clone())
        .collect();
    paths.sort_by_key(|path| path.components().count());
    paths.dedup();
    let mut covered: Vec<PathBuf> = Vec::new();
    for path in paths {
        if covered.iter().any(|parent| path.starts_with(parent)) {
            continue;
        }
        if path.is_dir() {
            covered.push(path.clone());
        }
        if let Some(candidate) = preview
            .request
            .installer_candidates
            .iter()
            .find(|candidate| candidate.path == path)
        {
            let bytes = fs::read(&path).map_err(|error| error.to_string())?;
            let expected = candidate
                .verified_sha256
                .as_ref()
                .ok_or("Installer verification disappeared.")?;
            if !hex(&Sha256::digest(bytes)).eq_ignore_ascii_case(expected) {
                return Err(format!(
                    "Installer changed after review: {}",
                    path.display()
                ));
            }
        }
        if let Err(error) = validate_target(&StdFileSystem, &preview.request, &path)
            .and_then(|_| services.remove_data(&path))
        {
            result
                .remaining
                .push(format!("{}: {error}", path.display()));
        } else {
            result.removed.push(path.display().to_string());
        }
    }
    // Preserve the installed command/app if any data remains, so the operator
    // has a recovery tool rather than an apparently successful partial purge.
    if result.remaining.is_empty() {
        for installation in &preview.installations {
            if let Err(error) = services.remove_installation(installation) {
                result.remaining.push(format!("{installation:?}: {error}"));
                break;
            }
            result.removed.push(format!("{installation:?}"));
        }
    }
    if !result.remaining.is_empty() {
        result.state = "partial";
    }
    Ok(result)
}

pub fn remove_path(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path).map_err(|error| error.to_string())
        }
        Ok(_) => fs::remove_file(path).map_err(|error| error.to_string()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(target_os = "windows")]
pub fn powershell_encoded(script: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    )
}
