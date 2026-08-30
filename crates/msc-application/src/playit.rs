//! Application service for one server's Playit Minecraft tunnel.
//!
//! This deliberately starts only `playitd`; it neither accepts nor forwards
//! MSC HTTP traffic.  The management listener remains independently bound and
//! bearer-authenticated by `msc-agent`.

use crate::operations::{LifecycleOperations, lifecycle_error};
use msc_domain::helper::{
    FirstRunTransport, HelperSnapshot, HelperStatus, decide_playit_start, first_run_timeout,
};
use msc_domain::networking::{
    PlayitTunnelAddresses, PlayitTunnelKind, PlayitTunnelSpec, parse_playit_address,
};
use msc_infrastructure::helper_process::{
    HelperKey, HelperProcessError, HelperProcessManager, ManagedHelperEvent, ManagedHelperStatus,
};
use msc_infrastructure::playit::{
    PLAYIT_SECRET_KEY, PlayitBinaryAcquisition, PlayitLaunch, PlayitSecretBridge,
};
use msc_infrastructure::playit_api::{
    PLAYIT_AGENT_NAME, PlayitApi, PlayitApiError, PlayitHttpTransport, PlayitTunnel,
};
use msc_infrastructure::process::ProcessSupervisor;
use msc_infrastructure::secret_store::SecretStore;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub const PLAYIT_OPERATION_TYPE: &str = "playit-tunnel";
pub const PLAYIT_SETUP_OPERATION_TYPE: &str = "playit-setup";
pub const PLAYIT_SETUP_OPERATION_TARGET: &str = "playit-account";

const PLAYIT_RESET_STOP_TIMEOUT: Duration = Duration::from_secs(5);
const PLAYIT_RESET_POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayitStartResult {
    pub operation_id: Option<String>,
    pub status: HelperStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayitLifecycleStatus {
    SetupRequired,
    Starting,
    WaitingForTunnels,
    Running,
    Stopping,
    Stopped,
    TimedOut,
    Failed { message: String },
}

impl PlayitLifecycleStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::SetupRequired => "Setup required",
            Self::Starting => "Starting",
            Self::WaitingForTunnels => "Waiting for tunnels",
            Self::Running => "Running",
            Self::Stopping => "Stopping",
            Self::Stopped => "Stopped",
            Self::TimedOut => "Timed out",
            Self::Failed { .. } => "Failed",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Starting | Self::WaitingForTunnels | Self::Running | Self::Stopping
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayitError {
    Disabled,
    MissingSecret,
    AlreadyManaged,
    Acquisition(String),
    SecretBridge(String),
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
            Self::SecretBridge(message) => {
                write!(f, "Playit secret bridge could not be created: {message}")
            }
            Self::SecretStore(message) | Self::Operation(message) | Self::Process(message) => {
                write!(f, "{message}")
            }
        }
    }
}

impl std::error::Error for PlayitError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayitSetupStage {
    SigningIn,
    ClaimingOrReusingAgent,
    WaitingForAgent,
    CreatingOrReusingJavaTunnel,
    CreatingOrReusingBedrockTunnel,
    CreatingOrReusingVoiceTunnel,
    ReceivingPublicAddresses,
}

impl PlayitSetupStage {
    pub fn status_line(self) -> &'static str {
        match self {
            Self::SigningIn => "Signing in to Playit.",
            Self::ClaimingOrReusingAgent => "Claiming or reusing the Playit agent.",
            Self::WaitingForAgent => "Waiting for Playit to finish setting up the agent.",
            Self::CreatingOrReusingJavaTunnel => "Creating or reusing the MSC Java tunnel.",
            Self::CreatingOrReusingBedrockTunnel => "Creating or reusing the MSC Bedrock tunnel.",
            Self::CreatingOrReusingVoiceTunnel => "Creating or reusing the MSC Voice tunnel.",
            Self::ReceivingPublicAddresses => "Receiving public Playit addresses.",
        }
    }

    pub fn progress(self) -> (u64, u64) {
        match self {
            Self::SigningIn => (1, 3),
            Self::ClaimingOrReusingAgent => (2, 3),
            Self::WaitingForAgent => (3, 3),
            Self::CreatingOrReusingJavaTunnel => (4, 7),
            Self::CreatingOrReusingBedrockTunnel => (5, 7),
            Self::CreatingOrReusingVoiceTunnel => (6, 7),
            Self::ReceivingPublicAddresses => (7, 7),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayitSetupResult {
    pub agent_id: String,
    pub reused_existing_agent: bool,
    pub tunnel_addresses: PlayitTunnelAddresses,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayitSetupError {
    Cancelled,
    Api(PlayitApiError),
    CredentialStore,
    AgentStart(String),
    TunnelMismatch(PlayitTunnelKind),
}

impl PlayitSetupError {
    pub fn stable_code(&self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Api(error) => error.stable_code(),
            Self::CredentialStore => "credential_store_failed",
            Self::AgentStart(_) => "playit_helper_start_failed",
            Self::TunnelMismatch(_) => "tunnel_mismatch",
        }
    }
}

impl fmt::Display for PlayitSetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => write!(f, "Playit setup was cancelled."),
            Self::Api(error) => error.fmt(f),
            Self::CredentialStore => {
                write!(f, "MSC could not save the Playit credentials on this host.")
            }
            Self::AgentStart(message) => {
                write!(f, "MSC could not start the Playit agent: {message}")
            }
            Self::TunnelMismatch(kind) => write!(
                f,
                "The existing {} Playit tunnel does not match this server's saved agent or local port; repair it on playit.gg before trying again.",
                kind.name()
            ),
        }
    }
}

impl std::error::Error for PlayitSetupError {}

/// Owns the native account workflow.  It receives an injected provider
/// transport and writes only the permanent key to `SecretStore`; the session
/// and password never leave this call as a return value.
pub struct PlayitAccountSetup<'a> {
    api: PlayitApi<'a>,
    secrets: &'a dyn SecretStore,
    agent_starter: Option<&'a dyn Fn() -> Result<(), String>>,
}

impl<'a> PlayitAccountSetup<'a> {
    pub fn new(transport: &'a dyn PlayitHttpTransport, secrets: &'a dyn SecretStore) -> Self {
        Self {
            api: PlayitApi::new(transport),
            secrets,
            agent_starter: None,
        }
    }

    /// Builds account setup with a temporary hook for launching the local
    /// agent before tunnel provisioning. The hook is borrowed only for the
    /// synchronous setup call and is never persisted with account state.
    pub fn with_agent_starter(
        transport: &'a dyn PlayitHttpTransport,
        secrets: &'a dyn SecretStore,
        agent_starter: &'a dyn Fn() -> Result<(), String>,
    ) -> Self {
        Self {
            api: PlayitApi::new(transport),
            secrets,
            agent_starter: Some(agent_starter),
        }
    }

    /// Signs in and either reuses the host's existing agent or performs the
    /// claim/setup/exchange handshake for a new agent.  `should_cancel` is
    /// checked between provider calls and while waiting for claim propagation.
    pub fn run(
        &self,
        email: &str,
        password: &str,
        existing_agent_id: Option<&str>,
        should_cancel: impl Fn() -> bool,
        report: impl Fn(PlayitSetupStage),
    ) -> Result<PlayitSetupResult, PlayitSetupError> {
        self.run_with_tunnels(
            email,
            password,
            existing_agent_id,
            &[],
            should_cancel,
            report,
        )
    }

    /// Runs account setup and, when a server supplies tunnel specs, provisions
    /// the exact named inventory for that server. The no-spec wrapper above
    /// keeps account-only callers useful for hosts without an active server.
    pub fn run_with_tunnels(
        &self,
        email: &str,
        password: &str,
        existing_agent_id: Option<&str>,
        tunnel_specs: &[PlayitTunnelSpec],
        should_cancel: impl Fn() -> bool,
        report: impl Fn(PlayitSetupStage),
    ) -> Result<PlayitSetupResult, PlayitSetupError> {
        report(PlayitSetupStage::SigningIn);
        let session = self
            .api
            .sign_in(email, password)
            .map_err(PlayitSetupError::Api)?;
        if should_cancel() {
            return Err(PlayitSetupError::Cancelled);
        }

        let existing_secret = self
            .secrets
            .get(PLAYIT_SECRET_KEY)
            .map_err(|_| PlayitSetupError::CredentialStore)?;
        report(PlayitSetupStage::ClaimingOrReusingAgent);
        let (agent_id, reused_existing_agent, secret_key) = if existing_agent_id
            .is_some_and(|agent_id| !agent_id.trim().is_empty())
            && existing_secret
                .as_deref()
                .is_some_and(|secret| !secret.trim().is_empty())
        {
            (
                existing_agent_id.expect("checked above").to_owned(),
                true,
                existing_secret.expect("checked above"),
            )
        } else {
            let claim_code = generate_claim_code();
            self.api
                .claim_setup(&claim_code)
                .map_err(PlayitSetupError::Api)?;

            let claimed_agent_id = self.accept_claim(&claim_code, &session, &should_cancel)?;
            report(PlayitSetupStage::WaitingForAgent);
            let secret = self.exchange_claim(&claim_code, &should_cancel)?;
            self.secrets
                .set(PLAYIT_SECRET_KEY, secret.as_str())
                .map_err(|_| PlayitSetupError::CredentialStore)?;
            (claimed_agent_id, false, secret.as_str().to_owned())
        };

        if !tunnel_specs.is_empty() {
            // MSC1 starts playitd before asking the provider to list or create
            // tunnels. The API reports AgentNotFound until that local agent
            // has connected, so provisioning must not run ahead of it.
            if should_cancel() {
                if !reused_existing_agent {
                    let _ = self.secrets.delete(PLAYIT_SECRET_KEY);
                }
                return Err(PlayitSetupError::Cancelled);
            }
            if reused_existing_agent {
                report(PlayitSetupStage::WaitingForAgent);
            }
            if let Some(agent_starter) = self.agent_starter
                && let Err(error) = agent_starter()
            {
                if !reused_existing_agent {
                    let _ = self.secrets.delete(PLAYIT_SECRET_KEY);
                }
                return Err(PlayitSetupError::AgentStart(error));
            }
        }

        let tunnel_addresses = if tunnel_specs.is_empty() {
            PlayitTunnelAddresses::default()
        } else {
            match self.provision_tunnels(
                &agent_id,
                &secret_key,
                &session,
                tunnel_specs,
                &should_cancel,
                &report,
            ) {
                Ok(addresses) => addresses,
                Err(error) => {
                    if !reused_existing_agent {
                        let _ = self.secrets.delete(PLAYIT_SECRET_KEY);
                    }
                    return Err(error);
                }
            }
        };

        Ok(PlayitSetupResult {
            agent_id,
            reused_existing_agent,
            tunnel_addresses,
        })
    }

    fn provision_tunnels(
        &self,
        agent_id: &str,
        secret_key: &str,
        session: &msc_infrastructure::playit_api::PlayitSession,
        tunnel_specs: &[PlayitTunnelSpec],
        should_cancel: &impl Fn() -> bool,
        report: &impl Fn(PlayitSetupStage),
    ) -> Result<PlayitTunnelAddresses, PlayitSetupError> {
        let mut inventory = self.list_tunnels(secret_key, should_cancel)?;
        for spec in tunnel_specs {
            report(stage_for_tunnel(spec.kind));
            let matching: Vec<&PlayitTunnel> = inventory
                .iter()
                .filter(|tunnel| tunnel.name == spec.kind.name())
                .collect();
            match matching.as_slice() {
                [tunnel] if tunnel_matches(tunnel, *spec, agent_id) => {}
                [] => self.create_tunnel(agent_id, *spec, session, should_cancel)?,
                _ => return Err(PlayitSetupError::TunnelMismatch(spec.kind)),
            }
            if should_cancel() {
                return Err(PlayitSetupError::Cancelled);
            }
        }

        report(PlayitSetupStage::ReceivingPublicAddresses);
        inventory = self.list_tunnels(secret_key, should_cancel)?;
        let mut addresses = PlayitTunnelAddresses::default();
        for spec in tunnel_specs {
            let matching: Vec<&PlayitTunnel> = inventory
                .iter()
                .filter(|tunnel| tunnel.name == spec.kind.name())
                .collect();
            let tunnel = match matching.as_slice() {
                [] => return Err(PlayitSetupError::Api(PlayitApiError::AgentNotFound)),
                [tunnel] if tunnel_matches(tunnel, *spec, agent_id) => tunnel,
                _ => return Err(PlayitSetupError::TunnelMismatch(spec.kind)),
            };
            let address = tunnel.active.then(|| {
                msc_domain::networking::playit_public_address(
                    spec.kind,
                    tunnel.assigned_domain.as_deref(),
                    tunnel.static_ip4.as_deref(),
                    tunnel.port_start,
                )
            });
            let address = address.flatten();
            match spec.kind {
                PlayitTunnelKind::Java => addresses.java = address,
                PlayitTunnelKind::Bedrock => addresses.bedrock = address,
                PlayitTunnelKind::Voice => addresses.voice = address,
            }
        }
        Ok(addresses)
    }

    fn list_tunnels(
        &self,
        secret_key: &str,
        should_cancel: &impl Fn() -> bool,
    ) -> Result<Vec<PlayitTunnel>, PlayitSetupError> {
        const TUNNEL_ATTEMPTS: usize = 16;
        for attempt in 0..TUNNEL_ATTEMPTS {
            if should_cancel() {
                return Err(PlayitSetupError::Cancelled);
            }
            match self.api.list_tunnels(secret_key) {
                Ok(tunnels) => return Ok(tunnels),
                Err(error) if retryable_tunnel_error(error) => {
                    if attempt + 1 < TUNNEL_ATTEMPTS && wait_for_tunnel_retry(should_cancel) {
                        return Err(PlayitSetupError::Cancelled);
                    }
                }
                Err(error) => return Err(PlayitSetupError::Api(error)),
            }
        }
        Err(PlayitSetupError::Api(PlayitApiError::AgentNotFound))
    }

    fn create_tunnel(
        &self,
        agent_id: &str,
        spec: PlayitTunnelSpec,
        session: &msc_infrastructure::playit_api::PlayitSession,
        should_cancel: &impl Fn() -> bool,
    ) -> Result<(), PlayitSetupError> {
        const TUNNEL_ATTEMPTS: usize = 16;
        for attempt in 0..TUNNEL_ATTEMPTS {
            if should_cancel() {
                return Err(PlayitSetupError::Cancelled);
            }
            match self
                .api
                .create_tunnel(agent_id, spec.kind, spec.local_port, session)
            {
                Ok(()) => return Ok(()),
                Err(error) if retryable_tunnel_error(error) => {
                    if attempt + 1 < TUNNEL_ATTEMPTS && wait_for_tunnel_retry(should_cancel) {
                        return Err(PlayitSetupError::Cancelled);
                    }
                }
                Err(error) => return Err(PlayitSetupError::Api(error)),
            }
        }
        Err(PlayitSetupError::Api(PlayitApiError::AgentNotFound))
    }

    fn accept_claim(
        &self,
        claim_code: &str,
        session: &msc_infrastructure::playit_api::PlayitSession,
        should_cancel: &impl Fn() -> bool,
    ) -> Result<String, PlayitSetupError> {
        const CLAIM_ATTEMPTS: usize = 15;
        for attempt in 0..CLAIM_ATTEMPTS {
            if should_cancel() {
                return Err(PlayitSetupError::Cancelled);
            }
            if attempt > 0 && wait_for_claim_retry(should_cancel) {
                return Err(PlayitSetupError::Cancelled);
            }
            if attempt > 0 {
                self.api
                    .claim_setup(claim_code)
                    .map_err(PlayitSetupError::Api)?;
            }
            match self.api.claim_details(claim_code, session) {
                Ok(()) => match self
                    .api
                    .claim_accept(claim_code, PLAYIT_AGENT_NAME, session)
                {
                    Ok(agent_id) => return Ok(agent_id),
                    Err(error) if retryable_claim_error(error) => continue,
                    Err(error) => return Err(PlayitSetupError::Api(error)),
                },
                Err(error) if retryable_claim_error(error) => continue,
                Err(error) => return Err(PlayitSetupError::Api(error)),
            }
        }
        Err(PlayitSetupError::Api(PlayitApiError::AgentNotFound))
    }

    fn exchange_claim(
        &self,
        claim_code: &str,
        should_cancel: &impl Fn() -> bool,
    ) -> Result<msc_infrastructure::playit_api::PlayitSecret, PlayitSetupError> {
        const EXCHANGE_ATTEMPTS: usize = 20;
        for attempt in 0..EXCHANGE_ATTEMPTS {
            if should_cancel() {
                return Err(PlayitSetupError::Cancelled);
            }
            if let Some(secret) = self
                .api
                .claim_exchange(claim_code)
                .map_err(PlayitSetupError::Api)?
            {
                return Ok(secret);
            }
            self.api
                .claim_setup(claim_code)
                .map_err(PlayitSetupError::Api)?;
            if attempt + 1 < EXCHANGE_ATTEMPTS && wait_for_claim_retry(should_cancel) {
                return Err(PlayitSetupError::Cancelled);
            }
        }
        Err(PlayitSetupError::Api(PlayitApiError::AgentNotFound))
    }
}

fn stage_for_tunnel(kind: PlayitTunnelKind) -> PlayitSetupStage {
    match kind {
        PlayitTunnelKind::Java => PlayitSetupStage::CreatingOrReusingJavaTunnel,
        PlayitTunnelKind::Bedrock => PlayitSetupStage::CreatingOrReusingBedrockTunnel,
        PlayitTunnelKind::Voice => PlayitSetupStage::CreatingOrReusingVoiceTunnel,
    }
}

fn tunnel_matches(tunnel: &PlayitTunnel, spec: PlayitTunnelSpec, agent_id: &str) -> bool {
    let common_origin = tunnel.origin_type.as_deref() == Some("agent")
        && tunnel.agent_id.as_deref() == Some(agent_id)
        && tunnel.local_ip.as_deref() == Some("127.0.0.1")
        && tunnel.local_port == Some(spec.local_port)
        && tunnel.port_type.as_deref() == Some(spec.kind.port_type());
    if !common_origin {
        return false;
    }
    match spec.kind {
        PlayitTunnelKind::Java | PlayitTunnelKind::Bedrock => {
            tunnel.tunnel_type.as_deref() == spec.kind.tunnel_type()
        }
        PlayitTunnelKind::Voice => {
            // The legacy `/tunnels/list` account model identifies custom UDP
            // tunnels with no tunnel type and does not include the
            // raw-ports protocol marker. The common checks above still bind
            // this named tunnel to this agent, loopback, UDP, and port.
            tunnel.tunnel_type.is_none()
        }
    }
}

fn generate_claim_code() -> String {
    Uuid::new_v4().simple().to_string()[..10].to_owned()
}

fn retryable_tunnel_error(error: PlayitApiError) -> bool {
    matches!(error, PlayitApiError::AgentNotFound)
}

fn retryable_claim_error(error: PlayitApiError) -> bool {
    matches!(
        error,
        PlayitApiError::AgentNotFound | PlayitApiError::ApiFailure
    )
}

fn wait_for_claim_retry(should_cancel: &impl Fn() -> bool) -> bool {
    for _ in 0..6 {
        if should_cancel() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn wait_for_tunnel_retry(should_cancel: &impl Fn() -> bool) -> bool {
    // MSC1 allowed roughly 24 seconds for a newly started agent to register
    // before giving up. Keep cancellation responsive while preserving that
    // provider propagation window between the sixteen API attempts.
    for _ in 0..15 {
        if should_cancel() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// A small façade around P9.6's common process manager.  It owns no secret
/// values: it asks `SecretStore` only whether the stable Playit key exists.
pub struct PlayitService<'a> {
    server_id: String,
    enabled: bool,
    supervisor: &'a dyn ProcessSupervisor,
    helpers: HelperProcessManager<'a>,
    secrets: &'a dyn SecretStore,
    operations: &'a LifecycleOperations<'a>,
    snapshot: HelperSnapshot,
    expecting_address: bool,
    active_operation: Option<msc_domain::operation::OperationId>,
    active_stop_operation: Option<msc_domain::operation::OperationId>,
    lifecycle_status: PlayitLifecycleStatus,
    secret_bridge: Option<PlayitSecretBridge>,
    secret_bridge_path: Option<PathBuf>,
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
            supervisor,
            helpers: HelperProcessManager::new(supervisor),
            secrets,
            operations,
            snapshot: HelperSnapshot::stopped(),
            expecting_address: false,
            active_operation: None,
            active_stop_operation: None,
            lifecycle_status: PlayitLifecycleStatus::Stopped,
            secret_bridge: None,
            secret_bridge_path: None,
        }
    }

    pub fn status(&self) -> &HelperSnapshot {
        &self.snapshot
    }

    pub fn lifecycle_status(&self) -> PlayitLifecycleStatus {
        if self.lifecycle_status == PlayitLifecycleStatus::Stopped
            && self.enabled
            && !self.has_secret().unwrap_or(false)
        {
            return PlayitLifecycleStatus::SetupRequired;
        }
        self.lifecycle_status.clone()
    }

    pub fn is_active(&self) -> bool {
        self.lifecycle_status().is_active()
    }

    /// The first-start coordinator needs a provider-neutral answer rather
    /// than the helper's lower-level `Starting`/`Running` label. A Playit
    /// transport is ready only after the managed process has emitted a safe
    /// player address; process spawn alone is not enough.
    pub fn first_start_ready(&self) -> bool {
        self.lifecycle_status() == PlayitLifecycleStatus::Running
            && self.snapshot.player_address.is_some()
    }

    pub fn diagnostics(&self) -> Vec<msc_infrastructure::helper_process::HelperDiagnostic> {
        self.helpers
            .snapshot(&self.key())
            .map(|snapshot| snapshot.diagnostics)
            .unwrap_or_default()
    }

    pub fn has_secret(&self) -> Result<bool, PlayitError> {
        self.secrets
            .get(PLAYIT_SECRET_KEY)
            .map(|secret| secret.is_some_and(|value| !value.trim().is_empty()))
            .map_err(|error| PlayitError::SecretStore(error.to_string()))
    }

    /// Leaves a failed start visible to status readers and clears any local
    /// readiness state.  The lifecycle route uses this for failures that
    /// happen before `start` can create its operation, such as an unsupported
    /// helper platform.
    pub fn record_start_failure(&mut self, message: impl Into<String>) {
        self.expecting_address = false;
        self.snapshot.status = HelperStatus::Stopped;
        self.lifecycle_status = PlayitLifecycleStatus::Failed {
            message: message.into(),
        };
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
        let secret = self
            .secrets
            .get(PLAYIT_SECRET_KEY)
            .map_err(|error| PlayitError::SecretStore(error.to_string()))?
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().to_owned());
        match decide_playit_start(self.enabled, secret.is_some()) {
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
                self.record_start_failure(message.clone());
                return Err(PlayitError::Acquisition(message));
            }
        };
        let secret_bridge =
            match launch.write_secret_bridge(secret.as_deref().expect("checked above")) {
                Ok(bridge) => bridge,
                Err(error) => {
                    let message = error.to_string();
                    let _ = self.operations.fail(
                        &operation_id,
                        lifecycle_error("playit_secret_bridge_failed", message.clone()),
                    );
                    self.record_start_failure(message.clone());
                    return Err(PlayitError::SecretBridge(message));
                }
            };
        self.reset_stopped_helper_manager();
        match self
            .helpers
            .start(key, launch.process_request(&acquired.artifact.path))
        {
            Ok(_) => {
                self.snapshot.status = HelperStatus::Starting;
                self.lifecycle_status = PlayitLifecycleStatus::Starting;
                self.active_operation = Some(operation_id.clone());
                self.active_stop_operation = None;
                self.secret_bridge_path = Some(secret_bridge.path().to_owned());
                self.secret_bridge = Some(secret_bridge);
                Ok(PlayitStartResult {
                    operation_id: Some(operation_id.as_str().to_string()),
                    status: HelperStatus::Starting,
                })
            }
            Err(error) => {
                let _ = secret_bridge.remove();
                let message = error.to_string();
                let _ = self.operations.fail(
                    &operation_id,
                    lifecycle_error("playit_start_failed", message.clone()),
                );
                self.record_start_failure(message);
                Err(map_process_error(error))
            }
        }
    }

    /// Feed one already-framed helper line into the provider-specific parser.
    /// This is where a tunnel's readiness is established, never at process spawn.
    pub fn observe_output(&mut self, line: &str) -> Result<(), PlayitError> {
        self.observe_output_line(line, true)
    }

    fn observe_output_line(
        &mut self,
        line: &str,
        record_helper_ready: bool,
    ) -> Result<(), PlayitError> {
        if line.to_ascii_lowercase().contains("tunnel setup") {
            self.expecting_address = true;
            if self.lifecycle_status == PlayitLifecycleStatus::Starting {
                self.lifecycle_status = PlayitLifecycleStatus::WaitingForTunnels;
            }
            return Ok(());
        }
        let address = parse_playit_address(line, self.expecting_address);
        self.expecting_address = false;
        if let Some(address) = address {
            let key = self.key();
            if record_helper_ready {
                self.helpers.record_ready(&key).map_err(map_process_error)?;
            }
            self.snapshot = self.snapshot.clone().on_ready(address.clone());
            self.lifecycle_status = PlayitLifecycleStatus::Running;
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

    /// Drain the process supervisor without relying on an HTTP status request
    /// to make progress. Output is redacted before it reaches diagnostics and
    /// the readiness parser, and exits reconcile the operation immediately.
    pub fn poll(&mut self) -> Result<(), PlayitError> {
        let secret = self
            .secrets
            .get(PLAYIT_SECRET_KEY)
            .ok()
            .flatten()
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().to_owned());
        let events = self
            .helpers
            .poll_with_redactor(|line| redact_secret(line, secret.as_deref()))
            .map_err(map_process_error)?;
        for event in events {
            match event {
                ManagedHelperEvent::Output { line, .. } => {
                    self.observe_output_line(&line, false)?;
                }
                ManagedHelperEvent::Exited(exit) => self.reconcile_exit(exit)?,
            }
        }
        if self.lifecycle_status == PlayitLifecycleStatus::Starting
            && matches!(
                self.helpers.snapshot(&self.key()).map(|value| value.status),
                Some(msc_infrastructure::helper_process::ManagedHelperStatus::Starting)
            )
        {
            self.lifecycle_status = PlayitLifecycleStatus::WaitingForTunnels;
        }
        Ok(())
    }

    fn reconcile_exit(
        &mut self,
        exit: msc_infrastructure::process::ProcessExitStatus,
    ) -> Result<(), PlayitError> {
        self.cleanup_secret_bridge()?;
        let stop_operation = self.active_stop_operation.take();
        if let Some(operation_id) = stop_operation {
            if exit.success() {
                self.operations
                    .succeed(&operation_id, "Playit tunnel stopped.", BTreeMap::new())
                    .map_err(|error| PlayitError::Operation(error.to_string()))?;
                self.lifecycle_status = self.stopped_lifecycle_status();
            } else {
                let message = exit_message(exit);
                self.operations
                    .fail(
                        &operation_id,
                        lifecycle_error("playit_stop_failed", message.clone()),
                    )
                    .map_err(|error| PlayitError::Operation(error.to_string()))?;
                self.lifecycle_status = PlayitLifecycleStatus::Failed { message };
            }
        } else if let Some(operation_id) = self.active_operation.take() {
            let error = if exit.success() {
                lifecycle_error(
                    "playit_exited_before_ready",
                    "Playit stopped before it reported a usable player address.",
                )
            } else {
                lifecycle_error("playit_helper_failed", exit_message(exit))
            };
            let message = error.message.clone();
            self.operations
                .fail(&operation_id, error)
                .map_err(|error| PlayitError::Operation(error.to_string()))?;
            self.lifecycle_status = PlayitLifecycleStatus::Failed { message };
        } else if exit.success() {
            self.lifecycle_status = self.stopped_lifecycle_status();
        } else {
            self.lifecycle_status = PlayitLifecycleStatus::Failed {
                message: exit_message(exit),
            };
        }
        self.snapshot = HelperSnapshot::stopped();
        Ok(())
    }

    pub fn stop(&mut self) -> Result<Option<String>, PlayitError> {
        if !self.lifecycle_status().is_active() {
            return Ok(None);
        }
        if self.lifecycle_status() == PlayitLifecycleStatus::Stopping {
            return Ok(self
                .active_stop_operation
                .as_ref()
                .map(|operation_id| operation_id.as_str().to_owned()));
        }
        let key = self.key();
        if let Some(operation_id) = self.active_operation.clone() {
            if let Err(error) = self.helpers.request_graceful_stop(&key) {
                let message = error.to_string();
                let _ = self.helpers.force_terminate(&key);
                let _ = self.operations.fail(
                    &operation_id,
                    lifecycle_error("playit_stop_failed", message.clone()),
                );
                self.active_operation = None;
                self.record_start_failure(message);
                return Err(map_process_error(error));
            }
            self.operations
                .cancel(&operation_id, "Playit tunnel start stopped.")
                .map_err(|error| PlayitError::Operation(error.to_string()))?;
            self.active_operation = None;
        }
        let operation_id = self
            .operations
            .begin_running(
                PLAYIT_OPERATION_TYPE,
                Some(key.operation_target()),
                "Stopping Playit tunnel.",
            )
            .map_err(|error| PlayitError::Operation(error.to_string()))?;
        if self.active_operation.is_none() {
            // A start operation already requested graceful shutdown above.
            // Running helpers still need the stop request here.
            let helper_status = self.helpers.snapshot(&key).map(|snapshot| snapshot.status);
            if !matches!(
                helper_status,
                Some(msc_infrastructure::helper_process::ManagedHelperStatus::Stopping)
            ) && let Err(error) = self.helpers.request_graceful_stop(&key)
            {
                let message = error.to_string();
                let _ = self.helpers.force_terminate(&key);
                let _ = self.operations.fail(
                    &operation_id,
                    lifecycle_error("playit_stop_failed", message.clone()),
                );
                self.record_start_failure(message);
                return Err(map_process_error(error));
            }
        }
        self.lifecycle_status = PlayitLifecycleStatus::Stopping;
        self.snapshot.status = HelperStatus::Starting;
        self.active_stop_operation = Some(operation_id.clone());
        Ok(Some(operation_id.as_str().to_owned()))
    }

    /// Reset removes this service's host-local bridge. The route removes the
    /// shared host key after every service has stopped. The helper is stopped
    /// synchronously because clearing its bridge while it is still alive would
    /// let a later setup race the old process. A short grace request is
    /// followed by force termination and bounded event reconciliation.
    pub fn reset(&mut self) -> Result<(), PlayitError> {
        let key = self.key();
        self.poll()?;

        if self.helper_is_live(&key) {
            let _ = self.helpers.request_graceful_stop(&key);
            if self.helper_is_live(&key) {
                match self.helpers.force_terminate(&key) {
                    Ok(()) | Err(HelperProcessError::NotRunning(_)) => {}
                    Err(error) => return Err(map_process_error(error)),
                }
            }

            let deadline = Instant::now() + PLAYIT_RESET_STOP_TIMEOUT;
            loop {
                self.poll()?;
                if !self.helper_is_live(&key) {
                    break;
                }
                if Instant::now() >= deadline {
                    return Err(PlayitError::Process(
                        "Playit helper did not stop during reset.".into(),
                    ));
                }
                std::thread::sleep(PLAYIT_RESET_POLL_INTERVAL);
            }
        }

        self.cleanup_secret_bridge()?;
        self.expecting_address = false;
        self.snapshot = HelperSnapshot::stopped();
        self.lifecycle_status = if self.enabled {
            PlayitLifecycleStatus::SetupRequired
        } else {
            PlayitLifecycleStatus::Stopped
        };
        Ok(())
    }

    fn helper_is_live(&self, key: &HelperKey) -> bool {
        matches!(
            self.helpers.snapshot(key).map(|snapshot| snapshot.status),
            Some(
                ManagedHelperStatus::Starting
                    | ManagedHelperStatus::Running
                    | ManagedHelperStatus::Stopping
            )
        )
    }

    /// Recovery never trusts a former PID.  The caller must reconcile with
    /// Playit before starting a fresh tunnel after an agent restart.
    pub fn recover_after_restart(&mut self) {
        let _ = self.cleanup_secret_bridge();
        self.helpers.discard_live_processes_after_restart();
        self.snapshot = HelperSnapshot::after_agent_restart();
        self.lifecycle_status = PlayitLifecycleStatus::Stopped;
        self.active_operation = None;
        self.active_stop_operation = None;
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
        self.lifecycle_status = PlayitLifecycleStatus::Stopping;
        self.snapshot.status = HelperStatus::Starting;
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
        let _ = self.helpers.request_graceful_stop(&self.key());
        self.snapshot.status = HelperStatus::TimedOut;
        self.lifecycle_status = PlayitLifecycleStatus::TimedOut;
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

    /// First-start naming for the existing 75-second technical watchdog.
    /// Keeping this as a façade lets the coordinator use the same timeout
    /// rule without creating a second timer or changing normal helper runs.
    pub fn first_start_timeout_elapsed(
        &mut self,
        seconds_waiting: u64,
    ) -> Result<bool, PlayitError> {
        self.ready_timeout_elapsed(seconds_waiting)
    }

    fn key(&self) -> HelperKey {
        HelperKey::new(&self.server_id, "playit")
    }

    fn reset_stopped_helper_manager(&mut self) {
        let status = self
            .helpers
            .snapshot(&self.key())
            .map(|snapshot| snapshot.status);
        if status.is_none_or(|status| {
            matches!(
                status,
                ManagedHelperStatus::Stopped | ManagedHelperStatus::Failed { .. }
            )
        }) {
            self.helpers = HelperProcessManager::new(self.supervisor);
        }
    }

    fn stopped_lifecycle_status(&self) -> PlayitLifecycleStatus {
        if self.enabled && !self.has_secret().unwrap_or(false) {
            PlayitLifecycleStatus::SetupRequired
        } else {
            PlayitLifecycleStatus::Stopped
        }
    }

    fn cleanup_secret_bridge(&mut self) -> Result<(), PlayitError> {
        if let Some(bridge) = self.secret_bridge.take() {
            bridge
                .remove()
                .map_err(|error| PlayitError::SecretBridge(error.to_string()))?;
        } else if let Some(path) = self.secret_bridge_path.as_deref() {
            PlayitSecretBridge::remove_path(path)
                .map_err(|error| PlayitError::SecretBridge(error.to_string()))?;
        }
        self.secret_bridge_path = None;
        Ok(())
    }
}

fn redact_secret(line: &str, secret: Option<&str>) -> String {
    secret.filter(|secret| !secret.is_empty()).map_or_else(
        || line.to_owned(),
        |secret| line.replace(secret, "<redacted>"),
    )
}

fn exit_message(exit: msc_infrastructure::process::ProcessExitStatus) -> String {
    match (exit.code, exit.signal) {
        (Some(code), _) => format!("Playit helper exited with code {code}."),
        (_, Some(signal)) => format!("Playit helper was terminated by signal {signal}."),
        (None, None) => "Playit helper exited without a status.".to_owned(),
    }
}

fn map_process_error(error: HelperProcessError) -> PlayitError {
    match error {
        HelperProcessError::AlreadyManaged(_) => PlayitError::AlreadyManaged,
        other => PlayitError::Process(other.to_string()),
    }
}
