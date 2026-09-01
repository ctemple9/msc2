use msc_application::operations::LifecycleOperations;
use msc_application::playit::{
    PLAYIT_OPERATION_TYPE, PlayitError, PlayitLifecycleStatus, PlayitService,
};
use msc_domain::helper::HelperStatus;
use msc_domain::networking::{PlayitTunnelKind, PlayitTunnelSpec};
use msc_domain::operation::OperationState;
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::helper_acquisition::{
    HelperPlatform, PinnedHelperAsset, PinnedHelperRelease,
};
use msc_infrastructure::jar_provider::{JarProviderError, Transport};
use msc_infrastructure::playit::{PLAYIT_SECRET_KEY, PlayitBinaryAcquisition, PlayitLaunch};
use msc_infrastructure::playit_api::{
    PlayitHttpResponse, PlayitHttpTransport, PlayitTransportError,
};
use msc_infrastructure::process::FakeProcessSupervisor;
use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};
use serde_json::Value;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const OPERATIONS_DIR: &str = "/agent/operations";

fn launch() -> PlayitLaunch {
    let working_directory =
        std::env::temp_dir().join(format!("msc2-playit-test-{}", uuid::Uuid::new_v4()));
    PlayitLaunch::managed(working_directory)
}

const RELEASE_URL: &str =
    "https://api.github.com/repos/example/playitd/releases/tags/playitd-v1.0.10";
const ASSET_URL: &str =
    "https://github.com/example/playitd/releases/download/playitd-v1.0.10/playitd-linux-x86_64";
const ASSET_BYTES: &[u8] = b"pinned playitd bytes";
const ASSET_SHA256: &str = "6d220b9914cafaccc949e466ec5935dea79ef413d0655cf14bd24baba58805f2";

struct FakeTransport {
    responses: Mutex<HashMap<String, Vec<u8>>>,
}

impl FakeTransport {
    fn with_playit_fixture() -> Self {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/networking/helper-acquisition-pinned-release.json");
        let metadata = std::fs::read(fixture).expect("helper acquisition fixture");
        Self {
            responses: Mutex::new(HashMap::from([
                (RELEASE_URL.into(), metadata),
                (ASSET_URL.into(), ASSET_BYTES.to_vec()),
            ])),
        }
    }
}

impl Transport for FakeTransport {
    fn get(&self, url: &str, what: &str, max_bytes: u64) -> Result<Vec<u8>, JarProviderError> {
        let bytes = self
            .responses
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or_else(|| {
                JarProviderError::Network(format!("{what}: no fake response for {url}"))
            })?;
        if bytes.len() as u64 > max_bytes {
            return Err(JarProviderError::ResponseTooLarge {
                what: what.into(),
                max_bytes,
            });
        }
        Ok(bytes)
    }
}

fn acquisition<'a>(
    transport: &'a FakeTransport,
    fs: &'a FakeFileSystem,
) -> PlayitBinaryAcquisition<'a> {
    PlayitBinaryAcquisition::new(
        transport,
        fs,
        Path::new("/cache"),
        PinnedHelperRelease {
            helper: "playitd".into(),
            version: "playitd-v1.0.10".into(),
            release_metadata_url: RELEASE_URL.into(),
            assets: vec![PinnedHelperAsset {
                platform: HelperPlatform::LinuxX86_64,
                asset_name: "playitd-linux-x86_64".into(),
                sha256: ASSET_SHA256.into(),
            }],
        },
        HelperPlatform::LinuxX86_64,
    )
}

struct FakeAccountTransport {
    responses: Mutex<VecDeque<PlayitHttpResponse>>,
    requests: Mutex<Vec<(String, Value, Option<String>)>>,
}

impl FakeAccountTransport {
    fn new(responses: impl IntoIterator<Item = (u16, Value)>) -> Self {
        Self {
            responses: Mutex::new(
                responses
                    .into_iter()
                    .map(|(status, value)| PlayitHttpResponse {
                        status,
                        body: serde_json::to_vec(&value).unwrap(),
                    })
                    .collect(),
            ),
            requests: Mutex::new(Vec::new()),
        }
    }
}

impl PlayitHttpTransport for FakeAccountTransport {
    fn post_json(
        &self,
        path: &str,
        body: &Value,
        authorization: Option<&str>,
    ) -> Result<PlayitHttpResponse, PlayitTransportError> {
        self.requests.lock().unwrap().push((
            path.to_owned(),
            body.clone(),
            authorization.map(str::to_owned),
        ));
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or(PlayitTransportError::Network)
    }
}

#[test]
fn playit_start_is_journaled_and_secret_never_reaches_process_arguments() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let transport = FakeTransport::with_playit_fixture();
    let binary = acquisition(&transport, &fs);
    let secrets = FakeSecretStore::new();
    secrets
        .set(PLAYIT_SECRET_KEY, "playit-secret-must-not-leak")
        .unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);

    let launch = launch();
    let bridge_path = launch.secret_path.clone();
    let socket_path = launch.socket_path.clone();
    let started = service.start(launch, &binary).unwrap();
    let operation_id = started.operation_id.unwrap();
    assert_eq!(started.status, HelperStatus::Starting);
    assert_eq!(
        std::fs::read_to_string(&bridge_path).unwrap(),
        "playit-secret-must-not-leak\n"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&bridge_path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    assert_eq!(
        operations
            .snapshot(&msc_domain::operation::OperationId::new(operation_id))
            .unwrap()
            .unwrap()
            .operation_type,
        PLAYIT_OPERATION_TYPE
    );

    let (_, request) = supervisor.spawned_requests().pop().unwrap();
    let mut expected_arguments = vec![
        "--secret-path".to_owned(),
        bridge_path.to_string_lossy().into_owned(),
    ];
    if let Some(socket_path) = socket_path {
        expected_arguments.extend([
            "--socket-path".to_owned(),
            socket_path.to_string_lossy().into_owned(),
        ]);
    }
    assert_eq!(request.arguments, expected_arguments);
    assert_eq!(
        request.executable_path,
        PathBuf::from("/cache/playitd/playitd-v1.0.10/playitd-linux-x86_64")
    );
    assert!(!format!("{request:?}").contains("playit-secret-must-not-leak"));
    let operation_path = fs
        .list(Path::new(OPERATIONS_DIR))
        .unwrap()
        .into_iter()
        .find(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .expect("start operation");
    assert!(
        !String::from_utf8_lossy(&fs.read(&operation_path).unwrap())
            .contains("playit-secret-must-not-leak")
    );
}

#[cfg(unix)]
#[test]
fn playit_start_removes_a_stale_server_scoped_socket() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let transport = FakeTransport::with_playit_fixture();
    let binary = acquisition(&transport, &fs);
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);
    // macOS Unix-domain sockets have a 104-byte pathname limit. The normal
    // test temp directory can exceed it, so keep this real-socket fixture
    // directly below /tmp while preserving the production path shape.
    let launch = PlayitLaunch::managed(
        PathBuf::from("/tmp").join(format!("msc2-playit-test-{}", uuid::Uuid::new_v4())),
    );
    let socket_path = launch.socket_path.clone().expect("Unix socket path");
    std::fs::create_dir_all(&launch.working_directory).unwrap();
    let listener = match std::os::unix::net::UnixListener::bind(&socket_path) {
        Ok(listener) => listener,
        // The macOS application sandbox used by the focused test runner can
        // prohibit Unix sockets altogether. Production and ordinary CI both
        // exercise the assertion below; the restricted runner cannot create
        // the fixture it needs.
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return,
        Err(error) => panic!("create stale Playit socket fixture: {error}"),
    };
    drop(listener);
    assert!(socket_path.exists());

    service.start(launch, &binary).unwrap();

    assert!(!socket_path.exists());
}

#[test]
fn playit_lifecycle_distinguishes_setup_and_supervised_states() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let secrets = FakeSecretStore::new();
    let service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);

    assert_eq!(
        service.lifecycle_status(),
        PlayitLifecycleStatus::SetupRequired
    );
    assert_eq!(PlayitLifecycleStatus::Starting.label(), "Starting");
    assert_eq!(
        PlayitLifecycleStatus::WaitingForTunnels.label(),
        "Waiting for tunnels"
    );
    assert_eq!(PlayitLifecycleStatus::Running.label(), "Running");
    assert_eq!(PlayitLifecycleStatus::Stopping.label(), "Stopping");
    assert_eq!(PlayitLifecycleStatus::Stopped.label(), "Stopped");
    assert_eq!(PlayitLifecycleStatus::TimedOut.label(), "Timed out");
    assert_eq!(
        PlayitLifecycleStatus::Failed {
            message: "failed".into()
        }
        .label(),
        "Failed"
    );
}

#[test]
fn playit_output_pump_redacts_diagnostics_and_reconciles_exit() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let transport = FakeTransport::with_playit_fixture();
    let binary = acquisition(&transport, &fs);
    let secrets = FakeSecretStore::new();
    let secret = "playit-secret-must-not-leak";
    secrets.set(PLAYIT_SECRET_KEY, secret).unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);
    let launch = launch();
    let bridge_path = launch.secret_path.clone();
    let started = service.start(launch, &binary).unwrap();
    let operation_id = msc_domain::operation::OperationId::new(started.operation_id.unwrap());
    let (pid, _request) = supervisor.spawned_requests().pop().unwrap();

    supervisor
        .emit_stderr(pid, format!("provider output contains {secret}\n"))
        .unwrap();
    supervisor.emit_stdout(pid, b"tunnel setup\n").unwrap();
    supervisor
        .emit_stdout(pid, b"join.example.joinmc.link\n")
        .unwrap();
    service.poll().unwrap();

    assert!(bridge_path.exists());
    assert_eq!(service.lifecycle_status(), PlayitLifecycleStatus::Running);
    assert!(
        service
            .diagnostics()
            .iter()
            .all(|diagnostic| !diagnostic.line.contains(secret))
    );
    assert!(
        service
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.line.contains("<redacted>"))
    );
    assert_eq!(
        operations.snapshot(&operation_id).unwrap().unwrap().state,
        OperationState::Succeeded
    );
    assert!(!format!("{:?}", operations.snapshot(&operation_id).unwrap()).contains(secret));

    supervisor.exit_normally(pid).unwrap();
    service.poll().unwrap();
    assert!(!bridge_path.exists());
    assert_eq!(service.lifecycle_status(), PlayitLifecycleStatus::Stopped);
}

#[test]
fn playit_recognizes_its_connected_agent_before_any_tunnel_address_exists() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let transport = FakeTransport::with_playit_fixture();
    let binary = acquisition(&transport, &fs);
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);

    service.start(launch(), &binary).unwrap();
    let pid = supervisor.spawned_requests().pop().unwrap().0;
    supervisor
        .emit_stdout(
            pid,
            b"INFO playitd::daemon: playit connected; tunnels loaded agent_id=6af10e70-44cc-4b60-b8f7-f6949ee4f999 tunnel_count=0\n",
        )
        .unwrap();
    service.poll().unwrap();

    assert!(service.agent_connection_matches("6af10e70-44cc-4b60-b8f7-f6949ee4f999"));
    assert!(!service.agent_connection_matches("6af10e70-44cc-4b60-b8f7-f6949ee4f998"));
    assert!(service.status().player_address.is_none());
}

#[test]
fn playit_stop_hard_stops_the_daemon_and_journals_completion() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let transport = FakeTransport::with_playit_fixture();
    let binary = acquisition(&transport, &fs);
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);
    let launch = launch();
    let bridge_path = launch.secret_path.clone();
    service.start(launch, &binary).unwrap();
    let pid = supervisor.spawned_requests().pop().unwrap().0;
    supervisor.emit_stdout(pid, b"tunnel setup\n").unwrap();
    supervisor
        .emit_stdout(pid, b"join.example.joinmc.link\n")
        .unwrap();
    service.poll().unwrap();

    let stop_operation = service
        .stop()
        .unwrap()
        .expect("running helper stop operation");
    assert_eq!(
        service.stop().unwrap().as_deref(),
        Some(stop_operation.as_str())
    );
    let stop_operation_id = msc_domain::operation::OperationId::new(stop_operation);
    assert_eq!(service.lifecycle_status(), PlayitLifecycleStatus::Stopping);
    assert!(supervisor.graceful_stops().is_empty());
    assert_eq!(supervisor.force_terminations(), vec![pid]);
    assert_eq!(
        operations
            .snapshot(&stop_operation_id)
            .unwrap()
            .unwrap()
            .state,
        OperationState::Running
    );
    assert!(bridge_path.exists());

    service.poll().unwrap();
    assert_eq!(service.lifecycle_status(), PlayitLifecycleStatus::Stopped);
    assert_eq!(
        operations
            .snapshot(&stop_operation_id)
            .unwrap()
            .unwrap()
            .state,
        OperationState::Succeeded
    );
    assert!(!bridge_path.exists());
}

#[test]
fn playit_reset_reconciles_helper_before_removing_bridge() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let transport = FakeTransport::with_playit_fixture();
    let binary = acquisition(&transport, &fs);
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);
    let first_launch = launch();
    let bridge_path = first_launch.secret_path.clone();

    service.start(first_launch, &binary).unwrap();
    let pid = supervisor.spawned_requests().pop().unwrap().0;
    service.reset().unwrap();

    assert_eq!(supervisor.graceful_stops(), vec![pid]);
    assert_eq!(supervisor.force_terminations(), vec![pid]);
    assert!(!bridge_path.exists());
    assert_eq!(
        service.lifecycle_status(),
        PlayitLifecycleStatus::SetupRequired
    );

    // The manager has observed the exit, so a fresh setup/start can create a
    // new supervised helper instead of colliding with the old one.
    service.start(launch(), &binary).unwrap();
    assert_eq!(supervisor.spawned_requests().len(), 2);
}

#[test]
fn playit_can_start_again_after_a_clean_stop() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let transport = FakeTransport::with_playit_fixture();
    let binary = acquisition(&transport, &fs);
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);

    service.start(launch(), &binary).unwrap();
    let first_pid = supervisor.spawned_requests().pop().unwrap().0;
    supervisor
        .emit_stdout(first_pid, b"tunnel setup\n")
        .unwrap();
    supervisor
        .emit_stdout(first_pid, b"join.example.joinmc.link\n")
        .unwrap();
    service.poll().unwrap();
    service.stop().unwrap();
    service.poll().unwrap();
    assert_eq!(service.lifecycle_status(), PlayitLifecycleStatus::Stopped);

    service.start(launch(), &binary).unwrap();

    let spawned = supervisor.spawned_requests();
    assert_eq!(spawned.len(), 2);
    assert_ne!(spawned[0].0, spawned[1].0);
}

#[test]
fn playit_exit_before_readiness_fails_operation_and_removes_bridge() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let transport = FakeTransport::with_playit_fixture();
    let binary = acquisition(&transport, &fs);
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);
    let launch = launch();
    let bridge_path = launch.secret_path.clone();
    let operation_id = msc_domain::operation::OperationId::new(
        service
            .start(launch, &binary)
            .unwrap()
            .operation_id
            .unwrap(),
    );
    let pid = supervisor.spawned_requests().pop().unwrap().0;

    supervisor
        .emit_stderr(pid, b"startup diagnostics\n")
        .unwrap();
    supervisor.crash(pid, 23).unwrap();
    service.poll().unwrap();

    assert!(!bridge_path.exists());
    assert_eq!(service.lifecycle_status().label(), "Failed");
    let operation = operations.snapshot(&operation_id).unwrap().unwrap();
    assert_eq!(operation.state, OperationState::Failed);
    assert_eq!(operation.error.unwrap().code, "playit_helper_failed");
}

#[test]
fn playit_readiness_is_a_player_address_not_a_management_address() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let transport = FakeTransport::with_playit_fixture();
    let binary = acquisition(&transport, &fs);
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);
    let operation_id = service
        .start(launch(), &binary)
        .unwrap()
        .operation_id
        .unwrap();

    service.observe_output("tunnel setup").unwrap();
    service.observe_output("join.example.joinmc.link").unwrap();

    assert_eq!(service.status().status, HelperStatus::Running);
    assert_eq!(
        service.status().player_address.as_deref(),
        Some("join.example.joinmc.link")
    );
    assert!(service.first_start_ready());
    assert_eq!(
        operations
            .snapshot(&msc_domain::operation::OperationId::new(operation_id))
            .unwrap()
            .unwrap()
            .state,
        OperationState::Succeeded
    );
}

#[test]
fn playit_start_can_be_cancelled_before_ready_and_restart_is_unknown() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let transport = FakeTransport::with_playit_fixture();
    let binary = acquisition(&transport, &fs);
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);
    let operation_id = service
        .start(launch(), &binary)
        .unwrap()
        .operation_id
        .unwrap();
    let operation_id = msc_domain::operation::OperationId::new(operation_id);

    operations
        .request_cancel(&operation_id, "Cancelling Playit.")
        .unwrap();
    assert!(service.cancel_start_if_requested().unwrap());
    assert_eq!(
        operations.snapshot(&operation_id).unwrap().unwrap().state,
        OperationState::Cancelled
    );

    service.recover_after_restart();
    assert_eq!(
        service.status().status,
        HelperStatus::UnknownUntilReconciled
    );
}

#[test]
fn playit_ready_signal_times_out_at_the_msc1_watchdog_boundary() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let transport = FakeTransport::with_playit_fixture();
    let binary = acquisition(&transport, &fs);
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);
    let operation_id = msc_domain::operation::OperationId::new(
        service
            .start(launch(), &binary)
            .unwrap()
            .operation_id
            .unwrap(),
    );

    assert!(!service.ready_timeout_elapsed(74).unwrap());
    assert!(service.first_start_timeout_elapsed(75).unwrap());
    assert_eq!(service.status().status, HelperStatus::TimedOut);
    let operation = operations.snapshot(&operation_id).unwrap().unwrap();
    assert_eq!(operation.state, OperationState::Failed);
    assert_eq!(operation.error.unwrap().code, "playit_ready_timeout");
}

#[test]
fn disabled_or_unconfigured_playit_does_not_spawn_a_process() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let transport = FakeTransport::with_playit_fixture();
    let binary = acquisition(&transport, &fs);
    let secrets = FakeSecretStore::new();
    let mut service = PlayitService::new("paper-1", false, &supervisor, &secrets, &operations);
    assert_eq!(
        service.start(launch(), &binary).unwrap_err(),
        PlayitError::Disabled
    );

    let mut enabled = PlayitService::new("paper-2", true, &supervisor, &secrets, &operations);
    assert_eq!(
        enabled.start(launch(), &binary).unwrap_err(),
        PlayitError::MissingSecret
    );
    assert!(supervisor.spawned_requests().is_empty());
}

#[test]
fn playit_secret_is_trimmed_and_only_the_secret_store_retains_it() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let secrets = FakeSecretStore::new();
    let service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);

    service.save_secret("  configured-secret  ").unwrap();
    assert!(service.has_secret().unwrap());
    assert_eq!(
        secrets.get(PLAYIT_SECRET_KEY).unwrap().as_deref(),
        Some("configured-secret")
    );
    service.remove_secret().unwrap();
    assert!(!service.has_secret().unwrap());
}

#[test]
fn acquisition_failure_is_journaled_and_never_arms_readiness_watchdog() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    let transport = FakeTransport::with_playit_fixture();
    let binary = PlayitBinaryAcquisition::new(
        &transport,
        &fs,
        Path::new("/cache"),
        PinnedHelperRelease {
            helper: "playitd".into(),
            version: "playitd-v1.0.10".into(),
            release_metadata_url: RELEASE_URL.into(),
            assets: vec![PinnedHelperAsset {
                platform: HelperPlatform::LinuxX86_64,
                asset_name: "playitd-linux-x86_64".into(),
                sha256: "0".repeat(64),
            }],
        },
        HelperPlatform::LinuxX86_64,
    );
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);

    assert!(matches!(
        service.start(launch(), &binary),
        Err(PlayitError::Acquisition(message)) if message.contains("checksum mismatch")
    ));
    assert!(supervisor.spawned_requests().is_empty());
    assert!(!service.ready_timeout_elapsed(75).unwrap());
    let operation_path = fs
        .list(Path::new(OPERATIONS_DIR))
        .unwrap()
        .into_iter()
        .find(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .expect("acquisition operation");
    let operation: serde_json::Value =
        serde_json::from_slice(&fs.read(&operation_path).unwrap()).unwrap();
    assert_eq!(operation["state"], "failed");
    assert_eq!(operation["error"]["code"], "playit_acquisition_failed");
}

#[test]
fn helper_start_failure_is_visible_and_removes_the_secret_bridge() {
    let fs = FakeFileSystem::new()
        .with_file(format!("{OPERATIONS_DIR}/.keep"), [], false)
        .with_dir("/cache");
    let operations = LifecycleOperations::new(&fs, OPERATIONS_DIR);
    let supervisor = FakeProcessSupervisor::new();
    supervisor.fail_next_spawn("playitd could not be started");
    let transport = FakeTransport::with_playit_fixture();
    let binary = acquisition(&transport, &fs);
    let secrets = FakeSecretStore::new();
    secrets.set(PLAYIT_SECRET_KEY, "secret").unwrap();
    let mut service = PlayitService::new("paper-1", true, &supervisor, &secrets, &operations);
    let launch = launch();
    let bridge_path = launch.secret_path.clone();

    let error = service.start(launch, &binary).unwrap_err();

    assert!(
        matches!(error, PlayitError::Process(message) if message.contains("could not be started"))
    );
    assert_eq!(
        service.lifecycle_status(),
        PlayitLifecycleStatus::Failed {
            message: "playitd could not be started".into()
        }
    );
    assert!(!bridge_path.exists());
}

#[test]
fn native_setup_claims_a_new_agent_and_persists_only_the_permanent_key() {
    let transport = FakeAccountTransport::new([
        (
            200,
            serde_json::json!({"status":"success","data":{"session_key":"session-secret"}}),
        ),
        (
            200,
            serde_json::json!({"status":"success","data":"WaitingForAgent"}),
        ),
        (200, serde_json::json!({"status":"success","data":"ok"})),
        (
            200,
            serde_json::json!({"status":"success","data":{"agent_id":"agent-123"}}),
        ),
        (
            200,
            serde_json::json!({"status":"success","data":{"secret_key":"agent-secret"}}),
        ),
    ]);
    let secrets = FakeSecretStore::new();
    let setup = msc_application::playit::PlayitAccountSetup::new(&transport, &secrets);
    let stages = Mutex::new(Vec::new());
    let result = setup
        .run(
            "owner@example.test",
            "password",
            None,
            || false,
            |stage| stages.lock().unwrap().push(stage),
        )
        .unwrap();

    assert_eq!(result.agent_id, "agent-123");
    assert!(!result.reused_existing_agent);
    assert_eq!(
        secrets.get(PLAYIT_SECRET_KEY).unwrap().as_deref(),
        Some("agent-secret")
    );
    assert_eq!(
        *stages.lock().unwrap(),
        [
            msc_application::playit::PlayitSetupStage::SigningIn,
            msc_application::playit::PlayitSetupStage::ClaimingOrReusingAgent,
            msc_application::playit::PlayitSetupStage::WaitingForAgent,
        ]
    );
}

#[test]
fn native_setup_keeps_a_newly_claimed_agent_when_first_tunnel_creation_fails() {
    let transport = FakeAccountTransport::new([
        (
            200,
            serde_json::json!({"status":"success","data":{"session_key":"session-secret"}}),
        ),
        (
            200,
            serde_json::json!({"status":"success","data":"WaitingForAgent"}),
        ),
        (200, serde_json::json!({"status":"success","data":"ok"})),
        (
            200,
            serde_json::json!({"status":"success","data":{"agent_id":"agent-123"}}),
        ),
        (
            200,
            serde_json::json!({"status":"success","data":{"secret_key":"agent-secret"}}),
        ),
        tunnel_list_response(Vec::new()),
        (
            400,
            serde_json::json!({"status":"fail","data":"NoFreeTunnels"}),
        ),
    ]);
    let secrets = FakeSecretStore::new();
    let saved_agent_id = Mutex::new(None);
    let save_agent_configuration = |agent_id: &str| {
        *saved_agent_id.lock().unwrap() = Some(agent_id.to_owned());
        Ok(())
    };
    let start_agent = |agent_id: &str| {
        assert_eq!(agent_id, "agent-123");
        Ok(())
    };
    let setup = msc_application::playit::PlayitAccountSetup::with_agent_lifecycle(
        &transport,
        &secrets,
        &save_agent_configuration,
        &start_agent,
    );
    let specs = [PlayitTunnelSpec {
        kind: PlayitTunnelKind::Java,
        local_port: 25565,
    }];

    assert_eq!(
        setup
            .run_with_tunnels(
                "owner@example.test",
                "password",
                None,
                &specs,
                || false,
                |_| {},
            )
            .unwrap_err(),
        msc_application::playit::PlayitSetupError::Api(
            msc_infrastructure::playit_api::PlayitApiError::ApiFailure
        )
    );
    assert_eq!(
        secrets.get(PLAYIT_SECRET_KEY).unwrap().as_deref(),
        Some("agent-secret")
    );
    assert_eq!(saved_agent_id.lock().unwrap().as_deref(), Some("agent-123"));
}

#[test]
fn native_setup_reuses_a_configured_host_agent_without_claiming_another() {
    let transport = FakeAccountTransport::new([(
        200,
        serde_json::json!({"status":"success","data":{"session_key":"session-secret"}}),
    )]);
    let secrets = FakeSecretStore::new();
    secrets
        .set(PLAYIT_SECRET_KEY, "existing-agent-secret")
        .unwrap();
    let setup = msc_application::playit::PlayitAccountSetup::new(&transport, &secrets);

    let result = setup
        .run(
            "owner@example.test",
            "password",
            Some("agent-existing"),
            || false,
            |_| {},
        )
        .unwrap();

    assert_eq!(result.agent_id, "agent-existing");
    assert!(result.reused_existing_agent);
    assert_eq!(
        secrets.get(PLAYIT_SECRET_KEY).unwrap().as_deref(),
        Some("existing-agent-secret")
    );
    assert!(transport.responses.lock().unwrap().is_empty());
}

#[test]
fn native_setup_cancellation_drops_the_temporary_session_before_claiming() {
    let transport = FakeAccountTransport::new([(
        200,
        serde_json::json!({"status":"success","data":{"session_key":"session-secret"}}),
    )]);
    let secrets = FakeSecretStore::new();
    let setup = msc_application::playit::PlayitAccountSetup::new(&transport, &secrets);
    let result = setup.run("owner@example.test", "password", None, || true, |_| {});

    assert_eq!(
        result.unwrap_err(),
        msc_application::playit::PlayitSetupError::Cancelled
    );
    assert_eq!(secrets.get(PLAYIT_SECRET_KEY).unwrap(), None);
}

fn tunnel_fixture(kind: PlayitTunnelKind, local_port: u16) -> Value {
    let origin = match kind {
        PlayitTunnelKind::Voice => serde_json::json!({
            "type": "agent",
            "data": {
                "agent_id": "agent-existing",
                "config": {
                    "fields": [
                        {"name": "local_ip", "value": "127.0.0.1"},
                        {"name": "local_port", "value": local_port.to_string()}
                    ]
                }
            }
        }),
        PlayitTunnelKind::Java | PlayitTunnelKind::Bedrock => serde_json::json!({
            "type": "agent",
            "data": {
                "agent_id": "agent-existing",
                "local_ip": "127.0.0.1",
                "local_port": local_port
            }
        }),
    };
    let mut tunnel = serde_json::json!({
        "name": kind.name(),
        "active": true,
        "origin": origin,
        "alloc": {
            "data": {
                "assigned_domain": format!("{}.example.joinmc.link", kind.name().replace(' ', "-").to_ascii_lowercase()),
                "port_start": match kind {
                    PlayitTunnelKind::Java => 25565,
                    PlayitTunnelKind::Bedrock => 19132,
                    PlayitTunnelKind::Voice => 24454,
                }
            }
        }
    });
    match kind {
        PlayitTunnelKind::Java | PlayitTunnelKind::Bedrock => {
            tunnel["tunnel_type"] = Value::String(kind.tunnel_type().unwrap().to_owned());
            tunnel["port_type"] = Value::String(kind.port_type().to_owned());
        }
        PlayitTunnelKind::Voice => {
            tunnel["protocol"] = serde_json::json!({
                "type": "raw-ports",
                "details": {"port_type": "udp"}
            });
            tunnel["alloc"]["data"]["static_ip4"] = Value::String("203.0.113.10".into());
        }
    }
    if kind == PlayitTunnelKind::Bedrock {
        tunnel["alloc"]["data"]["static_ip4"] = Value::String("198.51.100.10".into());
    }
    tunnel
}

fn legacy_voice_tunnel_fixture(local_port: u16) -> Value {
    let mut tunnel = tunnel_fixture(PlayitTunnelKind::Voice, local_port);
    tunnel["origin"] = serde_json::json!({
        "type": "agent",
        "data": {
            "agent_id": "agent-existing",
            "local_ip": "127.0.0.1",
            "local_port": local_port
        }
    });
    tunnel["port_type"] = Value::String("udp".into());
    tunnel.as_object_mut().unwrap().remove("protocol");
    tunnel
}

fn tunnel_list_response(tunnels: Vec<Value>) -> (u16, Value) {
    (
        200,
        serde_json::json!({"status": "success", "data": {"tunnels": tunnels}}),
    )
}

fn setup_responses(specs: &[PlayitTunnelSpec], inventory: Vec<Value>) -> Vec<(u16, Value)> {
    let mut responses = vec![
        (
            200,
            serde_json::json!({"status": "success", "data": {"session_key": "session-secret"}}),
        ),
        tunnel_list_response(Vec::new()),
    ];
    responses.extend(specs.iter().map(|_| {
        (
            200,
            serde_json::json!({"status": "success", "data": {"id": "tunnel"}}),
        )
    }));
    responses.push(tunnel_list_response(inventory));
    responses
}

#[test]
fn native_setup_provisions_one_two_and_three_tunnel_accounts() {
    for specs in [
        vec![PlayitTunnelSpec {
            kind: PlayitTunnelKind::Java,
            local_port: 25565,
        }],
        vec![
            PlayitTunnelSpec {
                kind: PlayitTunnelKind::Java,
                local_port: 25565,
            },
            PlayitTunnelSpec {
                kind: PlayitTunnelKind::Bedrock,
                local_port: 19132,
            },
        ],
        vec![
            PlayitTunnelSpec {
                kind: PlayitTunnelKind::Java,
                local_port: 25565,
            },
            PlayitTunnelSpec {
                kind: PlayitTunnelKind::Bedrock,
                local_port: 19132,
            },
            PlayitTunnelSpec {
                kind: PlayitTunnelKind::Voice,
                local_port: 24454,
            },
        ],
    ] {
        let inventory = specs
            .iter()
            .map(|spec| tunnel_fixture(spec.kind, spec.local_port))
            .collect::<Vec<_>>();
        let transport = FakeAccountTransport::new(setup_responses(&specs, inventory));
        let secrets = FakeSecretStore::new();
        secrets
            .set(PLAYIT_SECRET_KEY, "existing-agent-secret")
            .unwrap();
        let setup = msc_application::playit::PlayitAccountSetup::new(&transport, &secrets);
        let result = setup
            .run_with_tunnels(
                "owner@example.test",
                "password",
                Some("agent-existing"),
                &specs,
                || false,
                |_| {},
            )
            .unwrap();

        assert!(result.tunnel_addresses.java.is_some());
        assert_eq!(
            result.tunnel_addresses.bedrock.is_some(),
            specs
                .iter()
                .any(|spec| spec.kind == PlayitTunnelKind::Bedrock)
        );
        assert_eq!(
            result.tunnel_addresses.voice.as_deref(),
            specs
                .iter()
                .any(|spec| spec.kind == PlayitTunnelKind::Voice)
                .then_some(Some("203.0.113.10:24454"))
                .flatten()
        );
        let requests = transport.requests.lock().unwrap();
        assert_eq!(requests[1].0, "/tunnels/list");
        assert_eq!(requests[1].1["agent_id"], Value::Null);
        assert_eq!(
            requests[1].2.as_deref(),
            Some("Agent-Key existing-agent-secret")
        );
        for (request, spec) in requests[2..2 + specs.len()].iter().zip(&specs) {
            assert_eq!(request.2.as_deref(), Some("session session-secret"));
            assert_eq!(request.1["name"], spec.kind.name());
            assert_eq!(request.1["origin"]["data"]["agent_id"], "agent-existing");
            match spec.kind {
                PlayitTunnelKind::Java | PlayitTunnelKind::Bedrock => {
                    assert_eq!(request.0, "/tunnels/create");
                    assert_eq!(request.1["origin"]["data"]["local_port"], spec.local_port);
                }
                PlayitTunnelKind::Voice => {
                    assert_eq!(request.0, "/v1/tunnels/create");
                    assert_eq!(
                        request.1["origin"]["data"]["config"]["fields"][1]["value"],
                        spec.local_port.to_string()
                    );
                }
            }
        }
        assert_eq!(
            requests
                .iter()
                .filter(|(path, _, _)| path.ends_with("/tunnels/create"))
                .count(),
            specs.len()
        );
    }
}

#[test]
fn native_setup_waits_for_public_addresses_before_succeeding() {
    let specs = [PlayitTunnelSpec {
        kind: PlayitTunnelKind::Java,
        local_port: 25565,
    }];
    let mut not_ready = tunnel_fixture(PlayitTunnelKind::Java, 25565);
    not_ready["active"] = Value::Bool(false);
    let mut responses = setup_responses(&specs, vec![not_ready]);
    responses.push(tunnel_list_response(vec![tunnel_fixture(
        PlayitTunnelKind::Java,
        25565,
    )]));
    let transport = FakeAccountTransport::new(responses);
    let secrets = FakeSecretStore::new();
    secrets
        .set(PLAYIT_SECRET_KEY, "existing-agent-secret")
        .unwrap();
    let setup = msc_application::playit::PlayitAccountSetup::new(&transport, &secrets);

    let result = setup
        .run_with_tunnels(
            "owner@example.test",
            "password",
            Some("agent-existing"),
            &specs,
            || false,
            |_| {},
        )
        .unwrap();

    assert_eq!(
        result.tunnel_addresses.java.as_deref(),
        Some("msc-java.example.joinmc.link:25565")
    );
    assert_eq!(
        transport
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|request| request.0 == "/tunnels/list")
            .count(),
        3
    );
}

#[test]
fn saved_agent_address_refresh_reuses_the_key_without_signing_in_or_mutating_tunnels() {
    let specs = [
        PlayitTunnelSpec {
            kind: PlayitTunnelKind::Java,
            local_port: 25565,
        },
        PlayitTunnelSpec {
            kind: PlayitTunnelKind::Bedrock,
            local_port: 19132,
        },
        PlayitTunnelSpec {
            kind: PlayitTunnelKind::Voice,
            local_port: 24454,
        },
    ];
    let transport = FakeAccountTransport::new([tunnel_list_response(
        specs
            .iter()
            .map(|spec| tunnel_fixture(spec.kind, spec.local_port))
            .collect(),
    )]);
    let secrets = FakeSecretStore::new();
    secrets
        .set(PLAYIT_SECRET_KEY, "existing-agent-secret")
        .unwrap();
    let setup = msc_application::playit::PlayitAccountSetup::new(&transport, &secrets);

    let addresses = setup
        .refresh_tunnel_addresses("agent-existing", &specs)
        .unwrap();

    assert_eq!(
        addresses.java.as_deref(),
        Some("msc-java.example.joinmc.link:25565")
    );
    assert_eq!(addresses.bedrock.as_deref(), Some("198.51.100.10:19132"));
    assert_eq!(addresses.voice.as_deref(), Some("203.0.113.10:24454"));
    let requests = transport.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0, "/tunnels/list");
    assert_eq!(
        requests[0].2.as_deref(),
        Some("Agent-Key existing-agent-secret")
    );
}

#[test]
fn native_setup_starts_agent_before_first_tunnel_request() {
    let specs = [PlayitTunnelSpec {
        kind: PlayitTunnelKind::Java,
        local_port: 25565,
    }];
    let transport = FakeAccountTransport::new(setup_responses(
        &specs,
        vec![tunnel_fixture(PlayitTunnelKind::Java, 25565)],
    ));
    let secrets = FakeSecretStore::new();
    secrets
        .set(PLAYIT_SECRET_KEY, "existing-agent-secret")
        .unwrap();
    let agent_starts = Mutex::new(0);
    let ensure_agent = |agent_id: &str| {
        *agent_starts.lock().unwrap() += 1;
        let requests = transport.requests.lock().unwrap();
        assert_eq!(requests.len(), 1, "only sign-in may precede agent start");
        assert_eq!(agent_id, "agent-existing");
        Ok(())
    };
    let setup = msc_application::playit::PlayitAccountSetup::with_agent_starter(
        &transport,
        &secrets,
        &ensure_agent,
    );

    setup
        .run_with_tunnels(
            "owner@example.test",
            "password",
            Some("agent-existing"),
            &specs,
            || false,
            |_| {},
        )
        .unwrap();

    assert_eq!(*agent_starts.lock().unwrap(), 1);
    assert_eq!(transport.requests.lock().unwrap()[1].0, "/tunnels/list");
}

#[test]
fn native_setup_reuses_existing_tunnels_without_duplicates_on_repeat() {
    let specs = [PlayitTunnelSpec {
        kind: PlayitTunnelKind::Java,
        local_port: 25565,
    }];
    let inventory = vec![tunnel_fixture(PlayitTunnelKind::Java, 25565)];
    let mut responses = setup_responses(&specs, inventory.clone());
    responses.extend([
        (
            200,
            serde_json::json!({"status": "success", "data": {"session_key": "session-secret"}}),
        ),
        tunnel_list_response(inventory.clone()),
        tunnel_list_response(inventory),
    ]);
    let transport = FakeAccountTransport::new(responses);
    let secrets = FakeSecretStore::new();
    secrets
        .set(PLAYIT_SECRET_KEY, "existing-agent-secret")
        .unwrap();
    let setup = msc_application::playit::PlayitAccountSetup::new(&transport, &secrets);

    setup
        .run_with_tunnels(
            "owner@example.test",
            "password",
            Some("agent-existing"),
            &specs,
            || false,
            |_| {},
        )
        .unwrap();
    setup
        .run_with_tunnels(
            "owner@example.test",
            "password",
            Some("agent-existing"),
            &specs,
            || false,
            |_| {},
        )
        .unwrap();

    let requests = transport.requests.lock().unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|(path, _, _)| path.ends_with("/tunnels/create"))
            .count(),
        1
    );
}

#[test]
fn native_setup_reuses_legacy_voice_inventory_without_protocol_marker() {
    let specs = [PlayitTunnelSpec {
        kind: PlayitTunnelKind::Voice,
        local_port: 24454,
    }];
    let inventory = vec![legacy_voice_tunnel_fixture(24454)];
    let transport = FakeAccountTransport::new([
        (
            200,
            serde_json::json!({"status": "success", "data": {"session_key": "session-secret"}}),
        ),
        tunnel_list_response(inventory.clone()),
        tunnel_list_response(inventory),
    ]);
    let secrets = FakeSecretStore::new();
    secrets
        .set(PLAYIT_SECRET_KEY, "existing-agent-secret")
        .unwrap();
    let setup = msc_application::playit::PlayitAccountSetup::new(&transport, &secrets);

    let result = setup
        .run_with_tunnels(
            "owner@example.test",
            "password",
            Some("agent-existing"),
            &specs,
            || false,
            |_| {},
        )
        .unwrap();

    assert_eq!(
        result.tunnel_addresses.voice.as_deref(),
        Some("203.0.113.10:24454")
    );
    assert_eq!(
        transport
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _, _)| path.ends_with("/tunnels/create"))
            .count(),
        0
    );
}

#[test]
fn native_setup_reuses_a_named_tunnel_that_targets_another_local_port_in_shared_mode() {
    let transport = FakeAccountTransport::new([
        (
            200,
            serde_json::json!({"status": "success", "data": {"session_key": "session-secret"}}),
        ),
        tunnel_list_response(vec![tunnel_fixture(PlayitTunnelKind::Java, 25566)]),
        tunnel_list_response(vec![tunnel_fixture(PlayitTunnelKind::Java, 25566)]),
    ]);
    let secrets = FakeSecretStore::new();
    secrets
        .set(PLAYIT_SECRET_KEY, "existing-agent-secret")
        .unwrap();
    let setup = msc_application::playit::PlayitAccountSetup::new(&transport, &secrets);
    let result = setup.run_with_tunnels(
        "owner@example.test",
        "password",
        Some("agent-existing"),
        &[PlayitTunnelSpec {
            kind: PlayitTunnelKind::Java,
            local_port: 25565,
        }],
        || false,
        |_| {},
    );

    assert!(result.is_ok());
    assert_eq!(
        secrets.get(PLAYIT_SECRET_KEY).unwrap().as_deref(),
        Some("existing-agent-secret")
    );
    assert_eq!(
        transport
            .requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _, _)| path.ends_with("/tunnels/create"))
            .count(),
        0
    );
}

#[test]
fn simple_voice_chat_patch_preserves_unowned_properties() {
    let patched = msc_domain::networking::patch_voice_chat_properties(
        "# keep this\nvoice_host=old.example:24454\nport=19132\nmotd=hello\n",
        "203.0.113.10:24454",
    );
    assert!(patched.contains("# keep this\n"));
    assert!(patched.contains("voice_host=203.0.113.10:24454\n"));
    assert!(patched.contains("bind_address=*\n"));
    assert!(patched.contains("port=24454\n"));
    assert!(patched.contains("motd=hello\n"));
}
