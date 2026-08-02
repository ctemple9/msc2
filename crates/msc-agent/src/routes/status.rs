//! `GET /v1/status` — lifecycle-backed status for the active Java server.

use axum::Json;
use axum::extract::State;
use msc_api::dto::{PerformanceMetricNumberDto, PerformanceSnapshotDto, RemoteApiStatus};
use msc_application::status::{LifecycleStatusSnapshot, PerformanceSnapshot};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct StatusRoutesState {
    inner: Arc<RwLock<StatusSnapshots>>,
}

#[derive(Debug, Clone)]
struct StatusSnapshots {
    status: LifecycleStatusSnapshot,
    performance: PerformanceSnapshot,
}

impl Default for StatusRoutesState {
    fn default() -> Self {
        Self::new(
            LifecycleStatusSnapshot {
                running: false,
                active_server_id: None,
                pid: None,
                server_type: None,
            },
            PerformanceSnapshot {
                ts: unix_timestamp_string(),
                tps_1m: None,
                players_online: Some(0),
                cpu_percent: None,
                ram_used_mb: None,
                ram_max_mb: None,
                world_size_mb: None,
                server_type: None,
            },
        )
    }
}

impl StatusRoutesState {
    pub fn new(status: LifecycleStatusSnapshot, performance: PerformanceSnapshot) -> Self {
        Self {
            inner: Arc::new(RwLock::new(StatusSnapshots {
                status,
                performance,
            })),
        }
    }

    #[allow(dead_code)]
    pub fn replace(&self, status: LifecycleStatusSnapshot, performance: PerformanceSnapshot) {
        *self.inner.write().unwrap() = StatusSnapshots {
            status,
            performance,
        };
    }

    pub fn status(&self) -> RemoteApiStatus {
        let snapshot = self.inner.read().unwrap().status.clone();
        RemoteApiStatus {
            running: snapshot.running,
            active_server_id: snapshot.active_server_id,
            pid: snapshot.pid,
            server_type: snapshot.server_type,
            docker_container_running: None,
            docker_container_status: None,
        }
    }

    pub fn performance(&self) -> PerformanceSnapshotDto {
        let snapshot = self.inner.read().unwrap().performance.clone();
        PerformanceSnapshotDto {
            ts: snapshot.ts,
            tps_1m: metric(snapshot.tps_1m, "performance.tps"),
            players_online: snapshot.players_online,
            cpu_percent: metric(snapshot.cpu_percent, "performance.cpu"),
            ram_used_mb: metric(snapshot.ram_used_mb, "performance.ram"),
            ram_max_mb: metric(snapshot.ram_max_mb, "performance.ram"),
            world_size_mb: metric(snapshot.world_size_mb, "performance.world-size"),
            server_type: snapshot.server_type,
        }
    }
}

pub async fn status(State(state): State<StatusRoutesState>) -> Json<RemoteApiStatus> {
    Json(state.status())
}

fn metric(value: Option<f64>, help_id: &'static str) -> Option<PerformanceMetricNumberDto> {
    value.map(|value| PerformanceMetricNumberDto {
        value,
        help_id: Some(help_id.to_string()),
    })
}

fn unix_timestamp_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_performance_routes_serialize_lifecycle_snapshots() {
        let state = StatusRoutesState::new(
            LifecycleStatusSnapshot {
                running: true,
                active_server_id: Some("paper-1".to_string()),
                pid: Some(51234),
                server_type: Some("paper".to_string()),
            },
            PerformanceSnapshot {
                ts: "2026-08-02T00:00:00Z".to_string(),
                tps_1m: Some(19.8),
                players_online: Some(2),
                cpu_percent: Some(37.5),
                ram_used_mb: Some(768.0),
                ram_max_mb: Some(2048.0),
                world_size_mb: Some(512.0),
                server_type: Some("paper".to_string()),
            },
        );

        let status = state.status();
        assert!(status.running);
        assert_eq!(status.active_server_id.as_deref(), Some("paper-1"));
        assert_eq!(status.pid, Some(51234));
        assert_eq!(status.server_type.as_deref(), Some("paper"));

        let performance = state.performance();
        assert_eq!(performance.ts, "2026-08-02T00:00:00Z");
        assert_eq!(performance.players_online, Some(2));
        assert_eq!(
            performance.tps_1m.as_ref().map(|metric| metric.value),
            Some(19.8)
        );
        assert_eq!(
            performance
                .tps_1m
                .as_ref()
                .and_then(|metric| metric.help_id.as_deref()),
            Some("performance.tps")
        );
        assert_eq!(
            performance
                .ram_max_mb
                .as_ref()
                .and_then(|metric| metric.help_id.as_deref()),
            Some("performance.ram")
        );
    }
}
