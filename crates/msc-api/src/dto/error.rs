//! `ErrorDTO` — `docs/msc2/api-contract/versioning-and-errors.md` §5-6's
//! single error envelope, used by every non-2xx `/v1/` response and reused
//! verbatim by `OperationDto.error` (`operation-model.md` §2).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorDto {
    /// Stable, machine-readable snake_case identifier — `not_found`,
    /// `conflict`, `invalid_body`, … Clients branch on this, never on
    /// `message` text.
    pub code: String,
    /// Human-readable, iOS-visible.
    pub message: String,
    #[serde(rename = "helpId", default, skip_serializing_if = "Option::is_none")]
    pub help_id: Option<String>,
    /// Free-form structured context — validation field names, conflicting
    /// version strings, retry-after seconds, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
