//! Terminal-native performance presentation.
//!
//! The agent already exposes the complete metric snapshot. This view keeps a
//! small in-memory history so a terminal can show direction without claiming
//! to be a pixel-for-pixel chart or inventing edition-specific values.

use std::time::{Duration, Instant};

use msc_api::dto::{PerformanceSnapshotDto, RemoteApiStatus};

use super::transport::SharedClient;
use crate::cli::CliError;

const HISTORY_LIMIT: usize = 24;

#[derive(Debug, Clone, Default)]
pub struct PerformanceSample {
    pub tps: Option<f64>,
    pub cpu: Option<f64>,
    pub ram_used_mb: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct PerformanceState {
    pub latest: Option<PerformanceSnapshotDto>,
    pub history: Vec<PerformanceSample>,
    pub loaded: bool,
    pub error: Option<String>,
    pub server_type: String,
    observed_running: Option<bool>,
    started_at: Option<Instant>,
    last_poll: Option<Instant>,
}

impl PerformanceState {
    pub async fn load(
        client: &SharedClient,
        status: Option<&RemoteApiStatus>,
        server_type: &str,
    ) -> Result<Self, CliError> {
        let snapshot: PerformanceSnapshotDto = client.get_json("/v1/performance").await?;
        let mut state = Self {
            latest: None,
            history: Vec::new(),
            loaded: true,
            error: None,
            server_type: server_type.to_string(),
            observed_running: None,
            started_at: None,
            last_poll: None,
        };
        state.record(snapshot, status.map(|value| value.running));
        Ok(state)
    }

    pub async fn refresh(
        &mut self,
        client: &SharedClient,
        running: Option<bool>,
    ) -> Result<(), CliError> {
        let snapshot: PerformanceSnapshotDto = client.get_json("/v1/performance").await?;
        self.record(snapshot, running);
        self.error = None;
        Ok(())
    }

    pub fn record(&mut self, snapshot: PerformanceSnapshotDto, running: Option<bool>) {
        self.server_type = snapshot
            .server_type
            .clone()
            .unwrap_or_else(|| self.server_type.clone());
        self.latest = Some(snapshot.clone());
        self.history.push(PerformanceSample {
            tps: snapshot.tps_1m.map(|metric| metric.value),
            cpu: snapshot.cpu_percent.map(|metric| metric.value),
            ram_used_mb: snapshot.ram_used_mb.map(|metric| metric.value),
        });
        if self.history.len() > HISTORY_LIMIT {
            let remove = self.history.len() - HISTORY_LIMIT;
            self.history.drain(..remove);
        }
        if let Some(running) = running {
            if running && self.observed_running == Some(false) {
                self.started_at = Some(Instant::now());
            }
            if !running {
                self.started_at = None;
            }
            self.observed_running = Some(running);
        }
        self.last_poll = Some(Instant::now());
    }

    pub fn should_poll(&self) -> bool {
        self.last_poll
            .is_none_or(|last| last.elapsed() >= Duration::from_secs(5))
    }

    pub fn current(&self) -> Option<&PerformanceSnapshotDto> {
        self.latest.as_ref()
    }

    pub fn uptime_label(&self) -> String {
        match (self.observed_running, self.started_at) {
            (Some(true), Some(started)) => format_duration(started.elapsed()),
            (Some(true), None) => "Running".to_string(),
            _ => "Offline".to_string(),
        }
    }

    pub fn runtime_note(&self) -> Option<&str> {
        self.latest
            .as_ref()
            .and_then(|snapshot| snapshot.runtime.as_ref())
            .filter(|runtime| runtime.state != "available")
            .and_then(|runtime| runtime.message.as_deref())
    }

    pub fn trend(&self, metric: TrendMetric) -> String {
        let values = self
            .history
            .iter()
            .filter_map(|sample| match metric {
                TrendMetric::Tps => sample.tps,
                TrendMetric::Cpu => sample.cpu,
                TrendMetric::Memory => sample.ram_used_mb,
            })
            .collect::<Vec<_>>();
        trend_line(&values)
    }

    pub fn status_label(&self) -> &'static str {
        match self.observed_running {
            Some(true) => "ONLINE",
            Some(false) => "OFFLINE",
            None => "UNKNOWN",
        }
    }

    pub fn status_detail(&self) -> String {
        match self.observed_running {
            Some(true) if self.server_type.eq_ignore_ascii_case("bedrock") => {
                "Bedrock runtime accepting connections".to_string()
            }
            Some(true) => "Java runtime accepting connections".to_string(),
            Some(false) => "Server is stopped".to_string(),
            None => "Server status was not reported".to_string(),
        }
    }

    pub fn poll_due(&self) -> bool {
        self.loaded && self.should_poll()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendMetric {
    Tps,
    Cpu,
    Memory,
}

pub fn trend_line(values: &[f64]) -> String {
    if values.is_empty() {
        return "no samples yet".to_string();
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max - min).max(f64::EPSILON);
    const LEVELS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    values
        .iter()
        .map(|value| {
            let index = (((value - min) / span) * (LEVELS.len() - 1) as f64).round() as usize;
            LEVELS[index.min(LEVELS.len() - 1)]
        })
        .collect()
}

pub fn format_metric(value: Option<f64>, unit: &str, decimals: usize) -> String {
    value.map_or_else(
        || "—".to_string(),
        |value| format!("{value:.decimals$}{unit}"),
    )
}

pub fn format_memory_mb(value: Option<f64>) -> String {
    value.map_or_else(
        || "—".to_string(),
        |value| format_bytes(value * 1024.0 * 1024.0),
    )
}

pub fn format_bytes(bytes: f64) -> String {
    if bytes >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} GB", bytes / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024.0 * 1024.0 {
        format!("{:.0} MB", bytes / (1024.0 * 1024.0))
    } else {
        format!("{bytes:.0} B")
    }
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else if minutes > 0 {
        format!("{minutes}m {seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}
