//! Windows updates keep the signed old MSI and SCM recovery material until
//! the replacement desktop and owned local agent have acknowledged readiness.
use super::{client_config, release_update, StagedUpdate};
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::windows::{
        fs::{MetadataExt, OpenOptionsExt},
        process::CommandExt,
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const UPGRADE_CODE: &str = "816be706-5775-5a3a-917c-339fbf0976e9";
const COORDINATOR: &str =
    include_str!("../../../../packaging/windows/desktop-service-lifecycle.ps1");

struct MsiHandle(u32);
struct UpdateLock(isize);
#[link(name = "kernel32")]
extern "system" {
    fn CreateMutexW(security: *const std::ffi::c_void, owner: i32, name: *const u16) -> isize;
    fn WaitForSingleObject(handle: isize, timeout: u32) -> u32;
    fn ReleaseMutex(handle: isize) -> i32;
    fn CloseHandle(handle: isize) -> i32;
}
impl UpdateLock {
    fn acquire() -> Result<Self, String> {
        // SAFETY: the name is fixed and terminated. We retain and release
        // ownership only if Windows reports acquisition of this mutex.
        unsafe {
            let handle = CreateMutexW(
                std::ptr::null(),
                0,
                wide("Global\\MSC2.DesktopUpdate").as_ptr(),
            );
            if handle == 0 {
                return Err("Another session owns the Windows update coordinator.".into());
            }
            if !matches!(WaitForSingleObject(handle, 0), 0 | 128) {
                CloseHandle(handle);
                return Err("Another MSC Windows update is already running; no installer or recovery action was started.".into());
            }
            Ok(Self(handle))
        }
    }
}
impl Drop for UpdateLock {
    fn drop(&mut self) {
        // SAFETY: this guard owns exactly the mutex handle acquired above.
        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
    }
}
impl Drop for MsiHandle {
    fn drop(&mut self) {
        unsafe {
            MsiCloseHandle(self.0);
        }
    }
}
#[link(name = "msi")]
extern "system" {
    fn MsiOpenDatabaseW(path: *const u16, mode: *const u16, handle: *mut u32) -> u32;
    fn MsiDatabaseOpenViewW(db: u32, query: *const u16, view: *mut u32) -> u32;
    fn MsiViewExecute(view: u32, record: u32) -> u32;
    fn MsiViewFetch(view: u32, record: *mut u32) -> u32;
    fn MsiRecordGetStringW(record: u32, field: u32, value: *mut u16, size: *mut u32) -> u32;
    fn MsiGetProductInfoW(
        product: *const u16,
        property: *const u16,
        value: *mut u16,
        size: *mut u32,
    ) -> u32;
    fn MsiCloseHandle(handle: u32) -> u32;
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn msi_value(path: &Path, query: &str) -> Result<String, String> {
    let mut db = 0;
    // SAFETY: UTF-16 strings and out handles remain valid for these synchronous
    // read-only MSI calls. Every acquired handle is closed by its guard.
    unsafe {
        if MsiOpenDatabaseW(
            wide(&path.to_string_lossy()).as_ptr(),
            std::ptr::null(),
            &mut db,
        ) != 0
        {
            return Err("Could not inspect the verified MSI.".into());
        }
        let db = MsiHandle(db);
        let mut view = 0;
        if MsiDatabaseOpenViewW(db.0, wide(query).as_ptr(), &mut view) != 0 {
            return Err("The MSI maintenance contract is missing.".into());
        }
        let view = MsiHandle(view);
        if MsiViewExecute(view.0, 0) != 0 {
            return Err("Could not inspect MSI properties.".into());
        }
        let mut record = 0;
        if MsiViewFetch(view.0, &mut record) != 0 {
            return Err("The verified MSI lacks required update metadata.".into());
        }
        let record = MsiHandle(record);
        let mut value = vec![0u16; 32768];
        let mut size = 32767;
        if MsiRecordGetStringW(record.0, 1, value.as_mut_ptr(), &mut size) != 0 {
            return Err("MSI property exceeds its bound.".into());
        }
        String::from_utf16(&value[..size as usize]).map_err(|_| "Invalid MSI text.".into())
    }
}
fn property(path: &Path, name: &str) -> Result<String, String> {
    msi_value(
        path,
        &format!("SELECT `Value` FROM `Property` WHERE `Property`='{name}'"),
    )
}
fn system(name: &str) -> Result<PathBuf, String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
    }
    let mut buffer = vec![0u16; 32768];
    // SAFETY: Windows writes within the supplied UTF-16 buffer capacity.
    let size = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    if size == 0 || size >= buffer.len() {
        return Err("Windows system directory unavailable.".into());
    }
    Ok(
        PathBuf::from(
            String::from_utf16(&buffer[..size]).map_err(|_| "Invalid system directory.")?,
        )
        .join(name),
    )
}
fn powershell(script: &str) -> Result<std::process::Output, String> {
    let encoded = msc_infrastructure::uninstall::native::powershell_encoded(script);
    let mut child = Command::new(system("WindowsPowerShell/v1.0/powershell.exe")?)
        .args(["-NoProfile", "-NonInteractive", "-Command", "-"])
        .creation_flags(0x08000000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "Could not start the local update coordinator.")?;
    child.stdin.take().ok_or("Coordinator input unavailable.")?
        .write_all(format!("Invoke-Expression ([Text.Encoding]::Unicode.GetString([Convert]::FromBase64String('{encoded}')))\n").as_bytes())
        .map_err(|_| "Could not send the update request.")?;
    child
        .wait_with_output()
        .map_err(|_| "Could not wait for local update coordination.".into())
}
fn literal(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}
fn trusted_runner(executable: &Path) -> Result<(), String> {
    let script = format!(
        r#"$ErrorActionPreference='Stop'; try {{
        $trusted=(New-Object Security.Principal.NTAccount('NT SERVICE\TrustedInstaller')).Translate([Security.Principal.SecurityIdentifier]).Value;
        $allowed=@('S-1-5-18','S-1-5-32-544',$trusted);
        $p='{}'; $first=$true;
        while ($p) {{
            $acl=Get-Acl -LiteralPath $p;
            if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -notin $allowed) {{ throw 'Unprotected coordinator owner' }};
            $write=[Security.AccessControl.FileSystemRights]::Delete -bor [Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles -bor [Security.AccessControl.FileSystemRights]::ChangePermissions -bor [Security.AccessControl.FileSystemRights]::TakeOwnership;
            if ($first) {{ $write=$write -bor [Security.AccessControl.FileSystemRights]::Write }};
            foreach ($r in $acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])) {{
                if ($r.AccessControlType -eq 'Allow' -and $r.IdentityReference.Value -notin $allowed -and ($r.FileSystemRights -band $write)) {{ throw 'Unprotected coordinator permissions' }}
            }};
            $first=$false; $p=Split-Path -Parent $p
        }}; exit 0
    }} catch {{ exit 1 }}"#,
        literal(executable)
    );
    if !powershell(&script)?.status.success() {
        return Err("Windows coordinated updates require a machine-protected desktop executable and directory. Repair/reinstall into a protected machine directory before updating.".into());
    }
    Ok(())
}
fn coordinator(
    operation: &str,
    product: &str,
    executable: Option<&Path>,
) -> Result<String, String> {
    let mut request = serde_json::json!({"operation":operation,"transaction":product});
    let mut hashes = serde_json::Map::new();
    if let Some(executable) = executable {
        let root = executable
            .parent()
            .ok_or("The installed desktop has no directory.")?;
        request["previousRoot"] = root.to_string_lossy().into_owned().into();
        request["installed"] = true.into();
        for name in ["msc.exe", "vantage.exe", "bedrock-map.exe"] {
            hashes.insert(
                name.into(),
                hash_file(&root.join("agent").join(name))?.into(),
            );
        }
    }
    let request = request.to_string().replace('\'', "''");
    let hashes = serde_json::Value::Object(hashes)
        .to_string()
        .replace('\'', "''");
    let output = powershell(&format!("{COORDINATOR}\ntry {{ Invoke-MscDesktopLifecycle ('{request}' | ConvertFrom-Json) ('{hashes}' | ConvertFrom-Json); exit 0 }} catch {{ [Console]::Error.WriteLine($_.Exception.Message); exit 1 }}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr)
            .trim()
            .chars()
            .take(1600)
            .collect());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
#[derive(Deserialize)]
struct Agent {
    owned: bool,
    running: bool,
    present: bool,
    pid: Option<u32>,
    port: Option<u16>,
    #[serde(rename = "oldDesktopHash")]
    old_desktop_hash: Option<String>,
}
fn probe(executable: &Path) -> Result<Agent, String> {
    serde_json::from_str(&coordinator(
        "probe",
        "{00000000-0000-0000-0000-000000000000}",
        Some(executable),
    )?)
    .map_err(|_| "Could not read the local service identity.".into())
}
fn hash_file(path: &Path) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(fs::read(path).map_err(|_| "Could not read retained installation bytes.")?)
    ))
}

/// Hold files and their ancestor directories against writes/renames while
/// Windows Installer consumes the exact bytes that passed signature checks.
fn pin(path: &Path) -> Result<Vec<File>, String> {
    let mut handles = Vec::new();
    let mut ancestors: Vec<_> = path.ancestors().collect();
    ancestors.reverse();
    for ancestor in ancestors {
        let metadata =
            fs::symlink_metadata(ancestor).map_err(|_| "Update path cannot be inspected.")?;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("Redirected update paths are not eligible.".into());
        }
        let mut options = OpenOptions::new();
        options
            .read(true)
            .share_mode(if metadata.is_dir() { 3 } else { 1 });
        if metadata.is_dir() {
            options.custom_flags(0x02200000);
        }
        let handle = options
            .open(ancestor)
            .map_err(|_| "Could not pin verified update paths.")?;
        if handle
            .metadata()
            .map_err(|_| "Pinned path cannot be inspected.")?
            .file_attributes()
            & 0x400
            != 0
        {
            return Err("Redirected update paths are not eligible.".into());
        }
        handles.push(handle);
    }
    Ok(handles)
}

pub(super) fn wait_parent(pid: u32) -> Result<(), String> {
    let script=format!("$ErrorActionPreference='Stop'; $p=Get-Process -Id {pid} -ErrorAction SilentlyContinue; if ($p -and -not $p.WaitForExit(60000)) {{ exit 1 }}; exit 0");
    if !powershell(&script)?.status.success() {
        return Err("The desktop did not close within one minute; no installer was run.".into());
    }
    Ok(())
}
pub(super) fn notify(detail: &str) {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(owner: isize, text: *const u16, title: *const u16, flags: u32) -> i32;
    }
    // SAFETY: both terminated UTF-16 buffers remain alive until the modal
    // result dialog returns; it runs in the original desktop user's session.
    unsafe {
        MessageBoxW(
            0,
            wide(detail).as_ptr(),
            wide("MSC 2 update").as_ptr(),
            0x10030,
        );
    }
}
fn packages(
    release: &str,
    data: &Path,
    executable: &Path,
    resume: bool,
) -> Result<(StagedUpdate, StagedUpdate, Vec<File>), String> {
    let config = client_config()?;
    let previous = release_update::verify_staged(&config, data, env!("CARGO_PKG_VERSION"))
        .map_err(|_| "A signed, verified MSI for the installed version must be retained in update staging before Windows can update safely.".to_string())?;
    let target = release_update::verify_staged(&config, data, release)?;
    let mut handles = pin(&previous.artifact_path)?;
    handles.extend(pin(&target.artifact_path)?);
    let previous = release_update::verify_staged(&config, data, env!("CARGO_PKG_VERSION"))?;
    let target = release_update::verify_staged(&config, data, release)?;
    for package in [&previous, &target] {
        if property(&package.artifact_path,"MSC_UPDATE_PROTOCOL")? != "1" { return Err("Both Windows MSIs must support the coordinated recovery protocol. Upgrade legacy installations manually to a protocol-capable signed release first.".into()); }
        if property(&package.artifact_path, "UpgradeCode")?
            .trim_matches(['{', '}'])
            .to_ascii_lowercase()
            != UPGRADE_CODE
            || property(&package.artifact_path, "ProductVersion")? != package.manifest.release_id
        {
            return Err("The signed MSI does not match the MSC desktop product/version.".into());
        }
        msi_value(
            &package.artifact_path,
            "SELECT `Target` FROM `CustomAction` WHERE `Action`='MscApplyService'",
        )?;
    }
    let numeric = |value: &str| -> Result<Vec<u32>, String> {
        value
            .split('.')
            .map(|part| {
                part.parse()
                    .map_err(|_| "Windows MSI versions must be numeric.".to_string())
            })
            .collect()
    };
    if numeric(&target.manifest.release_id)? <= numeric(&previous.manifest.release_id)? {
        return Err("Ordinary updates require a newer MSI. Older packages use deliberate verified recovery.".into());
    }
    let secure = property(&target.artifact_path, "SecureCustomProperties")?;
    if property(&target.artifact_path, "MSC_UPDATE_PROTOCOL")? != "1" {
        return Err("The target MSI's update recovery protocol is unsupported.".into());
    }
    if !secure.split(';').any(|p| p == "MSC_UPDATE_MANAGED") {
        return Err("This target MSI does not support coordinated health recovery.".into());
    }
    let registered = if resume { &target } else { &previous };
    let product = property(&registered.artifact_path, "ProductCode")?;
    let mut value = vec![0u16; 32768];
    let mut size = 32767;
    // SAFETY: the product comes from the verified MSI and all buffers are bounded.
    let code = unsafe {
        MsiGetProductInfoW(
            wide(&product).as_ptr(),
            wide("InstallLocation").as_ptr(),
            value.as_mut_ptr(),
            &mut size,
        )
    };
    if code != 0 || size as usize >= value.len() {
        return Err("The retained signed MSI is not the registered desktop installation.".into());
    }
    let location = String::from_utf16(&value[..size as usize])
        .map_err(|_| "Invalid registered install directory.")?;
    if code != 0
        || fs::canonicalize(location).ok().as_deref()
            != executable
                .parent()
                .and_then(|p| fs::canonicalize(p).ok())
                .as_deref()
    {
        return Err(
            "The retained signed MSI is not the registered installation running this desktop."
                .into(),
        );
    }
    Ok((previous, target, handles))
}
pub(super) fn schedule(release: &str, data: &Path) -> Result<(), String> {
    let _lock = UpdateLock::acquire()?;
    let executable = std::env::current_exe().map_err(|_| "Installed desktop unavailable.")?;
    release_update::retain_installed_release(&client_config()?, data)?;
    let _ = packages(release, data, &executable, false)?;
    let _pins = pin(&executable)?;
    trusted_runner(&executable)?;
    let before = probe(&executable)?;
    if !agent_health(&before, &executable, env!("CARGO_PKG_VERSION")) {
        return Err("The existing owned agent must pass authenticated health before its usable installation can be retained.".into());
    }
    let mut nonce = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut nonce);
    let work = data
        .join("updates")
        .join(format!("windows-worker-{:x}", Sha256::digest(nonce)));
    fs::create_dir(&work).map_err(|_| "Could not create update continuation directory.")?;
    let helper = work.join("msc2-update-worker.exe");
    fs::copy(&executable, &helper).map_err(|_| "Could not retain the update continuation.")?;
    if hash_file(&helper)? != hash_file(&executable)? {
        return Err("The copied update continuation changed.".into());
    }
    Command::new(&helper)
        .arg("--msc2-apply-desktop-update")
        .args(["--release-id", release, "--parent-pid"])
        .arg(std::process::id().to_string())
        .arg("--data-dir")
        .arg(data)
        .arg("--installed-executable")
        .arg(executable)
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|_| "Could not schedule the retained update worker.")?;
    Ok(())
}
fn run_msi(package: &StagedUpdate, root: &Path, log: &Path, recovery: bool) -> Result<i32, String> {
    let mut command = Command::new(system("msiexec.exe")?);
    command
        .arg("/i")
        .arg(&package.artifact_path)
        .args([
            "/passive",
            "/norestart",
            "REBOOT=ReallySuppress",
            "MSC_UPDATER_OWNS_RELAUNCH=1",
            "MSC_LAUNCH_APP=0",
        ])
        .arg(format!("INSTALLDIR={}", root.display()))
        .arg("/l*v")
        .arg(log);
    if recovery {
        command.arg("MSC_RESTORE_PREVIOUS=1");
    } else {
        command.arg("MSC_UPDATE_MANAGED=1");
    }
    command
        .status()
        .map_err(|_| "Could not run the verified Windows installer.".to_string())?
        .code()
        .ok_or_else(|| "Windows Installer returned no result code.".into())
}
fn elevate(operation: &str, product: &str, executable: &Path) -> Result<(), String> {
    if !msc_infrastructure::uninstall::native::valid_msi_product(product) {
        return Err("Invalid service recovery identity.".into());
    }
    // The user-owned retained continuation is never elevated. Only the
    // machine-protected installed executable runs fixed recovery operations.
    let helper = executable.to_owned();
    let _pins = pin(&helper)?;
    trusted_runner(&helper)?;
    let script=format!("$ErrorActionPreference='Stop'; $p=Start-Process -FilePath '{}' -WindowStyle Hidden -Verb RunAs -Wait -PassThru -ArgumentList @('--msc2-coordinate-update-service','{operation}','{product}'); exit $p.ExitCode",literal(&helper));
    if !powershell(&script)?.status.success() {
        return Err("Windows service recovery/acceptance was refused or failed. Its protected recovery record was retained; do not discard it.".into());
    }
    Ok(())
}
pub(super) fn run_service_helper() -> bool {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some("--msc2-coordinate-update-service") {
        return false;
    }
    let result = (|| {
        let operation = args.get(2).ok_or("Missing service recovery operation.")?;
        let product = args.get(3).ok_or("Missing service recovery identity.")?;
        if !matches!(operation.as_str(), "recover" | "finalize" | "resume-health")
            || !msc_infrastructure::uninstall::native::valid_msi_product(product)
        {
            return Err("Invalid fixed recovery request.".to_string());
        }
        coordinator(operation, product, None).map(|_| ())
    })();
    std::process::exit(if result.is_ok() { 0 } else { 1 });
}
fn launch(executable: &Path, marker: &Path, token: &str) -> Result<Child, String> {
    Command::new(executable)
        .arg("--msc2-update-health-file")
        .arg(marker)
        .args(["--msc2-update-health-token", token])
        .spawn()
        .map_err(|_| "Could not relaunch the installed desktop.".into())
}
fn agent_health(before: &Agent, executable: &Path, version: &str) -> bool {
    if !before.owned {
        return true;
    }
    let Ok(after) = probe(executable) else {
        return false;
    };
    if !after.owned || !after.present || after.running != before.running {
        return false;
    }
    if !before.running {
        return true;
    }
    let (Some(pid), Some(port)) = (after.pid, after.port) else {
        return false;
    };
    let script=format!("$ErrorActionPreference='Stop'; $c=Get-NetTCPConnection -State Listen -LocalAddress 127.0.0.1 -LocalPort {port}; if (@($c).Count -ne 1 -or $c.OwningProcess -ne {pid}) {{ exit 1 }}; exit 0");
    if !powershell(&script).is_ok_and(|o| o.status.success()) {
        return false;
    }
    let Ok(store) = crate::desktop_secret_store() else {
        return false;
    };
    let Ok(Some(host)) = store.get(crate::LOCAL_HOST_ID_KEY) else {
        return false;
    };
    let Ok(Some(raw)) = store.get(&crate::credential_key(&host)) else {
        return false;
    };
    let Ok(record) = serde_json::from_str::<crate::StoredDesktopCredential>(&raw) else {
        return false;
    };
    // The saved local-host credential is host-scoped. SCM owns this loopback
    // listener before the token is sent; no selected remote route is consulted.
    tauri::async_runtime::block_on(async {
        let Ok(client) = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(2))
            .build()
        else {
            return false;
        };
        let base = format!("http://127.0.0.1:{port}");
        let Ok(me) = client
            .get(format!("{base}/v1/me"))
            .bearer_auth(&record.token)
            .send()
            .await
        else {
            return false;
        };
        if !me.status().is_success() {
            return false;
        }
        let Ok(mut response) = client
            .get(format!("{base}/v1/capabilities"))
            .bearer_auth(&record.token)
            .send()
            .await
        else {
            return false;
        };
        if !response.status().is_success() {
            return false;
        }
        let mut bytes = Vec::new();
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) if bytes.len() + chunk.len() <= 65536 => {
                    bytes.extend_from_slice(&chunk)
                }
                Ok(None) => break,
                _ => return false,
            }
        }
        serde_json::from_slice::<serde_json::Value>(&bytes).is_ok_and(|v| {
            v["agentVersion"].as_str() == Some(version) && v["apiMajor"].as_u64() == Some(1)
        })
    })
}
fn healthy(
    child: &mut Child,
    marker: &Path,
    token: &str,
    before: &Agent,
    executable: &Path,
    version: &str,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(90);
    while Instant::now() < deadline {
        if child
            .try_wait()
            .map_err(|_| "Could not inspect relaunched desktop.")?
            .is_some()
        {
            return Err("The replacement desktop exited before readiness.".into());
        }
        if fs::read_to_string(marker).is_ok_and(|v| v == token)
            && agent_health(before, executable, version)
        {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err("Desktop readiness and the owned local agent's authenticated/version-checked health did not both pass within 90 seconds.".into())
}
fn save_result(data: &Path, state: &str, release: &str, detail: &str) -> Result<(), String> {
    let path = data.join("updates/windows-last-result.json");
    let temporary = path.with_extension("incoming");
    fs::write(&temporary,serde_json::json!({"state":state,"releaseId":release,"previousRelease":env!("CARGO_PKG_VERSION"),"detail":detail}).to_string())
        .map_err(|_| "Could not retain the Windows update result.")?;
    // Windows cannot rename over an existing file. Keep the old report until
    // the new report has been fully written.
    if path.exists() {
        fs::remove_file(&path).map_err(|_| "Could not replace the last update report.")?;
    }
    fs::rename(temporary, path).map_err(|_| "Could not retain the Windows update result.".into())
}
pub(super) fn apply(
    release: &str,
    data: &Path,
    executable: &Path,
    resume: bool,
) -> Result<String, String> {
    let _lock = UpdateLock::acquire()?;
    let (previous, target, _pins) = match packages(release, data, executable, resume) {
        Ok(value) => value,
        Err(error) => {
            let detail=format!("Update re-verification failed before installer launch: {error}. Previous app relaunch was requested; no installer was run.");
            let _ = save_result(data, "verification-failed", release, &detail);
            if !resume {
                let _ = Command::new(executable).spawn();
            }
            return Err(detail);
        }
    };
    let product = property(&target.artifact_path, "ProductCode")?;
    let before = if resume {
        serde_json::from_str::<Agent>(&coordinator("probe-recovery", &product, None)?)
            .map_err(|_| "Protected recovery state is invalid.")?
    } else {
        probe(executable)?
    };
    let root = executable
        .parent()
        .ok_or("Installed desktop directory unavailable.")?;
    let old_desktop = if resume {
        before
            .old_desktop_hash
            .clone()
            .ok_or("The protected prior desktop hash is missing.")?
    } else {
        hash_file(executable)?
    };
    let work = std::env::current_exe()
        .map_err(|_| "Update worker unavailable.")?
        .parent()
        .ok_or("Update work directory unavailable.")?
        .to_owned();
    let marker = work.join("desktop-ready");
    let mut nonce = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce);
    let token = format!("{:x}", Sha256::digest(nonce));
    save_result(
        data,
        "installing",
        release,
        "Update continuation started; success has not been established.",
    )?;
    let code = if resume {
        elevate("resume-health", &product, executable)?;
        0
    } else {
        run_msi(&target, root, &work.join("install.log"), false)?
    };
    if code == 1618 {
        let detail="Windows Installer is busy with another installation. MSC did not attempt service or package recovery against that transaction. Retry after it completes.";
        save_result(data, "installer-busy", release, detail)?;
        return Err(detail.into());
    }
    if matches!(code, 3010 | 1641) {
        let detail=format!("Windows installed {release}; restart status {code}. MSC requested no automatic reboot and has not claimed healthy completion. Signed previous MSI and service recovery material remain retained. After restarting and closing MSC, resume with: & '{}' --msc2-resume-desktop-update --release-id {release} --data-dir '{}' --installed-executable '{}'", literal(&work.join("msc2-update-worker.exe")), literal(data), literal(executable));
        save_result(data, "reboot-required", release, &detail)?;
        notify(&detail);
        return Ok(detail);
    }
    let mut child = None;
    let outcome = if code == 0 {
        match launch(executable, &marker, &token) {
            Ok(mut process) => {
                let result = healthy(&mut process, &marker, &token, &before, executable, release);
                child = Some(process);
                result
            }
            Err(error) => Err(error),
        }
    } else {
        Err(format!(
            "Windows Installer returned {code}{}.",
            if code == 1602 { " (cancelled)" } else { "" }
        ))
    };
    if outcome.is_ok() {
        if let Err(error) = elevate("finalize", &product, executable) {
            let detail=format!("Replacement health passed, but acceptance remains pending: {error}. Close MSC and resume its retained worker to retry acceptance.");
            save_result(data, "acceptance-pending", release, &detail)?;
            return Err(detail);
        }
        let detail=format!("MSC {release} passed desktop readiness and the required local-agent checks. Previous signed package and copied payloads remain available for deliberate restoration.");
        save_result(data, "succeeded", release, &detail)?;
        fs::write(marker.with_extension("accepted"), b"accepted")
            .map_err(|_| "Could not release the desktop health trial.")?;
        return Ok(detail);
    }
    let failure = outcome.unwrap_err();
    if let Some(mut child) = child {
        // Only our newly launched desktop is terminated; service recovery uses
        // the graceful protocol and never kills Minecraft or an arbitrary PID.
        if child
            .try_wait()
            .map_err(|_| "Could not inspect failed desktop.")?
            .is_none()
        {
            child
                .kill()
                .map_err(|_| "Could not close the failed replacement desktop.")?;
            child
                .wait()
                .map_err(|_| "Could not wait for the failed desktop to close.")?;
        }
    }
    let recovery = (|| {
        elevate("recover", &product, executable)?;
        if code == 0 || hash_file(executable).ok().as_deref() != Some(&old_desktop) {
            let restored = run_msi(&previous, root, &work.join("recovery.log"), true)?;
            if restored != 0 {
                return Err(format!("Previous MSI restoration returned {restored}; recovery is incomplete or needs restart."));
            }
        }
        if hash_file(executable)? != old_desktop {
            return Err(
                "Restored desktop bytes differ from the retained previous installation.".into(),
            );
        }
        let old_marker = work.join("previous-ready");
        let mut previous_child = launch(executable, &old_marker, &token)?;
        healthy(
            &mut previous_child,
            &old_marker,
            &token,
            &before,
            executable,
            &previous.manifest.release_id,
        )?;
        fs::write(old_marker.with_extension("accepted"), b"accepted")
            .map_err(|_| "Could not release the recovered desktop health trial.".to_string())
    })();
    match recovery {
        Ok(()) => {
            let state = if code == 1602 {
                "cancelled"
            } else {
                "recovered"
            };
            let detail=format!("{failure} Previous MSC {} was restored/relaunched and passed health. Logs are in {}.",previous.manifest.release_id,work.display());
            save_result(data, state, release, &detail)?;
            Err(detail)
        }
        Err(error) => {
            let detail=format!("{failure} Recovery failed: {error}. Retained signed MSI, service snapshot and logs in {} require inspection; data was not reset.",work.display());
            save_result(data, "recovery-failed", release, &detail)?;
            Err(detail)
        }
    }
}
