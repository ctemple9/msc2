//! Platform bindings for Linux. The `systemd` unit itself and cgroups
//! handling (`msc2-engineering.md` §6's other two items for this crate)
//! are Phase 4. This crate currently ships the P3.11 `SecretStore`
//! implementation plus P4.10's Java process supervisor.

#[cfg(unix)]
pub mod credential_helper;
#[cfg(target_os = "linux")]
pub mod power;
#[cfg(target_os = "linux")]
pub mod process;
#[cfg(target_os = "linux")]
pub mod secret_store;
#[cfg(unix)]
pub mod service;
