//! The terminal console dock and its local presentation state.
//!
//! The agent remains the source of truth for console output and command
//! policy. This module only keeps a bounded client-side view, reconnects the
//! existing console stream, and sends literal input to the existing command
//! route.

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;

use msc_api::dto::CommandResultDto;
use msc_infrastructure::console_buffer::ConsoleLine;

use super::transport::{SharedClient, StreamChannel};
use crate::cli::CliError;

pub const LOCAL_SCROLLBACK_LIMIT: usize = 1_000;

const PALETTE: [(&str, &str); 7] = [
    ("Time", "time set day"),
    ("Weather", "weather clear"),
    ("Difficulty", "difficulty normal"),
    ("Gamemode", "gamemode survival"),
    ("Whitelist", "whitelist on"),
    ("Save all", "save-all"),
    ("Reload", "reload"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsoleFilter {
    All,
    Server,
    Plugins,
    Warnings,
    Controller,
    Commands,
    Custom(String),
}

impl ConsoleFilter {
    pub fn label(&self) -> &str {
        match self {
            Self::All => "All",
            Self::Server => "Server",
            Self::Plugins => "Plugins",
            Self::Warnings => "Warnings",
            Self::Controller => "Controller",
            Self::Commands => "Commands",
            Self::Custom(_) => "Custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Normal,
    Command,
    Search,
    Palette,
}

#[derive(Debug)]
struct ConsoleFeed {
    receiver: Mutex<Receiver<FeedEvent>>,
}

#[derive(Debug)]
enum FeedEvent {
    Line(ConsoleLine),
    Recovery(Vec<ConsoleLine>),
    Status(String),
}

#[derive(Debug, Clone)]
pub struct ConsoleView {
    lines: VecDeque<ConsoleLine>,
    filter: ConsoleFilter,
    search: String,
    command: String,
    command_history: VecDeque<String>,
    history_index: Option<usize>,
    input_mode: InputMode,
    palette_index: usize,
    selection_anchor: Option<usize>,
    selected_index: usize,
    follow: bool,
    paused: bool,
    collapsed: bool,
    status: Option<String>,
    feed: Option<Arc<ConsoleFeed>>,
}

impl ConsoleView {
    pub fn from_lines(lines: impl IntoIterator<Item = ConsoleLine>) -> Self {
        let mut view = Self {
            lines: VecDeque::new(),
            filter: ConsoleFilter::All,
            search: String::new(),
            command: String::new(),
            command_history: VecDeque::new(),
            history_index: None,
            input_mode: InputMode::Normal,
            palette_index: 0,
            selection_anchor: None,
            selected_index: 0,
            follow: true,
            paused: false,
            collapsed: false,
            status: None,
            feed: None,
        };
        for line in lines {
            view.append_line(line);
        }
        view
    }

    pub fn visible_lines(&self) -> Vec<&ConsoleLine> {
        self.lines
            .iter()
            .filter(|line| self.matches(line))
            .collect()
    }

    pub fn filter(&self) -> &ConsoleFilter {
        &self.filter
    }

    pub fn search(&self) -> &str {
        &self.search
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn is_editing(&self) -> bool {
        !matches!(self.input_mode, InputMode::Normal)
    }

    pub fn palette_open(&self) -> bool {
        self.input_mode == InputMode::Palette
    }

    pub fn palette_index(&self) -> usize {
        self.palette_index
    }

    pub fn palette_entries() -> &'static [(&'static str, &'static str)] {
        &PALETTE
    }

    pub fn follow(&self) -> bool {
        self.follow
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn collapsed(&self) -> bool {
        self.collapsed
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn start_feed(&mut self, client: SharedClient) {
        if self.feed.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let feed = Arc::new(ConsoleFeed {
            receiver: Mutex::new(receiver),
        });
        self.feed = Some(feed);
        thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("console feed runtime builds");
            runtime.block_on(run_feed(client, sender));
        });
    }

    pub fn poll(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        loop {
            let event = feed
                .receiver
                .lock()
                .expect("console feed lock poisoned")
                .try_recv();
            match event {
                Ok(FeedEvent::Line(line)) => self.append_line(line),
                Ok(FeedEvent::Recovery(lines)) => self.replace_lines(lines),
                Ok(FeedEvent::Status(status)) => self.status = Some(status),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
    }

    pub fn set_filter(&mut self, filter: ConsoleFilter) {
        self.filter = filter;
        self.selected_index = 0;
        self.selection_anchor = None;
    }

    pub fn select_filter_key(&mut self, key: char) -> bool {
        let filter = match key {
            '0' => ConsoleFilter::All,
            '1' => ConsoleFilter::Server,
            '2' => ConsoleFilter::Plugins,
            '3' => ConsoleFilter::Warnings,
            '4' => ConsoleFilter::Controller,
            '5' => ConsoleFilter::Commands,
            '6' if !self.search.is_empty() => ConsoleFilter::Custom(self.search.clone()),
            _ => return false,
        };
        self.set_filter(filter);
        true
    }

    pub fn set_search(&mut self, search: impl Into<String>) {
        self.search = search.into();
        self.selected_index = 0;
        self.selection_anchor = None;
    }

    pub fn toggle_follow(&mut self) {
        self.follow = !self.follow;
        if self.follow {
            self.selected_index = self.visible_lines().len().saturating_sub(1);
        }
    }

    pub fn toggle_paused(&mut self) {
        self.paused = !self.paused;
    }

    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
    }

    pub fn clear_local_history(&mut self) {
        self.lines.clear();
        self.selected_index = 0;
        self.selection_anchor = None;
        self.status = Some("Local console history cleared".to_string());
    }

    pub fn move_selection(&mut self, offset: isize) {
        let length = self.visible_lines().len();
        if length == 0 {
            self.selected_index = 0;
            return;
        }
        self.selected_index = (self.selected_index as isize + offset)
            .clamp(0, length.saturating_sub(1) as isize) as usize;
        self.follow = false;
    }

    pub fn toggle_selection_anchor(&mut self) {
        self.selection_anchor = if self.selection_anchor.is_some() {
            None
        } else {
            Some(self.selected_index)
        };
    }

    pub fn selected_text(&self) -> String {
        let visible = self.visible_lines();
        if visible.is_empty() {
            return String::new();
        }
        let start = self
            .selection_anchor
            .unwrap_or(self.selected_index)
            .min(visible.len() - 1);
        let end = self.selected_index.min(visible.len() - 1);
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        visible[start..=end]
            .iter()
            .map(|line| format!("[{}] {} {}", line.ts, line.source, line.text))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn begin_command(&mut self) {
        self.input_mode = InputMode::Command;
        self.history_index = None;
    }

    pub fn begin_search(&mut self) {
        self.input_mode = InputMode::Search;
    }

    pub fn begin_palette(&mut self) {
        self.input_mode = InputMode::Palette;
        self.palette_index = 0;
    }

    pub fn cancel_input(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn push_input(&mut self, character: char) {
        match self.input_mode {
            InputMode::Command => self.command.push(character),
            InputMode::Search => self.search.push(character),
            InputMode::Normal | InputMode::Palette => {}
        }
    }

    pub fn pop_input(&mut self) {
        match self.input_mode {
            InputMode::Command => {
                self.command.pop();
            }
            InputMode::Search => {
                self.search.pop();
            }
            InputMode::Normal | InputMode::Palette => {}
        }
    }

    pub fn history_previous(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        let next = self
            .history_index
            .map_or(0, |index| index.saturating_sub(1));
        self.history_index = Some(next);
        self.command = self.command_history[next].clone();
    }

    pub fn history_next(&mut self) {
        let Some(index) = self.history_index else {
            return;
        };
        if index + 1 >= self.command_history.len() {
            self.history_index = None;
            self.command.clear();
        } else {
            self.history_index = Some(index + 1);
            self.command = self.command_history[index + 1].clone();
        }
    }

    pub fn move_palette(&mut self, offset: isize) {
        self.palette_index =
            (self.palette_index as isize + offset).rem_euclid(PALETTE.len() as isize) as usize;
    }

    pub fn selected_palette_command(&self) -> &'static str {
        PALETTE[self.palette_index].1
    }

    pub fn take_command(&mut self) -> Option<String> {
        let command = self.command.trim().to_string();
        self.command.clear();
        self.input_mode = InputMode::Normal;
        self.history_index = None;
        if command.is_empty() {
            None
        } else {
            self.command_history.push_back(command.clone());
            while self.command_history.len() > 100 {
                self.command_history.pop_front();
            }
            Some(command)
        }
    }

    pub async fn send_command(
        client: &SharedClient,
        command: &str,
    ) -> Result<CommandResultDto, CliError> {
        client
            .post_json("/v1/command", &serde_json::json!({ "command": command }))
            .await
    }

    fn append_line(&mut self, line: ConsoleLine) {
        if self.lines.iter().any(|existing| existing == &line) {
            return;
        }
        self.lines.push_back(line);
        while self.lines.len() > LOCAL_SCROLLBACK_LIMIT {
            self.lines.pop_front();
        }
        if self.follow {
            self.selected_index = self.visible_lines().len().saturating_sub(1);
        }
    }

    fn replace_lines(&mut self, lines: Vec<ConsoleLine>) {
        self.lines.clear();
        for line in lines {
            self.append_line(line);
        }
        self.status = Some("Console stream recovered from the agent tail".to_string());
    }

    fn matches(&self, line: &ConsoleLine) -> bool {
        let source = line.source.to_ascii_lowercase();
        let text = line.text.to_ascii_lowercase();
        let filter_matches = match &self.filter {
            ConsoleFilter::All => true,
            ConsoleFilter::Server => matches!(source.as_str(), "server" | "bedrock"),
            ConsoleFilter::Plugins => {
                source.contains("plugin") || source.contains("playit") || source.contains("xbox")
            }
            ConsoleFilter::Warnings => {
                line.level.as_deref().is_some_and(|level| {
                    matches!(
                        level.to_ascii_lowercase().as_str(),
                        "warn" | "warning" | "error"
                    )
                }) || text.contains("warn")
                    || text.contains("error")
                    || text.contains("failed")
            }
            ConsoleFilter::Controller => {
                matches!(source.as_str(), "controller" | "system" | "msc")
            }
            ConsoleFilter::Commands => source.contains("command") || text.starts_with("> "),
            ConsoleFilter::Custom(value) => {
                let value = value.to_ascii_lowercase();
                source.contains(&value) || text.contains(&value)
            }
        };
        filter_matches
            && (self.search.is_empty()
                || source.contains(&self.search.to_ascii_lowercase())
                || text.contains(&self.search.to_ascii_lowercase()))
    }
}

async fn run_feed(client: SharedClient, sender: mpsc::Sender<FeedEvent>) {
    loop {
        let mut stream = match client.open_stream(StreamChannel::Console).await {
            Ok(stream) => stream,
            Err(error) => {
                let _ = sender.send(FeedEvent::Status(format!(
                    "Console stream reconnecting: {error}"
                )));
                tokio::time::sleep(super::transport::reconnect_delay(1)).await;
                continue;
            }
        };
        let _ = sender.send(FeedEvent::Status("Console stream connected".to_string()));
        loop {
            match stream.next_json::<ConsoleLine>().await {
                Ok(Some(line)) => {
                    if sender.send(FeedEvent::Line(line)).is_err() {
                        return;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    if let Ok(lines) = client
                        .get_json::<Vec<ConsoleLine>>("/v1/console/tail?n=200")
                        .await
                    {
                        let _ = sender.send(FeedEvent::Recovery(lines));
                    }
                    let _ = sender.send(FeedEvent::Status(format!(
                        "Console stream interrupted; retrying: {error}"
                    )));
                    break;
                }
            }
        }
    }
}
