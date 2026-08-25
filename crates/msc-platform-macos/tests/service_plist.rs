#![cfg(target_os = "macos")]

use msc_infrastructure::service::{
    ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceState,
};
use msc_platform_macos::service::{Launchctl, MacosLaunchdServiceManager};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone)]
struct FakeLaunchctl {
    state: Arc<Mutex<FakeLaunchctlState>>,
}

#[derive(Debug, Default)]
struct FakeLaunchctlState {
    calls: Vec<String>,
    print_output: BTreeMap<String, String>,
}

impl FakeLaunchctl {
    fn set_print_output(&self, target: &str, output: &str) {
        self.state
            .lock()
            .unwrap()
            .print_output
            .insert(target.to_string(), output.to_string());
    }

    fn calls(&self) -> Vec<String> {
        self.state.lock().unwrap().calls.clone()
    }
}

impl Launchctl for FakeLaunchctl {
    fn bootstrap(
        &self,
        plist_path: &Path,
    ) -> Result<(), msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("bootstrap {}", plist_path.display()));
        Ok(())
    }

    fn bootout(&self, plist_path: &Path) -> Result<(), msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("bootout {}", plist_path.display()));
        Ok(())
    }

    fn start(&self, label: &str) -> Result<(), msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("start {label}"));
        Ok(())
    }

    fn stop(&self, label: &str) -> Result<(), msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("stop {label}"));
        Ok(())
    }

    fn print(
        &self,
        service_target: &str,
    ) -> Result<String, msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("print {service_target}"));
        Ok(self
            .state
            .lock()
            .unwrap()
            .print_output
            .get(service_target)
            .cloned()
            .unwrap_or_else(|| "state = waiting\n".to_string()))
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("msc2-service-plist-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn request() -> ServiceInstallRequest {
    ServiceInstallRequest::new(
        "com.msc2.agent",
        "/usr/local/bin/msc",
        "/private/tmp/msc2-service-tests/agent",
        "/private/tmp/msc2-service-tests/agent.log",
        48001,
    )
    .args(["serve", "--bind", "127.0.0.1:48001"])
    .env("MSC2_TEST_BOOTSTRAP_TOKEN", "secret")
    .run_user("cameron")
}

#[test]
fn install_writes_launchdaemon_plist_with_username_and_expected_port_metadata() {
    let temp = TempDir::new("install");
    let launchctl = FakeLaunchctl::default();
    let manager = MacosLaunchdServiceManager::with_launchctl(&temp.path, launchctl.clone());
    let request = request();

    let report = manager
        .execute(ServiceManagerCommand::Install(request.clone()))
        .expect("install succeeds");

    assert_eq!(report.state, ServiceState::Stopped);
    let plist_path = temp.path.join("com.msc2.agent.plist");
    let plist = std::fs::read_to_string(&plist_path).expect("plist exists");
    assert!(plist.contains("<key>UserName</key>"));
    assert!(plist.contains("<string>cameron</string>"));
    assert!(plist.contains("<key>MSC2_EXPECTED_PORT</key>"));
    assert!(plist.contains("<string>48001</string>"));
    assert_eq!(
        launchctl.calls(),
        vec![format!("bootstrap {}", plist_path.display())]
    );
}

#[test]
fn status_reconstructs_definition_and_running_pid_from_installed_plist() {
    let temp = TempDir::new("status");
    let launchctl = FakeLaunchctl::default();
    let manager = MacosLaunchdServiceManager::with_launchctl(&temp.path, launchctl.clone());
    let request = request();
    let target = "system/com.msc2.agent";

    manager
        .execute(ServiceManagerCommand::Install(request.clone()))
        .expect("install succeeds");
    launchctl.set_print_output(target, "state = running\npid = 4242\n");

    let report = manager
        .execute(ServiceManagerCommand::Status {
            service_name: request.service_name.clone(),
        })
        .expect("status succeeds");

    assert_eq!(report.state, ServiceState::Running);
    assert_eq!(report.pid, Some(4242));
    let definition = report.definition.expect("status includes definition");
    assert_eq!(definition.binary_path, request.binary_path);
    assert_eq!(definition.arguments, request.arguments);
    assert_eq!(definition.run_user, request.run_user);
    assert_eq!(definition.expected_port, request.expected_port);
    assert_eq!(definition.environment, request.environment);
}

#[test]
fn start_stop_and_uninstall_issue_expected_launchctl_calls() {
    let temp = TempDir::new("lifecycle");
    let launchctl = FakeLaunchctl::default();
    let manager = MacosLaunchdServiceManager::with_launchctl(&temp.path, launchctl.clone());
    let request = request();
    let target = "system/com.msc2.agent";

    manager
        .execute(ServiceManagerCommand::Install(request.clone()))
        .expect("install succeeds");
    launchctl.set_print_output(target, "state = running\npid = 777\n");

    let started = manager
        .execute(ServiceManagerCommand::Start {
            service_name: request.service_name.clone(),
        })
        .expect("start succeeds");
    assert_eq!(started.state, ServiceState::Running);
    assert_eq!(started.pid, Some(777));

    let stopped = manager
        .execute(ServiceManagerCommand::Stop {
            service_name: request.service_name.clone(),
        })
        .expect("stop succeeds");
    assert_eq!(stopped.state, ServiceState::Stopped);

    let removed = manager
        .execute(ServiceManagerCommand::Uninstall {
            service_name: request.service_name.clone(),
        })
        .expect("uninstall succeeds");
    assert_eq!(removed.state, ServiceState::NotInstalled);

    let plist_path = temp.path.join("com.msc2.agent.plist");
    assert_eq!(
        launchctl.calls(),
        vec![
            format!("bootstrap {}", plist_path.display()),
            "start com.msc2.agent".to_string(),
            "print system/com.msc2.agent".to_string(),
            "stop com.msc2.agent".to_string(),
            format!("bootout {}", plist_path.display()),
        ]
    );
    assert!(!plist_path.exists());
}

#[test]
fn install_rejects_missing_run_user_for_launchdaemon_scope() {
    let temp = TempDir::new("invalid");
    let launchctl = FakeLaunchctl::default();
    let manager = MacosLaunchdServiceManager::with_launchctl(&temp.path, launchctl);
    let request = ServiceInstallRequest::new(
        "com.msc2.agent",
        "/usr/local/bin/msc",
        "/tmp",
        "/tmp/msc.log",
        48001,
    );

    let error = manager
        .execute(ServiceManagerCommand::Install(request))
        .expect_err("missing run_user should fail");
    assert!(
        error
            .to_string()
            .contains("requires run_user so UserName matches the installing user")
    );
}
