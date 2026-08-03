//! Process and snapshot metrics helpers for the Java lifecycle slice.

use crate::process::ProcessId;
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::Path;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessResourceUsage {
    pub cpu_percent: Option<f64>,
    pub ram_used_mb: Option<f64>,
}

pub trait ProcessMetricsProvider {
    fn process_usage(&self, pid: ProcessId) -> Option<ProcessResourceUsage>;
}

#[derive(Debug, Clone, Copy)]
pub struct PsProcessMetricsProvider {
    #[cfg_attr(not(any(target_os = "macos", target_os = "linux")), allow(dead_code))]
    logical_core_count: usize,
}

impl Default for PsProcessMetricsProvider {
    fn default() -> Self {
        Self {
            logical_core_count: std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1),
        }
    }
}

impl PsProcessMetricsProvider {
    pub fn new(logical_core_count: usize) -> Self {
        Self { logical_core_count }
    }
}

impl ProcessMetricsProvider for PsProcessMetricsProvider {
    fn process_usage(&self, pid: ProcessId) -> Option<ProcessResourceUsage> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let output = Command::new("/bin/ps")
                .args(["-p", &pid.raw().to_string(), "-o", "%cpu,rss"])
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            let stdout = String::from_utf8(output.stdout).ok()?;
            parse_ps_cpu_rss(&stdout, self.logical_core_count)
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = pid;
            None
        }
    }
}

pub fn parse_ps_cpu_rss(output: &str, logical_core_count: usize) -> Option<ProcessResourceUsage> {
    let data_line = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .find(|line| !line.starts_with('%'))?;
    let mut parts = data_line.split_whitespace();
    let raw_cpu = parts.next()?.parse::<f64>().ok();
    let rss_kb = parts.next()?.parse::<f64>().ok();

    let cpu_percent = raw_cpu.map(|cpu| {
        if logical_core_count > 0 {
            (cpu / logical_core_count as f64).clamp(0.0, 100.0)
        } else {
            cpu
        }
    });
    let ram_used_mb = rss_kb.map(|kb| kb / 1024.0);

    Some(ProcessResourceUsage {
        cpu_percent,
        ram_used_mb,
    })
}

#[derive(Debug, Clone)]
pub struct BoundedMetricHistory<T> {
    capacity: usize,
    samples: VecDeque<T>,
}

impl<T> BoundedMetricHistory<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            samples: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, sample: T) {
        if self.capacity == 0 {
            return;
        }
        while self.samples.len() >= self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn samples(&self) -> impl Iterator<Item = &T> {
        self.samples.iter()
    }
}

pub fn directory_size_mb(path: &Path) -> io::Result<f64> {
    Ok(directory_size_bytes(path)? as f64 / 1_048_576.0)
}

fn directory_size_bytes(path: &Path) -> io::Result<u64> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Ok(0);
    }

    let mut total = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        total += directory_size_bytes(&entry.path())?;
    }
    Ok(total)
}
