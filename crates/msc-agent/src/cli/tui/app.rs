//! Interaction state for the terminal shell. Agent data remains in the
//! overview view model and is never invented or persisted as local authority.

use std::collections::BTreeMap;
use std::future::Future;

use crossterm::event::KeyCode;
use ratatui::layout::Rect;

use super::layout::{LayoutMode, ShellLayout};
use super::overview::{OverviewState, TAB_NAMES};
use super::transport::SharedClient;
use crate::cli::CommonArgs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    Host,
    Rail,
    Sections,
    Content,
    Console,
}

impl FocusTarget {
    pub fn label(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Rail => "controls",
            Self::Sections => "sections",
            Self::Content => "content",
            Self::Console => "console",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmallSurface {
    Overview,
    Sections,
    Console,
    Help,
}

#[derive(Debug, Clone)]
pub struct App {
    sessions: Vec<HostSession>,
    active_host: usize,
    notes: BTreeMap<String, String>,
    last_action: Option<String>,
    active_tab: usize,
    focus: FocusTarget,
    rail_visible: bool,
    console_visible: bool,
    mode: LayoutMode,
    small_surface: SmallSurface,
}

#[derive(Debug, Clone)]
struct HostSession {
    host: String,
    client: Option<SharedClient>,
    overview: OverviewState,
}

impl App {
    pub fn from_common(common: &CommonArgs) -> Self {
        let host = common
            .base_url
            .clone()
            .unwrap_or_else(|| format!("{}:{}", common.host, common.port));
        let client = client_from_common(common);
        let overview = client.as_ref().map_or_else(
            || OverviewState {
                error: Some(
                    "Authentication is required: pass --token or set MSC2_CLI_TOKEN".into(),
                ),
                ..OverviewState::default()
            },
            |client| {
                OverviewState::load_blocking(client).unwrap_or_else(|error| OverviewState {
                    error: Some(error.to_string()),
                    ..OverviewState::default()
                })
            },
        );
        Self::with_session(host, client, overview)
    }

    pub fn new(host: impl Into<String>) -> Self {
        Self::with_session(host.into(), None, OverviewState::placeholder())
    }

    pub fn with_overview(host: impl Into<String>, overview: OverviewState) -> Self {
        Self::with_session(host.into(), None, overview)
    }

    fn with_session(host: String, client: Option<SharedClient>, overview: OverviewState) -> Self {
        Self {
            sessions: vec![HostSession {
                host,
                client,
                overview,
            }],
            active_host: 0,
            notes: BTreeMap::new(),
            last_action: None,
            active_tab: 0,
            focus: FocusTarget::Content,
            rail_visible: true,
            console_visible: true,
            mode: LayoutMode::Wide,
            small_surface: SmallSurface::Overview,
        }
    }

    pub fn prepare_layout(&mut self, area: Rect) -> ShellLayout {
        let layout = ShellLayout::for_area(area, self.rail_visible, self.console_visible);
        self.mode = layout.mode;
        self.normalize_focus();
        layout
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.mode = super::layout::layout_mode(Rect::new(0, 0, width, height));
        self.normalize_focus();
    }

    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') => return true,
            KeyCode::Tab => self.advance_focus(),
            KeyCode::BackTab => self.reverse_focus(),
            KeyCode::Char('r') if self.mode == LayoutMode::Medium => {
                self.rail_visible = !self.rail_visible;
                self.normalize_focus();
            }
            KeyCode::Char('c') if self.mode == LayoutMode::Medium => {
                self.console_visible = !self.console_visible;
                self.normalize_focus();
            }
            KeyCode::Char('c') if self.mode == LayoutMode::Small => {
                self.small_surface = SmallSurface::Console;
                self.focus = FocusTarget::Console;
            }
            KeyCode::Char('?') => self.small_surface = SmallSurface::Help,
            KeyCode::Char('a') => self.request_lifecycle(true),
            KeyCode::Char('x') => self.request_lifecycle(false),
            KeyCode::Char('1'..='7') => {
                let index = key_to_tab(key);
                if self.overview().tab_is_available(index) {
                    self.active_tab = index;
                }
            }
            KeyCode::Char('s') if self.mode == LayoutMode::Small => {
                self.small_surface = SmallSurface::Sections;
                self.focus = FocusTarget::Sections;
            }
            KeyCode::Char('s') if self.mode == LayoutMode::Medium => {
                self.focus = FocusTarget::Sections;
            }
            KeyCode::Esc if self.mode == LayoutMode::Small => {
                self.small_surface = SmallSurface::Overview;
                self.focus = FocusTarget::Content;
            }
            KeyCode::Left | KeyCode::Up | KeyCode::Char('k') if self.focus == FocusTarget::Host => {
                self.previous_server()
            }
            KeyCode::Right | KeyCode::Down | KeyCode::Char('j')
                if self.focus == FocusTarget::Host =>
            {
                self.next_server()
            }
            KeyCode::Enter if self.focus == FocusTarget::Host => self.activate_selected_server(),
            KeyCode::Left | KeyCode::Up | KeyCode::Char('k') => self.previous_tab(),
            KeyCode::Right | KeyCode::Down | KeyCode::Char('j') => self.next_tab(),
            KeyCode::Enter if self.small_surface == SmallSurface::Sections => {
                self.small_surface = SmallSurface::Overview;
                self.focus = FocusTarget::Content;
            }
            _ => {}
        }
        false
    }

    pub fn host(&self) -> &str {
        &self.current_session().host
    }

    pub fn active_tab(&self) -> usize {
        self.active_tab
    }

    pub fn active_tab_name(&self) -> &'static str {
        TAB_NAMES[self.active_tab]
    }

    pub fn tabs() -> &'static [&'static str] {
        &TAB_NAMES
    }

    pub fn focus(&self) -> FocusTarget {
        self.focus
    }

    pub fn rail_visible(&self) -> bool {
        self.rail_visible
    }

    pub fn console_visible(&self) -> bool {
        self.console_visible
    }

    pub fn small_surface(&self) -> SmallSurface {
        self.small_surface
    }

    pub fn overview(&self) -> &OverviewState {
        &self.current_session().overview
    }

    pub fn available_tabs(&self) -> Vec<usize> {
        self.overview().available_tabs()
    }

    pub fn notes_for_selected_server(&self) -> Option<&str> {
        let key = self.note_key()?;
        self.notes.get(&key).map(String::as_str)
    }

    pub fn set_note(&mut self, note: impl Into<String>) {
        if let Some(key) = self.note_key() {
            self.notes.insert(key, note.into());
        }
    }

    pub fn last_action(&self) -> Option<&str> {
        self.last_action.as_deref()
    }

    /// Add a host session without writing a profile or token to disk. This is
    /// the only host-switching state the first TUI slice owns.
    pub fn add_host_session(&mut self, common: &CommonArgs) -> Result<(), String> {
        let Some(client) = client_from_common(common) else {
            return Err("host session cannot be created without a bearer token".to_string());
        };
        let host = common
            .base_url
            .clone()
            .unwrap_or_else(|| format!("{}:{}", common.host, common.port));
        let overview = OverviewState::load_blocking(&client).map_err(|error| error.to_string())?;
        self.sessions.push(HostSession {
            host,
            client: Some(client),
            overview,
        });
        Ok(())
    }

    pub fn switch_host(&mut self, index: usize) -> bool {
        if index >= self.sessions.len() {
            return false;
        }
        self.active_host = index;
        self.active_tab = 0;
        self.normalize_focus();
        true
    }

    fn next_tab(&mut self) {
        let tabs = self.available_tabs();
        let current = tabs
            .iter()
            .position(|tab| *tab == self.active_tab)
            .unwrap_or(0);
        self.active_tab = tabs[(current + 1) % tabs.len()];
    }

    fn previous_tab(&mut self) {
        let tabs = self.available_tabs();
        let current = tabs
            .iter()
            .position(|tab| *tab == self.active_tab)
            .unwrap_or(0);
        self.active_tab = tabs[(current + tabs.len() - 1) % tabs.len()];
    }

    fn focus_order(&self) -> Vec<FocusTarget> {
        match self.mode {
            LayoutMode::Wide => vec![
                FocusTarget::Host,
                FocusTarget::Rail,
                FocusTarget::Sections,
                FocusTarget::Content,
                FocusTarget::Console,
            ],
            LayoutMode::Medium => {
                let mut order = vec![FocusTarget::Host];
                if self.rail_visible {
                    order.push(FocusTarget::Rail);
                }
                order.extend([FocusTarget::Sections, FocusTarget::Content]);
                if self.console_visible {
                    order.push(FocusTarget::Console);
                }
                order
            }
            LayoutMode::Small => vec![
                FocusTarget::Host,
                FocusTarget::Sections,
                FocusTarget::Content,
                FocusTarget::Console,
            ],
        }
    }

    fn advance_focus(&mut self) {
        let order = self.focus_order();
        let current = order
            .iter()
            .position(|item| *item == self.focus)
            .unwrap_or(0);
        self.focus = order[(current + 1) % order.len()];
    }

    fn reverse_focus(&mut self) {
        let order = self.focus_order();
        let current = order
            .iter()
            .position(|item| *item == self.focus)
            .unwrap_or(0);
        self.focus = order[(current + order.len() - 1) % order.len()];
    }

    fn normalize_focus(&mut self) {
        if !self.focus_order().contains(&self.focus) {
            self.focus = FocusTarget::Content;
        }
        if !self.overview().tab_is_available(self.active_tab) {
            self.active_tab = self.available_tabs().into_iter().next().unwrap_or(0);
        }
    }

    fn current_session(&self) -> &HostSession {
        &self.sessions[self.active_host]
    }

    fn current_session_mut(&mut self) -> &mut HostSession {
        &mut self.sessions[self.active_host]
    }

    fn note_key(&self) -> Option<String> {
        Some(format!(
            "{}::{}",
            self.host(),
            self.overview().selected_server_id.as_deref()?
        ))
    }

    fn next_server(&mut self) {
        self.select_server_offset(1);
    }

    fn previous_server(&mut self) {
        self.select_server_offset(-1);
    }

    fn select_server_offset(&mut self, offset: isize) {
        let servers = &self.overview().servers;
        if servers.is_empty() {
            return;
        }
        let current = self
            .overview()
            .selected_server_id
            .as_ref()
            .and_then(|id| servers.iter().position(|server| &server.id == id))
            .unwrap_or(0);
        let next = (current as isize + offset).rem_euclid(servers.len() as isize) as usize;
        let selector = servers[next].id.clone();
        self.run_overview_action(&selector);
    }

    fn activate_selected_server(&mut self) {
        let Some(id) = self.overview().selected_server_id.clone() else {
            return;
        };
        self.run_overview_action(&id);
    }

    fn run_overview_action(&mut self, selector: &str) {
        let Some(client) = self.current_session().client.clone() else {
            self.last_action = Some("No authenticated host session".to_string());
            return;
        };
        match run_blocking(OverviewState::select_server(&client, selector)) {
            Ok(overview) => {
                self.current_session_mut().overview = overview;
                self.last_action = Some(format!("Selected server {selector}"));
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn request_lifecycle(&mut self, start: bool) {
        let Some(client) = self.current_session().client.clone() else {
            self.last_action = Some("No authenticated host session".to_string());
            return;
        };
        match run_blocking(OverviewState::request_lifecycle(&client, start)) {
            Ok(overview) => {
                self.current_session_mut().overview = overview;
                self.last_action = Some(if start {
                    "Start requested".to_string()
                } else {
                    "Stop requested".to_string()
                });
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }
}

fn key_to_tab(key: KeyCode) -> usize {
    match key {
        KeyCode::Char(value) => value.to_digit(10).unwrap_or(1).saturating_sub(1) as usize,
        _ => 0,
    }
}

fn run_blocking<T>(future: impl Future<Output = T>) -> T {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(future))
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("TUI temporary runtime builds")
            .block_on(future)
    }
}

impl OverviewState {
    fn load_blocking(client: &SharedClient) -> Result<Self, crate::cli::CliError> {
        run_blocking(Self::load(client))
    }
}

#[cfg(not(test))]
fn client_from_common(common: &CommonArgs) -> Option<SharedClient> {
    SharedClient::from_common(common).ok()
}

#[cfg(test)]
fn client_from_common(_common: &CommonArgs) -> Option<SharedClient> {
    // Integration tests include the CLI module with a small transport shim;
    // production construction is exercised by the binary itself.
    None
}
