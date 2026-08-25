use msc_infrastructure::service::{
    FakeServiceManager, ServiceError, ServiceInstallRequest, ServiceManager, ServiceManagerCommand,
    ServiceName, ServiceState, ServiceStatusReport,
};

fn request() -> ServiceInstallRequest {
    ServiceInstallRequest::new(
        "msc-agent",
        "/usr/local/bin/msc",
        "/Users/cameron/Library/Application Support/MSC2/agent",
        "/Users/cameron/Library/Logs/MSC2/agent.log",
        48001,
    )
    .args(["serve", "--bind", "127.0.0.1:48001"])
    .env("RUST_LOG", "info")
    .env("MSC2_CONFIG_DIR", "/Users/cameron/.msc2")
    .run_user("cameron")
}

#[test]
fn service_model_install_request_carries_platform_definition_fields() {
    let request = request();

    assert_eq!(request.service_name.as_str(), "msc-agent");
    assert_eq!(request.binary_path.to_string_lossy(), "/usr/local/bin/msc");
    assert_eq!(
        request.working_directory.to_string_lossy(),
        "/Users/cameron/Library/Application Support/MSC2/agent"
    );
    assert_eq!(
        request.log_path.to_string_lossy(),
        "/Users/cameron/Library/Logs/MSC2/agent.log"
    );
    assert_eq!(request.expected_port, 48001);
    assert_eq!(request.run_user.as_deref(), Some("cameron"));
    assert_eq!(
        request.arguments,
        vec!["serve", "--bind", "127.0.0.1:48001"]
    );
    assert_eq!(
        request.environment.get("RUST_LOG").map(String::as_str),
        Some("info")
    );
    assert_eq!(
        request
            .environment
            .get("MSC2_CONFIG_DIR")
            .map(String::as_str),
        Some("/Users/cameron/.msc2")
    );
}

#[test]
fn service_model_status_report_helpers_preserve_definition_details() {
    let request = request();
    let report = ServiceStatusReport::running(request.clone(), 4242);

    assert_eq!(report.service_name, request.service_name);
    assert_eq!(report.state, ServiceState::Running);
    assert_eq!(report.pid, Some(4242));
    assert_eq!(report.definition, Some(request));
}

#[test]
fn service_model_fake_manager_tracks_install_start_stop_and_status() {
    let manager = FakeServiceManager::new();
    let definition = request();

    let installed = manager
        .execute(ServiceManagerCommand::Install(definition.clone()))
        .expect("install");
    assert_eq!(installed.state, ServiceState::Stopped);

    let running = manager
        .execute(ServiceManagerCommand::Start {
            service_name: definition.service_name.clone(),
        })
        .expect("start");
    assert_eq!(running.state, ServiceState::Running);
    assert!(running.pid.is_some());

    let status = manager
        .execute(ServiceManagerCommand::Status {
            service_name: definition.service_name.clone(),
        })
        .expect("status");
    assert_eq!(status.state, ServiceState::Running);
    assert_eq!(status.pid, running.pid);

    let stopped = manager
        .execute(ServiceManagerCommand::Stop {
            service_name: definition.service_name.clone(),
        })
        .expect("stop");
    assert_eq!(stopped.state, ServiceState::Stopped);
    assert_eq!(stopped.pid, None);

    let commands = manager.commands();
    assert_eq!(commands.len(), 4);
    assert_eq!(commands[0], ServiceManagerCommand::Install(definition));
}

#[test]
fn service_model_fake_manager_reports_not_installed_and_injected_failures() {
    let manager = FakeServiceManager::new();

    let status = manager
        .execute(ServiceManagerCommand::Status {
            service_name: ServiceName::new("missing"),
        })
        .expect("status");
    assert_eq!(status.state, ServiceState::NotInstalled);
    assert_eq!(status.definition, None);

    manager.fail_next(ServiceError::Unsupported(
        "launchd adapter is not wired yet".to_string(),
    ));
    let error = manager
        .execute(ServiceManagerCommand::Install(request()))
        .expect_err("fail_next should win");
    assert_eq!(
        error,
        ServiceError::Unsupported("launchd adapter is not wired yet".to_string())
    );
}
