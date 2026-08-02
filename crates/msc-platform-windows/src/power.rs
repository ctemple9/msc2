//! Windows sleep inhibition via `SetThreadExecutionState`.

use msc_infrastructure::power::{PowerBackend, PowerError, PowerPolicyReason};
use windows_sys::Win32::System::Power::{
    ES_AWAYMODE_REQUIRED, ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsPowerBackend;

#[derive(Debug, Clone, Copy)]
pub struct WindowsPowerHandle;

impl PowerBackend for WindowsPowerBackend {
    type Handle = WindowsPowerHandle;

    fn backend_name(&self) -> &'static str {
        "SetThreadExecutionState"
    }

    fn activate(&self, reason: &PowerPolicyReason) -> Result<Self::Handle, PowerError> {
        let mut flags = ES_CONTINUOUS | ES_SYSTEM_REQUIRED;
        if reason.is_dedicated_remote_management() {
            flags |= ES_AWAYMODE_REQUIRED;
        }
        // SAFETY: this Win32 API takes a plain flag bitmask and returns the
        // previous thread state; no pointers cross the boundary.
        let previous = unsafe { SetThreadExecutionState(flags) };
        if previous == 0 {
            return Err(PowerError::Backend(
                "SetThreadExecutionState returned 0 while enabling sleep inhibition".to_string(),
            ));
        }
        Ok(WindowsPowerHandle)
    }

    fn deactivate(&self, _handle: Self::Handle) -> Result<(), PowerError> {
        // SAFETY: this clears any continuous execution state the current thread
        // asked Windows to hold.
        let previous = unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
        if previous == 0 {
            return Err(PowerError::Backend(
                "SetThreadExecutionState returned 0 while clearing sleep inhibition".to_string(),
            ));
        }
        Ok(())
    }
}

pub fn dry_run_action(reason: &PowerPolicyReason) -> String {
    let mut flags = vec!["ES_CONTINUOUS", "ES_SYSTEM_REQUIRED"];
    if reason.is_dedicated_remote_management() {
        flags.push("ES_AWAYMODE_REQUIRED");
    }
    format!("SetThreadExecutionState({})", flags.join(" | "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use msc_infrastructure::power::PowerPolicyReason;

    #[test]
    fn power_policy_windows_remote_management_adds_away_mode() {
        let action = dry_run_action(&PowerPolicyReason::RemoteManagement);
        assert!(action.contains("SetThreadExecutionState"));
        assert!(action.contains("ES_AWAYMODE_REQUIRED"));
    }

    #[test]
    fn power_policy_windows_active_server_omits_away_mode() {
        let action = dry_run_action(&PowerPolicyReason::ActiveServer { count: 1 });
        assert!(!action.contains("ES_AWAYMODE_REQUIRED"));
    }
}
