//! Live server metrics, with a small history owned by this TUI session.

use std::collections::VecDeque;
use std::io::{self, Stdout};

use crossterm::style::Color;

use super::{Snapshot, paint};

const MAX_SAMPLES: usize = 120;
const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

#[derive(Default)]
pub(super) struct PerformanceUi {
    server_id: Option<String>,
    last_timestamp: Option<String>,
    tps: VecDeque<Option<f64>>,
    cpu: VecDeque<Option<f64>>,
    memory_used_mb: VecDeque<Option<f64>>,
}

impl PerformanceUi {
    pub(super) fn record(&mut self, snapshot: &Snapshot) {
        if self.server_id != snapshot.active_server_id {
            self.server_id = snapshot.active_server_id.clone();
            self.last_timestamp = None;
            self.tps.clear();
            self.cpu.clear();
            self.memory_used_mb.clear();
        }
        let Some(metric) = snapshot.performance.as_ref() else {
            return;
        };
        if self.server_id.is_none() || self.last_timestamp.as_deref() == Some(metric.ts.as_str()) {
            return;
        }
        self.last_timestamp = Some(metric.ts.clone());
        push(
            &mut self.tps,
            metric.tps_1m.as_ref().map(|value| value.value),
        );
        push(
            &mut self.cpu,
            metric.cpu_percent.as_ref().map(|value| value.value),
        );
        push(
            &mut self.memory_used_mb,
            metric.ram_used_mb.as_ref().map(|value| value.value),
        );
    }

    pub(super) fn draw(
        &self,
        out: &mut Stdout,
        snapshot: &Snapshot,
        x: u16,
        width: u16,
        height: u16,
    ) -> io::Result<()> {
        if snapshot.active_server_id.is_none() {
            return paint(
                out,
                x,
                7,
                width,
                "Select or create a server to view performance.",
                Color::DarkGrey,
            );
        }
        let metric = snapshot.performance.as_ref();
        let tps = metric
            .and_then(|value| value.tps_1m.as_ref())
            .map_or_else(|| "—".into(), |value| format!("{:.1}", value.value));
        let cpu = metric
            .and_then(|value| value.cpu_percent.as_ref())
            .map_or_else(|| "—".into(), |value| format!("{:.0}%", value.value));
        let used = metric
            .and_then(|value| value.ram_used_mb.as_ref())
            .map(|value| value.value);
        let limit = metric
            .and_then(|value| value.ram_max_mb.as_ref())
            .map(|value| value.value);
        let memory = used.map_or_else(
            || "—".into(),
            |used| {
                limit.map_or_else(
                    || format!("{:.1} GB", used / 1024.0),
                    |limit| format!("{:.1} / {:.1} GB", used / 1024.0, limit / 1024.0),
                )
            },
        );
        let players = metric
            .and_then(|value| value.players_online)
            .map_or_else(|| "—".into(), |value| value.to_string());
        for (row, label, value) in [
            (6, "TPS", tps),
            (7, "CPU", cpu),
            (8, "Memory", memory),
            (9, "Players", players),
        ] {
            paint(
                out,
                x,
                row,
                width,
                &format!("{label:<8}{value}"),
                Color::White,
            )?;
        }
        let chart_width = width;
        paint(
            out,
            x,
            11,
            chart_width,
            "TPS HISTORY · 20 is healthy",
            Color::DarkGrey,
        )?;
        paint(
            out,
            x,
            12,
            chart_width,
            &sparkline(&self.tps, chart_width as usize, 20.0),
            Color::Green,
        )?;
        paint(
            out,
            x,
            14,
            chart_width,
            "CPU HISTORY · percent",
            Color::DarkGrey,
        )?;
        paint(
            out,
            x,
            15,
            chart_width,
            &sparkline(&self.cpu, chart_width as usize, 100.0),
            Color::Cyan,
        )?;
        let memory_scale = limit.filter(|limit| *limit > 0.0).or_else(|| {
            self.memory_used_mb
                .iter()
                .flatten()
                .copied()
                .reduce(f64::max)
        });
        paint(
            out,
            x,
            16,
            chart_width,
            if limit.is_some() {
                "MEMORY HISTORY · used / limit"
            } else {
                "MEMORY HISTORY · used (relative scale)"
            },
            Color::DarkGrey,
        )?;
        paint(
            out,
            x,
            17,
            chart_width,
            &sparkline(
                &self.memory_used_mb,
                chart_width as usize,
                memory_scale.unwrap_or(1.0).max(1.0),
            ),
            Color::Magenta,
        )?;
        if height >= 30 {
            paint(
                out,
                x,
                20,
                width,
                "Recent samples are kept only while this TUI is open.",
                Color::DarkGrey,
            )?;
        }
        Ok(())
    }
}

fn push(history: &mut VecDeque<Option<f64>>, value: Option<f64>) {
    if history.len() == MAX_SAMPLES {
        history.pop_front();
    }
    history.push_back(value.filter(|value| value.is_finite()));
}

fn sparkline(history: &VecDeque<Option<f64>>, width: usize, max: f64) -> String {
    let mut line = String::new();
    for sample in history
        .iter()
        .rev()
        .take(width)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        line.push(sample.map_or('·', |value| {
            LEVELS[((value / max).clamp(0.0, 1.0) * 7.0).round() as usize]
        }));
    }
    if line.is_empty() {
        "Waiting for samples…".into()
    } else {
        line
    }
}
