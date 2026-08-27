#[allow(dead_code)]
#[path = "../src/cli/mod.rs"]
mod cli;

use msc_infrastructure::service::{FakeServiceManager, ServiceManagerCommand};

fn common() -> cli::CommonArgs {
    cli::CommonArgs {
        base_url: None,
        host: "127.0.0.1".to_string(),
        port: 48001,
        token: None,
        json: false,
    }
}

fn target() -> cli::service::ServiceTargetArgs {
    cli::service::ServiceTargetArgs {
        service_name: "msc-agent".to_string(),
    }
}

#[tokio::test]
async fn service_commands_execute_through_the_manager() {
    let manager = FakeServiceManager::new();

    cli::service::run_with_manager(
        common(),
        cli::service::ServiceCommand::Status(target()),
        &manager,
    )
    .await
    .expect("status executes through the injected manager");
    cli::service::run_with_manager(
        common(),
        cli::service::ServiceCommand::Install(cli::service::ServiceInstallArgs {
            service_name: "msc-agent".to_string(),
            binary_path: "/opt/msc/msc".to_string(),
            working_directory: "/var/lib/msc".to_string(),
            log_path: "/var/log/msc/agent.log".to_string(),
            run_user: Some("msc".to_string()),
            expected_port: 48001,
            arguments: vec!["serve".to_string()],
            environment: vec!["MSC2_DATA_DIR=/var/lib/msc".to_string()],
        }),
        &manager,
    )
    .await
    .expect("install executes through the injected manager");
    cli::service::run_with_manager(
        common(),
        cli::service::ServiceCommand::Start(target()),
        &manager,
    )
    .await
    .expect("start executes through the injected manager");
    cli::service::run_with_manager(
        common(),
        cli::service::ServiceCommand::Stop(target()),
        &manager,
    )
    .await
    .expect("stop executes through the injected manager");
    cli::service::run_with_manager(
        common(),
        cli::service::ServiceCommand::Uninstall(target()),
        &manager,
    )
    .await
    .expect("uninstall executes through the injected manager");

    assert!(matches!(
        manager.commands().as_slice(),
        [
            ServiceManagerCommand::Status { .. },
            ServiceManagerCommand::Install(_),
            ServiceManagerCommand::Start { .. },
            ServiceManagerCommand::Stop { .. },
            ServiceManagerCommand::Uninstall { .. },
        ]
    ));
}
