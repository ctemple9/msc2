//! `OperationDTO` — `operation-model.md` §2's wire shape for a long-running
//! operation, and the `state` enum from §3's closed state machine.
//!
//! Deliberately independent of `msc_domain::operation`: per
//! `msc2-engineering.md` §6's module-boundary rule, `msc-domain` carries no
//! serde dependency, so the wire representation is defined fresh here
//! rather than derived from the domain type. Converting between the two is
//! `msc-agent`'s job once P2.13/P2.14 wire real handlers.

use super::error::ErrorDto;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationStateDto {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// `progress` — `{ current, total }`, both non-negative. `null` while
/// `queued`, or for a `type` with no natural countable unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationProgressDto {
    pub current: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationDto {
    /// Opaque, server-generated. Never client-supplied.
    pub id: String,
    /// Kind of work, e.g. `demo-install`. Not a closed enum — new values
    /// are additive (§2).
    pub r#type: String,
    /// The thing the operation acts on, typically a server name/ID. `null`
    /// if the operation type has no natural target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub state: OperationStateDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<OperationProgressDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_line: Option<String>,
    /// Present only when `state == succeeded`. Shape defined per `type`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Present only when `state == failed`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDto>,
}
