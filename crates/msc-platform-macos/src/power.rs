//! macOS sleep inhibition via `IOPMAssertion`.

use msc_infrastructure::power::{PowerBackend, PowerError, PowerPolicyReason, PowerWarning};
use std::ffi::{CString, c_char, c_void};
use std::process::Command;

const K_CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;
const K_IOPM_ASSERTION_LEVEL_ON: u32 = 255;
const PREVENT_IDLE_SYSTEM_SLEEP: &str = "PreventUserIdleSystemSleep";

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithName(
        assertion_type: CFStringRef,
        level: u32,
        assertion_name: CFStringRef,
        assertion_id: *mut u32,
    ) -> i32;
    fn IOPMAssertionRelease(assertion_id: u32) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(
        alloc: *const c_void,
        c_str: *const c_char,
        encoding: u32,
    ) -> CFStringRef;
    fn CFRelease(value: *const c_void);
}

type CFStringRef = *const c_void;

#[derive(Debug, Default, Clone, Copy)]
pub struct MacosPowerBackend;

#[derive(Debug)]
pub struct MacosPowerAssertion {
    id: u32,
}

impl Drop for MacosPowerAssertion {
    fn drop(&mut self) {
        // SAFETY: the assertion id was returned by IOPMAssertionCreateWithName
        // and remains owned by this handle until Drop.
        unsafe {
            let _ = IOPMAssertionRelease(self.id);
        }
    }
}

impl PowerBackend for MacosPowerBackend {
    type Handle = MacosPowerAssertion;

    fn backend_name(&self) -> &'static str {
        "IOPMAssertion"
    }

    fn activate(&self, reason: &PowerPolicyReason) -> Result<Self::Handle, PowerError> {
        let assertion_type = cf_string(PREVENT_IDLE_SYSTEM_SLEEP)?;
        let assertion_name = cf_string(&format!("MSC2: {}", reason.human_label()))?;
        let mut assertion_id = 0u32;
        // SAFETY: both CFStrings are valid for the duration of the call and
        // `assertion_id` points to writable storage.
        let status = unsafe {
            IOPMAssertionCreateWithName(
                assertion_type,
                K_IOPM_ASSERTION_LEVEL_ON,
                assertion_name,
                &mut assertion_id,
            )
        };
        // SAFETY: CoreFoundation retains the strings during the call only; we
        // release our local references immediately afterward.
        unsafe {
            CFRelease(assertion_type);
            CFRelease(assertion_name);
        }
        if status != 0 {
            return Err(PowerError::Backend(format!(
                "creating IOPMAssertion failed with IOReturn {status}"
            )));
        }
        Ok(MacosPowerAssertion { id: assertion_id })
    }

    fn deactivate(&self, handle: Self::Handle) -> Result<(), PowerError> {
        drop(handle);
        Ok(())
    }
}

pub fn dry_run_action(reason: &PowerPolicyReason) -> String {
    format!(
        "IOPMAssertionCreateWithName({PREVENT_IDLE_SYSTEM_SLEEP}, level={K_IOPM_ASSERTION_LEVEL_ON}, reason=\"MSC2: {}\")",
        reason.human_label()
    )
}

pub fn detected_warnings() -> Vec<PowerWarning> {
    let Ok(output) = Command::new("pmset").args(["-g", "custom"]).output() else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_pmset_custom(&String::from_utf8_lossy(&output.stdout))
}

fn cf_string(value: &str) -> Result<CFStringRef, PowerError> {
    let value = CString::new(value).map_err(|err| {
        PowerError::Backend(format!("power assertion reason contains NUL byte: {err}"))
    })?;
    // SAFETY: null allocator means kCFAllocatorDefault; the C string lives for
    // the duration of the call and uses the requested UTF-8 encoding.
    let value = unsafe {
        CFStringCreateWithCString(std::ptr::null(), value.as_ptr(), K_CFSTRING_ENCODING_UTF8)
    };
    if value.is_null() {
        Err(PowerError::Backend(
            "creating CoreFoundation string for power assertion failed".to_string(),
        ))
    } else {
        Ok(value)
    }
}

fn parse_pmset_custom(output: &str) -> Vec<PowerWarning> {
    let mut warnings = Vec::new();
    for line in output.lines() {
        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        let Some(value) = parts.next() else {
            continue;
        };
        match key {
            "sleep" if value != "0" => warnings.push(PowerWarning::new(
                "macos-sleep-timer",
                format!("macOS sleep timer is set to {value} minute(s)."),
            )),
            "hibernatemode" if value != "0" => warnings.push(PowerWarning::new(
                "macos-hibernation",
                format!("macOS hibernatemode is {value}, so deep sleep may override assertions."),
            )),
            "standby" if value == "1" => warnings.push(PowerWarning::new(
                "macos-standby",
                "macOS standby is enabled; long idle periods may still sleep the host.",
            )),
            "autopoweroff" if value == "1" => warnings.push(PowerWarning::new(
                "macos-autopoweroff",
                "macOS autopoweroff is enabled; long idle periods may still sleep the host.",
            )),
            _ => {}
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use msc_infrastructure::power::PowerPolicyReason;

    #[test]
    fn power_policy_macos_dry_run_mentions_iopmassertion() {
        let action = dry_run_action(&PowerPolicyReason::RemoteManagement);
        assert!(action.contains("IOPMAssertionCreateWithName"));
        assert!(action.contains("PreventUserIdleSystemSleep"));
    }

    #[test]
    fn power_policy_macos_parses_pmset_warnings() {
        let warnings = parse_pmset_custom(
            "Battery Power:\n sleep 10\n hibernatemode 3\n standby 1\n autopoweroff 1\n",
        );
        assert_eq!(warnings.len(), 4);
        assert_eq!(warnings[0].code, "macos-sleep-timer");
    }
}
