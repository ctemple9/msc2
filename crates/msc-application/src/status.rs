//! Status and performance snapshot models for the active Java server.

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleStatusSnapshot {
    pub running: bool,
    pub active_server_id: Option<String>,
    pub pid: Option<i64>,
    pub server_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceSnapshot {
    pub ts: String,
    pub tps_1m: Option<f64>,
    /// Paper-family's real 5m/15m rolling averages, parsed from the same
    /// `/tps` console reply as `tps_1m` (`msc_domain::tps::Sample`). `None`
    /// for single-value flavors (Forge/vanilla) and whenever no sample has
    /// arrived yet -- same absence rule as `tps_1m` itself.
    pub tps_5m: Option<f64>,
    pub tps_15m: Option<f64>,
    pub players_online: Option<i64>,
    pub cpu_percent: Option<f64>,
    pub ram_used_mb: Option<f64>,
    pub ram_max_mb: Option<f64>,
    pub world_size_mb: Option<f64>,
    pub server_type: Option<String>,
}
