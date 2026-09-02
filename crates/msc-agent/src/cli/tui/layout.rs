//! Geometry for the terminal shell. Keeping the breakpoints here makes the
//! rendering contract testable without a real terminal.

use ratatui::layout::{Constraint, Layout, Rect};

pub const WIDE_MIN_WIDTH: u16 = 120;
pub const WIDE_MIN_HEIGHT: u16 = 36;
pub const MEDIUM_MIN_WIDTH: u16 = 80;
pub const MEDIUM_MIN_HEIGHT: u16 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Wide,
    Medium,
    Small,
}

#[derive(Debug, Clone, Copy)]
pub struct ShellLayout {
    pub mode: LayoutMode,
    pub header: Rect,
    pub rail: Option<Rect>,
    pub identity: Option<Rect>,
    pub tabs: Option<Rect>,
    pub content: Rect,
    pub console: Option<Rect>,
}

impl ShellLayout {
    pub fn for_area(area: Rect, rail_visible: bool, console_visible: bool) -> Self {
        let mode = layout_mode(area);
        match mode {
            LayoutMode::Wide => wide(area),
            LayoutMode::Medium => medium(area, rail_visible, console_visible),
            LayoutMode::Small => small(area),
        }
    }
}

pub fn layout_mode(area: Rect) -> LayoutMode {
    if area.width >= WIDE_MIN_WIDTH && area.height >= WIDE_MIN_HEIGHT {
        LayoutMode::Wide
    } else if area.width >= MEDIUM_MIN_WIDTH && area.height >= MEDIUM_MIN_HEIGHT {
        LayoutMode::Medium
    } else {
        LayoutMode::Small
    }
}

fn wide(area: Rect) -> ShellLayout {
    let vertical = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(area);
    let horizontal =
        Layout::horizontal([Constraint::Length(26), Constraint::Min(1)]).split(vertical[1]);
    let main = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(8),
    ])
    .split(horizontal[1]);

    ShellLayout {
        mode: LayoutMode::Wide,
        header: vertical[0],
        rail: Some(horizontal[0]),
        identity: Some(main[0]),
        tabs: Some(main[1]),
        content: main[2],
        console: Some(main[3]),
    }
}

fn medium(area: Rect, rail_visible: bool, console_visible: bool) -> ShellLayout {
    let vertical = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).split(area);
    let (rail, main_area) = if rail_visible {
        let horizontal =
            Layout::horizontal([Constraint::Length(24), Constraint::Min(1)]).split(vertical[1]);
        (Some(horizontal[0]), horizontal[1])
    } else {
        (None, vertical[1])
    };
    let main = if console_visible {
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(7),
        ])
        .split(main_area)
    } else {
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(main_area)
    };

    ShellLayout {
        mode: LayoutMode::Medium,
        header: vertical[0],
        rail,
        identity: Some(main[0]),
        tabs: Some(main[1]),
        content: main[2],
        console: if console_visible { Some(main[3]) } else { None },
    }
}

fn small(area: Rect) -> ShellLayout {
    let vertical = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).split(area);
    ShellLayout {
        mode: LayoutMode::Small,
        header: vertical[0],
        rail: None,
        identity: None,
        tabs: None,
        content: vertical[1],
        console: None,
    }
}
