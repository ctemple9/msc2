//! Java lifecycle console-line reducer for the Phase 4 vertical slice.
//!
//! MSC 1 hangs several side effects off each sanitized console line. This
//! reducer ports only the Java lifecycle subset needed before status snapshots:
//! ready detection, Java join/leave events, TPS samples, and the distinction
//! between "never reached ready" startup failures and later unexpected stops.

use msc_domain::tps;

#[derive(Debug, Clone, PartialEq)]
pub enum OutputEvent {
    Ready,
    PlayerJoined(String),
    PlayerLeft(String),
    TpsSample(tps::Sample),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnexpectedStopKind {
    StartupFailedBeforeReady,
    StoppedAfterReady,
}

#[derive(Debug, Default)]
pub struct JavaOutputReducer {
    reached_ready: bool,
    online_players: Vec<String>,
    expecting_auto_spark_block: bool,
    in_spark_block: bool,
    expect_spark_tps_values: bool,
    spark_cpu_values_remaining: usize,
    spark_block_guard: usize,
}

impl JavaOutputReducer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reached_ready(&self) -> bool {
        self.reached_ready
    }

    pub fn online_players(&self) -> &[String] {
        &self.online_players
    }

    /// Arms the reducer for the multi-line reply produced by `spark tps`.
    /// Spark's values are only accepted after this explicit request so an
    /// unrelated log line cannot become a false TPS sample.
    pub fn expect_spark_tps_reply(&mut self) {
        self.end_spark_block();
        self.expecting_auto_spark_block = true;
    }

    pub fn process_line(&mut self, clean: &str) -> Vec<OutputEvent> {
        let mut events = Vec::new();

        if !self.reached_ready && is_paper_ready_line(clean) {
            self.reached_ready = true;
            events.push(OutputEvent::Ready);
        }

        if let Some(sample) = tps::parse(clean) {
            events.push(OutputEvent::TpsSample(sample));
        }

        if let Some(sample) = self.consume_spark_tps_line(clean) {
            events.push(OutputEvent::TpsSample(sample));
        }

        if let Some(name) = parse_java_player_name(clean, " joined the game") {
            self.upsert_online_player(&name);
            events.push(OutputEvent::PlayerJoined(name));
        } else if let Some(name) = parse_java_player_name(clean, " left the game") {
            self.remove_online_player(&name);
            events.push(OutputEvent::PlayerLeft(name));
        }

        events
    }

    pub fn classify_unexpected_stop(&self) -> UnexpectedStopKind {
        if self.reached_ready {
            UnexpectedStopKind::StoppedAfterReady
        } else {
            UnexpectedStopKind::StartupFailedBeforeReady
        }
    }

    fn upsert_online_player(&mut self, name: &str) {
        if let Some(existing) = self
            .online_players
            .iter_mut()
            .find(|existing| existing.eq_ignore_ascii_case(name))
        {
            *existing = name.to_string();
        } else {
            self.online_players.push(name.to_string());
        }
    }

    fn remove_online_player(&mut self, name: &str) {
        self.online_players
            .retain(|existing| !existing.eq_ignore_ascii_case(name));
    }

    fn consume_spark_tps_line(&mut self, clean: &str) -> Option<tps::Sample> {
        if tps::is_spark_tps_header(clean) {
            if self.expecting_auto_spark_block && !self.in_spark_block {
                self.expecting_auto_spark_block = false;
                self.in_spark_block = true;
                self.expect_spark_tps_values = true;
                self.spark_cpu_values_remaining = 0;
                // Bound the state machine so a truncated spark reply cannot
                // consume unrelated console output forever.
                self.spark_block_guard = 12;
            }
            return None;
        }

        if !self.in_spark_block {
            return None;
        }

        self.spark_block_guard = self.spark_block_guard.saturating_sub(1);
        let mut end_block = false;
        let sample = if self.expect_spark_tps_values {
            let sample = tps::parse_spark_values(clean);
            if sample.is_some() {
                self.expect_spark_tps_values = false;
            }
            sample
        } else if clean.contains("CPU usage from last 10s, 1m, 15m") {
            self.spark_cpu_values_remaining = 2;
            None
        } else if self.spark_cpu_values_remaining > 0 {
            self.spark_cpu_values_remaining -= 1;
            if self.spark_cpu_values_remaining == 0 {
                end_block = true;
            }
            None
        } else {
            None
        };

        if self.spark_block_guard == 0 || end_block {
            self.end_spark_block();
        }
        sample
    }

    fn end_spark_block(&mut self) {
        self.expecting_auto_spark_block = false;
        self.in_spark_block = false;
        self.expect_spark_tps_values = false;
        self.spark_cpu_values_remaining = 0;
        self.spark_block_guard = 0;
    }
}

pub fn is_paper_ready_line(clean: &str) -> bool {
    clean.contains("Done (")
}

pub fn parse_java_player_name(clean: &str, marker: &str) -> Option<String> {
    let before_marker = clean.split_once(marker)?.0;
    let name = before_marker.split_whitespace().last()?.trim();
    (!name.is_empty()).then(|| name.to_string())
}
