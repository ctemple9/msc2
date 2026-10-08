//! Windows MSI/service removal keeps credentials in the installing user's
//! session. Only fixed service operations and verified product IDs elevate.
use crate::service::WindowsServiceManager;
use msc_infrastructure::service::{
    ServiceManager, ServiceManagerCommand, ServiceName, ServiceState, ServiceStatusReport,
};
use msc_infrastructure::uninstall::native::{self, Installation, LocalServices};
use std::path::Path;

/// Launch the fixed continuation without inheriting the scheduling command's
/// output pipes. Otherwise the desktop waits for EOF while this worker waits
/// for the desktop to exit, even when its own standard streams are null.
pub fn launch_uninstall_worker(worker: &Path, job: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        CREATE_NO_WINDOW, CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW,
    };

    if !worker.is_absolute()
        || worker
            .file_name()
            .is_none_or(|name| name != "msc-uninstall-worker.exe")
        || job.parent() != worker.parent()
        || job.file_name().is_none_or(|name| name != "job.json")
    {
        return Err("Invalid uninstall continuation paths.".into());
    }
    let application: Vec<u16> = worker.as_os_str().encode_wide().chain([0]).collect();
    let mut arguments = std::ffi::OsString::from("\"");
    arguments.push(worker);
    arguments.push("\" uninstall --danger --confirm \"UNINSTALL MSC 2\" --apply \"");
    arguments.push(job);
    arguments.push("\"");
    let mut arguments: Vec<u16> = arguments.encode_wide().chain([0]).collect();
    if application[..application.len() - 1].contains(&0)
        || arguments[..arguments.len() - 1].contains(&0)
    {
        return Err("Invalid uninstall continuation path encoding.".into());
    }
    let startup = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut process = PROCESS_INFORMATION::default();
    // SAFETY: Both UTF-16 buffers are terminated and live through the call;
    // arguments is writable. All optional pointers are null. Inheritance is
    // explicitly disabled; no parent pipes or file handles reach the worker.
    let created = unsafe {
        CreateProcessW(
            application.as_ptr(),
            arguments.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_NO_WINDOW,
            std::ptr::null(),
            std::ptr::null(),
            &startup,
            &mut process,
        )
    };
    if created == 0 {
        return Err(format!(
            "Could not launch uninstall worker: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: CreateProcessW succeeded and owns these two handles. Closing our
    // copies does not terminate the independent continuation process.
    unsafe {
        CloseHandle(process.hThread);
        CloseHandle(process.hProcess);
    }
    Ok(())
}

pub struct WindowsUninstall;
impl LocalServices for WindowsUninstall {
    fn inspect(&self) -> Result<Vec<ServiceStatusReport>, String> {
        let report = WindowsServiceManager::new()
            .execute(ServiceManagerCommand::Status {
                service_name: ServiceName::new("com.ctemple.msc2.agent"),
            })
            .map_err(|error| error.to_string())?;
        if report.state == ServiceState::NotInstalled
            && native::capture("sc.exe", &["qc", "com.ctemple.msc2.agent"]).is_ok()
        {
            return Err("An MSC service exists without readable installation metadata; retain it for repair/inspection.".into());
        }
        Ok(vec![report])
    }
    fn remove(&self) -> Result<(), String> {
        if self
            .inspect()?
            .iter()
            .any(|report| report.state != ServiceState::NotInstalled)
        {
            elevate(include_str!(
                "../../../packaging/windows/full-uninstall-service.ps1"
            ))?;
        }
        Ok(())
    }
    fn remove_data(&self, path: &Path) -> Result<(), String> {
        if path == Path::new(r"C:\ProgramData\MSC2\Services") {
            elevate(include_str!(
                "../../../packaging/windows/full-uninstall-cache.ps1"
            ))
        } else {
            native::remove_path(path)
        }
    }
    fn remove_installation(&self, installation: &Installation) -> Result<(), String> {
        match installation {
            Installation::Msi { product } if native::valid_msi_product(product) => {
                // MSI performs UAC/registration cleanup; never execute the
                // registry's arbitrary UninstallString or delete MSI caches.
                let status = std::process::Command::new("msiexec.exe")
                    .args(["/x", product, "/passive", "/norestart"])
                    .status()
                    .map_err(|error| error.to_string())?;
                match status.code() {
                    Some(0) => Ok(()),
                    Some(3010) => Err("Windows Installer requires a restart to finish package removal. Save your work and restart Windows; removal is not yet confirmed complete.".into()),
                    _ => Err(format!("MSI uninstall failed ({status}).")),
                }
            }
            Installation::MarkedArchive { path } => {
                if !std::fs::read_to_string(path.join(".msc2-owned"))
                    .is_ok_and(|value| value.trim() == "msc2-headless-archive")
                {
                    return Err("Headless ownership marker changed.".into());
                }
                let program_files = std::env::var_os("ProgramFiles")
                    .map(std::path::PathBuf::from)
                    .map(|root| root.join("MSC2/bin"));
                let local = std::env::var_os("LOCALAPPDATA")
                    .map(std::path::PathBuf::from)
                    .map(|root| root.join("MSC2/bin"));
                let machine = program_files.as_ref() == Some(path);
                if !machine && local.as_ref() != Some(path) {
                    return Err("Refusing an unknown archive path.".into());
                }
                let literal = path.display().to_string().replace('\'', "''");
                let scope = if machine { "Machine" } else { "User" };
                let script = format!(
                    "$ErrorActionPreference='Stop'; $p='{literal}'; if ((Get-Item -LiteralPath $p).Attributes -band [IO.FileAttributes]::ReparsePoint) {{ throw 'Refusing reparse point' }}; if ((Get-Content -LiteralPath (Join-Path $p '.msc2-owned') -Raw).Trim() -ne 'msc2-headless-archive') {{ throw 'Ownership changed' }}; Remove-Item -LiteralPath $p -Recurse -Force; $old=[Environment]::GetEnvironmentVariable('Path','{scope}'); $new=($old -split ';' | Where-Object {{ $_.TrimEnd([char]92) -ine $p.TrimEnd([char]92) }}) -join ';'; [Environment]::SetEnvironmentVariable('Path',$new,'{scope}')"
                );
                if machine {
                    elevate(&script)
                } else {
                    let encoded = native::powershell_encoded(&script);
                    native::run(
                        "powershell.exe",
                        &["-NoProfile", "-NonInteractive", "-EncodedCommand", &encoded],
                    )
                }
            }
            _ => Err("Unsupported Windows installation identity.".into()),
        }
    }
    fn clear_credentials(&self) -> Result<(), String> {
        crate::secret_store::remove_all_msc_credentials().map_err(|error| error.to_string())
    }
}
fn elevate(script: &str) -> Result<(), String> {
    // Literal-quote the fixed native script when requesting a separate UAC
    // process. Paths are checked before being inserted by the archive branch.
    // Base64 encoding is supplied by infrastructure without another package.
    let encoded = msc_infrastructure::uninstall::native::powershell_encoded(script);
    let launcher = format!(
        "$ErrorActionPreference='Stop'; $p=Start-Process -FilePath powershell.exe -WindowStyle Hidden -Verb RunAs -Wait -PassThru -ArgumentList @('-NoProfile','-NonInteractive','-EncodedCommand','{encoded}'); exit $p.ExitCode"
    );
    native::run(
        "powershell.exe",
        &["-NoProfile", "-NonInteractive", "-Command", &launcher],
    )
}
