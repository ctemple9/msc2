//! Shared power-management policy for the Phase 4 lifecycle slice.
//!
//! D-024 has two user-visible rules:
//! - a dedicated/headless host stays awake whenever remote management is on
//! - any host stays awake while servers or critical operations are active
//!
//! The platform crates supply the native sleep-inhibition mechanism
//! (`IOPMAssertion`, `SetThreadExecutionState`, or `systemd-inhibit`).

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostRole {
    DedicatedHeadless,
    NormalDesktop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PowerPolicyContext {
    pub host_role: Option<HostRole>,
    pub remote_management_enabled: bool,
    pub running_servers: usize,
    pub critical_operations: usize,
}

impl PowerPolicyContext {
    pub fn with_host_role(mut self, host_role: HostRole) -> Self {
        self.host_role = Some(host_role);
        self
    }

    pub fn with_remote_management(mut self, enabled: bool) -> Self {
        self.remote_management_enabled = enabled;
        self
    }

    pub fn with_running_servers(mut self, running_servers: usize) -> Self {
        self.running_servers = running_servers;
        self
    }

    pub fn with_critical_operations(mut self, critical_operations: usize) -> Self {
        self.critical_operations = critical_operations;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerPolicyReason {
    RemoteManagement,
    ActiveServer {
        count: usize,
    },
    CriticalOperation {
        count: usize,
    },
    ActiveServerAndCriticalOperation {
        running_servers: usize,
        critical_operations: usize,
    },
}

impl PowerPolicyReason {
    pub fn human_label(&self) -> String {
        match self {
            Self::RemoteManagement => {
                "remote management is enabled on a dedicated/headless host".to_string()
            }
            Self::ActiveServer { count } => {
                format!("{count} server(s) are running")
            }
            Self::CriticalOperation { count } => {
                format!("{count} critical operation(s) are running")
            }
            Self::ActiveServerAndCriticalOperation {
                running_servers,
                critical_operations,
            } => format!(
                "{running_servers} server(s) and {critical_operations} critical operation(s) are running"
            ),
        }
    }

    pub fn is_dedicated_remote_management(&self) -> bool {
        matches!(self, Self::RemoteManagement)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerPolicyDecision {
    pub prevent_sleep: bool,
    pub reason: Option<PowerPolicyReason>,
}

impl PowerPolicyDecision {
    pub fn allow_sleep() -> Self {
        Self {
            prevent_sleep: false,
            reason: None,
        }
    }

    pub fn prevent_sleep(reason: PowerPolicyReason) -> Self {
        Self {
            prevent_sleep: true,
            reason: Some(reason),
        }
    }
}

pub fn evaluate_power_policy(context: PowerPolicyContext) -> PowerPolicyDecision {
    if matches!(context.host_role, Some(HostRole::DedicatedHeadless))
        && context.remote_management_enabled
    {
        return PowerPolicyDecision::prevent_sleep(PowerPolicyReason::RemoteManagement);
    }

    match (context.running_servers, context.critical_operations) {
        (0, 0) => PowerPolicyDecision::allow_sleep(),
        (running_servers, 0) => {
            PowerPolicyDecision::prevent_sleep(PowerPolicyReason::ActiveServer {
                count: running_servers,
            })
        }
        (0, critical_operations) => {
            PowerPolicyDecision::prevent_sleep(PowerPolicyReason::CriticalOperation {
                count: critical_operations,
            })
        }
        (running_servers, critical_operations) => PowerPolicyDecision::prevent_sleep(
            PowerPolicyReason::ActiveServerAndCriticalOperation {
                running_servers,
                critical_operations,
            },
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerError {
    Backend(String),
}

impl fmt::Display for PowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for PowerError {}

pub trait PowerBackend {
    type Handle;

    fn backend_name(&self) -> &'static str;

    fn activate(&self, reason: &PowerPolicyReason) -> Result<Self::Handle, PowerError>;

    fn deactivate(&self, handle: Self::Handle) -> Result<(), PowerError>;
}

#[derive(Debug)]
pub struct PowerPolicyController<B: PowerBackend> {
    backend: B,
    active: Option<B::Handle>,
}

impl<B: PowerBackend> PowerPolicyController<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            active: None,
        }
    }

    pub fn backend_name(&self) -> &'static str {
        self.backend.backend_name()
    }

    pub fn update(
        &mut self,
        context: PowerPolicyContext,
    ) -> Result<PowerPolicyDecision, PowerError> {
        let decision = evaluate_power_policy(context);
        match decision.reason.as_ref() {
            Some(reason) => {
                if let Some(handle) = self.active.take() {
                    self.backend.deactivate(handle)?;
                }
                let handle = self.backend.activate(reason)?;
                self.active = Some(handle);
            }
            None => {
                if let Some(handle) = self.active.take() {
                    self.backend.deactivate(handle)?;
                }
            }
        }
        Ok(decision)
    }
}

impl<B: PowerBackend> Drop for PowerPolicyController<B> {
    fn drop(&mut self) {
        if let Some(handle) = self.active.take() {
            let _ = self.backend.deactivate(handle);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerWarning {
    pub code: &'static str,
    pub message: String,
}

impl PowerWarning {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct FakePowerBackend {
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl FakePowerBackend {
        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl PowerBackend for &FakePowerBackend {
        type Handle = String;

        fn backend_name(&self) -> &'static str {
            "fake"
        }

        fn activate(&self, reason: &PowerPolicyReason) -> Result<Self::Handle, PowerError> {
            let label = reason.human_label();
            self.calls.lock().unwrap().push(format!("activate:{label}"));
            Ok(label)
        }

        fn deactivate(&self, handle: Self::Handle) -> Result<(), PowerError> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("deactivate:{handle}"));
            Ok(())
        }
    }

    #[test]
    fn power_policy_dedicated_headless_remote_management_prevents_sleep_when_idle() {
        let context = PowerPolicyContext::default()
            .with_host_role(HostRole::DedicatedHeadless)
            .with_remote_management(true);

        let decision = evaluate_power_policy(context);

        assert_eq!(
            decision,
            PowerPolicyDecision::prevent_sleep(PowerPolicyReason::RemoteManagement)
        );
    }

    #[test]
    fn power_policy_normal_desktop_only_prevents_sleep_for_active_work() {
        let idle = evaluate_power_policy(
            PowerPolicyContext::default()
                .with_host_role(HostRole::NormalDesktop)
                .with_remote_management(true),
        );
        assert_eq!(idle, PowerPolicyDecision::allow_sleep());

        let running = evaluate_power_policy(
            PowerPolicyContext::default()
                .with_host_role(HostRole::NormalDesktop)
                .with_running_servers(1),
        );
        assert_eq!(
            running,
            PowerPolicyDecision::prevent_sleep(PowerPolicyReason::ActiveServer { count: 1 })
        );
    }

    #[test]
    fn power_policy_combines_server_and_operation_activity() {
        let decision = evaluate_power_policy(
            PowerPolicyContext::default()
                .with_running_servers(2)
                .with_critical_operations(1),
        );

        assert_eq!(
            decision,
            PowerPolicyDecision::prevent_sleep(
                PowerPolicyReason::ActiveServerAndCriticalOperation {
                    running_servers: 2,
                    critical_operations: 1,
                }
            )
        );
    }

    #[test]
    fn power_policy_controller_activates_and_releases_backend_handles() {
        let backend = FakePowerBackend::default();
        let mut controller = PowerPolicyController::new(&backend);

        let decision = controller
            .update(
                PowerPolicyContext::default()
                    .with_host_role(HostRole::DedicatedHeadless)
                    .with_remote_management(true),
            )
            .expect("activate");
        assert!(decision.prevent_sleep);

        let decision = controller
            .update(PowerPolicyContext::default())
            .expect("deactivate");
        assert!(!decision.prevent_sleep);

        assert_eq!(
            backend.calls(),
            vec![
                "activate:remote management is enabled on a dedicated/headless host".to_string(),
                "deactivate:remote management is enabled on a dedicated/headless host".to_string(),
            ]
        );
    }
}
