pub mod commands;
pub mod crash_analysis;
pub mod identity;
pub mod java_runtime;
pub mod properties;
pub mod settings_schema;
pub mod slug;
pub mod tps;
pub mod version;

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
