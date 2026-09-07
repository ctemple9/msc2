use msc_infrastructure::release_update::{
    self, StagedUpdate, UpdateClientConfig, UpdateResult,
};
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Command};

const DEFAULT_RELEASE_REPOSITORY: &str = "ctemple9/msc2";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub state: &'static str,
    pub release_id: String,
    pub detail: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
    pub release_id: String,
}

pub fn check(data_directory: &Path) -> Result<UpdateResult, String> {
    release_update::check_and_stage(&client_config()?, data_directory)
}

pub fn install(
    request: InstallRequest,
    data_directory: &Path,
) -> Result<InstallResult, String> {
    let config = client_config()?;
    let staged = release_update::verify_staged(&config, data_directory, &request.release_id)?;
    launch_local_install(&staged)?;
    Ok(InstallResult {
        state: if staged.install_mode == "tauri-coordinated" {
            "installer-launched"
        } else {
            "package-installed"
        },
        release_id: request.release_id,
        detail: installation_detail(&staged),
    })
}

fn client_config() -> Result<UpdateClientConfig, String> {
    let trusted_key = option_env!("MSC2_RELEASE_PUBLIC_KEY_HEX")
        .ok_or_else(|| "This desktop package has no configured release-signing key.".to_string())?;
    let trusted_key = decode_hex_key(trusted_key)?;
    Ok(UpdateClientConfig {
        repository: std::env::var("MSC2_RELEASE_REPOSITORY")
            .unwrap_or_else(|_| DEFAULT_RELEASE_REPOSITORY.to_string()),
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        api_major: 1,
        api_minor: 0,
        target: current_target()?,
        trusted_key,
    })
}

fn current_target() -> Result<String, String> {
    if std::env::consts::ARCH != "x86_64" {
        return Err("This MSC release supports x86_64 desktop updates only.".to_string());
    }
    match std::env::consts::OS {
        "macos" => Ok("x86_64-apple-darwin".to_string()),
        "windows" => Ok("x86_64-pc-windows-msvc".to_string()),
        "linux" => Ok("x86_64-unknown-linux-gnu".to_string()),
        _ => Err("This desktop platform is not supported by the MSC updater.".to_string()),
    }
}

fn decode_hex_key(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("This desktop package has an invalid release-signing key.".to_string());
    }
    let mut key = [0_u8; 32];
    for (index, slot) in key.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| "This desktop package has an invalid release-signing key.".to_string())?;
    }
    Ok(key)
}

fn launch_local_install(staged: &StagedUpdate) -> Result<(), String> {
    let mut command = match staged.install_mode.as_str() {
        "tauri-coordinated" => {
            #[cfg(target_os = "macos")]
            {
                let mut command = Command::new("open");
                command.arg(&staged.artifact_path);
                command
            }
            #[cfg(target_os = "windows")]
            {
                let mut command = Command::new("msiexec.exe");
                command.args(["/i"]).arg(&staged.artifact_path);
                command
            }
            #[cfg(target_os = "linux")]
            {
                return Err(
                    "Linux desktop updates must use an authorized package installation.".to_string(),
                );
            }
            #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
            {
                return Err("This platform has no desktop installer handoff.".to_string());
            }
        }
        "authorized-package-install" => {
            #[cfg(target_os = "linux")]
            {
                let mut command = Command::new("pkexec");
                if staged.asset_role == "package-deb" {
                    command.args(["dpkg", "--install"]);
                } else if staged.asset_role == "package-rpm" {
                    command.args(["rpm", "--upgrade"]);
                } else {
                    return Err("The staged Linux package role is invalid.".to_string());
                }
                command.arg(&staged.artifact_path);
                command
            }
            #[cfg(not(target_os = "linux"))]
            {
                return Err("This platform has no package-manager installer handoff.".to_string());
            }
        }
        _ => return Err("The staged update has an unsupported installation mode.".to_string()),
    };

    let status = command
        .status()
        .map_err(|error| format!("Could not launch the local MSC installer: {error}"))?;
    if !status.success() {
        return Err(format!(
            "The local MSC installer exited unsuccessfully: {status}"
        ));
    }
    Ok(())
}

fn installation_detail(staged: &StagedUpdate) -> String {
    match staged.install_mode.as_str() {
        "tauri-coordinated" => format!(
            "The verified {} installer was launched. It may replace the desktop, bundled agent, and required local components together. Configuration, secrets, worlds, and server files remain outside the staged release.",
            staged.artifact_path.display()
        ),
        "authorized-package-install" => format!(
            "The verified {} package was handed to the local package manager. Package ownership remains with the operating system; MSC did not overwrite package-owned files.",
            staged.artifact_path.display()
        ),
        _ => "The verified update was handed to its local installer.".to_string(),
    }
}
