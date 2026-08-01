//! Platform bindings for Windows. Windows Service registration, Job
//! Objects, and firewall handling (`msc2-engineering.md` §6's other three
//! items for this crate) are Phase 4 -- this crate currently ships only
//! the P3.10 `SecretStore` implementation.

#[cfg(target_os = "windows")]
pub mod secret_store;
