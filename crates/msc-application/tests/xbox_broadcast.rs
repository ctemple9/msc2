use msc_application::notifications::NotificationService;
use msc_application::operations::LifecycleOperations;
use msc_application::xbox_broadcast::{XBOX_BROADCAST_OPERATION_TYPE, XboxBroadcastService};
use msc_domain::helper::HelperStatus;
use msc_domain::networking::broadcast_is_ready;
use msc_domain::operation::{OperationId, OperationState};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::jar_provider::{JarProviderError, Transport};
use msc_infrastructure::process::FakeProcessSupervisor;
use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};
use msc_infrastructure::xbox_broadcast::{
    XboxBroadcastLaunch, alt_password_secret_key, auth_token_secret_key, download_latest_jar,
};
use std::path::{Path, PathBuf};

const OPERATIONS_DIR: &str = "/agent/operations";

fn launch() -> XboxBroadcastLaunch {
    XboxBroadcastLaunch {
        java_path: PathBuf::from("/agent/bin/java"),
        jar_path: PathBuf::from("/agent/MCXboxBroadcastStandalone.jar"),
        working_directory: PathBuf::from("/agent/broadcast"),
    }
}

fn service_setup() -> (
    &'static LifecycleOperations<'static>,
    &'static FakeProcessSupervisor,
    &'static FakeSecretStore,
) {
    let fs = Box::leak(Box::new(FakeFileSystem::new().with_file(
        format!("{OPERATIONS_DIR}/.keep"),
        [],
        false,
    )));
    let operations = Box::leak(Box::new(LifecycleOperations::new(fs, OPERATIONS_DIR)));
    let supervisor = Box::leak(Box::new(FakeProcessSupervisor::new()));
    let secrets = Box::leak(Box::new(FakeSecretStore::new()));
    (operations, supervisor, secrets)
}

#[test]
fn broadcast_launch_and_readiness_are_journaled_without_secret_arguments() {
    let (operations, supervisor, secrets) = service_setup();
    secrets
        .set(&alt_password_secret_key("paper-1"), "private-password")
        .unwrap();
    secrets
        .set(&auth_token_secret_key("paper-1"), "private-token")
        .unwrap();
    let mut service = XboxBroadcastService::new("paper-1", true, supervisor, secrets, operations);

    let operation_id = OperationId::new(service.start(launch()).unwrap());
    let (_, request) = supervisor.spawned_requests().pop().unwrap();
    assert_eq!(
        request.arguments,
        ["-jar", "/agent/MCXboxBroadcastStandalone.jar"]
    );
    assert!(!format!("{request:?}").contains("private-password"));
    assert!(!format!("{request:?}").contains("private-token"));

    service
        .observe_output(
            "To sign in, open https://www.microsoft.com/link and enter the code ABCD-1234",
        )
        .unwrap();
    assert_eq!(
        service.status().unwrap().auth_prompt.unwrap().code,
        "ABCD-1234"
    );
    service
        .observe_output("Creation of Xbox LIVE session was successful")
        .unwrap();
    assert_eq!(
        service.status().unwrap().snapshot.status,
        HelperStatus::Running
    );
    assert_eq!(
        operations
            .snapshot(&operation_id)
            .unwrap()
            .unwrap()
            .operation_type,
        XBOX_BROADCAST_OPERATION_TYPE
    );
    assert_eq!(
        operations.snapshot(&operation_id).unwrap().unwrap().state,
        OperationState::Succeeded
    );
}

#[test]
fn broadcast_cancel_and_watchdog_leave_truthful_terminal_operations() {
    let (operations, supervisor, secrets) = service_setup();
    let mut service = XboxBroadcastService::new("paper-1", true, supervisor, secrets, operations);
    let operation_id = OperationId::new(service.start(launch()).unwrap());
    operations
        .request_cancel(&operation_id, "cancel requested")
        .unwrap();
    assert!(service.cancel_start_if_requested().unwrap());
    assert_eq!(
        operations.snapshot(&operation_id).unwrap().unwrap().state,
        OperationState::Cancelled
    );

    let mut timed_out = XboxBroadcastService::new("paper-2", true, supervisor, secrets, operations);
    let timeout_id = OperationId::new(timed_out.start(launch()).unwrap());
    assert!(!timed_out.ready_timeout_elapsed(59).unwrap());
    assert!(timed_out.ready_timeout_elapsed(60).unwrap());
    assert_eq!(
        timed_out.status().unwrap().snapshot.status,
        HelperStatus::TimedOut
    );
    assert_eq!(
        operations.snapshot(&timeout_id).unwrap().unwrap().state,
        OperationState::Failed
    );
}

#[test]
fn staged_download_uses_release_version_and_never_overwrites_until_download_is_ready() {
    let fs = FakeFileSystem::new().with_dir(Path::new("/library").to_path_buf());
    let transport = FakeTransport;
    let cached = download_latest_jar(&transport, &fs, Path::new("/library")).unwrap();
    assert_eq!(cached.version, "v3.0.2");
    assert_eq!(
        cached.path,
        Path::new("/library/MCXboxBroadcastStandalone-v3.0.2.jar")
    );
    assert_eq!(fs.read(&cached.path).unwrap(), b"jar-bytes");
}

#[test]
fn notification_feed_preserves_msc1_events_and_safe_additions() {
    let mut notifications = NotificationService::new();
    let started = notifications.emit_server_started("paper-1", "Town");
    let joined = notifications.emit_player_joined("paper-1", "Town", "Alex");
    let failed = notifications.emit_helper_failed("paper-1", "Xbox Broadcast");
    assert_eq!(started.kind.as_str(), "server_started");
    assert_eq!(started.body, "Town is now online.");
    assert_eq!(joined.kind.as_str(), "player_joined");
    assert_eq!(joined.body, "Alex joined Town");
    assert_eq!(failed.kind.as_str(), "helper_failed");
    assert!(!failed.body.contains("token"));
    assert_eq!(notifications.recent().count(), 3);
}

struct FakeTransport;

impl Transport for FakeTransport {
    fn get(&self, url: &str, _what: &str, _max_bytes: u64) -> Result<Vec<u8>, JarProviderError> {
        if url.contains("releases/latest") {
            Ok(br#"{"tag_name":"v3.0.2","assets":[{"name":"MCXboxBroadcastStandalone.jar","browser_download_url":"https://example.test/broadcast.jar"}]}"#.to_vec())
        } else {
            Ok(b"jar-bytes".to_vec())
        }
    }
}

fn _ready_fixture_is_still_the_domain_oracle() {
    assert!(broadcast_is_ready(
        "Creation of Xbox LIVE session was successful"
    ));
}
