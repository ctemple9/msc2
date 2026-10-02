//! Destructive local uninstall. No remote endpoint/token is accepted. Both
//! desktop and terminal use this inventory and the same copied worker.
use super::{CliError, CommonArgs, print_json};
use clap::Args;
use msc_infrastructure::uninstall::native::{self, LocalServices, Options, RemovalResult};
use msc_infrastructure::uninstall::{CONFIRMATION, EntryState};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Args)]
pub struct UninstallArgs {
    /// Permit permanent local removal; still requires exact confirmation.
    #[arg(long, conflicts_with = "dry_run")]
    danger: bool,
    /// Show the local removal inventory without making any changes.
    #[arg(long)]
    dry_run: bool,
    /// Exact UNINSTALL MSC 2 phrase for an explicitly approved invocation.
    #[arg(long)]
    confirm: Option<String>,
    /// Refuse if the inventory differs from the previously reviewed preview.
    #[arg(long)]
    fingerprint: Option<String>,
    /// Additional installer to verify against a locally staged signed release.
    #[arg(long = "installer")]
    installers: Vec<PathBuf>,
    /// Keep a removal result outside the directories being removed.
    #[arg(long)]
    report: Option<PathBuf>,
    #[arg(long, hide = true)]
    desktop_origin: Option<PathBuf>,
    #[arg(long, hide = true)]
    wait_pid: Option<u32>,
    #[arg(long, hide = true)]
    apply: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Job {
    options: Options,
    fingerprint: String,
    parent_pid: u32,
    desktop_pid: Option<u32>,
    report: Option<PathBuf>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Scheduled {
    state: &'static str,
    report_path: PathBuf,
    detail: String,
}

fn backend() -> Box<dyn LocalServices> {
    #[cfg(target_os = "macos")]
    return Box::new(msc_platform_macos::uninstall::MacUninstall);
    #[cfg(target_os = "linux")]
    return Box::new(msc_platform_linux::uninstall::LinuxUninstall);
    #[cfg(target_os = "windows")]
    return Box::new(msc_platform_windows::uninstall::WindowsUninstall);
}

pub async fn run(common: CommonArgs, args: UninstallArgs) -> Result<(), CliError> {
    if let Some(path) = &args.apply {
        if !args.danger || args.confirm.as_deref() != Some(CONFIRMATION) {
            return Err(CliError::usage("Worker confirmation is missing."));
        }
        return apply(&common, path).await;
    }
    if !args.dry_run && !args.danger {
        return Err(CliError::usage(
            "Use msc uninstall --dry-run to review, or --danger to request permanent local removal.",
        ));
    }
    let options = Options {
        origin: std::env::current_exe().map_err(|error| CliError::internal(error.to_string()))?,
        desktop_origin: args.desktop_origin,
        installers: args.installers,
    };
    let services = backend();
    let preview = native::discover(
        services.as_ref(),
        &options,
        option_env!("MSC2_RELEASE_PUBLIC_KEY_HEX"),
    )
    .map_err(CliError::internal)?;
    if args.dry_run {
        if common.json {
            return print_json(&preview.inventory);
        }
        print_preview(&preview.inventory);
        return Ok(());
    }
    if preview.inventory.is_blocked() {
        if !common.json {
            print_preview(&preview.inventory);
        }
        return Err(CliError::usage(
            "Uninstall is blocked by one or more inventory findings. Review --dry-run; nothing was removed.",
        ));
    }
    if args
        .fingerprint
        .as_ref()
        .is_some_and(|expected| expected != &preview.inventory.fingerprint)
    {
        return Err(CliError::usage(
            "The removal inventory changed after review. Obtain a new preview and confirm again.",
        ));
    }
    if let Some(report) = &args.report {
        msc_infrastructure::uninstall::validate_target(
            &msc_infrastructure::fs::StdFileSystem,
            &preview.request,
            report,
        )
        .map_err(CliError::usage)?;
        if report.exists() {
            return Err(CliError::usage(
                "The report path already exists; choose a new filename.",
            ));
        }
        if preview
            .inventory
            .entries
            .iter()
            .filter_map(|entry| entry.path.as_ref())
            .any(|path| report.starts_with(path))
        {
            return Err(CliError::usage(
                "The result report must be outside all removal targets.",
            ));
        }
    }
    if let Some(confirmation) = &args.confirm {
        if confirmation != CONFIRMATION {
            return Err(CliError::usage(
                "Confirmation must be exactly UNINSTALL MSC 2.",
            ));
        }
    } else {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() || common.json {
            return Err(CliError::usage(
                "Non-interactive removal requires --danger --confirm 'UNINSTALL MSC 2'. Review --dry-run first.",
            ));
        }
        print_preview(&preview.inventory);
        print!(
            "\nAll listed worlds and backups will be permanently lost.\nType {CONFIRMATION} to continue: "
        );
        io::stdout()
            .flush()
            .map_err(|error| CliError::internal(error.to_string()))?;
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|error| CliError::internal(error.to_string()))?;
        if input.trim_end_matches(['\r', '\n']) != CONFIRMATION {
            return Err(CliError::usage("Uninstall cancelled; nothing was removed."));
        }
    }
    let scheduled = schedule(
        Job {
            options,
            fingerprint: preview.inventory.fingerprint,
            parent_pid: std::process::id(),
            desktop_pid: args.wait_pid,
            report: args.report,
        },
        &preview.agent_data_dir,
        common.json,
    )?;
    if common.json {
        print_json(&scheduled)
    } else {
        println!(
            "{}\nFailure/result report: {}",
            scheduled.detail,
            scheduled.report_path.display()
        );
        Ok(())
    }
}

fn print_preview(inventory: &msc_infrastructure::uninstall::Inventory) {
    println!(
        "Uninstall MSC 2 from {} (local computer only)\n",
        inventory.computer
    );
    for entry in &inventory.entries {
        if entry.state != EntryState::Missing {
            println!(
                "{:?}: {} — {}",
                entry.state,
                entry.identity,
                entry.problem.as_deref().unwrap_or(&entry.evidence)
            );
        }
    }
    for warning in &inventory.warnings {
        println!("\n{warning}");
    }
    for exclusion in &inventory.exclusions {
        println!("Preserved: {exclusion}");
    }
    println!("Preview fingerprint: {}", inventory.fingerprint);
}

fn schedule(job: Job, data_dir: &Path, quiet: bool) -> Result<Scheduled, CliError> {
    let suffix: String = rand::random::<[u8; 16]>()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let directory = std::env::temp_dir().join(format!("msc2-uninstall-{suffix}"));
    let builder = fs::DirBuilder::new();
    #[cfg(unix)]
    let mut builder = builder;
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&directory)
        .map_err(|error| CliError::internal(error.to_string()))?;
    let directory =
        fs::canonicalize(directory).map_err(|error| CliError::internal(error.to_string()))?;
    let worker = directory.join(if cfg!(windows) {
        "msc-uninstall-worker.exe"
    } else {
        "msc-uninstall-worker"
    });
    let job_path = directory.join("job.json");
    let outcome = (|| {
        fs::copy(
            std::env::current_exe().map_err(|error| CliError::internal(error.to_string()))?,
            &worker,
        )
        .map_err(|error| CliError::internal(error.to_string()))?;
        let mut job_file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&job_path)
            .map_err(|error| CliError::internal(error.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            job_file
                .set_permissions(fs::Permissions::from_mode(0o600))
                .map_err(|error| CliError::internal(error.to_string()))?;
        }
        serde_json::to_writer(&mut job_file, &job)
            .map_err(|error| CliError::internal(error.to_string()))?;
        job_file
            .flush()
            .map_err(|error| CliError::internal(error.to_string()))?;
        let mut command = Command::new(&worker);
        command
            .args([
                "uninstall",
                "--danger",
                "--confirm",
                CONFIRMATION,
                "--apply",
            ])
            .arg(&job_path)
            .env("MSC2_DATA_DIR", data_dir)
            .stdin(Stdio::null());
        if job.desktop_pid.is_some() || quiet {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        command.spawn().map_err(|error| {
            CliError::internal(format!("Could not launch uninstall worker: {error}"))
        })?;
        Ok(Scheduled { state: "scheduled", report_path: job.report.clone().unwrap_or_else(|| directory.join("result.json")), detail: "Uninstall scheduled. The worker waits for this command and the desktop to exit, then verifies and removes only local MSC installations. Scheduled does not mean completed.".into() })
    })();
    if outcome.is_err() {
        let _ = fs::remove_dir_all(&directory);
    }
    outcome
}

async fn apply(common: &CommonArgs, job_path: &Path) -> Result<(), CliError> {
    let worker = std::env::current_exe().map_err(|error| CliError::internal(error.to_string()))?;
    let directory = worker
        .parent()
        .ok_or_else(|| CliError::usage("Invalid worker location."))?;
    if job_path.parent() != Some(directory)
        || job_path.file_name().is_none_or(|name| name != "job.json")
        || directory
            .file_name()
            .is_none_or(|name| !name.to_string_lossy().starts_with("msc2-uninstall-"))
    {
        return Err(CliError::usage(
            "Worker job must be in its private continuation directory.",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::symlink_metadata(directory)
            .map_err(|error| CliError::internal(error.to_string()))?;
        let file = fs::symlink_metadata(job_path)
            .map_err(|error| CliError::internal(error.to_string()))?;
        if metadata.file_type().is_symlink()
            || file.file_type().is_symlink()
            || metadata.mode() & 0o077 != 0
            || file.mode() & 0o077 != 0
            || metadata.uid() != file.uid()
        {
            return Err(CliError::usage(
                "Uninstall job permissions or ownership changed.",
            ));
        }
    }
    let job: Job = serde_json::from_slice(
        &fs::read(job_path).map_err(|error| CliError::internal(error.to_string()))?,
    )
    .map_err(|error| CliError::internal(error.to_string()))?;
    let report = job
        .report
        .clone()
        .unwrap_or_else(|| directory.join("result.json"));
    let result = execute_job(&job).await;
    let value = match &result {
        Ok(value) => {
            serde_json::to_value(value).map_err(|error| CliError::internal(error.to_string()))?
        }
        Err(error) => serde_json::json!({"state":"failed", "message":error.to_string()}),
    };
    // No credential contents enter the report. The owner explicitly chooses a
    // retained report; a temporary failure report is kept for diagnosis.
    if job.report.is_some()
        || !result
            .as_ref()
            .is_ok_and(|result| result.state == "uninstalled")
    {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&report)
            .map_err(|error| {
                CliError::internal(format!("Could not preserve uninstall report: {error}"))
            })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))
                .map_err(|error| CliError::internal(error.to_string()))?;
        }
        serde_json::to_writer_pretty(file, &value)
            .map_err(|error| CliError::internal(error.to_string()))?;
    }
    if common.json {
        print_json(&value)?;
    } else {
        println!("MSC uninstall result: {value}");
    }
    let completed = result
        .as_ref()
        .is_ok_and(|result| result.state == "uninstalled");
    if completed || job.report.is_some() {
        cleanup_worker(directory);
    }
    match result {
        Ok(result) if result.state == "uninstalled" => Ok(()),
        Ok(_) => Err(CliError::internal(format!(
            "Uninstall was partial. See {}.",
            report.display()
        ))),
        Err(error) => Err(CliError::internal(format!(
            "{error} See {}.",
            report.display()
        ))),
    }
}

async fn execute_job(job: &Job) -> Result<RemovalResult, CliError> {
    wait_for_exit(job.parent_pid).await?;
    if let Some(pid) = job.desktop_pid {
        wait_for_exit(pid).await?;
    }
    let services = backend();
    let preview = native::discover(
        services.as_ref(),
        &job.options,
        option_env!("MSC2_RELEASE_PUBLIC_KEY_HEX"),
    )
    .map_err(CliError::internal)?;
    if preview.inventory.fingerprint != job.fingerprint || preview.inventory.is_blocked() {
        return Err(CliError::usage(
            "The local inventory changed after confirmation; nothing was removed. Review and confirm again.",
        ));
    }
    if preview.agent_running {
        let client = super::transport::SharedClient::connect_local().await?;
        let status: msc_api::dto::RemoteApiStatus = client.get_json("/v1/status").await?;
        if status.running {
            let _: serde_json::Value = client.post_json("/v1/stop", &serde_json::json!({})).await?;
            let deadline = Instant::now() + Duration::from_secs(60);
            loop {
                let status: msc_api::dto::RemoteApiStatus = client.get_json("/v1/status").await?;
                if !status.running {
                    break;
                }
                if Instant::now() >= deadline {
                    return Err(CliError::internal(
                        "Minecraft did not stop; data was retained.",
                    ));
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
    } else if std::net::TcpStream::connect_timeout(
        &"127.0.0.1:48001".parse().expect("fixed local address"),
        Duration::from_secs(1),
    )
    .is_ok()
    {
        return Err(CliError::usage(
            "An agent is running outside the inspected service. Stop that process before uninstalling; data was retained.",
        ));
    }
    native::remove(services.as_ref(), preview, CONFIRMATION).map_err(CliError::internal)
}

async fn wait_for_exit(pid: u32) -> Result<(), CliError> {
    if pid == 0 || pid == std::process::id() {
        return Err(CliError::usage("Invalid uninstall parent PID."));
    }
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        #[cfg(unix)]
        let alive = Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        #[cfg(windows)]
        let alive = native::capture(
            "powershell.exe",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ 'alive' }}"),
            ],
        )
        .map_err(CliError::internal)?
            == "alive";
        if !alive {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(CliError::internal(
                "The parent app/command did not close; nothing was removed.",
            ));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn cleanup_worker(directory: &Path) {
    #[cfg(unix)]
    {
        let _ = fs::remove_dir_all(directory);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let literal = directory.display().to_string().replace('\'', "''");
        let script = format!(
            "$ErrorActionPreference='Stop'; while (Get-Process -Id {} -ErrorAction SilentlyContinue) {{ Start-Sleep -Milliseconds 250 }}; Remove-Item -LiteralPath '{literal}' -Recurse -Force",
            std::process::id()
        );
        let _ = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-EncodedCommand",
                &native::powershell_encoded(&script),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x08000000)
            .spawn();
    }
}
