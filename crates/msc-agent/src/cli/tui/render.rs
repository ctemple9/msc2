//! Terminal-native rendering for the first overview slice.
//!
//! The order mirrors the established MSC window: context header, controls
//! rail, server identity, section tabs, overview content, and console dock.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Wrap};

use super::app::{App, FocusTarget, SmallSurface};
use super::console::ConsoleView;
use super::layout::{LayoutMode, ShellLayout};
use super::overview::TAB_NAMES;

pub fn render(frame: &mut Frame, app: &mut App) {
    app.poll_console();
    let layout = app.prepare_layout(frame.area());
    render_header(frame, layout.header, app, layout.mode);

    match layout.mode {
        LayoutMode::Wide => render_standard_shell(frame, &layout, app, true),
        LayoutMode::Medium => render_standard_shell(frame, &layout, app, false),
        LayoutMode::Small => render_small(frame, layout.content, app),
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App, mode: LayoutMode) {
    let controls = match mode {
        LayoutMode::Wide => "[Tab] focus  [1-7] section  [a] start  [x] stop  [q] quit",
        LayoutMode::Medium => "[r] rail  [c] console  [s] section  [a/x] lifecycle  [q] quit",
        LayoutMode::Small => "[s] sections  [c] console  [?] help  [q] quit",
    };
    let state = app.overview().lifecycle_label();
    let line = Line::from(vec![
        Span::styled("MSC", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::raw(format!("Host: {}", app.host())),
        Span::raw("  |  "),
        Span::raw(format!("Server: {}", app.overview().selected_server_name())),
        Span::raw("  |  "),
        Span::styled(format!("State: {state}"), state_style(state)),
        Span::raw("  |  "),
        Span::styled(
            format!("Focus: {}", app.focus().label()),
            focus_style(app.focus() == FocusTarget::Host),
        ),
        Span::raw("  "),
        Span::raw(controls),
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
    let lifecycle = app.overview().lifecycle_label();
    let selected = app.overview().selected_server_name();
    let action = app.last_action().unwrap_or("Ready");
    let text = format!(
        "Host session\n{}\n\nSelected server\n{}\n\nLifecycle\n{}  [a] start  [x] stop\n\nServices\nHow to connect\nMaintenance\nQuick commands\n\n{}",
        app.host(),
        selected,
        lifecycle,
        action
    );
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
    let overview = app.overview();
    let (path, name) = overview
        .selected_server()
        .map(|server| (server.directory.as_str(), server.name.as_str()))
        .unwrap_or(("—", overview.selected_server_name()));
    let text = format!(
        "{title}\n{name}  ·  {}  ·  {}  ·  {path}",
        overview.edition_label(),
        overview.lifecycle_label()
    );
    frame.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

fn render_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let available = app.available_tabs();
    let titles = available
        .iter()
        .map(|index| Line::from(format!(" {} ", TAB_NAMES[*index])))
        .collect::<Vec<_>>();
    let selected = available
        .iter()
        .position(|index| *index == app.active_tab())
        .unwrap_or(0);
    let tabs = Tabs::new(titles)
        .select(selected)
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
    let overview = app.overview();
    let body = if app.active_tab() == 0 {
        let health = overview.health_summary();
        let health_state = overview
            .health
            .as_ref()
            .map(|value| value.overall_severity.as_str())
            .unwrap_or("unknown");
        let note = app.notes_for_selected_server().unwrap_or("No local note");
        let activity = if overview.activity.is_empty() {
            "No recent activity".to_string()
        } else {
            overview.activity.join("  |  ")
        };
        format!(
            "Connection Information\nConnection and live stats\n{}\n\nLive Stats\n{}\n\nServer Health  [{}]\n{}\n\nActivity\n{}\n\nNotes (local to this host/server)\n{}",
            overview.connection_detail(),
            overview.stats_summary(),
            health_state,
            health,
            activity,
            note
        )
    } else {
        format!(
            "{} is available only when its capability and token permission are advertised.",
            app.active_tab_name()
        )
    };
    let text = if overview.error.is_some() {
        format!(
            "{}\n\n{}\n\nConnection and live stats\nServer health\nActivity",
            overview.error.as_deref().unwrap_or("Agent unavailable"),
            body
        )
    } else {
        body
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
    let console = app.console();
    let mut lines = vec![Line::from(format!(
        "Filter: {}{}  Follow: {}  {}",
        console.filter().label(),
        if console.search().is_empty() {
            String::new()
        } else {
            format!("  Search: /{}", console.search())
        },
        if console.follow() { "on" } else { "off" },
        if console.paused() { "PAUSED" } else { "LIVE" },
    ))];
    if let Some(status) = console.status() {
        lines.push(Line::from(format!("Status: {status}")));
    }
    if console.palette_open() {
        lines.push(Line::from(
            "COMMAND PALETTE  [j/k] choose  [enter] run  [esc] close",
        ));
        lines.extend(ConsoleView::palette_entries().iter().enumerate().map(
            |(index, (label, command))| {
                let marker = if index == console.palette_index() {
                    "›"
                } else {
                    " "
                };
                Line::from(format!("{marker} {label:<12} {command}"))
            },
        ));
    } else if console.collapsed() {
        lines.push(Line::from(
            "Console collapsed  [C] expand  [>] raw command  [p] palette",
        ));
    } else {
        lines.extend(
            console
                .visible_lines()
                .into_iter()
                .map(|line| Line::from(format!("[{}] {} {}", line.ts, line.source, line.text))),
        );
        if lines.len() == 1 {
            lines.push(Line::from("No console lines yet."));
        }
        lines.push(Line::from(format!(
            "> {}",
            if console.command().is_empty() {
                "raw Minecraft command"
            } else {
                console.command()
            }
        )));
        lines.push(Line::from(
            "[c] collapse  [/] search  [f] follow  [space] pause  [v/y] select/copy  [l] clear  [p] palette",
        ));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::TOP).title(title))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_small(frame: &mut Frame, area: Rect, app: &App) {
    match app.small_surface() {
        SmallSurface::Overview => {
            let text = format!(
                "{}\n\nHost: {}\nServer: {}\nState: {}\n\n{}\n\n[s] sections  [c] console  [?] help",
                app.active_tab_name(),
                app.host(),
                app.overview().selected_server_name(),
                app.overview().lifecycle_label(),
                app.overview().stats_summary()
            );
            frame.render_widget(
                Paragraph::new(text)
                    .block(Block::default().borders(Borders::ALL).title("› FOCUSED VIEW"))
                    .wrap(Wrap { trim: true }),
                area,
            );
        }
        SmallSurface::Sections => {
            let rows = app
                .available_tabs()
                .iter()
                .map(|index| {
                    if *index == app.active_tab() {
                        format!("› {}", TAB_NAMES[*index])
                    } else {
                        format!("  {}", TAB_NAMES[*index])
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
        SmallSurface::Console => {
            let console = if app.overview().console.is_empty() {
                "No console data loaded."
            } else {
                "Console tail loaded."
            };
            frame.render_widget(
                Paragraph::new(format!(
                    "Host: {}\n\n{}\n\n[s] sections  [?] help  [esc] overview",
                    app.host(),
                    console
                ))
                .block(Block::default().borders(Borders::ALL).title("› CONSOLE"))
                .wrap(Wrap { trim: true }),
                area,
            );
        }
        SmallSurface::Help => frame.render_widget(
            Paragraph::new(
                "[s] section switcher\n[c] console\n[tab] move focus\n[a/x] start or stop\n[q] quit\n\nThe terminal shell uses the same MSC API as every other client.",
            )
            .block(Block::default().borders(Borders::ALL).title("› HELP"))
            .wrap(Wrap { trim: true }),
            area,
        ),
    }
}

fn state_style(state: &str) -> Style {
    let color = match state {
        "RUNNING" => Color::Green,
        "STOPPED" => Color::Yellow,
        _ => Color::DarkGray,
    };
    Style::default().fg(color)
}

fn focus_style(focused: bool) -> Style {
    if focused {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    }
}
