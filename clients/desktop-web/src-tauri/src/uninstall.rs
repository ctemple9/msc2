//! The desktop's local uninstall seam. The selected HTTP host is never used.
use msc_infrastructure::uninstall::{Inventory, CONFIRMATION};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static SCHEDULED: AtomicBool = AtomicBool::new(false);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallRequest {
    confirmation: String,
    fingerprint: String,
    installers: Vec<PathBuf>,
    keep_report: bool,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Scheduled {
    state: String,
    report_path: PathBuf,
    detail: String,
}

fn local_command(installers: &[PathBuf]) -> Result<std::process::Command, String> {
    #[cfg(target_os = "linux")]
    let binary = super::linux_system_agent_path()?;
    #[cfg(not(target_os = "linux"))]
    let binary = super::packaged_agent_path()?;
    if !binary.is_file() {
        return Err("The packaged uninstall command is missing. Rebuild or reinstall MSC before using this action.".into());
    }
    let mut command = std::process::Command::new(binary);
    command
        .args(["uninstall", "--json", "--desktop-origin"])
        .arg(std::env::current_exe().map_err(|error| error.to_string())?);
    // Windows discovers the agent's data path from its service metadata. The
    // desktop's staging directory is not an override for that configuration.
    #[cfg(not(target_os = "windows"))]
    command.env("MSC2_DATA_DIR", super::agent_data_directory()?);
    for installer in installers {
        command.arg("--installer").arg(installer);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    Ok(command)
}
fn output<T: serde::de::DeserializeOwned>(mut command: std::process::Command) -> Result<T, String> {
    let output = command
        .output()
        .map_err(|error| format!("Could not run the local uninstall command: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().into());
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Invalid local uninstall response: {error}"))
}

#[tauri::command]
pub async fn preview_local_uninstall(installers: Vec<PathBuf>) -> Result<Inventory, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut command = local_command(&installers)?;
        command.arg("--dry-run");
        output(command)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn uninstall_local(request: UninstallRequest) -> Result<Scheduled, String> {
    if request.confirmation != CONFIRMATION {
        return Err("Type exactly UNINSTALL MSC 2 before proceeding.".into());
    }
    if request.fingerprint.is_empty() {
        return Err("Review the local deletion preview first.".into());
    }
    if SCHEDULED.swap(true, Ordering::SeqCst) {
        return Err("Local uninstall is already being scheduled.".into());
    }
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut command = local_command(&request.installers)?;
        command.args([
            "--danger",
            "--confirm",
            CONFIRMATION,
            "--fingerprint",
            &request.fingerprint,
            "--wait-pid",
            &std::process::id().to_string(),
        ]);
        if request.keep_report {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| error.to_string())?
                .as_millis();
            let home = std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .ok_or("Cannot determine your home directory for the result report.")?;
            command
                .arg("--report")
                .arg(PathBuf::from(home).join(format!("msc2-uninstall-result-{timestamp}.json")));
        }
        let scheduled: Scheduled = output(command)?;
        if scheduled.state != "scheduled" {
            return Err("The local uninstall worker did not accept the request.".into());
        }
        Ok(scheduled)
    })
    .await
    .map_err(|error| error.to_string())
    .and_then(|result| result);
    if result.is_err() {
        SCHEDULED.store(false, Ordering::SeqCst);
    }
    result
}
