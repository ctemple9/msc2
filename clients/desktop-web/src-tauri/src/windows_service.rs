//! Fixed desktop service operations; passwords stay inside the elevated helper.

use msc_infrastructure::service::ServiceInstallRequest;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};

pub fn repair(request: &ServiceInstallRequest, owner_window: isize) -> Result<(), String> {
    use msc_infrastructure::service::{
        ServiceManager, ServiceManagerCommand, ServiceName, ServiceState,
    };
    let status = msc_platform_windows::service::WindowsServiceManager::new()
        .execute(ServiceManagerCommand::Status {
            service_name: ServiceName::new(super::AGENT_SERVICE_NAME),
        })
        .map_err(|error| error.to_string())?;
    if status.state == ServiceState::NotInstalled {
        return install(request, owner_window);
    }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let package_root = executable
        .parent()
        .ok_or("The desktop package has no directory.")?;
    let mut hashes = serde_json::Map::new();
    for name in ["msc.exe", "vantage.exe", "bedrock-map.exe"] {
        let bytes = std::fs::read(package_root.join("agent").join(name))
            .map_err(|error| format!("Could not read the repair payload {name}: {error}"))?;
        use sha2::Digest;
        hashes.insert(
            name.into(),
            super::hex_lower(&sha2::Sha256::digest(&bytes)).into(),
        );
    }
    let account = request
        .run_user
        .as_deref()
        .ok_or("The local service needs an owner.")?;
    let repair = serde_json::json!({
        "operation": "repair", "transaction": uuid::Uuid::new_v4().to_string(),
        "packageRoot": package_root, "dataRoot": request.working_directory, "account": account,
    });
    elevate(
        &request.binary_path,
        account,
        owner_window,
        &[
            repair.to_string(),
            serde_json::Value::Object(hashes).to_string(),
        ],
        "repair",
    )
}

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
    let mut script = format!(
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
    if operation == "repair" {
        script = format!(
            "{}\n{script}",
            include_str!("../../../../packaging/windows/desktop-service-lifecycle.ps1")
        );
    }
    let encoded = msc_infrastructure::uninstall::native::powershell_encoded(&script);
    let powershell = PathBuf::from(
        std::env::var_os("SystemRoot").ok_or("Could not locate Windows PowerShell.")?,
    )
    .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    // Compress the embedded script in memory to fit Windows' command-line limit.
    // Stdin avoids temporary script files across the elevation boundary.
    let launcher = format!(
        "$ErrorActionPreference='Stop'; try {{ $bytes=[Convert]::FromBase64String('{encoded}'); $buffer=New-Object IO.MemoryStream; $gzip=New-Object IO.Compression.GZipStream($buffer,[IO.Compression.CompressionMode]::Compress,$true); $gzip.Write($bytes,0,$bytes.Length); $gzip.Dispose(); $packed=[Convert]::ToBase64String($buffer.ToArray()); $bootstrap=\"`$stream=New-Object IO.MemoryStream(,[Convert]::FromBase64String('$packed')); `$gzip=New-Object IO.Compression.GZipStream(`$stream,[IO.Compression.CompressionMode]::Decompress); `$reader=New-Object IO.StreamReader(`$gzip,[Text.Encoding]::Unicode); & ([scriptblock]::Create(`$reader.ReadToEnd()))\"; $command=[char]34 + $bootstrap + [char]34; if ($command.Length -gt 28000) {{ throw 'The Windows service helper request is too long.' }}; $p=Start-Process -FilePath {} -Verb RunAs -WindowStyle Hidden -Wait -PassThru -ArgumentList @('-NoProfile','-NonInteractive','-Command',$command); exit $p.ExitCode }} catch {{ [Console]::Error.WriteLine($_.Exception.Message); exit 1 }}",
        literal(&powershell.to_string_lossy()),
    );
    let mut child = std::process::Command::new(powershell)
        .args(["-NoProfile", "-NonInteractive", "-Command", "-"])
        .creation_flags(0x0800_0000)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| format!("Could not launch the Windows service helper: {error}"))?;
    use std::io::Write;
    child
        .stdin
        .take()
        .ok_or("Could not open the service helper input.")?
        .write_all(format!("{launcher}\n").as_bytes())
        .map_err(|error| format!("Could not send the service helper request: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
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
