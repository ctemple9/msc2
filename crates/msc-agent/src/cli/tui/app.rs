//! Interaction state for the terminal shell. Agent data remains in the
//! overview view model and is never invented or persisted as local authority.

use std::collections::BTreeMap;
use std::future::Future;

use crossterm::event::KeyCode;
use ratatui::layout::Rect;

use super::activity::ActivityState;
use super::confirm::{ConfirmAction, ConfirmationRequest, ConfirmationResult, ConfirmationState};
use super::console::ConsoleView;
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
    activity: ActivityState,
    confirmation: ConfirmationState,
}

#[derive(Debug, Clone)]
struct HostSession {
    host: String,
    client: Option<SharedClient>,
    overview: OverviewState,
    console: ConsoleView,
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
        let mut app = Self::with_session(host, client.clone(), overview);
        if let Some(client) = client {
            app.current_session_mut().console.start_feed(client.clone());
            app.activity.start_notifications(client);
        }
        app
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
                console: ConsoleView::from_lines(overview.console.clone()),
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
            activity: ActivityState::default(),
            confirmation: ConfirmationState::default(),
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
        if self.confirmation.is_open() {
            if let Some(result) = self.confirmation.handle_key(key) {
                let request = self.confirmation.resolve(result);
                if result == ConfirmationResult::Confirmed {
                    if let Some(request) = request {
                        self.execute_confirmed_action(request.action);
                    }
                } else {
                    self.last_action = Some("Request cancelled before dispatch".to_string());
                }
            }
            return false;
        }
        if self.console().is_editing() {
            self.handle_console_input(key);
            return false;
        }
        if self.focus == FocusTarget::Console && self.handle_console_key(key) {
            return false;
        }
        if self.activity.is_open() {
            match key {
                KeyCode::Esc | KeyCode::Char('i') => self.activity.close(),
                KeyCode::Up | KeyCode::Char('k') => self.activity.move_selection(-1),
                KeyCode::Down | KeyCode::Char('j') => self.activity.move_selection(1),
                KeyCode::Char('x') => self.begin_cancel_confirmation(),
                _ => {}
            }
            return false;
        }
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
            KeyCode::Char('i') => {
                self.activity.open();
                self.focus = FocusTarget::Content;
            }
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

    pub fn console(&self) -> &ConsoleView {
        &self.current_session().console
    }

    pub fn poll_console(&mut self) {
        self.current_session_mut().console.poll();
    }

    pub fn poll_activity(&mut self) {
        self.activity.poll();
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

    pub fn activity(&self) -> &ActivityState {
        &self.activity
    }

    pub fn confirmation(&self) -> &ConfirmationState {
        &self.confirmation
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
        let mut console = ConsoleView::from_lines(overview.console.clone());
        console.start_feed(client.clone());
        self.sessions.push(HostSession {
            host,
            client: Some(client),
            console,
            overview,
        });
        self.activity.start_notifications(
            self.sessions
                .last()
                .and_then(|session| session.client.clone())
                .expect("new host session has its client"),
        );
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
        let Some(server) = self.overview().selected_server() else {
            self.last_action = Some("No selected server to change".to_string());
            return;
        };
        self.confirmation.begin(ConfirmationRequest {
            host: self.host().to_string(),
            server: server.name.clone(),
            target: server.id.clone(),
            consequence: if start {
                "The agent will launch this server and begin managing its process.".to_string()
            } else {
                "The agent will ask the server to stop; players may be disconnected.".to_string()
            },
            action: if start {
                ConfirmAction::StartServer
            } else {
                ConfirmAction::StopServer
            },
        });
    }

    fn execute_confirmed_action(&mut self, action: ConfirmAction) {
        let Some(client) = self.current_session().client.clone() else {
            self.last_action = Some("No authenticated host session".to_string());
            return;
        };
        match action {
            ConfirmAction::StartServer | ConfirmAction::StopServer => {
                let start = matches!(action, ConfirmAction::StartServer);
                match run_blocking(ActivityState::request_lifecycle(&client, start)) {
                    Ok((result, overview)) => {
                        self.current_session_mut().overview = overview;
                        if let Some(operation_id) = result.operation_id {
                            self.activity.track_operation(client, operation_id.clone());
                            self.last_action = Some(format!(
                                "{} requested; operation {operation_id} is being tracked",
                                if start { "Start" } else { "Stop" }
                            ));
                        } else {
                            self.last_action = Some(if start {
                                "Start requested".to_string()
                            } else {
                                "Stop requested".to_string()
                            });
                        }
                    }
                    Err(error) => self.last_action = Some(error.to_string()),
                }
            }
            ConfirmAction::CancelOperation { operation_id } => {
                match run_blocking(ActivityState::cancel_operation(&client, &operation_id)) {
                    Ok(_) => {
                        self.last_action = Some(format!(
                            "Cancellation requested for operation {operation_id}"
                        ));
                    }
                    Err(error) => self.last_action = Some(error.to_string()),
                }
            }
        }
    }

    fn begin_cancel_confirmation(&mut self) {
        if !self.activity.selected_operation_is_active() {
            self.last_action = Some("Select a queued or running operation to cancel".to_string());
            return;
        }
        let Some(operation_id) = self.activity.selected_operation_id() else {
            self.last_action = Some("Select a running operation to cancel".to_string());
            return;
        };
        let operation_id = operation_id.to_string();
        let target = self
            .activity
            .operations()
            .find(|operation| operation.id == operation_id)
            .and_then(|operation| operation.target.clone())
            .unwrap_or_else(|| operation_id.clone());
        self.confirmation.begin(ConfirmationRequest {
            host: self.host().to_string(),
            server: self.overview().selected_server_name().to_string(),
            target,
            consequence:
                "The agent will stop this operation at its next cooperative safe boundary."
                    .to_string(),
            action: ConfirmAction::CancelOperation { operation_id },
        });
    }

    fn handle_console_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('>') => self.current_session_mut().console.begin_command(),
            KeyCode::Char('/') => self.current_session_mut().console.begin_search(),
            KeyCode::Char('p') => self.current_session_mut().console.begin_palette(),
            KeyCode::Char('f') => self.current_session_mut().console.toggle_follow(),
            KeyCode::Char(' ') => self.current_session_mut().console.toggle_paused(),
            KeyCode::Char('l') => self.current_session_mut().console.clear_local_history(),
            KeyCode::Char('v') => self.current_session_mut().console.toggle_selection_anchor(),
            KeyCode::Char('y') => {
                let count = self.console().selected_text().lines().count();
                self.last_action = Some(format!("Selected {count} console line(s) for copying"));
            }
            KeyCode::Char('C') => self.current_session_mut().console.toggle_collapsed(),
            KeyCode::Char('0'..='6') => {
                let character = match key {
                    KeyCode::Char(character) => character,
                    _ => unreachable!(),
                };
                self.current_session_mut()
                    .console
                    .select_filter_key(character);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.current_session_mut().console.move_selection(-1)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.current_session_mut().console.move_selection(1)
            }
            _ => return false,
        }
        true
    }

    fn handle_console_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.current_session_mut().console.cancel_input(),
            KeyCode::Backspace => self.current_session_mut().console.pop_input(),
            KeyCode::Up if !self.console().palette_open() => {
                self.current_session_mut().console.history_previous()
            }
            KeyCode::Down if !self.console().palette_open() => {
                self.current_session_mut().console.history_next()
            }
            KeyCode::Up => self.current_session_mut().console.move_palette(-1),
            KeyCode::Down => self.current_session_mut().console.move_palette(1),
            KeyCode::Enter if self.console().palette_open() => {
                let command = self.console().selected_palette_command().to_string();
                self.current_session_mut().console.cancel_input();
                self.send_console_command(command);
            }
            KeyCode::Enter => {
                if let Some(command) = self.current_session_mut().console.take_command() {
                    self.send_console_command(command);
                } else {
                    self.current_session_mut().console.cancel_input();
                }
            }
            KeyCode::Char(character) => self.current_session_mut().console.push_input(character),
            _ => {}
        }
    }

    fn send_console_command(&mut self, command: String) {
        let Some(client) = self.current_session().client.clone() else {
            self.last_action = Some("No authenticated host session".to_string());
            return;
        };
        match run_blocking(ConsoleView::send_command(&client, &command)) {
            Ok(result) => {
                self.last_action = Some(format!("Sent raw command: {}", result.command));
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
