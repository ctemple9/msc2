use msc_application::operations::LifecycleOperations;
use msc_application::playit::{PLAYIT_OPERATION_TYPE, PlayitError, PlayitService};
use msc_domain::helper::HelperStatus;
use msc_domain::operation::OperationState;
use msc_infrastructure::fs::{FakeFileSystem, FileSystem};
use msc_infrastructure::helper_acquisition::{
    HelperPlatform, PinnedHelperAsset, PinnedHelperRelease,
};
use msc_infrastructure::jar_provider::{JarProviderError, Transport};
use msc_infrastructure::playit::{PLAYIT_SECRET_KEY, PlayitBinaryAcquisition, PlayitLaunch};
use msc_infrastructure::process::FakeProcessSupervisor;
use msc_infrastructure::secret_store::{FakeSecretStore, SecretStore};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const OPERATIONS_DIR: &str = "/agent/operations";

fn launch() -> PlayitLaunch {
    PlayitLaunch {
        working_directory: PathBuf::from("/agent"),
        secret_path: PathBuf::from("/agent/secret-bridge/playit"),
    }
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

    let started = service.start(launch(), &binary).unwrap();
    let operation_id = started.operation_id.unwrap();
    assert_eq!(started.status, HelperStatus::Starting);
    assert_eq!(
        operations
            .snapshot(&msc_domain::operation::OperationId::new(operation_id))
            .unwrap()
            .unwrap()
            .operation_type,
        PLAYIT_OPERATION_TYPE
    );

    let (_, request) = supervisor.spawned_requests().pop().unwrap();
    assert_eq!(
        request.arguments,
        ["--secret-path", "/agent/secret-bridge/playit"]
    );
    assert_eq!(
        request.executable_path,
        PathBuf::from("/cache/playitd/playitd-v1.0.10/playitd-linux-x86_64")
    );
    assert!(!format!("{request:?}").contains("playit-secret-must-not-leak"));
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
    assert!(service.ready_timeout_elapsed(75).unwrap());
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
