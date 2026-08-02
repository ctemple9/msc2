//! Platform bindings for Windows. Windows Service registration, Job
//! Objects, and firewall handling (`msc2-engineering.md` §6's other three
//! items for this crate) are Phase 4. This crate currently ships the P3.10
//! `SecretStore` implementation plus P4.10's Java process supervisor.

#[cfg(target_os = "windows")]
pub mod power;
#[cfg(target_os = "windows")]
pub mod process;
#[cfg(target_os = "windows")]
pub mod secret_store;
pub mod service;
