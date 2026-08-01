//! `/v1/` wire DTOs — the serde structs `msc-agent`'s skeletal handlers
//! (P2.13/P2.14) serialize into HTTP responses. Hand-written against
//! `docs/msc2/api-contract/openapi.json` (P2.8), not generated — this
//! phase hand-writes both the Rust and (eventually) Swift sides of the
//! frozen schema, per this phase's own "not in this phase" note on a
//! generic codegen pipeline.
//!
//! Only the schemas the skeletal agent actually serves this phase are
//! covered: `OperationDTO`/`ErrorDTO` (operation lifecycle),
//! `CapabilitiesDTO`, and the status/health response shapes
//! (`RemoteAPIStatus`, `HealthResponseDTO`). Every other schema in
//! `openapi.json` belongs to a route this phase doesn't wire.

pub mod capabilities;
pub mod error;
pub mod health;
pub mod operation;
pub mod status;

pub use capabilities::*;
pub use error::*;
pub use health::*;
pub use operation::*;
pub use status::*;
