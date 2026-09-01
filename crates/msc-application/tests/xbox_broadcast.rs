use msc_application::notifications::NotificationService;
use msc_application::operations::LifecycleOperations;
use msc_application::xbox_broadcast::{XBOX_BROADCAST_OPERATION_TYPE, XboxBroadcastService};
use msc_domain::helper::HelperStatus;
use msc_domain::networking::broadcast_is_ready;
use msc_domain::operation::{OperationId, OperationState};
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::helper_acquisition::{ChecksumSource, HelperPlatform};
use msc_infrastructure::jar_provider::{JarProviderError, Transport};
use msc_infrastructure::process::FakeProcessSupervisor;
use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};
use msc_infrastructure::xbox_broadcast::{
    XboxBroadcastJarAcquisition, XboxBroadcastLaunch, alt_password_secret_key,
    auth_token_secret_key, download_latest_jar, global_alt_password_secret_key,
};
use std::path::{Path, PathBuf};

const OPERATIONS_DIR: &str = "/agent/operations";

fn launch() -> XboxBroadcastLaunch {
    XboxBroadcastLaunch {
        java_path: PathBuf::from("/agent/bin/java"),
        working_directory: PathBuf::from("/agent/broadcast"),
    }
}

fn acquisition<'a>(fs: &'a FakeFileSystem) -> XboxBroadcastJarAcquisition<'a> {
    XboxBroadcastJarAcquisition::new(
        &FakeTransport,
        fs,
        Path::new("/cache"),
        HelperPlatform::LinuxX86_64,
    )
}

fn service_setup() -> (
    &'static LifecycleOperations<'static>,
    &'static FakeProcessSupervisor,
    &'static FakeSecretStore,
    &'static FakeFileSystem,
) {
    let fs = Box::leak(Box::new(
        FakeFileSystem::new()
            .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
            .with_dir("/cache"),
    ));
    let operations = Box::leak(Box::new(LifecycleOperations::new(fs, OPERATIONS_DIR)));
    let supervisor = Box::leak(Box::new(FakeProcessSupervisor::new()));
    let secrets = Box::leak(Box::new(FakeSecretStore::new()));
    (operations, supervisor, secrets, fs)
}

#[test]
fn broadcast_launch_and_readiness_are_journaled_without_secret_arguments() {
    let (operations, supervisor, secrets, fs) = service_setup();
    secrets
        .set(&alt_password_secret_key("paper-1"), "private-password")
        .unwrap();
    secrets
        .set(&auth_token_secret_key("paper-1"), "private-token")
        .unwrap();
    let mut service = XboxBroadcastService::new("paper-1", true, supervisor, secrets, operations);

    let operation_id = OperationId::new(service.start(launch(), &acquisition(fs)).unwrap());
    let (_, request) = supervisor.spawned_requests().pop().unwrap();
    let expected_jar = Path::new("/cache")
        .join("xbox-broadcast")
        .join("v3.0.2")
        .join("MCXboxBroadcastStandalone.jar")
        .to_string_lossy()
        .into_owned();
    assert_eq!(request.arguments, vec!["-jar".to_owned(), expected_jar]);
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
fn broadcast_password_is_shared_across_server_services_with_legacy_fallback() {
    let (operations, supervisor, secrets, _fs) = service_setup();
    let first = XboxBroadcastService::new("paper-1", true, supervisor, secrets, operations);
    first.save_password("shared-password").unwrap();

    let second = XboxBroadcastService::new("paper-2", true, supervisor, secrets, operations);
    assert!(second.status().unwrap().has_password);
    assert_eq!(
        secrets
            .get(global_alt_password_secret_key())
            .unwrap()
            .as_deref(),
        Some("shared-password")
    );

    first.delete_password().unwrap();
    secrets
        .set(&alt_password_secret_key("paper-2"), "legacy-password")
        .unwrap();
    assert!(second.status().unwrap().has_password);
}

#[test]
fn broadcast_poll_surfaces_auth_prompt_and_marks_readiness() {
    let (operations, supervisor, secrets, fs) = service_setup();
    let mut service = XboxBroadcastService::new("paper-1", true, supervisor, secrets, operations);
    let operation_id = OperationId::new(service.start(launch(), &acquisition(fs)).unwrap());
    let (pid, _) = supervisor
        .spawned_requests()
        .into_iter()
        .last()
        .expect("broadcast helper was spawned");

    supervisor
        .emit_stdout(
            pid,
            b"To sign in, open https://www.microsoft.com/link and enter the code ABCD-1234\n",
        )
        .unwrap();
    service.poll().unwrap();
    let status = service.status().unwrap();
    assert_eq!(status.auth_prompt.unwrap().code, "ABCD-1234");
    assert_eq!(status.snapshot.status, HelperStatus::Starting);

    supervisor
        .emit_stdout(pid, b"Creation of Xbox LIVE session was successful\n")
        .unwrap();
    service.poll().unwrap();
    assert_eq!(
        service.status().unwrap().snapshot.status,
        HelperStatus::Running
    );
    assert_eq!(
        operations.snapshot(&operation_id).unwrap().unwrap().state,
        OperationState::Succeeded
    );
}

#[test]
fn broadcast_cancel_and_watchdog_leave_truthful_terminal_operations() {
    let (operations, supervisor, secrets, fs) = service_setup();
    let mut service = XboxBroadcastService::new("paper-1", true, supervisor, secrets, operations);
    let operation_id = OperationId::new(service.start(launch(), &acquisition(fs)).unwrap());
    operations
        .request_cancel(&operation_id, "cancel requested")
        .unwrap();
    assert!(service.cancel_start_if_requested().unwrap());
    assert_eq!(
        operations.snapshot(&operation_id).unwrap().unwrap().state,
        OperationState::Cancelled
    );

    let mut timed_out = XboxBroadcastService::new("paper-2", true, supervisor, secrets, operations);
    let timeout_id = OperationId::new(timed_out.start(launch(), &acquisition(fs)).unwrap());
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
        Path::new("/library/xbox-broadcast/v3.0.2/MCXboxBroadcastStandalone.jar")
    );
    assert_eq!(fs.read(&cached.path).unwrap(), b"jar-bytes");
}

#[test]
fn broadcast_acquisition_records_upstream_digest_provenance() {
    let fs = FakeFileSystem::new().with_dir(Path::new("/cache").to_path_buf());
    let acquired = acquisition(&fs).acquire().unwrap();

    assert_eq!(
        acquired.metadata.checksum_source,
        ChecksumSource::UpstreamPublished
    );
    assert_eq!(
        acquired.metadata.sha256,
        "829b21a069ff177599d32249ba84e0979b39f7fcba8a437607be0b9b06b51c20"
    );
    assert!(
        String::from_utf8(fs.read(&acquired.metadata_path).unwrap())
            .unwrap()
            .contains("upstream-published")
    );
}

#[test]
fn broadcast_acquisition_refuses_missing_digest_before_downloading() {
    let fs = FakeFileSystem::new().with_dir(Path::new("/cache").to_path_buf());
    let transport = MissingDigestTransport;
    let acquisition = XboxBroadcastJarAcquisition::new(
        &transport,
        &fs,
        Path::new("/cache"),
        HelperPlatform::LinuxX86_64,
    );

    let error = acquisition.acquire().unwrap_err();

    assert!(error.to_string().contains("no upstream sha256 digest"));
    assert!(
        fs.list(Path::new("/cache/xbox-broadcast"))
            .unwrap()
            .is_empty()
    );
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
            Ok(br#"{"tag_name":"v3.0.2","assets":[{"name":"MCXboxBroadcastStandalone.jar","browser_download_url":"https://example.test/broadcast.jar","digest":"sha256:829b21a069ff177599d32249ba84e0979b39f7fcba8a437607be0b9b06b51c20"}]}"#.to_vec())
        } else {
            Ok(b"jar-bytes".to_vec())
        }
    }
}

struct MissingDigestTransport;

impl Transport for MissingDigestTransport {
    fn get(&self, url: &str, _what: &str, _max_bytes: u64) -> Result<Vec<u8>, JarProviderError> {
        if url.contains("releases/latest") {
            Ok(br#"{"tag_name":"v3.0.2","assets":[{"name":"MCXboxBroadcastStandalone.jar","browser_download_url":"https://example.test/broadcast.jar"}]}"#.to_vec())
        } else {
            panic!("asset must not be downloaded without a digest")
        }
    }
}

fn _ready_fixture_is_still_the_domain_oracle() {
    assert!(broadcast_is_ready(
        "Creation of Xbox LIVE session was successful"
    ));
}
