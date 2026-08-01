//! Platform bindings for Linux. The `systemd` unit itself and cgroups
//! handling (`msc2-engineering.md` §6's other two items for this crate)
//! are Phase 4 -- this crate currently ships only the P3.11 `SecretStore`
//! implementation.

#[cfg(target_os = "linux")]
pub mod secret_store;
