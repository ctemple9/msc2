//! Application service for one server's Playit Minecraft tunnel.
//!
//! This deliberately starts only `playitd`; it neither accepts nor forwards
//! MSC HTTP traffic.  The management listener remains independently bound and
//! bearer-authenticated by `msc-agent`.

use crate::operations::{LifecycleOperations, lifecycle_error};
use msc_domain::helper::{
    FirstRunTransport, HelperSnapshot, HelperStatus, decide_playit_start, first_run_timeout,
};
use msc_domain::networking::parse_playit_address;
use msc_infrastructure::helper_process::{HelperKey, HelperProcessError, HelperProcessManager};
use msc_infrastructure::playit::{PLAYIT_SECRET_KEY, PlayitBinaryAcquisition, PlayitLaunch};
use msc_infrastructure::process::ProcessSupervisor;
use msc_infrastructure::secret_store::SecretStore;
use std::collections::BTreeMap;
use std::fmt;

pub const PLAYIT_OPERATION_TYPE: &str = "playit-tunnel";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayitStartResult {
    pub operation_id: Option<String>,
    pub status: HelperStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayitError {
    Disabled,
    MissingSecret,
    AlreadyManaged,
    Acquisition(String),
    SecretStore(String),
    Operation(String),
    Process(String),
}

impl fmt::Display for PlayitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => write!(f, "Playit is not enabled for this server"),
            Self::MissingSecret => write!(f, "no Playit secret is configured"),
            Self::AlreadyManaged => write!(f, "the Playit tunnel is already managed"),
            Self::Acquisition(message) => write!(f, "Playit binary acquisition failed: {message}"),
            Self::SecretStore(message) | Self::Operation(message) | Self::Process(message) => {
                write!(f, "{message}")
            }
        }
    }
}

impl std::error::Error for PlayitError {}

/// A small façade around P9.6's common process manager.  It owns no secret
/// values: it asks `SecretStore` only whether the stable Playit key exists.
pub struct PlayitService<'a> {
    server_id: String,
    enabled: bool,
    helpers: HelperProcessManager<'a>,
    secrets: &'a dyn SecretStore,
    operations: &'a LifecycleOperations<'a>,
    snapshot: HelperSnapshot,
    expecting_address: bool,
    active_operation: Option<msc_domain::operation::OperationId>,
}

impl<'a> PlayitService<'a> {
    pub fn new(
        server_id: impl Into<String>,
        enabled: bool,
        supervisor: &'a dyn ProcessSupervisor,
        secrets: &'a dyn SecretStore,
        operations: &'a LifecycleOperations<'a>,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            enabled,
            helpers: HelperProcessManager::new(supervisor),
            secrets,
            operations,
            snapshot: HelperSnapshot::stopped(),
            expecting_address: false,
            active_operation: None,
        }
    }

    pub fn status(&self) -> &HelperSnapshot {
        &self.snapshot
    }

    pub fn has_secret(&self) -> Result<bool, PlayitError> {
        self.secrets
            .get(PLAYIT_SECRET_KEY)
            .map(|secret| secret.is_some_and(|value| !value.trim().is_empty()))
            .map_err(|error| PlayitError::SecretStore(error.to_string()))
    }

    /// Stores the key only in the platform-backed secret store.  The caller
    /// receives no value back, so status, operations, and audit data have no
    /// reason to retain it.
    pub fn save_secret(&self, secret: &str) -> Result<(), PlayitError> {
        let secret = secret.trim();
        if secret.is_empty() {
            return Err(PlayitError::MissingSecret);
        }
        self.secrets
            .set(PLAYIT_SECRET_KEY, secret)
            .map_err(|error| PlayitError::SecretStore(error.to_string()))
    }

    pub fn remove_secret(&self) -> Result<(), PlayitError> {
        self.secrets
            .delete(PLAYIT_SECRET_KEY)
            .map_err(|error| PlayitError::SecretStore(error.to_string()))
    }

    pub fn start(
        &mut self,
        launch: PlayitLaunch,
        acquisition: &PlayitBinaryAcquisition<'_>,
    ) -> Result<PlayitStartResult, PlayitError> {
        let secret_present = self.has_secret()?;
        match decide_playit_start(self.enabled, secret_present) {
            msc_domain::helper::HelperStartDecision::NoAction => return Err(PlayitError::Disabled),
            msc_domain::helper::HelperStartDecision::PromptSecretSetup => {
                return Err(PlayitError::MissingSecret);
            }
            msc_domain::helper::HelperStartDecision::Launch => {}
        }

        let key = self.key();
        let operation_id = self
            .operations
            .begin_running(
                PLAYIT_OPERATION_TYPE,
                Some(key.operation_target()),
                "Starting Playit tunnel.",
            )
            .map_err(|error| PlayitError::Operation(error.to_string()))?;
        let acquired = match acquisition.acquire() {
            Ok(acquired) => acquired,
            Err(error) => {
                let message = error.to_string();
                let _ = self.operations.fail(
                    &operation_id,
                    lifecycle_error("playit_acquisition_failed", message.clone()),
                );
                return Err(PlayitError::Acquisition(message));
            }
        };
        match self
            .helpers
            .start(key, launch.process_request(&acquired.artifact.path))
        {
            Ok(_) => {
                self.snapshot.status = HelperStatus::Starting;
                self.active_operation = Some(operation_id.clone());
                Ok(PlayitStartResult {
                    operation_id: Some(operation_id.as_str().to_string()),
                    status: HelperStatus::Starting,
                })
            }
            Err(error) => {
                let _ = self.operations.fail(
                    &operation_id,
                    lifecycle_error("playit_start_failed", error.to_string()),
                );
                Err(map_process_error(error))
            }
        }
    }

    /// Feed one already-framed helper line into the provider-specific parser.
    /// This is where a tunnel's readiness is established, never at process spawn.
    pub fn observe_output(&mut self, line: &str) -> Result<(), PlayitError> {
        if line.to_ascii_lowercase().contains("tunnel setup") {
            self.expecting_address = true;
            return Ok(());
        }
        let address = parse_playit_address(line, self.expecting_address);
        self.expecting_address = false;
        if let Some(address) = address {
            let key = self.key();
            self.helpers.record_ready(&key).map_err(map_process_error)?;
            self.snapshot = self.snapshot.clone().on_ready(address.clone());
            if let Some(operation_id) = self.active_operation.take() {
                self.operations
                    .succeed(
                        &operation_id,
                        "Playit tunnel is ready.",
                        BTreeMap::from([("playerAddress".to_string(), address)]),
                    )
                    .map_err(|error| PlayitError::Operation(error.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), PlayitError> {
        let key = self.key();
        self.helpers
            .request_graceful_stop(&key)
            .map_err(map_process_error)?;
        self.snapshot.status = HelperStatus::Starting;
        Ok(())
    }

    /// Recovery never trusts a former PID.  The caller must reconcile with
    /// Playit before starting a fresh tunnel after an agent restart.
    pub fn recover_after_restart(&mut self) {
        self.snapshot = HelperSnapshot::after_agent_restart();
    }

    pub fn cancel_start_if_requested(&mut self) -> Result<bool, PlayitError> {
        let Some(operation_id) = self.active_operation.clone() else {
            return Ok(false);
        };
        if !self.operations.cancellation_check(&operation_id)() {
            return Ok(false);
        }
        self.helpers
            .request_graceful_stop(&self.key())
            .map_err(map_process_error)?;
        self.operations
            .cancel(&operation_id, "Playit tunnel start cancelled.")
            .map_err(|error| PlayitError::Operation(error.to_string()))?;
        self.active_operation = None;
        self.snapshot = HelperSnapshot::stopped();
        Ok(true)
    }

    /// MSC 1 gives Playit roughly 75 seconds to emit a usable join address
    /// during first-run setup.  The signal remains bounded even when the
    /// provider process keeps running, so creation orchestration can advance
    /// honestly instead of waiting forever.
    pub fn ready_timeout_elapsed(&mut self, seconds_waiting: u64) -> Result<bool, PlayitError> {
        if self.snapshot.status != HelperStatus::Starting
            || first_run_timeout(FirstRunTransport::Playit, seconds_waiting).is_none()
        {
            return Ok(false);
        }
        self.snapshot.status = HelperStatus::TimedOut;
        if let Some(operation_id) = self.active_operation.take() {
            self.operations
                .fail(
                    &operation_id,
                    lifecycle_error(
                        "playit_ready_timeout",
                        "Playit did not provide a player address within 75 seconds.",
                    ),
                )
                .map_err(|error| PlayitError::Operation(error.to_string()))?;
        }
        Ok(true)
    }

    fn key(&self) -> HelperKey {
        HelperKey::new(&self.server_id, "playit")
    }
}

fn map_process_error(error: HelperProcessError) -> PlayitError {
    match error {
        HelperProcessError::AlreadyManaged(_) => PlayitError::AlreadyManaged,
        other => PlayitError::Process(other.to_string()),
    }
}
