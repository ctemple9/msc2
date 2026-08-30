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

/// The transport outcomes shown by the first-start progress surface. These
/// are intentionally separate from helper process state: a user can skip a
/// transport, or the transport can be inapplicable to this server, without a
/// helper ever being started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstStartTransportState {
    Waiting,
    Ready,
    Skipped,
    Failed,
    NotApplicable,
}

impl FirstStartTransportState {
    pub fn is_resolved(self) -> bool {
        matches!(
            self,
            Self::Ready | Self::Skipped | Self::Failed | Self::NotApplicable
        )
    }
}

/// The server-side portion of MSC 1's `Initiate` workflow.
///
/// The client owns the setup sheet, but the agent owns the facts that make a
/// first run different from an ordinary start: which pass is active, whether
/// the Minecraft process has announced readiness, and whether every enabled
/// transport has reached a terminal outcome. Keeping this as a small state
/// machine makes crashes and repeated starts explicit instead of relying on a
/// UI flag that could disappear with a browser tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstStartPhase {
    PassOne,
    WaitingForSetup,
    PassTwo,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstStartCoordinator {
    pub server_id: String,
    pub phase: FirstStartPhase,
    pub server_ready: bool,
    pub playit: FirstStartTransportState,
    pub broadcast: FirstStartTransportState,
    auto_stop_issued: bool,
    safety_cap_reached: bool,
}

impl FirstStartCoordinator {
    pub fn new(
        server_id: impl Into<String>,
        playit_enabled: bool,
        broadcast_enabled: bool,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            phase: FirstStartPhase::PassOne,
            server_ready: false,
            playit: if playit_enabled {
                FirstStartTransportState::Waiting
            } else {
                FirstStartTransportState::NotApplicable
            },
            broadcast: if broadcast_enabled {
                FirstStartTransportState::Waiting
            } else {
                FirstStartTransportState::NotApplicable
            },
            auto_stop_issued: false,
            safety_cap_reached: false,
        }
    }

    pub fn needs_second_pass(&self) -> bool {
        !matches!(self.playit, FirstStartTransportState::NotApplicable)
            || !matches!(self.broadcast, FirstStartTransportState::NotApplicable)
    }

    pub fn begin_second_pass(&mut self) -> bool {
        if self.phase != FirstStartPhase::WaitingForSetup {
            return false;
        }
        self.phase = FirstStartPhase::PassTwo;
        self.server_ready = false;
        self.auto_stop_issued = false;
        self.safety_cap_reached = false;
        self.playit = if self.playit == FirstStartTransportState::NotApplicable {
            FirstStartTransportState::NotApplicable
        } else {
            FirstStartTransportState::Waiting
        };
        self.broadcast = if self.broadcast == FirstStartTransportState::NotApplicable {
            FirstStartTransportState::NotApplicable
        } else {
            FirstStartTransportState::Waiting
        };
        true
    }

    pub fn finish_first_pass(&mut self) {
        if self.phase == FirstStartPhase::WaitingForSetup {
            self.server_ready = false;
            self.auto_stop_issued = false;
        }
    }

    pub fn mark_server_ready(&mut self) -> bool {
        if self.server_ready {
            return false;
        }
        self.server_ready = true;
        if self.phase == FirstStartPhase::PassOne {
            self.phase = if self.needs_second_pass() {
                FirstStartPhase::WaitingForSetup
            } else {
                FirstStartPhase::Complete
            };
        }
        true
    }

    pub fn mark_transport(
        &mut self,
        transport: FirstRunTransport,
        state: FirstStartTransportState,
    ) -> bool {
        let destination = match transport {
            FirstRunTransport::Playit => &mut self.playit,
            FirstRunTransport::Broadcast => &mut self.broadcast,
        };
        if *destination == FirstStartTransportState::NotApplicable {
            return false;
        }
        *destination = state;
        true
    }

    pub fn all_transports_resolved(&self) -> bool {
        self.playit.is_resolved() && self.broadcast.is_resolved()
    }

    pub fn ready_to_stop(&self) -> bool {
        self.phase == FirstStartPhase::PassTwo
            && self.server_ready
            && self.all_transports_resolved()
    }

    pub fn issue_auto_stop(&mut self) -> bool {
        if self.auto_stop_issued {
            return false;
        }
        self.auto_stop_issued = true;
        true
    }

    pub fn mark_safety_cap_failures(&mut self) {
        self.safety_cap_reached = true;
        if self.playit == FirstStartTransportState::Waiting {
            self.playit = FirstStartTransportState::Failed;
        }
        if self.broadcast == FirstStartTransportState::Waiting {
            self.broadcast = FirstStartTransportState::Failed;
        }
    }

    pub fn complete(&mut self) {
        self.phase = FirstStartPhase::Complete;
    }

    pub fn safety_cap_reached(&self) -> bool {
        self.safety_cap_reached
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_start_pass_one_waits_for_setup_only_when_transport_is_enabled() {
        let mut local_only = FirstStartCoordinator::new("java-1", false, false);
        assert!(local_only.mark_server_ready());
        assert_eq!(local_only.phase, FirstStartPhase::Complete);

        let mut with_playit = FirstStartCoordinator::new("java-2", true, false);
        assert!(with_playit.mark_server_ready());
        assert_eq!(with_playit.phase, FirstStartPhase::WaitingForSetup);
        with_playit.finish_first_pass();
        assert!(with_playit.begin_second_pass());
        assert_eq!(with_playit.phase, FirstStartPhase::PassTwo);
        assert!(!with_playit.ready_to_stop());
        assert!(
            with_playit.mark_transport(FirstRunTransport::Playit, FirstStartTransportState::Ready)
        );
        assert!(with_playit.mark_server_ready());
        assert!(with_playit.ready_to_stop());
    }

    #[test]
    fn safety_cap_resolves_waiting_transports_for_cleanup() {
        let mut run = FirstStartCoordinator::new("java-3", true, true);
        assert!(run.mark_server_ready());
        run.finish_first_pass();
        assert!(run.begin_second_pass());
        run.mark_safety_cap_failures();
        assert!(run.safety_cap_reached());
        assert!(run.all_transports_resolved());
        assert_eq!(run.playit, FirstStartTransportState::Failed);
        assert_eq!(run.broadcast, FirstStartTransportState::Failed);
    }
}
