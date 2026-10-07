//! Fixed desktop service operations; passwords stay inside the elevated helper.

use msc_infrastructure::service::ServiceInstallRequest;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};

pub fn install(request: &ServiceInstallRequest, owner_window: isize) -> Result<(), String> {
    use msc_infrastructure::service::{
        ServiceManager, ServiceManagerCommand, ServiceName, ServiceState,
    };
    let existing = msc_platform_windows::service::WindowsServiceManager::new()
        .execute(ServiceManagerCommand::Status {
            service_name: ServiceName::new(super::AGENT_SERVICE_NAME),
        })
        .map_err(|error| error.to_string())?;
    if existing.state != ServiceState::NotInstalled {
        return Err("A local agent service already exists. Use the MSC installer to replace its package without changing the service account or asking for its password again.".into());
    }
    let account = request
        .run_user
        .as_deref()
        .ok_or("The Windows agent service needs the installing account.")?;
    let mut args = vec![
        "service".into(),
        "install".into(),
        "--service-name".into(),
        super::AGENT_SERVICE_NAME.into(),
        "--binary-path".into(),
        request.binary_path.to_string_lossy().into_owned(),
        "--working-directory".into(),
        request.working_directory.to_string_lossy().into_owned(),
        "--log-path".into(),
        request.log_path.to_string_lossy().into_owned(),
        "--run-user".into(),
        account.into(),
        "--expected-port".into(),
        request.expected_port.to_string(),
    ];
    args.extend(request.arguments.iter().map(|arg| format!("--arg={arg}")));
    args.extend(
        request
            .environment
            .iter()
            .map(|(key, value)| format!("--env={key}={value}")),
    );
    elevate(
        &request.binary_path,
        account,
        owner_window,
        &args,
        "install",
    )
}

pub fn uninstall(binary: &Path) -> Result<(), String> {
    elevate(binary, "", 0, &[], "uninstall")
}

fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

struct ResultDirectory(PathBuf);

impl Drop for ResultDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0.join("result.json"));
        let _ = std::fs::remove_dir(&self.0);
    }
}

fn elevate(
    binary: &Path,
    account: &str,
    owner_window: isize,
    args: &[String],
    operation: &str,
) -> Result<(), String> {
    let directory = ResultDirectory(
        std::env::temp_dir().join(format!("msc2-service-{:032x}", rand::random::<u128>())),
    );
    std::fs::create_dir(&directory.0)
        .map_err(|error| format!("Could not prepare the service helper: {error}"))?;
    let result_path = directory.0.join("result.json");
    let script = format!(
        "$agent={}\n$account={}\n$owner=[IntPtr]{owner_window}\n$serviceName={}\n$installArgs=@({})\n$resultPath={}\n$operation={}\n$nativeSource={}\n{}",
        literal(&binary.to_string_lossy()),
        literal(account),
        literal(super::AGENT_SERVICE_NAME),
        args.iter().map(|arg| literal(arg)).collect::<Vec<_>>().join(","),
        literal(&result_path.to_string_lossy()),
        literal(operation),
        literal(include_str!("windows_service_prompt.cs")),
        include_str!("windows_service.ps1"),
    );
    let encoded = msc_infrastructure::uninstall::native::powershell_encoded(&script);
    let powershell = PathBuf::from(
        std::env::var_os("SystemRoot").ok_or("Could not locate Windows PowerShell.")?,
    )
    .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let launcher = format!(
        "$ErrorActionPreference='Stop'; try {{ $p=Start-Process -FilePath {} -Verb RunAs -WindowStyle Hidden -Wait -PassThru -ArgumentList @('-NoProfile','-NonInteractive','-EncodedCommand','{encoded}'); exit $p.ExitCode }} catch {{ [Console]::Error.WriteLine('Windows administrator approval was cancelled or could not be opened.'); exit 1 }}",
        literal(&powershell.to_string_lossy()),
    );
    if launcher.encode_utf16().count() > 30_000 {
        return Err("The Windows service helper request is too long.".into());
    }
    let output = std::process::Command::new(powershell)
        .args(["-NoProfile", "-NonInteractive", "-Command", &launcher])
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|error| format!("Could not launch the Windows service helper: {error}"))?;
    #[derive(serde::Deserialize)]
    struct HelperResult {
        success: bool,
        message: Option<String>,
    }
    // This file carries only the operation result, never credentials or a script.
    let result = std::fs::read(&result_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<HelperResult>(&bytes).ok());
    match result {
        Some(result) if result.success && output.status.success() => Ok(()),
        Some(result) => Err(result
            .message
            .unwrap_or_else(|| "The Windows service helper did not complete successfully.".into())),
        None => {
            let detail = String::from_utf8_lossy(&output.stderr);
            Err(if detail.trim().is_empty() {
                "The Windows service helper exited without a result.".into()
            } else {
                detail.trim().into()
            })
        }
    }
}
