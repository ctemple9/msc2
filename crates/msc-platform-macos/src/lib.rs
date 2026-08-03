//! Platform bindings for macOS. This crate now ships the P3.9
//! `SecretStore` implementation, P4.10's Java process supervisor, and
//! P4.22's `launchd` LaunchDaemon registration. The VZ sidecar client
//! (`msc2-engineering.md` §6's remaining macOS item) stays Phase 10.

#[cfg(target_os = "macos")]
pub mod power;
#[cfg(target_os = "macos")]
pub mod process;
#[cfg(target_os = "macos")]
pub mod secret_store;
#[cfg(target_os = "macos")]
pub mod service;
