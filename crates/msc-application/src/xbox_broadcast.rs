//! Application service for one server's MCXboxBroadcast helper.

use crate::operations::{LifecycleOperations, lifecycle_error};
use msc_domain::helper::{HelperSnapshot, HelperStatus};
use msc_domain::networking::{broadcast_is_ready, parse_broadcast_auth_prompt};
use msc_infrastructure::helper_process::{HelperKey, HelperProcessError, HelperProcessManager};
use msc_infrastructure::process::ProcessSupervisor;
use msc_infrastructure::secret_store::SecretStore;
use msc_infrastructure::xbox_broadcast::{
    XboxBroadcastLaunch, alt_password_secret_key, auth_token_secret_key,
};
use std::collections::BTreeMap;
use std::fmt;

pub const XBOX_BROADCAST_OPERATION_TYPE: &str = "xbox-broadcast";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BroadcastAuthPrompt {
    pub code: String,
    pub link_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BroadcastStatus {
    pub snapshot: HelperSnapshot,
    pub auth_prompt: Option<BroadcastAuthPrompt>,
    pub has_password: bool,
    pub has_auth_token: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XboxBroadcastError {
    Disabled,
    AlreadyManaged,
    MissingJar,
    SecretStore(String),
    Operation(String),
    Process(String),
}

impl fmt::Display for XboxBroadcastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => write!(f, "Xbox Broadcast is not enabled for this server"),
            Self::AlreadyManaged => write!(f, "Xbox Broadcast is already managed"),
            Self::MissingJar => write!(f, "MCXboxBroadcast JAR is not configured"),
            Self::SecretStore(message) | Self::Operation(message) | Self::Process(message) => {
                write!(f, "{message}")
            }
        }
    }
}

impl std::error::Error for XboxBroadcastError {}

pub struct XboxBroadcastService<'a> {
    server_id: String,
    enabled: bool,
    helpers: HelperProcessManager<'a>,
    secrets: &'a dyn SecretStore,
    operations: &'a LifecycleOperations<'a>,
    snapshot: HelperSnapshot,
    auth_prompt: Option<BroadcastAuthPrompt>,
    active_operation: Option<msc_domain::operation::OperationId>,
}

impl<'a> XboxBroadcastService<'a> {
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
            auth_prompt: None,
            active_operation: None,
        }
    }

    pub fn status(&self) -> Result<BroadcastStatus, XboxBroadcastError> {
        Ok(BroadcastStatus {
            snapshot: self.snapshot.clone(),
            auth_prompt: self.auth_prompt.clone(),
            has_password: self.has_secret(&alt_password_secret_key(&self.server_id))?,
            has_auth_token: self.has_secret(&auth_token_secret_key(&self.server_id))?,
        })
    }

    pub fn save_password(&self, password: &str) -> Result<(), XboxBroadcastError> {
        let password = password.trim();
        if password.is_empty() {
            return self.delete_password();
        }
        self.secrets
            .set(&alt_password_secret_key(&self.server_id), password)
            .map_err(|error| XboxBroadcastError::SecretStore(error.to_string()))
    }

    pub fn delete_password(&self) -> Result<(), XboxBroadcastError> {
        self.secrets
            .delete(&alt_password_secret_key(&self.server_id))
            .map_err(|error| XboxBroadcastError::SecretStore(error.to_string()))
    }

    pub fn save_auth_token(&self, token: &str) -> Result<(), XboxBroadcastError> {
        let token = token.trim();
        if token.is_empty() {
            return self.delete_auth_token();
        }
        self.secrets
            .set(&auth_token_secret_key(&self.server_id), token)
            .map_err(|error| XboxBroadcastError::SecretStore(error.to_string()))
    }

    pub fn delete_auth_token(&self) -> Result<(), XboxBroadcastError> {
        self.secrets
            .delete(&auth_token_secret_key(&self.server_id))
            .map_err(|error| XboxBroadcastError::SecretStore(error.to_string()))
    }

    pub fn start(&mut self, launch: XboxBroadcastLaunch) -> Result<String, XboxBroadcastError> {
        if !self.enabled {
            return Err(XboxBroadcastError::Disabled);
        }
        if launch.jar_path.as_os_str().is_empty() {
            return Err(XboxBroadcastError::MissingJar);
        }
        let key = self.key();
        let operation_id = self
            .operations
            .begin_running(
                XBOX_BROADCAST_OPERATION_TYPE,
                Some(key.operation_target()),
                "Starting Xbox Broadcast.",
            )
            .map_err(|error| XboxBroadcastError::Operation(error.to_string()))?;
        match self.helpers.start(key, launch.process_request()) {
            Ok(_) => {
                self.snapshot = HelperSnapshot {
                    status: HelperStatus::Starting,
                    player_address: None,
                };
                self.active_operation = Some(operation_id.clone());
                Ok(operation_id.as_str().to_string())
            }
            Err(error) => {
                let _ = self.operations.fail(
                    &operation_id,
                    lifecycle_error("broadcast_start_failed", error.to_string()),
                );
                Err(map_process_error(error))
            }
        }
    }

    pub fn observe_output(&mut self, line: &str) -> Result<(), XboxBroadcastError> {
        if let Some(prompt) = parse_broadcast_auth_prompt(line) {
            self.auth_prompt = Some(BroadcastAuthPrompt {
                code: prompt.code,
                link_url: prompt.url,
            });
        }
        if broadcast_is_ready(line) {
            self.helpers
                .record_ready(&self.key())
                .map_err(map_process_error)?;
            self.snapshot = HelperSnapshot {
                status: HelperStatus::Running,
                player_address: None,
            };
            if let Some(operation_id) = self.active_operation.take() {
                self.operations
                    .succeed(&operation_id, "Xbox Broadcast is ready.", BTreeMap::new())
                    .map_err(|error| XboxBroadcastError::Operation(error.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn dismiss_auth_prompt(&mut self) {
        self.auth_prompt = None;
    }

    pub fn stop(&mut self) -> Result<(), XboxBroadcastError> {
        self.helpers
            .request_graceful_stop(&self.key())
            .map_err(map_process_error)?;
        self.snapshot = HelperSnapshot::stopped();
        Ok(())
    }

    pub fn cancel_start_if_requested(&mut self) -> Result<bool, XboxBroadcastError> {
        let Some(operation_id) = self.active_operation.clone() else {
            return Ok(false);
        };
        if !(self.operations.cancellation_check(&operation_id))() {
            return Ok(false);
        }
        self.helpers
            .request_graceful_stop(&self.key())
            .map_err(map_process_error)?;
        self.operations
            .cancel(&operation_id, "Xbox Broadcast start cancelled.")
            .map_err(|error| XboxBroadcastError::Operation(error.to_string()))?;
        self.active_operation = None;
        self.snapshot = HelperSnapshot::stopped();
        Ok(true)
    }

    pub fn ready_timeout_elapsed(
        &mut self,
        seconds_waiting: u64,
    ) -> Result<bool, XboxBroadcastError> {
        if seconds_waiting < 60 || self.active_operation.is_none() {
            return Ok(false);
        }
        let operation_id = self.active_operation.take().expect("checked above");
        let _ = self.helpers.request_graceful_stop(&self.key());
        self.operations
            .fail(
                &operation_id,
                lifecycle_error(
                    "broadcast_ready_timeout",
                    "Xbox Broadcast did not become ready within 60 seconds.",
                ),
            )
            .map_err(|error| XboxBroadcastError::Operation(error.to_string()))?;
        self.snapshot.status = HelperStatus::TimedOut;
        Ok(true)
    }

    pub fn recover_after_restart(&mut self) {
        self.snapshot = HelperSnapshot::after_agent_restart();
        self.active_operation = None;
    }

    fn has_secret(&self, key: &str) -> Result<bool, XboxBroadcastError> {
        self.secrets
            .get(key)
            .map(|value| value.is_some_and(|value| !value.trim().is_empty()))
            .map_err(|error| XboxBroadcastError::SecretStore(error.to_string()))
    }

    fn key(&self) -> HelperKey {
        HelperKey::new(&self.server_id, "xbox-broadcast")
    }
}

fn map_process_error(error: HelperProcessError) -> XboxBroadcastError {
    XboxBroadcastError::Process(error.to_string())
}
