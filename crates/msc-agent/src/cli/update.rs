//! Local application-update commands.
//!
//! Unlike the server-management commands, these commands deliberately do not
//! construct an HTTP client. Updating a headless host is a local installation
//! operation, even when the release bytes are downloaded from GitHub.

use super::{CliError, CommonArgs, print_json};
use clap::Subcommand;
use msc_infrastructure::config_repository::default_app_data_dir;
use msc_infrastructure::release_update::{
    self, StagedUpdate, UpdateChannel, UpdateClientConfig, UpdateResult,
};
use msc_infrastructure::service::{
    ServiceManager, ServiceManagerCommand, ServiceName, ServiceState,
};
use serde::Serialize;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::process::Command;
use std::time::Duration;

const DEFAULT_RELEASE_REPOSITORY: &str = "ctemple9/msc2";
const AGENT_SERVICE_NAME: &str = "com.ctemple.msc2.agent";
const AGENT_PORT: u16 = 48001;

#[derive(Debug, Clone, Subcommand)]
pub enum UpdateCommand {
    /// Fetch, verify, and stage the latest signed release without installing it.
    Check,
    /// Install one exact release that was previously staged by `update check`.
    Install {
        /// Release ID printed by `update check`, for example `0.1.2`.
        #[arg(long)]
        release_id: String,
        /// Approve the exact release without a prompt. Required for automation
        /// and other non-interactive invocations.
        #[arg(long, alias = "non-interactive")]
        yes: bool,
    },
    /// Internal continuation used after the parent CLI exits so Windows can
    /// replace the executable that launched the update.
    #[command(name = "apply", hide = true)]
    Apply {
        #[arg(long)]
        release_id: String,
        #[arg(long)]
        parent_pid: u32,
        #[arg(long, hide = true)]
        data_dir: Option<PathBuf>,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallOutput {
    state: &'static str,
    release_id: String,
    install_mode: String,
    release_notes: String,
    detail: String,
}

#[derive(Debug)]
struct PayloadChange {
    names: Vec<String>,
    rollback: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallationKind {
    Standalone,
    #[cfg(target_os = "linux")]
    LinuxPackageDeb,
    #[cfg(target_os = "linux")]
    LinuxPackageRpm,
}

pub fn run(common: CommonArgs, command: UpdateCommand) -> Result<(), CliError> {
    reject_remote_options(&common)?;
    match command {
        UpdateCommand::Check => check(&common),
        UpdateCommand::Install { release_id, yes } => install(&common, &release_id, yes),
        UpdateCommand::Apply {
            release_id,
            parent_pid,
            data_dir,
        } => apply(&common, &release_id, parent_pid, data_dir.as_deref()),
    }
}

fn check(common: &CommonArgs) -> Result<(), CliError> {
    let installation = detect_installation()?;
    let result =
        release_update::check_and_stage(&client_config(installation)?, &default_app_data_dir())
            .map_err(CliError::internal)?;
    if common.json {
        return print_json(&result);
    }
    print_check_result(&result);
    Ok(())
}

fn install(common: &CommonArgs, release_id: &str, yes: bool) -> Result<(), CliError> {
    let installation = detect_installation()?;
    let data_directory = default_app_data_dir();
    let staged =
        release_update::verify_staged(&client_config(installation)?, &data_directory, release_id)
            .map_err(CliError::internal)?;

    if !approve_install(release_id, yes)? {
        let output = InstallOutput {
            state: "declined",
            release_id: release_id.to_string(),
            install_mode: staged.install_mode.clone(),
            release_notes: read_release_notes(&staged),
            detail: "Update was not installed.".to_string(),
        };
        return if common.json {
            print_json(&output)
        } else {
            println!("{}", output.detail);
            Ok(())
        };
    }

    #[cfg(target_os = "windows")]
    {
        let child = Command::new(std::env::current_exe().map_err(|error| {
            CliError::internal(format!(
                "could not resolve the current msc executable: {error}"
            ))
        })?)
        .args([
            "update",
            "apply",
            "--release-id",
            release_id,
            "--parent-pid",
            &std::process::id().to_string(),
        ])
        .spawn()
        .map_err(|error| {
            CliError::internal(format!("could not schedule the local update: {error}"))
        })?;
        let output = InstallOutput {
            state: "scheduled",
            release_id: release_id.to_string(),
            install_mode: staged.install_mode.clone(),
            release_notes: read_release_notes(&staged),
            detail: format!(
                "The verified update was scheduled as local process {} and will restart the agent after this command exits.",
                child.id()
            ),
        };
        return if common.json {
            print_json(&output)
        } else {
            print_install_output(&output);
            Ok(())
        };
    }

    #[cfg(target_os = "linux")]
    if replacement_needs_authorization() {
        let current_executable = std::env::current_exe().map_err(|error| {
            CliError::internal(format!(
                "could not resolve the current msc executable: {error}"
            ))
        })?;
        let child = Command::new("pkexec")
            .arg(current_executable)
            .args(["update", "apply", "--release-id", release_id])
            .arg("--parent-pid")
            .arg(std::process::id().to_string())
            .arg("--data-dir")
            .arg(&data_directory)
            .spawn()
            .map_err(|error| {
                CliError::internal(format!(
                    "could not request local update authorization: {error}"
                ))
            })?;
        let output = InstallOutput {
            state: "scheduled",
            release_id: release_id.to_string(),
            install_mode: staged.install_mode.clone(),
            release_notes: read_release_notes(&staged),
            detail: format!(
                "The verified update was scheduled through local authorization process {} and will restart the agent after this command exits.",
                child.id()
            ),
        };
        return if common.json {
            print_json(&output)
        } else {
            print_install_output(&output);
            Ok(())
        };
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = apply_verified_update(&staged, &data_directory)?;
        if common.json {
            print_json(&output)
        } else {
            print_install_output(&output);
            Ok(())
        }
    }
}

fn apply(
    common: &CommonArgs,
    release_id: &str,
    parent_pid: u32,
    requested_data_directory: Option<&Path>,
) -> Result<(), CliError> {
    let installation = detect_installation()?;
    let data_directory = requested_data_directory
        .map(Path::to_path_buf)
        .unwrap_or_else(default_app_data_dir);
    let staged =
        release_update::verify_staged(&client_config(installation)?, &data_directory, release_id)
            .map_err(CliError::internal)?;

    wait_for_parent(parent_pid)?;
    let output = apply_verified_update(&staged, &data_directory)?;
    if common.json {
        print_json(&output)
    } else {
        print_install_output(&output);
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn wait_for_parent(parent_pid: u32) -> Result<(), CliError> {
    // Windows keeps an executable locked while its process is alive. The
    // helper must wait for the confirming CLI to exit before replacing msc.exe.
    let script = format!(
        "$process = Get-Process -Id {parent_pid} -ErrorAction SilentlyContinue; if ($process) {{ $process.WaitForExit() }}"
    );
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .status()
        .map_err(|error| {
            CliError::internal(format!("could not wait for the parent process: {error}"))
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(CliError::internal(
            "could not wait for the confirming update process to exit",
        ))
    }
}

#[cfg(not(target_os = "windows"))]
fn wait_for_parent(_parent_pid: u32) -> Result<(), CliError> {
    // The parent has already handed off the update. Unix permits replacing an
    // executing binary, so no file-lock wait is required for this path.
    Ok(())
}

fn apply_verified_update(
    staged: &StagedUpdate,
    data_directory: &Path,
) -> Result<InstallOutput, CliError> {
    let notes = read_release_notes(staged);
    if staged.install_mode == "authorized-package-install" {
        return Ok(InstallOutput {
            state: "package-manager-guidance",
            release_id: staged.manifest.release_id.clone(),
            install_mode: staged.install_mode.clone(),
            release_notes: notes,
            detail: package_manager_guidance(staged),
        });
    }
    if staged.install_mode != "standalone-archive" {
        return Err(CliError::internal(
            "The staged release is not a headless standalone archive.",
        ));
    }

    let current_executable = std::env::current_exe().map_err(|error| {
        CliError::internal(format!("could not resolve the current executable: {error}"))
    })?;
    let installation_root = current_executable.parent().ok_or_else(|| {
        CliError::internal("the current executable has no installation directory")
    })?;
    let payload = data_directory
        .join("updates")
        .join(&staged.manifest.release_id)
        .join("payload");
    release_update::extract_standalone_archive(&staged.artifact_path, &payload)
        .map_err(CliError::internal)?;

    let service_state = local_service_state()?;
    if service_state == ServiceState::Running {
        stop_local_service()?;
    }

    let rollback = data_directory
        .join("updates")
        .join(&staged.manifest.release_id)
        .join("rollback");
    let changed =
        replace_payload(&payload, installation_root, &rollback).map_err(CliError::internal)?;

    if service_state == ServiceState::Running
        && let Err(error) = start_local_service().and_then(|_| wait_for_agent_health())
    {
        let rollback_result = rollback_payload(&changed, installation_root);
        let _ = stop_local_service();
        let _ = start_local_service();
        let detail = match rollback_result {
            Ok(()) => format!(
                "The updated agent failed its health check and the previous payload was restored: {error}"
            ),
            Err(rollback_error) => format!(
                "The updated agent failed its health check ({error}); restoring the previous payload also failed: {rollback_error}"
            ),
        };
        return Err(CliError::internal(detail));
    }

    Ok(InstallOutput {
        state: if service_state == ServiceState::Running {
            "installed-and-restarted"
        } else {
            "installed"
        },
        release_id: staged.manifest.release_id.clone(),
        install_mode: staged.install_mode.clone(),
        release_notes: notes,
        detail: if service_state == ServiceState::Running {
            format!(
                "Verified {} was installed, the local agent service was restarted, and /v1/healthz recovered.",
                staged.manifest.release_id
            )
        } else {
            format!(
                "Verified {} was installed. The local agent service was not running, so it was left stopped.",
                staged.manifest.release_id
            )
        },
    })
}

fn client_config(installation: InstallationKind) -> Result<UpdateClientConfig, CliError> {
    let channel = match installation {
        InstallationKind::Standalone => UpdateChannel::Headless,
        #[cfg(target_os = "linux")]
        InstallationKind::LinuxPackageDeb => UpdateChannel::LinuxPackageDeb,
        #[cfg(target_os = "linux")]
        InstallationKind::LinuxPackageRpm => UpdateChannel::LinuxPackageRpm,
    };
    let trusted_key = option_env!("MSC2_RELEASE_PUBLIC_KEY_HEX").ok_or_else(|| {
        CliError::internal("This headless package has no configured release-signing key.")
    })?;
    let trusted_key = decode_hex_key(trusted_key)?;
    Ok(UpdateClientConfig {
        repository: std::env::var("MSC2_RELEASE_REPOSITORY")
            .unwrap_or_else(|_| DEFAULT_RELEASE_REPOSITORY.to_string()),
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        api_major: 1,
        api_minor: 0,
        target: current_target(),
        trusted_key,
        channel,
        linux_package_format: None,
    })
}

fn decode_hex_key(value: &str) -> Result<[u8; 32], CliError> {
    let mut key = [0_u8; 32];
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(CliError::internal(
            "This headless package has an invalid release-signing key.",
        ));
    }
    for (index, slot) in key.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).map_err(|_| {
            CliError::internal("This headless package has an invalid release-signing key.")
        })?;
    }
    Ok(key)
}

fn current_target() -> String {
    match std::env::consts::OS {
        "macos" => match std::env::consts::ARCH {
            "x86_64" => "x86_64-apple-darwin",
            "aarch64" => "aarch64-apple-darwin",
            _ => "unsupported",
        },
        "windows" => "x86_64-pc-windows-msvc",
        "linux" => "x86_64-unknown-linux-gnu",
        _ => "unsupported",
    }
    .to_string()
}

fn detect_installation() -> Result<InstallationKind, CliError> {
    #[cfg(target_os = "linux")]
    {
        let current = std::env::current_exe().map_err(|error| {
            CliError::internal(format!("could not resolve the current executable: {error}"))
        })?;
        let marker = current
            .parent()
            .map(|path| path.join(".msc2-installation-mode"));
        if marker.as_deref().is_some_and(|path| {
            fs::read_to_string(path)
                .map(|value| value.trim() == "standalone-archive")
                .unwrap_or(false)
        }) {
            return Ok(InstallationKind::Standalone);
        }
        if package_owns("dpkg-query", &["-S", &current.to_string_lossy()]) {
            return Ok(InstallationKind::LinuxPackageDeb);
        }
        if package_owns("rpm", &["-qf", &current.to_string_lossy()]) {
            return Ok(InstallationKind::LinuxPackageRpm);
        }
    }
    Ok(InstallationKind::Standalone)
}

#[cfg(target_os = "linux")]
fn replacement_needs_authorization() -> bool {
    let binary_name = if cfg!(target_os = "windows") {
        "msc.exe"
    } else {
        "msc"
    };
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|root| root.join(binary_name)))
        .is_some_and(|path| fs::OpenOptions::new().write(true).open(path).is_err())
}

#[cfg(target_os = "linux")]
fn package_owns(command: &str, args: &[&str]) -> bool {
    Command::new(command)
        .args(args)
        .output()
        .is_ok_and(|output| output.status.success())
}

fn reject_remote_options(common: &CommonArgs) -> Result<(), CliError> {
    if common.base_url.is_some()
        || common.token.is_some()
        || common.host != super::DEFAULT_HOST
        || common.port != super::DEFAULT_PORT
    {
        return Err(CliError::usage(
            "update commands are local-only; remove remote host, URL, port, and token options",
        ));
    }
    Ok(())
}

fn approve_install(release_id: &str, yes: bool) -> Result<bool, CliError> {
    if yes {
        return Ok(true);
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(CliError::usage(format!(
            "update install requires --yes when no interactive terminal is available; this approves release {release_id}"
        )));
    }
    print!("Install verified MSC 2 release {release_id} locally? Type 'yes' to continue: ");
    io::stdout().flush().map_err(|error| {
        CliError::internal(format!("could not display the update prompt: {error}"))
    })?;
    let mut response = String::new();
    io::stdin().read_line(&mut response).map_err(|error| {
        CliError::internal(format!("could not read update confirmation: {error}"))
    })?;
    Ok(response.trim().eq_ignore_ascii_case("yes"))
}

fn print_check_result(result: &UpdateResult) {
    println!("{}", result.detail);
    if let Some(release_id) = &result.release_id {
        println!("release: {release_id}");
    }
    if let Some(mode) = &result.install_mode {
        println!("installation: {mode}");
    }
    if let Some(artifact) = &result.artifact_filename {
        println!("artifact: {artifact}");
    }
    if !result.release_notes.is_empty() {
        println!("\nrelease notes:\n{}", result.release_notes);
    }
}

fn print_install_output(output: &InstallOutput) {
    println!("{}", output.detail);
    println!("release: {}", output.release_id);
    if !output.release_notes.is_empty() {
        println!("\nrelease notes:\n{}", output.release_notes);
    }
}

fn read_release_notes(staged: &StagedUpdate) -> String {
    fs::read_to_string(
        staged
            .artifact_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("RELEASE-NOTES.md"),
    )
    .unwrap_or_default()
}

fn package_manager_guidance(staged: &StagedUpdate) -> String {
    let command = match staged.asset_role.as_str() {
        "package-deb" => format!("sudo apt install {}", staged.artifact_path.display()),
        "package-rpm" => format!("sudo dnf install {}", staged.artifact_path.display()),
        _ => "Use this distribution's package manager for the staged package.".to_string(),
    };
    format!(
        "This installation is owned by the distribution package manager. MSC did not replace it. Run: {command}"
    )
}

fn replace_payload(
    payload: &Path,
    installation_root: &Path,
    rollback: &Path,
) -> Result<PayloadChange, String> {
    fs::create_dir_all(rollback)
        .map_err(|error| format!("Could not create rollback storage: {error}"))?;
    let binary_name = if cfg!(target_os = "windows") {
        "msc.exe"
    } else {
        "msc"
    };
    let mut names = vec![binary_name.to_string()];
    if payload.join("sidecar").is_dir() {
        names.push("sidecar".to_string());
    }

    let mut changed = Vec::new();
    let result: Result<(), String> = (|| {
        for name in names {
            let source = payload.join(&name);
            if !source.exists() {
                return Err(format!("The signed headless archive is missing {name}."));
            }
            let target = installation_root.join(&name);
            let backup = rollback.join(&name);
            if target.exists() {
                copy_path(&target, &backup)?;
            }
            let temporary =
                installation_root.join(format!(".{name}.msc2-update-{}", std::process::id()));
            if temporary.exists() {
                remove_path(&temporary)?;
            }
            copy_path(&source, &temporary)?;
            if target.exists() {
                remove_path(&target)?;
            }
            fs::rename(&temporary, &target)
                .map_err(|error| format!("Could not install {name}: {error}"))?;
            changed.push(name);
        }
        Ok(())
    })();
    if let Err(error) = result {
        let partial = PayloadChange {
            names: changed,
            rollback: rollback.to_path_buf(),
        };
        let rollback_error = rollback_payload(&partial, installation_root).err();
        return Err(match rollback_error {
            Some(rollback_error) => {
                format!("{error}; partial replacement rollback failed: {rollback_error}")
            }
            None => error,
        });
    }
    Ok(PayloadChange {
        names: changed,
        rollback: rollback.to_path_buf(),
    })
}

fn rollback_payload(changed: &PayloadChange, installation_root: &Path) -> Result<(), String> {
    for name in &changed.names {
        let target = installation_root.join(name);
        if target.exists() {
            remove_path(&target)?;
        }
        let backup = changed.rollback.join(name);
        if backup.exists() {
            let temporary =
                installation_root.join(format!(".{name}.msc2-rollback-{}", std::process::id()));
            copy_path(&backup, &temporary)?;
            fs::rename(&temporary, &target)
                .map_err(|error| format!("Could not restore {name}: {error}"))?;
        }
    }
    Ok(())
}

fn copy_path(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("Could not inspect {}: {error}", source.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("Refusing to copy symlink {}", source.display()));
    }
    if metadata.is_dir() {
        fs::create_dir_all(destination)
            .map_err(|error| format!("Could not create {}: {error}", destination.display()))?;
        for entry in fs::read_dir(source)
            .map_err(|error| format!("Could not read {}: {error}", source.display()))?
        {
            let entry =
                entry.map_err(|error| format!("Could not read directory entry: {error}"))?;
            copy_path(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else if metadata.is_file() {
        fs::copy(source, destination)
            .map_err(|error| format!("Could not copy {}: {error}", source.display()))?;
        fs::set_permissions(destination, metadata.permissions()).map_err(|error| {
            format!(
                "Could not preserve {} permissions: {error}",
                destination.display()
            )
        })?;
    } else {
        return Err(format!("Refusing unsupported file {}", source.display()));
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .map_err(|error| format!("Could not remove {}: {error}", path.display()))
}

fn local_service_state() -> Result<ServiceState, CliError> {
    let service_name = ServiceName::new(AGENT_SERVICE_NAME);
    #[cfg(target_os = "macos")]
    let result = msc_platform_macos::service::MacosLaunchdServiceManager::new()
        .execute(ServiceManagerCommand::Status { service_name });
    #[cfg(target_os = "linux")]
    let result = msc_platform_linux::service::LinuxSystemdServiceManager::new()
        .execute(ServiceManagerCommand::Status { service_name });
    #[cfg(target_os = "windows")]
    let result = msc_platform_windows::service::WindowsServiceManager::new()
        .execute(ServiceManagerCommand::Status { service_name });
    result.map(|report| report.state).map_err(|error| {
        CliError::internal(format!(
            "could not inspect the local agent service: {error}"
        ))
    })
}

fn stop_local_service() -> Result<(), CliError> {
    let service_name = ServiceName::new(AGENT_SERVICE_NAME);
    #[cfg(target_os = "macos")]
    {
        msc_platform_macos::service::stop_elevated(service_name.as_str()).map_err(|error| {
            CliError::internal(format!("could not stop the local agent service: {error}"))
        })?;
    }
    #[cfg(target_os = "linux")]
    {
        msc_platform_linux::service::LinuxSystemdServiceManager::new()
            .execute(ServiceManagerCommand::Stop { service_name })
            .map_err(|error| {
                CliError::internal(format!("could not stop the local agent service: {error}"))
            })?;
    }
    #[cfg(target_os = "windows")]
    {
        msc_platform_windows::service::WindowsServiceManager::new()
            .execute(ServiceManagerCommand::Stop { service_name })
            .map_err(|error| {
                CliError::internal(format!("could not stop the local agent service: {error}"))
            })?;
    }
    Ok(())
}

fn start_local_service() -> Result<(), CliError> {
    let service_name = ServiceName::new(AGENT_SERVICE_NAME);
    #[cfg(target_os = "macos")]
    {
        msc_platform_macos::service::start_elevated(service_name.as_str()).map_err(|error| {
            CliError::internal(format!("could not start the local agent service: {error}"))
        })?;
    }
    #[cfg(target_os = "linux")]
    {
        msc_platform_linux::service::LinuxSystemdServiceManager::new()
            .execute(ServiceManagerCommand::Start { service_name })
            .map_err(|error| {
                CliError::internal(format!("could not start the local agent service: {error}"))
            })?;
    }
    #[cfg(target_os = "windows")]
    {
        msc_platform_windows::service::WindowsServiceManager::new()
            .execute(ServiceManagerCommand::Start { service_name })
            .map_err(|error| {
                CliError::internal(format!("could not start the local agent service: {error}"))
            })?;
    }
    Ok(())
}

fn wait_for_agent_health() -> Result<(), CliError> {
    for _ in 0..40 {
        if let Ok(mut stream) = TcpStream::connect_timeout(
            &format!("127.0.0.1:{AGENT_PORT}")
                .parse()
                .expect("agent address is valid"),
            Duration::from_millis(250),
        ) {
            stream
                .set_read_timeout(Some(Duration::from_millis(250)))
                .map_err(|error| {
                    CliError::internal(format!("could not configure health check: {error}"))
                })?;
            stream
                .write_all(
                    b"GET /v1/healthz HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
                )
                .map_err(|error| {
                    CliError::internal(format!("could not query agent health: {error}"))
                })?;
            let mut response = String::new();
            let _ = stream.read_to_string(&mut response);
            if response.starts_with("HTTP/1.1 204") || response.starts_with("HTTP/1.1 200") {
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(CliError::internal(
        "the local agent did not recover its /v1/healthz endpoint",
    ))
}
