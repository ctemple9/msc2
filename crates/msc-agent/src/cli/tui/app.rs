//! Interaction state for the terminal shell. Agent data remains in the
//! overview view model and is never invented or persisted as local authority.

use std::collections::BTreeMap;
use std::future::Future;

use crossterm::event::KeyCode;
use ratatui::layout::Rect;

use super::access::{AccessIntent, AccessMutation, AccessState};
use super::activity::ActivityState;
use super::agent::{AgentIntent, AgentServiceAction, AgentState};
use super::app_settings::{AppSettingsIntent, AppSettingsState};
use super::backups::{BackupIntent, BackupMutation, BackupsState};
use super::components::{ComponentIntent, ComponentMutation, ComponentsState};
use super::confirm::{ConfirmAction, ConfirmationRequest, ConfirmationResult, ConfirmationState};
use super::connections::{ConnectionIntent, ConnectionMutation, ConnectionsState};
use super::console::ConsoleView;
use super::files::{FilesIntent, FilesState};
use super::handbook::{HandbookIntent, HandbookState, HandbookSurface};
use super::health::{HealthIntent, HealthMutation, HealthState};
use super::layout::{LayoutMode, ShellLayout};
#[cfg(target_os = "macos")]
use super::local_bootstrap;
use super::manage_servers::{ManageIntent, ManageMutation, ManageServersState};
use super::overview::{OverviewState, TAB_NAMES};
use super::performance::PerformanceState;
use super::players::{PlayerIntent, PlayerMutation, PlayersState};
use super::server_editor::{EditorIntent, EditorMutation, ServerEditorState};
use super::settings::{SettingsIntent, SettingsMutation, SettingsState};
use super::transport::SharedClient;
use super::worlds::{WorldIntent, WorldMutation, WorldsState};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportSurface {
    Help,
    Agent,
    Handbook,
    AppSettings,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AdminSurface {
    #[default]
    Settings,
    Connections,
    Health,
    Access,
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
    settings_surface: AdminSurface,
    support_surface: Option<SupportSurface>,
    agent: AgentState,
    handbook: HandbookState,
    app_settings: AppSettingsState,
}

#[derive(Debug, Clone)]
struct HostSession {
    host: String,
    client: Option<SharedClient>,
    overview: OverviewState,
    console: ConsoleView,
    players: PlayersState,
    performance: PerformanceState,
    worlds: WorldsState,
    backups: BackupsState,
    components: ComponentsState,
    manage_servers: ManageServersState,
    editor: Option<ServerEditorState>,
    settings: SettingsState,
    connections: ConnectionsState,
    health: HealthState,
    access: AccessState,
    files: FilesState,
}

enum AppIntent {
    World(WorldIntent),
    Backup(BackupIntent),
    Component(ComponentIntent),
    Files(FilesIntent),
}

enum AdminIntent {
    Settings(SettingsIntent),
    Connections(ConnectionIntent),
    Health(HealthIntent),
    Access(AccessIntent),
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
        let manage_servers = ManageServersState::from_servers(overview.servers.clone());
        Self {
            sessions: vec![HostSession {
                host,
                client,
                console: ConsoleView::from_lines(overview.console.clone()),
                overview,
                players: PlayersState::default(),
                performance: PerformanceState::default(),
                worlds: WorldsState::default(),
                backups: BackupsState::default(),
                components: ComponentsState::default(),
                manage_servers,
                editor: None,
                settings: SettingsState::default(),
                connections: ConnectionsState::default(),
                health: HealthState::default(),
                access: AccessState::default(),
                files: FilesState::default(),
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
            settings_surface: AdminSurface::Settings,
            support_surface: None,
            agent: AgentState::default(),
            handbook: HandbookState::default(),
            app_settings: AppSettingsState {
                show_first_session_guide: true,
                ..AppSettingsState::default()
            },
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
        if let Some(surface) = self.support_surface {
            if key == KeyCode::Esc {
                self.support_surface = None;
                self.focus = FocusTarget::Content;
                return false;
            }
            match surface {
                SupportSurface::Help => {}
                SupportSurface::Agent => {
                    if let Some(intent) = self.agent.handle_key(key) {
                        self.handle_agent_intent(intent);
                    }
                }
                SupportSurface::Handbook => {
                    if let Some(intent) = self.handbook.handle_key(key) {
                        self.handle_handbook_intent(intent);
                    }
                }
                SupportSurface::AppSettings => {
                    if let Some(intent) = self.app_settings.handle_key(key) {
                        self.handle_app_settings_intent(intent);
                    }
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
        if self.editor().is_some() {
            let intent = self
                .current_session_mut()
                .editor
                .as_mut()
                .and_then(|editor| editor.handle_key(key));
            if let Some(intent) = intent {
                self.handle_editor_intent(intent);
            }
            return false;
        }
        if self.manage_servers().is_open() {
            let intent = self.current_session_mut().manage_servers.handle_key(key);
            if let Some(intent) = intent {
                self.handle_manage_intent(intent);
            }
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
        if let Some(handled) = self.handle_global_navigation_key(key) {
            return handled;
        }
        if self.active_tab == 1 && self.focus == FocusTarget::Content {
            let intent = self.current_session_mut().players.handle_key(key);
            if let Some(intent) = intent {
                self.handle_player_intent(intent);
            }
            return false;
        }
        if self.active_tab == 3 && self.focus == FocusTarget::Content {
            if key == KeyCode::Char('r') {
                self.current_session_mut().performance.loaded = false;
            }
            return false;
        }
        if self.active_tab == 2 && self.focus == FocusTarget::Content {
            let intent = if self.current_session().backups.open {
                self.current_session_mut()
                    .backups
                    .handle_key(key)
                    .map(AppIntent::Backup)
            } else {
                self.current_session_mut()
                    .worlds
                    .handle_key(key)
                    .map(AppIntent::World)
            };
            if let Some(intent) = intent {
                self.handle_app_intent(intent);
            }
            return false;
        }
        if self.active_tab == 4 && self.focus == FocusTarget::Content {
            let intent = self
                .current_session_mut()
                .components
                .handle_key(key)
                .map(AppIntent::Component);
            if let Some(intent) = intent {
                self.handle_app_intent(intent);
            }
            return false;
        }
        if self.active_tab == 5 && self.focus == FocusTarget::Content {
            let intent = match self.settings_surface {
                AdminSurface::Settings => self
                    .current_session_mut()
                    .settings
                    .handle_key(key)
                    .map(AdminIntent::Settings),
                AdminSurface::Connections => self
                    .current_session_mut()
                    .connections
                    .handle_key(key)
                    .map(AdminIntent::Connections),
                AdminSurface::Health => self
                    .current_session_mut()
                    .health
                    .handle_key(key)
                    .map(AdminIntent::Health),
                AdminSurface::Access => self
                    .current_session_mut()
                    .access
                    .handle_key(key)
                    .map(AdminIntent::Access),
            };
            if let Some(intent) = intent {
                self.handle_admin_intent(intent);
            }
            return false;
        }
        if self.active_tab == 6 && self.focus == FocusTarget::Content {
            let intent = self
                .current_session_mut()
                .files
                .handle_key(key)
                .map(AppIntent::Files);
            if let Some(intent) = intent {
                self.handle_app_intent(intent);
            }
            return false;
        }
        match key {
            KeyCode::Char('q') => return true,
            KeyCode::Char('m') => {
                self.current_session_mut().manage_servers.open();
                self.focus = FocusTarget::Content;
            }
            KeyCode::Char('v') if self.focus == FocusTarget::Rail => {
                self.open_admin_surface(AdminSurface::Connections);
                self.current_session_mut().connections.surface =
                    super::connections::ConnectionSurface::Services;
            }
            KeyCode::Char('h') if self.focus == FocusTarget::Rail => {
                self.open_admin_surface(AdminSurface::Connections);
            }
            KeyCode::Char('d') if self.focus == FocusTarget::Rail => {
                self.open_admin_surface(AdminSurface::Health);
            }
            KeyCode::Char('u') if self.focus == FocusTarget::Rail => {
                self.open_admin_surface(AdminSurface::Access);
            }
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
            KeyCode::Char('?') => self.open_support(SupportSurface::Help),
            KeyCode::Char('g') => self.open_support(SupportSurface::Handbook),
            KeyCode::Char('A') => self.open_support(SupportSurface::Agent),
            KeyCode::Char(',') => self.open_support(SupportSurface::AppSettings),
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

    /// Global navigation must remain reachable from ordinary content views,
    /// but an active text field or detail/action surface owns its keystrokes.
    fn handle_global_navigation_key(&mut self, key: KeyCode) -> Option<bool> {
        if self.content_captures_keys() {
            return None;
        }

        match key {
            KeyCode::Char('q') => Some(true),
            KeyCode::Tab => {
                self.advance_focus();
                Some(false)
            }
            KeyCode::BackTab => {
                self.reverse_focus();
                Some(false)
            }
            KeyCode::Char('?') => {
                self.open_support(SupportSurface::Help);
                Some(false)
            }
            KeyCode::Char('g') => {
                self.open_support(SupportSurface::Handbook);
                Some(false)
            }
            KeyCode::Char('A') => {
                self.open_support(SupportSurface::Agent);
                Some(false)
            }
            KeyCode::Char(',') => {
                self.open_support(SupportSurface::AppSettings);
                Some(false)
            }
            KeyCode::Char('m') => {
                self.current_session_mut().manage_servers.open();
                self.focus = FocusTarget::Content;
                Some(false)
            }
            KeyCode::Char('1'..='7') if self.section_key_is_global() => {
                let index = key_to_tab(key);
                if self.overview().tab_is_available(index) {
                    self.active_tab = index;
                }
                Some(false)
            }
            _ => None,
        }
    }

    fn section_key_is_global(&self) -> bool {
        !matches!(self.active_tab, 4 | 5)
    }

    fn content_captures_keys(&self) -> bool {
        if self.focus != FocusTarget::Content {
            return false;
        }
        match self.active_tab {
            1 => {
                let players = &self.current_session().players;
                players.input.is_some() || players.detail_open
            }
            2 => {
                let session = self.current_session();
                session.worlds.input.is_some() || session.worlds.detail_open
            }
            4 => {
                let components = &self.current_session().components;
                components.input.is_some() || components.detail_open || components.action_menu_open
            }
            5 => {
                let session = self.current_session();
                session.settings.input.is_some()
                    || session.connections.input.is_some()
                    || session.health.detail_open
                    || session.access.input.is_some()
            }
            6 => self.current_session().files.detail_open,
            _ => false,
        }
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

    pub fn poll_sections(&mut self) {
        match self.active_tab {
            1 => self.load_players_if_needed(),
            2 => {
                self.load_worlds_if_needed();
                if self.current_session().backups.open {
                    self.load_backups_if_needed();
                }
            }
            3 => self.load_performance_if_needed(),
            4 => self.load_components_if_needed(),
            5 => self.load_admin_if_needed(),
            6 => self.load_files_if_needed(),
            _ => {}
        }
    }

    pub fn players(&self) -> &PlayersState {
        &self.current_session().players
    }

    pub fn performance(&self) -> &PerformanceState {
        &self.current_session().performance
    }

    pub fn worlds(&self) -> &WorldsState {
        &self.current_session().worlds
    }

    pub fn backups(&self) -> &BackupsState {
        &self.current_session().backups
    }

    pub fn components(&self) -> &ComponentsState {
        &self.current_session().components
    }

    pub fn manage_servers(&self) -> &ManageServersState {
        &self.current_session().manage_servers
    }

    pub fn editor(&self) -> Option<&ServerEditorState> {
        self.current_session().editor.as_ref()
    }

    pub fn settings_surface(&self) -> AdminSurface {
        self.settings_surface
    }

    pub fn settings(&self) -> &SettingsState {
        &self.current_session().settings
    }

    pub fn connections(&self) -> &ConnectionsState {
        &self.current_session().connections
    }

    pub fn health(&self) -> &HealthState {
        &self.current_session().health
    }

    pub fn access(&self) -> &AccessState {
        &self.current_session().access
    }

    pub fn files(&self) -> &FilesState {
        &self.current_session().files
    }

    pub fn open_admin_surface(&mut self, surface: AdminSurface) {
        self.settings_surface = surface;
        self.active_tab = 5;
        self.focus = FocusTarget::Content;
    }

    pub fn available_tabs(&self) -> Vec<usize> {
        let mut tabs = self.overview().available_tabs();
        if self
            .overview()
            .capabilities
            .as_ref()
            .is_some_and(|capabilities| {
                !capabilities
                    .base
                    .permissions
                    .contains(&msc_api::dto::PermissionCategoryDto::Admin)
            })
        {
            tabs.retain(|tab| *tab != 6);
        }
        tabs
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

    pub fn support_surface(&self) -> Option<SupportSurface> {
        self.support_surface
    }

    pub fn agent(&self) -> &AgentState {
        &self.agent
    }

    pub fn handbook(&self) -> &HandbookState {
        &self.handbook
    }

    pub fn app_settings(&self) -> &AppSettingsState {
        &self.app_settings
    }

    pub fn open_support(&mut self, surface: SupportSurface) {
        self.support_surface = Some(surface);
        self.focus = FocusTarget::Content;
        match surface {
            SupportSurface::Agent if !self.agent.loaded => self.load_agent_support(),
            SupportSurface::Handbook if !self.handbook.loaded => self.load_handbook_support(),
            _ => {}
        }
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
        let manage_servers = ManageServersState::from_servers(overview.servers.clone());
        let mut console = ConsoleView::from_lines(overview.console.clone());
        console.start_feed(client.clone());
        self.sessions.push(HostSession {
            host,
            client: Some(client),
            console,
            overview,
            players: PlayersState::default(),
            performance: PerformanceState::default(),
            worlds: WorldsState::default(),
            backups: BackupsState::default(),
            components: ComponentsState::default(),
            manage_servers,
            editor: None,
            settings: SettingsState::default(),
            connections: ConnectionsState::default(),
            health: HealthState::default(),
            access: AccessState::default(),
            files: FilesState::default(),
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
                let servers = overview.servers.clone();
                self.current_session_mut().overview = overview;
                self.current_session_mut()
                    .manage_servers
                    .set_servers(servers);
                self.current_session_mut().files = FilesState::default();
                self.last_action = Some(format!("Selected server {selector}"));
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn handle_manage_intent(&mut self, intent: ManageIntent) {
        match intent {
            ManageIntent::OpenEditor { server_id } => self.open_editor(&server_id),
            ManageIntent::Confirm(mutation) => self.begin_manage_confirmation(mutation),
        }
    }

    fn open_editor(&mut self, server_id: &str) {
        let Some(server) = self
            .manage_servers()
            .servers
            .iter()
            .find(|server| server.id == server_id)
            .cloned()
        else {
            self.last_action = Some("Selected server is no longer available".to_string());
            return;
        };
        let Some(client) = self.current_session().client.clone() else {
            self.last_action =
                Some("Server editor requires an authenticated host session".to_string());
            return;
        };
        match run_blocking(ServerEditorState::load(&client, server)) {
            Ok(mut editor) => {
                let playit_available = self
                    .overview()
                    .capabilities
                    .as_ref()
                    .is_some_and(|capabilities| capabilities.base.helpers.playit);
                let broadcast_available =
                    self.has_permission(msc_api::dto::PermissionCategoryDto::Broadcast);
                editor.set_capabilities(playit_available, broadcast_available);
                self.current_session_mut().editor = Some(editor);
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn begin_manage_confirmation(&mut self, mutation: ManageMutation) {
        let permission = match &mutation {
            ManageMutation::SetActive { .. } => msc_api::dto::PermissionCategoryDto::ServerControl,
            ManageMutation::Create(_)
            | ManageMutation::Import(_)
            | ManageMutation::Rename { .. }
            | ManageMutation::AcceptEula { .. }
            | ManageMutation::Delete { .. } => msc_api::dto::PermissionCategoryDto::Fleet,
        };
        if !self.has_permission(permission) {
            self.last_action = Some("This credential cannot manage the server fleet".to_string());
            return;
        }
        let (target, consequence) = manage_confirmation_details(&mutation);
        self.confirmation.begin(ConfirmationRequest {
            host: self.host().to_string(),
            server: self.overview().selected_server_name().to_string(),
            target,
            consequence,
            action: ConfirmAction::ManageMutation(mutation),
        });
    }

    fn handle_editor_intent(&mut self, intent: EditorIntent) {
        match intent {
            EditorIntent::Back => self.current_session_mut().editor = None,
            EditorIntent::Confirm(mutation) => self.begin_editor_confirmation(mutation),
        }
    }

    fn begin_editor_confirmation(&mut self, mutation: EditorMutation) {
        let permission = match &mutation {
            EditorMutation::Rename { .. } | EditorMutation::SetDirectory { .. } => {
                msc_api::dto::PermissionCategoryDto::Fleet
            }
            EditorMutation::UpdateRam { .. }
            | EditorMutation::UpdatePort { .. }
            | EditorMutation::SetJavaPath { .. }
            | EditorMutation::SetJavaArguments { .. } => {
                msc_api::dto::PermissionCategoryDto::Settings
            }
            EditorMutation::Playit { .. } => msc_api::dto::PermissionCategoryDto::Networking,
            EditorMutation::XboxBroadcast { .. } => msc_api::dto::PermissionCategoryDto::Broadcast,
        };
        if !self.has_permission(permission) {
            self.last_action =
                Some("This credential cannot change that editor section".to_string());
            return;
        }
        let (target, consequence) = editor_confirmation_details(&mutation);
        self.confirmation.begin(ConfirmationRequest {
            host: self.host().to_string(),
            server: self.overview().selected_server_name().to_string(),
            target,
            consequence,
            action: ConfirmAction::EditorMutation(mutation),
        });
    }

    fn execute_manage_mutation(&mut self, client: SharedClient, mutation: ManageMutation) {
        match run_blocking(super::manage_servers::execute(&client, mutation)) {
            Ok(result) => {
                self.last_action = Some(result.message);
                if let Some(operation_id) = result.operation_id {
                    self.activity
                        .track_operation(client.clone(), operation_id.clone());
                    self.last_action = Some(format!("Operation {operation_id} is being tracked"));
                }
                self.reload_overview(&client);
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn execute_editor_mutation(&mut self, client: SharedClient, mutation: EditorMutation) {
        match run_blocking(super::server_editor::execute(&client, mutation)) {
            Ok(result) => {
                self.last_action = Some(result.message.clone());
                if let Some(editor) = self.current_session_mut().editor.as_mut() {
                    editor.status = Some(result.message);
                    editor.loaded = false;
                }
                if let Some(operation_id) = result.operation_id {
                    self.activity
                        .track_operation(client.clone(), operation_id.clone());
                    self.last_action = Some(format!("Operation {operation_id} is being tracked"));
                }
                self.reload_overview(&client);
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn reload_overview(&mut self, client: &SharedClient) {
        match run_blocking(OverviewState::load(client)) {
            Ok(overview) => {
                let servers = overview.servers.clone();
                self.current_session_mut().overview = overview;
                self.current_session_mut()
                    .manage_servers
                    .set_servers(servers);
                self.current_session_mut().files = FilesState::default();
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
            ConfirmAction::PlayerMutation(mutation) => {
                self.execute_player_mutation(client, mutation)
            }
            ConfirmAction::WorldMutation(mutation) => self.execute_world_mutation(client, mutation),
            ConfirmAction::BackupMutation(mutation) => {
                self.execute_backup_mutation(client, mutation)
            }
            ConfirmAction::ComponentMutation(mutation) => {
                self.execute_component_mutation(client, mutation)
            }
            ConfirmAction::ManageMutation(mutation) => {
                self.execute_manage_mutation(client, mutation)
            }
            ConfirmAction::EditorMutation(mutation) => {
                self.execute_editor_mutation(client, mutation)
            }
            ConfirmAction::SettingsMutation(mutation) => {
                self.execute_settings_mutation(client, mutation)
            }
            ConfirmAction::ConnectionMutation(mutation) => {
                self.execute_connection_mutation(client, mutation)
            }
            ConfirmAction::HealthMutation(mutation) => {
                self.execute_health_mutation(client, mutation)
            }
            ConfirmAction::AccessMutation(mutation) => {
                self.execute_access_mutation(client, mutation)
            }
        }
    }

    fn handle_agent_intent(&mut self, intent: AgentIntent) {
        match intent {
            AgentIntent::Service(action) => match AgentState::execute_service(action) {
                Ok(status) => {
                    self.agent.service = status;
                    self.agent.status = Some(format!("Agent service {} completed", action.label()));
                    if action == AgentServiceAction::Stop {
                        self.current_session_mut().client = None;
                        self.agent.status = Some(
                            "Agent service stopped; the TUI session was disconnected.".to_string(),
                        );
                    } else if self.current_session().client.is_some() {
                        if action == AgentServiceAction::Reconnect {
                            self.replace_current_client_after_reconnect();
                        }
                    } else {
                        self.connect_local_agent();
                    }
                }
                Err(error) => self.agent.error = Some(error),
            },
            AgentIntent::BeginPairing => {
                let Some(client) = self.current_session().client.clone() else {
                    self.agent.error =
                        Some("Pairing creation needs an authenticated host session.".to_string());
                    return;
                };
                if let Err(error) = run_blocking(self.agent.create_pairing(&client)) {
                    self.agent.error = Some(error.to_string());
                }
            }
            AgentIntent::ExchangePairing(code) => {
                let Some(client) = self.current_session().client.clone() else {
                    self.agent.error =
                        Some("Pairing exchange needs a host URL and current session.".to_string());
                    return;
                };
                match run_blocking(self.agent.exchange_pairing(&client, code)) {
                    Ok(exchange) => {
                        self.current_session_mut().client = Some(exchange.client.clone());
                        self.reload_overview(&exchange.client);
                        self.current_session_mut()
                            .console
                            .start_feed(exchange.client.clone());
                        self.last_action = Some(format!(
                            "Paired host {}; credential kept in memory",
                            exchange.agent_host_id
                        ));
                    }
                    Err(error) => self.agent.error = Some(error.to_string()),
                }
            }
        }
    }

    fn handle_handbook_intent(&mut self, intent: HandbookIntent) {
        let Some(client) = self.current_session().client.clone() else {
            self.handbook.error =
                Some("Handbook requires an authenticated host session".to_string());
            return;
        };
        let result = match intent {
            HandbookIntent::LoadTopic(id) => {
                run_blocking(HandbookState::topic(&client, id)).map(|topic| {
                    self.handbook.topic = Some(topic);
                    self.handbook.surface = HandbookSurface::Topic;
                })
            }
            HandbookIntent::SearchRouter(query) => {
                self.handbook.query = query;
                run_blocking(HandbookState::search_router(
                    &client,
                    self.handbook.query.clone(),
                ))
                .map(|search| {
                    self.handbook.router_search = Some(search);
                    self.handbook.router_selected = 0;
                })
            }
            HandbookIntent::OpenRouterGuide(id) => {
                run_blocking(HandbookState::router_guide(&client, id)).map(|guide| {
                    self.handbook.router_guide = Some(guide);
                    self.handbook.surface = HandbookSurface::RouterGuide;
                })
            }
            HandbookIntent::AnalyzeTroubleshooting(symptoms) => run_blocking(
                HandbookState::analyze_troubleshooting(&client, symptoms),
            )
            .map(|analysis| {
                self.handbook.troubleshooting = Some(analysis);
            }),
        };
        if let Err(error) = result {
            self.handbook.error = Some(error.to_string());
        }
    }

    fn handle_app_settings_intent(&mut self, intent: AppSettingsIntent) {
        match intent {
            AppSettingsIntent::ResetClient => {
                self.notes.clear();
                self.app_settings.status = Some(
                    "Client preferences reset; the agent and host data were not changed."
                        .to_string(),
                );
                self.last_action = Some("Reset this client completed".to_string());
            }
            AppSettingsIntent::HostReset { mode, confirmation } => {
                let Some(client) = self.current_session().client.clone() else {
                    self.app_settings.error =
                        Some("Host reset requires an authenticated host session".to_string());
                    return;
                };
                match run_blocking(AppSettingsState::reset_host(&client, mode, confirmation)) {
                    Ok(result) => {
                        self.app_settings.host_reset_result = Some(result.clone());
                        self.app_settings.host_reset_confirmation = None;
                        self.app_settings.status = Some(result.message.clone());
                        self.activity
                            .track_operation(client, result.operation_id.clone());
                        self.last_action = Some(
                            "Host reset accepted; this credential is no longer a recovery path. Pair again after completion."
                                .to_string(),
                        );
                    }
                    Err(error) => self.app_settings.error = Some(error.to_string()),
                }
            }
        }
    }

    fn load_agent_support(&mut self) {
        let service = super::agent::local_service_status();
        if let Some(client) = self.current_session().client.clone() {
            match run_blocking(AgentState::load(&client)) {
                Ok(mut state) => {
                    state.service = service;
                    self.agent = state;
                }
                Err(error) => {
                    self.agent.service = service;
                    self.agent.error = Some(error.to_string());
                    self.agent.loaded = true;
                }
            }
        } else {
            self.agent.service = service;
            self.agent.status = Some(
                "No TUI session yet. Use I to install, S to start, or R to reconnect.".to_string(),
            );
            self.agent.loaded = true;
        }
    }

    fn load_handbook_support(&mut self) {
        let Some(client) = self.current_session().client.clone() else {
            self.handbook.error = Some("Handbook needs an authenticated host session".to_string());
            self.handbook.loaded = true;
            return;
        };
        match run_blocking(HandbookState::load(&client)) {
            Ok(state) => self.handbook = state,
            Err(error) => {
                self.handbook.error = Some(error.to_string());
                self.handbook.loaded = true;
            }
        }
    }

    fn replace_current_client_after_reconnect(&mut self) {
        let Some(client) = self.current_session().client.clone() else {
            return;
        };
        self.reload_overview(&client);
    }

    fn connect_local_agent(&mut self) {
        #[cfg(target_os = "macos")]
        {
            let mut connection = Err(String::new());
            for attempt in 0..10 {
                connection = local_bootstrap::connect_for_host(self.host());
                if connection.is_ok() || attempt == 9 {
                    break;
                }
                if !connection
                    .as_ref()
                    .err()
                    .is_some_and(|error| error.starts_with("connecting to local agent"))
                {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            match connection {
                Ok(client) => {
                    self.current_session_mut().client = Some(client.clone());
                    self.reload_overview(&client);
                    self.current_session_mut()
                        .console
                        .start_feed(client.clone());
                    self.activity.start_notifications(client);
                    self.agent.error = None;
                    self.agent.status = Some(
                        "Connected to the local agent; credential kept in memory.".to_string(),
                    );
                }
                Err(error) => {
                    self.agent.error = Some(format!(
                        "Service action completed, but local bootstrap is not ready: {error}"
                    ));
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.agent.error = Some(
                "Automatic local bootstrap is currently supported on macOS; pass --token for this host."
                    .to_string(),
            );
        }
    }

    fn handle_admin_intent(&mut self, intent: AdminIntent) {
        match intent {
            AdminIntent::Settings(SettingsIntent::Confirm(mutation)) => {
                if self.has_permission(msc_api::dto::PermissionCategoryDto::Settings) {
                    self.confirmation.begin(ConfirmationRequest {
                        host: self.host().to_string(),
                        server: self.overview().selected_server_name().to_string(),
                        target: "server settings".to_string(),
                        consequence: "The agent will validate and apply these settings; a restart may be required.".to_string(),
                        action: ConfirmAction::SettingsMutation(mutation),
                    });
                } else {
                    self.last_action =
                        Some("This credential cannot change server settings".to_string());
                }
            }
            AdminIntent::Connections(ConnectionIntent::Confirm(mutation)) => {
                let permission = match &mutation {
                    ConnectionMutation::Playit { .. } => {
                        msc_api::dto::PermissionCategoryDto::Networking
                    }
                    ConnectionMutation::Broadcast { .. }
                    | ConnectionMutation::BroadcastAutostart { .. }
                    | ConnectionMutation::BroadcastCredentials { .. } => {
                        msc_api::dto::PermissionCategoryDto::Broadcast
                    }
                    ConnectionMutation::DuckDns { .. } => {
                        msc_api::dto::PermissionCategoryDto::Settings
                    }
                };
                if self.has_permission(permission) {
                    let (target, consequence) = connection_confirmation_details(&mutation);
                    self.confirmation.begin(ConfirmationRequest {
                        host: self.host().to_string(),
                        server: self.overview().selected_server_name().to_string(),
                        target,
                        consequence,
                        action: ConfirmAction::ConnectionMutation(mutation),
                    });
                } else {
                    self.last_action =
                        Some("This credential cannot change that connection setting".to_string());
                }
            }
            AdminIntent::Health(HealthIntent::Confirm(mutation)) => {
                if self.has_permission(msc_api::dto::PermissionCategoryDto::Settings) {
                    let (target, consequence) = health_confirmation_details(&mutation);
                    self.confirmation.begin(ConfirmationRequest {
                        host: self.host().to_string(),
                        server: self.overview().selected_server_name().to_string(),
                        target,
                        consequence,
                        action: ConfirmAction::HealthMutation(mutation),
                    });
                } else {
                    self.last_action =
                        Some("This credential cannot repair server health problems".to_string());
                }
            }
            AdminIntent::Access(AccessIntent::Confirm(mutation)) => {
                let permission = match &mutation {
                    AccessMutation::AllowlistAdd { .. }
                    | AccessMutation::AllowlistRemove { .. } => {
                        msc_api::dto::PermissionCategoryDto::Players
                    }
                    AccessMutation::RevokeUser { .. } => msc_api::dto::PermissionCategoryDto::Admin,
                };
                if self.has_permission(permission) {
                    let (target, consequence) = access_confirmation_details(&mutation);
                    self.confirmation.begin(ConfirmationRequest {
                        host: self.host().to_string(),
                        server: self.overview().selected_server_name().to_string(),
                        target,
                        consequence,
                        action: ConfirmAction::AccessMutation(mutation),
                    });
                } else {
                    self.last_action =
                        Some("This credential cannot change that access list".to_string());
                }
            }
        }
    }

    fn handle_app_intent(&mut self, intent: AppIntent) {
        match intent {
            AppIntent::World(WorldIntent::OpenBackups) => {
                let slot_id = self
                    .current_session()
                    .worlds
                    .selected_slot_id()
                    .map(str::to_owned);
                let backups = &mut self.current_session_mut().backups;
                backups.context_slot_id = slot_id;
                backups.open = true;
                backups.loaded = false;
            }
            AppIntent::World(WorldIntent::Confirm(mutation)) => {
                self.begin_world_confirmation(mutation);
            }
            AppIntent::Backup(BackupIntent::Confirm(mutation)) => {
                self.begin_backup_confirmation(mutation);
            }
            AppIntent::Component(ComponentIntent::Search(query)) => {
                self.search_components(&query);
            }
            AppIntent::Component(ComponentIntent::Confirm(mutation)) => {
                self.begin_component_confirmation(mutation);
            }
            AppIntent::Files(intent) => self.handle_files_intent(intent),
        }
    }

    fn handle_files_intent(&mut self, intent: FilesIntent) {
        match intent {
            FilesIntent::Navigate(path) => {
                self.current_session_mut().files.set_requested_path(path);
            }
            FilesIntent::Preview(path) => {
                let Some(client) = self.current_session().client.clone() else {
                    self.current_session_mut().files.error =
                        Some("File preview requires an authenticated host session".to_string());
                    return;
                };
                if let Err(error) =
                    run_blocking(self.current_session_mut().files.preview(&client, &path))
                {
                    self.current_session_mut().files.error = Some(error.to_string());
                }
            }
            FilesIntent::ReportPath(path) => {
                self.last_action = Some(format!("Reported path: {path}"));
            }
        }
    }

    fn begin_component_confirmation(&mut self, mutation: ComponentMutation) {
        if !self.has_permission(msc_api::dto::PermissionCategoryDto::Addons) {
            self.last_action = Some("This credential cannot change components".to_string());
            return;
        }
        let (target, consequence) = component_confirmation_details(&mutation);
        self.confirmation.begin(ConfirmationRequest {
            host: self.host().to_string(),
            server: self.overview().selected_server_name().to_string(),
            target,
            consequence,
            action: ConfirmAction::ComponentMutation(mutation),
        });
    }

    fn execute_component_mutation(&mut self, client: SharedClient, mutation: ComponentMutation) {
        match run_blocking(super::components::execute(&client, mutation)) {
            Ok(result) => {
                self.current_session_mut().components.loaded = false;
                self.last_action = Some(result.message);
                if let Some(operation_id) = result.operation_id {
                    self.activity.track_operation(client, operation_id.clone());
                    self.last_action = Some(format!("Operation {operation_id} is being tracked"));
                }
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn search_components(&mut self, query: &str) {
        let Some(client) = self.current_session().client.clone() else {
            self.last_action =
                Some("Catalog search requires an authenticated host session".to_string());
            return;
        };
        match run_blocking(self.current_session_mut().components.search(&client, query)) {
            Ok(()) => self.last_action = Some(format!("Catalog search: {query}")),
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn begin_world_confirmation(&mut self, mutation: WorldMutation) {
        if !self.has_permission(msc_api::dto::PermissionCategoryDto::Worlds) {
            self.last_action = Some("This credential cannot change worlds or backups".to_string());
            return;
        }
        let (target, consequence) = world_confirmation_details(&mutation);
        self.confirmation.begin(ConfirmationRequest {
            host: self.host().to_string(),
            server: self.overview().selected_server_name().to_string(),
            target,
            consequence,
            action: ConfirmAction::WorldMutation(mutation),
        });
    }

    fn begin_backup_confirmation(&mut self, mutation: BackupMutation) {
        let permission = if matches!(mutation, BackupMutation::UpdateConfig { .. }) {
            msc_api::dto::PermissionCategoryDto::Settings
        } else {
            msc_api::dto::PermissionCategoryDto::Worlds
        };
        if !self.has_permission(permission) {
            self.last_action = Some("This credential cannot change backup state".to_string());
            return;
        }
        let (target, consequence) = backup_confirmation_details(&mutation);
        self.confirmation.begin(ConfirmationRequest {
            host: self.host().to_string(),
            server: self.overview().selected_server_name().to_string(),
            target,
            consequence,
            action: ConfirmAction::BackupMutation(mutation),
        });
    }

    fn has_permission(&self, permission: msc_api::dto::PermissionCategoryDto) -> bool {
        self.overview()
            .capabilities
            .as_ref()
            .is_some_and(|capabilities| capabilities.base.permissions.contains(&permission))
    }

    fn execute_world_mutation(&mut self, client: SharedClient, mutation: WorldMutation) {
        match run_blocking(super::worlds::execute(&client, mutation)) {
            Ok(result) => {
                self.current_session_mut().worlds.loaded = false;
                self.current_session_mut().backups.loaded = false;
                self.last_action = Some(result.message);
                if let Some(operation_id) = result.operation_id {
                    self.activity.track_operation(client, operation_id.clone());
                    self.last_action = Some(format!("Operation {operation_id} is being tracked"));
                }
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn execute_backup_mutation(&mut self, client: SharedClient, mutation: BackupMutation) {
        match run_blocking(super::backups::execute(&client, mutation)) {
            Ok(result) => {
                self.current_session_mut().backups.loaded = false;
                self.last_action = Some(result.message);
                if let Some(operation_id) = result.operation_id {
                    self.activity.track_operation(client, operation_id.clone());
                    self.last_action = Some(format!("Operation {operation_id} is being tracked"));
                }
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn handle_player_intent(&mut self, intent: PlayerIntent) {
        match intent {
            PlayerIntent::ClearLocalSession => {
                self.current_session_mut().players.clear_local_session();
                self.last_action = Some("Session history cleared locally".to_string());
            }
            PlayerIntent::ClearSessionLog => {
                self.begin_player_confirmation(
                    PlayerMutation::ClearSessionLog,
                    "The agent will delete the selected server's recorded join/leave history."
                        .to_string(),
                );
            }
            PlayerIntent::Confirm(mutation) => {
                let consequence = player_mutation_consequence(&mutation);
                self.begin_player_confirmation(mutation, consequence);
            }
        }
    }

    fn begin_player_confirmation(&mut self, mutation: PlayerMutation, consequence: String) {
        let Some(capabilities) = &self.overview().capabilities else {
            self.last_action = Some("Player permissions are unavailable".to_string());
            return;
        };
        if !capabilities
            .base
            .permissions
            .contains(&msc_api::dto::PermissionCategoryDto::Players)
        {
            self.last_action = Some("This credential cannot change player data".to_string());
            return;
        }
        let target = match &mutation {
            PlayerMutation::AllowlistAdd { name } | PlayerMutation::AllowlistRemove { name } => {
                name.clone()
            }
            PlayerMutation::ClearSessionLog => self.overview().selected_server_name().to_string(),
            PlayerMutation::ToggleHidden { profile_id, .. }
            | PlayerMutation::Delete { profile_id }
            | PlayerMutation::Duplicate { profile_id }
            | PlayerMutation::MigrateOffline { profile_id }
            | PlayerMutation::MigrateCustom { profile_id, .. }
            | PlayerMutation::Identify { profile_id, .. }
            | PlayerMutation::SkinOverride { profile_id, .. } => profile_id.clone(),
        };
        self.confirmation.begin(ConfirmationRequest {
            host: self.host().to_string(),
            server: self.overview().selected_server_name().to_string(),
            target,
            consequence,
            action: ConfirmAction::PlayerMutation(mutation),
        });
    }

    fn execute_player_mutation(&mut self, client: SharedClient, mutation: PlayerMutation) {
        let (path, body) = match mutation {
            PlayerMutation::ClearSessionLog => ("/v1/session-log/clear", serde_json::json!({})),
            PlayerMutation::ToggleHidden { profile_id, hidden } => (
                "/v1/players/hidden",
                serde_json::json!({"profileId": profile_id, "hidden": hidden}),
            ),
            PlayerMutation::Delete { profile_id } => (
                "/v1/players/delete",
                serde_json::json!({"profileId": profile_id}),
            ),
            PlayerMutation::Duplicate { profile_id } => (
                "/v1/players/duplicate",
                serde_json::json!({"profileId": profile_id}),
            ),
            PlayerMutation::MigrateOffline { profile_id } => (
                "/v1/players/migrate-offline",
                serde_json::json!({"profileId": profile_id}),
            ),
            PlayerMutation::MigrateCustom {
                profile_id,
                target_uuid,
            } => (
                "/v1/players/migrate",
                serde_json::json!({"profileId": profile_id, "targetUuid": target_uuid}),
            ),
            PlayerMutation::Identify {
                profile_id,
                gamertag,
            } => (
                "/v1/players/identify",
                serde_json::json!({"profileId": profile_id, "gamertag": gamertag}),
            ),
            PlayerMutation::SkinOverride {
                profile_id,
                lookup_identifier,
            } => (
                "/v1/players/skin-override",
                serde_json::json!({"profileId": profile_id, "lookupIdentifier": lookup_identifier}),
            ),
            PlayerMutation::AllowlistAdd { name } => (
                "/v1/allowlist",
                serde_json::json!({"action": "add", "name": name}),
            ),
            PlayerMutation::AllowlistRemove { name } => (
                "/v1/allowlist",
                serde_json::json!({"action": "remove", "name": name}),
            ),
        };
        let result = run_blocking(client.post_json::<_, serde_json::Value>(path, &body));
        match result {
            Ok(_) => {
                self.last_action = Some("Player request accepted".to_string());
                if path == "/v1/session-log/clear" {
                    self.current_session_mut().players.clear_local_session();
                } else {
                    self.current_session_mut().players.loaded = false;
                }
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn execute_settings_mutation(&mut self, client: SharedClient, mutation: SettingsMutation) {
        let SettingsMutation::Update { changes } = mutation;
        match run_blocking(SettingsState::update(&client, changes)) {
            Ok(result) => {
                self.current_session_mut().settings.loaded = false;
                self.last_action = Some(result.message);
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn execute_connection_mutation(&mut self, client: SharedClient, mutation: ConnectionMutation) {
        let result = match mutation {
            ConnectionMutation::Playit { start } => {
                run_blocking(ConnectionsState::playit(&client, start))
                    .map(|result| result.message.unwrap_or(result.result))
            }
            ConnectionMutation::Broadcast { start } => {
                run_blocking(ConnectionsState::broadcast(&client, start))
                    .map(|result| result.result)
            }
            ConnectionMutation::BroadcastAutostart { enabled } => {
                run_blocking(ConnectionsState::set_autostart(&client, enabled))
                    .map(|result| format!("Xbox Broadcast autostart: {}", result.enabled))
            }
            ConnectionMutation::BroadcastCredentials {
                email,
                password,
                gamertag,
            } => run_blocking(ConnectionsState::set_credentials(
                &client, email, password, gamertag,
            ))
            .map(|result| result.result),
            ConnectionMutation::DuckDns { hostname } => {
                run_blocking(ConnectionsState::set_duckdns(&client, hostname)).map(|result| {
                    result
                        .message
                        .unwrap_or_else(|| "DuckDNS updated".to_string())
                })
            }
        };
        match result {
            Ok(message) => {
                self.current_session_mut().connections.loaded = false;
                self.last_action = Some(message);
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn execute_health_mutation(&mut self, client: SharedClient, mutation: HealthMutation) {
        let HealthMutation::Repair { problem_id, action } = mutation;
        match run_blocking(HealthState::repair(&client, problem_id, action)) {
            Ok(result) => {
                self.current_session_mut().health.loaded = false;
                self.last_action = Some(result.message);
                if let Some(operation_id) = result.operation_id {
                    self.activity.track_operation(client, operation_id.clone());
                    self.last_action = Some(format!("Operation {operation_id} is being tracked"));
                }
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn execute_access_mutation(&mut self, client: SharedClient, mutation: AccessMutation) {
        let result = match mutation {
            AccessMutation::AllowlistAdd { name } => {
                run_blocking(AccessState::mutate_allowlist(&client, true, name))
            }
            AccessMutation::AllowlistRemove { name } => {
                run_blocking(AccessState::mutate_allowlist(&client, false, name))
            }
            AccessMutation::RevokeUser { user_id } => {
                run_blocking(AccessState::revoke_user(&client, user_id))
            }
        };
        match result {
            Ok(_) => {
                self.current_session_mut().access.loaded = false;
                self.last_action = Some("Access request accepted".to_string());
            }
            Err(error) => self.last_action = Some(error.to_string()),
        }
    }

    fn load_admin_if_needed(&mut self) {
        let Some(client) = self.current_session().client.clone() else {
            let session = self.current_session_mut();
            session.settings.error =
                Some("Settings require an authenticated host session".to_string());
            session.connections.error =
                Some("Connections require an authenticated host session".to_string());
            session.health.error =
                Some("Health requires an authenticated host session".to_string());
            session.access.error =
                Some("Access requires an authenticated host session".to_string());
            session.settings.loaded = true;
            session.connections.loaded = true;
            session.health.loaded = true;
            session.access.loaded = true;
            return;
        };
        if !self.current_session().settings.loaded {
            match run_blocking(SettingsState::load(&client)) {
                Ok(state) => self.current_session_mut().settings = state,
                Err(error) => {
                    self.current_session_mut().settings.error = Some(error.to_string());
                    self.current_session_mut().settings.loaded = true;
                }
            }
        }
        if !self.current_session().connections.loaded {
            match run_blocking(ConnectionsState::load(&client)) {
                Ok(state) => self.current_session_mut().connections = state,
                Err(error) => {
                    self.current_session_mut().connections.error = Some(error.to_string());
                    self.current_session_mut().connections.loaded = true;
                }
            }
        }
        if !self.current_session().health.loaded {
            match run_blocking(HealthState::load(&client)) {
                Ok(state) => self.current_session_mut().health = state,
                Err(error) => {
                    self.current_session_mut().health.error = Some(error.to_string());
                    self.current_session_mut().health.loaded = true;
                }
            }
        }
        if !self.current_session().access.loaded {
            match run_blocking(AccessState::load(&client)) {
                Ok(state) => self.current_session_mut().access = state,
                Err(error) => {
                    self.current_session_mut().access.error = Some(error.to_string());
                    self.current_session_mut().access.loaded = true;
                }
            }
        }
    }

    fn load_players_if_needed(&mut self) {
        if self.current_session().players.loaded {
            return;
        }
        let Some(client) = self.current_session().client.clone() else {
            let state = &mut self.current_session_mut().players;
            state.error = Some("Players require an authenticated host session".to_string());
            state.loaded = true;
            return;
        };
        let is_bedrock = self
            .overview()
            .server_type_label()
            .eq_ignore_ascii_case("bedrock");
        match run_blocking(PlayersState::load(&client, is_bedrock)) {
            Ok(players) => self.current_session_mut().players = players,
            Err(error) => {
                let state = &mut self.current_session_mut().players;
                state.error = Some(error.to_string());
                state.loaded = true;
            }
        }
    }

    fn load_worlds_if_needed(&mut self) {
        if self.current_session().worlds.loaded {
            return;
        }
        let Some(client) = self.current_session().client.clone() else {
            let state = &mut self.current_session_mut().worlds;
            state.error = Some("Worlds require an authenticated host session".to_string());
            state.loaded = true;
            return;
        };
        match run_blocking(super::worlds::WorldsState::load(&client)) {
            Ok(worlds) => self.current_session_mut().worlds = worlds,
            Err(error) => {
                let state = &mut self.current_session_mut().worlds;
                state.error = Some(error.to_string());
                state.loaded = true;
            }
        }
    }

    fn load_backups_if_needed(&mut self) {
        if self.current_session().backups.loaded {
            return;
        }
        let Some(client) = self.current_session().client.clone() else {
            let state = &mut self.current_session_mut().backups;
            state.error = Some("Backups require an authenticated host session".to_string());
            state.loaded = true;
            return;
        };
        let context_slot_id = self.current_session().backups.context_slot_id.clone();
        match run_blocking(super::backups::BackupsState::load(&client, context_slot_id)) {
            Ok(backups) => self.current_session_mut().backups = backups,
            Err(error) => {
                let state = &mut self.current_session_mut().backups;
                state.error = Some(error.to_string());
                state.loaded = true;
            }
        }
    }

    fn load_performance_if_needed(&mut self) {
        self.current_session_mut().performance.poll_pending();
        if !self.current_session().performance.poll_due() {
            return;
        }
        let Some(client) = self.current_session().client.clone() else {
            let state = &mut self.current_session_mut().performance;
            state.error = Some("Performance requires an authenticated host session".to_string());
            state.loaded = true;
            return;
        };
        let server_type = self.overview().server_type_label().to_string();
        let status = self.overview().status.clone();
        let running = status.as_ref().map(|value| value.running);
        self.current_session_mut()
            .performance
            .begin_request(client, running, server_type);
    }

    fn load_components_if_needed(&mut self) {
        if self.current_session().components.loaded {
            return;
        }
        let Some(client) = self.current_session().client.clone() else {
            let state = &mut self.current_session_mut().components;
            state.error = Some("Components require an authenticated host session".to_string());
            state.loaded = true;
            return;
        };
        match run_blocking(ComponentsState::load(&client)) {
            Ok(state) => self.current_session_mut().components = state,
            Err(error) => {
                let state = &mut self.current_session_mut().components;
                state.error = Some(error.to_string());
                state.loaded = true;
            }
        }
    }

    fn load_files_if_needed(&mut self) {
        if self.current_session().files.loaded {
            return;
        }
        if self.overview().capabilities.is_some()
            && !self.has_permission(msc_api::dto::PermissionCategoryDto::Admin)
        {
            let state = &mut self.current_session_mut().files;
            state.error = Some("Files require the agent's admin permission".to_string());
            state.loaded = true;
            return;
        }
        let Some(client) = self.current_session().client.clone() else {
            let state = &mut self.current_session_mut().files;
            state.error = Some("Files require an authenticated host session".to_string());
            state.loaded = true;
            return;
        };
        let requested_path = self.current_session().files.requested_path.clone();
        match run_blocking(FilesState::load(&client, requested_path)) {
            Ok(files) => self.current_session_mut().files = files,
            Err(error) => {
                let state = &mut self.current_session_mut().files;
                state.error = Some(error.to_string());
                state.loaded = true;
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

fn player_mutation_consequence(mutation: &PlayerMutation) -> String {
    match mutation {
        PlayerMutation::ToggleHidden { hidden, .. } => format!(
            "The selected profile will be {} from the player-data list.",
            if *hidden { "hidden" } else { "shown" }
        ),
        PlayerMutation::Delete { .. } => {
            "The selected player's stored data will be deleted from this server.".to_string()
        }
        PlayerMutation::Duplicate { .. } => {
            "The agent will create a second stored copy of this player's data.".to_string()
        }
        PlayerMutation::MigrateOffline { .. } => {
            "The agent will move this player's data to its offline UUID.".to_string()
        }
        PlayerMutation::MigrateCustom { target_uuid, .. } => {
            format!("The agent will move this player's data to custom UUID {target_uuid}.")
        }
        PlayerMutation::Identify { gamertag, .. } => {
            format!("The agent will identify this Bedrock profile as {gamertag}.")
        }
        PlayerMutation::SkinOverride {
            lookup_identifier, ..
        } => format!(
            "The agent will {} the skin lookup override.",
            if lookup_identifier.is_some() {
                "set"
            } else {
                "clear"
            }
        ),
        PlayerMutation::AllowlistAdd { name } => {
            format!("The Bedrock allowlist will include {name}.")
        }
        PlayerMutation::AllowlistRemove { name } => {
            format!("The Bedrock allowlist will remove {name}.")
        }
        PlayerMutation::ClearSessionLog => {
            "The agent will delete the selected server's recorded join/leave history.".to_string()
        }
    }
}

fn component_confirmation_details(mutation: &ComponentMutation) -> (String, String) {
    match mutation {
        ComponentMutation::ChangeVersion { version_id } => (
            version_id.clone(),
            "The selected server JAR/version will change; the agent may require a restart."
                .to_string(),
        ),
        ComponentMutation::UpdateAddon { jar_stem } => (
            jar_stem.clone(),
            "The agent will download and apply the selected add-on update.".to_string(),
        ),
        ComponentMutation::UpdateAllAddons => (
            "all compatible add-ons".to_string(),
            "The agent will update every compatible installed add-on.".to_string(),
        ),
        ComponentMutation::SetAddonEnabled { jar_stem, enabled } => (
            jar_stem.clone(),
            format!(
                "The selected add-on will be {} for the next server launch.",
                if *enabled { "enabled" } else { "disabled" }
            ),
        ),
        ComponentMutation::RemoveAddon { jar_stem } => (
            jar_stem.clone(),
            "The selected installed add-on will be removed from this server.".to_string(),
        ),
        ComponentMutation::UpdateSystem { component } => (
            component.clone(),
            "The agent will update this installed system component if a compatible version exists."
                .to_string(),
        ),
        ComponentMutation::InstallCatalog { title, .. } => (
            title.clone(),
            "The selected catalog add-on will be installed into this server.".to_string(),
        ),
        ComponentMutation::ActivateResourcePack { pack_id, .. } => (
            pack_id.clone(),
            "The selected resource pack will be activated for this server.".to_string(),
        ),
        ComponentMutation::ClearResourcePack => (
            "active resource pack".to_string(),
            "The active Java resource pack will be cleared from server settings.".to_string(),
        ),
        ComponentMutation::RemoveResourcePack { pack_id, .. } => (
            pack_id.clone(),
            "The selected resource pack will be removed from the agent-managed store.".to_string(),
        ),
        ComponentMutation::SetResourcePackUrl { url } => (
            url.clone(),
            "The server's resource-pack URL will be replaced with this value.".to_string(),
        ),
        ComponentMutation::Modpack { path, action } => (
            path.display().to_string(),
            format!(
                "The agent will {} this staged modpack archive for the selected server.",
                if action == "inspect" {
                    "inspect"
                } else {
                    "import or replace"
                }
            ),
        ),
    }
}

fn world_confirmation_details(mutation: &WorldMutation) -> (String, String) {
    match mutation {
        WorldMutation::Create { name } => (
            name.clone(),
            "The agent will create a new saved world slot from the current world.".to_string(),
        ),
        WorldMutation::Rename { slot_id, name } => (
            slot_id.clone(),
            format!("The selected slot name will change to {name}; its world files are untouched."),
        ),
        WorldMutation::RenameActive { name } => (
            name.clone(),
            "The agent will rename the active world's on-disk folders.".to_string(),
        ),
        WorldMutation::Delete { slot_id } => (
            slot_id.clone(),
            "The selected non-active world slot and its saved archive will be deleted.".to_string(),
        ),
        WorldMutation::Duplicate { slot_id } => (
            slot_id.clone(),
            "The agent will create another saved slot from this world.".to_string(),
        ),
        WorldMutation::Copy { destination_slot_id, source_slot_id } => (
            destination_slot_id.clone(),
            format!("Saved slot {source_slot_id} will replace the selected destination slot."),
        ),
        WorldMutation::SaveCurrent { slot_id } => (
            slot_id.clone(),
            "The current live world will be archived into the selected slot.".to_string(),
        ),
        WorldMutation::Activate { slot_id } => (
            slot_id.clone(),
            "The server must be stopped; the agent takes a safety backup before activation.".to_string(),
        ),
        WorldMutation::ReplaceActive { level_name, .. } => (
            level_name.clone(),
            "The server must be stopped; the agent takes a safety backup before replacing the live world.".to_string(),
        ),
        WorldMutation::Export { slot_id, output } => (
            slot_id.clone(),
            format!("The agent will stage this slot and write the archive to {}.", output.display()),
        ),
        WorldMutation::Import { path, name } => (
            name.clone(),
            format!("The agent will upload {} into a new world slot.", path.display()),
        ),
        WorldMutation::Convert { source_slot_id, target_server_id, .. } => (
            source_slot_id.clone(),
            format!("The agent will convert this world for target server {target_server_id}."),
        ),
        WorldMutation::Repair { slot_id } => (
            slot_id.clone(),
            "The agent will run its Bedrock level.dat repair operation.".to_string(),
        ),
    }
}

fn backup_confirmation_details(mutation: &BackupMutation) -> (String, String) {
    match mutation {
        BackupMutation::Manual => (
            "active server".to_string(),
            "The agent will create and verify a manual backup; a running server may pause saves."
                .to_string(),
        ),
        BackupMutation::Restore { backup_id } => (
            backup_id.clone(),
            "The server must be stopped; the agent takes a safety backup before restoring."
                .to_string(),
        ),
        BackupMutation::Delete { backup_id } => (
            backup_id.clone(),
            "The agent will delete this backup and enforce its last-verified-backup safety rule."
                .to_string(),
        ),
        BackupMutation::UpdateConfig {
            enabled,
            interval_minutes,
            max_count,
        } => (
            "backup schedule".to_string(),
            format!(
                "The agent will set enabled={enabled:?}, interval={interval_minutes:?} minutes, retention={max_count:?} backups."
            ),
        ),
    }
}

fn manage_confirmation_details(mutation: &ManageMutation) -> (String, String) {
    match mutation {
        ManageMutation::SetActive { server_id } => (
            server_id.clone(),
            "The selected server becomes the active server for lifecycle and editor requests."
                .to_string(),
        ),
        ManageMutation::Create(draft) => (
            draft.name.clone(),
            format!(
                "The agent will create a {} server and apply the staged port, world, and EULA choices.",
                draft.server_type.as_deref().unwrap_or("java")
            ),
        ),
        ManageMutation::Import(draft) => (
            draft.source_path.clone(),
            "The agent will scan and register this existing server source; no local desktop file picker is involved."
                .to_string(),
        ),
        ManageMutation::Rename { server_id, name } => (
            server_id.clone(),
            format!("The registered server display name will change to {name}; its directory is untouched."),
        ),
        ManageMutation::AcceptEula { server_id } => (
            server_id.clone(),
            "The agent will record acceptance of Minecraft's EULA for this server.".to_string(),
        ),
        ManageMutation::Delete { server_id } => (
            server_id.clone(),
            "The selected registered server and its managed data will be deleted. This cannot be undone by the TUI."
                .to_string(),
        ),
    }
}

fn editor_confirmation_details(mutation: &EditorMutation) -> (String, String) {
    match mutation {
        EditorMutation::Rename { server_id, name } => (
            server_id.clone(),
            format!("The server display name will change to {name}."),
        ),
        EditorMutation::SetDirectory {
            server_id,
            directory,
        } => (
            server_id.clone(),
            format!("The agent will use {directory} as this server's host-side directory."),
        ),
        EditorMutation::UpdateRam { min, max } => (
            "RAM allocation".to_string(),
            format!("The agent will set minimum RAM to {min:?} GB and maximum RAM to {max:?} GB."),
        ),
        EditorMutation::UpdatePort { port } => (
            port.to_string(),
            "The agent will update the server port through its validated settings route."
                .to_string(),
        ),
        EditorMutation::SetJavaPath { path } => (
            path.clone(),
            "The agent will use this Java executable path for server launches.".to_string(),
        ),
        EditorMutation::SetJavaArguments { arguments } => (
            "Java arguments".to_string(),
            format!("The agent will use these extra launch arguments: {arguments}"),
        ),
        EditorMutation::Playit { start } => (
            "Playit".to_string(),
            format!(
                "The agent will request a Playit tunnel {}.",
                if *start { "start" } else { "stop" }
            ),
        ),
        EditorMutation::XboxBroadcast { start } => (
            "Xbox Broadcast".to_string(),
            format!(
                "The agent will request Xbox Broadcast {}.",
                if *start { "start" } else { "stop" }
            ),
        ),
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

fn connection_confirmation_details(mutation: &ConnectionMutation) -> (String, String) {
    match mutation {
        ConnectionMutation::Playit { start } => (
            "Playit tunnel".to_string(),
            format!("The agent will {} the Playit tunnel.", if *start { "start" } else { "stop" }),
        ),
        ConnectionMutation::Broadcast { start } => (
            "Xbox Broadcast".to_string(),
            format!("The agent will {} Xbox Broadcast.", if *start { "start" } else { "stop" }),
        ),
        ConnectionMutation::BroadcastAutostart { enabled } => (
            "Xbox Broadcast autostart".to_string(),
            format!("The agent will {} automatic Xbox Broadcast startup.", if *enabled { "enable" } else { "disable" }),
        ),
        ConnectionMutation::BroadcastCredentials { .. } => (
            "Xbox Broadcast credentials".to_string(),
            "The agent will replace the stored account credentials; the password will not be returned or echoed.".to_string(),
        ),
        ConnectionMutation::DuckDns { hostname } => (
            "DuckDNS hostname".to_string(),
            format!("The agent will {} the DuckDNS hostname.", if hostname.is_some() { "set" } else { "clear" }),
        ),
    }
}

fn health_confirmation_details(mutation: &HealthMutation) -> (String, String) {
    let HealthMutation::Repair { problem_id, action } = mutation;
    (
        problem_id.clone(),
        format!("The agent will run the {action} repair. A running server may be refused."),
    )
}

fn access_confirmation_details(mutation: &AccessMutation) -> (String, String) {
    match mutation {
        AccessMutation::AllowlistAdd { name } => (
            name.clone(),
            "The Bedrock allowlist will include this player.".to_string(),
        ),
        AccessMutation::AllowlistRemove { name } => (
            name.clone(),
            "The Bedrock allowlist will remove this player.".to_string(),
        ),
        AccessMutation::RevokeUser { user_id } => (
            user_id.clone(),
            "The named bearer credential will be revoked and cannot authenticate again."
                .to_string(),
        ),
    }
}

impl OverviewState {
    fn load_blocking(client: &SharedClient) -> Result<Self, crate::cli::CliError> {
        run_blocking(Self::load(client))
    }
}

#[cfg(not(test))]
fn client_from_common(common: &CommonArgs) -> Option<SharedClient> {
    if let Ok(client) = SharedClient::from_common(common) {
        return Some(client);
    }
    #[cfg(target_os = "macos")]
    {
        let host = common
            .base_url
            .clone()
            .unwrap_or_else(|| format!("{}:{}", common.host, common.port));
        return local_bootstrap::connect_for_host(&host).ok();
    }
    #[allow(unreachable_code)]
    None
}

#[cfg(test)]
fn client_from_common(_common: &CommonArgs) -> Option<SharedClient> {
    // Integration tests include the CLI module with a small transport shim;
    // production construction is exercised by the binary itself.
    None
}
