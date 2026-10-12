use msc_infrastructure::release_update::{
    self, LinuxPackageFormat, StagedUpdate, UpdateChannel, UpdateClientConfig, UpdateResult,
};
use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

const DEFAULT_RELEASE_REPOSITORY: &str = "ctemple9/msc2";
#[cfg(target_os = "windows")]
#[path = "update_windows.rs"]
mod windows;

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

pub fn report_desktop_update_ready(page_finished: bool) {
    let args: Vec<_> = std::env::args_os().collect();
    let value_after = |name: &str| {
        args.windows(2)
            .find(|pair| pair[0] == name)
            .map(|pair| pair[1].clone())
    };
    let (Some(marker), Some(token)) = (
        value_after("--msc2-update-health-file"),
        value_after("--msc2-update-health-token"),
    ) else {
        return;
    };
    #[cfg(target_os = "macos")]
    let _ = std::fs::write(
        PathBuf::from(&marker).with_extension("pid"),
        std::process::id().to_string(),
    );
    if page_finished {
        let _ = std::fs::write(marker, token.to_string_lossy().as_bytes());
    }
}

pub(super) fn health_trial() -> bool {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let args: Vec<_> = std::env::args_os().collect();
        if let Some(pair) = args
            .windows(2)
            .find(|p| p[0] == "--msc2-update-health-file")
        {
            let marker = PathBuf::from(&pair[1]);
            return marker.parent().is_some_and(Path::exists)
                && !marker.with_extension("accepted").exists();
        }
    }
    false
}

/// Runs before Tauri starts so a replacement can move the old app bundle out
/// of the way. The normal desktop process schedules this helper and exits;
/// the helper then verifies the same staged release again before installing.
pub fn run_desktop_update_helper() -> bool {
    #[cfg(target_os = "windows")]
    if windows::run_service_helper() {
        return true;
    }
    let mut args = std::env::args_os();
    let Some(mode) = args.nth(1) else {
        return false;
    };
    #[cfg(target_os = "macos")]
    if mode == "--msc2-repair-update-agent" {
        if let Err(error) =
            super::manage_agent_service_blocking(super::AgentServiceAction::Repair, 0)
        {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return true;
    }
    if mode != "--msc2-apply-desktop-update"
        && !(cfg!(target_os = "windows") && mode == "--msc2-resume-desktop-update")
    {
        return false;
    }
    let values: Vec<String> = args.filter_map(|value| value.into_string().ok()).collect();
    let result =
        (|| {
            let release_id = argument_value(&values, "--release-id")?;
            let data_directory = PathBuf::from(argument_value(&values, "--data-dir")?);
            if mode == "--msc2-apply-desktop-update" {
                wait_for_parent(argument_value(&values, "--parent-pid")?.parse().map_err(
                    |_| {
                        "The desktop update helper received an invalid parent process ID."
                            .to_string()
                    },
                )?)?;
            }
            #[cfg(target_os = "windows")]
            return windows::apply(
                &release_id,
                &data_directory,
                &PathBuf::from(argument_value(&values, "--installed-executable")?),
                mode == "--msc2-resume-desktop-update",
            );
            #[cfg(not(target_os = "windows"))]
            apply_desktop_update(&release_id, &data_directory)
        })();
    match result {
        Ok(detail) => {
            println!("{detail}");
            std::process::exit(0);
        }
        Err(error) => {
            #[cfg(target_os = "windows")]
            windows::notify(&error);
            eprintln!("MSC 2 desktop update failed: {error}");
            std::process::exit(1);
        }
    }
}

pub fn check(data_directory: &Path) -> Result<UpdateResult, String> {
    let result = release_update::check_and_stage(&client_config()?, data_directory)?;
    #[cfg(target_os = "windows")]
    let mut result = result;
    #[cfg(target_os = "windows")]
    if let Ok(detail) =
        std::fs::read_to_string(data_directory.join("updates/windows-last-result.json"))
    {
        if let Ok(record) = serde_json::from_str::<serde_json::Value>(&detail) {
            if let Some(detail) = record["detail"].as_str() {
                result.detail = format!("Last Windows update: {detail}\n{}", result.detail);
            }
        }
    }
    Ok(result)
}

pub fn install(request: InstallRequest, data_directory: &Path) -> Result<InstallResult, String> {
    let config = client_config()?;
    let staged = release_update::verify_staged(&config, data_directory, &request.release_id)?;

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    if staged.install_mode == "tauri-coordinated" {
        #[cfg(target_os = "macos")]
        require_stopped_macos_server()?;
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
    #[cfg(target_os = "windows")]
    return windows::schedule(release_id, data_directory);
    #[cfg(not(target_os = "windows"))]
    {
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
}

#[cfg(not(target_os = "windows"))]
fn apply_desktop_update(release_id: &str, data_directory: &Path) -> Result<String, String> {
    let staged = release_update::verify_staged(&client_config()?, data_directory, release_id)?;
    #[cfg(target_os = "macos")]
    return install_macos_bundle(&staged);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = staged;
        Err("This desktop update helper is not supported on this platform.".to_string())
    }
}

#[cfg(target_os = "windows")]
fn wait_for_parent(parent_pid: u32) -> Result<(), String> {
    windows::wait_parent(parent_pid)
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

#[cfg(target_os = "macos")]
fn install_macos_bundle(staged: &StagedUpdate) -> Result<String, String> {
    require_stopped_macos_server()?;
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
    let mut rollback = None;
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
        rollback = Some(replace_app_bundle(&current_bundle, &replacement)?);
        // Bind the service and desktop trust identity to the replacement before
        // asking it to connect. The same transaction restores both on rollback.
        repair_macos_bundle_agent(&current_bundle)?;
        let marker = work_directory.join("desktop-ready");
        let token = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| format!("Could not create update health token: {error}"))?
                .as_nanos()
        );
        run_command(
            Command::new("/usr/bin/open")
                .arg("-n")
                .arg(&current_bundle)
                .args(["--args", "--msc2-update-health-file"])
                .arg(&marker)
                .args(["--msc2-update-health-token", &token]),
            "Could not relaunch MSC 2",
        )?;
        wait_for_macos_health(&marker, &token)?;
        fs::write(marker.with_extension("accepted"), &token)
            .map_err(|error| format!("Could not accept the update health trial: {error}"))?;
        if let Some(previous_installation) = rollback.take() {
            finalize_macos_replacement(&previous_installation)?;
        }
        Ok(format!(
            "MSC 2 {} was installed, relaunched, and passed desktop and agent health checks.",
            staged.manifest.release_id
        ))
    })();
    if let (Err(error), Some(rollback)) = (&result, rollback.take()) {
        let recovery = stop_macos_trial(&work_directory.join("desktop-ready"))
            .and_then(|_| rollback_macos_replacement(rollback))
            .and_then(|_| {
                repair_macos_bundle_agent(&current_bundle)?;
                run_command(
                    Command::new("/usr/bin/open").arg(&current_bundle),
                    "Could not relaunch the previous MSC app",
                )
            });
        if let Err(recovery_error) = recovery {
            return Err(format!(
                "{error}; restoring the previous MSC app failed: {recovery_error}"
            ));
        }
        return Err(format!(
            "{error}; the previous MSC app was restored and relaunched."
        ));
    }
    let _ = Command::new("/usr/bin/hdiutil")
        .args(["detach", "-force"])
        .arg(&mount_directory)
        .status();
    let _ = fs::remove_dir_all(&work_directory);
    result
}

#[cfg(target_os = "macos")]
fn wait_for_macos_health(marker: &Path, token: &str) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(90);
    while Instant::now() < deadline {
        let desktop_ready = fs::read_to_string(marker).is_ok_and(|value| value == token);
        if desktop_ready && macos_agent_is_healthy() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(
        "The replacement desktop and local agent did not both become healthy within 90 seconds."
            .to_string(),
    )
}

#[cfg(target_os = "macos")]
fn macos_agent_is_healthy() -> bool {
    let address = SocketAddr::from(([127, 0, 0, 1], 48001));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(500)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    if stream
        .write_all(b"GET /v1/healthz HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut response = String::new();
    stream.read_to_string(&mut response).is_ok() && healthy_macos_agent_response(&response)
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
fn replace_app_bundle(
    target: &Path,
    replacement: &Path,
) -> Result<(PathBuf, PathBuf, bool), String> {
    let backup = target.with_file_name(format!(
        ".{}.msc2-backup-{}",
        target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("MSC 2.app"),
        std::process::id()
    ));
    let direct = || -> Result<(PathBuf, PathBuf, bool), String> {
        fs::rename(target, &backup)
            .map_err(|error| format!("Could not back up the installed MSC app: {error}"))?;
        if let Err(error) = fs::rename(replacement, target) {
            let _ = fs::rename(&backup, target);
            return Err(format!("Could not activate the new MSC app: {error}"));
        }
        Ok((target.to_path_buf(), backup.clone(), false))
    };
    match direct() {
        Ok(rollback) => Ok(rollback),
        Err(error)
            if error.contains("Permission denied") || error.contains("Operation not permitted") =>
        {
            let command = format!(
                "set -e; /bin/mv {} {}; if ! /bin/mv {} {}; then /bin/mv {} {}; exit 1; fi",
                shell_quote(target),
                shell_quote(&backup),
                shell_quote(replacement),
                shell_quote(target),
                shell_quote(&backup),
                shell_quote(target)
            );
            run_command(
                Command::new("/usr/bin/osascript").args([
                    "-e",
                    &format!("do shell script {command:?} with administrator privileges"),
                ]),
                "Could not authorize replacement of the installed MSC app",
            )
            .map(|_| (target.to_path_buf(), backup.clone(), true))
        }
        Err(error) => Err(error),
    }
}

#[cfg(target_os = "macos")]
fn finalize_macos_replacement(rollback: &(PathBuf, PathBuf, bool)) -> Result<(), String> {
    let (_, backup, elevated) = rollback;
    if *elevated {
        let command = format!("/bin/rm -rf {}", shell_quote(&backup));
        run_command(
            Command::new("/usr/bin/osascript").args([
                "-e",
                &format!("do shell script {command:?} with administrator privileges"),
            ]),
            "Could not remove the previous MSC app after health succeeded",
        )
    } else {
        fs::remove_dir_all(&backup).map_err(|error| {
            format!("Could not remove the previous MSC app after health succeeded: {error}")
        })
    }
}

#[cfg(target_os = "macos")]
fn rollback_macos_replacement(rollback: (PathBuf, PathBuf, bool)) -> Result<(), String> {
    let (target, backup, elevated) = rollback;
    let rejected = target.with_file_name(format!(".msc2-rejected-{}", std::process::id()));
    if elevated {
        let command = format!(
            "set -e; /bin/mv {} {}; /bin/mv {} {}; /bin/rm -rf {}",
            shell_quote(&target),
            shell_quote(&rejected),
            shell_quote(&backup),
            shell_quote(&target),
            shell_quote(&rejected)
        );
        run_command(
            Command::new("/usr/bin/osascript").args([
                "-e",
                &format!("do shell script {command:?} with administrator privileges"),
            ]),
            "Could not restore the previous MSC app with administrator authorization",
        )
    } else {
        fs::rename(&target, &rejected)
            .map_err(|error| format!("Could not move the failed MSC app aside: {error}"))?;
        fs::rename(&backup, &target)
            .map_err(|error| format!("Could not restore the previous MSC app: {error}"))?;
        fs::remove_dir_all(&rejected)
            .map_err(|error| format!("Could not remove the failed MSC app: {error}"))
    }
}

#[cfg(target_os = "macos")]
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

#[cfg(target_os = "macos")]
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

#[cfg(any(target_os = "macos", test))]
fn healthy_macos_agent_response(response: &str) -> bool {
    // /v1/healthz's documented liveness response has no body (204).
    matches!(
        response
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1)),
        Some("204")
    ) && response.starts_with("HTTP/1.")
}

#[cfg(test)]
mod health_response_tests {
    use super::*;
    #[test]
    fn accepts_agent_liveness_contract_and_rejects_failures() {
        assert!(healthy_macos_agent_response(
            "HTTP/1.1 204 No Content\r\n\r\n"
        ));
        for response in [
            "HTTP/1.1 401 Unauthorized\r\n",
            "HTTP/1.1 503 Unavailable\r\n",
            "garbage 204",
            "HTTP/1.1 2040 Invalid",
        ] {
            assert!(!healthy_macos_agent_response(response));
        }
    }
}

#[cfg(target_os = "macos")]
fn repair_macos_bundle_agent(bundle: &Path) -> Result<(), String> {
    // Execute the installed bundle's code: the old helper may now reside in
    // the rollback folder, with the old package and signing requirement.
    let executable_name = std::env::current_exe()
        .map_err(|error| error.to_string())?
        .file_name()
        .ok_or("The desktop executable has no filename.")?
        .to_owned();
    run_command(
        Command::new(bundle.join("Contents/MacOS").join(executable_name))
            .arg("--msc2-repair-update-agent"),
        "Could not register the installed bundle's matching agent",
    )
}

#[cfg(target_os = "macos")]
fn require_stopped_macos_server() -> Result<(), String> {
    let (report, _) = super::current_local_agent_report()?;
    if report.state != super::ServiceState::Running {
        return Ok(());
    }
    let binary = report
        .definition
        .ok_or("Could not identify the running agent. Stop the server and agent before updating.")?
        .binary_path;
    let output = Command::new(binary)
        .args(["status", "--json"])
        .output()
        .map_err(|error| format!("Could not check the running server: {error}"))?;
    let status: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|_| {
        "Could not confirm the server is stopped. Stop the server and agent before updating."
            .to_string()
    })?;
    if !output.status.success() || status["running"].as_bool() != Some(false) {
        return Err(
            "Stop the Minecraft server before updating MSC 2. The update must restart its agent."
                .into(),
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn stop_macos_trial(marker: &Path) -> Result<(), String> {
    let Ok(pid) = fs::read_to_string(marker.with_extension("pid")) else {
        return Ok(());
    };
    let pid: u32 = pid
        .trim()
        .parse()
        .map_err(|_| "The update trial PID is invalid.".to_string())?;
    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .map_err(|error| format!("Could not inspect the update trial: {error}"))?;
    if !output.status.success() {
        return Ok(());
    }
    let command = String::from_utf8_lossy(&output.stdout);
    if !command.contains("--msc2-update-health-file")
        || !command.contains(marker.to_string_lossy().as_ref())
    {
        return Err(
            "The update trial PID now belongs to another process; it was left untouched.".into(),
        );
    }
    run_command(
        Command::new("/bin/kill").args(["-TERM", &pid.to_string()]),
        "Could not stop the failed update trial",
    )?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if !Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
        {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("The failed desktop trial did not exit. Close it before recovering the update.".into())
}
