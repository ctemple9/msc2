//! Focused confirmation state for disruptive TUI requests.
//!
//! The agent remains responsible for acknowledgement, permission checks, and
//! the mutation itself. This state only makes the target and consequence
//! visible before the request crosses the existing API boundary.

use crossterm::event::KeyCode;

use super::players::PlayerMutation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    StartServer,
    StopServer,
    CancelOperation { operation_id: String },
    PlayerMutation(PlayerMutation),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmationRequest {
    pub host: String,
    pub server: String,
    pub target: String,
    pub consequence: String,
    pub action: ConfirmAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationResult {
    Confirmed,
    Cancelled,
}

#[derive(Debug, Clone, Default)]
pub struct ConfirmationState {
    request: Option<ConfirmationRequest>,
}

impl ConfirmationState {
    pub fn begin(&mut self, request: ConfirmationRequest) {
        self.request = Some(request);
    }

    pub fn request(&self) -> Option<&ConfirmationRequest> {
        self.request.as_ref()
    }

    pub fn is_open(&self) -> bool {
        self.request.is_some()
    }

    pub fn resolve(&mut self, result: ConfirmationResult) -> Option<ConfirmationRequest> {
        let request = self.request.take();
        if request.is_some() {
            // Keeping the result in the key handler makes the state machine
            // explicit while the request itself remains the only payload.
            let _ = result;
        }
        request
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<ConfirmationResult> {
        match key {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.request.as_ref()?;
                Some(ConfirmationResult::Confirmed)
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                self.request.as_ref()?;
                Some(ConfirmationResult::Cancelled)
            }
            _ => None,
        }
    }
}
