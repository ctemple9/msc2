//! Route handlers for the skeletal `msc-agent`'s canned `/v1/` endpoints
//! (P2.13). Each handler returns a hard-coded example of its `msc-api`
//! DTO — there is no real server process to inspect yet, per this phase's
//! "Not in this phase" note in `rolling-plan.md`.

pub mod capabilities;
pub mod health;
pub mod operations;
pub mod status;
