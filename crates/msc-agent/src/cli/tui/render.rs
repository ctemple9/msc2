//! Deliberately quiet terminal rendering. The shell carries the desktop
//! information order with text hierarchy and focus, not simulated cards.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Wrap};

use super::app::{App, FocusTarget, SmallSurface};
use super::layout::{LayoutMode, ShellLayout};

pub fn render(frame: &mut Frame, app: &mut App) {
    let layout = app.prepare_layout(frame.area());
    render_header(frame, layout.header, app, layout.mode);

    match layout.mode {
        LayoutMode::Wide => render_standard_shell(frame, &layout, app, true),
        LayoutMode::Medium => render_standard_shell(frame, &layout, app, false),
        LayoutMode::Small => render_small(frame, layout.content, app),
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App, mode: LayoutMode) {
    let compact_controls = match mode {
        LayoutMode::Wide => "[Tab] focus  [q] quit",
        LayoutMode::Medium => "[r] rail  [c] console  [s] section  [q] quit",
        LayoutMode::Small => "[s] sections  [c] console  [?] help  [q] quit",
    };
    let line = Line::from(vec![
        Span::styled("MSC", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::raw(format!("Host: {}", app.host())),
        Span::raw("  |  "),
        Span::raw("Server: awaiting agent"),
        Span::raw("  |  "),
        Span::styled(
            format!("Focus: {}", app.focus().label()),
            focus_style(app.focus() == FocusTarget::Host),
        ),
        Span::raw("  "),
        Span::raw(compact_controls),
    ]);
    frame.render_widget(
        Paragraph::new(line).block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

fn render_standard_shell(frame: &mut Frame, layout: &ShellLayout, app: &App, wide: bool) {
    if let Some(rail) = layout.rail {
        render_rail(frame, rail, app);
    }
    if let Some(identity) = layout.identity {
        render_identity(frame, identity, app);
    }
    if let Some(tabs) = layout.tabs {
        if wide {
            render_tabs(frame, tabs, app);
        } else {
            render_section_selector(frame, tabs, app);
        }
    }
    render_content(frame, layout.content, app);
    if let Some(console) = layout.console {
        render_console(frame, console, app);
    }
}

fn render_rail(frame: &mut Frame, area: Rect, app: &App) {
    let title = if app.focus() == FocusTarget::Rail {
        "› SERVER CONTROLS"
    } else {
        "SERVER CONTROLS"
    };
    let text = "Host session\nAwaiting connection\n\nLifecycle\nStart / stop appear after agent state loads\n\nGroups\nServices\nHow to connect\nMaintenance\nQuick commands";
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().borders(Borders::RIGHT).title(title))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_identity(frame: &mut Frame, area: Rect, app: &App) {
    let title = if app.focus() == FocusTarget::Content {
        "› SERVER IDENTITY"
    } else {
        "SERVER IDENTITY"
    };
    let text = format!(
        "{title}\nSelected host: {}  ·  Server data has not been loaded",
        app.host()
    );
    frame.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

fn render_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let titles = App::tabs()
        .iter()
        .map(|tab| Line::from(format!(" {tab} ")))
        .collect::<Vec<_>>();
    let tabs = Tabs::new(titles)
        .select(app.active_tab())
        .highlight_style(focus_style(app.focus() == FocusTarget::Sections))
        .divider(" ");
    frame.render_widget(tabs, area);
}

fn render_section_selector(frame: &mut Frame, area: Rect, app: &App) {
    let marker = if app.focus() == FocusTarget::Sections {
        "› "
    } else {
        ""
    };
    let text = format!(
        "{marker}Section: {}  Rail: {} [r]  Console: {} [c]  [s] focus section switcher",
        app.active_tab_name(),
        if app.rail_visible() {
            "shown"
        } else {
            "hidden"
        },
        if app.console_visible() {
            "shown"
        } else {
            "hidden"
        },
    );
    frame.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

fn render_content(frame: &mut Frame, area: Rect, app: &App) {
    let title = if app.focus() == FocusTarget::Content {
        format!("› {}", app.active_tab_name().to_uppercase())
    } else {
        app.active_tab_name().to_uppercase()
    };
    let text = if app.active_tab() == 0 {
        "Connect to an MSC agent to load server details.\n\nConnection and live stats\nServer health\nActivity"
    } else {
        "This section is reserved in the shell. Connect to an MSC agent to load its capability-backed content."
    };
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_console(frame: &mut Frame, area: Rect, app: &App) {
    let title = if app.focus() == FocusTarget::Console {
        "› CONSOLE"
    } else {
        "CONSOLE"
    };
    frame.render_widget(
        Paragraph::new(
            "No console data loaded. Raw command entry is available after a server connects.",
        )
        .block(Block::default().borders(Borders::TOP).title(title))
        .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_small(frame: &mut Frame, area: Rect, app: &App) {
    match app.small_surface() {
        SmallSurface::Overview => {
            let text = format!(
                "{}\n\nHost: {}\nServer: awaiting agent\n\nConnect to load {}.\n\n[s] sections  [c] console  [?] help",
                app.active_tab_name(),
                app.host(),
                app.active_tab_name().to_lowercase()
            );
            frame.render_widget(
                Paragraph::new(text)
                    .block(Block::default().borders(Borders::ALL).title("› FOCUSED VIEW"))
                    .wrap(Wrap { trim: true }),
                area,
            );
        }
        SmallSurface::Sections => {
            let rows = App::tabs()
                .iter()
                .enumerate()
                .map(|(index, tab)| {
                    if index == app.active_tab() {
                        format!("› {tab}")
                    } else {
                        format!("  {tab}")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            frame.render_widget(
                Paragraph::new(format!("{rows}\n\n[j/k] choose  [enter] open  [c] console"))
                    .block(Block::default().borders(Borders::ALL).title("› SECTIONS")),
                area,
            );
        }
        SmallSurface::Console => frame.render_widget(
            Paragraph::new("Host: ".to_owned() + app.host() + "\n\nNo console data loaded.\n\n[s] sections  [?] help  [esc] overview")
                .block(Block::default().borders(Borders::ALL).title("› CONSOLE"))
                .wrap(Wrap { trim: true }),
            area,
        ),
        SmallSurface::Help => frame.render_widget(
            Paragraph::new("[s] section switcher\n[c] console\n[tab] move focus\n[q] quit\n\nThe terminal shell uses the same MSC API as every other client.")
                .block(Block::default().borders(Borders::ALL).title("› HELP"))
                .wrap(Wrap { trim: true }),
            area,
        ),
    }
}

fn focus_style(focused: bool) -> Style {
    if focused {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    }
}
