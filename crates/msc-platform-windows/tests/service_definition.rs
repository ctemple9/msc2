use msc_infrastructure::service::{
    ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceState,
};
use msc_platform_windows::service::{Sc, WindowsServiceManager};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone)]
struct FakeSc {
    state: Arc<Mutex<FakeScState>>,
}

#[derive(Debug, Default)]
struct FakeScState {
    calls: Vec<String>,
    query_output: BTreeMap<String, String>,
}

impl FakeSc {
    fn set_query_output(&self, service_name: &str, output: &str) {
        self.state
            .lock()
            .unwrap()
            .query_output
            .insert(service_name.to_string(), output.to_string());
    }

    fn calls(&self) -> Vec<String> {
        self.state.lock().unwrap().calls.clone()
    }
}

impl Sc for FakeSc {
    fn create(
        &self,
        service_name: &str,
        bin_path: &str,
        run_user: &str,
        password: Option<&str>,
    ) -> Result<(), msc_infrastructure::service::ServiceError> {
        let password_marker = if password.is_some() { " password" } else { "" };
        self.state.lock().unwrap().calls.push(format!(
            "create {service_name} [{bin_path}] as {run_user}{password_marker}"
        ));
        Ok(())
    }

    fn delete(&self, service_name: &str) -> Result<(), msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("delete {service_name}"));
        Ok(())
    }

    fn start(&self, service_name: &str) -> Result<(), msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("start {service_name}"));
        Ok(())
    }

    fn stop(&self, service_name: &str) -> Result<(), msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("stop {service_name}"));
        Ok(())
    }

    fn query(
        &self,
        service_name: &str,
    ) -> Result<String, msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("query {service_name}"));
        Ok(self
            .state
            .lock()
            .unwrap()
            .query_output
            .get(service_name)
            .cloned()
            .unwrap_or_else(|| {
                "STATE              : 1  STOPPED\nWIN32_EXIT_CODE    : 0  (0x0)\nPID                : 0\n".to_string()
            }))
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "msc2-windows-service-{name}-{}",
            std::process::id()
        ));
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
        "msc2-agent",
        r"C:\MSC2\service-host.ps1",
        r"C:\MSC2\state",
        r"C:\MSC2\logs\agent.log",
        48001,
    )
    .args([
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        r"C:\MSC2\service-host.ps1",
        "-Port",
        "48001",
    ])
    .env("MSC2_TEST_BOOTSTRAP_TOKEN", "secret")
    .run_user(r".\cameron")
}

#[test]
fn install_writes_service_metadata_and_sc_definition() {
    let temp = TempDir::new("install");
    let sc = FakeSc::default();
    let manager = WindowsServiceManager::with_sc(&temp.path, sc.clone());
    let request = request();

    let report = manager
        .execute(ServiceManagerCommand::Install(request.clone()))
        .expect("install succeeds");

    assert_eq!(report.state, ServiceState::Stopped);
    let metadata_path = temp.path.join("msc2-agent.metadata");
    let metadata = std::fs::read_to_string(&metadata_path).expect("metadata exists");
    assert!(metadata.contains("run_user=.\\cameron"));
    assert!(metadata.contains("expected_port=48001"));
    assert_eq!(
        sc.calls(),
        vec![
            "stop msc2-agent".to_string(),
            "delete msc2-agent".to_string(),
        "create msc2-agent [C:\\MSC2\\service-host.ps1 -ExecutionPolicy Bypass -File C:\\MSC2\\service-host.ps1 -Port 48001] as .\\cameron".to_string()
        ]
    );
}

#[test]
fn status_reconstructs_definition_and_running_pid_from_metadata() {
    let temp = TempDir::new("status");
    let sc = FakeSc::default();
    let manager = WindowsServiceManager::with_sc(&temp.path, sc.clone());
    let request = request();

    manager
        .execute(ServiceManagerCommand::Install(request.clone()))
        .expect("install succeeds");
    sc.set_query_output(
        "msc2-agent",
        "STATE              : 4  RUNNING\nWIN32_EXIT_CODE    : 0  (0x0)\nPID                : 4242\n",
    );

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
fn start_stop_and_uninstall_issue_expected_sc_calls() {
    let temp = TempDir::new("lifecycle");
    let sc = FakeSc::default();
    let manager = WindowsServiceManager::with_sc(&temp.path, sc.clone());
    let request = request();

    manager
        .execute(ServiceManagerCommand::Install(request.clone()))
        .expect("install succeeds");
    sc.set_query_output(
        "msc2-agent",
        "STATE              : 4  RUNNING\nWIN32_EXIT_CODE    : 0  (0x0)\nPID                : 777\n",
    );

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

    assert_eq!(
        sc.calls(),
        vec![
            "stop msc2-agent".to_string(),
            "delete msc2-agent".to_string(),
        "create msc2-agent [C:\\MSC2\\service-host.ps1 -ExecutionPolicy Bypass -File C:\\MSC2\\service-host.ps1 -Port 48001] as .\\cameron".to_string(),
            "start msc2-agent".to_string(),
            "query msc2-agent".to_string(),
            "stop msc2-agent".to_string(),
            "stop msc2-agent".to_string(),
            "delete msc2-agent".to_string(),
        ]
    );
    assert!(!temp.path.join("msc2-agent.metadata").exists());
}

#[test]
fn install_rejects_missing_run_user_for_installing_user_service_scope() {
    let temp = TempDir::new("invalid");
    let sc = FakeSc::default();
    let manager = WindowsServiceManager::with_sc(&temp.path, sc);
    let request = ServiceInstallRequest::new(
        "msc2-agent",
        r"C:\MSC2\agent.exe",
        r"C:\MSC2\state",
        r"C:\MSC2\logs\agent.log",
        48001,
    );

    let error = manager
        .execute(ServiceManagerCommand::Install(request))
        .expect_err("missing run_user should fail");
    assert!(
        error
            .to_string()
            .contains("requires run_user so Log On As matches the installing user")
    );
}
