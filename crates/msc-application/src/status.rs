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
    pub players_online: Option<i64>,
    pub cpu_percent: Option<f64>,
    pub ram_used_mb: Option<f64>,
    pub ram_max_mb: Option<f64>,
    pub world_size_mb: Option<f64>,
    pub server_type: Option<String>,
}
