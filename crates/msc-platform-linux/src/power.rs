//! Linux sleep inhibition via `systemd-inhibit`.

use msc_infrastructure::power::{PowerBackend, PowerError, PowerPolicyReason, PowerWarning};
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

const HOLD_SCRIPT: &str = "trap 'exit 0' TERM INT; while :; do sleep 3600; done";

#[derive(Debug, Clone)]
pub struct LinuxPowerBackend {
    inhibit_path: PathBuf,
}

impl LinuxPowerBackend {
    pub fn new() -> Self {
        Self {
            inhibit_path: PathBuf::from("systemd-inhibit"),
        }
    }
}

impl Default for LinuxPowerBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct LinuxInhibitHandle {
    child: Child,
}

impl PowerBackend for LinuxPowerBackend {
    type Handle = LinuxInhibitHandle;

    fn backend_name(&self) -> &'static str {
        "systemd-inhibit"
    }

    fn activate(&self, reason: &PowerPolicyReason) -> Result<Self::Handle, PowerError> {
        let child = Command::new(&self.inhibit_path)
            .arg("--what=sleep")
            .arg("--who=MSC2")
            .arg("--mode=block")
            .arg(format!("--why={}", reason.human_label()))
            .arg("/bin/sh")
            .arg("-c")
            .arg(HOLD_SCRIPT)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| {
                PowerError::Backend(format!(
                    "starting systemd-inhibit hold process failed: {err}"
                ))
            })?;
        Ok(LinuxInhibitHandle { child })
    }

    fn deactivate(&self, mut handle: Self::Handle) -> Result<(), PowerError> {
        handle.child.kill().map_err(|err| {
            PowerError::Backend(format!(
                "stopping systemd-inhibit hold process failed: {err}"
            ))
        })?;
        let _ = handle.child.wait();
        Ok(())
    }
}

pub fn dry_run_action(reason: &PowerPolicyReason) -> String {
    format!(
        "systemd-inhibit --what=sleep --who=MSC2 --mode=block --why='{}' /bin/sh -c '{}'",
        reason.human_label(),
        HOLD_SCRIPT
    )
}

pub fn detected_warnings() -> Vec<PowerWarning> {
    let Ok(contents) = fs::read_to_string("/etc/systemd/logind.conf") else {
        return Vec::new();
    };
    parse_logind_conf(&contents)
}

fn parse_logind_conf(contents: &str) -> Vec<PowerWarning> {
    let mut warnings = Vec::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(value) = line.strip_prefix("HandleLidSwitch=")
            && value != "ignore"
        {
            warnings.push(PowerWarning::new(
                "linux-lid-switch",
                format!("systemd logind HandleLidSwitch is {value}."),
            ));
        }
        if let Some(value) = line.strip_prefix("IdleAction=")
            && value != "ignore"
        {
            warnings.push(PowerWarning::new(
                "linux-idle-action",
                format!("systemd logind IdleAction is {value}."),
            ));
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use msc_infrastructure::power::PowerPolicyReason;

    #[test]
    fn power_policy_linux_dry_run_mentions_systemd_inhibit() {
        let action = dry_run_action(&PowerPolicyReason::RemoteManagement);
        assert!(action.contains("systemd-inhibit"));
        assert!(action.contains("--what=sleep"));
    }

    #[test]
    fn power_policy_linux_parses_logind_warnings() {
        let warnings = parse_logind_conf(
            "HandleLidSwitch=suspend\nIdleAction=hibernate\n#HandleLidSwitch=ignore\n",
        );
        assert_eq!(warnings.len(), 2);
        assert_eq!(warnings[0].code, "linux-lid-switch");
    }
}
