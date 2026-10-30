//! Register the graphical companion only for desktop-package users.

pub(super) fn install() -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Ok(());
    }
    let status = super::service_manager()?
        .execute(msc_infrastructure::service::ServiceManagerCommand::Status {
            service_name: msc_infrastructure::service::ServiceName::new(super::AGENT_SERVICE_NAME),
        })
        .map_err(|error| error.to_string())?;
    if status.state == msc_infrastructure::service::ServiceState::NotInstalled {
        return Ok(());
    }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let executable = executable
        .to_str()
        .ok_or("Indicator executable path is not UTF-8")?;
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME").ok_or("Missing home directory")?;
        let directory = std::path::PathBuf::from(home).join("Library/LaunchAgents");
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let escaped = executable
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        std::fs::write(directory.join("com.ctemple.msc2.indicator.plist"), format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict><key>Label</key><string>com.ctemple.msc2.indicator</string><key>ProgramArguments</key><array><string>{escaped}</string><string>--agent-indicator</string></array><key>RunAtLoad</key><true/></dict></plist>"
        )).map_err(|error| error.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        let result = std::process::Command::new("reg.exe")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "MSC2AgentIndicator",
                "/t",
                "REG_SZ",
                "/d",
                &format!("\"{executable}\" --agent-indicator"),
                "/f",
            ])
            .status()
            .map_err(|error| error.to_string())?;
        if !result.success() {
            return Err("Could not register indicator startup".into());
        }
    }
    #[cfg(target_os = "linux")]
    {
        let directory = std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
            })
            .ok_or("Missing user configuration directory")?
            .join("autostart");
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        let escaped = executable
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('`', "\\`")
            .replace('$', "\\$")
            .replace('%', "%%");
        std::fs::write(directory.join("msc2-agent-indicator.desktop"), format!(
            "[Desktop Entry]\nType=Application\nName=MSC 2 Agent Indicator\nExec=\"{escaped}\" --agent-indicator\nTerminal=false\n"
        )).map_err(|error| error.to_string())?;
    }
    let mut command = std::process::Command::new(executable);
    command.arg("--agent-indicator");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command.spawn().map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) fn remove() -> Result<(), String> {
    super::agent_indicator::stop()?;
    remove_registration()
}

pub(super) fn remove_registration() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let path = std::path::PathBuf::from(std::env::var_os("HOME").ok_or("Missing home directory")?)
        .join("Library/LaunchAgents/com.ctemple.msc2.indicator.plist");
    #[cfg(target_os = "linux")]
    let path = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
        })
        .ok_or("Missing user configuration directory")?
        .join("autostart/msc2-agent-indicator.desktop");
    #[cfg(unix)]
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("reg.exe")
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "MSC2AgentIndicator",
                "/f",
            ])
            .status();
    }
    Ok(())
}
