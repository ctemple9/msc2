//! Presentation-only state for the first terminal shell. Agent data arrives
//! through the shared authenticated transport in the next Phase 13 step.

use crossterm::event::KeyCode;
use ratatui::layout::Rect;

use super::layout::{LayoutMode, ShellLayout};
use crate::cli::CommonArgs;

const TABS: [&str; 7] = [
    "Overview",
    "Players",
    "Worlds",
    "Performance",
    "Components",
    "Settings",
    "Files",
];

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
    host: String,
    active_tab: usize,
    focus: FocusTarget,
    rail_visible: bool,
    console_visible: bool,
    mode: LayoutMode,
    small_surface: SmallSurface,
}

impl App {
    pub fn from_common(common: &CommonArgs) -> Self {
        let host = common
            .base_url
            .clone()
            .unwrap_or_else(|| format!("{}:{}", common.host, common.port));
        Self::new(host)
    }

    pub fn new(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
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
        &self.host
    }

    pub fn active_tab(&self) -> usize {
        self.active_tab
    }

    pub fn active_tab_name(&self) -> &'static str {
        TABS[self.active_tab]
    }

    pub fn tabs() -> &'static [&'static str] {
        &TABS
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

    fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % TABS.len();
    }

    fn previous_tab(&mut self) {
        self.active_tab = (self.active_tab + TABS.len() - 1) % TABS.len();
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
    }
}
