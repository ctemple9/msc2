//! `GET /v1/performance` — latest bounded performance snapshot for the
//! active Java server.

use axum::Json;
use axum::extract::State;
use msc_api::dto::{PerformanceMetricNumberDto, PerformanceSnapshotDto};

use crate::routes::lifecycle::LifecycleRoutesState;

pub async fn performance(
    State(state): State<LifecycleRoutesState>,
) -> Json<PerformanceSnapshotDto> {
    let snapshot = state.performance_snapshot();
    Json(PerformanceSnapshotDto {
        ts: snapshot.ts,
        tps_1m: metric(snapshot.tps_1m, "performance.tps"),
        players_online: snapshot.players_online,
        cpu_percent: metric(snapshot.cpu_percent, "performance.cpu"),
        ram_used_mb: metric(snapshot.ram_used_mb, "performance.ram"),
        ram_max_mb: metric(snapshot.ram_max_mb, "performance.ram"),
        world_size_mb: metric(snapshot.world_size_mb, "performance.world-size"),
        server_type: snapshot.server_type,
        runtime: state
            .active_config_server()
            .filter(|server| server.server_type == msc_domain::identity::ServerType::Bedrock)
            .map(|_| state.bedrock_runtime_state()),
    })
}

fn metric(value: Option<f64>, help_id: &'static str) -> Option<PerformanceMetricNumberDto> {
    value.map(|value| PerformanceMetricNumberDto {
        value,
        help_id: Some(help_id.to_string()),
    })
}
