//! Windows MSI/service removal keeps credentials in the installing user's
//! session. Only fixed service operations and verified product IDs elevate.
use crate::service::WindowsServiceManager;
use msc_infrastructure::service::{
    ServiceManager, ServiceManagerCommand, ServiceName, ServiceState, ServiceStatusReport,
};
use msc_infrastructure::uninstall::native::{self, Installation, LocalServices};
use std::path::Path;

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
            elevate(
                r"$ErrorActionPreference='Stop'; & sc.exe stop com.ctemple.msc2.agent; if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne 1062) { exit $LASTEXITCODE }; $deadline=(Get-Date).AddSeconds(45); do { $s=Get-Service -Name com.ctemple.msc2.agent -ErrorAction Stop; if ($s.Status -eq 'Stopped') { break }; Start-Sleep -Milliseconds 250 } while ((Get-Date) -lt $deadline); if ($s.Status -ne 'Stopped') { throw 'Agent did not stop' }; & sc.exe delete com.ctemple.msc2.agent; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; $p='C:\ProgramData\MSC2\Services\com.ctemple.msc2.agent.metadata'; if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Force }",
            )?;
        }
        Ok(())
    }
    fn remove_data(&self, path: &Path) -> Result<(), String> {
        if path == Path::new(r"C:\ProgramData\MSC2\Services") {
            elevate(
                r"$ErrorActionPreference='Stop'; $p='C:\ProgramData\MSC2\Services'; if (Test-Path -LiteralPath $p) { foreach ($a in @('C:\ProgramData\MSC2',$p)) { if ((Get-Item -LiteralPath $a).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Refusing reparse point' } }; if (Get-ChildItem -LiteralPath $p -Force) { throw 'Unrecognized service metadata remains' }; Remove-Item -LiteralPath $p -Force; $root='C:\ProgramData\MSC2'; if (-not (Get-ChildItem -LiteralPath $root -Force)) { Remove-Item -LiteralPath $root -Force } }",
            )
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
                    Some(0 | 3010) => Ok(()),
                    _ => Err(format!("MSI uninstall failed ({status}).")),
                }
            }
            Installation::MarkedArchive { path } => {
                if !std::fs::read_to_string(path.join(".msc2-owned"))
                    .is_ok_and(|value| value.trim() == "msc2-headless-archive")
                {
                    return Err("Headless ownership marker changed.".into());
                }
                if native::remove_path(path).is_ok() {
                    return Ok(());
                }
                let program_files =
                    std::env::var_os("ProgramFiles").ok_or("ProgramFiles is unavailable.")?;
                if path != &std::path::PathBuf::from(program_files).join("MSC2/bin") {
                    return Err("Refusing an unknown elevated archive path.".into());
                }
                let literal = path.display().to_string().replace('\'', "''");
                elevate(&format!(
                    "$ErrorActionPreference='Stop'; $p='{literal}'; if ((Get-Item -LiteralPath $p).Attributes -band [IO.FileAttributes]::ReparsePoint) {{ throw 'Refusing reparse point' }}; if ((Get-Content -LiteralPath (Join-Path $p '.msc2-owned') -Raw).Trim() -ne 'msc2-headless-archive') {{ throw 'Ownership changed' }}; Remove-Item -LiteralPath $p -Recurse -Force; $old=[Environment]::GetEnvironmentVariable('Path','Machine'); $new=($old -split ';' | Where-Object {{ $_.TrimEnd('\\') -ine $p.TrimEnd('\\') }}) -join ';'; [Environment]::SetEnvironmentVariable('Path',$new,'Machine')"
                ))
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
        "$ErrorActionPreference='Stop'; $p=Start-Process -FilePath powershell.exe -Verb RunAs -Wait -PassThru -ArgumentList @('-NoProfile','-NonInteractive','-EncodedCommand','{encoded}'); exit $p.ExitCode"
    );
    native::run(
        "powershell.exe",
        &["-NoProfile", "-NonInteractive", "-Command", &launcher],
    )
}
