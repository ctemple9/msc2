pub mod addon_provider;
pub mod app_config_schema;
pub mod backup;
pub mod capability;
pub mod commands;
pub mod crash_analysis;
pub mod identity;
pub mod java_runtime;
pub mod launch_shape;
pub mod nbt;
pub mod network_safety;
pub mod operation;
pub mod plugin_source;
pub mod properties;
pub mod provisioning;
pub mod router;
pub mod server_versions;
pub mod settings_schema;
pub mod slug;
pub mod tps;
pub mod version;
pub mod world;

// Placeholder until P1.2 wires the real fixture-driven tests. Its only job
// is to prove `cargo build`/`cargo test` have something to run.
pub fn placeholder() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_passes() {
        assert!(placeholder());
    }
}
