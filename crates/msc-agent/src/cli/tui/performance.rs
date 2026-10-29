//! Live server metrics, with a small history owned by this TUI session.

use std::collections::VecDeque;
use std::io::{self, Stdout};

use crossterm::style::Color;

use super::{Snapshot, box_outline, paint};

const MAX_SAMPLES: usize = 120;
const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

#[derive(Default)]
pub(super) struct PerformanceUi {
    server_id: Option<String>,
    last_timestamp: Option<String>,
    tps: VecDeque<Option<f64>>,
    cpu: VecDeque<Option<f64>>,
    memory_used_mb: VecDeque<Option<f64>>,
    was_running: bool,
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
        if !snapshot.active_server_running {
            if self.was_running {
                self.tps.clear();
                self.cpu.clear();
                self.memory_used_mb.clear();
            }
            self.was_running = false;
            return;
        }
        self.was_running = true;
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
        let tps_title = format!("TPS HISTORY · {tps}");
        let cpu_title = format!("CPU HISTORY · {cpu}");
        let memory_title = format!("MEMORY HISTORY · {memory}");
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
        if !snapshot.active_server_running {
            paint(
                out,
                x,
                12,
                width,
                "No live data · server stopped. Graphs resume when it starts.",
                Color::DarkGrey,
            )?;
            return Ok(());
        }
        let memory_scale = limit.filter(|limit| *limit > 0.0).or_else(|| {
            self.memory_used_mb
                .iter()
                .flatten()
                .copied()
                .reduce(f64::max)
        });
        let top = 11;
        let bottom = height.saturating_sub(7);
        let available = bottom.saturating_sub(top);
        let gap = if available >= 10 { 1 } else { 0 };
        let panel_space = available.saturating_sub(2 * gap);
        let base = panel_space / 3;
        let extra = panel_space % 3;
        let memory_scale_label =
            memory_scale.map_or_else(|| "—".into(), |value| format!("{:.1}G", value / 1024.0));
        let charts = [
            (tps_title.as_str(), &self.tps, 20.0, Color::Green, "20"),
            (cpu_title.as_str(), &self.cpu, 100.0, Color::Cyan, "100"),
            (
                memory_title.as_str(),
                &self.memory_used_mb,
                memory_scale.unwrap_or(1.0).max(1.0),
                Color::Magenta,
                memory_scale_label.as_str(),
            ),
        ];
        let mut y = top;
        for (index, (title, samples, scale, color, scale_label)) in charts.into_iter().enumerate() {
            let panel_height = base + u16::from(index < extra as usize);
            draw_chart(
                out,
                x,
                y,
                width,
                panel_height,
                title,
                samples,
                scale,
                scale_label,
                color,
            )?;
            y += panel_height + gap;
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

#[allow(clippy::too_many_arguments)]
fn draw_chart(
    out: &mut Stdout,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    title: &str,
    history: &VecDeque<Option<f64>>,
    scale: f64,
    scale_label: &str,
    color: Color,
) -> io::Result<()> {
    if height < 5 {
        paint(out, x, y, width, title, Color::DarkGrey)?;
        if height > 1 {
            paint(
                out,
                x,
                y + 1,
                width,
                &sparkline(history, width as usize, scale),
                color,
            )?;
        }
        return Ok(());
    }
    box_outline(out, x, y, width, height)?;
    paint(out, x + 2, y + 1, width - 4, title, Color::DarkGrey)?;
    let plot_top = y + 2;
    let plot_height = height - 3;
    let plot_width = width.saturating_sub(10) as usize;
    paint(out, x + 2, plot_top, 5, scale_label, Color::DarkGrey)?;
    paint(
        out,
        x + 2,
        plot_top + plot_height - 1,
        5,
        "0",
        Color::DarkGrey,
    )?;
    if history.iter().all(Option::is_none) {
        paint(
            out,
            x + 8,
            plot_top + plot_height / 2,
            width - 10,
            "No samples available yet",
            Color::DarkGrey,
        )?;
        return Ok(());
    }
    let mut points = vec![vec![' '; plot_width]; plot_height as usize];
    for (column, sample) in history.iter().rev().take(plot_width).enumerate() {
        if let Some(value) = sample {
            let level = (value / scale).clamp(0.0, 1.0);
            let row = ((1.0 - level) * f64::from(plot_height - 1)).round() as usize;
            points[row][plot_width - 1 - column] = '●';
        }
    }
    for (row, chars) in points.into_iter().enumerate() {
        paint(
            out,
            x + 8,
            plot_top + row as u16,
            plot_width as u16,
            &chars.into_iter().collect::<String>(),
            color,
        )?;
    }
    Ok(())
}

fn sparkline(history: &VecDeque<Option<f64>>, width: usize, max: f64) -> String {
    if history.iter().all(Option::is_none) {
        return "No samples available yet".into();
    }
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
