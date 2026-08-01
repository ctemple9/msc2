//! Operation domain types: an opaque server-generated id, the operation
//! lifecycle state machine, in-flight progress, and the succeeded/failed
//! outcome.
//!
//! Greenfield MSC 2 construction, not a port — MSC 1 has no
//! operation-journal concept, so there is no baseline behavior to
//! characterize and no fixture to extract. The specification is
//! `docs/msc2/api-contract/operation-model.md` (P2.5), written before this
//! implementation; this module implements its §2 (`OperationDTO`) and §3
//! (the state machine) as far as pure domain types go.
//!
//! Wire serialization (the JSON `OperationDTO` shape, including `id`/
//! `type`/`target`) is `msc-api`'s job, not this crate's — per
//! `msc2-engineering.md` §6's module-boundary rule, `msc-domain` carries no
//! I/O and takes on no serde dependency. `OperationOutcome` reuses
//! `std::result::Result` rather than inventing a parallel type.

use std::collections::BTreeMap;

/// Opaque, server-generated identifier. Never client-supplied — see
/// operation-model.md §2. Format (ULID/UUID/etc.) is deliberately not this
/// type's concern; callers treat it as an opaque string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(String);

impl OperationId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The operation lifecycle. Closed set — operation-model.md §3 is the
/// transition table this type enforces; nothing outside it is legal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// An attempted transition operation-model.md §3 does not allow — e.g.
/// resuming a terminal operation, or skipping straight from `queued` to a
/// non-`cancelled` terminal state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IllegalTransition {
    pub from: OperationState,
    pub to: OperationState,
}

impl OperationState {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        match raw {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// `succeeded`/`failed`/`cancelled` accept no further transition —
    /// operation-model.md §3.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }

    fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Queued, Self::Running)
                | (Self::Running, Self::Succeeded)
                | (Self::Running, Self::Failed)
                | (Self::Queued, Self::Cancelled)
                | (Self::Running, Self::Cancelled)
        )
    }

    /// Applies a transition, rejecting anything outside operation-model.md
    /// §3's table — including any transition attempted out of a state
    /// where `is_terminal()` is already true.
    pub fn transition_to(self, next: Self) -> Result<Self, IllegalTransition> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(IllegalTransition {
                from: self,
                to: next,
            })
        }
    }
}

/// `{ current, total }` — operation-model.md §2. Deliberately just the two
/// counts, no derived percentage. Deliberately excludes the DTO's
/// `statusLine`: the contract keeps that field independently nullable (a
/// status line can exist before progress starts, or after it finishes), so
/// bundling it in here would force the two to appear and disappear
/// together when the wire contract says they don't.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationProgress {
    pub current: u64,
    pub total: u64,
}

/// Reuses P2.4's `ErrorDTO` shape (`code`, `message`, `helpId?`,
/// `details?`) rather than inventing an operation-specific failure type —
/// operation-model.md §2. `details` is a plain string map here; the
/// free-form-JSON wire shape it takes on the DTO is `msc-api`'s concern,
/// not this crate's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationError {
    pub code: String,
    pub message: String,
    pub help_id: Option<String>,
    pub details: BTreeMap<String, String>,
}

/// The DTO's `result?`/`error?` pair, collapsed into one type: `None`
/// while an operation is `queued`, `running`, or `cancelled` (cancellation
/// carries neither, per operation-model.md §2); `Some(Ok(_))` once
/// `succeeded`; `Some(Err(_))` once `failed`.
pub type OperationOutcome<T> = Option<Result<T, OperationError>>;
