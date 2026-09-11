use msc_infrastructure::release_update::{
    self, LinuxPackageFormat, StagedUpdate, UpdateChannel, UpdateClientConfig, UpdateResult,
};
use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use std::{fs, time::Duration};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

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

/// Runs before Tauri starts so a replacement can move the old app bundle out
/// of the way. The normal desktop process schedules this helper and exits;
/// the helper then verifies the same staged release again before installing.
pub fn run_desktop_update_helper() -> bool {
    let mut args = std::env::args_os();
    let Some(mode) = args.nth(1) else {
        return false;
    };
    if mode != "--msc2-apply-desktop-update" {
        return false;
    }
    let values: Vec<String> = args.filter_map(|value| value.into_string().ok()).collect();
    let result = (|| {
        let release_id = argument_value(&values, "--release-id")?;
        let data_directory = PathBuf::from(argument_value(&values, "--data-dir")?);
        wait_for_parent(
            argument_value(&values, "--parent-pid")?
                .parse()
                .map_err(|_| {
                    "The desktop update helper received an invalid parent process ID.".to_string()
                })?,
        )?;
        apply_desktop_update(&release_id, &data_directory)
    })();
    match result {
        Ok(detail) => {
            println!("{detail}");
            std::process::exit(0);
        }
        Err(error) => {
            eprintln!("MSC 2 desktop update failed: {error}");
            std::process::exit(1);
        }
    }
}

pub fn check(data_directory: &Path) -> Result<UpdateResult, String> {
    release_update::check_and_stage(&client_config()?, data_directory)
}

pub fn install(request: InstallRequest, data_directory: &Path) -> Result<InstallResult, String> {
    let config = client_config()?;
    let staged = release_update::verify_staged(&config, data_directory, &request.release_id)?;

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    if staged.install_mode == "tauri-coordinated" {
        schedule_desktop_update(&request.release_id, data_directory)?;
        return Ok(InstallResult {
            state: "scheduled",
            release_id: request.release_id,
            detail: "The verified update was scheduled. MSC 2 will close, install it, and relaunch when the replacement is complete.".to_string(),
        });
    }

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

fn argument_value(values: &[String], name: &str) -> Result<String, String> {
    values
        .windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
        .ok_or_else(|| format!("The desktop update helper is missing {name}."))
}

fn schedule_desktop_update(release_id: &str, data_directory: &Path) -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("Could not resolve the desktop executable: {error}"))?;
    Command::new(&executable)
        .arg("--msc2-apply-desktop-update")
        .args(["--release-id", release_id, "--parent-pid"])
        .arg(std::process::id().to_string())
        .args(["--data-dir"])
        .arg(data_directory)
        .spawn()
        .map_err(|error| format!("Could not schedule the desktop update: {error}"))?;
    Ok(())
}

fn apply_desktop_update(release_id: &str, data_directory: &Path) -> Result<String, String> {
    let staged = release_update::verify_staged(&client_config()?, data_directory, release_id)?;
    #[cfg(target_os = "macos")]
    return install_macos_bundle(&staged);
    #[cfg(target_os = "windows")]
    return install_windows_msi(&staged);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = staged;
        Err("This desktop update helper is not supported on this platform.".to_string())
    }
}

#[cfg(target_os = "windows")]
fn wait_for_parent(parent_pid: u32) -> Result<(), String> {
    let script = format!(
        "$process = Get-Process -Id {parent_pid} -ErrorAction SilentlyContinue; if ($process) {{ $process.WaitForExit() }}"
    );
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()
        .map_err(|error| format!("Could not wait for the desktop process: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| "The desktop update helper could not wait for MSC 2 to close.".to_string())
}

#[cfg(target_os = "macos")]
fn wait_for_parent(_parent_pid: u32) -> Result<(), String> {
    // The short delay gives Tauri time to tear down the webview before the
    // helper replaces the bundle. macOS permits replacing an executing file.
    std::thread::sleep(Duration::from_millis(500));
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn wait_for_parent(_parent_pid: u32) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_windows_msi(staged: &StagedUpdate) -> Result<String, String> {
    let status = Command::new("msiexec.exe")
        .args(["/i"])
        .arg(&staged.artifact_path)
        .status()
        .map_err(|error| format!("Could not run the MSC installer: {error}"))?;
    if !status.success() {
        return Err(format!("The MSC installer exited unsuccessfully: {status}"));
    }
    let executable = std::env::current_exe()
        .map_err(|error| format!("Could not resolve the installed desktop executable: {error}"))?;
    Command::new(&executable)
        .spawn()
        .map_err(|error| format!("Could not relaunch MSC 2 after updating: {error}"))?;
    Ok(format!(
        "MSC 2 {} was installed and relaunched.",
        staged.manifest.release_id
    ))
}

#[cfg(target_os = "macos")]
fn install_macos_bundle(staged: &StagedUpdate) -> Result<String, String> {
    let current_bundle = current_app_bundle()?;
    let work_directory = staged
        .artifact_path
        .parent()
        .ok_or_else(|| "The staged desktop installer has no containing directory.".to_string())?
        .join("desktop-install-work");
    if work_directory.exists() {
        fs::remove_dir_all(&work_directory).map_err(|error| {
            format!("Could not clear the previous desktop update work area: {error}")
        })?;
    }
    let mount_directory = work_directory.join("mounted");
    fs::create_dir_all(&mount_directory)
        .map_err(|error| format!("Could not create the desktop update work area: {error}"))?;
    let mounted = Command::new("/usr/bin/hdiutil")
        .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
        .arg(&mount_directory)
        .arg(&staged.artifact_path)
        .status()
        .map_err(|error| format!("Could not mount the MSC installer: {error}"))?;
    if !mounted.success() {
        return Err("The MSC disk image could not be mounted.".to_string());
    }
    let result = (|| {
        let source_bundle = find_app_bundle(&mount_directory, current_bundle.file_name())?;
        let replacement = work_directory.join(
            current_bundle
                .file_name()
                .ok_or_else(|| "The installed MSC bundle has no name.".to_string())?,
        );
        run_command(
            Command::new("/usr/bin/ditto")
                .args(["--rsrc", "--extattr", "--acl"])
                .arg(source_bundle)
                .arg(&replacement),
            "Could not copy the new MSC app bundle",
        )?;
        replace_app_bundle(&current_bundle, &replacement)?;
        run_command(
            Command::new("/usr/bin/open").arg(&current_bundle),
            "Could not relaunch MSC 2",
        )?;
        Ok(format!(
            "MSC 2 {} was installed and relaunched.",
            staged.manifest.release_id
        ))
    })();
    let _ = Command::new("/usr/bin/hdiutil")
        .args(["detach", "-force"])
        .arg(&mount_directory)
        .status();
    let _ = fs::remove_dir_all(&work_directory);
    result
}

#[cfg(target_os = "macos")]
fn current_app_bundle() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|error| format!("Could not resolve the running desktop executable: {error}"))?
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(Path::to_path_buf)
        .ok_or_else(|| "MSC 2 must be installed as an app bundle to update itself.".to_string())
}

#[cfg(target_os = "macos")]
fn find_app_bundle(
    root: &Path,
    expected_name: Option<&std::ffi::OsStr>,
) -> Result<PathBuf, String> {
    let bundles: Vec<PathBuf> = fs::read_dir(root)
        .map_err(|error| format!("Could not inspect the mounted MSC installer: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "app"))
        .collect();
    bundles
        .iter()
        .find(|path| expected_name.is_some_and(|name| path.file_name() == Some(name)))
        .or_else(|| (bundles.len() == 1).then_some(&bundles[0]))
        .cloned()
        .ok_or_else(|| {
            "The MSC disk image does not contain exactly one matching app bundle.".to_string()
        })
}

#[cfg(target_os = "macos")]
fn replace_app_bundle(target: &Path, replacement: &Path) -> Result<(), String> {
    let backup = target.with_file_name(format!(
        ".{}.msc2-backup-{}",
        target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("MSC 2.app"),
        std::process::id()
    ));
    let direct = || -> Result<(), String> {
        fs::rename(target, &backup)
            .map_err(|error| format!("Could not back up the installed MSC app: {error}"))?;
        if let Err(error) = fs::rename(replacement, target) {
            let _ = fs::rename(&backup, target);
            return Err(format!("Could not activate the new MSC app: {error}"));
        }
        fs::remove_dir_all(&backup)
            .map_err(|error| format!("Could not remove the old MSC app backup: {error}"))
    };
    match direct() {
        Ok(()) => Ok(()),
        Err(error)
            if error.contains("Permission denied") || error.contains("Operation not permitted") =>
        {
            let command = format!(
                "set -e; /bin/mv {} {}; /bin/mv {} {}; /bin/rm -rf {}",
                shell_quote(target),
                shell_quote(&backup),
                shell_quote(replacement),
                shell_quote(target),
                shell_quote(&backup)
            );
            run_command(
                Command::new("/usr/bin/osascript").args([
                    "-e",
                    &format!("do shell script {command:?} with administrator privileges"),
                ]),
                "Could not authorize replacement of the installed MSC app",
            )
        }
        Err(error) => Err(error),
    }
}

#[cfg(target_os = "macos")]
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

fn run_command(command: &mut Command, failure: &str) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|error| format!("{failure}: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{failure}: process exited unsuccessfully ({status})"))
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
        channel: UpdateChannel::Desktop,
        linux_package_format: linux_package_format(),
    })
}

#[cfg(target_os = "linux")]
fn linux_package_format() -> Option<LinuxPackageFormat> {
    let current = std::env::current_exe().ok()?;
    let path = current.to_string_lossy();
    if Command::new("dpkg-query")
        .args(["-S", path.as_ref()])
        .output()
        .is_ok_and(|output| output.status.success())
    {
        return Some(LinuxPackageFormat::Deb);
    }
    if Command::new("rpm")
        .args(["-qf", path.as_ref()])
        .output()
        .is_ok_and(|output| output.status.success())
    {
        return Some(LinuxPackageFormat::Rpm);
    }
    match std::env::var("MSC2_LINUX_PACKAGE_FORMAT")
        .ok()?
        .to_ascii_lowercase()
        .as_str()
    {
        "deb" => Some(LinuxPackageFormat::Deb),
        "rpm" => Some(LinuxPackageFormat::Rpm),
        _ => None,
    }
}

#[cfg(not(target_os = "linux"))]
fn linux_package_format() -> Option<LinuxPackageFormat> {
    None
}

fn current_target() -> Result<String, String> {
    match std::env::consts::OS {
        "macos" => match std::env::consts::ARCH {
            "x86_64" => Ok("x86_64-apple-darwin".to_string()),
            "aarch64" => Ok("aarch64-apple-darwin".to_string()),
            _ => Err("This macOS architecture is not supported by the MSC updater.".to_string()),
        },
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
                    "Linux desktop updates must use an authorized package installation."
                        .to_string(),
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
