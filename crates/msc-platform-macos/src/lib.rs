//! Platform bindings for macOS. `launchd` registration and the VZ sidecar
//! client (`msc2-engineering.md` §6's other two items for this crate) are
//! Phase 4 and Phase 10. This crate currently ships the P3.9
//! `SecretStore` implementation plus P4.10's Java process supervisor.

#[cfg(target_os = "macos")]
pub mod process;
#[cfg(target_os = "macos")]
pub mod secret_store;
