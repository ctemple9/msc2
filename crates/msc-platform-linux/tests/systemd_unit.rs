#![cfg(unix)]

use msc_infrastructure::service::{
    ServiceInstallRequest, ServiceManager, ServiceManagerCommand, ServiceState,
};
use msc_platform_linux::credential_helper::{
    CredentialHelperInstall, HelperOperation, HelperRequest, HelperResponse, parse_request_line,
    serialize_response,
};
use msc_platform_linux::service::{LinuxSystemdServiceManager, Systemctl};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone)]
struct FakeSystemctl {
    state: Arc<Mutex<FakeSystemctlState>>,
}

#[derive(Debug, Default)]
struct FakeSystemctlState {
    calls: Vec<String>,
    show_output: BTreeMap<String, String>,
}

impl FakeSystemctl {
    fn set_show_output(&self, unit_name: &str, output: &str) {
        self.state
            .lock()
            .unwrap()
            .show_output
            .insert(unit_name.to_string(), output.to_string());
    }

    fn calls(&self) -> Vec<String> {
        self.state.lock().unwrap().calls.clone()
    }
}

impl Systemctl for FakeSystemctl {
    fn daemon_reload(&self) -> Result<(), msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push("daemon-reload".to_string());
        Ok(())
    }

    fn enable(&self, unit_name: &str) -> Result<(), msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("enable {unit_name}"));
        Ok(())
    }

    fn disable(&self, unit_name: &str) -> Result<(), msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("disable {unit_name}"));
        Ok(())
    }

    fn start(&self, unit_name: &str) -> Result<(), msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("start {unit_name}"));
        Ok(())
    }

    fn stop(&self, unit_name: &str) -> Result<(), msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("stop {unit_name}"));
        Ok(())
    }

    fn show(&self, unit_name: &str) -> Result<String, msc_infrastructure::service::ServiceError> {
        self.state
            .lock()
            .unwrap()
            .calls
            .push(format!("show {unit_name}"));
        Ok(self
            .state
            .lock()
            .unwrap()
            .show_output
            .get(unit_name)
            .cloned()
            .unwrap_or_else(|| "ActiveState=inactive\nMainPID=0\n".to_string()))
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("msc2-systemd-unit-{name}-{}", std::process::id()));
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
        "/usr/local/bin/msc",
        "/tmp/msc2-service-tests/agent",
        "/tmp/msc2-service-tests/agent.log",
        48001,
    )
    .args(["serve", "--bind", "127.0.0.1:48001"])
    .env("MSC2_TEST_BOOTSTRAP_TOKEN", "secret")
    .run_user("cameron")
}

#[test]
fn install_writes_systemd_unit_with_user_group_and_expected_port_metadata() {
    let temp = TempDir::new("install");
    let systemctl = FakeSystemctl::default();
    let manager = LinuxSystemdServiceManager::with_systemctl(&temp.path, systemctl.clone());
    let request = request();

    let report = manager
        .execute(ServiceManagerCommand::Install(request.clone()))
        .expect("install succeeds");

    assert_eq!(report.state, ServiceState::Stopped);
    let unit_path = temp.path.join("msc2-agent.service");
    let unit = std::fs::read_to_string(&unit_path).expect("unit exists");
    assert!(unit.contains("User=cameron"));
    assert!(unit.contains("Group=cameron"));
    assert!(unit.contains("Environment=\"MSC2_EXPECTED_PORT=48001\""));
    assert_eq!(
        systemctl.calls(),
        vec![
            "daemon-reload".to_string(),
            "enable msc2-agent.service".to_string()
        ]
    );
}

#[test]
fn status_reconstructs_definition_and_running_pid_from_installed_unit() {
    let temp = TempDir::new("status");
    let systemctl = FakeSystemctl::default();
    let manager = LinuxSystemdServiceManager::with_systemctl(&temp.path, systemctl.clone());
    let request = request();

    manager
        .execute(ServiceManagerCommand::Install(request.clone()))
        .expect("install succeeds");
    systemctl.set_show_output("msc2-agent.service", "ActiveState=active\nMainPID=4242\n");

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
fn start_stop_and_uninstall_issue_expected_systemctl_calls() {
    let temp = TempDir::new("lifecycle");
    let systemctl = FakeSystemctl::default();
    let manager = LinuxSystemdServiceManager::with_systemctl(&temp.path, systemctl.clone());
    let request = request();

    manager
        .execute(ServiceManagerCommand::Install(request.clone()))
        .expect("install succeeds");
    systemctl.set_show_output("msc2-agent.service", "ActiveState=active\nMainPID=777\n");

    let started = manager
        .execute(ServiceManagerCommand::Start {
            service_name: request.service_name.clone(),
        })
        .expect("start succeeds");
    assert_eq!(started.state, ServiceState::Running);
    assert_eq!(started.pid, Some(777));

    systemctl.set_show_output("msc2-agent.service", "ActiveState=inactive\nMainPID=0\n");
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

    let unit_path = temp.path.join("msc2-agent.service");
    assert_eq!(
        systemctl.calls(),
        vec![
            "daemon-reload".to_string(),
            "enable msc2-agent.service".to_string(),
            "start msc2-agent.service".to_string(),
            "show msc2-agent.service".to_string(),
            "stop msc2-agent.service".to_string(),
            "show msc2-agent.service".to_string(),
            "stop msc2-agent.service".to_string(),
            "disable msc2-agent.service".to_string(),
            "daemon-reload".to_string(),
        ]
    );
    assert!(!unit_path.exists());
}

#[test]
fn install_rejects_missing_run_user_for_installing_user_service_scope() {
    let temp = TempDir::new("invalid");
    let systemctl = FakeSystemctl::default();
    let manager = LinuxSystemdServiceManager::with_systemctl(&temp.path, systemctl);
    let request = ServiceInstallRequest::new(
        "msc2-agent",
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
            .contains("requires run_user so User= and Group= match the installing user")
    );
}

#[test]
fn credential_helper_units_render_expected_socket_permissions_and_execstart() {
    let install = CredentialHelperInstall::new("/usr/local/bin/msc", 501, "cameron", "staff")
        .socket_path("/run/msc2/credential-helper.sock")
        .store_dir("/var/lib/msc2/credentials");

    let socket_unit = install.render_socket_unit().expect("socket unit renders");
    assert!(socket_unit.contains("ListenStream=/run/msc2/credential-helper.sock"));
    assert!(socket_unit.contains("SocketUser=cameron"));
    assert!(socket_unit.contains("SocketGroup=staff"));
    assert!(socket_unit.contains("SocketMode=0600"));

    let service_unit = install.render_service_unit().expect("service unit renders");
    assert!(service_unit.contains("User=root"));
    assert!(service_unit.contains("Group=root"));
    assert!(service_unit.contains("credential-helper serve --allowed-uid 501"));
    assert!(service_unit.contains("--store-dir '/var/lib/msc2/credentials'"));
    assert!(!service_unit.contains("StandardInput=socket"));
}

#[test]
fn credential_helper_rejects_binary_path_hidden_by_its_own_hardening() {
    for hidden_path in [
        "/tmp/msc2-run/bin/msc",
        "/var/tmp/msc2-run/bin/msc",
        "/home/cameron/.cargo/bin/msc",
        "/root/.cargo/bin/msc",
        "/run/user/1000/msc",
    ] {
        let install = CredentialHelperInstall::new(hidden_path, 501, "cameron", "staff");
        let error = install
            .render_service_unit()
            .expect_err("binary path hidden by PrivateTmp=yes/ProtectHome=yes must be rejected");
        assert!(error.contains(hidden_path), "unexpected error: {error}");
    }
}

#[test]
fn credential_helper_protocol_accepts_valid_requests_and_serializes_responses() {
    let request = parse_request_line(
        r#"{"version":1,"op":"set","key":"remote-api.token.example","value":"abc"}"#,
    )
    .expect("request parses");
    assert_eq!(
        request,
        HelperRequest {
            version: 1,
            op: HelperOperation::Set,
            key: Some("remote-api.token.example".to_string()),
            value: Some("abc".to_string()),
        }
    );

    let response = serialize_response(&HelperResponse::ok_with_value(Some("value".to_string())))
        .expect("response serializes");
    assert_eq!(response, r#"{"ok":true,"value":"value"}"#);
}

#[test]
fn credential_helper_protocol_rejects_invalid_keys_and_oversized_values() {
    let error = parse_request_line(r#"{"version":1,"op":"get","key":"../bad"}"#)
        .expect_err("invalid key should fail");
    assert!(error.contains("credential key is not allowed"));

    let oversized = "x".repeat(32 * 1024 + 1);
    let error = parse_request_line(&format!(
        r#"{{"version":1,"op":"set","key":"remote-api.token.example","value":"{oversized}"}}"#
    ))
    .expect_err("oversized value should fail");
    assert!(error.contains("value exceeds"));
}
