//! Side-effect-free helper lifecycle rules. Process ownership is deliberately
//! deferred to P9.6; this module only makes state changes explicit and honest.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperStartDecision {
    NoAction,
    PromptSecretSetup,
    Launch,
}

pub fn decide_playit_start(enabled: bool, secret_present: bool) -> HelperStartDecision {
    match (enabled, secret_present) {
        (false, _) => HelperStartDecision::NoAction,
        (true, false) => HelperStartDecision::PromptSecretSetup,
        (true, true) => HelperStartDecision::Launch,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperStatus {
    Stopped,
    Starting,
    Running,
    UnknownUntilReconciled,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperSnapshot {
    pub status: HelperStatus,
    pub player_address: Option<String>,
}

impl HelperSnapshot {
    pub fn stopped() -> Self {
        Self {
            status: HelperStatus::Stopped,
            player_address: None,
        }
    }
    pub fn after_agent_restart() -> Self {
        Self {
            status: HelperStatus::UnknownUntilReconciled,
            player_address: None,
        }
    }
    pub fn on_exit(self) -> Self {
        Self::stopped()
    }
    pub fn on_ready(self, player_address: String) -> Self {
        Self {
            status: HelperStatus::Running,
            player_address: Some(player_address),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRunTransport {
    Playit,
    Broadcast,
}

pub fn first_run_needs_second_pass(
    playit_enabled: bool,
    broadcast_enabled: bool,
    server_ready: bool,
) -> bool {
    server_ready && (playit_enabled || broadcast_enabled)
}

pub fn first_run_timeout(
    transport: FirstRunTransport,
    seconds_waiting: u64,
) -> Option<HelperStatus> {
    let limit = match transport {
        FirstRunTransport::Playit => 75,
        FirstRunTransport::Broadcast => 60,
    };
    (seconds_waiting >= limit).then_some(HelperStatus::TimedOut)
}

pub fn first_run_safety_cap_reached(seconds_waiting: u64) -> bool {
    seconds_waiting >= 600
}
